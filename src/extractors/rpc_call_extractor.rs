use actix_web;
use actix_web::{FromRequest, HttpMessage, HttpRequest};
use futures::future::{result, FutureResult};

use crate::defaults::*;
use crate::errors::Result;
use crate::extractors::parsing::*;

#[derive(Debug, Clone)]
pub struct RPCCallParams {
    pub client_id: String,
    pub username: String,
    pub password: String,
    pub publish_topic: String,
    pub publish_qos: u8,
    pub publish_retain: bool,
    pub subscribe_topic: String,
    pub subscribe_qos: u8,
    pub last_will_topic: Option<String>,
    pub last_will_message: Option<String>,
}

impl<S: 'static> FromRequest<S> for RPCCallParams {
    type Config = ();
    type Result = FutureResult<Self, actix_web::Error>;

    #[inline]
    fn from_request(req: &HttpRequest<S>, _: &Self::Config) -> Self::Result {
        result(RPCCallParams::new(req).map_err(|e| e.into()))
    }
}

impl RPCCallParams {
    pub fn new<S>(req: &HttpRequest<S>) -> Result<RPCCallParams> {
        let headers = req.headers();
        let client_id = parse_header_to_string(headers, CLIENT_ID_HEADER)?;
        let username = parse_header_to_string(headers, USERNAME_HEADER)?;
        let password = parse_header_to_string(headers, PASSWORD_HEADER)?;
        let publish_topic = parse_header_to_string(headers, PUBLISH_TOPIC_HEADER)?;
        let subscribe_topic = parse_header_to_string(headers, SUBSCRIBE_TOPIC_HEADER)?;
        let publish_qos = parse_qos_header(headers, PUBLISH_QOS_HEADER)?;
        let subscribe_qos = parse_qos_header(headers, SUBSCRIBE_QOS_HEADER)?;
        let last_will_topic = parse_last_will_header(headers, LAST_WILL_TOPIC_HEADER)?;
        let last_will_message = parse_last_will_header(headers, LAST_WILL_MESSAGE_HEADER)?;
        let publish_retain = parse_retain_header(headers, PUBLISH_RETAIN_HEADER)?;

        Ok(RPCCallParams {
            client_id: client_id.to_string(),
            username: username.to_string(),
            password: password.to_string(),
            publish_topic: publish_topic.to_string(),
            publish_qos: publish_qos,
            publish_retain: publish_retain,
            subscribe_topic: subscribe_topic.to_string(),
            subscribe_qos: subscribe_qos,
            last_will_topic: last_will_topic,
            last_will_message: last_will_message,
        })
    }
}
