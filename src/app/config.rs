use config;

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    pub(crate) id: svc_authn::AccountId,
    pub(crate) agent_label: String,
    pub(crate) authn: svc_authn::jose::ConfigMap,
    pub(crate) mqtt: svc_agent::mqtt::AgentConfig,
    pub(crate) http: crate::app::HttpConfig,
    pub(crate) http_client: crate::util::http_stream::Config,
    #[serde(default)]
    pub(crate) events: crate::app::endpoint::event::ConfigMap,
}

pub(crate) fn load() -> Result<Config, config::ConfigError> {
    let mut parser = config::Config::default();
    parser.merge(config::File::with_name("App"))?;
    parser.merge(config::Environment::with_prefix("APP").separator("__"))?;
    parser.try_into::<Config>()
}
