use crate::repository::repositories::{
    MarketRepositoriesByMarketValue, MarketRepositoriesByPairTuple,
};
use crate::worker::helper_functions::get_pair_ref;
use crate::worker::market_helpers::market_channels::ExternalMarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;
use crate::worker::market_helpers::percent_change::PercentChangeByInterval;
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
use crate::worker::network_helpers::ws_server::holders::helper_functions::HolderHashMap;
use crate::worker::network_helpers::ws_server::ws_channels::WsChannels;
use async_trait::async_trait;
use futures::FutureExt;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

pub async fn market_factory(
    mut spine: MarketSpine,
    exchange_pairs: Vec<(String, String)>,
    repositories: Option<MarketRepositoriesByPairTuple>,
    percent_change_holder: &HolderHashMap<PercentChangeByInterval>,
    percent_change_interval_sec: u64,
    ws_channels_holder: &HolderHashMap<WsChannels>,
) -> Arc<Mutex<dyn Market + Send + Sync>> {
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

    let market: Arc<Mutex<dyn Market + Send + Sync>> = match spine.name.as_ref() {
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

    for exchange_pair in exchange_pairs {
        market.lock().await.add_exchange_pair(
            exchange_pair.clone(),
            repositories.remove(&exchange_pair),
            percent_change_holder,
            percent_change_interval_sec,
            ws_channels_holder,
        );
    }

    market
}

/// Establishes websocket connection with market (subscribes to the channel with pair)
/// and calls lambda (param "callback" of SocketHelper constructor) when gets message from market
pub async fn subscribe_channel(
    market: Arc<Mutex<dyn Market + Send + Sync>>,
    pair: String,
    channel: ExternalMarketChannels,
) {
    trace!(
        "called subscribe_channel(). Market: {}, pair: {}, channel: {:?}",
        market.lock().await.get_spine().name,
        pair,
        channel,
    );

    let url = market.lock().await.get_websocket_url(&pair, channel).await;

    let on_open_msg = market
        .lock()
        .await
        .get_websocket_on_open_msg(&pair, channel);

    let ws_client = WsClient::new(url, on_open_msg, pair, market, channel);
    ws_client.run().await;
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

pub async fn is_graceful_shutdown(market: &Arc<Mutex<dyn Market + Send + Sync>>) -> bool {
    market
        .lock()
        .await
        .get_spine()
        .graceful_shutdown
        .get()
        .await
}

pub async fn market_update(market: Arc<Mutex<dyn Market + Send + Sync>>) {
    let market_name = market.lock().await.get_spine().name.to_string();

    match market_name.as_str() {
        "ftx" => Ftx::update(market).await,
        "poloniex" => Poloniex::update(market).await,
        _ => update(market).await,
    }
}

async fn update(market: Arc<Mutex<dyn Market + Send + Sync>>) {
    trace!(
        "called Market::update(). Market: {}",
        market.lock().await.get_spine().name
    );

    let mut futures = Vec::new();

    let channels = market.lock().await.get_spine().channels.clone();
    let exchange_pairs: Vec<String> = market
        .lock()
        .await
        .get_spine()
        .get_exchange_pairs()
        .keys()
        .cloned()
        .collect();

    let mut sleep_seconds = 0;
    for channel in channels {
        for exchange_pair in &exchange_pairs {
            if is_graceful_shutdown(&market).await {
                return;
            }

            let market_2 = Arc::clone(&market);
            let pair = exchange_pair.to_string();

            let future = do_websocket(market_2, pair, channel, sleep_seconds);
            futures.push(future.boxed());

            sleep_seconds += 12;
        }
    }

    futures::future::join_all(futures).await;
}

pub async fn do_rest_api(
    market: Arc<Mutex<dyn Market + Send + Sync>>,
    pair: String,
    sleep_seconds: u64,
) {
    // Sleep before await (sleep before subscribe)
    sleep(Duration::from_secs(sleep_seconds)).await;

    loop {
        if is_graceful_shutdown(&market).await {
            return;
        }

        let update_ticker_result = market.lock().await.update_ticker(pair.clone()).await;

        if update_ticker_result.is_some() {
            // if success
            let rest_timeout_sec = market.lock().await.get_spine().rest_timeout_sec;

            sleep(Duration::from_secs(rest_timeout_sec)).await;
        } else {
            // if error
            sleep(Duration::from_secs(10)).await;
        }
    }
}

pub async fn do_websocket(
    market: Arc<Mutex<dyn Market + Send + Sync>>,
    pair: String,
    channel: ExternalMarketChannels,
    sleep_seconds: u64,
) {
    // Sleep before await (sleep before subscribe)
    sleep(Duration::from_secs(sleep_seconds)).await;

    loop {
        if is_graceful_shutdown(&market).await {
            return;
        }

        subscribe_channel(Arc::clone(&market), pair.clone(), channel).await;
        sleep(Duration::from_secs(10)).await;
    }
}

#[async_trait]
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
        exchange_pair: (String, String),
        repositories: Option<MarketRepositoriesByMarketValue>,
        percent_change_holder: &HolderHashMap<PercentChangeByInterval>,
        percent_change_interval_sec: u64,
        ws_channels_holder: &HolderHashMap<WsChannels>,
    ) {
        let pair_string = self.make_pair(get_pair_ref(&exchange_pair));
        self.get_spine_mut().add_exchange_pair(
            pair_string,
            exchange_pair,
            repositories,
            percent_change_holder,
            percent_change_interval_sec,
            ws_channels_holder,
        );
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

    fn get_channel_text_view(&self, channel: ExternalMarketChannels) -> String;
    async fn get_websocket_url(&self, pair: &str, channel: ExternalMarketChannels) -> String;
    fn get_websocket_on_open_msg(
        &self,
        pair: &str,
        channel: ExternalMarketChannels,
    ) -> Option<String>;

    fn get_pair_text_view(&self, pair: String) -> String {
        let pair_text_view = if self.get_spine().name == "poloniex" {
            let pair_tuple = self.get_spine().get_pairs().get(&pair).unwrap();
            format!("{:?}", pair_tuple)
        } else {
            pair
        };

        pair_text_view
    }

    async fn update_ticker(&mut self, _pair: String) -> Option<()> {
        panic!("fn update_ticker is not implemented.");
    }

    async fn parse_ticker_json(&mut self, pair: String, json: serde_json::Value) -> Option<()>;
    async fn parse_ticker_json_inner(&mut self, pair: String, volume: f64) {
        let pair_text_view = self.get_pair_text_view(pair.clone());

        info!(
            "new {} ticker on {} with volume: {}",
            pair_text_view,
            self.get_spine().name,
            volume,
        );

        self.get_spine_mut().set_total_volume(&pair, volume).await;
    }

    async fn parse_last_trade_json(&mut self, pair: String, json: serde_json::Value) -> Option<()>;
    async fn parse_last_trade_json_inner(
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
            .set_last_trade_price(&pair, last_trade_price)
            .await;
    }

    async fn parse_depth_json(&mut self, pair: String, json: serde_json::Value) -> Option<()>;
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
    use crate::worker::market_helpers::market::{market_factory, market_update, Market};
    use crate::worker::market_helpers::market_channels::ExternalMarketChannels;
    use crate::worker::market_helpers::market_spine::test::make_spine;
    use crate::worker::worker::test::check_threads;
    use ntest::timeout;
    use std::sync::mpsc::Receiver;
    use std::sync::Arc;
    use std::thread::JoinHandle;
    use tokio::sync::Mutex;

    fn make_market(
        market_name: Option<&str>,
    ) -> (
        Arc<Mutex<dyn Market + Send + Sync>>,
        Receiver<JoinHandle<()>>,
    ) {
        let config = ConfigScheme::default();

        let RepositoriesPrepared {
            index_price_repository: _,
            pair_average_price_repositories: _,
            market_repositories,
            percent_change_holder,
            ws_channels_holder,
            index_price: _,
            pair_average_price: _,
        } = RepositoriesPrepared::make(&config);

        let (market_spine, rx) = make_spine(market_name);
        let market_name = market_spine.name.clone();
        let market = market_factory(
            market_spine,
            config.market.exchange_pairs,
            market_repositories.map(|mut v| v.remove(&market_name).unwrap()),
            &percent_change_holder,
            config.service.percent_change_interval_sec,
            &ws_channels_holder,
        )
        .await;

        (market, rx)
    }

    #[test]
    fn test_add_exchange_pair() {
        let (market, _) = make_market(None);
        let market_name = market.lock().await.get_spine().name.clone();

        let pair_tuple = ("some_coin_1".to_string(), "some_coin_2".to_string());
        let exchange_pair = pair_tuple.clone();

        let mut config = ConfigScheme::default();
        config.market.exchange_pairs = vec![exchange_pair.clone()];

        let RepositoriesPrepared {
            index_price_repository: _,
            pair_average_price_repositories: _,
            market_repositories,
            percent_change_holder,
            ws_channels_holder,
            index_price: _,
            pair_average_price: _,
        } = RepositoriesPrepared::make(&config);

        let pair_string = market.lock().await.make_pair(get_pair_ref(&exchange_pair));

        market.lock().await.add_exchange_pair(
            exchange_pair.clone(),
            market_repositories.map(|mut v| {
                v.remove(&market_name)
                    .unwrap()
                    .remove(&exchange_pair)
                    .unwrap()
            }),
            &percent_change_holder,
            config.service.percent_change_interval_sec,
            &ws_channels_holder,
        );

        assert!(market
            .lock()
            .await
            .get_spine()
            .get_exchange_pairs()
            .contains_key(&pair_string));

        assert_eq!(
            market
                .lock()
                .await
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

        let config = MarketConfig::default();
        let exchange_pair_keys: Vec<String> = market
            .lock()
            .await
            .get_spine()
            .get_exchange_pairs()
            .keys()
            .cloned()
            .collect();
        assert_eq!(exchange_pair_keys.len(), config.exchange_pairs.len());

        for pair in &config.exchange_pairs {
            let pair_string = market.lock().await.make_pair(get_pair_ref(pair));
            assert!(exchange_pair_keys.contains(&pair_string));
        }
    }

    #[test]
    #[timeout(120000)]
    /// TODO: Refactor (not always working)
    fn test_update() {
        let (market, rx) = make_market(None);

        let channels = ExternalMarketChannels::get_all();
        let exchange_pairs: Vec<String> = market
            .lock()
            .await
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
                    market.lock().await.get_spine().name,
                    pair,
                    channel
                );
                thread_names.push(thread_name);
            }
        }

        market_update(market);
        check_threads(thread_names, rx);
    }
}
