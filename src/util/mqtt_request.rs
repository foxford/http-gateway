use failure::{err_msg, Error};
use futures::sync::oneshot;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use svc_agent::mqtt::Agent;

////////////////////////////////////////////////////////////////////////////////

pub(crate) type IncomingResponse = svc_agent::mqtt::IncomingResponse<JsonValue>;
pub(crate) type OutgoingRequest = svc_agent::mqtt::OutgoingRequest<JsonValue>;

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

    pub(crate) fn request(
        &mut self,
        req: OutgoingRequest,
    ) -> Result<oneshot::Receiver<IncomingResponse>, Error> {
        let id = req.properties().correlation_data().to_owned();

        use svc_agent::mqtt::compat::IntoEnvelope;
        let envelope = req.into_envelope()?;
        self.tx.publish(&envelope)?;

        let (tx, rx) = oneshot::channel();
        self.store.insert(id, tx);

        Ok(rx)
    }

    pub(crate) fn commit_response(&mut self, resp: IncomingResponse) -> Result<(), Error> {
        let id = resp.properties().correlation_data();
        if let Some(tx) = self.store.remove(id) {
            return tx.send(resp).map_err(|_| {
                err_msg("error committing incoming MQTT response, a receiver may have been already destroyed by timeout")
            });
        }

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////
