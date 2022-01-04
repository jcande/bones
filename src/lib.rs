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
use crate::events::EventListenerOptions;
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

// These are defined in tiling.rs and we just copy them here to get this working
struct Tile {
    north: Pip,
    east: Pip,
    south: Pip,
    west: Pip,
}
type Pip = usize;

struct DapperTile {
    coord: (isize, isize),
    tile: Tile,
}
struct TileView<'a> {
    row_start: isize,
    row_end: isize,

    col_start: isize,
    col_end: isize,

    x: isize,
    y: isize,

    model: &'a Model,
}
impl<'a> Iterator for TileView<'a> {
    type Item = DapperTile;

    fn next(&mut self) -> Option<Self::Item> {
        let coord = (self.x, self.y);
        let tile = self.model
            .get_tile(coord.0, coord.1)
            .expect("We should have computed all tiles in the given view before handing out an iterator to them");

        // XXX TODO BUG This is buggy. The lower right square is not rendered
        self.x = self.x + 1;
        if self.x > self.row_end {
            self.x = self.row_start;

            self.y = self.y + 1;
            if self.y > self.col_end {
                return None;
            }
        }

        Some(DapperTile {
            coord: coord,
            tile: tile,
        })
    }
}

struct Model {
    pub data: usize,
}
impl<'a> Model {
    pub fn new() -> Self {
        Self {
            data: 0,
        }
    }

    // this should fail if we don't have the tile computed
    pub fn get_tile(&self, row: isize, col: isize) -> Option<Tile> {
        // bullshit data that will always be valid
        Some(Tile {
            north: (col % 2) as usize,
            east: (2 + (row % 2)) as usize,
            south: ((col + 1) % 2) as usize,
            west: (2 + (row + 1) % 2) as usize,
        })
    }

    pub fn compute(&mut self, row_start: isize, row_end: isize, col_start: isize, col_end: isize) -> Option<()> {
        // calculate new tiles, if necessary
        Some(())
    }

    pub fn tile_range(&'a self, row_start: isize, row_end: isize, col_start: isize, col_end: isize) -> TileView<'a> {
        // assert that compute() was called before. We seemingly have to split this up due to
        // mutable borrows being required to store the computation not mixing well with immutable
        // borrows into the tiles :(

        TileView {
            row_start: row_start,
            row_end: row_end,

            col_start: col_start,
            col_end: col_end,

            x: row_start,
            y: col_start,

            model: self,
        }
    }
}

#[derive(Debug)]
enum PointerState {
    Down,
    Up,
    Move,
}
struct Renderer {
    model: Model,
    zoom: f64,

    dispatch: Option<Rc<Dispatch>>,   // take an immutable pointer to the dispatcher to keep it alive

    canvas: web_sys::HtmlCanvasElement,
    canvas_ctx: web_sys::CanvasRenderingContext2d,
}
impl Renderer {
    const TILE_WIDTH: f64 = 100.0;
    const TILE_HEIGHT: f64 = 100.0;

    // XXX should we really pass this in like this?
    pub fn new(canvas: web_sys::HtmlCanvasElement, context: web_sys::CanvasRenderingContext2d) -> Self {
        context.set_image_smoothing_enabled(false);
        Self {
            model: Model::new(),
            zoom: 1.0,

            dispatch: None,

            canvas: canvas,
            canvas_ctx: context,
        }
    }
    pub fn initialize(&mut self, dispatch: Rc<Dispatch>) {
        self.dispatch = Some(dispatch);
        self.render();
    }

