pub mod parser;
pub mod token;
pub mod sexpr;

pub use token::tokenize_script;
pub use token::tokenize_script_without_ws;

use crate::repr::Module;

use parser::ParseError;
use token::TokenizeError;

#[derive(Debug)]
pub enum InputError {
    Parsing(ParseError),
    Tokenizing(TokenizeError),
}

pub fn parse_module(input: &str) -> Result<Module, InputError> {
    let tokens = tokenize_script_without_ws(&input).map_err(InputError::Tokenizing)?;
    let mut parser = parser::Parser { tokens: &tokens };
    let module = parser.module().map_err(InputError::Parsing)?;
    Ok(module)
}
