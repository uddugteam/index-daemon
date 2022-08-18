use crate::worker::market_helpers::market::{
    depth_helper_v2, do_rest_api, do_websocket, is_graceful_shutdown, Market,
};
use crate::worker::market_helpers::market_channels::ExternalMarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;
use async_trait::async_trait;
use futures::FutureExt;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Ftx {
    pub spine: MarketSpine,
}

impl Ftx {
    pub async fn update(market: Arc<Mutex<dyn Market + Send + Sync>>) {
        info!(
            "called Ftx::update(). Market: {}",
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
        let exchange_pairs_dummy = vec!["dummy".to_string()];

        let mut sleep_seconds = 0;
        for channel in channels {
            let channel_is_ticker = matches!(channel, ExternalMarketChannels::Ticker);

            // Channel "Ticker" of market "FTX" is not implemented, because it has no useful info.
            // Instead of websocket, we get needed info by REST API.
            // At the same time, REST API sends us info about all pairs at once,
            // so we don't need to request info about specific pairs solely.
            let exchange_pairs = if channel_is_ticker {
                &exchange_pairs_dummy
            } else {
                &exchange_pairs
            };

            let to_do_rest_api = channel_is_ticker;
            let to_do_websocket = !channel_is_ticker;

            for exchange_pair in exchange_pairs {
                if is_graceful_shutdown(&market).await {
                    return;
                }

                if to_do_rest_api {
                    let market_2 = Arc::clone(&market);
                    let pair = exchange_pair.to_string();

                    let future = do_rest_api(market_2, pair, sleep_seconds);
                    futures.push(future.boxed());
                }

                if to_do_websocket {
                    let market_2 = Arc::clone(&market);
                    let pair = exchange_pair.to_string();

                    let future = do_websocket(market_2, pair, channel, sleep_seconds);
                    futures.push(future.boxed());
                }

                sleep_seconds += 12;
            }
        }

        futures::future::join_all(futures).await;
    }
}

#[async_trait]
impl Market for Ftx {
    fn get_spine(&self) -> &MarketSpine {
        &self.spine
    }

    fn get_spine_mut(&mut self) -> &mut MarketSpine {
        &mut self.spine
    }

    fn get_channel_text_view(&self, channel: ExternalMarketChannels) -> String {
        match channel {
            ExternalMarketChannels::Ticker => {
                // Ticker channel of market FTX is not implemented, because it has no useful info.
                // Instead of websocket, we get needed info by REST API.
                unreachable!("Ticker channel of market FTX is not implemented, because it has no useful info.");
                // "ticker" 
            },
            ExternalMarketChannels::Trades => "trades",
            ExternalMarketChannels::Book => "orderbook",
        }
        .to_string()
    }

    async fn get_websocket_url(&self, _pair: &str, _channel: ExternalMarketChannels) -> String {
        "wss://ftx.com/ws/".to_string()
    }

    fn get_websocket_on_open_msg(
        &self,
        pair: &str,
        channel: ExternalMarketChannels,
    ) -> Option<String> {
        Some(format!(
            "{{\"op\": \"subscribe\", \"channel\": \"{}\", \"market\": \"{}\"}}",
            self.get_channel_text_view(channel),
            pair,
        ))
    }

    async fn update_ticker(&mut self, _pair: String) -> Option<()> {
        let response = Client::new()
            .get("https://ftx.com/api/markets")
            .send()
            .await;

        let response = response.ok()?;
        let response = response.text().await.ok()?;
        let json: serde_json::Value = serde_json::from_str(&response).ok()?;

        let object = json.as_object()?;
        let array = object.get("result")?.as_array()?;

        for object in array {
            let object = object.as_object()?;

            let pair = object.get("name")?.as_str()?.to_string();
            // Process only needed pairs
            if self.spine.get_exchange_pairs().contains_key(&pair) {
                let volume = object.get("quoteVolume24h")?.as_f64()?;
                self.parse_ticker_json_inner(&pair, volume).await;
            }
        }

        Some(())
    }

    async fn parse_ticker_json(&mut self, _pair: String, _json: serde_json::Value) -> Option<()> {
        // Ticker channel of market FTX is not implemented, because it has no useful info.
        // Instead of websocket, we get needed info by REST API.
        unreachable!(
            "Ticker channel of market FTX is not implemented, because it has no useful info."
        );
    }

    async fn parse_last_trade_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let object = json.as_object()?;
        let array = object.get("data")?.as_array()?;

        for object in array {
            let object = object.as_object()?;

            let mut last_trade_volume: f64 = object.get("size")?.as_f64()?;
            let last_trade_price: f64 = object.get("price")?.as_f64()?;

            let trade_type = object.get("side")?.as_str()?;
            // TODO: Check whether inversion is right
            if trade_type == "sell" {
                // sell
                last_trade_volume *= -1.0;
            } else if trade_type == "buy" {
                // buy
            }

            self.parse_last_trade_json_inner(&pair, last_trade_volume, last_trade_price)
                .await;
        }

        Some(())
    }

    async fn parse_depth_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let object = json.as_object()?;
        let object = object.get("data")?.as_object()?;

        if object.get("action")?.as_str()? == "partial" {
            let asks = object.get("asks")?;
            let bids = object.get("bids")?;

            let asks = depth_helper_v2(asks);
            let bids = depth_helper_v2(bids);
            self.parse_depth_json_inner(&pair, asks, bids);
        }

        Some(())
    }
}
