use thiserror::Error;

use std::collections::HashSet;
use std::fmt;

use crate::tiling::pip_from_components;
use crate::tiling::Direction;
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
use crate::constraint::RowError;

use crate::compiler;
use crate::wmach;

use crate::io_buffer::IoBuffer;

pub type BoardState = Vec<Tile>;
pub type BoardStateRef = Vec<TileRef>;

#[derive(Error, Debug)]
pub enum MosaicError {
    #[error("IO: {source}")]
    Io {
        #[from]
        source: std::io::Error,
        //backtrace: Backtrace,
    },

    #[error("RowError: {source}")]
    Row {
        #[from]
        source: RowError,
    },

    #[error("Invalid tile: {tile}. The tile is not contained in the given tile set.")]
    InvalidTile { tile: Tile },

    #[error("Invalid border tile: {tile}. The tile is not contained in the given tile set.")]
    InvalidTileBorder { tile: Tile },

    // TODO Make a test for this.
    #[error("Too many tiles. You'll need to expand TileRef to a wider type.")]
    TooManyTiles,

    #[error("Invalid input domino: {domino}. Its input alternate tiles must differ only in southern pip.")]
    InvalidInputAlts { domino: Domino },

    #[error("Invalid initial tile: {tile}. This tile does not match its neighbors.")]
    InvalidInitialTile { tile: Tile },

    #[error("Empty initial state.")]
    EmptyInitialState,
}

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

impl Program {
    pub fn new(
        set: HashSet<Domino>,
        border_tile: Tile,
        initial: BoardState,
    ) -> Result<Self, MosaicError> {
        // Ensure the number of tiles is small enough to be held in a TileRef
        if set.len() >= UNALLOCATED_PIP {
            Err(MosaicError::TooManyTiles)?;
        }

        // Ensure there is some state to work with.
        if initial.len() == 0 {
            Err(MosaicError::EmptyInitialState)?;
        }

        // Ensure the Input (impure) tile's alts share the same pips,
        // aside from the south (so the matching rules work)
        for domino in set.iter() {
            let main = domino.tile;
            let alts = match domino.side_effect {
                SideEffects::In(alts) => alts,
                _ => continue,
            };

            for alt in &alts {
                for dir in [Direction::North, Direction::East, Direction::West].iter() {
                    if alt.cardinal(dir) != main.cardinal(dir) {
                        Err(MosaicError::InvalidInputAlts { domino: *domino })?;
                    }
                }
            }
        }

        let tiles = DominoPile::new(set.into_iter().collect());

        // Ensure all border tile is contained in the tile-set.
        let border = *tiles
            .get(&border_tile)
            .ok_or(MosaicError::InvalidTileBorder { tile: border_tile })?;

        // Ensure all tiles in initial are contained in the tile-set and
        // convert Tiles into TileRefs.
        let mut state = Vec::new();
        for tile in initial.iter() {
            let r = tiles
                .get(tile)
                .ok_or(MosaicError::InvalidTile { tile: *tile })?;
            state.push(*r);
        }

        // Ensure initial is consistent (i.e., matches with itself!)
        let first: usize = 0;
        let last: usize = initial.len() - 1;
        for (i, tile) in initial.iter().enumerate() {
            // We do not verify the northern pip because we could feasibly be
            // passsed in some tile machine state that has already evolved from
            // an initial point. Maybe this is a dumb assumption. For now it
            // seems safest.

            // Look westward
            {
                let mut pred = &border_tile;
                if i > first {
                    pred = &initial[i - 1];
                }

                let dir = Direction::West;
                if tile.cardinal(&dir) != pred.cardinal(&-dir) {
                    Err(MosaicError::InvalidInitialTile { tile: *tile })?;
                }
            }

            // Look eastward
            {
                let mut succ = &border_tile;
                if i < last {
                    succ = &initial[i + 1];
                }

                let dir = Direction::East;
                if tile.cardinal(&dir) != succ.cardinal(&-dir) {
                    Err(MosaicError::InvalidInitialTile { tile: *tile })?;
                }
            }
        }

        // TODO precalculate west/east PossibleTiles

        Ok(Self {
            pile: tiles,
            border: border,

            io: IoBuffer::new(),
            state: state,
        })
    }

