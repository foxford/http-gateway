use std::net::SocketAddr;

mod request;

pub use request::{InFlightRequests, RequestResource};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub listen_addr: SocketAddr,
}
