use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;
use std::ops::Index;
use std::ops::Neg;

pub type Pip = usize;
pub fn pip_from_components(position: usize, value: usize) -> Pip {
    // N.B., We don't need a "head" field because the "position" (i.e., program
    // counter) is ALWAYS greater than 0 and is ONLY used in the "head" tile.
    // So if we see a program counter then we know that's where the head is
    // located.
    // XXX This should be a bitfield of some sort. Essentially the position is the top, head is
    // bit 1, and value is bit 0.
    assert!(position < ((std::usize::MAX << 1) & std::usize::MAX));
    assert!(!!value == value);

    (position << 1) | ((value & 1) << 0)
}

pub const EMPTY_PIP: Pip = 0;
pub const ZERO_PIP: Pip = 0;
pub const ONE_PIP: Pip = 1;
pub const UNALLOCATED_PIP: Pip = (std::u32::MAX >> 1) as Pip; // XXX This is tightly coupled with pip_from_components. This should be a bitfield or something

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    North,
    East,
    South,
    West,
}
pub type Orientation = Direction;

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Direction::North => "North",
                Direction::East => "East",
                Direction::South => "South",
                Direction::West => "West",
            }
        )
    }
}

impl Neg for Direction {
    type Output = Direction;

    fn neg(self) -> Self::Output {
        match self {
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::East => Direction::West,
            Direction::West => Direction::East,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Tile {
    north: Pip,
    east: Pip,
    south: Pip,
    west: Pip,
}

impl std::fmt::Display for Tile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Tile(")?;
        for (i, pip) in vec![self.north, self.east, self.south, self.west]
            .iter()
            .enumerate()
        {
            if *pip == UNALLOCATED_PIP {
                f.write_str("U")?;
            } else {
                f.write_fmt(format_args!("{:x}", pip))?;
            }

            if i + 1 < 4 {
                f.write_str(", ")?;
            }
        }
        f.write_str(")")?;

        Ok(())
    }
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

    #[inline]
    pub fn cardinal(&self, direction: &Direction) -> Pip {
        match direction {
            Direction::North => self.north,
            Direction::East => self.east,
            Direction::South => self.south,
            Direction::West => self.west,
        }
    }
}

type InputAlts<T> = [T; 2];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PurityBias {
    Nothing,
    Hidden,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SideEffects<T> {
    Pure(PurityBias),
    In(InputAlts<T>),
    Out(bool),
}

impl<T: std::cmp::Eq> Ord for SideEffects<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        match self {
            SideEffects::Out(_) => match other {
                SideEffects::Out(_) => Ordering::Equal,
                SideEffects::In(_) => Ordering::Less,
                SideEffects::Pure(_) => Ordering::Less,
            },
            SideEffects::In(_) => match other {
                SideEffects::Out(_) => Ordering::Greater,
                SideEffects::In(_) => Ordering::Equal,
                SideEffects::Pure(_) => Ordering::Less,
            },
            SideEffects::Pure(_) => match other {
                SideEffects::Out(_) => Ordering::Greater,
                SideEffects::In(_) => Ordering::Greater,
                SideEffects::Pure(_) => Ordering::Equal,
            },
        }
    }
}
impl<T: std::cmp::Eq> PartialOrd for SideEffects<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> SideEffects<T> {
    /*
    pub fn is_pure(&self) -> bool {
        match self {
            SideEffects::Pure(_) => true,
            _ => false,
        }
    }
    */

    pub fn is_input(&self) -> bool {
        match self {
            SideEffects::In(_) => true,
            _ => false,
        }
    }

    pub fn is_output(&self) -> bool {
        match self {
            SideEffects::Out(_) => true,
            _ => false,
        }
    }
}

impl<T> std::fmt::Display for SideEffects<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SideEffects::Pure(PurityBias::Hidden) => f.write_str("Pure(hidden)"),
            SideEffects::Pure(_) => f.write_str("Pure"),
            SideEffects::In(_) => f.write_str("In"),
            SideEffects::Out(_) => f.write_str("Out"),
        }?;

        Ok(())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Domino {
    pub side_effect: SideEffects<Tile>,
    pub tile: Tile,
}

