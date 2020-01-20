use std::fs::File;
use std::path::Path;
use std::collections::HashMap;
use std::str::FromStr;
use std::io::Read;
use std::fmt;

use thiserror::Error;
use anyhow::Result;

use nom::{
    branch::alt,

    bytes::complete::tag,
    bytes::complete::take_while1,
    bytes::complete::is_not,

    character::complete::multispace0,
    character::complete::multispace1,
    character::complete::space0,

    combinator::opt,

    multi::many0,

    sequence::pair,
    sequence::separated_pair,
    sequence::tuple,
};

/*
fn forge_symbol(pos: usize, name: &str) -> String {
    let mut symbol = name.to_string();
    symbol.push('_');
    symbol.push_str(&pos.to_string());
    symbol
}
*/

#[derive(Debug, Error)]
pub enum WmachErr {
    #[error("{message}")]
    GeneralError { message: String },

    #[error("Duplicate label: {label}")]
    DuplicateLabel { label: String },

    // this realy should be a LabelId but I don't know how to pull it out of the Target
    #[error("At instruction {offset} unknown target ``{target}'' referenced")]
    UnknownTarget { offset: InsnOffset, target: Target },

    #[error("IO error: {err}")]
    IoError { err: std::io::Error },
}

impl From<std::io::Error> for WmachErr {
    fn from(error: std::io::Error) -> WmachErr {
        WmachErr::IoError {
            err: error,
        }
    }
}

// This is what we get from Stmts
#[derive(Debug, Clone)]
pub enum Insn {
    Write(WriteOp),
    Seek(SeekOp),
    Io(IoOp),
    Jmp(InsnOffset, InsnOffset),
    Debug,
}

// This is what we get from a file
#[derive(Debug, Clone)]
pub enum Stmt {
    Write(WriteOp),
    Seek(SeekOp),
    Io(IoOp),
    Label(LabelId),
    Jmp(Target, Target),
    Debug,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WriteOp {
    Set,
    Unset,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SeekOp {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IoOp {
    In,
    Out,
}


pub type LabelId = String;
pub type InsnOffset = usize;
pub type LabelMap = HashMap<LabelId, InsnOffset>;
#[derive(Debug, Clone)]
pub enum Target {
    NextAddress,
    Name(LabelId),
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Target::NextAddress => write!(f, "<fallthrough>"),
            Target::Name(label) => write!(f, "{}", label),
        }
    }
}

pub type Code = Vec<Insn>;

#[derive(Debug)]
pub struct Program {
    pub instructions: Code,
    labels: LabelMap,
}

impl FromStr for Program {
    type Err = WmachErr;

