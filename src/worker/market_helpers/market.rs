use crate::worker::market_helpers::market_spine::MarketSpine;
// use crate::worker::markets::binance::Binance;
use crate::worker::markets::bitfinex::Bitfinex;
// use crate::worker::markets::bittrex::Bittrex;
// use crate::worker::markets::poloniex::Poloniex;
use crate::worker::market_helpers::conversion_type::ConversionType;
use crate::worker::network_helpers::socket_helper::SocketHelper;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

pub fn market_factory(spine: MarketSpine) -> Arc<Mutex<dyn Market + Send>> {
    let market: Arc<Mutex<dyn Market + Send>> = match spine.name.as_ref() {
        // "binance" => Box::new(RefCell::new(Binance { spine, arc: None })),
        "bitfinex" => Arc::new(Mutex::new(Bitfinex { spine })),
        // "bittrex" => Box::new(RefCell::new(Bittrex { spine, arc: None })),
        // "poloniex" => Box::new(RefCell::new(Poloniex { spine, arc: None })),
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
    channel: &str,
    url: String,
    on_open_msg: String,
) {
    // println!("called subscribe_channel()");

    let socker_helper =
        SocketHelper::new(
            url,
            on_open_msg,
            pair,
            |pair: String, info: String| match channel {
                "ticker" => market.lock().unwrap().parse_ticker_info__socket(pair, info),
                "trades" => market
                    .lock()
                    .unwrap()
                    .parse_last_trade_info__socket(pair, info),
                "book" => market.lock().unwrap().parse_depth_info__socket(pair, info),
                _ => println!("Error: channel not supported."),
            },
        );
    socker_helper.start();
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

    fn get_websocket_url(&self, pair: &str, channel: &str) -> String;
    fn get_websocket_on_open_msg(&self, pair: &str, channel: &str) -> String;

    // TODO: Replace `delay` constants with parameters
    fn update(&mut self) {
        // println!("called Market::update()");

        self.get_spine_mut().socket_enabled = true;

        let channels = ["ticker", "trades", "book"];

        for exchange_pair in self.get_spine().get_exchange_pairs() {
            for channel in channels {
                let market = Arc::clone(self.get_spine().arc.as_ref().unwrap());
                let pair = exchange_pair.0.to_string();
                let url = self.get_websocket_url(&pair, channel);
                let on_open_msg = self.get_websocket_on_open_msg(&pair, channel);
                let thread = thread::spawn(move || loop {
                    subscribe_channel(
                        Arc::clone(&market),
                        pair.clone(),
                        channel,
                        url.clone(),
                        on_open_msg.clone(),
                    );
                    thread::sleep(time::Duration::from_millis(10000));
                });
                thread::sleep(time::Duration::from_millis(3000));

                self.get_spine().tx.send(thread).unwrap();
            }
        }
    }

    fn perform(&mut self) {
        println!("called Market::perform()");

        self.get_spine_mut().refresh_capitalization();
        self.update();
    }
    fn parse_ticker_info__socket(&mut self, pair: String, info: String);
    fn parse_last_trade_info__socket(&mut self, pair: String, info: String);
    fn parse_depth_info__socket(&mut self, pair: String, info: String);
}
