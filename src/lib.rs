// TODO think of a more uniform/consistent/obvious way to deal with zooming. We also want to avoid
// small floats as that is inaccurate
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use std::ops::AddAssign;
use std::ops::SubAssign;

// This is recommended for debug builds.
extern crate console_error_panic_hook;

mod view_port;
mod renderer;
mod dispatch;
mod calcada;

mod compiler;
mod constraint;
mod io_buffer;
mod mosaic;
mod tiling;
mod wmach;


const SHOW_LINES: bool = false;
const SHOW_BORDER_TILES: bool = true;
const SCREEN_SAVER_MODE: bool = false;


#[derive(PartialEq, Clone, Copy, Debug)]
pub struct Coord {
    pub x: i32,
    pub y: i32,
}
impl Coord {
    fn new(x: i32, y: i32) -> Self {
        Self {
            x: x,
            y: y,
        }
    }
}
impl AddAssign for Coord {
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            x: self.x + other.x,
            y: self.y + other.y,
        };
    }
}
impl SubAssign for Coord {
    fn sub_assign(&mut self, other: Self) {
        *self = Self {
            x: self.x - other.x,
            y: self.y - other.y,
        };
    }
}

#[wasm_bindgen(start)]
pub fn js_main() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let window = web_sys::window()
        .ok_or(JsValue::from_str("no global window exists"))?;
    let document = window
        .document()
        .ok_or(JsValue::from_str("should have a document on window"))?;
    // the intent is to grab it and then we can expand/contract the canvas with this.
    let container = document.get_element_by_id("domino-div")
        .ok_or(JsValue::from_str("unable to locate domino container \"domino-div\" in document"))?
        .dyn_into::<web_sys::HtmlElement>()?;
    let canvas = document.get_element_by_id("domino")
        .ok_or(JsValue::from_str("unable to locate domino canvas \"domino\" in document"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;

    // this is a scary interaction from the html page. Anyway, we have a container div that takes
    // up the whole viewport. We now expand the canvas to the dimensions of this container
    // effectively making it the fullscreen. This is blowup when you resize so don't.
    canvas.set_width(container.offset_width().try_into().expect("someone hates you"));
    canvas.set_height(container.offset_height().try_into().expect("someone hates you"));

    let context = canvas
        .get_context("2d")?
        .ok_or(JsValue::from_str("unable to retrieve 2d context from domino canvas"))?
        .dyn_into::<web_sys::CanvasRenderingContext2d>()?;

    if let Err(e) = main(window, container, canvas, context) {
        panic!("{}", e);
    }

    Ok(())
}

fn main(window: web_sys::Window, container: web_sys::HtmlElement, canvas: web_sys::HtmlCanvasElement, context: web_sys::CanvasRenderingContext2d) -> anyhow::Result<()> {

    let calcada = calcada::Calcada::new()?;
    let _dispatch = dispatch::Dispatch::new(calcada, window, container, canvas, context);

    Ok(())
}
