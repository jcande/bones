#![feature(generic_const_exprs)]
#![feature(const_generics_defaults)]
#![feature(type_ascription)]

// TODO think of a more uniform/consistent/obvious way to deal with zooming. We also want to avoid
// small floats as that is inaccurate
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use std::ops::AddAssign;
use std::ops::SubAssign;
extern crate console_error_panic_hook;

mod tiling;
mod view_port;
mod renderer;
mod dispatch;

// A macro to provide `println!(..)`-style syntax for `console.log` logging.
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

/*
 * requires:
 * #![feature(generic_const_exprs)]
 * #![feature(const_generics_defaults)]
 * #![feature(type_ascription)]
 */
struct SpaceVec<const N: usize> {
    components: [i32; N],
}
impl SpaceVec<{2: usize}> {
    pub fn new(x: i32, y: i32) -> Self {
        let array: [i32; 2] = [x, y];
        SpaceVec::<{2: usize}> {
            components: array,
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
struct Coord {
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
    //utils::set_panic_hook();
    //#[cfg(debug_assertions)]
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

    let _dispatch = dispatch::Dispatch::new(window, container, canvas, context);

    Ok(())
}
