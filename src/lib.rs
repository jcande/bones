/*
 * Events we need to handle:
 *  - wheel : This is for zoom-in/zoom-out
 *      https://developer.mozilla.org/en-US/docs/Web/API/Document/wheel_event
 *  - drag : This is for scrolling the view
 *      https://developer.mozilla.org/en-US/docs/Web/API/HTML_Drag_and_Drop_API
 *  - resize : This is for the window chagning size
 *      https://developer.mozilla.org/en-US/docs/Web/API/VisualViewport/resize_event
 */
// TODO think of a more uniform/consistent/obvious way to deal with zooming. We also want to avoid
// small floats as that is inaccurate
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use gloo::events;
use crate::events::EventListener;
use crate::events::EventListenerOptions;
use std::rc::Rc;
use std::cell::RefCell;
use std::ops::AddAssign;
use std::ops::SubAssign;
extern crate console_error_panic_hook;

// A macro to provide `println!(..)`-style syntax for `console.log` logging.
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}
#[wasm_bindgen]
extern {
    pub fn alert(s: &str);
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
    coord: (i32, i32),
    tile: Tile,
}
struct TileView<'a> {
    row_start: i32,
    row_end: i32,

    col_start: i32,
    col_end: i32,

    x: i32,
    y: i32,

    model: &'a Model,
}
impl<'a> Iterator for TileView<'a> {
    type Item = DapperTile;

    fn next(&mut self) -> Option<Self::Item> {
        let coord = (self.x, self.y);

        // Check to see if we're outside the bounds. If that's the case, there are no more tiles
        // remaining in the iterator.
        if self.y > self.col_end {
            return None;
        }

        // Calculate the next tile's coordinate, ensuring we wrap to the next row if we are at the
        // end. We'll check on the next iteration if the computed coordinate is valid.
        self.x = self.x + 1;
        if self.x > self.row_end {
            self.x = self.row_start;

            self.y = self.y + 1;
        }

        let tile = self.model
            .get_tile(coord.0, coord.1)
            .expect("We should have computed all tiles in the given view before handing out an iterator to them");

        Some(DapperTile {
            coord: coord,
            tile: tile,
        })
    }
}

// This is a private/opaque type that serves to ensure the caller must go through our interface.
struct ComputeCertificate {
    row_start: i32,
    row_end: i32,

    col_start: i32,
    col_end: i32,
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
    pub fn get_tile(&self, row: i32, col: i32) -> Option<Tile> {
        // we don't want negative numbers with modulo
        let row = (row as u32) % 2;
        let col = (col as u32) % 2;

        // bullshit data that will always be valid
        let tile = Tile {
            north: (col % 2) as usize,
            east: (2 + (row % 2)) as usize,
            south: ((col + 1) % 2) as usize,
            west: (2 + (row + 1) % 2) as usize,
        };
        //log!("pips (nesw): {}, {}, {}, {}", tile.north, tile.east, tile.south, tile.west);
        Some(tile)
    }

    // XXX TODO this should return an opaque type that is all that tile_range() accepts
    pub fn compute(&mut self, row_start: i32, row_end: i32, col_start: i32, col_end: i32) -> Option<ComputeCertificate> {
        // calculate new tiles, if necessary

        Some(ComputeCertificate {
            row_start: row_start,
            row_end: row_end,
            col_start: col_start,
            col_end: col_end,
        })
    }

