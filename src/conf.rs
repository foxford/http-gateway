use config;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub id: svc_authn::AccountId,
    pub authn: svc_authn::jose::ConfigMap,
    pub web: crate::web::Config,
    pub mqtt: svc_agent::mqtt::AgentConfig,
    #[serde(default)]
    pub events: crate::event::ConfigMap,
}

pub fn load() -> Result<Config, config::ConfigError> {
    let mut parser = config::Config::default();
    parser.merge(config::File::with_name("App"))?;
    parser.merge(config::Environment::with_prefix("APP").separator("__"))?;
    parser.try_into::<Config>()
}
