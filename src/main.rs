extern crate actix;
extern crate actix_web;
extern crate bytes;
extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate futures;
#[macro_use]
extern crate log;
extern crate num_cpus;
extern crate paho_mqtt as mqtt;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

mod actors;
mod api;
mod application;
mod defaults;
mod errors;
mod extractors;

use actix::SyncArbiter;
use actix_web::{http, middleware, server, App};
use actors::mqtt_handler::MqttHandler;
use api::rpc_call::rpc_call;
use application::*;
use std::env;

fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "actix_web=info");
    }

    let broker_url = env::var("MQTT_BROKER_URL").unwrap_or(DEFAULT_BROKER_URL.to_owned());

    let listen_addr = env::var("LISTEN_ADDR").unwrap_or(DEFAULT_LISTEN_ADDR.to_owned());

    let mqtt_handler_coefficient = match env::var("MQTT_HANDLER_COEFFICIENT") {
        Ok(coef) => coef
            .parse::<usize>()
            .unwrap_or_else(|_| DEFAULT_MQTT_HANDLER_COEFFICIENT),
        Err(_) => DEFAULT_MQTT_HANDLER_COEFFICIENT,
    };

    let mqtt_handler_threads = num_cpus::get() * mqtt_handler_coefficient;
    let mqtt_handler_threads = match env::var("MQTT_HANDLER_THREADS") {
        Ok(num) => num
            .parse::<usize>()
            .unwrap_or_else(|_| mqtt_handler_threads),
        Err(_) => mqtt_handler_threads,
    };

    let mqtt_handler_timeout = match env::var("MQTT_HANDLER_TIMEOUT") {
        Ok(timeout) => timeout
            .parse::<u64>()
            .unwrap_or_else(|_| DEFAULT_MQTT_HANDLER_TIMEOUT),
        Err(_) => DEFAULT_MQTT_HANDLER_TIMEOUT,
    };

    env_logger::init();
    let sys = actix::System::new("http-gateway");

    let addr = SyncArbiter::start(mqtt_handler_threads, || MqttHandler);

    server::new(move || {
        App::with_state(AppState {
            mqtt_handler: addr.clone(),
            broker_url: broker_url.clone(),
            mqtt_handler_timeout: mqtt_handler_timeout,
        })
        .middleware(middleware::Logger::default())
        .resource("/rpc_call", |r| {
            r.method(http::Method::POST).with3(rpc_call)
        })
    })
    .bind(listen_addr)
    .unwrap()
    .shutdown_timeout(1)
    .start();

    let _ = sys.run();
}
