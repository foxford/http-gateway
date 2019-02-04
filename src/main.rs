#[macro_use]
extern crate tower_web;

use failure::Error;
use futures::{Future, IntoFuture, Stream};
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
    let (client, notifications) = mqtt::AgentBuilder::new(agent_id.clone()).start(&config.mqtt)?;

    let client = std::sync::Arc::new(std::sync::Mutex::new(client));
    let client_copy_for_notifications = client.clone();

    let handle_notifications = move |notif: rumqtt::Publish| -> Result<(), Error> {
        let envelope = serde_json::from_slice::<compat::IncomingEnvelope>(&notif.payload)?;
        match envelope.properties() {
            compat::IncomingEnvelopeProperties::Response(..) => {
                let response = compat::into_response(envelope)?;
                let correlation_data =
                    uuid::Uuid::parse_str(response.properties().correlation_data())?;

                client_copy_for_notifications
                    .lock()
                    .unwrap()
                    .finish_request(correlation_data, response);
            }
            _ => {}
        }

        Ok(())
    };

    let notifications = notifications.for_each(move |notif| {
        if let Notification::Publish(msg) = notif {
            info!(
                "Incoming message: {}",
                String::from_utf8_lossy(&msg.payload)
            );

            match handle_notifications(msg) {
                Ok(..) => {}
                Err(err) => {
                    error!("Error during notification handling: {}", err);
                }
            }
        }

        Ok(())
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
