use std::time::Duration;

use anyhow::{Context, Result};
use futures::{sync::mpsc, Future, Stream};
use log::{error, info};
use serde_json::Value as JsonValue;

use crate::util::headers::Headers;

////////////////////////////////////////////////////////////////////////////////

const DEFAULT_TIMEOUT: u64 = 5;

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    timeout: Option<u64>,
}

impl Config {
    pub(crate) fn timeout(&self) -> u64 {
        self.timeout.unwrap_or_else(|| DEFAULT_TIMEOUT)
    }
}

////////////////////////////////////////////////////////////////////////////////

type HttpClient = reqwest::r#async::Client;

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub(crate) struct OutgoingMessage {
    payload: JsonValue,
    headers: Headers,
    uri: String,
    token: String,
}

impl OutgoingMessage {
    pub(crate) fn new(payload: JsonValue, headers: Headers, uri: &str, token: &str) -> Self {
        Self {
            payload,
            headers,
            uri: uri.to_owned(),
            token: token.to_owned(),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub(crate) struct OutgoingStream {
    tx: mpsc::UnboundedSender<OutgoingMessage>,
}

impl OutgoingStream {
    pub(crate) fn new(config: &Config) -> (Self, impl Future<Item = (), Error = ()>) {
        let (tx, rx) = mpsc::unbounded::<OutgoingMessage>();
        let object = Self { tx };

        let client = HttpClient::builder()
            .timeout(Duration::from_secs(config.timeout()))
            .build()
            .expect("Error creating HTTP client");

        let ostream = rx.for_each(move |outev| Self::send_handler(&client, outev));
        (object, ostream)
    }

    pub(crate) fn send(&self, message: OutgoingMessage) -> Result<()> {
        self.tx
            .unbounded_send(message)
            .context("error sending message to the outgoing HTTP stream")
    }

    fn send_handler(
        client: &HttpClient,
        outev: OutgoingMessage,
    ) -> impl Future<Item = (), Error = ()> {
        client
            .post(&outev.uri)
            .bearer_auth(outev.token.to_owned())
            .headers(outev.headers.to_header_map())
            .json(&outev.payload)
            .send()
            .then(move |resp| {
                match resp {
                    Ok(ref res) if res.status().is_success() => {
                        info!("Message payload = '{}' sent successfully to the HTTP callback = '{}'",
                              &outev.payload, &outev.uri,
                        );
                    }
                    Ok(res) => {
                        error!(
                            "Error with status code = '{}' on sending the message payload = '{}' to the HTTP callback = '{}'",
                            res.status(), &outev.payload, &outev.uri,
                        );
                    }
                    Err(e) => {
                        error!(
                            "Network error on sending the message payload = '{}' to the HTTP callback = '{}', {}",
                            &outev.payload, &outev.uri, e,
                        );
                    }
                }

                Ok(())
            })
    }
}

////////////////////////////////////////////////////////////////////////////////
