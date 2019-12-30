use std::collections::HashMap;
use std::collections::HashSet;

// or Bone!
pub type Pip = usize;

// XXX shouldn't use this as it is too vague
pub enum Axis {
    Latitude,
    Longitude,
}

pub enum Direction {
    North,
    East,
    South,
    West,
}
pub type Orientation = Direction;

// A constraint satisfaction problem (CSP) has these 3 components:
// 1) set of variables
// 2) Domain for each variable (i.e., set of values each variable can have)
// 3) set of constraints/relations between each variable
// In our case, each variable is a tile, the domain is the tileset, and the constraints are
// relations between each side of the tile
// Luckily this is pretty nice and let's us solve a "row" at a time. The previous row caps the
// tiles for the current row (so our domain is nicely contained that way). Then we just need to
// continually apply the constraints of each side (E and W) until we reach a fixpoint. If there is
// more than 1 possible tile for a slot then we'll just yolo-choose one as any should work by
// that point.

// (looks like this is called Constraint Propogation)
// We want arc-consistency (I think path-consistency might be overkill)
// We want local-search
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Tile {
    north: Pip,
    east:  Pip,
    south: Pip,
    west:  Pip,
}

impl Tile {
    pub fn new(north: Pip, east: Pip, south: Pip, west: Pip) -> Self {
        Self {
            north,
            east,
            south,
            west,
        }
    }

    pub fn cardinal(&self, direction: &Direction) -> Pip {
        match direction {
            Direction::North => self.north,
            Direction::East  => self.east,
            Direction::South => self.south,
            Direction::West  => self.west,
        }
    }
}


pub type TileRef = u32;
pub type TileSet = HashMap<TileRef, Tile>;
pub type RevTileSet = HashMap<Tile, TileRef>;
pub type BoardState = Vec<Tile>;
pub type BoardStateRef = Vec<TileRef>;
