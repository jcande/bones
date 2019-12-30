use std::collections::HashMap;
use std::collections::HashSet;

use crate::tiling::Pip;
use crate::tiling::Tile;
use crate::tiling::Orientation;
use crate::tiling::Direction;

use crate::tiling::TileSet;
use crate::tiling::TileRef;

#[derive(Debug)]
pub struct Row<'a> {
    // XXX We need the western and eastern fronts that are permanent
    // the first and last slots are potentially something other than these permanent tiles but
    // in the general case will not be. There is some chance that the head could ran into them
    // and then we need to insert a blank tile to handle it.
    set: &'a TileSet,
    row: Vec<TileCloud<'a>>,
}

impl<'a> Row<'a> {
    pub fn new(set: &'a TileSet, border: &Tile, board: Vec<TileRef>) -> Self {
        let both_fronts = 2; // west + east
        let mut row: Vec<TileCloud> = Vec::with_capacity(board.len() + both_fronts);

        let row = row;
        Self {
            set: set,
            row: row,
        }
    }

    pub fn push(&mut self, cloud: TileCloud<'a>) {
        if self.row.is_empty() {
            // XXX push western cloud
        }
        self.row.push(cloud);
    }

    pub fn to_vec(self) -> Option<Vec<TileRef>> {
        // XXX push eastern cloud

        /*
                {
                    /*
                    let pred  = &    next[i-1];
                    let cloud = &mut next[i];
                    */
                    let (earlier, later) = next[i-1..i+1].split_at_mut(1);
                    let pred = &earlier[0];
                    let cloud = &mut later[0];
                    cloud.constrain(&self.from_ref, &Orientation::Backward, pred);
                }

                {
                    /*
                    let succ  = &    next[i+1];
                    let cloud = &mut next[i];
                    */
                    let (earlier, later) = next[i..i+2].split_at_mut(1);
                    let cloud = &mut earlier[0];
                    let succ = &later[0];
                    cloud.constrain(&self.from_ref, &Orientation::Forward,  succ);
                }
        */

        None
    }
}

#[derive(Debug)]
// XXX this probably needs a reference to TileSet otherwise a caller could confuse them and then
// we would crash
pub struct TileCloud<'a> {
    set: &'a TileSet,
    cloud: HashSet<TileRef>,
}

// XXX think of operations

impl<'a> TileCloud<'a> {
    pub fn new(tiles: &'a TileSet, initial: Vec<TileRef>) -> Self {
        // XXX may want to pre-calculate each pip as that is used a lot
        Self {
            set: tiles,
            cloud: initial.into_iter().collect(),
        }
    }

    pub fn print_tiles(&self, tiles: &TileSet) {
        for r in self.cloud.iter() {
            println!("{}: {:?}", r, tiles[*r]);
        }
    }

    pub fn positional_pips(&self, direction: &Direction) -> HashSet<Pip> {
        self.cloud.iter()
            .cloned()
            .map(|ref r| self.set[*r].cardinal(direction))
            .collect()
    }

   // pub fn constrain_pip(&mut self, tiles: &TileSet, direction: &Direction, values: &[Pip]) -> bool {
    pub fn constrain_pip(&mut self, tiles: &TileSet, direction: &Direction, value: &Pip) -> bool {
        // XXX maybe with_capacity(self.cloud.len())? We'll use more space but probably faster
        let mut keep = HashSet::new();
        for r in self.cloud.iter() {
            if tiles[*r].cardinal(direction) == *value {
                keep.insert(*r);
            }
        }
        self.cloud = keep;

        self.cloud.len() > 0
    }

    // TODO
    pub fn constrain(&mut self,
                     tiles: &TileSet,
                     orientation: &Orientation,
                     other: &TileCloud) -> bool {
        // XXX maybe with_capacity(self.cloud.len())? We'll use more space but probably faster
        let mut keep = HashSet::new();
        let (current, next) = match orientation {
            Orientation::East => (Direction::East, Direction::West),
            Orientation::West => (Direction::West, Direction::East),
            _ => panic!("north/south constraints don't make sense in this context"),
        };

        let available_pips = other.positional_pips(&next);
        for r in self.cloud.iter() {
            let pip = tiles[*r].cardinal(&current);
            if available_pips.contains(&pip) {
                keep.insert(*r);
            }
        }
        self.cloud = keep;
        println!("keeping: {:?}", self.cloud);

        self.cloud.len() > 0
    }
}
