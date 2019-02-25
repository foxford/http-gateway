use std::net::SocketAddr;

mod request;
mod serde;

pub use request::{InFlightRequests, RequestResource};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub listen_addr: SocketAddr,
    pub request_timeout: u64,
    pub cors: Cors,
}

#[derive(Debug, Deserialize)]
pub struct Cors {
    #[serde(deserialize_with = "self::serde::allowed_origins")]
    #[serde(default)]
    pub allow_origins: tower_web::middleware::cors::AllowedOrigins,
    #[serde(deserialize_with = "self::serde::duration")]
    #[serde(default)]
    pub max_age: std::time::Duration,
}
