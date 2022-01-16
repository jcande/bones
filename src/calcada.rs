use crate::tiling;
use crate::mosaic;
use crate::wmach;

use std::str::FromStr;
use crate::compiler::Backend;

// XXX make model either part of mosaic (still not convinced) or a standalone file that has a
// mosaic and keeps track of each step and behaves essentially like the code below expects

// A macro to provide `println!(..)`-style syntax for `console.log` logging.
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

pub struct DapperTile {
    // XXX this should embed the coloring! We could have this be randomly generated by mosaic
    pub coord: (i32, i32),
    pub tile: tiling::Tile,
}
pub struct TileView<'a> {
    row_start: i32,
    row_end: i32,

    col_start: i32,
    col_end: i32,

    x: i32,
    y: i32,

    model: &'a Calcada,
}
impl<'a> Iterator for TileView<'a> {
    type Item = DapperTile;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let coord = (self.x, self.y);

            // Check to see if we're outside the bounds. If that's the case, there are no more tiles
            // remaining in the iterator.
            if self.y > self.col_end {
                return None;
            }

            // Calculate the next tile's coordinate, ensuring we wrap to the next row if we are at the
            // end. We'll check on the next iteration if the computed coordinate is valid. We know that
            // the CURRENT coordinate must be ok and that's what the caller is asking for.
            self.x = self.x + 1;
            if self.x > self.row_end {
                self.x = self.row_start;

                self.y = self.y + 1;
            }

            // If the coordinate does not correspond with a tile (e.g., it is past the border and
            // somewhere in the void), then we'll try the next coordinate; no biggie.
            if let Some(tile) = self.model.get_tile(coord.0, coord.1) {
                return Some(DapperTile {
                    coord: coord,
                    tile: tile,
                });
            }
        }
    }
}

// This is a private/opaque type that serves to ensure the caller must go through our interface.
pub struct ComputeCertificate {
    row_start: i32,
    row_end: i32,

    col_start: i32,
    col_end: i32,
}

struct TileRow {
    offset: i32,
    tiles:  Vec<tiling::Tile>,
}

pub struct Calcada {
    program: mosaic::Program,
    mosaic: Vec<TileRow>,
}
impl<'a> Calcada {
    pub fn new() -> anyhow::Result<Self> {
        let raw_bytes = std::include_bytes!("wasm.wm");
        let wmach_source = String::from_utf8_lossy(raw_bytes);
        let program = wmach::Program::from_str(&wmach_source)?
            .compile()?;

        let mosaic = vec![TileRow {
            offset: 0,
            tiles: program.state(),
        }];

        Ok(Self {
            program: program,
            mosaic: mosaic,
        })
    }

    // this should fail if we don't have the tile computed
    pub fn get_tile(&self, row: i32, col: i32) -> Option<tiling::Tile> {
        /*
        if row > col || row < -col {
            return None;
        }

        // we don't want negative numbers with modulo
        let row = (row as u32) % 2;
        let col = (col as u32) % 2;

        // bullshit data that will always be valid
        let tile = tiling::Tile {
            north: (col % 2) as usize,
            east: (2 + (row % 2)) as usize,
            south: ((col + 1) % 2) as usize,
            west: (2 + (row + 1) % 2) as usize,
        };
        //log!("pips (nesw): {}, {}, {}, {}", tile.north, tile.east, tile.south, tile.west);
        Some(tile)
        */

        // We do not compute backward in time. The initial tape is at row 0.
        if col < 0 {
            return None;
        }
        let col = col as usize;

        assert!(self.mosaic[col].offset <= 0);
        let adjusted = (row - self.mosaic[col].offset) as usize;
        let lower = self.mosaic[col].offset;
        let upper = self.mosaic[col].tiles.len();
        if adjusted >= upper || row < lower {
            return None;
        }

        Some(self.mosaic[col].tiles[adjusted])
    }

    pub fn compute(&mut self, row_start: i32, row_end: i32, col_start: i32, col_end: i32) -> Option<ComputeCertificate> {
        // calculate new tiles, if necessary
        if col_end >= 0 {
            while self.mosaic.len() <= (col_end as usize) {
                self.program.step()
                    .ok()?;
                let state = self.program.state();
                // XXX need to figure out which direction (east vs west) the state grew...
                self.mosaic.push(TileRow {
                    offset: 0,//XXX TODO FIXME this is wrong because we don't calculate which direction the state grew to impact this offset
                    tiles: state,
                });
            }
        }

        Some(ComputeCertificate {
            row_start: row_start,
            row_end: row_end,
            col_start: col_start,
            col_end: col_end,
        })
    }

    pub fn tile_range(&'a self, proof: ComputeCertificate) -> TileView<'a> {
        // assert that compute() was called before. We seemingly have to split this up due to
        // mutable borrows being required to store the computation not mixing well with immutable
        // borrows into the tiles :(

        TileView {
            row_start: proof.row_start,
            row_end: proof.row_end,

            col_start: proof.col_start,
            col_end: proof.col_end,

            x: proof.row_start,
            y: proof.col_start,

            model: self,
        }
    }
}

