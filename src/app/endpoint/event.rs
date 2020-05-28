use std::collections::HashMap;
use std::convert::TryFrom;

use anyhow::{bail, format_err, Result};
use serde_json::Value as JsonValue;
use svc_agent::{AccountId, Authenticable};

use crate::util::headers::Headers;
use crate::util::http_stream::OutgoingMessage;

////////////////////////////////////////////////////////////////////////////////

pub(crate) type ConfigMap = HashMap<String, Config>;

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Config {
    callback: String,
    sources: Vec<AccountId>,
}

impl Config {
    pub(crate) fn callback(&self) -> &str {
        &self.callback
    }

    pub(crate) fn sources(&self) -> &Vec<AccountId> {
        &self.sources
    }
}

////////////////////////////////////////////////////////////////////////////////

type IncomingEvent = svc_agent::mqtt::IncomingEvent<JsonValue>;

////////////////////////////////////////////////////////////////////////////////

pub(crate) struct State {
    config: ConfigMap,
    tokens: HashMap<String, String>,
}

impl State {
    pub(crate) fn new(config: ConfigMap, tokens: HashMap<String, String>) -> Self {
        Self { config, tokens }
    }

    pub(crate) fn handle(&self, topic: &str, inev: &IncomingEvent) -> Result<OutgoingMessage> {
        let from_account_id = inev.properties().as_account_id();
        let audience = extract_audience(topic)?;

        let config = self.config.get(audience).ok_or_else(|| {
            format_err!(
                "sending events for audience = '{}' is not allowed",
                audience
            )
        })?;

        if !config.sources().contains(from_account_id) {
            bail!(
                "sending events for audience = '{}' from application = '{}' is not allowed",
                audience,
                from_account_id
            );
        }

        let token = match self.tokens.get(audience) {
            Some(value) => value,
            None => bail!("missing token for audience = '{}'", audience),
        };

        Ok(OutgoingMessage::new(
            inev.payload().clone(),
            Headers::try_from(inev)?,
            config.callback(),
            token,
        ))
    }
}

//////////////////////////////////////////////////////////////////////////////////

fn extract_audience(topic: &str) -> Result<&str> {
    use std::ffi::OsStr;
    use std::path::{Component, Path};

    let topic_path = Path::new(topic);
    let mut topic = topic_path.components();

    let events_literal = Some(Component::Normal(OsStr::new("events")));

    if topic.next_back() != events_literal {
        bail!(
            "topic does not match the pattern 'audiences/AUDIENCE/events': {}",
            topic_path.display()
        );
    }

    let maybe_audience = topic.next_back();

    let audiences_literal = Some(Component::Normal(OsStr::new("audiences")));
    if topic.next_back() == audiences_literal {
        match maybe_audience {
            Some(Component::Normal(audience)) => {
                let audience = audience.to_str().ok_or_else(|| {
                    format_err!(
                        "non utf-8 characters in audience name: {}",
                        topic_path.display()
                    )
                })?;

                Ok(audience)
            }
            _ => bail!(
                "topic does not match the pattern 'audiences/AUDIENCE/events': {}",
                topic_path.display()
            ),
        }
    } else {
        bail!(
            "topic does not match the pattern 'audiences/AUDIENCE/events': {}",
            topic_path.display()
        );
    }
}

//////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod test {
    #[test]
    fn extracts_audience() {
        let topic = "test/test/test/audiences/test-audience/events";
        let res = super::extract_audience(topic);

        assert!(res.is_ok());
        assert_eq!("test-audience", res.unwrap());

        let topic = "test/test/test/audiences";
        let res = super::extract_audience(topic);
        assert!(res.is_err());
        dbg!(res);

        let topic = "test/test/test/audiences/events";
        let res = super::extract_audience(topic);
        assert!(res.is_err());
        dbg!(res);
    }
}
