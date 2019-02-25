use std::collections::HashMap;
use std::str::FromStr;

use failure::Error;
use futures::{future, sync::oneshot, Future, IntoFuture};
use futures_locks::{Mutex, MutexFut};
use serde_derive::Deserialize;
use svc_agent::mqtt::{
    compat::IntoEnvelope, Agent, IncomingResponse, OutgoingRequest, OutgoingRequestProperties,
    SubscriptionTopic,
};
use svc_agent::{AgentId, ResponseSubscription, Source};
use svc_authn::AccountId;
use tower_web::{impl_web, Extract};

pub struct InFlightRequests {
    map: HashMap<uuid::Uuid, oneshot::Sender<IncomingResponse<serde_json::Value>>>,
}

impl InFlightRequests {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn save_request(
        &mut self,
        request: uuid::Uuid,
    ) -> oneshot::Receiver<IncomingResponse<serde_json::Value>> {
        let (sender, receiver) = oneshot::channel();
        self.map.insert(request, sender);

        receiver
    }

    pub fn finish_request(
        &mut self,
        request: uuid::Uuid,
        response: IncomingResponse<serde_json::Value>,
    ) {
        if let Some(in_flight_request) = self.map.remove(&request) {
            in_flight_request.send(response).unwrap();
        }
    }
}

pub struct RequestResource {
    pub agent: Mutex<Agent>,
    pub in_flight_requests: Mutex<InFlightRequests>,
}

impl RequestResource {
    pub fn new(agent: Agent, in_flight_requests: Mutex<InFlightRequests>) -> Self {
        Self {
            agent: Mutex::new(agent),
            in_flight_requests,
        }
    }
}

#[derive(Debug, Extract, Deserialize)]
struct RequestData {
    me: String,
    destination: String,
    payload: serde_json::Value,
    method: String,
}

impl RequestResource {
    fn lock(&self) -> future::Join<MutexFut<Agent>, MutexFut<InFlightRequests>> {
        self.agent.lock().join(self.in_flight_requests.lock())
    }

    fn request_sync(
        req: RequestData,
        agent: &mut Agent,
        in_flight_requests: &mut InFlightRequests,
    ) -> Result<oneshot::Receiver<IncomingResponse<serde_json::Value>>, Error> {
        let me = AgentId::from_str(&req.me)?;
        let destination = AccountId::from_str(&req.destination)?;

        let src = Source::Unicast(Some(&destination));
        let sub = ResponseSubscription::new(src);
        let response_topic = sub.subscription_topic(agent.id())?;

        let correlation_data = uuid::Uuid::new_v4();
        let mut props = OutgoingRequestProperties::new(
            &req.method,
            &response_topic,
            &correlation_data.to_string(),
        );
        props.set_authn(me.into());

        let out = OutgoingRequest::multicast(req.payload, props, &destination);
        agent.publish(&out.into_envelope()?)?;

        Ok(in_flight_requests.save_request(correlation_data))
    }
}

impl_web! {
    impl RequestResource {
        #[post("/api/v1/request")]
        fn request(&self, body: RequestData, _account_id: AccountId) -> impl Future<Item = serde_json::Value, Error = Error> {
            self
                .lock()
                .map_err(|_| failure::err_msg("Unexpected error during lock acqusition"))
                .and_then(|(mut agent, mut in_flight_requests)| Self::request_sync(
                    body,
                    &mut agent,
                    &mut in_flight_requests
                ).into_future()
                .and_then(|f| f.map_err(Error::from))
                .and_then(|response| {
                    let payload = response.payload().clone();
                    future::ok(payload)
                }))
        }
    }
}

#[cfg(test)]
mod test {
    use serde_json::{self, json};

    use super::RequestData;

    #[test]
    fn ser() {
        let val = json!({
            "me": "web.12345.netology.ru",
            "destination": "conference.netology-group.services",
            "payload": "test",
            "method": "room.create",
        });

        let d: RequestData = serde_json::from_value(val).unwrap();

        dbg!(d);
    }
}