    pub fn tile_range(&'a self, proof: ComputeCertificate) -> TileView<'a> {
        // assert that compute() was called before. We seemingly have to split this up due to
        // mutable borrows being required to store the computation not mixing well with immutable
        // borrows into the tiles :(

        TileView {
            row_start: proof.row_start,
            row_end: proof.row_end,

            col_start: proof.col_start,
            col_end: proof.col_end,

            x: proof.row_start,
            y: proof.col_start,

            model: self,
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
#[derive(PartialEq, Clone, Debug)]
enum PointerState {
    Released,
    Leased(Coord),
}
impl PointerState {
    fn delta(&self, rhs: &Self) -> Option<Coord> {
        match &self {
            PointerState::Released => None,
            PointerState::Leased(ref lhs_xy) => {
                match &rhs {
                    PointerState::Leased(ref rhs_xy) => {
                        //log!("offset: {:?} => {:?}", xy, new_xy);
                        Some(Coord::new(lhs_xy.x - rhs_xy.x, lhs_xy.y - rhs_xy.y))
                    },
                    _ => None,
                }
            },
        }
    }
}
#[derive(Debug)]
enum PointerEvent {
    Down(Coord),
    Up(Coord),
    Out(Coord),
    Move(Coord),
}

struct ViewPort {
    zoom: f64,

    cursor: PointerState,

    width: i32,
    height: i32,

    offset: Coord,
}
impl ViewPort {
    fn new(width: u32, height: u32) -> Self {
        Self {
            zoom: 1.0,

            cursor: PointerState::Released,

            offset: Coord::new(0, 0),

            // Yeah, we cast it. The interface gives us some stuff as i32 and others u32. It's
            // annoying. Maybe I'll add asserts later...
            width: width as i32,
            height: height as i32,
        }
    }

    fn offset(&self) -> Coord {
        Coord::new(self.offset.x, self.offset.y)
    }

    fn update_dimensions(&mut self, width: u32, height: u32) -> bool {
        let width = width as i32;
        let height = height as i32;

        let old_width = self.width;
        let old_height = self.height;

        self.width = width;
        self.height = height;

        // TODO this flickers a lot and doesn't render half the time
        //(old_width < width || old_height < height)
        true
    }

    fn update_scale(&mut self, xy: Coord, delta: f64) -> bool {
        //self.offset += xy;
        //log!("zoom: {}, {}, {} => {}", xy.x, xy.y, delta, self.zoom);

        let width = self.width as f64;
        let height = self.height as f64;

        let new_zoom = self.zoom - (delta * 0.001);


        // TODO figure out zoom-to-point logic



        /*
        let x_delta = (width / self.zoom) - (width / new_zoom);
        let x_ratio = ((xy.x as f64) - (width / 2.0)) / width;
        let y_delta = (height / self.zoom) - (height / new_zoom);
        let y_ratio = ((xy.y as f64) - (height / 2.0)) / height;
        self.offset += Coord::new((x_delta * x_ratio) as i32, (y_delta * y_ratio) as i32);
        */




        /*
        let x_pos = (xy.x as f64) * self.zoom - (self.offset.x as f64);
        let y_pos = (xy.y as f64) * self.zoom - (self.offset.y as f64);

        let new_x = (xy.x as f64) * new_zoom - x_pos;
        let new_y = (xy.y as f64) * new_zoom - y_pos;

        self.offset.x = new_x as i32;
        self.offset.y = new_y as i32;
        */

        self.zoom = new_zoom;

        true
    }

    fn update_cursor(&mut self, event: PointerEvent) -> Result<(), ()> {
        // Convert the PointerEvent into PointerState
        let new_cursor = match &self.cursor {
            PointerState::Released => {
                match event {
                    PointerEvent::Down(xy) => PointerState::Leased(xy),
                    _ => PointerState::Released,
                }
            },
            PointerState::Leased(old_xy) => {
                match event {
                    PointerEvent::Down(xy) | PointerEvent::Move(xy) => PointerState::Leased(xy),
                    _ => PointerState::Released,
                }
            },
        };

        // Calculate the new offset based on the updated PointerState
        let delta = new_cursor.delta(&self.cursor);

        self.cursor = new_cursor;
        self.offset += delta.ok_or(())?;

        Ok(())
    }

    fn scope(&self) -> ((i32,i32), (i32,i32)) {
        //let row_start = Renderer::TILE_WIDTH * self.zoom + self.offset.x as f64;
        let width = self.width as f64;
        let height = self.height as f64;

        // width

        let tile_width = Renderer::TILE_WIDTH * self.zoom;
        // XXX this isn't true. self.width ITSELF might not be a multiple of tile width
        let split_visible = if self.offset.x % self.width != 0 {
            1
        } else {
            0
        };
        let view_width_capacity = (width / tile_width).ceil() as i32 + split_visible;

        let row_start = ((-self.offset.x as f64) / tile_width).floor() as i32;
        //log!("scope: width capacity: {}, row: [{}, {}], offset: {}", view_width_capacity, row_start, row_start + view_width_capacity, self.offset.x);


        // height

        let tile_height = Renderer::TILE_HEIGHT * self.zoom;
        // XXX this isn't true. self.width ITSELF might not be a multiple of tile width
        let split_visible = if self.offset.y % self.height != 0 {
            1
        } else {
            0
        };
        let view_height_capacity = (height / tile_height).ceil() as i32 + split_visible;

        let col_start = ((-self.offset.y as f64) / tile_height).floor() as i32;
        //log!("scope: height capacity: {}, col: [{}, {}], offset: {}", view_height_capacity, col_start, col_start + view_height_capacity, self.offset.y);

        // calculate how many tiles can fit in our width and height
        //  maybe: x_count = ceil((width - (offset % tile_width)) / tile_width) + same thing but
        //  with % instead of /
        // Then obtain starting row/col by doing offset / x_count to +x_count

        ((row_start, row_start + view_width_capacity), (col_start, col_start + view_height_capacity))
    }
}

struct Renderer {
    model: Model,

    view: ViewPort,

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

            view: ViewPort::new(canvas.width(), canvas.height()),

            dispatch: None,

            canvas: canvas,
            canvas_ctx: context,
        }
    }
    pub fn initialize(&mut self, dispatch: Rc<Dispatch>) {
        self.dispatch = Some(dispatch);
        self.render();
    }

    fn draw_triangle(&self, row: i32, col: i32, cardinal: Direction, color: u32) {
        let tile_width: f64 = Renderer::TILE_WIDTH * self.view.zoom;
        let tile_height: f64 = Renderer::TILE_HEIGHT * self.view.zoom;

        let offset = self.view.offset();
        let mut x = (row as f64) * tile_width;
        x += offset.x as f64;
        let mut y = (col as f64) * tile_height;
        y += offset.y as f64;

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

    fn draw_square(&self, row: i32, col: i32) {
        let tile_width: f64 = Renderer::TILE_WIDTH * self.view.zoom;
        let tile_height: f64 = Renderer::TILE_HEIGHT * self.view.zoom;

        let offset = self.view.offset();
        let mut x = (row as f64) * tile_width;
        x += offset.x as f64;
        let mut y = (col as f64) * tile_height;
        y += offset.y as f64;

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
        const PURPLE: u32 = 0x800080;
        const PINK: u32 = 0xee82ee;
        let colors = [RED, BLUE, GREEN, ORANGE];

        let ((row_start, row_end), (col_start, col_end)) = self.view.scope();

        let range_handle = self.model.compute(row_start, row_end, col_start, col_end)
            .expect("why couldn't we compute? Out of memory?");

        // Second, display the tiles
        for enriched_tile in self.model.tile_range(range_handle) { // bullshit range just to get some fake data
            self.draw_triangle(enriched_tile.coord.0, enriched_tile.coord.1, Direction::North, colors[enriched_tile.tile.north]);
            self.draw_triangle(enriched_tile.coord.0, enriched_tile.coord.1, Direction::East, colors[enriched_tile.tile.east]);
            self.draw_triangle(enriched_tile.coord.0, enriched_tile.coord.1, Direction::South, colors[enriched_tile.tile.south]);
            self.draw_triangle(enriched_tile.coord.0, enriched_tile.coord.1, Direction::West, colors[enriched_tile.tile.west]);

            // if the tile is the tape head, draw a square around it
            if enriched_tile.coord.0 == 0 && enriched_tile.coord.1 == 0 {
                self.draw_square(enriched_tile.coord.0, enriched_tile.coord.1);
            }
        }
    }

    pub fn update_pointer(&mut self, event: PointerEvent) {
        if self.view.update_cursor(event).is_ok() {
            self.render();
        }
    }

    pub fn zoom(&mut self, xy: Coord, delta: f64) {
        if self.view.update_scale(xy, delta) {
            self.render();
        }
    }

    pub fn update_dimensions(&mut self, width: u32, height: u32) {
        self.canvas.set_width(width);
        self.canvas.set_height(height);
        if self.view.update_dimensions(width, height) {
            self.render();
        }
    }
}

struct Dispatch {
    _listeners: Vec<EventListener>,

    renderer: Rc<RefCell<Renderer>>,
}
impl Dispatch {
    pub fn new(window: web_sys::Window, container: web_sys::HtmlElement, canvas: web_sys::HtmlCanvasElement, context: web_sys::CanvasRenderingContext2d) -> Rc<Self> {
        // First construct the Dispatch object with uninitialized receivers (e.g., renderer).
        let renderer = Rc::new(RefCell::new(Renderer::new(canvas.clone(), context)));

        // Construct the various callbacks that we're interested in.
        let mut listeners = Vec::new();
        let canvas_target = web_sys::EventTarget::from(canvas);
        let window_target = web_sys::EventTarget::from(window);

        let renderer_clone = Rc::clone(&renderer);
        listeners.push(EventListener::new_with_options(&canvas_target,
                                                       "wheel",
                                                       EventListenerOptions::enable_prevent_default(),
                                                       move |event: &web_sys::Event| {
            let wheel = event.clone()
                .dyn_into::<web_sys::WheelEvent>()
                .expect("The event passed to wheel callback doesn't match");
            // Prevent the scrollbar from being touched.
            wheel.prevent_default();

            renderer_clone.try_borrow_mut()
                .expect("Unable to borrow renderer for wheel event")
                .zoom(Coord::new(wheel.client_x(), wheel.client_y()), wheel.delta_y());
        }));

        let renderer_clone = Rc::clone(&renderer);
        listeners.push(EventListener::new(&canvas_target, "pointerdown", move |event: &web_sys::Event| {
            let pointer = event.clone()
                .dyn_into::<web_sys::PointerEvent>()
                .expect("The event passed to pointerdown callback doesn't match");

            renderer_clone.try_borrow_mut()
                .expect("Unable to borrow renderer for pointerdown event")
                .update_pointer(PointerEvent::Down(Coord::new(pointer.client_x(), pointer.client_y())));
        }));
        let renderer_clone = Rc::clone(&renderer);
        listeners.push(EventListener::new(&canvas_target, "pointerup", move |event: &web_sys::Event| {
            let pointer = event.clone()
                .dyn_into::<web_sys::PointerEvent>()
                .expect("The event passed to pointerup callback doesn't match");

            renderer_clone.try_borrow_mut()
                .expect("Unable to borrow renderer for pointerup event")
                .update_pointer(PointerEvent::Up(Coord::new(pointer.client_x(), pointer.client_y())));
        }));
        let renderer_clone = Rc::clone(&renderer);
        listeners.push(EventListener::new(&canvas_target, "pointerout", move |event: &web_sys::Event| {
            let pointer = event.clone()
                .dyn_into::<web_sys::PointerEvent>()
                .expect("The event passed to pointerout callback doesn't match");

            // We treat pointerout the same as if the user released it
            renderer_clone.try_borrow_mut()
                .expect("Unable to borrow renderer for pointerout event")
                .update_pointer(PointerEvent::Out(Coord::new(pointer.client_x(), pointer.client_y())));
        }));
        let renderer_clone = Rc::clone(&renderer);
        listeners.push(EventListener::new(&canvas_target, "pointermove", move |event: &web_sys::Event| {
            let pointer = event.clone()
                .dyn_into::<web_sys::PointerEvent>()
                .expect("The event passed to pointermove callback doesn't match");

            renderer_clone.try_borrow_mut()
                .expect("Unable to borrow renderer for pointermove event")
                .update_pointer(PointerEvent::Move(Coord::new(pointer.client_x(), pointer.client_y())));
        }));

        let renderer_clone = Rc::clone(&renderer);
        listeners.push(EventListener::new(&window_target, "resize", move |event: &web_sys::Event| {
            // I wanted to use `?` but couldn't change the closure interface. The inner-closure's
            // return is ignored.
            let _ = || -> Result<(), ()> {
                let width = container.offset_width()
                    .try_into()
                    .or(Err(()))?;
                let height = container.offset_height()
                    .try_into()
                    .or(Err(()))?;
                renderer_clone.try_borrow_mut()
                    .expect("Unable to borrow renderer for resize event")
                    .update_dimensions(width, height);
                Ok(())
            }();
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
        log!("calling drop on Dispatch");
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
    //utils::set_panic_hook();
    #[cfg(debug_assertions)]
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

    let _dispatch = Dispatch::new(window, container, canvas, context);

    Ok(())
}
