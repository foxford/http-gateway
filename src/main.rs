#[macro_use]
extern crate tower_web;

fn main() {
    env_logger::init();
    app::run();
}

mod app;
mod serde;
mod util;
