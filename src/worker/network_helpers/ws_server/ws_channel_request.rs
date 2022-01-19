#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "method", content = "params")]
pub enum WsChannelRequest {
    CoinAveragePrice {
        coins: Vec<String>,
        frequency_ms: u64,
    },
    Unsubscribe,
}
