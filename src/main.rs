use anyhow::Result;
use thiserror::Error;

use std::env;
use std::path::Path;
use std::str::FromStr;

extern crate getopts;

mod compiler;
mod constraint;
mod io_buffer;
mod mosaic;
mod tiling;
mod wmach;

use compiler::Backend;


#[derive(Error, Debug)]
pub enum BoneError {
    #[error("Command-line help requested.")]
    Help,

    #[error("Missing filename.")]
    MissingFilename,

    #[error("Missing source code.")]
    MissingSource,
}

fn go(mosaic: &mut mosaic::Program) -> Result<()> {
    loop {
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
}

fn usage(opts: getopts::Options) -> Result<()> {
    let brief = format!("Usage: <xxx> FILE [options]");
    eprintln!("{}", opts.usage(&brief));

    Err(BoneError::Help)?;

    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut opts = getopts::Options::new();
    opts.optopt("f", "file", "source file to interpret", "NAME");
    opts.optopt("s", "src", "source string to interpret", "SRC-CODE");
    opts.optflag("h", "help", "print this help menu");

    let matches = opts.parse(&args[1..])?;
    if matches.opt_present("h") {
        usage(opts)?;
    }

    let mut mosaic = if matches.opt_present("f") {
        let filename = matches.opt_str("f").ok_or(BoneError::MissingFilename)?;

        wmach::Program::from_file(Path::new(&filename))
    } else if matches.opt_present("src") {
        let src = matches.opt_str("src").ok_or(BoneError::MissingSource)?;

        wmach::Program::from_str(&src)
    } else {
        //usage(opts)
        todo!("this shit sucks")
    }?.compile()?;

    go(&mut mosaic)?;

    println!("\n\n{}", mosaic);

    Ok(())
}