    fn from_str(unparsed: &str) -> Result<Program, WmachErr> {
        let statements = Program::parse_statements(unparsed)?;

        // make jmp table
        let mut jmp_table: LabelMap = HashMap::new();
        let mut offset: InsnOffset = 0;
        for stmt in statements.iter() {
            if let Stmt::Label(label_id) = stmt {
                if jmp_table.contains_key(label_id) {
                    Err(WmachErr::DuplicateLabel{ label: label_id.to_owned() })?
                }

                jmp_table.insert(label_id.to_owned(), offset);
            } else {
                offset += 1;
            }
        }

        // XXX Might want to bind the previous offset and the subsequent offset more tightly.
        // Maybe we can use the type-system somehow.

        // make instructions
        let mut insns: Vec<Insn> = Vec::new();
        for (offset, stmt) in statements.iter().filter(|stmt| {
            // Skip labels
            match stmt {
                Stmt::Label(_) => false,
                _ => true,
            }
        }).enumerate() {
            let insn = match stmt {
                Stmt::Write(value) => {
                    Insn::Write(*value)
                },
                Stmt::Seek(direction) => {
                    Insn::Seek(*direction)
                },
                Stmt::Io(rw) => {
                    Insn::Io(*rw)
                },
                Stmt::Jmp(branch_t, branch_f) => {
                    let target_address = |branch: &Target| match branch {
                        Target::NextAddress     => Some(offset + 1),
                        Target::Name(label_id)  => jmp_table.get(label_id).cloned(),
                    };

                    // missing label error
                    let t = target_address(branch_t).ok_or(WmachErr::UnknownTarget {
                        offset: offset,
                        target: branch_t.to_owned(),
                    })?;
                    let f = target_address(branch_f).ok_or(WmachErr::UnknownTarget {
                        offset: offset,
                        target: branch_f.to_owned(),
                    })?;

                    Insn::Jmp(t, f)
                },
                Stmt::Debug => Insn::Debug,

                _ => {
                    panic!("Shouldn't reach this");
                },
            };

            insns.push(insn);
        }

        Ok(Program{
            instructions: insns,
            labels: jmp_table,
        })
    }
}

fn label(input: &str) -> nom::IResult<&str, &str> {
    take_while1(|input| {
            // misc = { "'" | '_' }
            // label_id = (alpha | digit | misc)+
            match input {
                'a' ..= 'z' => true,
                'A' ..= 'Z' => true,
                '0' ..= '9' => true,
                '\'' => true,
                '_' => true,
                _ => false,
            }
        })(input)
}

fn label_op(input: &str) -> nom::IResult<&str, Stmt> {
        let colon = tag(":");

        let (input, (label_id, _, _, _)) =
            tuple((label, space0, colon, multispace0))(input)?;
        let label_id = label_id.to_string();

        Ok((input, Stmt::Label(label_id)))
}

fn jmp_op(input: &str) -> nom::IResult<&str, Stmt> {
    let op = tag("jmp");
    let (input, (_, true_branch)) =
        separated_pair(op, multispace1, label)(input)?;
    let true_branch = Target::Name(true_branch.to_string());

    let separator = tag(",");
    let (input, result) =
        opt(tuple((multispace0, separator, multispace0, label, multispace0)))(input)?;
    let false_branch = match result {
        Some((_, _, _, name, _)) => Target::Name(name.to_string()),
        None => Target::NextAddress,
    };

    Ok((input, Stmt::Jmp(true_branch, false_branch)))
}

fn set_op(input: &str) -> nom::IResult<&str, Stmt> {
    let op = tag("+");
    let (input, (_, _)) =
        pair(op, multispace0)(input)?;

    Ok((input, Stmt::Write(WriteOp::Set)))
}

fn unset_op(input: &str) -> nom::IResult<&str, Stmt> {
    let op = tag("-");
    let (input, (_, _)) =
        tuple((op, multispace0))(input)?;

    Ok((input, Stmt::Write(WriteOp::Unset)))
}

fn seek_left_op(input: &str) -> nom::IResult<&str, Stmt> {
    let op = tag("<");
    let (input, (_, _)) =
        tuple((op, multispace0))(input)?;

    Ok((input, Stmt::Seek(SeekOp::Left)))
}

fn seek_right_op(input: &str) -> nom::IResult<&str, Stmt> {
    let op = tag(">");
    let (input, (_, _)) =
        tuple((op, multispace0))(input)?;

    Ok((input, Stmt::Seek(SeekOp::Right)))
}

fn input_op(input: &str) -> nom::IResult<&str, Stmt> {
    let op = tag(",");
    let (input, (_, _)) =
        tuple((op, multispace0))(input)?;

    Ok((input, Stmt::Io(IoOp::In)))
}

fn output_op(input: &str) -> nom::IResult<&str, Stmt> {
    let op = tag(".");
    let (input, (_, _)) =
        tuple((op, multispace0))(input)?;

    Ok((input, Stmt::Io(IoOp::Out)))
}

fn debug_op(input: &str) -> nom::IResult<&str, Stmt> {
    let op = tag("!");
    let (input, (_, _)) =
        tuple((op, multispace0))(input)?;

    Ok((input, Stmt::Debug))
}

fn statement(input: &str) -> nom::IResult<&str, Stmt> {
    alt((label_op, jmp_op, set_op, unset_op, seek_left_op, seek_right_op, input_op, output_op,
         debug_op))(input)
}

fn comment(input: &str) -> nom::IResult<&str, ()> {
    let (input, _) =
        tuple((tag("/*"), is_not("*/"), tag("*/"), multispace0))(input)?;

    Ok((input, ()))
}

fn any_statement(input: &str) -> nom::IResult<&str, Stmt> {
    // XXX Yeah, you can't put a comment anywhere. I am willing to live with that for the time
    // being
    let (input, _) = opt(comment)(input)?;
    let (input, _) = multispace0(input)?;
    statement(input)
}


fn program_statements(input: &str) -> nom::IResult<&str, Vec<Stmt>> {
    many0(any_statement)(input)
}

impl Program {
    fn parse_statements(unparsed: &str) -> Result<Vec<Stmt>, WmachErr> {
        let (rest, statements) = program_statements(unparsed)
            .map_err(|e| WmachErr::GeneralError {
                message: format!("Nom Error: {}", e),
             })?;

        let rest = String::from_utf8(rest
                                     .as_bytes()
                                     .to_vec())
            .expect("Invalid UTF8");
        if rest.len() > 0 {
            Err(WmachErr::GeneralError {
                message: format!("Left over data: {}", rest),
            })?;
        }

        Ok(statements)
    }

