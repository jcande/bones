use url;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use std::ops::AddAssign;
use std::ops::SubAssign;
use std::borrow::Cow;

// This is recommended for debug builds.
extern crate console_error_panic_hook;

mod view_port;
mod renderer;
mod dispatch;
mod mosaic;

mod compiler;
mod constraint;
mod io_buffer;
mod tessera;
mod tiling;
mod wmach;


const SCREEN_SAVER_MODE: bool = false;
const RULE110_MODE: bool = false;


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
    let href = window.location().href()?;
    let url = url::Url::parse(&href)
        .or(Err(JsValue::from_str("unable to parse url")))?;

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

    let border_option = document.get_element_by_id("border")
        .ok_or(JsValue::from_str("unable to locate checkbox \"border\" in document"))?
        .dyn_into::<web_sys::HtmlElement>()?;
    let tile_lines_option = document.get_element_by_id("tile_lines")
        .ok_or(JsValue::from_str("unable to locate checkbox \"tile_lines\" in document"))?
        .dyn_into::<web_sys::HtmlElement>()?;

    let color_add_option = document.get_element_by_id("palette_add")
        .ok_or(JsValue::from_str("unable to locate number field \"palette_add\" in document"))?
        .dyn_into::<web_sys::HtmlElement>()?;
    let color_mul_option = document.get_element_by_id("palette_mul")
        .ok_or(JsValue::from_str("unable to locate number field \"palette_mul\" in document"))?
        .dyn_into::<web_sys::HtmlElement>()?;

    // this is a scary interaction from the html page. Anyway, we have a container div that takes
    // up the whole viewport. We now expand the canvas to the dimensions of this container
    // effectively making it the fullscreen. This is blowup when you resize so don't.
    canvas.set_width(container.offset_width().try_into().expect("someone hates you"));
    canvas.set_height(container.offset_height().try_into().expect("someone hates you"));

    let state_element = document.get_element_by_id("state_link")
        .ok_or(JsValue::from_str("unable to locate shareable \"state_link\" in document"))?
        .dyn_into::<web_sys::HtmlElement>()?;
    let attr = state_element.attributes();
    let state = attr.get_named_item("href").expect("The state_link element needs an href");

    let context = canvas
        .get_context("2d")?
        .ok_or(JsValue::from_str("unable to retrieve 2d context from domino canvas"))?
        .dyn_into::<web_sys::CanvasRenderingContext2d>()?;

    let params = dispatch::Parameters {
        window: window,
        url: url,
        state: state,

        container: container,
        canvas: canvas,
        context: context,

        border: border_option,
        tile_lines: tile_lines_option,
        color_add: color_add_option,
        color_mul: color_mul_option,
    };

    if let Err(e) = main(params) {
        panic!("{}", e);
    }

    Ok(())
}

fn main(params: dispatch::Parameters) -> anyhow::Result<()> {

    let src = params.url.query_pairs()
        .find(|(key, _)| key == "src")
        .map_or(
            Cow::from(String::from_utf8_lossy(std::include_bytes!("wasm.wm"))),
            |(_, value)| value);

    let mosaic = mosaic::Mosaic::new(&src)?;
    let _dispatch = dispatch::Dispatch::new(mosaic, params);

    Ok(())
}
