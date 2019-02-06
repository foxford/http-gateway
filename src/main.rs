#[macro_use]
extern crate tower_web;

use std::thread;

use failure::Error;
use futures::{sync::mpsc, Future, IntoFuture, Stream};
use log::{error, info};
use rumqtt::Notification;
use tokio::net::TcpListener;
use tower_web::{middleware::log::LogMiddleware, ServiceBuilder};

mod authn;
mod conf;
mod mqtt;
mod web;

use mqtt::compat;

fn main() -> Result<(), Error> {
    env_logger::init();

    let config = conf::load()?;
    info!("Config: {:?}", config);

    let label = uuid::Uuid::new_v4().to_string();

    let agent_id = authn::AgentId::new(&label, config.id);
    info!("Agent Id: {}", agent_id);
    let (mut agent, messages) = mqtt::AgentBuilder::new(agent_id)
        .mode(mqtt::ConnectionMode::Bridge)
        .start(&config.mqtt)?;

    let messages = wrap_async(messages);

    let src = mqtt::Source::Unicast(None);
    let sub = mqtt::ResponseSubscription::new(src);
    agent.subscribe(&sub, rumqtt::QoS::AtLeastOnce, None)?;

    let in_flight_requests = web::InFlightRequests::new();
    let in_flight_requests = futures_locks::Mutex::new(in_flight_requests);
    let in_flight_requests_copy = in_flight_requests.clone();

    let messages = messages.for_each(move |msg| {
        in_flight_requests_copy
            .lock()
            .and_then(|mut in_flight_requests| {
                if let Notification::Publish(msg) = msg {
                    info!(
                        "Incoming message: {}",
                        String::from_utf8_lossy(&msg.payload)
                    );

                    match handle_message(&mut in_flight_requests, msg) {
                        Ok(..) => {}
                        Err(err) => {
                            error!("Error during notification handling: {}", err);
                        }
                    }
                }

                Ok(())
            })
    });

    let request_resource = web::RequestResource::new(agent, in_flight_requests);

    let tcp_stream = TcpListener::bind(&config.web.listen_addr)?;

    let server = ServiceBuilder::new()
        .config(config.authn)
        .middleware(LogMiddleware::new("http_gateway::web"))
        .resource(request_resource)
        .serve(tcp_stream.incoming());

    let server = messages
        .into_future()
        .map_err(|_| ())
        .join(server)
        .map(|_| ());

    tokio::run(server);

    Ok(())
}

fn wrap_async(
    notifications: crossbeam::Receiver<Notification>,
) -> mpsc::UnboundedReceiver<Notification> {
    let (sender, receiver) = mpsc::unbounded();

    thread::spawn(move || {
        for msg in notifications {
            if let Err(err) = sender.unbounded_send(msg) {
                error!(
                    "MqttStream: error sending notification to main stream: {}",
                    err
                );
            }
        }
    });

    receiver
}

fn handle_message(
    in_flight_requests: &mut crate::web::InFlightRequests,
    notif: rumqtt::Publish,
) -> Result<(), Error> {
    let envelope = serde_json::from_slice::<compat::IncomingEnvelope>(&notif.payload)?;
    match envelope.properties() {
        compat::IncomingEnvelopeProperties::Response(..) => {
            let response = compat::into_response(envelope)?;
            let correlation_data = uuid::Uuid::parse_str(response.properties().correlation_data())?;
            in_flight_requests.finish_request(correlation_data, response);
        }
        _ => {}
    }

    Ok(())
}
