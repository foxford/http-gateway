use std::collections::HashMap;

use crate::authn::AccountId;

pub type ConfigMap = HashMap<String, Config>;

#[derive(Debug, Deserialize)]
pub struct Config {
    callback: String,
    apps: Vec<AccountId>,
}
