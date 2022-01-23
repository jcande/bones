use std::collections::HashSet;

use crate::tiling;
use crate::tessera;
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

#[derive(PartialEq)]
pub enum TileRetrieval {
    OnlyComputed,
    IncludeBorder,
}
pub struct DapperTile {
    pub coord: (i32, i32),
    pub tile: tiling::Tile,
}
pub struct TileView<'a> {
    row_start: i32,
    row_end: i32,

    _col_start: i32,
    col_end: i32,

    x: i32,
    y: i32,

    model: &'a Mosaic,

    options: TileRetrieval,
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
            if let Some(tile) = self.model.get_tile(coord.0, coord.1, &self.options) {
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

pub struct Mosaic {
    program: tessera::Program,
    mosaic: Vec<TileRow>,
    running: bool,
}
impl<'a> Mosaic {
    pub fn new(source_code: &str) -> anyhow::Result<Self> {
        let program = if crate::RULE110_MODE {
            // This is rule110 taken from https://esolangs.org/wiki/Hao
            let n229 = tiling::Tile::new(0, 0, 0, 0);       // 0
            let n44 = tiling::Tile::new(1, 1, 0, 1);        // 1
            let n3158 = tiling::Tile::new(0, 2, 1, 0);      // 2
            let n54 = tiling::Tile::new(1, 3, 1, 1);        // 3
            let n1538 = tiling::Tile::new(0, 0, 0, 3);      // 4
            let n1539 = tiling::Tile::new(1, 1, 1, 2);      // 5
            let n14876 = tiling::Tile::new(0, 2, 1, 3);     // 6
            let n18144 = tiling::Tile::new(1, 3, 1, 2);     // 7
            let x = 4;
            let y = 5;
            let initial_set_bit = tiling::Tile::new(y, x, 1, x);
            let initial_clear_bit = tiling::Tile::new(y, x, 0, x);
            let cap = tiling::Tile::new(0, x, y, x);
            let west_cap_a = tiling::Tile::new(y, x, 0, 0);
            let west_cap_b = tiling::Tile::new(y, x, 1, 0);
            let east_cap_a = tiling::Tile::new(y, 0, 0, x);
            let east_cap_b = tiling::Tile::new(y, 0, 1, x);
            let border_tile = n229;
            let tiles = [n229, n44, n3158, n54, n1538, n1539, n14876, n18144, initial_set_bit, initial_clear_bit, cap, west_cap_a, west_cap_b, east_cap_a, east_cap_b];
            let tile_set = HashSet::from(tiles.map(tiling::Domino::pure));
            let initial_state_vec = vec![tiles[2], tiles[5], tiles[3], tiles[4]];
            // Yeah we can make it "legit" but it doesn't look as nice so WHO CARES
            //let initial_state_vec = vec![west_cap_a, initial_set_bit, initial_set_bit, initial_clear_bit, east_cap_a];

            tessera::Program::new(tile_set, border_tile, initial_state_vec)?
        } else {
            wmach::Program::from_str(source_code)?
                .compile()?
        };

        let mosaic = vec![TileRow {
            offset: 0,
            tiles: program.state(),
        }];

        Ok(Self {
            program: program,
            mosaic: mosaic,
            running: true,
        })
    }

    pub fn get_tile(&self, row: i32, col: i32, options: &TileRetrieval) -> Option<tiling::Tile> {
        let default = if *options == TileRetrieval::IncludeBorder {
            Some(self.program.border())
        } else {
            None
        };

        // We do not compute backward in time. The initial tape is at col 0.
        if col < 0 {
            return default;
        }

        let col = col as usize;
        if col < self.mosaic.len() {
            assert!(self.mosaic[col].offset <= 0);
            let adjusted = (row - self.mosaic[col].offset) as usize;
            let lower = self.mosaic[col].offset;
            let upper = self.mosaic[col].tiles.len();
            if adjusted >= upper || row < lower {
                return default;
            }

            return Some(self.mosaic[col].tiles[adjusted]);
        }

        return None;
    }

    pub fn compute(&mut self, row_start: i32, row_end: i32, col_start: i32, col_end: i32) -> Result<ComputeCertificate, tessera::MosaicError> {
        // calculate new tiles, if necessary
        if col_end >= 0 {
            while self.mosaic.len() <= (col_end as usize) && self.running {
                if let Err(e) = self.program.step() {
                    log!("Unable to step: {:?}", e);
                    self.running = false;
                    break;
                }

                let state = self.program.state();

                // We have 3 cases:
                //  1) the new state is the same length as the previous one
                //  2) the new state is larger on the western border
                //  3) the new state is larger on the eastern border
                //
                // For 1) we just re-use the previous offset. For 2) and 3) we either change the
                // offset or leave it. The only time we'd need to update the offset is for the
                // western case 2. Let's just examine that and ignore the eastern case.

                assert!(state.len() > 2, "All tile programs should have at least 1 tile and 2
                    borders in the initial state and every subsequent state.");
                let prev = self.mosaic.last().expect("We can only evolve from an initial tile set. Where is that row?");
                let prev_offset = prev.offset;

                let offset = if state.len() == prev.tiles.len() {
                    // This is case 1. There is no expansion of either border.
                    0
                } else {
                    // This is case 2 and 3, but we're only concerning ourselves with the western
                    // expansion case.

                    // Find the western border of the previous state
                    let west_prev = prev.tiles.iter()
                        .find(|tile| **tile != self.program.border())
                        .expect("Somehow the machine state consists entirely of implicit border tiles. This likely shouldn't happen.");

                    // Now see which tile of the current state matches it. This grants us our
                    // offset
                    let offset = state.iter().enumerate()
                        .find(|(_, tile)| west_prev.south == tile.north)
                        .map_or(0, |(i, _)| i) as i32;
                    // Think about the numberline. The west is leftwards which is negative. Since
                    // we had to travel `offset` tiles east-ward relative to the current state,
                    // that means that the current border is the same `offset` westward relative to
                    // the previous state. Since the previous state came first it gets dibs on the
                    // coordinates.
                    -offset
                };

                self.mosaic.push(TileRow {
                    offset: prev_offset + offset,
                    tiles: state,
                });
            }
        }

        Ok(ComputeCertificate {
            row_start: row_start,
            row_end: row_end,
            col_start: col_start,
            col_end: (self.mosaic.len() - 1) as i32,
        })
    }

    pub fn tile_range(&'a self, proof: ComputeCertificate, options: TileRetrieval) -> TileView<'a> {
        // assert that compute() was called before. We seemingly have to split this up due to
        // mutable borrows being required to store the computation not mixing well with immutable
        // borrows into the tiles :(

        TileView {
            row_start: proof.row_start,
            row_end: proof.row_end,

            _col_start: proof.col_start,
            col_end: proof.col_end,

            x: proof.row_start,
            y: proof.col_start,

            model: self,

            options: options,
        }
    }
}

