use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    Window,
    CanvasRenderingContext2d,
    HtmlCanvasElement,
    HtmlImageElement
};

#[wasm_bindgen(start)]
pub fn js_main() -> Result<(), JsValue> {
    Ok(())
}

/*
pub fn main() {
    println!("Hello, world!");
}
*/
