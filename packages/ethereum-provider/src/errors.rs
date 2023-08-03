use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub(crate) fn error(s: &str);
}

macro_rules! console_error {
    ($($t:tt)*) => (crate::errors::error(&format_args!($($t)*).to_string()))
}
pub(crate) use console_error;
