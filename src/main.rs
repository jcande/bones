use anyhow::Result;

use std::path::Path;

extern crate nom;

mod compiler;
mod constraint;
mod mosaic;
mod tiling;
mod wmach;

// This is required to use the Backend::compile method. There is probably a better/nicer way to do
// this.
use compiler::Backend;

fn main() -> Result<()> {
    let mut mosaic = wmach::Program::from_file(Path::new("test.wm"))?
        .compile()?;

    mosaic.step()?;

    /*
     * TODO
     * - Think about IO. Will we need to actual throw this down into the Tiles?
     * - Implement the actual compiler backend (or at least some of the more trivial
     *      instructions)
     */

    println!("{:?}", mosaic);

    Ok(())
}
