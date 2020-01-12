use std::collections::HashMap;
use std::fmt;
use std::ops::Index;
use std::ops::Neg;

pub type Pip = usize;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    North,
    East,
    South,
    West,
}
pub type Orientation = Direction;

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Direction::North => "North",
                Direction::East => "East",
                Direction::South => "South",
                Direction::West => "West",
            }
        )
    }
}

impl Neg for Direction {
    type Output = Direction;

    fn neg(self) -> Self::Output {
        match self {
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::East => Direction::West,
            Direction::West => Direction::East,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Tile {
    north: Pip,
    east: Pip,
    south: Pip,
    west: Pip,
}

impl std::fmt::Display for Tile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(n: {}, e: {}, s: {}, w: {})",
            self.north, self.east, self.south, self.west
        )
    }
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
            Direction::East => self.east,
            Direction::South => self.south,
            Direction::West => self.west,
        }
    }
}

// If we have more than 4 billion then we'll have to bump it
pub type TileRef = u32;

#[derive(Debug)]
pub struct TileSet {
    set: Vec<Tile>,
    lookup: HashMap<Tile, TileRef>,
}

impl TileSet {
    pub fn new(set: Vec<Tile>) -> Self {
        let mut lookup = HashMap::new();
        for (i, tile) in set.iter().enumerate() {
            lookup.insert(*tile, i as TileRef);
        }

        TileSet { set, lookup }
    }

    pub fn get(&self, tile: &Tile) -> Option<&TileRef> {
        self.lookup.get(tile)
    }

    // The orientation is relative to the pip. In other words, orientation refers to where the
    // pip is located within a tile.
    pub fn matches_pip(&self, pip: &Pip, direction: Orientation) -> Vec<TileRef> {
        let next = -direction;

        self.lookup
            .iter()
            .filter(|(tile, _)| *pip == tile.cardinal(&next))
            .map(|(_, r)| *r)
            .collect()
    }

    pub fn matches_tile(&self, tile: &Tile, direction: Orientation) -> Vec<TileRef> {
        let pip = tile.cardinal(&direction);
        self.matches_pip(&pip, direction)
    }

    // The orientation is relative to the provided tile. E.g., if we say West, then we look at
    // the westernmost pip of the tile and find all eastern pips that match
    pub fn matches_ref(&self, tile_ref: &TileRef, direction: Orientation) -> Vec<TileRef> {
        let tile = self.set[*tile_ref as usize];
        self.matches_tile(&tile, direction)
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

#[cfg(test)]
mod tile_tests {
    use super::*;

    #[test]
    fn make_tile() {
        let (north, east, south, west) = (0, 1, 2, 3);
        let tile = Tile::new(north, east, south, west);

        assert!(tile.cardinal(&Direction::North) == north);
        assert!(tile.cardinal(&Direction::East) == east);
        assert!(tile.cardinal(&Direction::South) == south);
        assert!(tile.cardinal(&Direction::West) == west);
    }

    #[test]
    fn direction_negation() {
        let (n, e, s, w) = (
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        );
        assert!(n == -s);
        assert!(e == -w);
        assert!(s == -n);
        assert!(w == -e);
    }

    #[test]
    fn tile_to_ref() {
        let tile = Tile::new(0, 0, 0, 0);
        let set = TileSet::new(vec![tile]);

        let tile_ref = set.get(&tile).expect("tile should be present");
        assert!(tile == set[*tile_ref]);
    }

    #[test]
    fn matches() {
        let pip0 = 0;
        let pip1 = 1;
        let fancy = Tile::new(pip0, pip1, pip0, pip1);
        let zero = Tile::new(pip0, 100, 100, 100);
        let tiles = vec![fancy, zero];
        let set = TileSet::new(tiles.clone());
        let tile_refs: Vec<TileRef> = tiles
            .iter()
            .map(|tile| *set.get(tile).expect("tile should be present"))
            .collect();

        // Assume pip0 is the southernmost pip. This means it will attempt to match it with
        // northern pips.
        let matches = set.matches_pip(&pip0, Direction::South);
        assert!(matches.len() == 2);
        for r in tile_refs.iter() {
            assert!(matches.contains(r));
        }

        // Take the southernmost pip from fancy and find all the (northern) matches
        let matches = set.matches_tile(&fancy, Direction::South);
        assert!(matches.len() == 2);
        for r in tile_refs.iter() {
            assert!(matches.contains(r));
        }

        // Take the northernmost pip from fancy and find all the (southern) matches
        let matches = set.matches_tile(&fancy, Direction::North);
        assert!(matches.len() == 1);
        assert!(set[matches[0]] == fancy);

        // Take any reference and verify the east/west pips match up.
        let chosen_ref = tile_refs[0];
        let chosen_tile = set[chosen_ref];
        let matches = set.matches_ref(&chosen_ref, Direction::West);
        assert!(matches.len() == 1);
        assert!(set[matches[0]] == chosen_tile);
    }
}
