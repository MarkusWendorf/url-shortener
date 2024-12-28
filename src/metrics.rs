use rand::{thread_rng, Rng};
use tokio::pin;
use tokio_postgres::{binary_copy::BinaryCopyInWriter, types::ToSql};
use tokio_postgres::{types::Type, Error};
use uuid::Uuid;

pub struct Metric {
    pub visitor_id: String,
    pub shorthand_id: String,
    pub url: String,
    pub ip: String,
    pub android: Option<bool>,
    pub ios: Option<bool>,
    pub mobile: Option<bool>,
    pub region_name: Option<String>,
    pub country: Option<String>,
    pub city: Option<String>,
    pub zip_code: Option<String>,
    pub time_zone: Option<String>,
    pub user_agent: Option<String>,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
}

const COPY_STMT: &str = r"COPY metrics (
  id, 
  key, 
  user_id, 
  url, 
  ip, 
  android, 
  ios, 
  mobile, 
  region_name, 
  country, 
  city, 
  zip_code, 
  time_zone, 
  user_agent, 
  longitude, 
  latitude, 
  visitor_id
) FROM STDIN BINARY";

const COPY_TYPES: [Type; 17] = [
    Type::TEXT,
    Type::TEXT,
    Type::TEXT,
    Type::TEXT,
    Type::TEXT,
    Type::BOOL,
    Type::BOOL,
    Type::BOOL,
    Type::TEXT,
    Type::TEXT,
    Type::TEXT,
    Type::TEXT,
    Type::TEXT,
    Type::TEXT,
    Type::FLOAT8,
    Type::FLOAT8,
    Type::TEXT,
];

pub async fn persist_metrics(
    mut client: deadpool_postgres::Object,
    metrics: Vec<Metric>,
) -> Result<(), Error> {
    let transaction = client.transaction().await?;
    let sink = transaction.copy_in(COPY_STMT).await?;

    let writer = BinaryCopyInWriter::new(sink, &COPY_TYPES);
    pin!(writer);

    let mut row: Vec<&'_ (dyn ToSql + Sync)> = Vec::new();

    for metric in metrics {
        row.clear();

        let id = Uuid::now_v7();
        let user_id: u32 = thread_rng().gen_range(1..100);

        // TODO: add timestamp instead of relying on auto generated value (insert time != event time)
        writer
            .as_mut()
            .write(&[
                &id.to_string(),
                &metric.shorthand_id,
                &user_id.to_string(),
                &metric.url,
                &metric.ip,
                &metric.android,
                &metric.ios,
                &metric.mobile,
                &metric.region_name,
                &metric.country,
                &metric.city,
                &metric.zip_code,
                &metric.time_zone,
                &metric.user_agent,
                &metric.longitude,
                &metric.latitude,
                &metric.visitor_id,
            ])
            .await?;
    }

    writer.finish().await?;
    transaction.commit().await?;

    Ok(())
}
