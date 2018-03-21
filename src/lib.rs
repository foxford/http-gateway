extern crate actix;
extern crate actix_web;
extern crate env_logger;
extern crate futures;
extern crate http;
#[macro_use]
extern crate log;

pub mod actors;
pub mod defaults;
pub mod headers_checker;

use actix::*;

pub struct State {
    pub mqtt_handler: Addr<Syn, actors::mqtt_handler::MqttHandler>,
    pub request_parser: Addr<Syn, actors::request_parser::RequestParser>,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
