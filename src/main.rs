use std::collections::HashSet;

mod tiling;
use crate::tiling::Tile;

use crate::tiling::TileSet;
use crate::tiling::TileRef;

mod constraint;
use crate::constraint::Row;

pub type BoardState = Vec<Tile>;
pub type BoardStateRef = Vec<TileRef>;


#[derive(Debug)]
struct Mosaic {
    set: TileSet,
    border: Tile,

    state: BoardStateRef,
}

impl Mosaic {
    // XXX learn how to use E from result properly
    pub fn new(set: HashSet<Tile>,
               border_tile: Tile,
               initial: BoardState) -> Result<Self, ()> {
        if !set.contains(&border_tile) {
               Err(())?;
        }
        // Ensure all tiles in initial are contained in the tile-set.
        for tile in initial.iter() {
            if !set.contains(tile) {
                Err(())?;
            }
        }

        let tiles = TileSet::new(set.into_iter().collect());

        let state = initial.into_iter()
            .map(|tile| *tiles.get(&tile).expect("we should have already verified this"))
            .collect();
        println!("tile set: {:?}", tiles);

        // XXX precalculate west/east PossibleTiles

        Ok(Self {
            set: tiles,
            border: border_tile,
            state: state,
        })
    }

    // evolve current state to next state
    pub fn step(&mut self) -> bool {
        let row = Row::new(&self.set, &self.border, &self.state);

        if let Some(row) = row.to_vec() {
            self.state = row;
            println!("state: {:?}", self.state);

            true
        } else {
            false
        }
    }
}

fn main() -> Result<(), ()> {
    let set: HashSet<Tile> = vec![
        Tile::new(0, 0, 0, 0),
        Tile::new(0, 0, 10, 0),
        Tile::new(10, 7, 1, 0),
        Tile::new(1, 0, 1, 0),
        Tile::new(0, 0, 10, 7),
    ].into_iter().collect();
    // We have the 0 pip for the EASTERN side as that is what connects this to the rest of the
    // row. Likewise we have a 0 for the east tile for analogous reasons.
    let border = Tile::new(0, 0, 0, 0);
    let init: BoardState = vec![Tile::new(0, 0, 10, 0)];
    let mut board = Mosaic::new(set, border, init)?;

    for i in 0..3 {
        println!("{}", i+1);
        board.step();
    }

    Ok(())
}


#[cfg(test)]
mod tests {
//    use super::*;

    #[test]
    fn construct() {
        /*
        let set: TileSet = vec![Tile::new(0, 0, 0, 0), Tile::new(0, 0, 1, 0)].into_iter().collect();
        let init: BoardState = vec![Tile::new(0, 0, 0, 0)];
        */

        //assert_eq!(Ok(Board { tiles: set, state: init }), Mosaic::new(set, init));
    }
}
