use anyhow::Result;

use std::collections::HashSet;
use std::path::Path;
use std::str::FromStr;

extern crate nom;

mod constraint;
mod emulator;
mod tiling;
mod wmach;

use crate::tiling::Tile;

use crate::emulator::BoardState;
use crate::emulator::Mosaic;

fn main() -> Result<()> {
    let program = wmach::Program::from_file(Path::new("test.wm"))?;

    println!("{:?}", program);

    Ok(())
}
