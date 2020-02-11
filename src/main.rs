use anyhow::Result;

use std::path::Path;
use std::str::FromStr;

mod compiler;
mod constraint;
mod mosaic;
mod tiling;
mod wmach;

use compiler::Backend;

fn main() -> Result<()> {
    //let mut mosaic = wmach::Program::from_file(Path::new("test.wm"))?
    //let mut mosaic = wmach::Program::from_str("top:+> jmp top, top")?
    let mut mosaic = wmach::Program::from_str("->->+>->->+>->->")? // 00100100 => '$'
        .compile()?;

    println!("full: {}", mosaic);

    // https://beautifier.io/
    for i in 0..1000 {
        println!("{}: {}", i, mosaic);

        mosaic.step()?;
    }

    /*
     * TODO
     * - Make east/west pips a different type from north/south?
     *
     * - Think about IO. Will we need to actual throw this down into the Tiles?
     *   + I think we can have a separate hashmap. The key will be the southern
     *     pip that requires IO and the value will be an array of the two possible
     *     result tiles indexed by the IO bit read. Similarly, we could have
     *     another hashmap with the output IO with the key being the output IO
     *     pip and the value being either 0 or 1.
     *   + We could copy the tag productions and just check each tile after the
     *     cloud is generated to see if it is pure, input, or output. We then
     *     replace the input with the corresponding tile or emit the output bit.
     */

    println!("{}", mosaic);

    Ok(())
}
