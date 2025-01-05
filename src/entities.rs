#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub email: String,
}

#[derive(Debug)]
pub struct MetricsWithinInterval {
    pub count: i64,
    pub unique_count: i64,
}
