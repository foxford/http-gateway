use futures::{Future, future::result};
use bytes;
use actix_web::{error, AsyncResponder, FutureResponse, HttpResponse, State};
use application::*;
use actors::mqtt_handler::RPCCallMessage;
use std::time::Duration;
use extractors::{parsing::get_request_id, rpc_call_extractor::RPCCallParams};

pub fn rpc_call(
    params: RPCCallParams,
    state: State<AppState>,
    body: bytes::Bytes,
) -> FutureResponse<HttpResponse> {
    let id = if let Some(i) = get_request_id(&body) {
        i
    } else {
        return result(Err(error::ErrorBadRequest("missed 'id' field"))).responder();
    };
    state
        .mqtt_handler
        .send(RPCCallMessage {
            id: id,
            params: params,
            broker_url: state.broker_url.clone(),
            payload: body.to_vec(),
        })
        .timeout(Duration::from_millis(state.mqtt_handler_timeout))
        .from_err()
        .and_then(|res| match res {
            Ok(response) => Ok(HttpResponse::Ok().body(response)),
            Err(err) => Err(error::ErrorInternalServerError(err)),
        })
        .responder()
}

#[cfg(test)]
mod tests {
    use actix_web::{http, HttpMessage, client::ClientRequest, test::TestServer};
    use actix::*;
    use futures::Future;
    use actors::mqtt_handler::MqttHandler;
    use application::{AppState, DEFAULT_BROKER_URL, DEFAULT_MQTT_HANDLER_TIMEOUT};
    use api::rpc_call::rpc_call;
    use defaults::*;
    use mqtt;

    fn initialize_server() -> TestServer {
        TestServer::build_with_state(|| {
            let addr = SyncArbiter::start(3, || MqttHandler);
            AppState {
                mqtt_handler: addr.clone(),
                broker_url: DEFAULT_BROKER_URL.to_string(),
                mqtt_handler_timeout: DEFAULT_MQTT_HANDLER_TIMEOUT,
            }
        }).start(|app| {
            app.resource("/rpc_call", |r| {
                r.method(http::Method::POST).with3(rpc_call)
            });
        })
    }

    fn request(
        srv: &TestServer,
        client_id: &'static str,
        publish_topic: &'static str,
        subscribe_topic: &'static str,
        payload: &'static str,
    ) -> ClientRequest {
        srv.client(http::Method::POST, "/rpc_call")
            .header(CLIENT_ID_HEADER, client_id)
            .header(USERNAME_HEADER, "test_username")
            .header(PASSWORD_HEADER, "test_password")
            .header(PUBLISH_TOPIC_HEADER, publish_topic)
            .header(PUBLISH_QOS_HEADER, "1")
            .header(SUBSCRIBE_TOPIC_HEADER, subscribe_topic)
            .header(SUBSCRIBE_QOS_HEADER, "1")
            .body(payload)
            .unwrap()
    }

    fn publish_retained(client_id: &'static str, topic: &'static str, payload: &'static str) {
        let create_opts = mqtt::CreateOptionsBuilder::new()
            .server_uri(DEFAULT_BROKER_URL)
            .client_id(client_id)
            .finalize();

        let cli = mqtt::Client::new(create_opts).unwrap();

        let conn_opts = mqtt::ConnectOptionsBuilder::new()
            .clean_session(true)
            .finalize();

        cli.connect(conn_opts).unwrap();

        let message = mqtt::MessageBuilder::new()
            .topic(topic)
            .qos(1)
            .retained(true)
            .payload(payload)
            .finalize();

        cli.publish(message).unwrap();

        if cli.is_connected() {
            cli.disconnect(None).unwrap();
        }
    }

    #[test]
    fn good_request() {
        let client_id = "good_request";
        let topic = "good/request";
        let payload = r#"{"id":"test_id"}"#;

        let mut srv = initialize_server();
        let req = request(&srv, client_id, topic, topic, payload);
        let resp = srv.execute(req.send()).unwrap();

        assert!(resp.status().is_success());
        assert_eq!(resp.body().wait().unwrap(), payload.as_bytes());
    }

    #[test]
    fn call_with_retained_message_in_subscribe_topic() {
        let client_id = "call_with_retained";
        let topic = "call/with/retained";
        let payload_retained = "test_r";
        let payload = r#"{"id":"test_id"}"#;

        publish_retained(client_id, topic, payload_retained);

        let mut srv = initialize_server();
        let req = request(&srv, client_id, topic, topic, payload);
        let resp = srv.execute(req.send()).unwrap();

        assert!(resp.status().is_success());
        assert_eq!(resp.body().wait().unwrap(), payload.as_bytes());
    }

    #[test]
    fn without_answer_from_broker() {
        let client_id = "without_answer_from_broker";
        let publish_topic = "wafb/1";
        let subscribe_topic = "wafb/2";
        let payload = r#"{"id":"test_id"}"#;

        let mut srv = initialize_server();
        let req = request(&srv, client_id, publish_topic, subscribe_topic, payload);
        let resp = srv.execute(req.send()).unwrap();

        assert!(resp.status().is_server_error());
    }

}
