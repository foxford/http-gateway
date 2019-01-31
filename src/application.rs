use actix::{Addr, Syn};

use crate::actors::mqtt_handler::MqttHandler;

pub const DEFAULT_BROKER_URL: &'static str = "localhost:1883";
pub const DEFAULT_LISTEN_ADDR: &'static str = "localhost:8080";
pub const DEFAULT_MQTT_HANDLER_COEFFICIENT: usize = 10;
pub const DEFAULT_MQTT_HANDLER_TIMEOUT: u64 = 5000; //milliseconds

pub struct AppState {
  pub mqtt_handler: Addr<Syn, MqttHandler>,
  pub broker_url: String,
  pub mqtt_handler_timeout: u64,
}
