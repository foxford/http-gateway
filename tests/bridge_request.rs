use std::fs::File;
use std::io::Read;
use std::thread;

use http::StatusCode as HttpStatus;
use reqwest::header::{self, HeaderMap, HeaderValue};
use serde_json::{json, Value as JsonValue};
use svc_agent::{
    mqtt::{
        compat, AgentBuilder, AgentConfig, ConnectionMode, Notification, OutgoingResponse, QoS,
    },
    AccountId, AgentId, SharedGroup, Subscription,
};
use svc_authn::{jose::Algorithm, token::jws_compact};

///////////////////////////////////////////////////////////////////////////////

fn run_ping_service() {
    // Create agent.
    let account_id = AccountId::new("ping-service", "test.svc.example.org");
    let agent_id = AgentId::new("test", account_id.clone());
    let builder = AgentBuilder::new(agent_id).mode(ConnectionMode::Service);

    let config = serde_json::from_str::<AgentConfig>(r#"{"uri": "0.0.0.0:1883"}"#)
        .expect("Failed to parse agent config");

    let (mut agent, rx) = builder
        .start(&config)
        .expect("Failed to start ping service");

    // Subscribe to multicast requests topic.
    agent
        .subscribe(
            &Subscription::multicast_requests(),
            QoS::AtLeastOnce,
            Some(&SharedGroup::new("loadbalancer", account_id)),
        )
        .expect("Error subscribing to multicast requests");

    // Message handling loop.
    while let Ok(Notification::Publish(message)) = rx.recv() {
        let bytes = message.payload.as_slice();

        let envelope = serde_json::from_slice::<compat::IncomingEnvelope>(bytes)
            .expect("Failed to parse incoming message");

        // Handle request.
        match compat::into_request::<JsonValue>(envelope) {
            Ok(request) => {
                assert_eq!(request.properties().method(), "ping");
                assert_eq!(request.payload()["message"].as_str(), Some("ping"));

                let response = OutgoingResponse::unicast(
                    json!({"message": "pong"}),
                    request.properties().to_response(HttpStatus::CREATED),
                    request.properties(),
                );

                agent
                    .publish(Box::new(response))
                    .expect("Failed to publish response");
            }
            Err(err) => panic!(err),
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

const PING_REQUEST_BODY: &'static str = r#"
    {
        "me": "node-1.mock-tenant.test.svc.example.org",
        "destination": "ping-service.test.svc.example.org",
        "method": "ping",
        "payload": {
            "message": "ping"
        }
    }
"#;

#[test]
fn bridge_request() {
    // Client <-> HTTP-Gateway <-> PingService.
    let client = reqwest::Client::new();
    thread::spawn(move || http_gateway::run("App.test.toml"));
    thread::spawn(run_ping_service);
    thread::sleep(std::time::Duration::from_secs(3));

    // Access PingService with HTTP through HTTP-Gateway.
    let authz_header = HeaderValue::from_str(&format!("Bearer {}", build_authz_token()))
        .expect("Failed to build Authorization header");

    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, authz_header);
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );

    let mut resp = client
        .post("http://0.0.0.0:31181/api/v1/request")
        .headers(headers)
        .body(PING_REQUEST_BODY)
        .send()
        .expect("HTTP request failed");

    // Assert HTTP status.
    assert_eq!(resp.status(), HttpStatus::CREATED);

    // Assert payload.
    let payload = resp.json::<JsonValue>().expect("Failed to parse response");
    assert_eq!(payload["message"].as_str(), Some("pong"));
}

fn build_authz_token() -> String {
    let mut key_file =
        File::open("data/keys/iam.private_key.p8.der.sample").expect("Failed to open key file");

    let mut key = Vec::<u8>::new();
    key_file.read_to_end(&mut key).expect("Failed to read key");

    jws_compact::TokenBuilder::new()
        .issuer("iam.test.svc.example.org")
        .subject(&AccountId::new("mock-tenant", "test.svc.example.org"))
        .key(Algorithm::ES256, &key)
        .build()
        .expect("Error creating an id token")
}