    fn draw_triangle(&self, row: isize, col: isize, cardinal: Direction, color: u32) {
        let tile_width: f64 = Renderer::TILE_WIDTH * self.zoom;
        let tile_height: f64 = Renderer::TILE_HEIGHT * self.zoom;

        // TODO take offset into account

        let x = (row as f64) * tile_width;
        let y = (col as f64) * tile_height;

        self.canvas_ctx.save();
        {
            self.canvas_ctx.translate(x, y)
                .expect("oh god how can this fail?");
            self.canvas_ctx.begin_path();
            match cardinal {
                Direction::North => {
                    self.canvas_ctx.move_to(0.0, 0.0);
                    self.canvas_ctx.line_to(tile_width, 0.0);
                    self.canvas_ctx.line_to(tile_width / 2.0, tile_height / 2.0);
                    self.canvas_ctx.line_to(0.0, 0.0);
                },
                Direction::East => {
                    self.canvas_ctx.move_to(tile_width, 0.0);
                    self.canvas_ctx.line_to(tile_width, tile_height);
                    self.canvas_ctx.line_to(tile_width / 2.0, tile_height / 2.0);
                    self.canvas_ctx.line_to(tile_width, 0.0);
                },
                Direction::South => {
                    self.canvas_ctx.move_to(tile_width, tile_height);
                    self.canvas_ctx.line_to(0.0, tile_height);
                    self.canvas_ctx.line_to(tile_width / 2.0, tile_height / 2.0);
                    self.canvas_ctx.line_to(tile_width, tile_height);
                },
                Direction::West => {
                    self.canvas_ctx.move_to(0.0, tile_height);
                    self.canvas_ctx.line_to(0.0, 0.0);
                    self.canvas_ctx.line_to(tile_width / 2.0, tile_height / 2.0);
                    self.canvas_ctx.line_to(0.0, tile_height);
                },
            };
            self.canvas_ctx.close_path();

            // This is dumb. Can we really not give it a more direct value?
            let s = format!("#{:0>6x}", color);
            let color = JsValue::from_str(&s);
            self.canvas_ctx.set_fill_style(&color);

            self.canvas_ctx.fill();
        }
        self.canvas_ctx.restore();
    }

    fn draw_square(&self, row: isize, col: isize) {
        let tile_width: f64 = Renderer::TILE_WIDTH * self.zoom;
        let tile_height: f64 = Renderer::TILE_HEIGHT * self.zoom;

        let x = (row as f64) * tile_width;
        let y = (col as f64) * tile_height;

        self.canvas_ctx.save();
        self.canvas_ctx.translate(x, y)
            .expect("oh god how can this fail?");
        {
            self.canvas_ctx.begin_path();
            self.canvas_ctx.rect(0.0, 0.0, tile_width, tile_height);
            self.canvas_ctx.close_path();

            self.canvas_ctx.set_line_width(1.0);
            self.canvas_ctx.stroke();
        }
        self.canvas_ctx.restore();
    }


    fn render(&mut self) {
        self.canvas_ctx.clear_rect(0.0,
                                   0.0,
                                   self.canvas.width().into(),
                                   self.canvas.height().into());

        const ORANGE: u32 = 0xffa500;
        const GREEN: u32 = 0x008000;
        const BLUE: u32 = 0x0000ff;
        const RED: u32 = 0xff0000;
        let colors = [RED, BLUE, GREEN, ORANGE];

        // First, compute any matches necessary (bullshit range)
        // XXX TODO compute real range based on canvas dimensions and zoom + offset
        self.model.compute(0, 5, 0, 5);

        // Second, display the tiles
        for tile in self.model.tile_range(0, 5, 0, 5) { // bullshit range just to get some fake data
            self.draw_triangle(tile.coord.0, tile.coord.1, Direction::North, colors[tile.tile.north]);
            self.draw_triangle(tile.coord.0, tile.coord.1, Direction::East, colors[tile.tile.east]);
            self.draw_triangle(tile.coord.0, tile.coord.1, Direction::South, colors[tile.tile.south]);
            self.draw_triangle(tile.coord.0, tile.coord.1, Direction::West, colors[tile.tile.west]);

            if tile.coord.0 == 2 && tile.coord.1 == 3 {
                self.draw_square(tile.coord.0, tile.coord.1);
            }
        }
    }

    pub fn callback_handler(&mut self, data: usize) {
        log("callback_handler");
        self.model.data = data;
    }

    pub fn update_offset(&mut self, x: i32, y: i32, state: PointerState) {
        log(&format!("offset: {}, {}, {:?}", x, y, state));
    }

