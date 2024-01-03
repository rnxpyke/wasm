use crate::{repr, wat};

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
    UnknownError
}


pub fn parse_script(input: &str) -> Result<Script, ParseError> {
    let tokens = wat::tokenize_script(input).unwrap();
    return Err(ParseError::UnknownError)
}