#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum JsonRpcId {
    Int(i64),
    Float(f64),
    Str(String),
}
impl Clone for JsonRpcId {
    fn clone(&self) -> Self {
        match self {
            Self::Int(x) => Self::Int(*x),
            Self::Float(x) => Self::Float(*x),
            Self::Str(s) => Self::Str(s.to_string()),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct JsonRpcErr {
    pub code: i64,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
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