    pub fn zoom(&mut self, x: i32, y: i32, delta: f64) {
        self.zoom += -delta * 0.001;
        log(&format!("zoom: {}, {}, {} => {}", x, y, delta, self.zoom));
        self.render();
    }
}

struct Dispatch {
    _listeners: Vec<EventListener>,

    renderer: Rc<RefCell<Renderer>>,
}
impl Dispatch {
    pub fn new(canvas: web_sys::HtmlCanvasElement, context: web_sys::CanvasRenderingContext2d) -> Rc<Self> {
        // First construct the Dispatch object with uninitialized receivers (e.g., renderer).
        let renderer = Rc::new(RefCell::new(Renderer::new(canvas.clone(), context)));

        // Construct the various callbacks that we're interested in.
        let mut listeners = Vec::new();
        let target = web_sys::EventTarget::from(canvas.clone());

        let renderer_clone = Rc::clone(&renderer);
        listeners.push(EventListener::new_with_options(&target, "wheel", EventListenerOptions::enable_prevent_default(), move |event: &web_sys::Event| {
            let wheel = event.clone()
                .dyn_into::<web_sys::WheelEvent>()
                .expect("The event passed to wheel callback doesn't match");
            // Prevent the scrollbar from being touched.
            wheel.prevent_default();

            renderer_clone.try_borrow_mut()
                .expect("Unable to borrow renderer for wheel event")
                .zoom(wheel.client_x(), wheel.client_y(), wheel.delta_y());
        }));

        let renderer_clone = Rc::clone(&renderer);
        listeners.push(EventListener::new(&target, "pointerdown", move |event: &web_sys::Event| {
            let pointer = event.clone()
                .dyn_into::<web_sys::PointerEvent>()
                .expect("The event passed to pointerdown callback doesn't match");

            renderer_clone.try_borrow_mut()
                .expect("Unable to borrow renderer for pointerdown event")
                .update_offset(pointer.client_x(), pointer.client_y(), PointerState::Down);
        }));
        let renderer_clone = Rc::clone(&renderer);
        listeners.push(EventListener::new(&target, "pointerup", move |event: &web_sys::Event| {
            let pointer = event.clone()
                .dyn_into::<web_sys::PointerEvent>()
                .expect("The event passed to pointerup callback doesn't match");

            renderer_clone.try_borrow_mut()
                .expect("Unable to borrow renderer for pointerup event")
                .update_offset(pointer.client_x(), pointer.client_y(), PointerState::Up);
        }));
        let renderer_clone = Rc::clone(&renderer);
        listeners.push(EventListener::new(&target, "pointerout", move |event: &web_sys::Event| {
            let pointer = event.clone()
                .dyn_into::<web_sys::PointerEvent>()
                .expect("The event passed to pointerout callback doesn't match");

            // We treat pointerout the same as if the user released it
            renderer_clone.try_borrow_mut()
                .expect("Unable to borrow renderer for pointerout event")
                .update_offset(pointer.client_x(), pointer.client_y(), PointerState::Up);
        }));
/*
        let renderer_clone = Rc::clone(&renderer);
        listeners.push(EventListener::new(&target, "pointermove", move |event: &web_sys::Event| {
            let pointer = event.clone()
                .dyn_into::<web_sys::PointerEvent>()
                .expect("The event passed to pointermove callback doesn't match");

            renderer_clone.try_borrow_mut()
                .expect("Unable to borrow renderer for pointermove event")
                .update_offset(pointer.client_x(), pointer.client_y(), PointerState::Move);
        }));
*/

        // XXX this doesn't work. I think the target needs to be the window or something.
        let renderer_clone = Rc::clone(&renderer);
        listeners.push(EventListener::new(&target, "resize", move |_event: &web_sys::Event| {
            log("resize closure");
            renderer_clone.try_borrow_mut()
                .expect("you better believe it")
                .callback_handler(100);
        }));

        let obj = Rc::new(Self {
            _listeners: listeners,

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

#[wasm_bindgen(start)]
pub fn js_main() -> Result<(), JsValue> {
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

    let _dispatch = Dispatch::new(canvas, context);

    Ok(())
}
