use serde::Serialize;
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct MetricsWithinInterval {
    #[serde(with = "time::serde::timestamp::milliseconds")]
    pub timestamp: OffsetDateTime,
    pub count: i64,
    pub unique_count: i64,
}
