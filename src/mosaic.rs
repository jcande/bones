use anyhow::Result;

use thiserror::Error;

use std::collections::HashSet;

use crate::tiling::Tile;

use crate::tiling::TileRef;
use crate::tiling::TileSet;

use crate::constraint::Row;

use crate::wmach;
use crate::compiler;

pub type BoardState = Vec<Tile>;
pub type BoardStateRef = Vec<TileRef>;

#[derive(Debug)]
pub struct Program {
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

impl Program {
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

// This is our compiler from w-machine to wang tiles.
impl compiler::Backend for wmach::Program {
    type Target = Program;

    // XXX remove me later
    #[allow(unreachable_code)]
    fn compile(&self) -> Result<Self::Target> {
        println!("{:?}", self);
        todo!("the compiler");

        // shut the compiler up for now
        let border = Tile::new(0, 0, 0, 0);
        let set: HashSet<Tile> = vec![border].into_iter().collect();
        let init: BoardState = vec![border];
        Ok(Program::new(set, border, init)?)
    }
    //
    // XXX should also return some debug symbols (jmp_table?)
    /*
    pub fn compile(&self) -> Result<crate::mosaic::Program> {

        todo!("sad");

        let mut rules: tag::Rules = HashMap::new();

        for (i, insn) in self.instructions.iter().enumerate() {
            let translated = match insn {
                Insn::Write(value) => {
                    Self::mk_write(i, &value)
                },
                Insn::Seek(direction) => {
                    Self::mk_seek(i, &direction)
                },
                Insn::Io(rw) => {
                    Self::mk_io(i, &rw)
                },
                Insn::Jmp(branch_t, branch_f) => {
                    Self::mk_jmp(i, &branch_t, &branch_f)
                },
                Insn::Debug => {
                    Self::mk_debug(i)   // XXX need to think about how to do this
                },
            };

            rules.extend(translated);
        }

        // XXX start start? This can then generate .data
        let default_queue = vec!["s0_0".to_owned(), "s0_0".to_owned()];
        tag::Program::from_components(2, rules, default_queue)
    }
    */
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
        let mut board = Program::new(set, border, init).expect("should construct successfully");

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
