use axum::http::HeaderMap;

pub fn header_to_bool(headers: &HeaderMap, key: &str) -> Option<bool> {
    headers.get(key).map(|value| value == "true")
}

pub fn header_to_string(headers: &HeaderMap, key: &str) -> Option<String> {
    headers
        .get(key)
        .and_then(|v| v.to_str().ok().map(String::from))
}

pub fn header_to_float(headers: &HeaderMap, key: &str) -> Option<f64> {
    headers.get(key).and_then(|v| v.to_str().ok()?.parse().ok())
}
