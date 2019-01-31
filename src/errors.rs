use actix_web;
use actix_web::{error, http, HttpResponse};
use failure::Fail;
use serde_json;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Fail, Debug)]
pub enum Error {
    // #[fail(display = "mqtt error: {}", _0)]
    // MqttError(mqtt::MqttError),
    #[fail(display = "parse string error: {}", _0)]
    Utf8Error(std::string::FromUtf8Error),
    #[fail(display = "parse json error: {}", _0)]
    JsonError(serde_json::Error),
    #[fail(display = "parse header error: {}", _0)]
    HeaderError(actix_web::http::header::ToStrError),
    #[fail(display = "parse integer error: {}", _0)]
    IntError(std::num::ParseIntError),
    #[fail(display = "missed header: {}", _0)]
    MissedHeader(&'static str),
    #[fail(display = "bad header: {}", _0)]
    BadHeader(&'static str),
}

impl error::ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        match *self {
            // Error::MqttError(_) => HttpResponse::new(http::StatusCode::INTERNAL_SERVER_ERROR),
            Error::Utf8Error(_) => HttpResponse::new(http::StatusCode::BAD_REQUEST),
            Error::JsonError(_) => HttpResponse::new(http::StatusCode::BAD_REQUEST),
            Error::HeaderError(_) => HttpResponse::new(http::StatusCode::BAD_REQUEST),
            Error::IntError(_) => HttpResponse::new(http::StatusCode::BAD_REQUEST),
            Error::MissedHeader(_) => HttpResponse::new(http::StatusCode::BAD_REQUEST),
            Error::BadHeader(_) => HttpResponse::new(http::StatusCode::BAD_REQUEST),
        }
    }
}

// impl From<mqtt::MqttError> for Error {
//     fn from(err: mqtt::MqttError) -> Error {
//         Error::MqttError(err)
//     }
// }

impl From<std::string::FromUtf8Error> for Error {
    fn from(err: std::string::FromUtf8Error) -> Error {
        Error::Utf8Error(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Error {
        Error::JsonError(err)
    }
}

impl From<actix_web::http::header::ToStrError> for Error {
    fn from(err: actix_web::http::header::ToStrError) -> Error {
        Error::HeaderError(err)
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(err: std::num::ParseIntError) -> Error {
        Error::IntError(err)
    }
}
