#[macro_use]
extern crate tower_web;

use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_derive::{Deserialize, Serialize};
use svc_agent::{
    mqtt::{
        Agent, AgentBuilder, AgentConfig, ConnectionMode, OutgoingEvent, OutgoingEventProperties,
    },
    AccountId, AgentId,
};
use svc_authn::token::jws_compact::extract::parse_jws_compact;
use tower_web::ServiceBuilder;

///////////////////////////////////////////////////////////////////////////////

struct IncomingRequest {
    authorization: String,
    body: String,
}

struct TenantMock {
    tx: Arc<Mutex<mpsc::Sender<IncomingRequest>>>,
}

impl TenantMock {
    fn run() -> mpsc::Receiver<IncomingRequest> {
        let (tx, rx) = mpsc::channel::<IncomingRequest>();
        let tx = Arc::new(Mutex::new(tx));
        let tenant_mock = Self { tx };

        thread::spawn(move || {
            let addr = "0.0.0.0:31947".parse().expect("Failed to parse address");

            ServiceBuilder::new()
                .resource(tenant_mock)
                .run(&addr)
                .expect("Failed to start tenant mock");
        });

        rx
    }
}

impl_web! {
    impl TenantMock {
        #[post("/callback")]
        fn callback(&self, authorization: String, body: String) -> Result<&'static str, ()> {
            let request = IncomingRequest { authorization, body };
            let tx = self.tx.lock().expect("Failed to obtain sender lock");
            tx.send(request).expect("Failed to publish message to sender");
            Ok("OK")
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Deserialize, Serialize)]
struct EventPayload {
    message: String,
}

impl EventPayload {
    fn new(message: &str) -> Self {
        Self {
            message: message.to_owned(),
        }
    }
}

struct ServiceMock {
    agent: Agent,
}

impl ServiceMock {
    fn new() -> Self {
        let account_id = AccountId::new("mock-service", "test.svc.example.org");
        let agent_id = AgentId::new("test", account_id);
        let builder = AgentBuilder::new(agent_id).mode(ConnectionMode::Service);

        let config = serde_json::from_str::<AgentConfig>(r#"{"uri": "0.0.0.0:1883"}"#)
            .expect("Failed to parse agent config");

        let (agent, _) = builder
            .start(&config)
            .expect("Failed to start service mock");

        Self { agent }
    }

    fn broadcast_message(&mut self, message: &str) {
        let payload = EventPayload::new(message);
        let props = OutgoingEventProperties::new("message");
        let event = OutgoingEvent::broadcast(payload, props, "audiences/example.org/events");

        self.agent
            .publish(Box::new(event))
            .expect("Failed to publish event");
    }
}

///////////////////////////////////////////////////////////////////////////////

#[test]
fn send_event() {
    // Service mock <-> HTTP-Gateway <-> Tenant mock
    let tenant_rx = TenantMock::run();
    let mut service = ServiceMock::new();
    thread::spawn(move || http_gateway::run("App.test.toml"));

    // Broadcast message as mock service.
    service.broadcast_message("hello");

    // Wait for HTTP request to tenant mock.
    let timeout = Duration::from_secs(5);
    let inreq = tenant_rx.recv_timeout(timeout).expect("No response");

    // Assert payload.
    let payload =
        serde_json::from_str::<EventPayload>(&inreq.body).expect("Failed to parse payload");

    assert_eq!(payload.message, "hello");

    // Assert authorization header.
    let words: Vec<&str> = inreq.authorization.split(' ').collect();

    let token = match words[..] {
        ["Bearer", token] => token,
        _ => panic!("Failed to parse bearer token: {}"),
    };

    let jws_data =
        parse_jws_compact::<String>(&token).expect(&format!("Failed to extract JWS: {}", token));

    assert_eq!(jws_data.claims.subject(), "http-gateway");
    assert_eq!(jws_data.claims.audience(), "test.svc.example.org");
    assert_eq!(jws_data.claims.issuer(), "test.svc.example.org");
}
