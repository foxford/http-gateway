use std::str::FromStr;

use failure::Error;
use futures::{future, sync::oneshot::Receiver, Future, IntoFuture};
use futures_locks::Mutex;
use serde_derive::Deserialize;
use tower_web::{impl_web, Extract};

use crate::authn::{AccountId, AgentId};
use crate::mqtt::{
    Agent, Destination, IncomingResponse, OutgoingRequest, OutgoingRequestProperties,
};

pub struct HttpGatewayApp {
    client: Mutex<Agent>,
}

impl HttpGatewayApp {
    pub fn new(client: Mutex<Agent>) -> Self {
        Self { client }
    }
}

#[derive(Debug, Extract, Deserialize)]
struct RequestData {
    me: String,
    destination: RequestDestination,
    payload: String,
    method: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type")]
enum RequestDestination {
    Unicast { value: String },
    Multicast { value: String },
}

impl RequestDestination {
    fn validate(self) -> Result<Destination, Error> {
        let dest = match self {
            RequestDestination::Unicast { value } => {
                let agent_id = AgentId::from_str(&value)?;
                Destination::Unicast(agent_id)
            }
            RequestDestination::Multicast { value } => {
                let account_id = AccountId::from_str(&value)?;
                Destination::Multicast(account_id)
            }
        };

        Ok(dest)
    }
}

impl HttpGatewayApp {
    fn request_sync<T: std::ops::DerefMut<Target = Agent>>(
        req: RequestData,
        mut client: T,
    ) -> Result<Receiver<IncomingResponse<serde_json::Value>>, Error> {
        let destination = req.destination.validate()?;

        let dest_account_id = match &destination {
            Destination::Unicast(agent_id) => agent_id.account_id(),
            Destination::Multicast(ref account_id) => account_id,
            Destination::Broadcast(..) => unreachable!(),
        };

        let agent_id = AgentId::from_str(&req.me)?;

        let props = OutgoingRequestProperties::new(
            req.method,
            client.response_topic(dest_account_id)?,
            Some(agent_id.into()),
        );

        let out = OutgoingRequest::new(req.payload, props, destination);

        client.publish(out)
    }
}

impl_web! {
    impl HttpGatewayApp {
        #[post("/api/v1/request")]
        fn request(&self, body: RequestData) -> impl Future<Item = serde_json::Value, Error = Error> {
            self
                .client.lock()
                .map_err(|_| failure::err_msg("Unexpected error during lock acqusition"))
                .and_then(|client| Self::request_sync(body, client).into_future()
                .and_then(|f| f.map_err(Error::from))
                .and_then(|response| {
                    let (payload, _props) = response.destructure();
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
            "destination": {
                "value": "conference.netology-group.services",
                "type": "multicast",
            },
            "payload": "test",
            "method": "room.create",
        });

        let d: RequestData = serde_json::from_value(val).unwrap();

        dbg!(d);
    }
}
