use thiserror::Error;

use std::collections::HashSet;

use std::fmt;

use crate::tiling::Direction;
use crate::tiling::Orientation;
use crate::tiling::Pip;
use crate::tiling::Tile;

use crate::tiling::PurityBias;
use crate::tiling::SideEffects;

use crate::tiling::DominoPile;
use crate::tiling::TileRef;

#[derive(Error, Debug)]
pub enum TileCloudError {
    #[error("The cloud had unsatisfiable constraints. There are no possible tiles available.")]
    NoTilesLeft,
}

#[derive(Debug)]
#[cfg_attr(not(test), allow(dead_code))]
pub enum TileCloudConf {
    Prefer(TileRef),
    Avoid(TileRef),
    Whatever,
}

#[derive(Debug)]
pub struct TileCloud<'process> {
    tiles: &'process DominoPile,
    cloud: HashSet<TileRef>,
    conf: TileCloudConf,
}

impl<'process> std::fmt::Display for TileCloud<'process> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("TileCloud("))?;

        let tiles: Vec<Tile> = self
            .cloud
            .iter()
            .map(|tile_ref| self.tiles[*tile_ref])
            .collect();
        for (i, tile) in tiles.iter().enumerate() {
            f.write_fmt(format_args!("{}", tile))?;

            if i != tiles.len() - 1 {
                f.write_fmt(format_args!(", "))?;
            }
        }

        f.write_fmt(format_args!(")"))?;

        Ok(())
    }
}

impl<'process> TileCloud<'process> {
    pub fn new(tiles: &'process DominoPile, initial: Vec<TileRef>, conf: TileCloudConf) -> Self {
        // XXX may want to pre-calculate each pip as that is used a lot
        Self {
            tiles: tiles,
            cloud: initial
                .into_iter()
                .filter(|tile_ref| {
                    // Bar hidden tiles from consideration.
                    match tiles.get_side_effects(tile_ref) {
                        SideEffects::Pure(PurityBias::Hidden) => false,
                        _ => true,
                    }
                })
                .collect(),
            conf: conf,
        }
    }

    pub fn positional_pips(&self, direction: &Direction) -> HashSet<Pip> {
        self.cloud
            .iter()
            .cloned()
            .map(|ref r| self.tiles[*r].cardinal(direction))
            .collect()
    }

    // Orientation is where the other tilecloud is in relation to self
    pub fn constrain(
        &mut self,
        other: &TileCloud,
        orientation: &Orientation,
    ) -> Result<(), TileCloudError> {
        assert!(
            *orientation != Orientation::North && *orientation != Orientation::South,
            "north/south constraints don't make sense in this context"
        );

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

        if self.cloud.is_empty() {
            Err(TileCloudError::NoTilesLeft)
        } else {
            Ok(())
        }
    }

    pub fn select(&self) -> Result<TileRef, TileCloudError> {
        // The thinking behind these preferences is that we can use the border tile as a
        // tie-breaker. If the cloud is along the border then we prefer to keep a border as we
        // can discard it later. If the tile is interior then we would rather not keep the
        // border as that is not likely the tile we want (assuming a lot here). In the end,
        // however, we take what we can get.
        for r in self.cloud.iter() {
            match self.conf {
                TileCloudConf::Prefer(tile_ref) => {
                    if tile_ref == *r {
                        return Ok(*r);
                    }
                }
                TileCloudConf::Avoid(tile_ref) => {
                    if tile_ref != *r {
                        return Ok(*r);
                    }
                }
                TileCloudConf::Whatever => (),
            }
        }

        // Just grab the first available tile
        let tile = self
            .cloud
            .iter()
            .next()
            .map(|tile| *tile)
            .ok_or(TileCloudError::NoTilesLeft)?;
        Ok(tile)
    }
}

#[derive(Debug)]
// XXX This only handles the very narrow case where a SINGLE tile changes between rows. To clarify,
// if we have a single head that moves one square left or right, this case is covered. An
// optimization where we can move the head n-squares is NOT. I still need to think more about how
// to achieve this but for now it will not work.
// Maybe use a list instead of a vector while we're handling TileClouds? Gotta think about how to
// represent that "infinite" stuff though.
pub struct Row<'process> {
    pile: &'process DominoPile,
    row: Vec<TileCloud<'process>>,
    border: TileRef,
}

