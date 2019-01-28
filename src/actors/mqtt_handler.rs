use mqtt;
use serde_json;
use actix::prelude::*;
use extractors::parsing::RequestId;
use extractors::rpc_call_extractor::RPCCallParams;
use errors::Result;
use std::result;

pub struct MqttHandler;

impl Actor for MqttHandler {
    type Context = SyncContext<Self>;
}

#[derive(Debug)]
pub struct RPCCallMessage {
    pub id: String,
    pub broker_url: String,
    pub params: RPCCallParams,
    pub payload: Vec<u8>,
}

impl Message for RPCCallMessage {
    type Result = Result<String>;
}

impl Handler<RPCCallMessage> for MqttHandler {
    type Result = Result<String>;

    fn handle(&mut self, msg: RPCCallMessage, _ctx: &mut Self::Context) -> Self::Result {
        let create_opts = mqtt::CreateOptionsBuilder::new()
            .server_uri(msg.broker_url)
            .client_id(msg.params.client_id)
            .finalize();

        let mut cli = mqtt::Client::new(create_opts)?;

        let conn_opts =
            if msg.params.last_will_topic.is_some() && msg.params.last_will_message.is_some() {
                let lwt = mqtt::MessageBuilder::new()
                    .topic(msg.params.last_will_topic.unwrap())
                    .payload(msg.params.last_will_message.unwrap().as_bytes())
                    .finalize();

                mqtt::ConnectOptionsBuilder::new()
                    .clean_session(true)
                    .will_message(lwt)
                    .finalize()
            } else {
                mqtt::ConnectOptionsBuilder::new()
                    .clean_session(true)
                    .finalize()
            };

        debug!("Connecting to the MQTT broker...");
        cli.connect(conn_opts)?;

        debug!("Subscribing to topics...");
        let rx = cli.start_consuming();

        let subscriptions = [&msg.params.subscribe_topic];

        cli.subscribe_many(&subscriptions, &[msg.params.subscribe_qos as i32])?;

        let message = mqtt::MessageBuilder::new()
            .topic(msg.params.publish_topic)
            .qos(msg.params.publish_qos as i32)
            .retained(msg.params.publish_retain)
            .payload(msg.payload)
            .finalize();

        debug!("Publishing message: {:?}", message);
        cli.publish(message)?;

        let mut response = String::new();
        debug!("Waiting for messages...");
        for m in rx.iter() {
            if let Some(m) = m {
                let r = m.payload_str();
                let id: result::Result<RequestId, serde_json::Error> = serde_json::from_str(&r);
                if id.is_err() {
                    continue;
                }
                if id.unwrap().id == msg.id {
                    response = r.to_string();
                    break;
                }
            } else {
                continue;
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
