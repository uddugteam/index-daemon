use crate::repository::repositories::{
    MarketRepositoriesByMarketValue, MarketRepositoriesByPairTuple,
};
use crate::worker::helper_functions::get_pair_ref;
use crate::worker::market_helpers::exchange_pair::ExchangePair;
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;
use crate::worker::markets::binance::Binance;
use crate::worker::markets::bitfinex::Bitfinex;
use crate::worker::markets::bybit::Bybit;
use crate::worker::markets::coinbase::Coinbase;
use crate::worker::markets::ftx::Ftx;
use crate::worker::markets::gateio::Gateio;
use crate::worker::markets::gemini::Gemini;
use crate::worker::markets::hitbtc::Hitbtc;
use crate::worker::markets::huobi::Huobi;
use crate::worker::markets::kraken::Kraken;
use crate::worker::markets::kucoin::Kucoin;
use crate::worker::markets::okcoin::Okcoin;
use crate::worker::markets::poloniex::Poloniex;
use crate::worker::network_helpers::ws_client::WsClient;
use crate::worker::network_helpers::ws_server::ws_channels_holder::WsChannelsHolderHashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

pub fn market_factory(
    mut spine: MarketSpine,
    exchange_pairs: Vec<ExchangePair>,
    repositories: Option<MarketRepositoriesByPairTuple>,
    ws_channels_holder: &WsChannelsHolderHashMap,
) -> Arc<Mutex<dyn Market + Send>> {
    let mut repositories = repositories.unwrap_or_default();

    let mask_pairs = match spine.name.as_ref() {
        "binance" => vec![("IOT", "IOTA"), ("USD", "USDT")],
        "bitfinex" => vec![("DASH", "dsh"), ("QTUM", "QTM")],
        "poloniex" => vec![("USD", "USDT"), ("XLM", "STR")],
        "kraken" => vec![("BTC", "XBT")],
        "huobi" | "hitbtc" | "okcoin" | "gateio" | "kucoin" => vec![("USD", "USDT")],
        "ftx" => vec![("USD", "PERP")],
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
        "gemini" => Arc::new(Mutex::new(Gemini { spine })),
        "bybit" => Arc::new(Mutex::new(Bybit { spine })),
        "gateio" => Arc::new(Mutex::new(Gateio { spine })),
        "kucoin" => Arc::new(Mutex::new(Kucoin { spine })),
        "ftx" => Arc::new(Mutex::new(Ftx { spine })),
        _ => panic!("Market not found: {}", spine.name),
    };

    market
        .lock()
        .unwrap()
        .get_spine_mut()
        .set_arc(Arc::clone(&market));

    for exchange_pair in exchange_pairs {
        market.lock().unwrap().add_exchange_pair(
            exchange_pair.clone(),
            repositories.remove(&exchange_pair.pair),
            ws_channels_holder,
        );
    }

    market
}

// Establishes websocket connection with market (subscribes to the channel with pair)
// and calls lambda (param "callback" of SocketHelper constructor) when gets message from market
pub fn subscribe_channel(
    market: Arc<Mutex<dyn Market + Send>>,
    pair: String,
    channel: MarketChannels,
) {
    trace!(
        "called subscribe_channel(). Market: {}, pair: {}, channel: {:?}",
        market.lock().unwrap().get_spine().name,
        pair,
        channel,
    );

    let url = market.lock().unwrap().get_websocket_url(&pair, channel);

    let on_open_msg = market
        .lock()
        .unwrap()
        .get_websocket_on_open_msg(&pair, channel);

    let ws_client = WsClient::new(url, on_open_msg, pair, |pair: String, info: String| {
        if is_graceful_shutdown(&market) {
            return;
        }

        if let Ok(json) = serde_json::from_str(&info) {
            // This "match" returns value, but we shouldn't use it
            match channel {
                MarketChannels::Ticker => market.lock().unwrap().parse_ticker_json(pair, json),
                MarketChannels::Trades => market.lock().unwrap().parse_last_trade_json(pair, json),
                MarketChannels::Book => market.lock().unwrap().parse_depth_json(pair, json),
            };
        } else {
            // Either parse json error or received string is not json
        }
    });
    ws_client.start();
}

