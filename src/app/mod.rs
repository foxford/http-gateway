use std::collections::HashMap;
use std::convert::TryFrom;
use std::net::SocketAddr;
use std::time::Duration;
use std::{sync::Arc, thread};

use chrono::Utc;
use failure::{format_err, Error};
use futures::{sync::mpsc, Future, Stream};
use futures_locks::Mutex;
use http::{header, Method, Response as HttpResponse, StatusCode};
use log::{error, info, warn};
use serde_derive::Deserialize;
use serde_json::Value as JsonValue;
use svc_agent::mqtt::{
    compat, AgentBuilder, ConnectionMode, Notification, OutgoingRequest, OutgoingRequestProperties,
    QoS, SubscriptionTopic,
};
use svc_agent::{
    mqtt::{Agent, ShortTermTimingProperties},
    AccountId, AgentId, Authenticable, ResponseSubscription, SharedGroup, Source, Subscription,
};
use svc_authn::{jose::Algorithm, token::jws_compact};
use svc_error::{extension::sentry, Error as SvcError};
use tokio::net::TcpListener;
use tokio::prelude::FutureExt;
use tower_web::{
    impl_web, middleware::cors::CorsBuilder, middleware::log::LogMiddleware, Extract,
    ServiceBuilder,
};
use uuid::Uuid;

use crate::util::headers::Headers;
use crate::util::http_stream::OutgoingStream;
use crate::util::mqtt_request::Adapter;

const API_VERSION: &str = "v1";

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Extract, Deserialize)]
struct RequestPayload {
    me: AgentId,
    destination: AccountId,
    payload: JsonValue,
    method: String,
}

//////////////////////////////////////////////////////////////////////////////////

struct Request {
    tx: Mutex<Adapter>,
    timeout: Duration,
}

impl Request {
    fn new(tx: Mutex<Adapter>, timeout: Duration) -> Self {
        Self { tx, timeout }
    }
}

impl_web! {
    impl Request {
        #[post("/api/v1/request")]
        #[content_type("application/json")]
        fn request(
            &self,
            body: RequestPayload,
            sub: AccountId,
        ) -> impl Future<Item = Result<HttpResponse<String>, tower_web::Error>, Error = ()> {
            let error = || SvcError::builder().kind("request_error", "Error sending a request");
            let timeout = self.timeout;

            self.tx.lock()
                .map_err(move |_| {
                    let detail = "error acquiring a mutex for outgoing MQTT request";
                    error().status(StatusCode::UNPROCESSABLE_ENTITY).detail(&detail).build()
                })
                .and_then(move |mut tx| {
                    let payload_account_id = body.me.as_account_id();
                    if &sub != payload_account_id {
                        let detail = format!("account id = '{}' from the access token doesn't match one in payload.me = '{}' payload", &sub, payload_account_id);
                        return Err(error().status(StatusCode::FORBIDDEN).detail(&detail).build());
                    }

                    let response_topic = {
                        let src = Source::Unicast(Some(&body.destination));
                        let sub = ResponseSubscription::new(src);

                        sub.subscription_topic(tx.id(), API_VERSION).map_err(|err| {
                            error()
                                .status(StatusCode::UNPROCESSABLE_ENTITY)
                                .detail(&err.to_string()).build()
                        })?
                    };

                    let mut props = OutgoingRequestProperties::new(
                        &body.method,
                        &response_topic,
                        &Uuid::new_v4().to_string(),
                        ShortTermTimingProperties::new(Utc::now()),
                    );
                    props.set_agent_id(body.me);
                    let req = OutgoingRequest::multicast(body.payload, props, &body.destination);

                    // Send request
                    tx.request(req).map_err(|err| {
                        error()
                            .status(StatusCode::UNPROCESSABLE_ENTITY)
                            .detail(&err.to_string())
                            .build()
                    })
                })
                .and_then(move |req| {
                    req
                        .timeout(timeout)
                        .map_err(move |_| {
                            let detail = "timeout on an outgoing HTTP response";
                            error().status(StatusCode::GATEWAY_TIMEOUT).detail(&detail).build()
                        })
                })
                .then(|result| match result {
                    Ok(resp) => Ok(HttpResponse::builder()
                        .status(resp.properties().status())
                        .body(resp.payload().to_string())
                        .map_err(|err| {
                            tower_web::Error::builder()
                                .status(StatusCode::UNPROCESSABLE_ENTITY)
                                .kind("http_response_build_error", "Failed to build HTTP response")
                                .detail(&err.to_string())
                                .build()
                        })
                        .and_then(|mut http_response| Headers::try_from(&resp)
                            .map(|headers| {
                                headers.add_to_header_map(http_response.headers_mut());
                                http_response
                            })
                            .map_err(|err| {
                                tower_web::Error::builder()
                                    .status(StatusCode::UNPROCESSABLE_ENTITY)
                                    .kind("http_response_headers_error", "Failed to set HTTP response headers")
                                    .detail(&err.to_string())
                                    .build()
                            })
                        )),
                    Err(err) => {
                        notify_error(err.clone());

                        let builder = tower_web::Error::builder()
                            .status(err.status_code())
                            .kind(err.kind(), err.title());

                        let builder = match err.detail() {
                            Some(detail) => builder.detail(detail),
                            None => builder,
                        };

                        Ok(Err(builder.build()))
                    }
                })
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Deserialize)]
pub(crate) struct IdTokenConfig {
    #[serde(deserialize_with = "svc_authn::serde::algorithm")]
    algorithm: Algorithm,
    #[serde(deserialize_with = "svc_authn::serde::file")]
    key: Vec<u8>,
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Deserialize)]
pub(crate) struct HttpConfig {
    listener_address: SocketAddr,
    cors: Cors,
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Deserialize)]
pub(crate) struct Cors {
    #[serde(deserialize_with = "crate::serde::allowed_origins")]
    #[serde(default)]
    allow_origins: tower_web::middleware::cors::AllowedOrigins,
    #[serde(deserialize_with = "crate::serde::duration")]
    #[serde(default)]
    max_age: Duration,
}