    pub fn state(&self) -> BoardState {
        self.state.clone()
            .into_iter()
            .map(|r| self.pile[r])
            .collect()
    }

    fn perform_io(&mut self, state: BoardStateRef) -> Result<BoardStateRef, MosaicError> {
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
    }

    // evolve current state to next state
    pub fn step(&mut self) -> Result<(), MosaicError> {
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
        // matching problems. We'll simply use the offset for this instruction
        // and be done with it. The idea here is that position MAY be "1" which
        // we treat as a magic number. This shouldn't ever occur in an
        // east/west pip. This may change later but I hope not. Also, YOLO.
        // TODO Make this into a variable that we increment after each use to
        // ensure we never re-use a value.
        let bind: Pip = position;
        assert!(bind > 0);

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
impl compiler::Backend<Program> for wmach::Program {
    type Target = Program;
    type Error = MosaicError;

    fn compile(&self) -> Result<Self::Target, Self::Error> {
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

        // TODO Use the same mechanism that mk_seek's bind variable should also
        // use. In this way it should be globally unique.
        let unique_magic = std::usize::MAX;
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

    #[test]
    fn check_empty_state() {
        let border = Tile::new(0, 0, 0, 0);
        let set = vec![Domino::pure(border)].into_iter().collect();

        match Program::new(set, border, vec![]) {
            Err(MosaicError::EmptyInitialState) => (),
            _ => panic!("Failed to ensure that initial state is non-empty."),
        };
    }

    #[test]
    fn verify_alts() {
        let border = Tile::new(0, 0, 0, 0);
        let tile = Tile::new(1, 1, 0, 1);
        let alt0 = Tile::new(1, 0xbad, 0, 1);
        let alt1 = Tile::new(1, 1, 0, 1);
        let set = vec![Domino::pure(border), Domino::input(tile, [alt0, alt1])]
            .into_iter()
            .collect();

        match Program::new(set, border, vec![border]) {
            Err(MosaicError::InvalidInputAlts { domino: _ }) => (),
            x => panic!("Failed to ensure that the alts are drop-in replacements of the originator tile: {:?}", x),
        };
    }

    #[test]
    fn verify_border_check() {
        let border = Tile::new(0, 0, 0, 0);
        let initial = Tile::new(1, 1, 1, 1);
        let set = vec![Domino::pure(initial)].into_iter().collect();

        match Program::new(set, border, vec![initial]) {
            Err(MosaicError::InvalidTileBorder { tile: _ }) => (),
            x => panic!("Failed to ensure border tile is present: {:?}", x),
        };
    }

    #[test]
    fn verify_initial_tiles_preset() {
        let border = Tile::new(0, 0, 0, 0);
        let extra = Tile::new(1, 1, 1, 1);
        let set = vec![Domino::pure(border)].into_iter().collect();

        match Program::new(set, border, vec![border, extra]) {
            Err(MosaicError::InvalidTile { tile: _ }) => (),
            x => panic!("Failed to ensure tile is present: {:?}", x),
        };
    }

    #[test]
    fn verify_initial_tiles() {
        let border = Tile::new(0, 0, 0, 0);
        let left = Tile::new(0, 1, 0, 0);
        let right = Tile::new(0, 0xbad, 0, 1);
        let set = vec![border, left, right]
            .into_iter()
            .map(Domino::pure)
            .collect();

        match Program::new(set, border, vec![left, right]) {
            Err(MosaicError::InvalidInitialTile { tile: _ }) => (),
            x => panic!("Failed to ensure tiles match: {:?}", x),
        };
    }
}
