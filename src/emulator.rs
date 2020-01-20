use anyhow::Result;

use thiserror::Error;

use std::collections::HashSet;

use crate::tiling::Tile;

use crate::tiling::TileRef;
use crate::tiling::TileSet;

use crate::constraint::Row;

pub type BoardState = Vec<Tile>;
pub type BoardStateRef = Vec<TileRef>;

#[derive(Debug)]
pub struct Mosaic {
    set: TileSet,
    border: Tile,

    state: BoardStateRef,
}

#[derive(Error, Debug)]
pub enum MosaicError {
    #[error("Invalid tile: {tile}. The tile is not contained in the given tile set")]
    InvalidTile { tile: Tile },

    #[error("Invalid border tile: {tile}. The tile is not contained in the given tile set")]
    InvalidTileBorder { tile: Tile },
}

impl Mosaic {
    pub fn new(set: HashSet<Tile>, border_tile: Tile, initial: BoardState) -> Result<Self> {
        if !set.contains(&border_tile) {
            Err(MosaicError::InvalidTileBorder { tile: border_tile })?;
        }
        // Ensure all tiles in initial are contained in the tile-set.
        for tile in initial.iter() {
            if !set.contains(tile) {
                Err(MosaicError::InvalidTile { tile: *tile })?;
            }
        }

        let tiles = TileSet::new(set.into_iter().collect());

        // Convert Tiles into TileRefs.
        let state = initial
            .into_iter()
            .map(|tile| {
                *tiles
                    .get(&tile)
                    .expect("we should have already verified this")
            })
            .collect();

        // XXX precalculate west/east PossibleTiles

        Ok(Self {
            set: tiles,
            border: border_tile,
            state: state,
        })
    }

    // evolve current state to next state
    pub fn step(&mut self) -> Result<()> {
        self.state = Row::new(&self.set, &self.border, &self.state)?.to_vec()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_shift_program() {
        let border = Tile::new(0, 0, 0, 0);
        let starter_tile = Tile::new(0, 0, 10, 0);
        let set_and_shift = Tile::new(10, 7, 1, 0);
        let stay_set = Tile::new(1, 0, 1, 0);
        let shift_and_repeat = Tile::new(0, 0, 10, 7);
        let set: HashSet<Tile> = vec![
            border,
            starter_tile,
            set_and_shift,
            stay_set,
            shift_and_repeat,
        ]
        .into_iter()
        .collect();
        let init: BoardState = vec![starter_tile];
        let mut board = Mosaic::new(set, border, init).expect("should construct successfully");

        for _ in 0..3 {
            board.step().expect("should step successfully");
        }

        let state: Vec<TileRef> = vec![stay_set, stay_set, set_and_shift, shift_and_repeat]
            .iter()
            .map(|tile| *board.set.get(tile).expect("tile present"))
            .collect();
        assert_eq!(state, board.state);
    }
}
