use std::{iter::Peekable, collections::{VecDeque, BTreeMap}};

use crate::repr::{self, Module};
use crate::text;
use text::token::Token;

pub struct Script {
    commands: Vec<Command>,
}

pub enum Command {
    Module(repr::Module),
    Action(Action),
    Assert(Assertion),
    Meta(Meta),
}

pub enum Action {
    Invoke,
    Get,
}

pub enum Assertion {

}

pub enum Meta {
    Script { name: Option<String>, subscript: Script },
    Input { name: Option<String>, val: String },
    Output { name: Option<String>, val: String },
}

#[derive(Debug)]
pub enum ParseError {
    UnknownError,
    FailedParsingCommand,
    UnexpectedEof,
    UnexpectedToken,
}

pub enum Tree {
    Single(Token),
    List(VecDeque<Tree>),
}

impl core::fmt::Debug for Tree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Single(arg0) => write!(f, "{:?}", arg0),
            Self::List(arg0) => write!(f, "{:?}", arg0),
        }
    }
}

pub fn tree(tokens: &mut Peekable<impl Iterator<Item = Token>>) -> Result<Tree, ParseError> {
    let Some(next) = tokens.peek() else { return Err(ParseError::UnexpectedEof)};
    match next {
        Token::RightParen => return Err(ParseError::UnexpectedToken),
        Token::LeftParen => {
            let _left = tokens.next().unwrap();
            let mut inner = VecDeque::new();
            while let Ok(t) = tree(tokens) {
                inner.push_back(t);
            }
            let Some(Token::RightParen) = tokens.next() else { return Err(ParseError::UnexpectedToken)};
            Ok(Tree::List(inner))
        },
        a => {
            Ok(Tree::Single(tokens.next().unwrap()))
        }
    }
}

pub fn tokens_to_tree(tokens: Vec<Token>) -> Result<Vec<Tree>, ParseError> {
    let mut tokens = tokens.into_iter().peekable();
    let mut trees = vec![];
    while tokens.peek().is_some() {
        trees.push(tree(&mut tokens)?);
    }

    return Ok(trees)
}

fn to_command(tree: Tree) -> Result<(String, VecDeque<Tree>), ParseError> {
    let Tree::List(mut items) = tree else { return Err(ParseError::UnexpectedToken) };
    let cmd = items.pop_front().ok_or(ParseError::UnexpectedEof)?;
    let Tree::Single(Token::Atom(cmd)) = cmd else { return Err(ParseError::UnexpectedToken) };
    Ok((cmd, items))
}


pub struct Context {
    registered_modules: BTreeMap<String, Module>,
    last_module: Option<Module>,
    errors: Vec<ScriptError>,
}

impl Context {
    fn new() -> Self {
        Self { registered_modules: BTreeMap::new(), last_module: None, errors: vec![] }
    }
}

#[derive(Debug)]
pub enum ScriptError {}

pub fn run_script(input: &str) -> Result<(), ScriptError> {
    let tokens = text::tokenize_script_without_ws(input).unwrap();
    let trees = tokens_to_tree(tokens).unwrap();
    let ctx = Context::new();
    for tree in trees {
        let (cmd, args) = to_command(tree).unwrap();
        //println!("{:?}", args);
        match cmd.as_str() {
            a => panic!("unknown command: {:?}", a),
        };
    }
    Ok(())
}