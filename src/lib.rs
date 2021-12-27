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
    // Use `web_sys`'s global `window` function to get a handle on the global
    // window object.
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let body = document.body().expect("document should have a body");

    // Manufacture the element we're gonna append
    let val = document.create_element("p")?;
    val.set_inner_html("Hello from Rust!");

    body.append_child(&val)?;

    Ok(())
}

/*
pub fn main() {
    println!("Hello, world!");
}
*/
