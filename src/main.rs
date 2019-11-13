fn main() {
    env_logger::init();
    http_gateway::run("App.toml");
}
