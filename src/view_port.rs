use crate::renderer;
use crate::Coord;

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
                        Some(Coord::new(lhs_xy.x - rhs_xy.x, lhs_xy.y - rhs_xy.y))
                    },
                    _ => None,
                }
            },
        }
    }
}

#[derive(Debug)]
pub enum PointerEvent {
    Down(Coord),
    Up(Coord),
    Out(Coord),
    Move(Coord),
}

pub struct ViewPort {
    pub zoom: f64,

    cursor: PointerState,

    width: i32,
    height: i32,

    offset: Coord,
}
impl ViewPort {
    pub fn new(width: u32, height: u32) -> Self {
        let starting_coord = if crate::SCREEN_SAVER_MODE {
            Coord::new(300, 0)
        } else {
            Coord::new(0, 0)
        };
        Self {
            zoom: 1.0,

            cursor: PointerState::Released,

            offset: starting_coord,

            // Yeah, we cast it. The interface gives us some stuff as i32 and others u32. It's
            // annoying. Maybe I'll add asserts later...
            width: width as i32,
            height: height as i32,
        }
    }

    pub fn offset(&self) -> Coord {
        Coord::new(self.offset.x, self.offset.y)
    }

    pub fn update_dimensions(&mut self, width: u32, height: u32) -> bool {
        let width = width as i32;
        let height = height as i32;

        self.width = width;
        self.height = height;

        true
    }

    pub fn update_scale(&mut self, xy: Coord, delta: f64) -> bool {
        // These are just magic numbers that seem to work ok. More thought should probably be put
        // into this.
        let new_zoom = delta.mul_add(-0.001, self.zoom).clamp(0.08, 4.0);

        let mut x = self.offset.x as f64;
        let mut y = self.offset.y as f64;
        x = x - (xy.x as f64 - x) * (new_zoom / self.zoom - 1.0);
        y = y - (xy.y as f64 - y) * (new_zoom / self.zoom - 1.0);

        self.offset.x = x as i32;
        self.offset.y = y as i32;
        self.zoom = new_zoom;

        true
    }

    // TODO make this a bool
    pub fn update_cursor(&mut self, event: PointerEvent) -> Result<(), ()> {
        // Convert the PointerEvent into PointerState
        let new_cursor = match &self.cursor {
            PointerState::Released => {
                match event {
                    PointerEvent::Down(xy) => PointerState::Leased(xy),
                    _ => PointerState::Released,
                }
            },
            PointerState::Leased(_) => {
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

    pub fn scope(&self) -> ((i32,i32), (i32,i32)) {
        //let row_start = renderer::Renderer::TILE_WIDTH * self.zoom + self.offset.x as f64;
        let width = self.width as f64;
        let height = self.height as f64;

        // width

        let tile_width = renderer::Renderer::TILE_WIDTH * self.zoom;
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

        let tile_height = renderer::Renderer::TILE_HEIGHT * self.zoom;
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
