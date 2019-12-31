use std::collections::HashMap;
use std::collections::HashSet;
use std::ops::Index;
use std::ops::Neg;

// or Bone!
pub type Pip = usize;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    North,
    East,
    South,
    West,
}
pub type Orientation = Direction;

impl Neg for Direction {
    type Output = Direction;

    fn neg(self) -> Self::Output {
        match self {
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::East  => Direction::West,
            Direction::West  => Direction::East,
        }
    }
}

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

    #[inline]
    pub fn cardinal(&self, direction: &Direction) -> Pip {
        match direction {
            Direction::North => self.north,
            Direction::East  => self.east,
            Direction::South => self.south,
            Direction::West  => self.west,
        }
    }
}


// If we have more than 4 billion then we'll have to bump it
pub type TileRef = u32;

#[derive(Debug)]
pub struct TileSet {
    set:    Vec<Tile>,
    lookup: HashMap<Tile, TileRef>,
}

impl TileSet {
    pub fn new(set: Vec<Tile>) -> Self {
        let mut lookup = HashMap::new();
        for (i, tile) in set.iter().enumerate() {
            lookup.insert(*tile, i as TileRef);
        }

        TileSet {
            set,
            lookup,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item=&Tile> {
        self.set.iter()
    }

    pub fn get(&self, tile: &Tile) -> Option<&TileRef> {
        self.lookup.get(tile)
    }

    // XXX make a type for the return value
    // The orientation is relative to the provided tile. E.g., if we say West, then we look at
    // the westernmost pip of the tile and find all eastern pips that match
    pub fn matches_ref(&self,
               tile_ref: &TileRef,
               direction: Orientation)
        -> Vec<TileRef> {
        let tile = self.set[*tile_ref as usize];
        self.matches_tile(&tile, direction)
    }

    pub fn matches_tile(&self,
                        tile: &Tile,
                        direction: Orientation)
        -> Vec<TileRef> {
        let (current, next) = (direction, -direction);

        let pip = tile.cardinal(&current);
        self.lookup.iter()
            .filter(|(tile, _)| pip == tile.cardinal(&next))
            .map(|(_, r)| *r)
            .collect()
    }

    // The orientation is relative to the pip. In other words, orientation refers to where the
    // pip is located within a tile.
    pub fn matches_pip(&self,
               pip: &Pip,
               direction: Orientation)
        -> Vec<TileRef> {
        let next = -direction;

        self.lookup.iter()
            .filter(|(tile, _)| *pip == tile.cardinal(&next))
            .map(|(_, r)| *r)
            .collect()
    }
}

// XXX how can I inline this to the main struct impl?
impl Index<TileRef> for TileSet {
    type Output = Tile;

    #[inline]
    fn index(&self, index: TileRef) -> &Self::Output {
        self.set.get(index as usize).expect("Out of bounds access")
    }
}
