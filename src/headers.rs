use axum::http::HeaderMap;

pub trait TypedHeaderValues {
    fn bool(&self, key: &str) -> Option<bool>;
    fn string(&self, key: &str) -> Option<String>;
    fn float(&self, key: &str) -> Option<f64>;
}

impl TypedHeaderValues for HeaderMap {
    fn bool(&self, key: &str) -> Option<bool> {
        self.get(key).map(|v| v == "true")
    }

    fn string(&self, key: &str) -> Option<String> {
        self.get(key).and_then(|v| v.to_str().ok().map(String::from))
    }

    fn float(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(|v| v.to_str().ok()?.parse().ok())
    }
}
