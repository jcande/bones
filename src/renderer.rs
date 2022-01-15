use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

use crate::view_port;
use crate::view_port::Model;//XXX temp
use crate::tiling;
use crate::dispatch;

// Cleanup
use crate::Coord;

// A macro to provide `println!(..)`-style syntax for `console.log` logging.
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

pub struct Renderer {
    model: Model,

    view: view_port::ViewPort,

    dispatch: Option<Rc<dispatch::Dispatch>>,   // take an immutable pointer to the dispatcher to keep it alive

    canvas: web_sys::HtmlCanvasElement,
    canvas_ctx: web_sys::CanvasRenderingContext2d,
}

impl Renderer {
    pub const TILE_WIDTH: f64 = 100.0;
    pub const TILE_HEIGHT: f64 = 100.0;

    // XXX should we really pass this in like this?
    pub fn new(canvas: web_sys::HtmlCanvasElement, context: web_sys::CanvasRenderingContext2d) -> Self {
        context.set_image_smoothing_enabled(false);
        Self {
            model: Model::new(),

            view: view_port::ViewPort::new(canvas.width(), canvas.height()),

            dispatch: None,

            canvas: canvas,
            canvas_ctx: context,
        }
    }
    pub fn initialize(&mut self, dispatch: Rc<dispatch::Dispatch>) {
        self.dispatch = Some(dispatch);
        self.render();
    }

    fn draw_triangle(&self, row: i32, col: i32, cardinal: tiling::Direction, color: u32) {
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
                tiling::Direction::North => {
                    self.canvas_ctx.move_to(0.0, 0.0);
                    self.canvas_ctx.line_to(tile_width, 0.0);
                    self.canvas_ctx.line_to(tile_width / 2.0, tile_height / 2.0);
                    self.canvas_ctx.line_to(0.0, 0.0);
                },
                tiling::Direction::East => {
                    self.canvas_ctx.move_to(tile_width, 0.0);
                    self.canvas_ctx.line_to(tile_width, tile_height);
                    self.canvas_ctx.line_to(tile_width / 2.0, tile_height / 2.0);
                    self.canvas_ctx.line_to(tile_width, 0.0);
                },
                tiling::Direction::South => {
                    self.canvas_ctx.move_to(tile_width, tile_height);
                    self.canvas_ctx.line_to(0.0, tile_height);
                    self.canvas_ctx.line_to(tile_width / 2.0, tile_height / 2.0);
                    self.canvas_ctx.line_to(tile_width, tile_height);
                },
                tiling::Direction::West => {
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

    fn render(&mut self) {
        self.canvas_ctx.clear_rect(0.0,
                                   0.0,
                                   self.canvas.width().into(),
                                   self.canvas.height().into());

        const TURQUOISE: u32 = 0x00c1ae;
        const PURPLE: u32 = 0x7320af;
        const ORANGE: u32 = 0xfa6211;
        const YELLOW: u32 = 0xfdee00;
        let colors = [TURQUOISE, ORANGE, PURPLE, YELLOW];

        let ((row_start, row_end), (col_start, col_end)) = self.view.scope();

        let range_handle = self.model.compute(row_start, row_end, col_start, col_end)
            .expect("why couldn't we compute? Out of memory?");

        // Second, display the tiles
        for tile_context in self.model.tile_range(range_handle) {
            let tile = tile_context.tile;
            self.draw_triangle(tile_context.coord.0, tile_context.coord.1, tiling::Direction::North, colors[tile.north]);
            self.draw_triangle(tile_context.coord.0, tile_context.coord.1, tiling::Direction::East, colors[tile.east]);
            self.draw_triangle(tile_context.coord.0, tile_context.coord.1, tiling::Direction::South, colors[tile.south]);
            self.draw_triangle(tile_context.coord.0, tile_context.coord.1, tiling::Direction::West, colors[tile.west]);
        }
    }

    pub fn update_pointer(&mut self, event: view_port::PointerEvent) {
        // TODO make this a bool
        if self.view.update_cursor(event).is_ok() {
            self.render();
        }
    }

    pub fn update_scale(&mut self, xy: Coord, delta: f64) {
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

    pub fn periodic(&mut self) {
        self.view.update_cursor(view_port::PointerEvent::Down(Coord::new(0, 0)));
        self.view.update_cursor(view_port::PointerEvent::Move(Coord::new(-1, -1)));
        self.view.update_cursor(view_port::PointerEvent::Up(Coord::new(10000, 10000)));
        self.render();
    }
}
