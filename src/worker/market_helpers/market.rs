use crate::worker::market_helpers::exchange_pair::ExchangePair;
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;
use crate::worker::markets::binance::Binance;
use crate::worker::markets::bitfinex::Bitfinex;
use crate::worker::markets::coinbase::Coinbase;
use crate::worker::markets::hitbtc::Hitbtc;
use crate::worker::markets::huobi::Huobi;
use crate::worker::markets::kraken::Kraken;
use crate::worker::markets::okcoin::Okcoin;
use crate::worker::markets::poloniex::Poloniex;
use crate::worker::network_helpers::socket_helper::SocketHelper;
use rustc_serialize::json::{Array, Object};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

pub fn market_factory(
    mut spine: MarketSpine,
    exchange_pairs: Vec<ExchangePair>,
) -> Arc<Mutex<dyn Market + Send>> {
    let mask_pairs = match spine.name.as_ref() {
        "binance" => vec![("IOT", "IOTA"), ("USD", "USDT")],
        "bitfinex" => vec![("DASH", "dsh"), ("QTUM", "QTM")],
        "poloniex" => vec![("USD", "USDT"), ("XLM", "STR")],
        "kraken" => vec![("BTC", "XBT")],
        "huobi" | "hitbtc" | "okcoin" => vec![("USD", "USDT")],
        _ => vec![],
    };

    spine.add_mask_pairs(mask_pairs);

    let market: Arc<Mutex<dyn Market + Send>> = match spine.name.as_ref() {
        "binance" => Arc::new(Mutex::new(Binance { spine })),
        "bitfinex" => Arc::new(Mutex::new(Bitfinex { spine })),
        "coinbase" => Arc::new(Mutex::new(Coinbase { spine })),
        "poloniex" => Arc::new(Mutex::new(Poloniex::new(spine))),
        "kraken" => Arc::new(Mutex::new(Kraken { spine })),
        "huobi" => Arc::new(Mutex::new(Huobi { spine })),
        "hitbtc" => Arc::new(Mutex::new(Hitbtc { spine })),
        "okcoin" => Arc::new(Mutex::new(Okcoin { spine })),
        _ => panic!("Market not found: {}", spine.name),
    };

    market
        .lock()
        .unwrap()
        .get_spine_mut()
        .set_arc(Arc::clone(&market));

    for exchange_pair in exchange_pairs {
        market.lock().unwrap().add_exchange_pair(exchange_pair);
    }

    market
}

// Establishes websocket connection with market (subscribes to the channel with pair)
// and calls lambda (param "callback" of SocketHelper constructor) when gets message from market
pub fn subscribe_channel(
    market: Arc<Mutex<dyn Market + Send>>,
    pair: String,
    channel: MarketChannels,
    url: String,
    on_open_msg: Option<String>,
) {
    trace!(
        "called subscribe_channel(). Market: {}, pair: {}, channel: {:?}",
        market.lock().unwrap().get_spine().name,
        pair,
        channel,
    );

    let socker_helper =
        SocketHelper::new(
            url,
            on_open_msg,
            pair,
            |pair: String, info: String| match channel {
                MarketChannels::Ticker => market.lock().unwrap().parse_ticker_info(pair, info),
                MarketChannels::Trades => market.lock().unwrap().parse_last_trade_info(pair, info),
                MarketChannels::Book => market.lock().unwrap().parse_depth_info(pair, info),
            },
        );
    socker_helper.start();
}

pub fn parse_str_from_json_object<T: FromStr>(object: &Object, key: &str) -> Option<T> {
    if let Some(value) = object.get(key) {
        if let Some(value) = value.as_string() {
            if let Ok(value) = value.parse() {
                return Some(value);
            }
        }
    }

    None
}

pub fn parse_str_from_json_array<T: FromStr>(array: &Array, key: usize) -> Option<T> {
    if let Some(value) = array.get(key) {
        if let Some(value) = value.as_string() {
            if let Ok(value) = value.parse() {
                return Some(value);
            }
        }
    }

    None
}

fn update(market: Arc<Mutex<dyn Market + Send>>) {
    let channels = MarketChannels::get_all();
    let exchange_pairs: Vec<String> = market
        .lock()
        .unwrap()
        .get_spine()
        .get_exchange_pairs()
        .keys()
        .cloned()
        .collect();
    let exchange_pairs_dummy = vec!["dummy".to_string()];
    let market_is_poloniex = market.lock().unwrap().get_spine().name == "poloniex";

    for channel in channels {
        // There are no distinct Trades channel in Poloniex. We get Trades inside of Book channel.
        if market_is_poloniex {
            if let MarketChannels::Trades = channel {
                continue;
            }
        }

        // Channel Ticker of Poloniex has different subscription system
        // - we can subscribe only for all exchange pairs together,
        // thus, we need to subscribe to a channel only once
        let exchange_pairs = if market_is_poloniex {
            if let MarketChannels::Ticker = channel {
                &exchange_pairs_dummy
            } else {
                &exchange_pairs
            }
        } else {
            &exchange_pairs
        };

        for exchange_pair in exchange_pairs {
            let market_2 = Arc::clone(&market);
            let pair = exchange_pair.to_string();
            let url = market.lock().unwrap().get_websocket_url(&pair, channel);

            let on_open_msg = market
                .lock()
                .unwrap()
                .get_websocket_on_open_msg(&pair, channel);

            let thread_name = format!(
                "fn: subscribe_channel, market: {}, pair: {}, channel: {:?}",
                market.lock().unwrap().get_spine().name,
                pair,
                channel
            );
            let thread = thread::Builder::new()
                .name(thread_name)
                .spawn(move || loop {
                    subscribe_channel(
                        Arc::clone(&market_2),
                        pair.clone(),
                        channel,
                        url.clone(),
                        on_open_msg.clone(),
                    );
                    thread::sleep(time::Duration::from_millis(10000));
                })
                .unwrap();
            thread::sleep(time::Duration::from_millis(12000));

            market.lock().unwrap().get_spine().tx.send(thread).unwrap();
        }
    }
}

