use core::fmt;
use std::iter::Peekable;

use super::{token::{TextToken, Token}, tokenize_script_without_ws, InputError};

#[derive(Clone, PartialEq)]
pub enum Sexpr {
    Atom(String),
    Name(String),
    Text(TextToken),
    Nat(usize),
    Int(isize),
    Float(f64),
    Equal,
    List(Vec<Sexpr>)
}

impl fmt::Debug for Sexpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Atom(arg0) => write!(f, "{}", arg0),
            Self::Name(arg0) => write!(f, "${}", arg0),
            Self::Text(arg0) => write!(f, "{:?}", arg0),
            Self::Nat(arg0) => write!(f, "{}", arg0),
            Self::Int(arg0) => write!(f, "{}", arg0),
            Self::Float(arg0) => write!(f, "{}", arg0),
            Self::Equal => write!(f, "Equal"),
            Self::List(arg0) => write!(f, "{:#?}", arg0),
        }
    }
}

pub fn parse_module_to_sexpr(input: &str) -> Result<Sexpr, InputError> {
    let tokens = tokenize_script_without_ws(&input).map_err(InputError::Tokenizing)?;
    let mut tokens_iter = tokens.into_iter().peekable();
    let sexpr = tokens_to_sexpr(&mut tokens_iter).unwrap();
    Ok(sexpr)
}


fn tokens_to_sexpr<I>(tokens: &mut Peekable<I>) -> Option<Sexpr>
where I: Iterator<Item=Token>
{
    let t = tokens.next()?;
    let expr = match t {
        Token::LeftParen => {
            let mut exprs = vec![];
            let expr = loop {
                if let Some(&Token::RightParen) = tokens.peek() {
                    tokens.next();
                    break Sexpr::List(exprs);
                }
                if let Some(expr) = tokens_to_sexpr(tokens) {
                    exprs.push(expr)
                }
            };
            expr
        },
        Token::RightParen => panic!("should not encounter right paren"),
        Token::Atom(a) => Sexpr::Atom(a),
        Token::Name(n) => Sexpr::Name(n),
        Token::Text(t) => Sexpr::Text(t),
        Token::Nat(n) => Sexpr::Nat(n),
        Token::Int(i) => Sexpr::Int(i),
        Token::Float(f) => Sexpr::Float(f),
        Token::Equal => Sexpr::Equal,
        Token::Comment(_) => return None,
        Token::Whitespace => return None,
    };
    return Some(expr)
}