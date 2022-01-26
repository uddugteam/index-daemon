pub fn get_pair_ref(pair: &(String, String)) -> (&str, &str) {
    (pair.0.as_str(), pair.1.as_str())
}

pub fn strip_usd(pair: &(String, String)) -> Option<String> {
    match get_pair_ref(pair) {
        ("USD", coin) | (coin, "USD") => {
            // good pair (coin-USD)
            Some(coin.to_string())
        }
        _ => {
            // bad pair (coin-coin)
            None
        }
    }
}

pub fn add_jsonrpc_version(response: &mut String) {
    let mut value: serde_json::Value = serde_json::from_str(response).unwrap();
    let object = value.as_object_mut().unwrap();
    object.insert(
        "jsonrpc".to_string(),
        serde_json::Value::from("2.0".to_string()),
    );

    *response = value.to_string();
}