#[derive(Error, Debug)]
pub enum RowError {
    #[error("TileCloudError: {source}")]
    Cloud {
        #[from]
        source: TileCloudError,
    },

    #[error("Constraints proved impossible to satisfy: {context}.")]
    UnsatisfiableConstraints { context: String },
}

// XXX We need a more robust concept of fronts. We should keep adding border tiles on both the
// east and western "fronts" until we get a border back. This way we'll be able to tile
// configurations that expand by more than 1 tile per row. E.g., [west] [meat] [east] that can
// all grow independantly. Once this completes all 3 components become the next row.
impl<'process> Row<'process> {
    pub fn new(
        pile: &'process DominoPile,
        border: &TileRef,
        board: &Vec<TileRef>,
    ) -> Result<Self, RowError> {
        let both_fronts = 2; // west + east
        let mut row: Vec<TileCloud> = Vec::with_capacity(board.len() + both_fronts);

        // XXX We have no way of verifying whether or not border is a valid
        // reference. Is this ok?

        // The main idea is that we may or may not use the border clouds. They are only added in
        // case the machine expands. That leaves the loop where we generate the successor cloud
        // based on the current row of tiles.

        // XXX depending on how costly this is, we should pre-compute the western and eastern
        // clouds
        let latitude: HashSet<TileRef> =
            pile.matches(border, Direction::South).into_iter().collect();

        // west
        {
            let longitude: HashSet<TileRef> =
                pile.matches(border, Direction::East).into_iter().collect();
            let cloud: Vec<TileRef> = longitude.intersection(&latitude).cloned().collect();
            let cloud = TileCloud::new(pile, cloud, TileCloudConf::Prefer(*border));
            row.push(cloud);
        }

        for r in board.iter() {
            let cloud = pile.matches(r, Direction::South);
            let cloud = TileCloud::new(pile, cloud, TileCloudConf::Avoid(*border));
            row.push(cloud);
        }

        // east
        {
            let longitude: HashSet<TileRef> =
                pile.matches(border, Direction::West).into_iter().collect();
            let cloud: Vec<TileRef> = longitude.intersection(&latitude).cloned().collect();
            let cloud = TileCloud::new(pile, cloud, TileCloudConf::Prefer(*border));
            row.push(cloud);
        }

        Ok(Self {
            pile: pile,
            row: row,
            border: *border,
        })
    }

    pub fn to_vec(mut self) -> Result<Vec<TileRef>, RowError> {
        let first: usize = 0;
        let last: usize = self.row.len() - 1;
        for i in 0..self.row.len() {
            if i > first {
                // westward
                /*
                let pred  = &    self.row[i-1];
                let cloud = &mut self.row[i];
                */
                let (earlier, later) = self.row[i - 1..i + 1].split_at_mut(1);
                let pred = &earlier[0];
                let cloud = &mut later[0];

                cloud.constrain(pred, &Orientation::West).map_err(|_| {
                    RowError::UnsatisfiableConstraints {
                        context: format!("western: cloud {}: {}, other: {}", i, cloud, pred),
                    }
                })?;
            }

            if i < last {
                // eastward
                /*
                let succ  = &    self.row[i+1];
                let cloud = &mut self.row[i];
                */
                let (earlier, later) = self.row[i..i + 2].split_at_mut(1);
                let cloud = &mut earlier[0];
                let succ = &later[0];
                cloud.constrain(succ, &Orientation::East).map_err(|_| {
                    RowError::UnsatisfiableConstraints {
                        context: format!("eastern: cloud {}: {}, other: {}", i, cloud, succ),
                    }
                })?;
            }
        }

        // Check to see if we even have a valid set of tiles to work with.
        let mut next = Vec::new();
        for (i, cloud) in self.row.iter().enumerate() {
            let tile_ref = cloud.select()?;

            // Now that we have some valid tiles, let's see if we need to
            // remove the ends. Remove the border pieces if they are the
            // expected border pieces. This is to prevent us adding 2 tiles per
            // step.
            let in_border_position = i == 0 || i == (self.row.len() - 1);
            let is_border = tile_ref == self.border;
            let keep = !(in_border_position && is_border);

            if keep {
                next.push(tile_ref);
            }
        }
        Ok(next)
    }
}

#[cfg(test)]
mod constraint_tests {
    use super::*;
    use crate::tiling::Domino;

