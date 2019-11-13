#[macro_use]
extern crate tower_web;

pub fn run(config_filename: &str) {
    app::run(config_filename);
}

mod app;
mod serde;
mod util;
