#[macro_use]
extern crate tower_web;

pub fn run() {
    app::run();
}

mod app;
mod serde;
mod util;
