use std::fmt;
use std::str::FromStr;

use ::serde::Serialize;
use failure::{format_err, Error};
use serde_derive::Deserialize;

use crate::authn::{AccountId, AgentId, Authenticable};
pub use agent::{Agent, AgentBuilder};

mod agent;
pub mod compat;
mod serde;

#[derive(Debug, Deserialize)]
pub struct AgentOptions {
    pub uri: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct SharedGroup {
    label: String,
    account_id: AccountId,
}

impl SharedGroup {
    pub(crate) fn new(label: &str, account_id: AccountId) -> Self {
        Self {
            label: label.to_owned(),
            account_id,
        }
    }
}

impl fmt::Display for SharedGroup {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}", self.label, self.account_id)
    }
}

impl FromStr for SharedGroup {
    type Err = Error;

    fn from_str(val: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = val.splitn(2, '.').collect();
        match parts[..] {
            [ref label, ref rest] => Ok(Self::new(label, rest.parse::<AccountId>()?)),
            _ => Err(format_err!(
                "invalid value for the application group: {}",
                val
            )),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Serialize)]
pub(crate) enum Destination {
    Broadcast(BroadcastUri),
    Multicast(AccountId),
    Unicast(AgentId),
}

pub(crate) type BroadcastUri = String;

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct AuthnProperties {
    agent_label: String,
    account_label: String,
    audience: String,
}

impl From<AgentId> for AuthnProperties {
    fn from(agent: AgentId) -> Self {
        Self {
            agent_label: agent.label().to_owned(),
            account_label: agent.account_id().label().to_owned(),
            audience: agent.account_id().audience().to_owned(),
        }
    }
}

impl From<&AuthnProperties> for AccountId {
    fn from(authn: &AuthnProperties) -> Self {
        AccountId::new(&authn.account_label, &authn.audience)
    }
}

impl From<&AuthnProperties> for AgentId {
    fn from(authn: &AuthnProperties) -> Self {
        AgentId::new(&authn.agent_label, AccountId::from(authn))
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Deserialize)]
pub(crate) struct IncomingEventProperties {
    #[serde(flatten)]
    authn: AuthnProperties,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct IncomingRequestProperties {
    method: String,
    correlation_data: String,
    response_topic: String,
    #[serde(flatten)]
    authn: AuthnProperties,
}

impl IncomingRequestProperties {
    pub(crate) fn new(
        method: &str,
        correlation_data: &str,
        response_topic: &str,
        authn: AuthnProperties,
    ) -> Self {
        Self {
            method: method.to_owned(),
            correlation_data: correlation_data.to_owned(),
            response_topic: response_topic.to_owned(),
            authn,
        }
    }

    pub(crate) fn method(&self) -> &str {
        &self.method
    }

