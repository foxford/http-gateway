extern crate paho_mqtt as mqtt;

use actix::prelude::*;
use actix_web::*;

use std::time::Duration;

pub struct MqttHandler;

impl Actor for MqttHandler {
    type Context = SyncContext<Self>;
}

pub struct Params {
    pub client_id: String,
    pub username: String,
    pub password: String,
    pub last_will_topic: String,
    pub last_will_message: String,
    pub request_topic: String,
    pub request_qos: u8,
    pub response_topic: String,
    pub response_qos: u8,
}

pub struct RPCCall {
    pub broker_addr: String,
    pub params: Params,
    pub payload: Vec<u8>,
}

impl Message for RPCCall {
    type Result = Result<String, mqtt::MqttError>;
}

impl Handler<RPCCall> for MqttHandler {
    type Result = Result<String, mqtt::MqttError>;

    fn handle(&mut self, msg: RPCCall, _ctx: &mut Self::Context) -> Self::Result {
        let create_opts = mqtt::CreateOptionsBuilder::new()
            .server_uri(msg.broker_addr)
            .client_id(msg.params.client_id)
            .finalize();

        let mut cli = mqtt::Client::new(create_opts)?;

        let lwt = mqtt::MessageBuilder::new()
            .topic(&msg.params.last_will_topic)
            .payload(msg.params.last_will_message.as_bytes())
            .finalize();

        let conn_opts = mqtt::ConnectOptionsBuilder::new()
            .keep_alive_interval(Duration::from_secs(20))
            .clean_session(true)
            .will_message(lwt)
            .finalize();

        debug!("Connecting to the MQTT broker...");
        cli.connect(conn_opts)?;

        debug!("Subscribing to topics...");
        let rx = cli.start_consuming();

        let subscriptions = [&msg.params.response_topic];

        cli.subscribe_many(&subscriptions, &[msg.params.response_qos as i32])?;

        let message = mqtt::Message::new(
            &msg.params.request_topic,
            msg.payload,
            msg.params.request_qos as i32,
        );

        debug!("Publishing message: {:?}", message);
        cli.publish(message)?;

        let mut response = String::new();
        debug!("Waiting for messages...");
        for m in rx.iter() {
            if let Some(m) = m {
                response = m.get_payload_str().unwrap();
                break;
            } else {
                break;
            }
        }

        if cli.is_connected() {
            debug!("Trying to unsubscribe");
            cli.unsubscribe_many(&subscriptions)?;
            debug!("Trying to disconnect");
            cli.disconnect(None)?;
        }

        return Ok(response);
    }
}
