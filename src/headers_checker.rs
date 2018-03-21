use actix_web::*;
use actix_web::middleware::{Middleware, Started};
use defaults::*;

pub struct HeadersChecker;

impl<S> Middleware<S> for HeadersChecker {
    fn start(&self, req: &mut HttpRequest<S>) -> Result<Started> {
        if !req.headers().contains_key(CLIENT_ID_HEADER)
            || !req.headers().contains_key(RESPONSE_TOPIC_HEADER)
            || !req.headers().contains_key(RESPONSE_QOS_HEADER)
            || !req.headers().contains_key(USERNAME_HEADER)
            || !req.headers().contains_key(PASSWORD_HEADER)
            || !req.headers().contains_key(REQUEST_QOS_HEADER)
            || !req.headers().contains_key(LWT_HEADER)
            || !req.headers().contains_key(LWM_HEADER)
            || !req.headers().contains_key(REQUEST_TOPIC_HEADER)
        {
            return Ok(Started::Response(HttpResponse::build(
                StatusCode::BAD_REQUEST,
            ).finish()?));
        }

        Ok(Started::Done)
    }
}