    pub(crate) fn to_response(
        &self,
        status: &'static OutgoingResponseStatus,
    ) -> OutgoingResponseProperties {
        OutgoingResponseProperties::new(status, &self.correlation_data, Some(&self.response_topic))
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IncomingResponseProperties {
    correlation_data: String,
    #[serde(flatten)]
    authn: AuthnProperties,
}

impl IncomingResponseProperties {
    pub fn correlation_data(&self) -> &str {
        &self.correlation_data
    }
}

impl Authenticable for IncomingEventProperties {
    fn account_id(&self) -> AccountId {
        AccountId::from(&self.authn)
    }

    fn agent_id(&self) -> AgentId {
        AgentId::from(&self.authn)
    }
}

impl Authenticable for IncomingRequestProperties {
    fn account_id(&self) -> AccountId {
        AccountId::from(&self.authn)
    }

    fn agent_id(&self) -> AgentId {
        AgentId::from(&self.authn)
    }
}

impl Authenticable for IncomingResponseProperties {
    fn account_id(&self) -> AccountId {
        AccountId::from(&self.authn)
    }

    fn agent_id(&self) -> AgentId {
        AgentId::from(&self.authn)
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Deserialize, Serialize)]
pub struct IncomingMessage<T, P>
where
    P: Authenticable,
{
    payload: T,
    properties: P,
}

impl<T, P> IncomingMessage<T, P>
where
    P: Authenticable,
{
    pub fn new(payload: T, properties: P) -> Self {
        Self {
            payload,
            properties,
        }
    }

    pub fn payload(&self) -> &T {
        &self.payload
    }

    pub fn properties(&self) -> &P {
        &self.properties
    }

    pub fn destructure(self) -> (T, P) {
        (self.payload, self.properties)
    }
}

impl<T> IncomingRequest<T> {
    pub(crate) fn to_response<R>(
        &self,
        data: R,
        status: &'static OutgoingResponseStatus,
    ) -> OutgoingResponse<R>
    where
        R: Serialize,
    {
        OutgoingMessage::new(
            data,
            self.properties.to_response(status),
            Destination::Unicast(self.properties.agent_id()),
        )
    }
}

pub(crate) type IncomingEvent<T> = IncomingMessage<T, IncomingEventProperties>;
pub(crate) type IncomingRequest<T> = IncomingMessage<T, IncomingRequestProperties>;
pub(crate) type IncomingResponse<T> = IncomingMessage<T, IncomingResponseProperties>;

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Serialize)]
pub(crate) struct OutgoingEventProperties {
    #[serde(rename = "type")]
    kind: &'static str,
}

impl OutgoingEventProperties {
    pub(crate) fn new(kind: &'static str) -> Self {
        Self { kind }
    }
}

#[derive(Debug, Serialize)]
pub struct OutgoingRequestProperties {
    method: String,
    correlation_data: uuid::Uuid,
    response_topic: String,
    #[serde(flatten)]
    authn: Option<AuthnProperties>,
}

impl OutgoingRequestProperties {
    pub(crate) fn new(
        method: String,
        response_topic: String,
        authn: Option<AuthnProperties>,
    ) -> Self {
        Self {
            method,
            response_topic,
            authn,
            correlation_data: uuid::Uuid::new_v4(),
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct OutgoingResponseProperties {
    #[serde(with = "serde::HttpStatusCodeRef")]
    status: &'static OutgoingResponseStatus,
    correlation_data: String,
    #[serde(skip)]
    response_topic: Option<String>,
}

impl OutgoingResponseProperties {
    pub(crate) fn new(
        status: &'static OutgoingResponseStatus,
        correlation_data: &str,
        response_topic: Option<&str>,
    ) -> Self {
        Self {
            status,
            correlation_data: correlation_data.to_owned(),
            response_topic: response_topic.map(|val| val.to_owned()),
        }
    }
}

pub(crate) type OutgoingResponseStatus = http::StatusCode;

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Serialize)]
pub struct OutgoingMessage<T, P>
where
    T: Serialize,
{
    payload: T,
    properties: P,
    destination: Destination,
}

impl<T, P> OutgoingMessage<T, P>
where
    T: Serialize,
{
    pub(crate) fn new(payload: T, properties: P, destination: Destination) -> Self {
        Self {
            payload,
            properties,
            destination,
        }
    }
}

pub(crate) type OutgoingEvent<T> = OutgoingMessage<T, OutgoingEventProperties>;
pub(crate) type OutgoingRequest<T> = OutgoingMessage<T, OutgoingRequestProperties>;
pub(crate) type OutgoingResponse<T> = OutgoingMessage<T, OutgoingResponseProperties>;

impl<T> compat::IntoEnvelope for OutgoingEvent<T>
where
    T: Serialize,
{
    fn into_envelope(self) -> Result<compat::OutgoingEnvelope, Error> {
        let payload = serde_json::to_string(&self.payload)?;
        let envelope = compat::OutgoingEnvelope::new(
            &payload,
            compat::OutgoingEnvelopeProperties::Event(self.properties),
            self.destination,
        );
        Ok(envelope)
    }
}

impl<T> compat::IntoEnvelope for OutgoingRequest<T>
where
    T: Serialize,
{
    fn into_envelope(self) -> Result<compat::OutgoingEnvelope, Error> {
        let payload = serde_json::to_string(&self.payload)?;
        let envelope = compat::OutgoingEnvelope::new(
            &payload,
            compat::OutgoingEnvelopeProperties::Request(self.properties),
            self.destination,
        );
        Ok(envelope)
    }
}

impl<T> compat::IntoEnvelope for OutgoingResponse<T>
where
    T: Serialize,
{
    fn into_envelope(self) -> Result<compat::OutgoingEnvelope, Error> {
        let payload = serde_json::to_string(&self.payload)?;
        let envelope = compat::OutgoingEnvelope::new(
            &payload,
            compat::OutgoingEnvelopeProperties::Response(self.properties),
            self.destination,
        );
        Ok(envelope)
    }
}

////////////////////////////////////////////////////////////////////////////////

pub trait Publishable {
    fn destination_topic(&self, agent_id: &AgentId) -> Result<String, Error>;
    fn to_bytes(&self) -> Result<String, Error>;
}

////////////////////////////////////////////////////////////////////////////////

// pub(crate) trait Publish<'a> {
//     fn publish(&'a self, tx: &mut Agent) -> Result<(), Error>;
// }

// impl<'a, T> Publish<'a> for T
// where
//     T: Publishable,
// {
//     fn publish(&'a self, tx: &mut Agent) -> Result<(), Error> {
//         tx.publish(self)?;
//         Ok(())
//     }
// }

// impl<'a, T1, T2> Publish<'a> for (T1, T2)
// where
//     T1: Publishable,
//     T2: Publishable,
// {
//     fn publish(&'a self, tx: &mut Agent) -> Result<(), Error> {
//         tx.publish(&self.0)?;
//         tx.publish(&self.1)?;
//         Ok(())
//     }
// }

////////////////////////////////////////////////////////////////////////////////

trait DestinationTopic {
    fn destination_topic(&self, agent_id: &AgentId, dest: &Destination) -> Result<String, Error>;
}

impl DestinationTopic for OutgoingEventProperties {
    fn destination_topic(&self, agent_id: &AgentId, dest: &Destination) -> Result<String, Error> {
        match dest {
            Destination::Broadcast(ref dest_uri) => Ok(format!(
                "apps/{app_name}/api/v1/{uri}",
                app_name = &agent_id.account_id(),
                uri = dest_uri,
            )),
            _ => Err(format_err!(
                "destination = '{:?}' incompatible with event message type",
                dest,
            )),
        }
    }
}

impl DestinationTopic for OutgoingRequestProperties {
    fn destination_topic(&self, agent_id: &AgentId, dest: &Destination) -> Result<String, Error> {
        match dest {
            Destination::Unicast(ref dest_agent_id) => Ok(format!(
                "agents/{agent_id}/api/v1/in/{app_name}",
                agent_id = dest_agent_id,
                app_name = agent_id.account_id(),
            )),
            Destination::Multicast(ref dest_account_id) => Ok(format!(
                "agents/{agent_id}/api/v1/out/{app_name}",
                agent_id = agent_id,
                app_name = dest_account_id,
            )),
            _ => Err(format_err!(
                "destination = '{:?}' incompatible with request message type",
                dest,
            )),
        }
    }
}

impl DestinationTopic for OutgoingResponseProperties {
    fn destination_topic(&self, agent_id: &AgentId, dest: &Destination) -> Result<String, Error> {
        match &self.response_topic {
            Some(ref val) => Ok(val.to_owned()),
            None => match dest {
                Destination::Unicast(ref dest_agent_id) => Ok(format!(
                    "agents/{agent_id}/api/v1/in/{app_name}",
                    agent_id = dest_agent_id,
                    app_name = &agent_id.account_id(),
                )),
                _ => Err(format_err!(
                    "destination = '{:?}' incompatible with response message type",
                    dest,
                )),
            },
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub(crate) enum Source {
    Broadcast(AccountId, BroadcastUri),
    Multicast,
    Unicast(AccountId),
}

pub(crate) trait SubscriptionTopic {
    fn subscription_topic(&self, agent_id: &AgentId) -> Result<String, Error>;
}

pub(crate) struct EventSubscription {
    source: Source,
}

impl EventSubscription {
    pub(crate) fn new(source: Source) -> Self {
        Self { source }
    }
}

impl SubscriptionTopic for EventSubscription {
    fn subscription_topic(&self, _: &AgentId) -> Result<String, Error> {
        match self.source {
            Source::Broadcast(ref source_account_id, ref source_uri) => Ok(format!(
                "apps/{app_name}/api/v1/{uri}",
                app_name = source_account_id,
                uri = source_uri,
            )),
            _ => Err(format_err!(
                "source = '{:?}' is incompatible with event subscription",
                self.source,
            )),
        }
    }
}

pub(crate) struct RequestSubscription {
    source: Source,
}

impl RequestSubscription {
    pub(crate) fn new(source: Source) -> Self {
        Self { source }
    }
}

impl SubscriptionTopic for RequestSubscription {
    fn subscription_topic(&self, agent_id: &AgentId) -> Result<String, Error> {
        match self.source {
            Source::Unicast(ref source_account_id) => Ok(format!(
                "agents/{agent_id}/api/v1/in/{app_name}",
                agent_id = agent_id,
                app_name = source_account_id,
            )),
            Source::Multicast => Ok(format!(
                "agents/+/api/v1/out/{app_name}",
                app_name = agent_id.account_id(),
            )),
            _ => Err(format_err!(
                "source = '{:?}' is incompatible with request subscription",
                self.source,
            )),
        }
    }
}

pub(crate) struct ResponseSubscription {
    source: Source,
}

impl ResponseSubscription {
    pub(crate) fn new(source: Source) -> Self {
        Self { source }
    }
}

impl SubscriptionTopic for ResponseSubscription {
    fn subscription_topic(&self, agent_id: &AgentId) -> Result<String, Error> {
        match self.source {
            Source::Unicast(ref source_account_id) => Ok(format!(
                "agents/{agent_id}/api/v1/in/{app_name}",
                agent_id = agent_id,
                app_name = source_account_id,
            )),
            _ => Err(format_err!(
                "source = '{:?}' is incompatible with response subscription",
                self.source,
            )),
        }
    }
}