pub fn parse_str_from_json_object<T: FromStr>(
    object: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Option<T> {
    object.get(key)?.as_str()?.parse().ok()
}

pub fn parse_str_from_json_array<T: FromStr>(array: &[serde_json::Value], key: usize) -> Option<T> {
    array.get(key)?.as_str()?.parse().ok()
}

pub fn depth_helper_v1(json: &serde_json::Value) -> Vec<(f64, f64)> {
    json.as_array()
        .unwrap()
        .iter()
        .map(|v| {
            let v = v.as_array().unwrap();
            (
                parse_str_from_json_array(v, 0).unwrap(),
                parse_str_from_json_array(v, 1).unwrap(),
            )
        })
        .collect()
}

pub fn depth_helper_v2(json: &serde_json::Value) -> Vec<(f64, f64)> {
    json.as_array()
        .unwrap()
        .iter()
        .map(|v| {
            let v = v.as_array().unwrap();
            (v[0].as_f64().unwrap(), v[1].as_f64().unwrap())
        })
        .collect()
}

fn is_graceful_shutdown(market: &Arc<Mutex<dyn Market + Send>>) -> bool {
    *market
        .lock()
        .unwrap()
        .get_spine()
        .graceful_shutdown
        .lock()
        .unwrap()
}

fn update(market: Arc<Mutex<dyn Market + Send>>) {
    let market_is_poloniex = market.lock().unwrap().get_spine().name == "poloniex";
    let market_is_ftx = market.lock().unwrap().get_spine().name == "ftx";
    let market_is_gemini = market.lock().unwrap().get_spine().name == "gemini";

    let channels = market.lock().unwrap().get_spine().channels.clone();
    let exchange_pairs: Vec<String> = market
        .lock()
        .unwrap()
        .get_spine()
        .get_exchange_pairs()
        .keys()
        .cloned()
        .collect();
    let exchange_pairs_dummy = vec!["dummy".to_string()];

    for channel in channels {
        let channel_is_ticker = matches!(channel, MarketChannels::Ticker);

        // Channel Ticker of Poloniex has different subscription system
        // - we can subscribe only for all exchange pairs together,
        // thus, we need to subscribe to a channel only once.
        // -----------------------------------------------------------------------------
        // Ticker channel of market FTX is not implemented, because it has no useful info.
        // Instead of websocket, we get needed info by REST API.
        // At the same time, REST API sends us info about all pairs at once,
        // so we don't need to request info about specific pairs solely.
        let exchange_pairs = if channel_is_ticker && (market_is_poloniex || market_is_ftx) {
            &exchange_pairs_dummy
        } else {
            &exchange_pairs
        };

        let do_rest_api = channel_is_ticker && (market_is_ftx || market_is_gemini);
        let do_websocket = !(market_is_ftx && channel_is_ticker) || market_is_gemini;

        for exchange_pair in exchange_pairs {
            if is_graceful_shutdown(&market) {
                return;
            }

            if do_rest_api {
                let market_2 = Arc::clone(&market);
                let pair = exchange_pair.to_string();

                let thread_name = format!(
                    "fn: subscribe_channel, market: {}, pair: {}, channel: {:?}, REST API",
                    market.lock().unwrap().get_spine().name,
                    pair,
                    channel
                );

                let thread = thread::Builder::new()
                    .name(thread_name)
                    .spawn(move || loop {
                        if is_graceful_shutdown(&market_2) {
                            return;
                        }

                        let update_ticker_result =
                            market_2.lock().unwrap().update_ticker(pair.clone());
                        if update_ticker_result.is_some() {
                            // if success
                            let rest_timeout_sec =
                                market_2.lock().unwrap().get_spine().rest_timeout_sec;
                            thread::sleep(time::Duration::from_secs(rest_timeout_sec));
                        } else {
                            // if error
                            thread::sleep(time::Duration::from_millis(10000));
                        }
                    })
                    .unwrap();
                thread::sleep(time::Duration::from_millis(12000));
                market.lock().unwrap().get_spine().tx.send(thread).unwrap();
            }

            if do_websocket {
                let market_2 = Arc::clone(&market);
                let pair = exchange_pair.to_string();

                let thread_name = format!(
                    "fn: subscribe_channel, market: {}, pair: {}, channel: {:?}",
                    market.lock().unwrap().get_spine().name,
                    pair,
                    channel
                );

                let thread = thread::Builder::new()
                    .name(thread_name)
                    .spawn(move || loop {
                        if is_graceful_shutdown(&market_2) {
                            return;
                        }

                        subscribe_channel(Arc::clone(&market_2), pair.clone(), channel);
                        thread::sleep(time::Duration::from_millis(10000));
                    })
                    .unwrap();
                thread::sleep(time::Duration::from_millis(12000));
                market.lock().unwrap().get_spine().tx.send(thread).unwrap();
            }
        }
    }
}

pub trait Market {
    fn get_spine(&self) -> &MarketSpine;
    fn get_spine_mut(&mut self) -> &mut MarketSpine;
    fn make_pair(&self, pair: (&str, &str)) -> String {
        match self.get_spine().name.as_str() {
            "hitbtc" | "bybit" | "gemini" => {
                (self.get_spine().get_masked_value(pair.0).to_string()
                    + self.get_spine().get_masked_value(pair.1))
                .to_uppercase()
            }
            "binance" | "huobi" => (self.get_spine().get_masked_value(pair.0).to_string()
                + self.get_spine().get_masked_value(pair.1))
            .to_lowercase(),
            "coinbase" | "okcoin" | "kucoin" | "ftx" => {
                (self.get_spine().get_masked_value(pair.0).to_string()
                    + "-"
                    + self.get_spine().get_masked_value(pair.1))
                .to_uppercase()
            }
            _ => panic!("fn make_pair is not implemented"),
        }
    }

    fn add_exchange_pair(
        &mut self,
        exchange_pair: ExchangePair,
        repositories: Option<MarketRepositoriesByMarketValue>,
        ws_channels_holder: &WsChannelsHolderHashMap,
    ) {
        let pair_string = self.make_pair(get_pair_ref(&exchange_pair.pair));
        self.get_spine_mut().add_exchange_pair(
            pair_string,
            exchange_pair,
            repositories,
            ws_channels_holder,
        );
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

    fn get_pair_text_view(&self, pair: String) -> String {
        let pair_text_view = if self.get_spine().name == "poloniex" {
            let pair_tuple = self.get_spine().get_pairs().get(&pair).unwrap();
            format!("{:?}", pair_tuple)
        } else {
            pair
        };

        pair_text_view
    }

    fn update_ticker(&mut self, _pair: String) -> Option<()> {
        panic!("fn update_ticker is not implemented.");
    }

    fn parse_ticker_json(&mut self, pair: String, json: serde_json::Value) -> Option<()>;
    fn parse_ticker_json_inner(&mut self, pair: String, volume: f64) {
        let pair_text_view = self.get_pair_text_view(pair.clone());

        info!(
            "new {} ticker on {} with volume: {}",
            pair_text_view,
            self.get_spine().name,
            volume,
        );

        self.get_spine_mut().set_total_volume(&pair, volume);
    }

    fn parse_last_trade_json(&mut self, pair: String, json: serde_json::Value) -> Option<()>;
    fn parse_last_trade_json_inner(
        &mut self,
        pair: String,
        last_trade_volume: f64,
        last_trade_price: f64,
    ) {
        let pair_text_view = self.get_pair_text_view(pair.clone());

        info!(
            "new {} trade on {} with volume: {}, price: {}",
            pair_text_view,
            self.get_spine().name,
            last_trade_volume,
            last_trade_price,
        );

        self.get_spine_mut()
            .set_last_trade_volume(&pair, last_trade_volume);
        self.get_spine_mut()
            .set_last_trade_price(&pair, last_trade_price);
    }

    fn parse_depth_json(&mut self, pair: String, json: serde_json::Value) -> Option<()>;
    fn parse_depth_json_inner(
        &mut self,
        pair: String,
        asks: Vec<(f64, f64)>,
        bids: Vec<(f64, f64)>,
    ) {
        let pair_text_view = self.get_pair_text_view(pair.clone());

        let mut ask_sum: f64 = 0.0;
        for (_price, size) in asks {
            ask_sum += size;
        }
        self.get_spine_mut().set_total_ask(&pair, ask_sum);

        let mut bid_sum: f64 = 0.0;
        for (price, size) in bids {
            bid_sum += size * price;
        }
        self.get_spine_mut().set_total_bid(&pair, bid_sum);

        info!(
            "new {} book on {} with ask_sum: {}, bid_sum: {}",
            pair_text_view,
            self.get_spine().name,
            ask_sum,
            bid_sum
        );
    }
}

#[cfg(test)]
mod test {
    use crate::config_scheme::config_scheme::ConfigScheme;
    use crate::config_scheme::market_config::MarketConfig;
    use crate::config_scheme::repositories_prepared::RepositoriesPrepared;
    use crate::worker::helper_functions::get_pair_ref;
    use crate::worker::market_helpers::conversion_type::ConversionType;
    use crate::worker::market_helpers::exchange_pair::ExchangePair;
    use crate::worker::market_helpers::market::{market_factory, update, Market};
    use crate::worker::market_helpers::market_channels::MarketChannels;
    use crate::worker::market_helpers::market_spine::test::make_spine;
    use crate::worker::worker::test::check_threads;
    use ntest::timeout;
    use std::sync::mpsc::Receiver;
    use std::sync::{Arc, Mutex};
    use std::thread::JoinHandle;

    fn make_market(
        market_name: Option<&str>,
    ) -> (Arc<Mutex<dyn Market + Send>>, Receiver<JoinHandle<()>>) {
        let config = ConfigScheme::default();

        let RepositoriesPrepared {
            pair_average_price_repository: _,
            market_repositories,
            ws_channels_holder,
            pair_average_price: _,
        } = RepositoriesPrepared::make(&config);

        let (market_spine, rx) = make_spine(market_name);
        let market_name = market_spine.name.clone();
        let market = market_factory(
            market_spine,
            config.market.exchange_pairs,
            market_repositories.map(|mut v| v.remove(&market_name).unwrap()),
            &ws_channels_holder,
        );

        (market, rx)
    }

    #[test]
    fn test_add_exchange_pair() {
        let (market, _) = make_market(None);
        let market_name = market.lock().unwrap().get_spine().name.clone();

        let pair_tuple = ("some_coin_1".to_string(), "some_coin_2".to_string());
        let conversion_type = ConversionType::Crypto;
        let exchange_pair = ExchangePair {
            pair: pair_tuple.clone(),
            conversion: conversion_type,
        };

        let mut config = ConfigScheme::default();
        config.market.exchange_pairs = vec![exchange_pair.clone()];

        let RepositoriesPrepared {
            pair_average_price_repository: _,
            market_repositories,
            ws_channels_holder,
            pair_average_price: _,
        } = RepositoriesPrepared::make(&config);

        let pair_string = market
            .lock()
            .unwrap()
            .make_pair(get_pair_ref(&exchange_pair.pair));

        market.lock().unwrap().add_exchange_pair(
            exchange_pair.clone(),
            market_repositories.map(|mut v| {
                v.remove(&market_name)
                    .unwrap()
                    .remove(&exchange_pair.pair)
                    .unwrap()
            }),
            &ws_channels_holder,
        );

        assert!(market
            .lock()
            .unwrap()
            .get_spine()
            .get_exchange_pairs()
            .contains_key(&pair_string));

        assert_eq!(
            market
                .lock()
                .unwrap()
                .get_spine()
                .get_conversions()
                .get(&pair_string)
                .unwrap(),
            &conversion_type
        );

        assert_eq!(
            market
                .lock()
                .unwrap()
                .get_spine()
                .get_pairs()
                .get(&pair_string)
                .unwrap(),
            &pair_tuple
        );
    }

    #[test]
    #[should_panic]
    fn test_market_factory_panic() {
        let (_, _) = make_market(Some("not_existing_market"));
    }

    #[test]
    fn test_market_factory() {
        let (market, _) = make_market(None);

        assert!(market.lock().unwrap().get_spine().arc.is_some());

        let config = MarketConfig::default();
        let exchange_pair_keys: Vec<String> = market
            .lock()
            .unwrap()
            .get_spine()
            .get_exchange_pairs()
            .keys()
            .cloned()
            .collect();
        assert_eq!(exchange_pair_keys.len(), config.exchange_pairs.len());

        for pair in &config.exchange_pairs {
            let pair_string = market.lock().unwrap().make_pair(get_pair_ref(&pair.pair));
            assert!(exchange_pair_keys.contains(&pair_string));
        }
    }

    #[test]
    #[timeout(3000)]
    fn test_perform() {
        let (market, rx) = make_market(None);

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
        let (market, rx) = make_market(None);

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