    pub fn from_file(filename: &Path) -> Result<Program, WmachErr> {
        let mut unparsed_file = String::new();
        File::open(filename)?.read_to_string(&mut unparsed_file)?;

        Program::from_str(&unparsed_file)
    }

    // XXX should also return some debug symbols (jmp_table?)
    //pub fn compile(&self) -> Result<tag::Program, failure::Error> 
    pub fn compile(&self) -> Result<()> {

        todo!("need to rip out the tag specific bits. Can we make this method a trait?");

        /*
        let mut rules: tag::Rules = HashMap::new();

        for (i, insn) in self.instructions.iter().enumerate() {
            let translated = match insn {
                Insn::Write(value) => {
                    Self::mk_write(i, &value)
                },
                Insn::Seek(direction) => {
                    Self::mk_seek(i, &direction)
                },
                Insn::Io(rw) => {
                    Self::mk_io(i, &rw)
                },
                Insn::Jmp(branch_t, branch_f) => {
                    Self::mk_jmp(i, &branch_t, &branch_f)
                },
                Insn::Debug => {
                    Self::mk_debug(i)   // XXX need to think about how to do this
                },
            };

            rules.extend(translated);
        }

        // XXX start start? This can then generate .data
        let default_queue = vec!["s0_0".to_owned(), "s0_0".to_owned()];
        tag::Program::from_components(2, rules, default_queue)
        */
    }
}

#[cfg(test)]
mod constraint_tests {
    use super::*;

    #[test]
    fn parse_label() {
        let name = "my_label";
        let program = format!("{}:", name);
        let result = label_op(&program);

        let (_, id) = match result {
            Ok(whatever) => whatever,
            _ => panic!("parse failed: {:?}", result),
        };

        let id = match id {
            Stmt::Label(id) => id,
            _ => panic!("wrong variant"),
        };
        assert_eq!(id, name);
    }

    #[test]
    fn parse_jmp_single() {
        let true_branch = "first";
        let program = format!("jmp {}", true_branch);
        let result = jmp_op(&program);

        let (_, jmp) = match result {
            Ok(whatever) => whatever,
            _ => panic!("parse failed: {:?}", result),
        };

        let (branch_a, branch_b) = match jmp {
            Stmt::Jmp(branch_a, branch_b) => (branch_a, branch_b),
            _ => panic!("parsed stmt incorrect: {:?}", jmp),
        };

        match branch_a {
            Target::Name(name) => assert_eq!(name, true_branch),
            _ => panic!("parsed frist branch incorrectly: {:?}", branch_a),
        };
        match branch_b {
            Target::NextAddress => assert!(true),
            _ => panic!("parsed second branch incorrectly: {:?}", branch_b),
        };
    }

