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
    border: TileRef,
}

impl<'a> Row<'a> {
    pub fn new(set: &'a TileSet, border: &Tile, board: &Vec<TileRef>) -> Self {
        let both_fronts = 2; // west + east
        let mut row: Vec<TileCloud> = Vec::with_capacity(board.len() + both_fronts);
        let border_ref = *set.get(border).expect("border must be contained in tileset");

        // The main idea is that we may or may not use the border clouds. They are only added in
        // case the machine expands. That leaves the loop where we generate the successor cloud
        // based on the current row of tiles.

        // west
        {
            let cloud = set.matches_tile(border, Direction::East);
            let cloud = TileCloud::new(set, cloud, TileCloudConf::Prefer(border_ref));
            row.push(cloud);
        }

        for r in board.iter() {
            let cloud = set.matches_ref(r, Direction::South);
            let cloud = TileCloud::new(set, cloud, TileCloudConf::Avoid(border_ref));
            row.push(cloud);
        }

        // east
        {
            let cloud = set.matches_tile(border, Direction::West);
            let cloud = TileCloud::new(set, cloud, TileCloudConf::Prefer(border_ref));
            row.push(cloud);
        }

        Self {
            set: set,
            row: row,
            border: border_ref,
        }
    }

    pub fn to_vec(mut self) -> Option<Vec<TileRef>> {
        for i in 0..self.row.len() {
            println!("everything referring to cloud {}", i);

            if i > 0 {
                // westward
                /*
                let pred  = &    self.row[i-1];
                let cloud = &mut self.row[i];
                */
                let (earlier, later) = self.row[i-1..i+1].split_at_mut(1);
                let pred = &earlier[0];
                let cloud = &mut later[0];

                if !cloud.constrain(pred, &Orientation::West) {
println!("to_vec: {}, westward!", i);
                    return None;
                }
            }

            if i < (self.row.len() - 1) {
                // eastward
                /*
                let succ  = &    self.row[i+1];
                let cloud = &mut self.row[i];
                */
                let (earlier, later) = self.row[i..i+2].split_at_mut(1);
                let cloud = &mut earlier[0];
                let succ = &later[0];
                if !cloud.constrain(succ, &Orientation::East) {
println!("to_vec: {}, eastward! {:?}", i, succ);
                    return None;
                }
            }
        }

        let mut next: Vec<TileRef> = self.row.iter().map(|cloud| cloud.select()).collect();
        if next[0] == self.border {
            next.remove(0);
        }
        if next[next.len() - 1] == self.border {
            next.pop();
        }

        /*
        let next = self.row.iter().enumerate()
            .map(|(i, cloud)| (i, cloud.select()))
            // Remove the border pieces if they are the expected border pieces. This is to
            // prevent us adding 2 tiles per step.
            .filter(|(i, tile)| (*i == 0 || *i == (self.row.len() - 1)) &&
                    *tile != self.border)
            .map(|(_, tile)| tile)
            .collect();
        */

        println!("next: {:?}", next);

        Some(next)
    }
}

#[derive(Debug)]
pub enum TileCloudConf {
    Prefer(TileRef),
    Avoid(TileRef),
}

#[derive(Debug)]
// XXX this probably needs a reference to TileSet otherwise a caller could confuse them and then
// we would crash
pub struct TileCloud<'a> {
    tiles: &'a TileSet,
    cloud: HashSet<TileRef>,
    conf: TileCloudConf,
}

// XXX think of operations

impl<'a> TileCloud<'a> {
    pub fn new(tiles: &'a TileSet, initial: Vec<TileRef>, conf: TileCloudConf) -> Self {
        // XXX may want to pre-calculate each pip as that is used a lot
        Self {
            tiles: tiles,
            cloud: initial.into_iter().collect(),
            conf: conf,
        }
    }

    pub fn positional_pips(&self, direction: &Direction) -> HashSet<Pip> {
        self.cloud.iter()
            .cloned()
            .map(|ref r| self.tiles[*r].cardinal(direction))
            .collect()
    }

    // Orientation is where the other tilecloud is in relation to self
    pub fn constrain(&mut self,
                     other: &TileCloud,
                     orientation: &Orientation
                     ) -> bool {
        // XXX maybe with_capacity(self.cloud.len())? We'll use more space but probably faster
        let mut keep = HashSet::new();
        if *orientation == Orientation::North ||
            *orientation == Orientation::South {
            panic!("north/south constraints don't make sense in this context");
        }
        let (current, next) = (orientation, -*orientation);

        let available_pips = other.positional_pips(&next);
        for r in self.cloud.iter() {
            let pip = self.tiles[*r].cardinal(&current);
            if available_pips.contains(&pip) {
                keep.insert(*r);
            }
        }
        println!("was: {:?}", self.cloud);
        self.cloud = keep;
        println!("keeping: {:?}", self.cloud);

        self.cloud.len() > 0
    }

    pub fn select(&self) -> TileRef {
        for r in self.cloud.iter() {
            match self.conf {
                TileCloudConf::Prefer(tile_ref) => if tile_ref == *r {
                    println!("preference obtained");
                    return *r;
                },
                TileCloudConf::Avoid(tile_ref) => if tile_ref != *r {
                    println!("avoidance obtained");
                    return *r;
                },
            }
        }

        println!("gotta take what we can get");
        *self.cloud.iter().next().expect("TileCloud needs to have valid tiles before it can select one")
    }
}
