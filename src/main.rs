use std::collections::HashMap;
use std::collections::HashSet;

type Pip = usize;

enum Orientation {
    Forward,
    Backward,
}

enum Axis {
    Latitude,
    Longitude,
}

enum Direction {
    North,
    East,
    South,
    West,
}

// or Bone!

// A constraint satisfaction problem (CSP) has these 3 components:
// 1) set of variables
// 2) Domain for each variable (i.e., set of values each variable can have)
// 3) set of constraints/relations between each variable
// In our case, each variable is a tile, the domain is the tileset, and the constraints are
// relations between each side of the tile
// Luckily this is pretty nice and let's us solve a "row" at a time. The previous row caps the
// tiles for the current row (so our domain is nicely contained that way). Then we just need to
// continually apply the constraints of each side (E and W) until we reach a fixpoint. If there is
// more than 1 possible tile for a slot then we'll just yolo-choose one as any should work by
// that point.

// (looks like this is called Constraint Propogation)
// We want arc-consistency (I think path-consistency might be overkill)
// We want local-search
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct Tile {
    north: Pip,
    east:  Pip,
    south: Pip,
    west:  Pip,
}

impl Tile {
    pub fn new(north: Pip, east: Pip, south: Pip, west: Pip) -> Self {
        Self {
            north,
            east,
            south,
            west,
        }
    }

    pub fn cardinal(&self, direction: &Direction) -> Pip {
        match direction {
            Direction::North => self.north,
            Direction::East  => self.east,
            Direction::South => self.south,
            Direction::West  => self.west,
        }
    }
}



#[derive(Debug)]
// XXX this probably needs a reference to TileSet otherwise a caller could confuse them and then
// we would crash
struct PossibleTiles {
    cloud: HashSet<TileRef>,
}

impl PossibleTiles {
    pub fn new(initial: Vec<TileRef>) -> Self {
        Self {
            cloud: initial.into_iter().collect(),
        }
    }

    pub fn print_tiles(&self, tiles: &TileSet) {
        for r in self.cloud.iter() {
            println!("{}: {:?}", r, tiles[r]);
        }
    }

    pub fn lookup(&self, tiles: &TileSet, direction: &Direction) -> HashSet<Pip> {
        self.cloud.iter()
            .cloned()
            .map(|ref r| tiles[r].cardinal(direction))
            .collect()
    }

    // XXX gotta fix these stupid references as it is such a pain in the ass
    // probably need a helper method on the tileset like as_ref taking a Tile
    // TODO make a list of all use-cases to see what would make sense
    pub fn contains_tile(&self, tile_ref: &TileRef) -> bool {
        let result = self.cloud.contains(tile_ref);
        println!("ref: {}, included?: {}", tile_ref, result);
        self.cloud.contains(tile_ref)
    }

   // pub fn constrain_pip(&mut self, tiles: &TileSet, direction: &Direction, values: &[Pip]) -> bool {
    pub fn constrain_pip(&mut self, tiles: &TileSet, direction: &Direction, value: &Pip) -> bool {
        // XXX maybe with_capacity(self.cloud.len())? We'll use more space but probably faster
        let mut keep = HashSet::new();
        for r in self.cloud.iter() {
            if tiles[r].cardinal(direction) == *value {
                keep.insert(*r);
            }
        }
        self.cloud = keep;

        self.cloud.len() > 0
    }

    // TODO
    pub fn constrain(&mut self, tiles: &TileSet, orientation: &Orientation, other: &PossibleTiles) -> bool {
        // XXX maybe with_capacity(self.cloud.len())? We'll use more space but probably faster
        let mut keep = HashSet::new();
        let (current, next) = match orientation {
            Orientation::Forward  => (Direction::East, Direction::West),
            Orientation::Backward => (Direction::West, Direction::East),
        };

        let table = other.lookup(tiles, &next);
        for r in self.cloud.iter() {
            let pip = tiles[r].cardinal(&current);
            if table.contains(&pip) {
                keep.insert(*r);
            }
        }
        self.cloud = keep;
        println!("keeping: {:?}", self.cloud);

        self.cloud.len() > 0
    }
}

type TileRef = u32;
type TileSet = HashMap<TileRef, Tile>;
type RevTileSet = HashMap<Tile, TileRef>;
type BoardState = Vec<Tile>;
type BoardStateRef = Vec<TileRef>;

