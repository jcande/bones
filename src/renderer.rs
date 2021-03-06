use std::rc::Rc;
use wasm_bindgen::JsValue;
use std::hash::Hash;
use std::hash::Hasher;
use std::collections::HashMap;

use url::Url;

use crate::view_port;
use crate::mosaic;
use crate::tiling;
use crate::dispatch;

// Cleanup
use crate::Coord;


struct UserParameters {
    show_border_tiles: bool,
    show_tile_boundaries: bool,

    color_add: u32,
    color_mul: u32,
}

impl UserParameters {
    fn default() -> Self {
        let (add, mul) = if crate::RULE110_MODE {
            (28, 22)
        } else {
            // This is a good scheme for tiles from compiled program
            (3, 1)
        };

        let border_disposition = if crate::RULE110_MODE {
            true
        } else {
            false
        };

        UserParameters {
            show_border_tiles: border_disposition,
            show_tile_boundaries: false,

            color_add: add,
            color_mul: mul,
        }
    }
}

pub struct Renderer {
    model: mosaic::Mosaic,

    view: view_port::ViewPort,

    dispatch: Option<Rc<dispatch::Dispatch>>,   // take an immutable pointer to the dispatcher to keep it alive

    canvas: web_sys::HtmlCanvasElement,
    canvas_ctx: web_sys::CanvasRenderingContext2d,

    options: UserParameters,
}

impl Renderer {
    pub const TILE_WIDTH: f64 = 100.0;
    pub const TILE_HEIGHT: f64 = 100.0;

    pub fn new(url: &url::Url, mosaic: mosaic::Mosaic, canvas: web_sys::HtmlCanvasElement, context: web_sys::CanvasRenderingContext2d) -> Self {
        context.set_image_smoothing_enabled(false);

        // Forgive me for I have sinned
        let hash_query: HashMap<_, _> = url.query_pairs().into_owned().collect();
        let mut options = UserParameters::default();
        for (k, v) in &hash_query {
            if k == "palette_add" {
                if let Ok(value) = v.parse::<u32>() {
                    options.color_add = value;
                }
            } else if k == "palette_mul" {
                if let Ok(value) = v.parse::<u32>() {
                    options.color_mul = value;
                }
            }
        }

        Self {
            model: mosaic,

            view: view_port::ViewPort::new(canvas.width(), canvas.height()),

            dispatch: None,

            canvas: canvas,
            canvas_ctx: context,

            options: options,
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

            if self.options.show_tile_boundaries {
                self.canvas_ctx.set_stroke_style(&JsValue::from_str("#000000"));
                self.canvas_ctx.set_line_width(0.5 * self.view.zoom);
                self.canvas_ctx.stroke();
            }
        }
        self.canvas_ctx.restore();
    }

    fn render(&mut self) {
        self.canvas_ctx.clear_rect(0.0,
                                   0.0,
                                   self.canvas.width().into(),
                                   self.canvas.height().into());

        /*
        const TURQUOISE: u32 = 0x00c1ae;
        const PURPLE: u32 = 0x7320af;
        const ORANGE: u32 = 0xfa6211;
        const YELLOW: u32 = 0xfdee00;
        let colors = [TURQUOISE, ORANGE, PURPLE, YELLOW];
        */

        let ((row_start, row_end), (col_start, col_end)) = self.view.scope();

        let range_handle = self.model.compute(row_start, row_end, col_start, col_end)
            .expect("Unable to compute view");

        // Second, display the tiles
        let query_option = if self.options.show_border_tiles {
            crate::mosaic::TileRetrieval::IncludeBorder
        } else {
            crate::mosaic::TileRetrieval::OnlyComputed
        };
        for tile_context in self.model.tile_range(range_handle, query_option) {
            let tile = tile_context.tile;
            let [n, e, s, w] = [tile.north, tile.east, tile.south, tile.west]
                .map(|d| -> u32 {
                    let d = d as u32;
                    let mut s = std::collections::hash_map::DefaultHasher::new();
                    // interesting: (0, 1), (3, 1)
                    d.wrapping_add(self.options.color_add).wrapping_mul(self.options.color_mul).hash(&mut s);
                    let wide = s.finish();
                    let upper = ((wide >> 32) & 0xffffffff) as u32;
                    let lower = ((wide >> 0) & 0xffffffff) as u32;
                    upper ^ lower
                });
            self.draw_triangle(tile_context.coord.0, tile_context.coord.1, tiling::Direction::North, n);
            self.draw_triangle(tile_context.coord.0, tile_context.coord.1, tiling::Direction::East, e);
            self.draw_triangle(tile_context.coord.0, tile_context.coord.1, tiling::Direction::South, s);
            self.draw_triangle(tile_context.coord.0, tile_context.coord.1, tiling::Direction::West, w);
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

    pub fn update_border(&mut self, border: bool) {
        let different = self.options.show_border_tiles != border;
        self.options.show_border_tiles = border;
        if different {
            self.render();
        }
    }

    pub fn update_tile_boundary(&mut self, boundary: bool) {
        let different = self.options.show_tile_boundaries != boundary;
        self.options.show_tile_boundaries = boundary;
        if different {
            self.render();
        }
    }

    pub fn update_color_add(&mut self, value: u32) {
        let different = self.options.color_add != value;
        self.options.color_add = value;
        if different {
            self.render();
        }
    }

    pub fn update_color_mul(&mut self, value: u32) {
        let different = self.options.color_mul != value;
        if different && value != 0 {
            self.options.color_mul = value;
            self.render();
        }
    }

    pub fn periodic(&mut self) {
        _ = self.view.update_cursor(view_port::PointerEvent::Down(Coord::new(0, 0)));
        _ = self.view.update_cursor(view_port::PointerEvent::Move(Coord::new(-1, -3)));
        let value_never_used_and_does_not_matter = 9999;
        _ = self.view.update_cursor(view_port::PointerEvent::Up(Coord::new(value_never_used_and_does_not_matter, value_never_used_and_does_not_matter)));
        self.render();
    }
}
