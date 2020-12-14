use std::convert::TryFrom;

use anyhow::{format_err, Context, Error};
use http::header::{HeaderMap, HeaderName, HeaderValue};
use serde::ser::Serialize;
use svc_agent::{mqtt::IncomingMessageContent, Addressable};

#[derive(Debug)]
pub(crate) struct Headers(Vec<(String, String)>);

impl Headers {
    pub(crate) fn add_to_header_map(&self, header_map: &mut HeaderMap) {
        header_map.reserve(self.0.len());

        for (name_str, value_str) in self.0.iter() {
            let prefixed_name = format!("gateway-{}", name_str.replace("_", "-"));

            if let Ok(name) = HeaderName::from_bytes(prefixed_name.as_bytes()) {
                if let Ok(value) = HeaderValue::from_bytes(value_str.as_bytes()) {
                    header_map.append(name, value);
                }
            }
        }
    }

    pub(crate) fn to_header_map(&self) -> HeaderMap {
        let mut header_map = HeaderMap::new();
        self.add_to_header_map(&mut header_map);
        header_map
    }
}

impl<T, P: Addressable + Serialize> TryFrom<&IncomingMessageContent<T, P>> for Headers {
    type Error = Error;

    fn try_from(message: &IncomingMessageContent<T, P>) -> Result<Self, Self::Error> {
        let props_value =
            serde_json::to_value(message.properties()).context("Failed to serialize properties")?;

        let props_object = props_value
            .as_object()
            .ok_or_else(|| format_err!("Properties is not an object"))?;

        let mut headers = Vec::with_capacity(props_object.len());
        headers.push((
            "User-Agent".into(),
            format!(
                "{} through http-gateway",
                message.properties().as_agent_id()
            ),
        ));

        for (key, json_value) in props_object.into_iter() {
            let value = json_value
                .as_str()
                .ok_or_else(|| format_err!("Property '{}' has not a string value", key))?;

            headers.push((key.to_owned(), value.to_string()));
        }

        Ok(Self(headers))
    }
}
