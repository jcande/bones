use anyhow::Result;

use thiserror::Error;

use std::collections::HashSet;
use std::fmt;

use crate::tiling::pip_from_components;
use crate::tiling::Domino;
use crate::tiling::DominoPile;
use crate::tiling::Pip;
use crate::tiling::SideEffects;
use crate::tiling::Tile;
use crate::tiling::TileRef;
use crate::tiling::EMPTY_PIP;
use crate::tiling::ONE_PIP;
use crate::tiling::UNALLOCATED_PIP;
use crate::tiling::ZERO_PIP;

use crate::constraint::Row;

use crate::compiler;
use crate::wmach;

use crate::io_buffer::IoBuffer;

pub type BoardState = Vec<Tile>;
pub type BoardStateRef = Vec<TileRef>;

#[derive(Debug)]
pub struct Program {
    pile: DominoPile,
    border: TileRef,

    io: IoBuffer<std::io::Stdin, std::io::Stdout>,
    state: BoardStateRef,
}

impl std::fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "Program(Code: {}; Border: {}; State: (",
            self.pile, self.pile[self.border]
        ))?;

        let last = self.state.len() - 1;
        for (i, r) in self.state.iter().enumerate() {
            f.write_fmt(format_args!("{}", self.pile[*r]))?;

            if i < last {
                f.write_str(", ")?;
            }
        }

        f.write_str("))")?;

        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum MosaicError {
    #[error("Invalid tile: {tile}. The tile is not contained in the given tile set")]
    InvalidTile { tile: Tile },

    #[error("Invalid border tile: {tile}. The tile is not contained in the given tile set")]
    InvalidTileBorder { tile: Tile },
}

impl Program {
    pub fn new(set: HashSet<Domino>, border_tile: Tile, initial: BoardState) -> Result<Self> {
        let tiles = DominoPile::new(set.into_iter().collect());

        // Ensure all border tile is contained in the tile-set.
        /*
        let border = *tiles.get(&border_tile)
            .ok_or(Err(MosaicError::InvalidTileBorder { tile: border_tile }))?;
        */
        // XXX Get the above working. for now just unwrap like a savage
        let border = *tiles.get(&border_tile).unwrap();

        // Ensure all tiles in initial are contained in the tile-set.
        for tile in initial.iter() {
            if let None = tiles.get(tile) {
                Err(MosaicError::InvalidTile { tile: *tile })?;
            }
        }
        // Convert Tiles into TileRefs.
        // XXX clean this up so it is less hacky
        let state = initial
            .into_iter()
            .map(|tile| {
                *tiles
                    .get(&tile)
                    .expect("we should have already verified this")
            })
            .collect();

        // TODO precalculate west/east PossibleTiles
        // TODO ensure initial is consistent (i.e., matches with itself!)
        // TODO ensure the Input (impure) tile's alts share the same pips, aside from the south (so the matching rules work)
        // TODO ensure the number of tiles is big enough to be held in a TileRef

        Ok(Self {
            pile: tiles,
            border: border,

            io: IoBuffer::new(),
            state: state,
        })
    }

    fn perform_io(&mut self, state: BoardStateRef) -> Result<BoardStateRef> {
        let mut next = Vec::with_capacity(state.len());
        for r in state.into_iter() {
            let r = match self.pile.get_side_effects(&r) {
                SideEffects::Out(bit) => {
                    self.io.put(bit)?;

                    r
                }
                SideEffects::In(alts) => {
                    let bit = self.io.get()?;

                    if bit {
                        alts[1]
                    } else {
                        alts[0]
                    }
                }
                SideEffects::Pure(_) => r,
            };
            next.push(r);
        }

        Ok(next)

        /*
                state.into_iter().map(|r| {
                    match self.pile.get_side_effects(&r) {
                        SideEffects::Out(bit) => {
                            self.io.put(bit)?;

                            r
                        },
                        SideEffects::In(alts) => {
                            let bit = self.io.get()?;

                            if bit {
                                alts[1]
                            } else {
                                alts[0]
                            }
                        },
                        SideEffects::Pure(_) => r,
                    }
                }).collect()
        */
    }

    // evolve current state to next state
    pub fn step(&mut self) -> Result<()> {
        let next = Row::new(&self.pile, &self.border, &self.state)?.to_vec()?;
        let next = self.perform_io(next)?;

        self.state = next;

        Ok(())
    }

    fn mk_write(position: usize, value: &wmach::WriteOp) -> Vec<Domino> {
        let mut set = Vec::new();

        let north_0 = pip_from_components(position, 0);
        let north_1 = pip_from_components(position, 1);

        let east = EMPTY_PIP;
        let west = EMPTY_PIP;

        let south = match value {
            wmach::WriteOp::Unset => pip_from_components(position + 1, 0),
            wmach::WriteOp::Set => pip_from_components(position + 1, 1),
        };

        let tile_0 = Tile::new(north_0, east, south, west);
        set.push(tile_0);

        let tile_1 = Tile::new(north_1, east, south, west);
        set.push(tile_1);

        set.into_iter().map(Domino::pure).collect()
    }

