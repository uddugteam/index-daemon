#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum JsonRpcId {
    Int(i64),
    Float(f64),
    Str(String),
}

#[derive(Serialize, Deserialize)]
pub struct JsonRpcErr {
    pub code: i64,
    pub message: String,
}

#[derive(Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub id: Option<JsonRpcId>,
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
pub struct JsonRpcResponseSucc {
    pub id: Option<JsonRpcId>,
    pub result: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
pub struct JsonRpcResponseErr {
    pub id: Option<JsonRpcId>,
    pub result: JsonRpcErr,
}
