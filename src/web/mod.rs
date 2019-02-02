use std::net::SocketAddr;

mod app;

pub use app::HttpGatewayApp;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub listen_addr: SocketAddr,
}