    fn mk_seek(position: usize, direction: &wmach::SeekOp) -> Vec<Domino> {
        let mut set = Vec::new();
        // This must be UNIQUE per instruction in order to rule out annoying
        // matching problems. We'll simply use the offset+1 for this instruction
        // and be done with it. The idea here is that position MAY be "1" which
        // we treat as a magic number. By adding to it, we at least have >=2
        // which shouldn't ever occur in an east/west pip. This may change
        // later but I hope not. Also, YOLO.
        let bind: Pip = position + 1; // XXX shitty hack just to try shit out

        // Entry point tiles.
        {
            let north_0 = pip_from_components(position, 0);
            let north_1 = pip_from_components(position, 1);

            let (east, west) = match direction {
                wmach::SeekOp::Left => (EMPTY_PIP, bind),
                wmach::SeekOp::Right => (bind, EMPTY_PIP),
            };

            let south_0 = ZERO_PIP;
            let south_1 = ONE_PIP;

            let tile_0 = Tile::new(north_0, east, south_0, west);
            set.push(tile_0);

            let tile_1 = Tile::new(north_1, east, south_1, west);
            set.push(tile_1);
        }

        // Next, bound, tile.
        {
            let north_0 = ZERO_PIP;
            let north_1 = ONE_PIP;
            let north_u = UNALLOCATED_PIP;

            let (east, west) = match direction {
                wmach::SeekOp::Left => (bind, EMPTY_PIP),
                wmach::SeekOp::Right => (EMPTY_PIP, bind),
            };

            let south_0 = pip_from_components(position + 1, 0);
            let south_1 = pip_from_components(position + 1, 1);

            let tile_0 = Tile::new(north_0, east, south_0, west);
            set.push(tile_0);

            let tile_1 = Tile::new(north_1, east, south_1, west);
            set.push(tile_1);

            let tile_u = Tile::new(north_u, east, south_0, west);
            set.push(tile_u);
        }

        set.into_iter().map(Domino::pure).collect()
    }

    fn mk_io(position: usize, rw: &wmach::IoOp) -> Vec<Domino> {
        let mut set = Vec::new();

        let north_0 = pip_from_components(position, 0);
        let north_1 = pip_from_components(position, 1);

        let east = EMPTY_PIP;
        let west = EMPTY_PIP;

        let south_u = 0xdead;
        let south_0 = pip_from_components(position + 1, 0);
        let south_1 = pip_from_components(position + 1, 1);

        match rw {
            wmach::IoOp::In => {
                let tile_0 = Tile::new(north_0, east, south_u, west);
                let tile_0_0 = Tile::new(north_0, east, south_0, west);
                let tile_0_1 = Tile::new(north_0, east, south_1, west);
                let domino_0 = Domino::input(tile_0, [tile_0_0, tile_0_1]);
                set.push(domino_0);

                let tile_1 = Tile::new(north_1, east, south_u, west);
                let tile_1_0 = Tile::new(north_1, east, south_0, west);
                let tile_1_1 = Tile::new(north_1, east, south_1, west);
                let domino_1 = Domino::input(tile_1, [tile_1_0, tile_1_1]);
                set.push(domino_1);
            }
            wmach::IoOp::Out => {
                let tile_0 = Tile::new(north_0, east, south_0, west);
                let domino_0 = Domino::output(tile_0, false);
                set.push(domino_0);

                let tile_1 = Tile::new(north_1, east, south_1, west);
                let domino_1 = Domino::output(tile_1, true);
                set.push(domino_1);
            }
        };

        set
    }

    fn mk_jmp(position: usize, br_t: &wmach::InsnOffset, br_f: &wmach::InsnOffset) -> Vec<Domino> {
        let mut set = Vec::new();

        let north_0 = pip_from_components(position, 0);
        let north_1 = pip_from_components(position, 1);

        let east = EMPTY_PIP;
        let west = EMPTY_PIP;

        let south_0 = pip_from_components(br_f + BASE_OFFSET, 0);
        let south_1 = pip_from_components(br_t + BASE_OFFSET, 1);

        let tile_0 = Tile::new(north_0, east, south_0, west);
        set.push(tile_0);

        let tile_1 = Tile::new(north_1, east, south_1, west);
        set.push(tile_1);

        set.into_iter().map(Domino::pure).collect()
    }
}

// This means we only get a single row to setup the environment
const BASE_OFFSET: usize = 1;

// This is our compiler from w-machine to wang tiles.
impl compiler::Backend for wmach::Program {
    type Target = Program;

