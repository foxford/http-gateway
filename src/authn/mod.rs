use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use failure::{format_err, Error};
use jsonwebtoken::Algorithm;

pub mod jwt;
mod serde;

pub type ConfigMap = HashMap<String, Config>;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    audience: String,
    #[serde(deserialize_with = "serde::algorithm")]
    algorithm: Algorithm,
    #[serde(deserialize_with = "serde::file")]
    key: Vec<u8>,
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct AccountId {
    label: String,
    audience: String,
}

impl AccountId {
    pub(crate) fn new(label: &str, audience: &str) -> Self {
        Self {
            label: label.to_owned(),
            audience: audience.to_owned(),
        }
    }

    pub(crate) fn audience(&self) -> &str {
        &self.audience
    }

    pub(crate) fn label(&self) -> &str {
        &self.label
    }
}

impl fmt::Display for AccountId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}", self.label, self.audience)
    }
}

impl FromStr for AccountId {
    type Err = Error;

    fn from_str(val: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = val.splitn(2, '.').collect();
        match parts[..] {
            [ref label, ref audience] => Ok(Self::new(label, audience)),
            _ => Err(format_err!(
                "invalid value for the application name: {}",
                val
            )),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentId {
    label: String,
    account_id: AccountId,
}

impl AgentId {
    pub(crate) fn new(label: &str, account_id: AccountId) -> Self {
        Self {
            label: label.to_owned(),
            account_id,
        }
    }

    pub(crate) fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    pub(crate) fn label(&self) -> &str {
        &self.label
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}.{}.{}",
            self.label, self.account_id.label, self.account_id.audience,
        )
    }
}

impl FromStr for AgentId {
    type Err = Error;

    fn from_str(val: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = val.splitn(3, '.').collect();
        match parts[..] {
            [ref agent_label, ref account_label, ref audience] => {
                let account_id = AccountId::new(account_label, audience);
                let agent_id = Self::new(agent_label, account_id);
                Ok(agent_id)
            }
            _ => Err(format_err!("invalid value for the agent id: {}", val)),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub trait Authenticable {
    fn account_id(&self) -> AccountId;
    fn agent_id(&self) -> AgentId;
}
