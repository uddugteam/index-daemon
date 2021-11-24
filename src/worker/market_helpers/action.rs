#[derive(Debug)]
pub enum Action {
    SubscribeTickerTradesDepth { pair: String, delay: u64 },
}
