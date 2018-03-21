use actix::prelude::*;
use actix_web::*;
use actors::mqtt_handler::Params;
use defaults::*;
use http::header::{HeaderMap, ToStrError};

pub struct RequestParser;

impl Actor for RequestParser {
    type Context = SyncContext<Self>;
}

type RPCCallParseResult = Result<Params, ToStrError>;

pub struct RPCCallParse {
    pub headers: HeaderMap,
}

impl Message for RPCCallParse {
    type Result = RPCCallParseResult;
}

impl Handler<RPCCallParse> for RequestParser {
    type Result = RPCCallParseResult;

    fn handle(&mut self, msg: RPCCallParse, _ctx: &mut Self::Context) -> Self::Result {
        let client_id = msg.headers[CLIENT_ID_HEADER].to_str()?;
        let username = msg.headers[USERNAME_HEADER].to_str()?;
        let password = msg.headers[PASSWORD_HEADER].to_str()?;
        let lwt = msg.headers[LWT_HEADER].to_str()?;
        let lwm = msg.headers[LWM_HEADER].to_str()?;
        let request_topic = msg.headers[REQUEST_TOPIC_HEADER].to_str()?;
        let request_qos = msg.headers[REQUEST_QOS_HEADER].to_str()?;
        let request_qos = request_qos.parse().unwrap_or_else(|_| DEFAULT_QOS);
        let response_topic = msg.headers[RESPONSE_TOPIC_HEADER].to_str()?;
        let response_qos = msg.headers[RESPONSE_QOS_HEADER].to_str()?;
        let response_qos = response_qos.parse().unwrap_or_else(|_| DEFAULT_QOS);
        Ok(Params {
            client_id: client_id.to_string(),
            username: username.to_string(),
            password: password.to_string(),
            last_will_topic: lwt.to_string(),
            last_will_message: lwm.to_string(),
            request_topic: request_topic.to_string(),
            request_qos: request_qos,
            response_topic: response_topic.to_string(),
            response_qos: response_qos,
        })
    }
}