////////////////////////////////////////////////////////////////////////////////

struct State {
    event: endpoint::event::State,
}

////////////////////////////////////////////////////////////////////////////////

pub(crate) fn run() {
    // Config
    let config = config::load().expect("Failed to load config");
    info!("Config: {:?}", config);

    // Sentry
    if let Some(sentry_config) = config.sentry.as_ref() {
        sentry::init(sentry_config);
    }

    // Agent
    let agent_id = AgentId::new(&config.agent_label, config.id.clone());
    info!("Agent Id: {}", agent_id);

    let token = jws_compact::TokenBuilder::new()
        .issuer(&agent_id.as_account_id().audience().to_string())
        .subject(&agent_id)
        .key(config.id_token.algorithm, config.id_token.key.as_slice())
        .build()
        .expect("Error creating an id token");

    let mut agent_config = config.mqtt.clone();
    agent_config.set_password(&token);

    let (mut tx, rx) = AgentBuilder::new(agent_id.clone(), API_VERSION)
        .connection_mode(ConnectionMode::Bridge)
        .start(&agent_config)
        .expect("Failed to create an agent");

    // Event loop for incoming messages of MQTT Agent
    let (mq_tx, mq_rx) = mpsc::unbounded::<Notification>();
    thread::spawn(move || {
        for message in rx {
            if mq_tx.unbounded_send(message).is_err() {
                error!("Error sending message to the internal channel");
            }
        }
    });

    // Create Subscriptions
    subscribe(&mut tx, &agent_id, &config.events).expect("Failed to subscribe");
    let agent = tx.clone();

    // Create MQTT Request Adapter
    let req_tx = Mutex::new(Adapter::new(tx));
    let resp_tx = req_tx.clone();

    // Generate bearer tokens for callback requests
    let mut tokens = HashMap::new();
    for audience in config.events.keys() {
        // Unique subject audience for each tenant to generate unique tokens
        let subject_audience = format!("{}:{}", config.id.audience(), audience);
        let subject = AccountId::new(config.id.label(), &subject_audience);

        let token = jws_compact::TokenBuilder::new()
            .issuer(&config.id.audience().to_owned())
            .subject(&subject)
            .key(config.id_token.algorithm, config.id_token.key.as_slice())
            .build()
            .unwrap_or_else(|_| panic!("Error creating an id token for audience = '{}'", audience));

        tokens.insert(audience.to_owned(), token);
    }

    // Application resources
    let state = Arc::new(State {
        event: endpoint::event::State::new(config.events, tokens),
    });

    let (hq_tx, hq_rx) = OutgoingStream::new(&config.http_client);
    let mq_rx = mq_rx.for_each(move |message| {
        let mut hq_tx = hq_tx.clone();
        let state = state.clone();
        let mut agent = agent.clone();
        let agent_id = agent_id.clone();

        resp_tx
            .lock()
            .and_then(move |mut resp_tx| {
                match message {
                    Notification::Publish(message) => {
                        let topic: &str = &message.topic_name;

                        // Log incoming messages
                        info!(
                            "Incoming message = '{}' sent to the topic = '{}', dup = '{}', pkid = '{:?}'",
                            String::from_utf8_lossy(message.payload.as_slice()),
                            topic,
                            message.dup,
                            message.pkid,
                        );

                        let result = handle_message(
                            &mut resp_tx,
                            &mut hq_tx,
                            topic,
                            message.payload.clone(),
                            state.clone(),
                        );

                        if let Err(err) = result {
                            error!(
                                "Error processing a message = '{text}' sent to the topic = '{topic}', {detail}",
                                text = String::from_utf8_lossy(message.payload.as_slice()),
                                topic = topic,
                                detail = err,
                            );

                            let err = SvcError::builder()
                                .kind("message_processing_error", "Message processing error")
                                .detail(&err.to_string())
                                .build();

                            notify_error(err);
                        }
                    }
                    Notification::Disconnection => {
                        error!("Disconnected from broker");
                    }
                    Notification::Reconnection => {
                        error!("Reconnected to broker");
                        resubscribe(&mut agent, &agent_id, state.event.config());
                    }
                    _ => error!("Unsupported notification type = '{:?}'", message),
                }

                Ok(())
            })
    });

    // Resources
    let request = Request::new(req_tx, Duration::from_secs(config.http_client.timeout()));

    // Middleware
    let cors = CorsBuilder::new()
        .allow_origins(config.http.cors.allow_origins.clone())
        .allow_methods(vec![Method::POST])
        .allow_headers(vec![
            header::AUTHORIZATION,
            header::CONTENT_LENGTH,
            header::CONTENT_TYPE,
        ])
        .allow_credentials(true)
        .max_age(config.http.cors.max_age)
        .build();

    let tcp_stream =
        TcpListener::bind(&config.http.listener_address).expect("Invalid HTTP listener address");

    let server = ServiceBuilder::new()
        .config(config.authn.clone())
        .middleware(LogMiddleware::new("http_gateway::http"))
        .middleware(cors)
        .resource(request)
        .serve(tcp_stream.incoming());

    tokio::run(server.join(mq_rx).join(hq_rx).map(|_| ()));
}

