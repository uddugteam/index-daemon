#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum JsonRpcId {
    Int(i64),
    Float(f64),
    Str(String),
}

#[derive(Serialize)]
pub struct JsonRpcErr {
    pub code: i64,
    pub message: String,
}

#[derive(Deserialize)]
pub struct JsonRpcRequest {
    pub id: Option<JsonRpcId>,
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum JsonRpcResponse {
    Succ {
        id: Option<JsonRpcId>,
        result: serde_json::Value,
    },
    Err {
        id: Option<JsonRpcId>,
        result: JsonRpcErr,
    },
}
