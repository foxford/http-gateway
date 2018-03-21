extern crate http_gateway;
extern crate num_cpus;

extern crate futures;
use futures::Future;

extern crate actix;
use actix::*;

extern crate actix_web;
use actix_web::*;

extern crate env_logger;
#[macro_use]
extern crate log;

use http_gateway::headers_checker::HeadersChecker;
use http_gateway::actors::mqtt_handler::{MqttHandler, RPCCall};
use http_gateway::actors::request_parser::{RPCCallParse, RequestParser};
use http_gateway::State;

const DEFAULT_MQTT_PORT: &'static str = "1883";
const DEFAULT_MQTT_HOST: &'static str = "test.mosquitto.org";

fn rpc_call(req: HttpRequest<State>) -> Box<Future<Item = HttpResponse, Error = Error>> {
    let mqtt_host = match option_env!("MQTT_HOST") {
        Some(host) => host,
        _ => DEFAULT_MQTT_HOST,
    };

    let mqtt_port = match option_env!("MQTT_PORT") {
        Some(port) => port,
        _ => DEFAULT_MQTT_PORT,
    };

    let broker_addr = format!("{}:{}", mqtt_host, mqtt_port);
    let payload = req.clone().body().wait().unwrap().to_vec();

    req.state()
        .request_parser
        .send(RPCCallParse {
            headers: req.headers().clone(),
        })
        .from_err()
        .and_then(move |par| match par {
            Ok(params) => req.state()
                .mqtt_handler
                .send(RPCCall {
                    broker_addr: broker_addr,
                    params: params,
                    payload: payload,
                })
                .from_err()
                .and_then(|res| match res {
                    Ok(response) => Ok(httpcodes::HTTPOk.build().body(response)?),
                    Err(err) => {
                        error!("{}", err);
                        Ok(httpcodes::HTTPInternalServerError.into())
                    }
                })
                .wait(),
            Err(e) => {
                error!("{}", e);
                Ok(httpcodes::HTTPBadRequest.into())
            }
        })
        .responder()
}

fn main() {
    ::std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();
    let sys = actix::System::new("http-gateway");

    let addr = SyncArbiter::start(num_cpus::get() * 10, || MqttHandler);
    let parser_addr = SyncArbiter::start(num_cpus::get() * 10, || RequestParser);

    HttpServer::new(move || {
        Application::with_state(State {
            mqtt_handler: addr.clone(),
            request_parser: parser_addr.clone(),
        }).middleware(middleware::Logger::default())
            .middleware(HeadersChecker)
            .resource("/rpc_call", |r| r.method(Method::POST).a(rpc_call))
    }).bind("0.0.0.0:80")
        .unwrap()
        .start();

    info!("Started http server: 0.0.0.0:80");
    sys.run();
}
