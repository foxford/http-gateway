use std::fmt;

use crossbeam::Receiver;
use failure::{err_msg, Error};
use log::info;
use rumqtt::{MqttClient, MqttOptions, Notification, QoS, ReconnectOptions};

use super::{
    compat::IntoEnvelope, AgentOptions, OutgoingRequest, Publishable, SharedGroup,
    SubscriptionTopic,
};
use crate::authn::AgentId;

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum ConnectionMode {
    Agent,
    Bridge,
}

impl fmt::Display for ConnectionMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ConnectionMode::Agent => "agents",
                ConnectionMode::Bridge => "bridge-agents",
            }
        )
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct AgentBuilder {
    agent_id: AgentId,
    version: String,
    mode: ConnectionMode,
}

impl AgentBuilder {
    pub fn new(agent_id: AgentId) -> Self {
        Self {
            agent_id,
            version: String::from("v1.mqtt3"),
            mode: ConnectionMode::Agent,
        }
    }

    pub fn version(self, version: &str) -> Self {
        Self {
            agent_id: self.agent_id,
            version: version.to_owned(),
            mode: self.mode,
        }
    }

    pub fn mode(self, mode: ConnectionMode) -> Self {
        Self {
            agent_id: self.agent_id,
            version: self.version,
            mode,
        }
    }

    pub fn start(self, config: &AgentOptions) -> Result<(Agent, Receiver<Notification>), Error> {
        let client_id = self.mqtt_client_id();
        info!("Client Id: {}", client_id);

        let options = Self::mqtt_options(&client_id, &config)?;
        let (client, rx) = MqttClient::start(options)?;

        let agent = Agent::new(self.agent_id, client);

        Ok((agent, rx))
    }

    fn mqtt_client_id(&self) -> String {
        format!(
            "{version}/{mode}/{agent_id}",
            version = self.version,
            mode = self.mode,
            agent_id = self.agent_id,
        )
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
}

impl Agent {
    fn new(id: AgentId, tx: MqttClient) -> Self {
        Self { id, tx }
    }

    pub fn id(&self) -> &AgentId {
        &self.id
    }

    pub fn publish(&mut self, message: OutgoingRequest<String>) -> Result<(), Error> {
        let message = message.into_envelope()?;

        info!("{:?}", message);

        let topic = message.destination_topic(&self.id)?;
        let bytes = message.to_bytes()?;

        self.tx
            .publish(topic, QoS::AtLeastOnce, false, bytes)
            .map_err(Error::from)
    }

    pub(crate) fn subscribe<S>(
        &mut self,
        subscription: &S,
        qos: QoS,
        maybe_group: Option<&SharedGroup>,
    ) -> Result<(), Error>
    where
        S: SubscriptionTopic,
    {
        let mut topic = subscription.subscription_topic(&self.id)?;
        if let Some(ref group) = maybe_group {
            topic = format!("$share/{group}/{topic}", group = group, topic = topic);
        };

        self.tx.subscribe(topic, qos)?;
        Ok(())
    }
}