#[derive(Debug)]
struct Board {
    // XXX just make TileSet a fucking vector and then we don't need the other set
    from_ref: TileSet,
    from_tile: RevTileSet,
    border: Tile,

    state: BoardStateRef,
}

impl Board {
    // XXX make a type for the return value
    fn matches(&self,
               tile: &Tile,
               axis: Axis)
        -> Vec<TileRef> {
        // Get a collection of all possible tiles that we can place
        let (current, next) = match axis {
            Axis::Latitude  => (Direction::South, Direction::North),
            Axis::Longitude => (Direction::East,  Direction::West),
        };

        let value = tile.cardinal(&current);
        self.from_tile.iter()
            .filter(|(tile, _)| value == tile.cardinal(&next))
            .map(|(_, r)| *r)
            .collect()
    }

    // XXX learn how to use E from result properly
    pub fn new(set: HashSet<Tile>,
               border_tile: Tile,
               initial: BoardState) -> Result<Self, ()> {
        let mut board = BoardStateRef::with_capacity(set.len() + 2);
        let mut tiles = TileSet::new();
        let mut reverse = HashMap::new();

        if !set.contains(&border_tile) {
               Err(())?;
        }
        // Ensure all tiles in initial are contained in the tile-set.
        for tile in initial.iter() {
            if !set.contains(tile) {
                Err(())?;
            }
        }


        for (i, tile) in set.iter().enumerate() {
            // Now add the "reference" and the tile to our tile-set.
            tiles.insert(i as TileRef, *tile);
            reverse.insert(*tile, i as TileRef);
        }

        // XXX we don't insert the borders. I think we need to do this on-demand. Thinking about
        // the cases of where we'll need to do this is not fun. Probably need to make a
        // customer iterator or something.
        for tile in initial.iter() {
            board.push(reverse[tile]);
        }

        // XXX precalculate west/east PossibleTiles

        Ok(Self {
            from_ref: tiles,
            from_tile: reverse,
            border: border_tile,
            state: board,
        })
    }

