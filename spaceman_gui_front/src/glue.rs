use wasm_bindgen::prelude::*;
use js_sys::{JsString, Function};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "tauri"], catch)]
    pub async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsString>;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
    pub fn emit(chan: &str, payload: JsValue);

    // The returned valhe is the function that unregisters the handler
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
    pub async fn listen(chan: &str, callback: &Closure<dyn FnMut(JsValue)>) -> JsValue;

    #[wasm_bindgen]
    pub async fn initMonaco(element: JsValue, editorName: &str, readOnly: bool);

    #[wasm_bindgen]
    pub fn monacoAddTab(editorName: &str) -> i32;

    #[wasm_bindgen]
    pub fn monacoDelTab(editorName: &str, idx: i32);

    #[wasm_bindgen]
    pub fn monacoDeselect(editorName: &str);

    #[wasm_bindgen]
    pub fn monacoGoToTab(editorName: &str, idx: i32);

    #[wasm_bindgen]
    pub fn monacoRead(editorName: &str, idx: i32) -> JsString;

    #[wasm_bindgen]
    pub fn monacoWrite(editorName: &str, idx: i32, value: &str);

    #[wasm_bindgen]
    pub fn monacoLayout(editorName: &str);
}
