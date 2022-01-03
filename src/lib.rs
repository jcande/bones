/*
 * Events we need to handle:
 *  - wheel : This is for zoom-in/zoom-out
 *      https://developer.mozilla.org/en-US/docs/Web/API/Document/wheel_event
 *  - drag : This is for scrolling the view
 *      https://developer.mozilla.org/en-US/docs/Web/API/HTML_Drag_and_Drop_API
 *  - resize : This is for the window chagning size
 *      https://developer.mozilla.org/en-US/docs/Web/API/VisualViewport/resize_event
 */
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use gloo::events;
use crate::events::EventListener;
use std::rc::Rc;
use std::cell::RefCell;

#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}
#[wasm_bindgen(module = "/index.js")]
extern {
    fn hosted_in_js();
}

struct Model {
    pub data: usize,
}
impl Model {
    pub fn new() -> Self {
        Self {
            data: 0,
        }
    }
}

// This looks to be a premature optimization. If we want to go down this down it may be interesting
// but let's punt on it for now.
/*
trait TileSubview {
    fn tile_range(&mut self, row_start: usize, row_end: usize, col_start: usize, col_end: usize) -> Vec<()>;
}
*/
struct Renderer {
    model: Model,
    dispatch: Option<Rc<Dispatch>>,   // take an immutable pointer to the dispatcher to keep it alive
}
impl Renderer {
    pub fn new() -> Self {
        Self {
            model: Model::new(),
            dispatch: None,
        }
    }
    pub fn initialize(&mut self, dispatch: Rc<Dispatch>) {
        self.dispatch = Some(dispatch);
    }

    pub fn callback_handler(&mut self, data: usize) {
        log("callback_handler");
        self.model.data = data;
    }
}

struct Dispatch {
    resize_listener: EventListener,
    drag_listener: EventListener,
    wheel_listener: EventListener,

