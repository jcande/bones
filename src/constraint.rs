use std::collections::HashSet;

use crate::tiling::Pip;
use crate::tiling::Tile;
use crate::tiling::Orientation;
use crate::tiling::Direction;

use crate::tiling::TileSet;
use crate::tiling::TileRef;


#[derive(Debug)]
pub enum TileCloudConf {
    Prefer(TileRef),
    Avoid(TileRef),
    Whatever,
}

#[derive(Debug)]
pub struct TileCloud<'a> {
    tiles: &'a TileSet,
    cloud: HashSet<TileRef>,
    conf: TileCloudConf,
}

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
        assert!(*orientation != Orientation::North && *orientation != Orientation::South,
            "north/south constraints don't make sense in this context");

        // XXX maybe with_capacity(self.cloud.len())? We'll use more space but probably faster
        let mut keep = HashSet::new();
        let (current, next) = (orientation, -*orientation);

        let available_pips = other.positional_pips(&next);
        for r in self.cloud.iter() {
            let pip = self.tiles[*r].cardinal(&current);
            if available_pips.contains(&pip) {
                keep.insert(*r);
            }
        }
        self.cloud = keep;

        !self.cloud.is_empty()
    }

    pub fn select(&self) -> TileRef {
        // The thinking behind these preferences is that we can use the border tile as a
        // tie-breaker. If the cloud is along the border then we prefer to keep a border as we
        // can discard it later. If the tile is interior then we would rather not keep the
        // border as that is not likely the tile we want (assuming a lot here). In the end,
        // however, we take what we can get.
        for r in self.cloud.iter() {
            match self.conf {
                TileCloudConf::Prefer(tile_ref) => if tile_ref == *r {
                    return *r;
                },
                TileCloudConf::Avoid(tile_ref) => if tile_ref != *r {
                    return *r;
                },
                TileCloudConf::Whatever => (),
            }
        }

        *self.cloud.iter()
            .next()
            .expect("TileCloud needs to have valid tiles before it can select one")
    }
}


#[derive(Debug)]
// XXX This only handles the very narrow case where a SINGLE tile changes between rows. To clarify,
// if we have a single head that moves one square left or right, this case is covered. An
// optimization where we can move the head n-squares is NOT. I still need to think more about how
// to achieve this but for now it will not work.
// Maybe use a list instead of a vector while we're handling TileClouds? Gotta think about how to
// represent that "infinite" stuff though.
pub struct Row<'a> {
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

        let next = self.row.iter().enumerate()
            .map(|(i, cloud)| (i, cloud.select()))
            // Remove the border pieces if they are the expected border pieces. This is to
            // prevent us adding 2 tiles per step.
            .filter(|(i, tile)| {
                let in_border_position = *i == 0 || *i == (self.row.len() - 1);
                let is_border = *tile == self.border;
                let remove = !(in_border_position && is_border);
                remove
            })
            .map(|(_, tile)| tile)
            .collect();

        Some(next)
    }
}

#[cfg(test)]
mod constraint_tests {
    use super::*;

    fn default_mosaic() -> (TileRef, Vec<TileRef>, TileSet) {
        let border = Tile::new(0, 0, 0, 0);
        let starter_tile = Tile::new(0, 0, 10, 0);
        // This program basically turns 0s into 1s and shifts right.
        let set = vec![
            border,
            starter_tile,
            Tile::new(10, 7, 1, 0),
            Tile::new(1, 0, 1, 0),
            Tile::new(0, 0, 10, 7),
        ];

        let set = TileSet::new(set);

        let init = vec![starter_tile];
        let init = init.iter()
            .map(|tile| *set.get(tile).expect("tile should be present"))
            .collect();

        let border = *set.get(&border).expect("tile should be present");
        (border, init, set)
    }

    fn pips_by_position() {
        let init = Tile::new(0, 1, 2, 3);
        let tiles = vec![
            init,
            Tile::new(1, 4, 5, 6),
        ];
        let set = TileSet::new(tiles.clone());
        let initial = vec![init].iter()
            .map(|tile| *set.get(tile).expect("should be present"))
            .collect();
        //let (_, init, set) = default_mosaic();

        let cloud = TileCloud::new(&set, initial, TileCloudConf::Whatever);

        for direction in [Direction::North,
                Direction::East,
                Direction::South,
                Direction::West]
                .iter() {
            let found = cloud.positional_pips(&direction);
            for tile in tiles.iter() {
                assert!(found.contains(&tile.cardinal(&direction)));
            }
        }
    }
}
