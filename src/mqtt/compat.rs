use super::{
    Destination, DestinationTopic, IncomingEvent, IncomingEventProperties, IncomingMessage,
    IncomingRequest, IncomingRequestProperties, IncomingResponse, IncomingResponseProperties,
    OutgoingEventProperties, OutgoingRequestProperties, OutgoingResponseProperties, Publishable,
};
use crate::authn::AgentId;
use failure::{err_msg, format_err, Error};
use serde_derive::{Deserialize, Serialize};

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type")]
pub(crate) enum IncomingEnvelopeProperties {
    Event(IncomingEventProperties),
    Request(IncomingRequestProperties),
    Response(IncomingResponseProperties),
}

#[derive(Debug, Deserialize)]
pub(crate) struct IncomingEnvelope {
    payload: String,
    properties: IncomingEnvelopeProperties,
}

impl IncomingEnvelope {
    pub(crate) fn properties(&self) -> &IncomingEnvelopeProperties {
        &self.properties
    }

    pub(crate) fn payload<T>(&self) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let payload = serde_json::from_str::<T>(&self.payload)?;
        Ok(payload)
    }
}

pub(crate) fn into_event<T>(envelope: IncomingEnvelope) -> Result<IncomingEvent<T>, Error>
where
    T: serde::de::DeserializeOwned,
{
    let payload = envelope.payload::<T>()?;
    match envelope.properties {
        IncomingEnvelopeProperties::Event(props) => Ok(IncomingMessage::new(payload, props)),
        val => Err(format_err!("error converting into event = {:?}", val)),
    }
}

pub(crate) fn into_request<T>(envelope: IncomingEnvelope) -> Result<IncomingRequest<T>, Error>
where
    T: serde::de::DeserializeOwned,
{
    let payload = envelope.payload::<T>()?;
    match envelope.properties {
        IncomingEnvelopeProperties::Request(props) => Ok(IncomingMessage::new(payload, props)),
        _ => Err(err_msg("Error converting into request")),
    }
}

pub(crate) fn into_response<T>(envelope: IncomingEnvelope) -> Result<IncomingResponse<T>, Error>
where
    T: serde::de::DeserializeOwned,
{
    let payload = envelope.payload::<T>()?;
    match envelope.properties {
        IncomingEnvelopeProperties::Response(props) => Ok(IncomingMessage::new(payload, props)),
        _ => Err(err_msg("error converting into response")),
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type")]
pub(crate) enum OutgoingEnvelopeProperties {
    Event(OutgoingEventProperties),
    Request(OutgoingRequestProperties),
    Response(OutgoingResponseProperties),
}

#[derive(Debug, Serialize)]
pub(crate) struct OutgoingEnvelope {
    payload: String,
    properties: OutgoingEnvelopeProperties,
    #[serde(skip)]
    destination: Destination,
}

impl OutgoingEnvelope {
    pub(crate) fn new(
        payload: &str,
        properties: OutgoingEnvelopeProperties,
        destination: Destination,
    ) -> Self {
        Self {
            payload: payload.to_owned(),
            properties,
            destination,
        }
    }
}

impl DestinationTopic for OutgoingEnvelopeProperties {
    fn destination_topic(&self, agent_id: &AgentId, dest: &Destination) -> Result<String, Error> {
        match self {
            OutgoingEnvelopeProperties::Event(val) => val.destination_topic(agent_id, dest),
            OutgoingEnvelopeProperties::Request(val) => val.destination_topic(agent_id, dest),
            OutgoingEnvelopeProperties::Response(val) => val.destination_topic(agent_id, dest),
        }
    }
}

impl<'a> Publishable for OutgoingEnvelope {
    fn destination_topic(&self, agent_id: &AgentId) -> Result<String, Error> {
        self.properties
            .destination_topic(agent_id, &self.destination)
    }

    fn to_bytes(&self) -> Result<String, Error> {
        Ok(serde_json::to_string(&self)?)
    }
}

////////////////////////////////////////////////////////////////////////////////

pub(crate) trait IntoEnvelope {
    fn into_envelope(self) -> Result<OutgoingEnvelope, Error>;
}
