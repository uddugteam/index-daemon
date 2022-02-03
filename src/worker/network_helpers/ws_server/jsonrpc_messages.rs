#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum JsonRpcId {
    Int(i64),
    Float(f64),
    Str(String),
}

#[derive(Deserialize)]
pub struct JsonRpcRequest {
    pub id: Option<JsonRpcId>,
    pub method: String,
    pub params: serde_json::Value,
}