    // evolve current state to next state
    pub fn step(&mut self) -> bool {
        // the north/south stuff must match
        // the east/west stuff must match

        let mut next: Vec<PossibleTiles> = Vec::with_capacity(self.state.len() + 2);
        //let west = 0;
        //let east = next.len() - 1;

        // Add the latitude connection.
        for (i, r) in self.state.iter().enumerate() {
            /*
            if i == west || i == east {
                next.push();
            } else {
            */ {
                let tile = &self.from_ref[r];
                let possible = self.matches(tile, Axis::Latitude);
                let cloud = PossibleTiles::new(possible);
                next.push(cloud);
            }
        }
        println!("initial next: {:?}", next);

        // At this point we have the new row and must constrain it.

        //for (i, ref mut cloud) in next.iter_mut().enumerate() {
        for i in 0..next.len() {
            /*
            if i == west {
                let cloud = &mut next[i];
                // western frontier
                let westmost = self.western_border.cardinal(&Direction::East);
                cloud.constrain_pip(&self.from_ref, &Direction::West, &westmost);
            } else if i == east {
                let cloud = &mut next[i];
                // eastern frontier
                let eastmost = self.eastern_border.cardinal(&Direction::West);
                cloud.constrain_pip(&self.from_ref, &Direction::East, &eastmost);
            } else {
            */
            {
                // XXX we need to use previous and next tiles to constrain it
                // XXX if there is only 1 tile remaining, are we done?
                /*println!("oh lawdy, {}: {:?}", i, cloud);
                for r in vec![0, 1, 2, 3, 4, 5, 6, 7, 8].iter() {
                    println!("{}: {:?}", r, self.from_ref[r]);
                }*/

                // iterate over the previous and next clouds, constraining our current cloud's
                // possibilities
                // XXX this is O(n^2) :(
                {
                    /*
                    let pred  = &    next[i-1];
                    let cloud = &mut next[i];
                    */
                    let (earlier, later) = next[i-1..i+1].split_at_mut(1);
                    let pred = &earlier[0];
                    let cloud = &mut later[0];
                    cloud.constrain(&self.from_ref, &Orientation::Backward, pred);
                }

                {
                    /*
                    let succ  = &    next[i+1];
                    let cloud = &mut next[i];
                    */
                    let (earlier, later) = next[i..i+2].split_at_mut(1);
                    let cloud = &mut earlier[0];
                    let succ = &later[0];
                    cloud.constrain(&self.from_ref, &Orientation::Forward,  succ);
                }
            }
        }
        println!("constrained next: {:?}", next);
        for cloud in next.iter() {
            cloud.print_tiles(&self.from_ref);
        }

        // XXX after all the constraints have been added we should have essentially 1 tile per
        // entry. Now we should check the frontiers to see if they match the west/east tiles
        // listed. If not, we should add them for the next step.
        /*
        if next[west].contains_tile(&self.from_tile[&self.border]) {
            println!("got that west!");
        }
        if next[east].contains_tile(&self.from_tile[&self.border]) {
            println!("got that east!");
        }
        */

        /*
          latitude_constraints = |left_tile tile right_tile|
              (tile(E) = right_tile(W)) &&
              (tile(W) = left_tile(E))
          new_state = state::new(tileset,
                          w_border_tile,
                          e_border_tile,
                          latitude_constraints);

          for tile in currentState
          {
              domain_constraints = {all tiles with N = tile(S)}
              new_state.add_variable(domain_constraints)
          }
        
          if (!new_state.constrain()) { panic!("couldn't tile this bitch") }
        */

        /*
        self.state.iter().fold(Vec::new(), |acc, tile| {
            let possible = Board::matches(tiles, tile.clone(), Axis::Latitude);
            acc.add_constraints(possible);
            acc.feedback or forward based on Longitude
        });
        */


        /*
        if false {
        // naive tiling without backtracking
        for tile in self.state.iter() {
            let tiles = self.tiles.iter().collect();
            let possible = Board::matches(tiles, tile.clone(), Axis::Latitude);

            println!("possible for {}: {:?}", tile.cardinal(&Direction::South), possible);

            // XXX this requires having a previously seen tile! E.g., this won't work on the very
            // first tile
            // XXX the code below is nonsense... We need to build a chain out of state, not these
            // tiles (which come from the current level or the next, and are latitude
            // dependent)
            let refined_possible = possible.clone();
            for tile in possible.iter() {
                let possible = Board::matches(refined_possible.clone(), *tile.clone(), Axis::Longitude);

                if possible.len() > 0 {
                    // we have at least one tile
                    // Since we aren't backtracking, choose it!
                    state.push(possible[0].clone());
                    break;
                }
            }
        }

        self.state = state;
        println!("next: {:?}", self.state);
    }
    */

        true
    }
}

fn main() -> Result<(), ()> {
    let set: HashSet<Tile> = vec![
                            // misc tiles
                            // 0, 0, H0, 0 (-> _0)
                            Tile::new(0, 0, 0100, 0),

                            // H0, 0, H1, 0
                            Tile::new(0100, 0, 1101, 0),
                            // H1, 0, H1, 0
                            Tile::new(0101, 0, 1101, 0),
                            Tile::new(  0, 0,   0, 0),
                            Tile::new(  1, 0,   1, 0),
                            // +_0 -> _1

                            // H0, >, 0, 0
                            Tile::new(1100, 62, 0, 0),
                            // H1, >, 0, 1
                            Tile::new(1101, 62, 0, 1),

                            // 0, 0, H0, >
                            Tile::new(0, 0, 0100, 62),
                            // 1, 0, H1, >
                            Tile::new(1, 0, 0101, 62),
                            // >_1 -> _0
    ].into_iter().collect();
    // We have the 0 pip for the EASTERN side as that is what connects this to the rest of the
    // row. Likewise we have a 0 for the east tile for analogous reasons.
    let border = Tile::new(0, 0, 0, 0);
    let init: BoardState = vec![Tile::new(0, 0, 0100, 0)];
    let mut board = Board::new(set, border, init)?;

    // idea: lazily chain (by longitude) "clouds" of possible tiles where each cloud
    // corresponds to all possible successor tiles of a given tile's latitude
    // I guess this would be a nice fold: for each tile, generate list of possibilities by
    // latitude and take that union with the accumulated-cloud by longitude
    board.step();

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construct() {
        let set: TileSet = vec![Tile::new(0, 0, 0, 0), Tile::new(0, 0, 1, 0)].into_iter().collect();
        let init: BoardState = vec![Tile::new(0, 0, 0, 0)];

        //assert_eq!(Ok(Board { tiles: set, state: init }), Board::new(set, init));
    }
}
