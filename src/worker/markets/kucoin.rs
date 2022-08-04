use crate::worker::market_helpers::market::{depth_helper_v1, Market};
use crate::worker::market_helpers::market_channels::ExternalMarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use tokio::time::{sleep, Duration};

pub struct Kucoin {
    pub spine: MarketSpine,
}

impl Kucoin {
    async fn get_token_inner() -> Result<String, String> {
        let response = Client::new()
            .post("https://api.kucoin.com/api/v1/bullet-private")
            .send()
            .await;

        let response = response.map_err(|e| e.to_string())?;
        let response = response.text().await.map_err(|e| e.to_string())?;
        let json: serde_json::Value = serde_json::from_str(&response).map_err(|e| e.to_string())?;
        println!("json: {:#?}", json);

        let object = json
            .as_object()
            .ok_or("Json must be an object.".to_string())?;
        let object = object
            .get("data")
            .ok_or("Json object must have \"data\" key.".to_string())?;
        let object = object
            .as_object()
            .ok_or("\"data\" must be an object.".to_string())?;

        let token = object
            .get("token")
            .ok_or("\"data\" object must have \"token\" key.".to_string())?;
        let token = token
            .as_str()
            .ok_or("\"token\" must be a string.".to_string())?;

        Ok(token.to_string())
    }

    async fn get_token() -> String {
        loop {
            match Self::get_token_inner().await {
                Ok(token) => return token,
                Err(e) => {
                    error!("api.kucoin.com: Get token error: {}", e);
                    sleep(Duration::from_millis(10000)).await;
                }
            }
        }
    }
}

#[async_trait]
impl Market for Kucoin {
    fn get_spine(&self) -> &MarketSpine {
        &self.spine
    }

    fn get_spine_mut(&mut self) -> &mut MarketSpine {
        &mut self.spine
    }

    fn get_channel_text_view(&self, channel: ExternalMarketChannels) -> String {
        match channel {
            ExternalMarketChannels::Ticker => "market/snapshot",
            ExternalMarketChannels::Trades => "/spotMarket/tradeOrders",
            ExternalMarketChannels::Book => "spotMarket/level2Depth50",
        }
        .to_string()
    }

    async fn get_websocket_url(&self, _pair: &str, _channel: ExternalMarketChannels) -> String {
        let token = Self::get_token().await;

        format!("wss://ws-api.kucoin.com/endpoint?token={}", token)
    }

    fn get_websocket_on_open_msg(
        &self,
        pair: &str,
        channel: ExternalMarketChannels,
    ) -> Option<String> {
        let res = if let ExternalMarketChannels::Trades = channel {
            // format!(
            //     "{{\"type\": \"subscribe\", \"topic\": \"/{}\", \"data\": {{\"symbol\": \"{}\"}} }}",
            //     self.get_channel_text_view(channel),
            //     pair,
            // )

            json!(
                {
                    "type": "subscribe",
                    "topic": self.get_channel_text_view(channel),
                    "channelType": "private",
                    "privateChannel": true,
                    "data": {
                        "symbol": pair,
                    }
                }
            )
            .to_string()
        } else {
            format!(
                "{{\"type\": \"subscribe\", \"topic\": \"/{}:{}\"}}",
                self.get_channel_text_view(channel),
                pair,
            )
        };

        Some(res)
    }

    async fn parse_ticker_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let object = json.as_object()?;
        let object = object.get("data")?.as_object()?;
        let object = object.get("data")?.as_object()?;

        let volume: f64 = object.get("volValue")?.as_f64()?;
        self.parse_ticker_json_inner(pair, volume).await;

        Some(())
    }

    /// TODO: Implement
    async fn parse_last_trade_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        println!("fn parse_last_trade_json(): pair: {}, json: {}", pair, json);

        None
    }

    async fn parse_depth_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let object = json.as_object()?;
        let object = object.get("data")?.as_object()?;
        let asks = object.get("asks")?;
        let bids = object.get("bids")?;

        let asks = depth_helper_v1(asks);
        let bids = depth_helper_v1(bids);
        self.parse_depth_json_inner(pair, asks, bids);

        Some(())
    }
}
