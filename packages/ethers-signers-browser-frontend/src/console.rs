use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub(crate) fn error(s: &str);
}

#[allow(unused_macros)]
macro_rules! console_error {
    ($($t:tt)*) => (crate::console::error(&format_args!($($t)*).to_string()))
}
#[allow(unused_imports)]
pub(crate) use console_error;
