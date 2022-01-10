use crate::worker::market_helpers::exchange_pair::ExchangePair;
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;
use crate::worker::markets::binance::Binance;
use crate::worker::markets::bitfinex::Bitfinex;
use crate::worker::markets::bybit::Bybit;
use crate::worker::markets::coinbase::Coinbase;
use crate::worker::markets::gateio::Gateio;
use crate::worker::markets::gemini::Gemini;
use crate::worker::markets::hitbtc::Hitbtc;
use crate::worker::markets::huobi::Huobi;
use crate::worker::markets::kraken::Kraken;
use crate::worker::markets::kucoin::Kucoin;
use crate::worker::markets::okcoin::Okcoin;
use crate::worker::markets::poloniex::Poloniex;
use crate::worker::network_helpers::socket_helper::SocketHelper;
use chrono::Utc;
use regex::Regex;
use rustc_serialize::json::{Array, Json, Object};
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
        "huobi" | "hitbtc" | "okcoin" | "gateio" | "kucoin" => vec![("USD", "USDT")],
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

    let socker_helper = SocketHelper::new(url, on_open_msg, pair, |pair: String, info: String| {
        // This block is needed for Huobi::parse_last_trade_json()
        let json = if let Ok(json) = Json::from_str(&info) {
            Some(json)
        } else {
            // Error: invalid number (integer is too long).
            // Since we don't need its real value, we can to replace it with fake, but valid number.
            let regex_too_big_integer = Regex::new("^(.*)(\\D)(?P<n>\\d{18,})(\\D)(.*)$").unwrap();
            let too_big_integer = regex_too_big_integer.replace_all(&info, "$n").to_string();

            let info = info.replace(&too_big_integer, "123456");

            Json::from_str(&info).ok()
        };
        if let Some(json) = json {
            // This match returns value, but we shouldn't use it
            match channel {
                MarketChannels::Ticker => market.lock().unwrap().parse_ticker_json(pair, json),
                MarketChannels::Trades => market.lock().unwrap().parse_last_trade_json(pair, json),
                MarketChannels::Book => market.lock().unwrap().parse_depth_json(pair, json),
            };
        }
    });
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

pub fn depth_helper_v1(json: &Json) -> Vec<(f64, f64)> {
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

fn update(market: Arc<Mutex<dyn Market + Send>>) {
    let market_is_poloniex = market.lock().unwrap().get_spine().name == "poloniex";

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
        // Channel Ticker of Poloniex has different subscription system
        // - we can subscribe only for all exchange pairs together,
        // thus, we need to subscribe to a channel only once
        let exchange_pairs = if market_is_poloniex && matches!(channel, MarketChannels::Ticker) {
            &exchange_pairs_dummy
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
        match self.get_spine().name.as_str() {
            "hitbtc" | "bybit" | "gemini" => {
                (self.get_spine().get_masked_value(pair.0).to_string()
                    + self.get_spine().get_masked_value(pair.1))
                .to_uppercase()
            }
            "binance" | "huobi" => (self.get_spine().get_masked_value(pair.0).to_string()
                + self.get_spine().get_masked_value(pair.1))
            .to_lowercase(),
            "coinbase" | "okcoin" | "kucoin" => {
                (self.get_spine().get_masked_value(pair.0).to_string()
                    + "-"
                    + self.get_spine().get_masked_value(pair.1))
                .to_uppercase()
            }
            _ => panic!("fn make_pair is not implemented"),
        }
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

    fn get_pair_text_view(&self, pair: String) -> String {
        let pair_text_view = if self.get_spine().name == "poloniex" {
            let pair_tuple = self.get_spine().get_pairs().get(&pair).unwrap();
            format!("{:?}", pair_tuple)
        } else {
            pair
        };

        pair_text_view
    }

    fn parse_ticker_json(&mut self, pair: String, json: Json) -> Option<()>;
    fn parse_ticker_json_inner(&mut self, pair: String, volume: f64) {
        let pair_text_view = self.get_pair_text_view(pair.clone());

        info!(
            "new {} ticker on {} with volume: {}",
            pair_text_view,
            self.get_spine().name,
            volume,
        );

        let conversion_coef: f64 = self.get_spine().get_conversion_coef(&pair);
        self.get_spine_mut()
            .set_total_volume(&pair, volume * conversion_coef);
    }

    fn parse_last_trade_json(&mut self, pair: String, json: Json) -> Option<()>;
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

        let conversion_coef: f64 = self.get_spine().get_conversion_coef(&pair);
        self.get_spine_mut()
            .set_last_trade_volume(&pair, last_trade_volume);
        self.get_spine_mut()
            .set_last_trade_price(&pair, last_trade_price * conversion_coef);
    }

    fn parse_depth_json(&mut self, pair: String, json: Json) -> Option<()>;
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
        bid_sum *= self.get_spine().get_conversion_coef(&pair);
        self.get_spine_mut().set_total_bid(&pair, bid_sum);

        info!(
            "new {} book on {} with ask_sum: {}, bid_sum: {}",
            pair_text_view,
            self.get_spine().name,
            ask_sum,
            bid_sum
        );

        let timestamp = Utc::now();
        self.get_spine_mut()
            .get_exchange_pairs_mut()
            .get_mut(&pair)
            .unwrap()
            .set_timestamp(timestamp);
    }
}

#[cfg(test)]
mod test {
    use crate::worker::market_helpers::conversion_type::ConversionType;
    use crate::worker::market_helpers::exchange_pair::ExchangePair;
    use crate::worker::market_helpers::market::{market_factory, update, Market};
    use crate::worker::market_helpers::market_channels::MarketChannels;
    use crate::worker::market_helpers::market_spine::test::make_spine;
    use crate::worker::worker::test::check_threads;
    use crate::worker::worker::Worker;
    use ntest::timeout;
    use std::sync::mpsc::Receiver;
    use std::sync::{Arc, Mutex};
    use std::thread::JoinHandle;

    fn make_market(
        market_name: Option<&str>,
    ) -> (Arc<Mutex<dyn Market + Send>>, Receiver<JoinHandle<()>>) {
        let exchange_pairs = Worker::make_exchange_pairs(None, None);

        let (market_spine, rx) = make_spine(market_name);
        let market = market_factory(market_spine, exchange_pairs);

        (market, rx)
    }

    #[test]
    fn test_add_exchange_pair() {
        let (market, _) = make_market(None);

        let pair_tuple = ("some_coin_1".to_string(), "some_coin_2".to_string());
        let conversion_type = ConversionType::Crypto;
        let exchange_pair = ExchangePair {
            pair: pair_tuple.clone(),
            conversion: conversion_type,
        };

        let pair_string = market
            .lock()
            .unwrap()
            .make_pair(exchange_pair.get_pair_ref());

        market.lock().unwrap().add_exchange_pair(exchange_pair);

        assert!(market
            .lock()
            .unwrap()
            .get_spine()
            .get_exchange_pairs()
            .get(&pair_string)
            .is_some());

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

        let exchange_pairs = Worker::make_exchange_pairs(None, None);
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
