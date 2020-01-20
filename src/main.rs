use anyhow::Result;

use std::collections::HashSet;

mod constraint;
mod emulator;
mod tiling;

use crate::tiling::Tile;

use crate::emulator::BoardState;
use crate::emulator::Mosaic;

fn main() -> Result<()> {
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
    let mut board = Mosaic::new(set, border, init)?;

    for _ in 0..3 {
        board.step()?;
    }

    Ok(())
}