    #[test]
    fn pips_by_position() {
        let init = Tile::new(0, 1, 2, 3);
        let tiles = vec![init, Tile::new(1, 4, 5, 6)];
        let pile = DominoPile::new(tiles.clone().into_iter().map(Domino::pure).collect());
        let initial = tiles
            .iter()
            .clone()
            .map(|tile| *pile.get(tile).expect("should be present"))
            .collect();

        let cloud = TileCloud::new(&pile, initial, TileCloudConf::Whatever);

        // Iterate over all directions and verify that the corresponding pips are present for all
        // tiles.
        for direction in [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ]
        .iter()
        {
            let found = cloud.positional_pips(&direction);
            for tile in tiles.iter() {
                assert!(found.contains(&tile.cardinal(&direction)));
            }
        }
    }

    #[test]
    fn verify_simple_constraint() {
        let init = Tile::new(0, 0, 1, 0);
        let succ = Tile::new(1, 2, 3, 4);
        let neighbor = Tile::new(5, 6, 7, 2);
        let misc = Tile::new(10, 10, 10, 10);
        let tiles = vec![init, succ, neighbor, misc];
        let pile = DominoPile::new(tiles.clone().into_iter().map(Domino::pure).collect());

        // A TileCloud that can be anything it wants!
        let initial = tiles
            .iter()
            .clone()
            .map(|tile| *pile.get(tile).expect("should be present"))
            .collect();
        let mut everything = TileCloud::new(&pile, initial, TileCloudConf::Whatever);

        // A TileCloud with only one thing on its mind.
        let initial = vec![neighbor]
            .iter()
            .clone()
            .map(|tile| *pile.get(tile).expect("should be present"))
            .collect();
        let neighbor = TileCloud::new(&pile, initial, TileCloudConf::Whatever);

        everything
            .constrain(&neighbor, &Orientation::East)
            .expect("There should be one tile that satisfies the neighbor constraint.");

        let tile_ref = everything.select().expect("there should be a valid tile");
        assert_eq!(succ, pile[tile_ref]);
    }

    // TODO add a test for the select() edge cases

    #[test]
    fn successor_row() {
        let border = Tile::new(0, 0, 0, 0);
        let starter_tile = Tile::new(0, 0, 10, 0);
        let set_and_shift = Tile::new(10, 7, 1, 0);
        let stay_set = Tile::new(1, 0, 1, 0);
        let shift_and_repeat = Tile::new(0, 0, 10, 7);
        // This program basically turns 0s into 1s and shifts right.
        let pile = vec![
            border,
            starter_tile,
            set_and_shift,
            stay_set,
            shift_and_repeat,
        ];

        let pile = DominoPile::new(pile.clone().into_iter().map(Domino::pure).collect());

        let init = vec![starter_tile];
        let init = init
            .iter()
            .map(|tile| *pile.get(tile).expect("tile should be present"))
            .collect();

        let row = Row::new(
            &pile,
            pile.get(&border).expect("tile should be present"),
            &init,
        )
        .expect("valid row");
        let succ = row.to_vec().expect("valid successor row");

        let verified_succ: Vec<TileRef> = vec![set_and_shift, shift_and_repeat]
            .iter()
            .map(|tile| *pile.get(tile).expect("tile should be present"))
            .collect();
        assert_eq!(succ, verified_succ);
    }

    #[test]
    fn impossible_constraints() {
        let border = Tile::new(0, 0, 0, 0);
        let bad_starter_tile = Tile::new(0, 0, 0xbad, 0);
        let set_and_shift = Tile::new(10, 7, 1, 0);
        let stay_set = Tile::new(1, 0, 1, 0);
        let shift_and_repeat = Tile::new(0, 0, 10, 7);
        // This program basically turns 0s into 1s and shifts right.
        let pile = vec![
            border,
            bad_starter_tile,
            set_and_shift,
            stay_set,
            shift_and_repeat,
        ];

        let pile = DominoPile::new(pile.clone().into_iter().map(Domino::pure).collect());

        let init = vec![bad_starter_tile]
            .iter()
            .map(|tile| *pile.get(tile).expect("tile should be present"))
            .collect();

        let row = Row::new(
            &pile,
            pile.get(&border).expect("tile should be present"),
            &init,
        )
        .expect("valid row");

        match row.to_vec() {
            Err(RowError::UnsatisfiableConstraints { context: _ }) => (),
            x => panic!("Managed to satisy impossible constraints: {:?}", x),
        };
    }
}
