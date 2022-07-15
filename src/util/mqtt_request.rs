use anyhow::{format_err, Result};
use futures::sync::oneshot;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use svc_agent::mqtt::{Agent, OutgoingMessage};
use svc_agent::AgentId;

////////////////////////////////////////////////////////////////////////////////

pub(crate) type IncomingResponse = svc_agent::mqtt::IncomingResponse<JsonValue>;

////////////////////////////////////////////////////////////////////////////////

pub(crate) struct Adapter {
    tx: Agent,
    store: HashMap<String, oneshot::Sender<IncomingResponse>>,
}

////////////////////////////////////////////////////////////////////////////////

impl Adapter {
    pub(crate) fn new(tx: Agent) -> Self {
        Self {
            tx,
            store: HashMap::new(),
        }
    }

    pub(crate) fn id(&self) -> &AgentId {
        self.tx.id()
    }

    pub(crate) fn request<T: serde::Serialize>(
        &mut self,
        req: OutgoingMessage<T>,
    ) -> Result<oneshot::Receiver<IncomingResponse>> {
        let id = match req {
            OutgoingMessage::Request(ref req) => req.properties().correlation_data().to_owned(),
            _ => return Err(format_err!("Wrong message type")),
        };
        self.tx.publish(req)?;

        let (tx, rx) = oneshot::channel();
        self.store.insert(id, tx);

        Ok(rx)
    }

    pub(crate) fn commit_response(&mut self, resp: IncomingResponse) -> Result<()> {
        let id = resp.properties().correlation_data();

        if let Some(tx) = self.store.remove(id) {
            return tx.send(resp).map_err(|_| {
                format_err!("error committing incoming MQTT response, a receiver may have been already destroyed by timeout")
            });
        }

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////
