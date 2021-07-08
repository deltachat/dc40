#![recursion_limit = "1024"]

#[macro_use]
extern crate validator_derive;

mod app;
mod components;

pub fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::start_app::<app::App>();
}
