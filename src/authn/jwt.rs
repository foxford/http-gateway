use http::{header::HeaderValue, StatusCode};
use jsonwebtoken::{decode, TokenData, Validation};
use serde_derive::{Deserialize, Serialize};
use tower_web::{
    extract::{Context, Error as ExtractError, Extract, Immediate},
    util::BufStream,
    Error, ErrorBuilder,
};

use crate::authn::{AccountId, ConfigMap};

////////////////////////////////////////////////////////////////////////////////

impl<B: BufStream> Extract<B> for AccountId {
    type Future = Immediate<AccountId>;

    fn extract(context: &Context) -> Self::Future {
        let authn = context
            .config::<ConfigMap>()
            .expect("missing an authn config");
        match context.request().headers().get(http::header::AUTHORIZATION) {
            Some(header) => match extract_account_id(&header, authn) {
                Ok(sub) => Immediate::ok(sub),
                Err(err) => Immediate::err(err),
            },
            None => Immediate::err(missing_token_error().into()),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

fn extract_account_id(header: &HeaderValue, authn: &ConfigMap) -> Result<AccountId, ExtractError> {
    let token = parse_bearer_token(header)?;
    let parts = parse_jws_compact(token)?;
    let config = authn.get(&parts.claims.iss).ok_or_else(|| {
        let detail = format!(
            "issuer = {} of the authentication token is not allowed",
            &parts.claims.iss,
        );
        ExtractError::from(error().detail(&detail).build())
    })?;

    let mut verifier = Validation::new(config.algorithm);
    verifier.set_audience(&config.audience);
    verifier.validate_exp = parts.claims.exp.is_some();

    decode_account_id(token, &verifier, config.key.as_ref())
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Claims {
    iss: String,
    aud: String,
    sub: String,
    exp: Option<u64>,
}

fn parse_bearer_token(header: &HeaderValue) -> Result<&str, ExtractError> {
    let val: Vec<&str> = header
        .to_str()
        .map_err(|_| {
            ExtractError::from(
                error()
                    .detail("invalid characters in the authorization header")
                    .build(),
            )
        })?
        .split(' ')
        .collect();

    match val[..] {
        ["Bearer", ref val] => Ok(val),
        _ => Err(error()
            .detail("unsupported or invalid type of the authentication token")
            .build()
            .into()),
    }
}

fn parse_jws_compact(token: &str) -> Result<TokenData<Claims>, ExtractError> {
    jsonwebtoken::dangerous_unsafe_decode(token).map_err(|_| {
        ExtractError::from(
            error()
                .detail("invalid claims of the authentication token")
                .build(),
        )
    })
}

fn decode_account_id(
    token: &str,
    verifier: &Validation,
    key: &[u8],
) -> Result<AccountId, ExtractError> {
    let data = decode::<Claims>(token, key, &verifier).map_err(|_| {
        ExtractError::from(
            error()
                .detail("verification of the authentication token failed")
                .build(),
        )
    })?;

    Ok(AccountId::new(&data.claims.sub, &data.claims.aud))
}

////////////////////////////////////////////////////////////////////////////////

fn error() -> ErrorBuilder {
    Error::builder()
        .kind("authn_error", "Error processing the authentication token")
        .status(StatusCode::UNAUTHORIZED)
}

fn missing_token_error() -> Error {
    Error::builder()
        .kind("authn_error", "Error processing the authentication token")
        .status(StatusCode::FORBIDDEN)
        .detail("missing authentication token")
        .build()
}
