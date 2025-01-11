use serde::{Deserialize, Serialize};

use crate::entities::MetricsWithinInterval;

#[derive(Deserialize)]
pub struct CreateShortUrl {
    pub url: String,
}

#[derive(Serialize)]
pub struct ShortUrlCreated {
    pub id: String,
}

#[derive(Deserialize)]
pub struct Signup {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct Login {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct MetricsRequest {
    pub measuring_interval_minutes: u8,
}

#[derive(Serialize)]
pub struct MetricsResponse {
    pub metrics: Vec<MetricsWithinInterval>,
}
