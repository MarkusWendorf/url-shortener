use postgis::ewkb::Point;
use time::OffsetDateTime;
use tokio::pin;
use tokio_postgres::types::{Kind, Type};
use tokio_postgres::Error;
use tokio_postgres::{binary_copy::BinaryCopyInWriter, types::ToSql};
use uuid::Uuid;

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
  visitor_id,
  created_at,
  location
) FROM STDIN BINARY";

pub struct Metric {
    pub visitor_id: String,
    pub shorthand_id: String,
    pub user_id: i64,
    pub created_at: OffsetDateTime,
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

pub async fn persist_metrics(mut client: deadpool_postgres::Object, metrics: Vec<Metric>) -> Result<(), Error> {
    let geography_type = Type::new("geography".to_owned(), 9999, Kind::Simple, "public".to_owned());

    let types = [
        Type::TEXT,
        Type::TEXT,
        Type::INT8,
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
        Type::TEXT,
        Type::TIMESTAMPTZ,
        geography_type,
    ];

    let transaction = client.transaction().await?;
    let sink = transaction.copy_in(COPY_STMT).await?;

    let writer = BinaryCopyInWriter::new(sink, &types);
    pin!(writer);

    let mut row: Vec<&'_ (dyn ToSql + Sync)> = Vec::new();

    for metric in metrics {
        row.clear();

        let id = Uuid::now_v7();

        let location = match (metric.longitude, metric.latitude) {
            (Some(lng), Some(lat)) => Some(Point::new(lng, lat, Some(4326))),
            _ => None,
        };

        writer
            .as_mut()
            .write(&[
                &id.to_string(),
                &metric.shorthand_id,
                &metric.user_id,
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
                &metric.visitor_id,
                &metric.created_at,
                &location,
            ])
            .await?;
    }

    writer.finish().await?;
    transaction.commit().await?;

    Ok(())
}
