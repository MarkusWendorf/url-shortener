use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CreateShortUrl {
    pub url: String,
}

#[derive(Serialize)]
pub struct ShortUrlCreated {
    pub id: String,
}
