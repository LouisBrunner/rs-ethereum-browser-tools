use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub(crate) fn log(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    pub(crate) fn error(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (crate::console::log(&format_args!($($t)*).to_string()))
}
pub(crate) use console_log;

macro_rules! console_error {
    ($($t:tt)*) => (crate::console::error(&format_args!($($t)*).to_string()))
}
pub(crate) use console_error;