    renderer: Rc<RefCell<Renderer>>,
}
impl Dispatch {
    pub fn new(element: &web_sys::HtmlElement) -> Rc<Self> {
        // First construct the Dispatch object with uninitialized receivers (e.g., renderer).
        let renderer = Rc::new(RefCell::new(Renderer::new()));

        // Construct the various callbacks that we're interested in.
        let target = web_sys::EventTarget::from(element.clone());
        let renderer_clone = Rc::clone(&renderer);
        let wheel_listener = EventListener::new(&target, "wheel", move |event: &web_sys::Event| {
            //let event: web_sys::WheelEvent = *event.into();
            log(&format!("wheel closure: {:?}", event));
            renderer_clone.try_borrow_mut()
                .expect("you better believe it")
                .callback_handler(100);
        });
        let renderer_clone = Rc::clone(&renderer);
        let drag_listener = EventListener::new(&target, "drag", move |_event: &web_sys::Event| {
            log("drag closure");
            renderer_clone.try_borrow_mut()
                .expect("you better believe it")
                .callback_handler(100);
        });
        let renderer_clone = Rc::clone(&renderer);
        let resize_listener = EventListener::new(&target, "resize", move |_event: &web_sys::Event| {
            log("resize closure");
            renderer_clone.try_borrow_mut()
                .expect("you better believe it")
                .callback_handler(100);
        });

        let obj = Rc::new(Self {
            resize_listener: resize_listener,
            drag_listener: drag_listener,
            wheel_listener: wheel_listener,
            renderer: renderer,
        });

        // Now initialize the receivers.
        {
            let mut r = obj.renderer
                .borrow_mut();

            r.initialize(Rc::clone(&obj));
        }

        obj
    }
}
impl Drop for Dispatch {
    fn drop(&mut self) {
        log("calling drop on Dispatch");
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

fn draw_triangle(ctx: &web_sys::CanvasRenderingContext2d, row: usize, col: usize, cardinal: Direction, color: u32) {
    const TILE_WIDTH: f64 = 100.0;
    const TILE_HEIGHT: f64 = 100.0;
    let x = (row as f64) * TILE_WIDTH;
    let y = (col as f64) * TILE_HEIGHT;

    ctx.save();
    {
        ctx.translate(x, y)
            .expect("oh god how can this fail?");
        ctx.begin_path();
        match cardinal {
            Direction::North => {
                ctx.move_to(0.0, 0.0);
                ctx.line_to(TILE_WIDTH, 0.0);
                ctx.line_to(TILE_WIDTH / 2.0, TILE_HEIGHT / 2.0);
                ctx.line_to(0.0, 0.0);
            },
            Direction::East => {
                ctx.move_to(TILE_WIDTH, 0.0);
                ctx.line_to(TILE_WIDTH, TILE_HEIGHT);
                ctx.line_to(TILE_WIDTH / 2.0, TILE_HEIGHT / 2.0);
                ctx.line_to(TILE_WIDTH, 0.0);
            },
            Direction::South => {
                ctx.move_to(TILE_WIDTH, TILE_HEIGHT);
                ctx.line_to(0.0, TILE_HEIGHT);
                ctx.line_to(TILE_WIDTH / 2.0, TILE_HEIGHT / 2.0);
                ctx.line_to(TILE_WIDTH, TILE_HEIGHT);
            },
            Direction::West => {
                ctx.move_to(0.0, TILE_HEIGHT);
                ctx.line_to(0.0, 0.0);
                ctx.line_to(TILE_WIDTH / 2.0, TILE_HEIGHT / 2.0);
                ctx.line_to(0.0, TILE_HEIGHT);
            },
        };
        ctx.close_path();

        let s = format!("#{:0>6x}", color);
        let color = JsValue::from_str(&s);
        ctx.set_fill_style(&color);

// Use `web_sys`'s global `window` function to get a handle on the global
// window object.
//let window = web_sys::window().expect("no global `window` exists");
//let document = window.document().expect("should have a document on window");
//let body = document.body().expect("document should have a body");
//
//// Manufacture the element we're gonna append
//let val = document.create_element("p").expect("better create element");
//let debug_str = format!("debug: s: {}", s);
//val.set_inner_html(&debug_str);
//
//body.append_child(&val).expect("better append this effer");
        ctx.fill();
    }
    ctx.restore();
}

fn draw_square(ctx: &web_sys::CanvasRenderingContext2d, row: usize, col: usize) {
    const TILE_WIDTH: f64 = 100.0;
    const TILE_HEIGHT: f64 = 100.0;
    let x = (row as f64) * TILE_WIDTH;
    let y = (col as f64) * TILE_HEIGHT;

    ctx.save();
    ctx.translate(x, y)
        .expect("oh god how can this fail?");
    {
        ctx.begin_path();
        ctx.rect(0.0, 0.0, TILE_WIDTH, TILE_HEIGHT);
        ctx.close_path();

        ctx.set_line_width(1.0);
        ctx.stroke();
    }
    ctx.restore();
}

#[wasm_bindgen(start)]
pub fn js_main() -> Result<(), JsValue> {
    let document = web_sys::window()
        .ok_or(JsValue::from_str("no global window exists"))?
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

    {
        /*
        container.set_onresize();       // "resize"
        container.set_ondrag();         // "drag"
        container.set_onwheel();        // "wheel"
        */
        // the router/dispatcher would do this bit and setup callbacks directly into Renderer. Not
        // dynamic/modular but that's fine as we're not writing a framework.
        let _what = Dispatch::new(&container);
    }

    let context = canvas
        .get_context("2d")?
        .ok_or(JsValue::from_str("unable to retrieve 2d context from domino canvas"))?
        .dyn_into::<web_sys::CanvasRenderingContext2d>()?;

    context.set_image_smoothing_enabled(false);

    const ORANGE: u32 = 0xffa500;
    const GREEN: u32 = 0x008000;
    const BLUE: u32 = 0x0000ff;
    const RED: u32 = 0xff0000;
    let horizontal = [GREEN, ORANGE];
    let vertical = [RED, BLUE];
    for row in 0..25 {
        for col in 0..25 {
            draw_triangle(&context, row, col, Direction::North, vertical[(col + 0) % 2]);
            draw_triangle(&context, row, col, Direction::East, horizontal[(row + 0) % 2]);
            draw_triangle(&context, row, col, Direction::South, vertical[(col + 1) % 2]);
            draw_triangle(&context, row, col, Direction::West, horizontal[(row + 1) % 2]);
            draw_square(&context, row, col);
        }
    }

    Ok(())
}
