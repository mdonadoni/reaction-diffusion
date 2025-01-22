#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

mod app;
mod config;
mod diffusion;
mod event;
mod log;

pub use crate::app::App;
pub use crate::config::Config;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn web_init() {
    console_error_panic_hook::set_once();
}