impl Domino {
    pub fn pure(tile: Tile) -> Domino {
        Domino {
            side_effect: SideEffects::Pure(PurityBias::Nothing),
            tile: tile,
        }
    }

    pub fn input(tile: Tile, alts: InputAlts<Tile>) -> Domino {
        Domino {
            side_effect: SideEffects::In(alts),
            tile: tile,
        }
    }

    pub fn output(tile: Tile, bit: bool) -> Domino {
        Domino {
            side_effect: SideEffects::Out(bit),
            tile: tile,
        }
    }
}

impl std::fmt::Display for Domino {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("Domino({}, {})", self.side_effect, self.tile))?;

        Ok(())
    }
}

// If we have more than 4 billion then we'll have to bump it
pub type TileRef = u32;

#[derive(Debug)]
pub struct DominoPile {
    // [Out; In; Pure := [Valid; Hidden]]
    buffer: Vec<Tile>,
    as_ref: HashMap<Tile, TileRef>,

    input: HashMap<TileRef, InputAlts<TileRef>>,
    output: HashMap<TileRef, bool>,

    impure_watermark: TileRef,
    hidden_watermark: TileRef,
}

impl DominoPile {
    pub fn new(mut dominoes: Vec<Domino>) -> Self {
        // Sort dominoes into [Out; In; Pure] order.
        dominoes.sort_unstable_by(|x, y| x.side_effect.cmp(&y.side_effect));

        // If a reference is strictly less than watermark then it must be an IO
        // related Tile. This property is guaranteed by the above sort.
        let watermark: TileRef = dominoes.iter().fold(0, |i, x| match x.side_effect {
            SideEffects::Pure(_) => i,
            _ => i + 1,
        });

        // Iterate over all of the In dominoes, pulling out their inner Tiles
        // so that we can add them to the overall set.
        let alts = dominoes.iter().fold(Vec::new(), |mut acc, domino| {
            if let SideEffects::In(inputs) = domino.side_effect {
                for alt in inputs.iter() {
                    acc.push(*alt);
                }
            }

            acc
        });

        // Now buffer contains every Tile that the machine can use. It is also
        // ordered [Out, In, Pure] so we can quickly check to see what style a
        // TileRef refers to. All that is left is to create Input and Output
        // lookup tables.
        let mut buffer: Vec<Tile> = dominoes.iter().map(|domino| domino.tile).clone().collect();

        // We now must append the (valid when placed but invalid when selected)
        // input-alt tiles. Let's keep track of where we've stashed them so
        // that we can use this information as another layer of constraints.
        // TODO Make a test to verify this in a test
        let hidden_watermark = buffer.len() as TileRef;
        buffer.extend(alts);

        let mut as_ref = HashMap::new();
        for (i, tile) in buffer.iter().enumerate() {
            as_ref.insert(*tile, i as TileRef);
        }

        // Create the input lookup table
        let input: HashMap<TileRef, InputAlts<TileRef>> = dominoes
            .iter()
            .filter(|domino| domino.side_effect.is_input())
            .map(|domino| match domino.side_effect {
                SideEffects::In(alts) => {
                    let zero = as_ref[&alts[0]];
                    let one = as_ref[&alts[1]];

                    (as_ref[&domino.tile], [zero, one])
                }
                _ => panic!("We must only operate on SideEffects::In"),
            })
            .clone()
            .collect();

        // Create the output lookup table
        let output: HashMap<TileRef, bool> = dominoes
            .iter()
            .filter(|domino| domino.side_effect.is_output())
            .map(|domino| match domino.side_effect {
                SideEffects::Out(value) => (as_ref[&domino.tile], value),
                _ => panic!("We must only operate on SideEffects::Out"),
            })
            .clone()
            .collect();

        DominoPile {
            buffer: buffer,
            as_ref: as_ref,

            input: input,
            output: output,

            impure_watermark: watermark,
            hidden_watermark: hidden_watermark,
        }
    }

    pub fn get(&self, tile: &Tile) -> Option<&TileRef> {
        self.as_ref.get(tile)
    }

