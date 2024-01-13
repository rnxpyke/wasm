use crate::repr::{ValType, Module, FuncType};

use super::token::Token;

pub struct Parser<'t> {
    pub (super) tokens: &'t [Token],
}

struct IdentifierContext {

}

#[derive(Clone, Debug)]
pub enum ParseError {
    FailedExpectedToken,
    UnexpectedEot,
    InvalidModulefield(String),
}

pub type ParseResult<T> = Result<T, ParseError>;

impl<'t> Parser<'t> {
    fn peek_token(&self) -> Option<&'t Token> {
        self.tokens.first()
    }

    // Decl = LParen atom ...
    fn peek_decl(&self) -> ParseResult<&'t str> {
        let (lparen, rest) = self.tokens.split_first().ok_or(ParseError::UnexpectedEot)?;
        if !matches!(lparen, Token::LeftParen) {
            return Err(ParseError::FailedExpectedToken);
        }
        let (atom, _) = rest.split_first().ok_or(ParseError::UnexpectedEot)?;
        match atom {
            Token::Atom(atom) => Ok(atom.as_str()),
            _ => Err(ParseError::FailedExpectedToken)
        }
    }

    fn accept_next_token(&mut self) -> Option<&'t Token> {
        let (t, rest) = self.tokens.split_first()?;
        self.tokens = rest;
        Some(t)
    }

    fn expect_any_atom(&mut self) -> ParseResult<&str> {
        let (t, rest) = self.tokens.split_first().ok_or(ParseError::UnexpectedEot)?;
        match t {
            Token::Atom(string) => {
                self.tokens = rest;
                Ok(&string)
            },
            _ => Err(ParseError::FailedExpectedToken)
        }
    }

    fn accept_token(&mut self, f: impl FnOnce(&Token) -> bool) -> Option<&'t Token> {
        let (t, rest) = self.tokens.split_first()?;
        if f(t) {
            self.tokens = rest;
            Some(t)
        } else {
            None
        }
    }

    fn accept_lparen(&mut self) -> bool {
        self.accept_token(|t| matches!(t, Token::LeftParen))
            .is_some()
    }

    fn accept_rparen(&mut self) -> bool {
        self.accept_token(|t| matches!(t, Token::RightParen))
            .is_some()
    }

    fn accept_atom(&mut self, atom: &str) -> bool {
        self.accept_token(|t| matches!(t, Token::Atom(atom)))
            .is_some()
    }

    fn expect_atom(&mut self, atom: &str) -> ParseResult<()> {
        if self.accept_atom(atom) {
            Ok(())
        } else {
            Err(ParseError::FailedExpectedToken)
        }
    }

    fn accept_valtype(&mut self) -> Option<ValType> {
        if self.accept_atom("i32") {
            return Some(ValType::I32)
        }
        if self.accept_atom("i64") {
            return Some(ValType::I64)
        }
        if self.accept_atom("f32") {
            return Some(ValType::F32)
        }
        if self.accept_atom("f64") {
            return Some(ValType::F64)
        }
        if self.accept_atom("v128") {
            return Some(ValType::V128)
        }
        if self.accept_atom("funcref") {
            return Some(ValType::FuncRef)
        }
        if self.accept_atom("externref") {
            return Some(ValType::ExternRef)
        }
        if self.accept_atom("func") {
            return Some(ValType::FuncRef)
        }
        if self.accept_atom("extern") {
            return Some(ValType::ExternRef)
        }
        None
    }

    
    fn expect_lparen(&mut self) -> ParseResult<()> {
        if self.accept_lparen() {
            Ok(())
        } else {
            Err(ParseError::FailedExpectedToken)
        }
    }

    fn expect_rparen(&mut self) -> ParseResult<()> {
        if self.accept_rparen() {
            Ok(())
        } else {
            Err(ParseError::FailedExpectedToken)
        }
    }

    fn accept_type(&mut self, ctx: &IdentifierContext) -> Option<FuncType> {
        todo!()
    }

    pub (super) fn module(&mut self) -> ParseResult<Module> {
        self.expect_lparen()?;
        self.expect_atom("module")?;
        let mut module = Module::default();
        let mut ctx = IdentifierContext{};
        loop {
            if self.accept_rparen() {
                return Ok(module);
            }
            match self.peek_decl()? {
                "type" => todo!("type"),
                "import" => todo!("import"),
                "func" => todo!("func"),
                "table" => todo!("table"),
                "mem" => todo!("mem"),
                "global" => todo!("global"),
                "export" => todo!("export"),
                "start" => todo!("start"),
                "elem" => todo!("elem"),
                "data" => todo!("data"),
                x => return Err(ParseError::InvalidModulefield(x.to_string()))
            }
           
        }
    }
}
