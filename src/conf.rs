use config;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub(crate) id: crate::authn::AccountId,
    pub(crate) authn: crate::authn::ConfigMap,
    pub(crate) web: crate::web::Config,
    pub(crate) mqtt: crate::mqtt::AgentOptions,
}

pub fn load() -> Result<Config, config::ConfigError> {
    let mut parser = config::Config::default();
    parser.merge(config::File::with_name("App"))?;
    parser.merge(config::Environment::with_prefix("APP").separator("__"))?;
    parser.try_into::<Config>()
}