    pub fn get_side_effects(&self, tile_ref: &TileRef) -> SideEffects<TileRef> {
        if *tile_ref >= self.hidden_watermark {
            return SideEffects::Pure(PurityBias::Hidden);
        } else if *tile_ref >= self.impure_watermark {
            return SideEffects::Pure(PurityBias::Nothing);
        }

        if let Some(alts) = self.input.get(tile_ref) {
            SideEffects::In(*alts)
        } else if let Some(value) = self.output.get(tile_ref) {
            SideEffects::Out(*value)
        } else {
            panic!("This should never happen.");
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn get_tile_side_effects(&self, tile: &Tile) -> SideEffects<TileRef> {
        let tile_ref = self.as_ref[tile];

        self.get_side_effects(&tile_ref)
    }

    // The orientation is relative to the pip. In other words, orientation refers to where the
    // pip is located within a tile.
    pub fn matches_pip(&self, pip: &Pip, direction: Orientation) -> Vec<TileRef> {
        let next = -direction;

        self.as_ref
            .iter()
            .filter(|(tile, _)| *pip == tile.cardinal(&next))
            .map(|(_, r)| *r)
            .collect()
    }

    pub fn matches_tile(&self, tile: &Tile, direction: Orientation) -> Vec<TileRef> {
        let pip = tile.cardinal(&direction);
        self.matches_pip(&pip, direction)
    }

    // The orientation is relative to the provided tile. E.g., if we say West, then we look at
    // the westernmost pip of the tile and find all eastern pips that match
    pub fn matches(&self, tile_ref: &TileRef, direction: Orientation) -> Vec<TileRef> {
        let tile = self.buffer[*tile_ref as usize];
        self.matches_tile(&tile, direction)
    }
}

// XXX how can I inline this to the main struct impl?
impl Index<TileRef> for DominoPile {
    type Output = Tile;

    #[inline]
    fn index(&self, index: TileRef) -> &Self::Output {
        self.buffer
            .get(index as usize)
            .expect("Out of bounds access")
    }
}
impl Index<&Tile> for DominoPile {
    type Output = TileRef;

    #[inline]
    fn index(&self, tile: &Tile) -> &Self::Output {
        self.as_ref.get(tile).expect("Out of bounds access")
    }
}

impl std::fmt::Display for DominoPile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DominoPile(")?;
        for (i, tile) in self.buffer.iter().enumerate() {
            f.write_fmt(format_args!("{}", tile))?;

            if i < self.buffer.len() - 1 {
                f.write_str(", ")?;
            }
        }
        f.write_str(")")?;

        Ok(())
    }
}

#[cfg(test)]
mod tile_tests {
    use super::*;

    #[test]
    fn make_tile() {
        let (north, east, south, west) = (0, 1, 2, 3);
        let tile = Tile::new(north, east, south, west);

        assert!(tile.cardinal(&Direction::North) == north);
        assert!(tile.cardinal(&Direction::East) == east);
        assert!(tile.cardinal(&Direction::South) == south);
        assert!(tile.cardinal(&Direction::West) == west);
    }

    #[test]
    fn direction_negation() {
        let (n, e, s, w) = (
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        );
        assert!(n == -s);
        assert!(e == -w);
        assert!(s == -n);
        assert!(w == -e);
    }

    #[test]
    fn tile_to_ref() {
        let tile = Tile::new(0, 0, 0, 0);
        let dominoes = vec![tile].into_iter().map(Domino::pure).collect();
        let pile = DominoPile::new(dominoes);

        let tile_ref = pile.get(&tile).expect("tile should be present");
        assert!(tile == pile[*tile_ref]);
    }