pub trait Market {
    fn get_spine(&self) -> &MarketSpine;
    fn get_spine_mut(&mut self) -> &mut MarketSpine;
    fn make_pair(&self, pair: (&str, &str)) -> String {
        pair.0.to_string() + pair.1
    }

    fn add_exchange_pair(&mut self, exchange_pair: ExchangePair) {
        let pair_string = self.make_pair(exchange_pair.get_pair_ref());
        self.get_spine_mut()
            .add_exchange_pair(pair_string, exchange_pair);
    }

    fn get_total_volume(&self, first_currency: &str, second_currency: &str) -> f64 {
        let pair: String = self.make_pair((first_currency, second_currency));
        self.get_spine().get_total_volume(&pair)
    }

    fn get_total_ask(&self, first_currency: &str, second_currency: &str) -> f64 {
        let pair: String = self.make_pair((first_currency, second_currency));

        let ask_sum: f64 = self
            .get_spine()
            .get_exchange_pairs()
            .get(&pair)
            .unwrap()
            .get_total_ask();

        ask_sum
    }

    fn get_total_bid(&self, first_currency: &str, second_currency: &str) -> f64 {
        let pair: String = self.make_pair((first_currency, second_currency));

        let bid_sum: f64 = self
            .get_spine()
            .get_exchange_pairs()
            .get(&pair)
            .unwrap()
            .get_total_bid();

        bid_sum
    }

    fn get_channel_text_view(&self, channel: MarketChannels) -> String;
    fn get_websocket_url(&self, pair: &str, channel: MarketChannels) -> String;
    fn get_websocket_on_open_msg(&self, pair: &str, channel: MarketChannels) -> Option<String>;

    fn perform(&mut self) {
        trace!(
            "called Market::perform(). Market: {}",
            self.get_spine().name
        );

        let market = Arc::clone(self.get_spine().arc.as_ref().unwrap());

        let thread_name = format!("fn: update, market: {}", self.get_spine().name);
        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || update(market))
            .unwrap();
        self.get_spine().tx.send(thread).unwrap();
    }
    fn parse_ticker_info(&mut self, pair: String, info: String);
    fn parse_last_trade_info(&mut self, pair: String, info: String);
    fn parse_depth_info(&mut self, pair: String, info: String);
}

#[cfg(test)]
mod test {
    use crate::worker::defaults::{COINS, FIATS};
    use crate::worker::market_helpers::market::{market_factory, update, Market};
    use crate::worker::market_helpers::market_channels::MarketChannels;
    use crate::worker::market_helpers::market_spine::MarketSpine;
    use crate::worker::worker::test::{check_threads, get_worker};
    use crate::worker::worker::Worker;
    use ntest::timeout;
    use std::sync::mpsc::Receiver;
    use std::sync::{Arc, Mutex};
    use std::thread::JoinHandle;

    fn get_market(
        market_name: Option<&str>,
    ) -> (Arc<Mutex<dyn Market + Send>>, Receiver<JoinHandle<()>>) {
        let market_name = market_name.unwrap_or("binance").to_string();
        let fiats = Vec::from(FIATS);
        let coins = Vec::from(COINS);
        let exchange_pairs = Worker::make_exchange_pairs(coins, fiats);

        let (worker, tx, rx) = get_worker();
        let market_spine = MarketSpine::new(worker, tx, market_name);
        let market = market_factory(market_spine, exchange_pairs);

        (market, rx)
    }

    #[test]
    #[should_panic]
    fn test_market_factory() {
        let (_, _) = get_market(Some("not_existing_market"));
    }

    #[test]
    fn test_market_factory_2() {
        let (market, _) = get_market(None);

        assert!(market.lock().unwrap().get_spine().arc.is_some());

        let fiats = Vec::from(FIATS);
        let coins = Vec::from(COINS);
        let exchange_pairs = Worker::make_exchange_pairs(coins, fiats);
        let exchange_pair_keys: Vec<String> = market
            .lock()
            .unwrap()
            .get_spine()
            .get_exchange_pairs()
            .keys()
            .cloned()
            .collect();
        assert_eq!(exchange_pair_keys.len(), exchange_pairs.len());

        for pair in &exchange_pairs {
            let pair_string = market.lock().unwrap().make_pair(pair.get_pair_ref());
            assert!(exchange_pair_keys.contains(&pair_string));
        }
    }

    #[test]
    #[timeout(3000)]
    fn test_perform() {
        let (market, rx) = get_market(None);

        let thread_names = vec![format!(
            "fn: update, market: {}",
            market.lock().unwrap().get_spine().name
        )];

        market.lock().unwrap().perform();

        // TODO: Refactor (not always working)
        check_threads(thread_names, rx);
    }

    #[test]
    #[timeout(120000)]
    /// TODO: Refactor (not always working)
    fn test_update() {
        let (market, rx) = get_market(None);

        let channels = MarketChannels::get_all();
        let exchange_pairs: Vec<String> = market
            .lock()
            .unwrap()
            .get_spine()
            .get_exchange_pairs()
            .keys()
            .cloned()
            .collect();

        let mut thread_names = Vec::new();
        for pair in exchange_pairs {
            for channel in channels {
                let thread_name = format!(
                    "fn: subscribe_channel, market: {}, pair: {}, channel: {:?}",
                    market.lock().unwrap().get_spine().name,
                    pair,
                    channel
                );
                thread_names.push(thread_name);
            }
        }

        update(market);
        check_threads(thread_names, rx);
    }
}