fn subscribe(
    agent: &mut Agent,
    agent_id: &AgentId,
    events_config: &endpoint::event::ConfigMap,
) -> Result<(), String> {
    let group = SharedGroup::new("loadbalancer", agent_id.as_account_id().clone());

    // Responses
    agent
        .subscribe(&Subscription::unicast_responses(), QoS::AtLeastOnce, None)
        .map_err(|err| format!("Error subscribing to app's responses topic: {}", err))?;

    // Audience level events for each tenant
    for (tenant_audience, tenant_events_config) in events_config {
        for from_account_id in tenant_events_config.sources() {
            agent
                .subscribe(
                    &Subscription::broadcast_events(
                        from_account_id,
                        API_VERSION,
                        &format!("audiences/{}/events", tenant_audience),
                    ),
                    QoS::AtLeastOnce,
                    Some(&group),
                )
                .map_err(|err| format!("Error subscribing to app's events topic: {}", err))?;
        }
    }

    Ok(())
}

fn resubscribe(agent: &mut Agent, agent_id: &AgentId, events_config: &endpoint::event::ConfigMap) {
    if let Err(err) = subscribe(agent, agent_id, events_config) {
        let err = format!("Failed to resubscribe after reconnection: {}", err);
        error!("{}", err);

        let svc_error = SvcError::builder()
            .kind("resubscription_error", "Resubscription error")
            .detail(&err)
            .build();

        sentry::send(svc_error)
            .unwrap_or_else(|err| warn!("Error sending error to Sentry: {}", err));
    }
}

//////////////////////////////////////////////////////////////////////////////////

fn handle_message(
    resp_tx: &mut Adapter,
    hq_tx: &mut OutgoingStream,
    topic: &str,
    payload: Arc<Vec<u8>>,
    state: Arc<State>,
) -> Result<(), Error> {
    let envelope = serde_json::from_slice::<compat::IncomingEnvelope>(payload.as_slice())?;
    match envelope.properties() {
        compat::IncomingEnvelopeProperties::Response(_) => {
            let inresp = compat::into_response(envelope)?;
            resp_tx.commit_response(inresp)
        }
        compat::IncomingEnvelopeProperties::Event(_) => {
            let inev = compat::into_event::<JsonValue>(envelope)?;
            let outev = state.event.handle(topic, &inev)?;
            hq_tx.send(outev)
        }
        _ => Err(format_err!(
            "unsupported message type, envelope = '{:?}'",
            envelope
        )),
    }
}

//////////////////////////////////////////////////////////////////////////////////

fn notify_error(error: SvcError) {
    if let Err(err) = sentry::send(error) {
        error!("Error sending error to Sentry: {}", err);
    }
}

//////////////////////////////////////////////////////////////////////////////////

mod config;
mod endpoint;

//////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod test {

    use super::RequestPayload;
    use serde_json::{self, json};

    #[test]
    fn ser() {
        let val = json!({
            "me": "web.12345.netology.ru",
            "destination": "conference.netology-group.services",
            "payload": "test",
            "method": "room.create",
        });

        let d: RequestPayload = serde_json::from_value(val).unwrap();

        dbg!(d);
    }
}

//////////////////////////////////////////////////////////////////////////////////