    fn compile(&self) -> Result<Self::Target> {
        let mut set: Vec<Tile> = Vec::new();

        // Void Wranglers
        {
            // Have some stopgap tiles so we don't grow each row.
            let west_alcove =
                Tile::new(UNALLOCATED_PIP, EMPTY_PIP, UNALLOCATED_PIP, UNALLOCATED_PIP);
            set.push(west_alcove);
            let east_alcove =
                Tile::new(UNALLOCATED_PIP, UNALLOCATED_PIP, UNALLOCATED_PIP, EMPTY_PIP);
            set.push(east_alcove);
        }

        // Defaults
        {
            let persist_0 = Tile::new(ZERO_PIP, EMPTY_PIP, ZERO_PIP, EMPTY_PIP);
            set.push(persist_0);

            let persist_1 = Tile::new(ONE_PIP, EMPTY_PIP, ONE_PIP, EMPTY_PIP);
            set.push(persist_1);
        }

        // This is our void. It sorrounds us on every side.
        let border = Tile::new(
            UNALLOCATED_PIP,
            UNALLOCATED_PIP,
            UNALLOCATED_PIP,
            UNALLOCATED_PIP,
        );
        set.push(border);

        let unique_magic = 0x41414141;
        // first instruction starts at BASE_OFFSET because it makes my life easier here
        let start_pip = pip_from_components(BASE_OFFSET, 0);
        let initial = Tile::new(UNALLOCATED_PIP, unique_magic, start_pip, unique_magic);
        set.push(initial);
        let initial_west = Tile::new(UNALLOCATED_PIP, unique_magic, EMPTY_PIP, UNALLOCATED_PIP);
        set.push(initial_west);
        let initial_east = Tile::new(UNALLOCATED_PIP, UNALLOCATED_PIP, EMPTY_PIP, unique_magic);
        set.push(initial_east);

        // Convert the pure tiles into dominoes.
        let mut set: Vec<Domino> = set.into_iter().map(Domino::pure).collect();

        for (i, insn) in self.instructions.iter().enumerate() {
            let i = i + BASE_OFFSET;
            let mut translated = match insn {
                wmach::Insn::Write(value) => Program::mk_write(i, value),
                wmach::Insn::Seek(direction) => Program::mk_seek(i, &direction),
                wmach::Insn::Io(rw) => Program::mk_io(i, &rw),
                wmach::Insn::Jmp(branch_t, branch_f) => Program::mk_jmp(i, &branch_t, &branch_f),
                wmach::Insn::Debug => {
                    todo!("debug: {:?}", insn);
                }
            };

            set.append(&mut translated);
        }

        Program::new(
            set.into_iter().collect(),
            border,
            vec![initial_west, initial, initial_east],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug() {
        // XXX TODO "+<<+" and see why it sometimes fails

        let border = Tile::new(0, 0, 0, 0);
        let starter = Tile::new(0, 0, 6, 0);
        let shift = Tile::new(6, 42, 0, 0);
        let next_1 = Tile::new(1, 0, 11, 42);
        let next_0 = Tile::new(0, 0, 10, 42);
        // Program:
        // top: > jmp top, top
        let set: HashSet<Domino> = vec![
            starter, border, shift, next_1,
            next_0,
            /*
            Tile::new(1, 0, 1, 0),
            Tile::new(10, 0, 6, 0),
            Tile::new(11, 0, 7, 0),
            Tile::new(7, 42, 1, 0),
            */
        ]
        .into_iter()
        .map(Domino::pure)
        .collect();
        let init: BoardState = vec![starter];
        let mut board = Program::new(set, border, init).expect("should construct successfully");

        board.step().expect("step ok");

        // XXX there's a bug where it selects next_1 instead of next_0. This is strange as the
        // border contains only zeroes. How is this satisfying the constraints?
        println!("board: {:?}", board);
        for r in board.state.iter() {
            println!("{} => {:?}", *r, board.pile[*r]);
        }
        println!("border: {:?}", border);

        assert!(board.pile[board.state[1]] == next_0);
    }

    #[test]
    fn set_and_shift_program() {
        let border = Tile::new(0, 0, 0, 0);
        let starter_tile = Tile::new(0, 0, 10, 0);
        let set_and_shift = Tile::new(10, 7, 1, 0);
        let stay_set = Tile::new(1, 0, 1, 0);
        let shift_and_repeat = Tile::new(0, 0, 10, 7);
        let set: HashSet<Domino> = vec![
            border,
            starter_tile,
            set_and_shift,
            stay_set,
            shift_and_repeat,
        ]
        .into_iter()
        .map(Domino::pure)
        .collect();
        let init: BoardState = vec![starter_tile];
        let mut board = Program::new(set, border, init).expect("should construct successfully");

        for _ in 0..3 {
            board.step().expect("should step successfully");
        }

        let state: Vec<TileRef> = vec![stay_set, stay_set, set_and_shift, shift_and_repeat]
            .iter()
            .map(|tile| *board.pile.get(tile).expect("tile present"))
            .collect();
        assert_eq!(state, board.state);
    }
}