    #[test]
    fn matches() {
        let pip0 = 0;
        let pip1 = 1;
        let fancy = Tile::new(pip0, pip1, pip0, pip1);
        let zero = Tile::new(pip0, 100, 100, 100);
        let tiles = vec![fancy, zero];
        let dominoes = tiles.clone().into_iter().map(Domino::pure).collect();
        let pile = DominoPile::new(dominoes);
        let tile_refs: Vec<TileRef> = tiles
            .iter()
            .map(|tile| *pile.get(tile).expect("tile should be present"))
            .collect();

        // Assume pip0 is the southernmost pip. This means it will attempt to match it with
        // northern pips.
        let matches = pile.matches_pip(&pip0, Direction::South);
        assert!(matches.len() == 2);
        for r in tile_refs.iter() {
            assert!(matches.contains(r));
        }

        // Take the southernmost pip from fancy and find all the (northern) matches
        let matches = pile.matches_tile(&fancy, Direction::South);
        assert!(matches.len() == 2);
        for r in tile_refs.iter() {
            assert!(matches.contains(r));
        }

        // Take the northernmost pip from fancy and find all the (southern) matches
        let matches = pile.matches_tile(&fancy, Direction::North);
        assert!(matches.len() == 1);
        assert!(pile[matches[0]] == fancy);

        // Take any reference and verify the east/west pips match up.
        let chosen_ref = tile_refs[0];
        let chosen_tile = pile[chosen_ref];
        let matches = pile.matches(&chosen_ref, Direction::West);
        assert!(matches.len() == 1);
        assert!(pile[matches[0]] == chosen_tile);
    }

    #[test]
    fn watermark_border() {
        let doms = vec![
            Domino::pure(Tile::new(0, 0, 0, 0)),
            Domino::pure(Tile::new(0, 0, 0, 0)),
            Domino::pure(Tile::new(0, 0, 0, 0)),
            Domino::input(
                Tile::new(1, 1, 1, 1),
                [Tile::new(255, 255, 255, 255), Tile::new(127, 127, 127, 127)],
            ),
            Domino::pure(Tile::new(0, 0, 0, 0)),
            Domino::pure(Tile::new(0, 0, 0, 0)),
            Domino::output(Tile::new(2, 2, 2, 2), false),
            Domino::pure(Tile::new(0, 0, 0, 0)),
        ];

        let pile = DominoPile::new(doms);
        let watermark = pile.impure_watermark;

        match pile.get_side_effects(&watermark) {
            SideEffects::Pure(_) => (),
            _ => panic!("At and above the watermark should be pure Tiles!"),
        };

        let below = pile.get_side_effects(&(watermark - 1));
        match below {
            SideEffects::Pure(_) => panic!("Beneath the watermark should be impure Tiles!"),
            _ => (),
        };
    }

    #[test]
    fn essence_ordering() {
        /*
        assert!(Essence::Out < Essence::In);
        assert!(Essence::In < Essence::Pure);
        */

        let doms = vec![
            Domino::pure(Tile::new(0, 0, 0, 0)),
            Domino::input(
                Tile::new(1, 1, 1, 1),
                [Tile::new(255, 255, 255, 255), Tile::new(127, 127, 127, 127)],
            ),
            Domino::output(Tile::new(2, 2, 2, 2), false),
        ];

        #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
        enum LookingFor {
            Out,
            In,
            Pure,
            Hidden,
        }

        let pile = DominoPile::new(doms);
        let mut state = LookingFor::Out;
        for tile in pile.buffer.iter() {
            if state == LookingFor::Out {
                match pile.get_tile_side_effects(tile) {
                    SideEffects::Out(_) => (),
                    _ => {
                        state = LookingFor::In;
                        ()
                    }
                };
            } else if state == LookingFor::In {
                match pile.get_tile_side_effects(tile) {
                    SideEffects::In(_) => (),
                    SideEffects::Out(_) => panic!("DominoPile::buffer is misordered"),
                    SideEffects::Pure(_) => {
                        state = LookingFor::Pure;
                        ()
                    }
                };
            } else if state == LookingFor::Pure {
                match pile.get_tile_side_effects(tile) {
                    SideEffects::Pure(PurityBias::Nothing) => (),
                    SideEffects::Pure(PurityBias::Hidden) => {
                        state = LookingFor::Hidden;
                        ()
                    }
                    _ => panic!("DominoPile::buffer is misordered"),
                };
            } else if state == LookingFor::Hidden {
                match pile.get_tile_side_effects(tile) {
                    SideEffects::Pure(PurityBias::Hidden) => (),
                    _ => panic!("DominoPile::buffer is misordered"),
                }
            } else {
                panic!("Test is broken. Fix the state transitions");
            }
        }
    }
}
