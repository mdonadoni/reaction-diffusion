#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub(crate) fn log(s: &str);

}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn log(s: &str) {
    eprintln!("{}", s);
}
