use actix_web::http;
use bytes::Bytes;
use serde_derive::{Deserialize, Serialize};
use serde_json;

use crate::errors::{Error, Result};

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestId {
    pub id: String,
}

pub fn get_request_id(body: &Bytes) -> Option<String> {
    serde_json::from_slice::<RequestId>(&body)
        .ok()
        .map(|obj| obj.id)
}

#[inline]
pub fn parse_header_to_string(
    headers: &http::header::HeaderMap,
    header_name: &'static str,
) -> Result<String> {
    let param = headers
        .get(header_name)
        .ok_or(Error::MissedHeader(header_name))?;
    let param = param.to_str()?;
    Ok(param.to_string())
}

#[inline]
pub fn parse_qos_header(
    headers: &http::header::HeaderMap,
    header_name: &'static str,
) -> Result<u8> {
    let param = headers
        .get(header_name)
        .ok_or(Error::MissedHeader(header_name))?;
    let param = param.to_str()?;
    let param = param.parse()?;
    if param != 0 && param != 1 && param != 2 {
        return Err(Error::BadHeader(header_name));
    }
    Ok(param)
}

#[inline]
pub fn parse_last_will_header(
    headers: &http::header::HeaderMap,
    header_name: &'static str,
) -> Result<Option<String>> {
    let param = headers.get(header_name);
    if param.is_none() {
        return Ok(None);
    }
    let param = param.unwrap();
    let param = param.to_str()?;
    Ok(Some(param.to_string()))
}

#[inline]
pub fn parse_retain_header(
    headers: &http::header::HeaderMap,
    header_name: &'static str,
) -> Result<bool> {
    let param = headers.get(header_name);
    if param.is_none() {
        return Ok(false);
    }
    let param = param.unwrap();
    let param = param.to_str()?;
    if param == "1" {
        Ok(true)
    } else if param == "0" {
        Ok(false)
    } else {
        Err(Error::BadHeader(header_name))
    }
}
