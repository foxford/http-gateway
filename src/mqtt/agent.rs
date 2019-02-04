use std::collections::HashMap;

use crossbeam::{Receiver, TryRecvError};
use failure::{err_msg, Error};
use futures::{sync::oneshot, Async, Poll, Stream};
use log::info;
use rumqtt::{MqttClient, MqttOptions, Notification, QoS, ReconnectOptions};

use super::{
    compat::IntoEnvelope, AgentOptions, IncomingResponse, OutgoingRequest, Publishable,
    ResponseSubscription, Source, SubscriptionTopic,
};
use crate::authn::{AccountId, AgentId};

#[derive(Debug)]
pub struct AgentBuilder {
    agent_id: AgentId,
}

impl AgentBuilder {
    pub fn new(agent_id: AgentId) -> Self {
        Self { agent_id }
    }

    pub fn start(self, config: &AgentOptions) -> Result<(Agent, MqttStream), Error> {
        let client_id = Self::mqtt_client_id(&self.agent_id);
        let options = Self::mqtt_options(&client_id, &config)?;
        let (client, rx) = MqttClient::start(options)?;

        let mut agent = Agent::new(self.agent_id, client);
        agent
            .tx
            .subscribe(agent.anyone_input_subscription()?, QoS::AtLeastOnce)?;

        let notifications = MqttStream(rx);

        Ok((agent, notifications))
    }

    fn mqtt_client_id(agent_id: &AgentId) -> String {
        format!("v1.mqtt3/bridge-agents/{agent_id}", agent_id = agent_id)
    }

    fn mqtt_options(client_id: &str, config: &AgentOptions) -> Result<MqttOptions, Error> {
        let uri = config.uri.parse::<http::Uri>()?;
        let host = uri.host().ok_or_else(|| err_msg("missing MQTT host"))?;
        let port = uri
            .port_part()
            .ok_or_else(|| err_msg("missing MQTT port"))?;

        let opts = MqttOptions::new(client_id, host, port.as_u16())
            .set_keep_alive(30)
            .set_reconnect_opts(ReconnectOptions::AfterFirstSuccess(5));

        Ok(opts)
    }
}

pub struct Agent {
    id: AgentId,
    tx: rumqtt::MqttClient,
    input_topic: String,
    in_flight_requests: HashMap<uuid::Uuid, oneshot::Sender<IncomingResponse<String>>>,
}

impl Agent {
    fn new(id: AgentId, tx: MqttClient) -> Self {
        let input_topic = format!("agents/{agent}/api/v1/in", agent = id);
        println!("{}", input_topic);

        Self {
            id,
            tx,
            input_topic,
            in_flight_requests: HashMap::new(),
        }
    }

    pub fn publish(
        &mut self,
        message: OutgoingRequest<String>,
    ) -> Result<oneshot::Receiver<IncomingResponse<String>>, Error> {
        let correlation_data = message.properties.correlation_data;

        let message = message.into_envelope()?;

        info!("{:?}", message);

        let topic = message.destination_topic(&self.id)?;
        let bytes = message.to_bytes()?;

        self.tx
            .publish(topic, QoS::AtLeastOnce, false, bytes)
            .map_err(Error::from)?;

        let (sender, receiver) = oneshot::channel();

        self.in_flight_requests.insert(correlation_data, sender);

        Ok(receiver)
    }

    pub fn finish_request(
        &mut self,
        correlation_data: uuid::Uuid,
        response: IncomingResponse<String>,
    ) {
        if let Some(in_flight_request) = self.in_flight_requests.remove(&correlation_data) {
            in_flight_request.send(response).unwrap();
        }
    }

    fn anyone_input_subscription(&self) -> Result<String, Error> {
        let src = Source::Unicast(None);
        let sub = ResponseSubscription::new(src);
        sub.subscription_topic(&self.id)
    }

    // TODO: overhead?
    pub fn response_topic(&self, account_id: &AccountId) -> Result<String, Error> {
        let src = Source::Unicast(Some(account_id));
        let sub = ResponseSubscription::new(src);
        sub.subscription_topic(&self.id)
    }
}

pub struct MqttStream(Receiver<Notification>);

impl Stream for MqttStream {
    type Item = Notification;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.0.try_recv() {
            Ok(notification) => Ok(Async::Ready(Some(notification))),
            Err(TryRecvError::Empty) => Ok(Async::NotReady),
            // stop stream if channel has been dismantled
            Err(TryRecvError::Disconnected) => Ok(Async::Ready(None)),
        }
    }
}
