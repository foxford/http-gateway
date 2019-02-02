#[macro_use]
extern crate tower_web;

use failure::Error;
use futures::{Future, Stream};
use log::info;
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
    let (client, notifications) = mqtt::AgentBuilder::new(agent_id.clone()).start(&config.mqtt)?;

    let client = std::sync::Arc::new(std::sync::Mutex::new(client));

    let client_copy_for_notifications = client.clone();

    let notifications = notifications.and_then(move |notif| {
        match notif {
            Notification::Publish(msg) => {
                info!("{}", msg.topic_name);
                let envelope =
                    serde_json::from_slice::<compat::IncomingEnvelope>(&msg.payload).unwrap();
                match envelope.properties() {
                    compat::IncomingEnvelopeProperties::Response(..) => {
                        let response = compat::into_response(envelope).unwrap();
                        let correlation_data =
                            uuid::Uuid::parse_str(response.properties().correlation_data())
                                .unwrap();

                        client_copy_for_notifications
                            .lock()
                            .unwrap()
                            .finish_request(correlation_data, response);
                    }
                    _ => {}
                }
            }
            _ => {}
        };
        futures::future::ok(())
    });

    let tcp_stream = TcpListener::bind(&config.web.listen_addr)?;

    let server = ServiceBuilder::new()
        .config(config.authn)
        .middleware(LogMiddleware::new("http_gateway::web"))
        .resource(web::HttpGatewayApp::new(client))
        .serve(tcp_stream.incoming());

    let server = notifications
        .into_future()
        .map_err(|_| ())
        .join(server)
        .map(|_| ());

    tokio::run(server);

    Ok(())
}
