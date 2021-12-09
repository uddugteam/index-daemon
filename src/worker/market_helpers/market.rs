use crate::worker::market_helpers::conversion_type::ConversionType;
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;
use crate::worker::markets::binance::Binance;
use crate::worker::markets::bitfinex::Bitfinex;
use crate::worker::markets::coinbase::Coinbase;
use crate::worker::network_helpers::socket_helper::SocketHelper;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

pub fn market_factory(spine: MarketSpine) -> Arc<Mutex<dyn Market + Send>> {
    let market: Arc<Mutex<dyn Market + Send>> = match spine.name.as_ref() {
        "binance" => Arc::new(Mutex::new(Binance { spine })),
        "bitfinex" => Arc::new(Mutex::new(Bitfinex { spine })),
        "coinbase" => Arc::new(Mutex::new(Coinbase { spine })),
        _ => panic!("Market not found: {}", spine.name),
    };

    market
        .lock()
        .unwrap()
        .get_spine_mut()
        .set_arc(Arc::clone(&market));

    market
}

fn subscribe_channel(
    market: Arc<Mutex<dyn Market + Send>>,
    pair: String,
    channel: MarketChannels,
    url: String,
    on_open_msg: Option<String>,
) {
    // println!("called subscribe_channel()");

    let socker_helper =
        SocketHelper::new(
            url,
            on_open_msg,
            pair,
            |pair: String, info: String| match channel {
                MarketChannels::Ticker => {
                    market.lock().unwrap().parse_ticker_info__socket(pair, info)
                }
                MarketChannels::Trades => market
                    .lock()
                    .unwrap()
                    .parse_last_trade_info__socket(pair, info),
                MarketChannels::Book => market.lock().unwrap().parse_depth_info__socket(pair, info),
            },
        );
    socker_helper.start();
}

// TODO: Replace `delay` constants with parameters
fn update(market: Arc<Mutex<dyn Market + Send>>) {
    // println!("called update()");

    market.lock().unwrap().get_spine_mut().socket_enabled = true;

    let channels = MarketChannels::get_all();

    let exchange_pairs: Vec<String> = market
        .lock()
        .unwrap()
        .get_spine()
        .get_exchange_pairs()
        .keys()
        .cloned()
        .collect();

    for exchange_pair in exchange_pairs {
        for channel in channels {
            let market_2 = Arc::clone(&market);
            let pair = exchange_pair.to_string();
            let url = market.lock().unwrap().get_websocket_url(&pair, channel);

            let on_open_msg = market
                .lock()
                .unwrap()
                .get_websocket_on_open_msg(&pair, channel);

            let thread = thread::spawn(move || loop {
                subscribe_channel(
                    Arc::clone(&market_2),
                    pair.clone(),
                    channel,
                    url.clone(),
                    on_open_msg.clone(),
                );
                thread::sleep(time::Duration::from_millis(10000));
            });
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

    fn add_exchange_pair(&mut self, pair: (&str, &str), conversion: ConversionType) {
        let pair_string = self.make_pair(pair);
        self.get_spine_mut()
            .add_exchange_pair(pair_string, pair, conversion);
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

    fn get_channel_text_view(&self, channel: MarketChannels) -> String {
        channel.to_string()
    }
    fn get_websocket_url(&self, pair: &str, channel: MarketChannels) -> String;
    fn get_websocket_on_open_msg(&self, pair: &str, channel: MarketChannels) -> Option<String>;

    fn perform(&mut self) {
        println!("called Market::perform()");

        self.get_spine_mut().refresh_capitalization();

        let market = Arc::clone(self.get_spine().arc.as_ref().unwrap());
        let thread = thread::spawn(move || update(market));
        self.get_spine().tx.send(thread).unwrap();
    }
    fn parse_ticker_info__socket(&mut self, pair: String, info: String);
    fn parse_last_trade_info__socket(&mut self, pair: String, info: String);
    fn parse_depth_info__socket(&mut self, pair: String, info: String);
}