    #[test]
    fn parse_jmp_double() {
        let true_branch = "first";
        let false_branch = "first";
        let program = format!("jmp {}, {}", true_branch, false_branch);
        let result = jmp_op(&program);

        let (_, jmp) = match result {
            Ok(whatever) => whatever,
            _ => panic!("parse failed: {:?}", result),
        };

        let (branch_a, branch_b) = match jmp {
            Stmt::Jmp(branch_a, branch_b) => (branch_a, branch_b),
            _ => panic!("parsed stmt incorrect: {:?}", jmp),
        };

        match branch_a {
            Target::Name(name) => assert_eq!(name, true_branch),
            _ => panic!("parsed frist branch incorrectly: {:?}", branch_a),
        };
        match branch_b {
            Target::Name(name) => assert_eq!(name, false_branch),
            _ => panic!("parsed second branch incorrectly: {:?}", branch_b),
        };
    }

    #[test]
    fn parse_set() {
        let program = format!("+");
        let result = set_op(&program);

        let (_, set) = match result {
            Ok(whatever) => whatever,
            _ => panic!("parse failed: {:?}", result),
        };

        let op = match set {
            Stmt::Write(op) => op,
            _ => panic!("parsed stmt incorect: {:?}", set),
        };

        match op {
            WriteOp::Set => assert!(true),
            _ => panic!("invalid op: {:?}", op),
        };
    }

    #[test]
    fn parse_unset() {
        let program = format!("-");
        let result = unset_op(&program);

        let (_, unset) = match result {
            Ok(whatever) => whatever,
            _ => panic!("parse failed: {:?}", result),
        };

        let op = match unset {
            Stmt::Write(op) => op,
            _ => panic!("parsed stmt incorect: {:?}", unset),
        };

        match op {
            WriteOp::Unset => assert!(true),
            _ => panic!("invalid op: {:?}", op),
        };
    }

    #[test]
    fn parse_seek_left() {
        let program = format!("<");
        let result = seek_left_op(&program);

        let (_, stmt) = match result {
            Ok(whatever) => whatever,
            _ => panic!("parse failed: {:?}", result),
        };

        let op = match stmt {
            Stmt::Seek(op) => op,
            _ => panic!("parsed stmt incorect: {:?}", stmt),
        };

        match op {
            SeekOp::Left => assert!(true),
            _ => panic!("invalid op: {:?}", op),
        };
    }

    #[test]
    fn parse_seek_right() {
        let program = format!(">");
        let result = seek_right_op(&program);

        let (_, stmt) = match result {
            Ok(whatever) => whatever,
            _ => panic!("parse failed: {:?}", result),
        };

        let op = match stmt {
            Stmt::Seek(op) => op,
            _ => panic!("parsed stmt incorect: {:?}", stmt),
        };

        match op {
            SeekOp::Right => assert!(true),
            _ => panic!("invalid op: {:?}", op),
        };
    }

    #[test]
    fn parse_input() {
        let program = format!(",");
        let result = input_op(&program);

        let (_, stmt) = match result {
            Ok(whatever) => whatever,
            _ => panic!("parse failed: {:?}", result),
        };

        let op = match stmt {
            Stmt::Io(op) => op,
            _ => panic!("parsed stmt incorect: {:?}", stmt),
        };

        match op {
            IoOp::In => assert!(true),
            _ => panic!("invalid op: {:?}", op),
        };
    }

    #[test]
    fn parse_output() {
        let program = format!(".");
        let result = output_op(&program);

        let (_, stmt) = match result {
            Ok(whatever) => whatever,
            _ => panic!("parse failed: {:?}", result),
        };

        let op = match stmt {
            Stmt::Io(op) => op,
            _ => panic!("parsed stmt incorect: {:?}", stmt),
        };

        match op {
            IoOp::Out => assert!(true),
            _ => panic!("invalid op: {:?}", op),
        };
    }

    #[test]
    fn parse_debug() {
        let program = format!("!");
        let result = debug_op(&program);

        let (_, stmt) = match result {
            Ok(whatever) => whatever,
            _ => panic!("parse failed: {:?}", result),
        };

        match stmt {
            Stmt::Debug => assert!(true),
            _ => panic!("parsed stmt incorect: {:?}", stmt),
        };
    }

    /*
    #[test]
    fn parse_statement() {
        for program in [] {
    }
    */

    // TODO finish tests
}
