use std::collections::HashMap;

use futures::{sync::mpsc, Future, Stream};
use log::{error, info};
use reqwest::r#async::Client;
use svc_agent::mqtt::IncomingEvent;
use svc_authn::AccountId;

pub type ConfigMap = HashMap<String, Config>;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub callback: String,
    pub sources: Vec<AccountId>,
}

#[derive(Debug)]
pub struct Event {
    event: IncomingEvent<serde_json::Value>,
    uri: String,
}

impl Event {
    pub fn new(event: IncomingEvent<serde_json::Value>, uri: String) -> Self {
        Self { event, uri }
    }
}

pub fn event_handler() -> (
    mpsc::UnboundedSender<Event>,
    impl Stream<Item = (), Error = ()>,
) {
    let (sender, receiver) = mpsc::unbounded();
    let client = Client::new();

    let receiver = receiver.and_then(move |event: Event| {
        client
            .post(&event.uri)
            .json(event.event.payload())
            .send()
            .then(move |res| {
                match res {
                    Ok(ref res) if res.status().is_success() => {
                        info!("Event sent successfully. Event: {:?}", event);
                    }
                    Ok(res) => {
                        error!(
                            "Event sent with {} status code. Event: {:?}",
                            res.status(),
                            event
                        );
                    }

                    Err(err) => {
                        error!(
                            "Error during tenant notification: {}. Event: {:?}",
                            err, event
                        );
                    }
                }
                Ok(())
            })
    });

    (sender, receiver)
}
