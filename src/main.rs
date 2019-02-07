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
mod events;
mod mqtt;
mod web;

use authn::Authenticable;
use mqtt::compat;

fn main() -> Result<(), Error> {
    env_logger::init();

    let config = conf::load()?;
    let config = std::sync::Arc::new(config);
    info!("Config: {:?}", config);

    let label = uuid::Uuid::new_v4().to_string();

    let agent_id = authn::AgentId::new(&label, config.id.clone());
    info!("Agent Id: {}", agent_id);
    let (mut agent, messages) = mqtt::AgentBuilder::new(agent_id)
        .mode(mqtt::ConnectionMode::Bridge)
        .start(&config.mqtt)?;

    let messages = wrap_async(messages);

    let src = mqtt::Source::Unicast(None);
    let sub = mqtt::ResponseSubscription::new(src);
    agent.subscribe(&sub, rumqtt::QoS::AtLeastOnce, None)?;

    // Subscribe for all configured events
    for (audience, audience_conf) in &config.events {
        for app in &audience_conf.sources {
            let uri = format!("audiences/{}", audience);
            let src = mqtt::Source::Broadcast(&app, &uri);
            let sub = mqtt::EventSubscription::new(src);
            agent.subscribe(&sub, rumqtt::QoS::AtLeastOnce, None)?;
        }
    }

    let in_flight_requests = web::InFlightRequests::new();
    let in_flight_requests = futures_locks::Mutex::new(in_flight_requests);
    let in_flight_requests_copy = in_flight_requests.clone();

    let config_copy = config.clone();

    let (event_sender, event_stream) = events::event_handler();
    let event_stream = event_stream.into_future().map_err(|_| ());

    let messages = messages.for_each(move |msg| {
        let config_copy = config_copy.clone();
        let event_sender = event_sender.clone();

        in_flight_requests_copy
            .lock()
            .and_then(move |mut in_flight_requests| {
                if let Notification::Publish(msg) = msg {
                    info!(
                        "Incoming message: {}",
                        String::from_utf8_lossy(&msg.payload)
                    );

                    match handle_message(&mut in_flight_requests, msg, &config_copy, &event_sender)
                    {
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
        .config(config.authn.clone())
        .middleware(LogMiddleware::new("http_gateway::web"))
        .resource(request_resource)
        .serve(tcp_stream.incoming());

    let server = messages
        .into_future()
        .join(server)
        .join(event_stream)
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
    config: &conf::Config,
    event_sender: &mpsc::UnboundedSender<events::Event>,
) -> Result<(), Error> {
    let envelope = serde_json::from_slice::<compat::IncomingEnvelope>(&notif.payload)?;
    match envelope.properties() {
        compat::IncomingEnvelopeProperties::Response(..) => {
            let response = compat::into_response(envelope)?;
            let correlation_data = uuid::Uuid::parse_str(response.properties().correlation_data())?;
            in_flight_requests.finish_request(correlation_data, response);
        }
        compat::IncomingEnvelopeProperties::Event(..) => {
            let event = compat::into_event::<serde_json::Value>(envelope)?;

            let audience = event.properties().target_audience();
            if let Some(audience_config) = config.events.get(audience) {
                let account_id = event.properties().account_id();

                if audience_config.sources.contains(&account_id) {
                    event_sender.unbounded_send(events::Event::new(
                        event,
                        audience_config.callback.to_owned(),
                    ))?;
                }
            }
        }
        _ => {}
    }

    Ok(())
}
