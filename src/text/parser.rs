use crate::repr::{FuncType, Import, ImportDesc, Module, ResultType, TypeIdx, ValType};

use super::token::{TextToken, Token};

pub struct Parser<'t> {
    pub(super) tokens: &'t [Token],
}

struct IdentifierContext {}

impl IdentifierContext {
    fn register_func(&mut self, name: &str) -> ParseResult<()> {
        // todo
        Ok(())
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ParseContext {
    FuncType,
    Type,
    Params,
    Result,
}

#[derive(Clone, Debug)]
pub enum ParseError {
    FailedExpectedToken,
    UnexpectedEot,
    InvalidModulefield(String),
    ExpectedLparen,
    ExpectedRparen,
    Context(ParseContext, Box<ParseError>),
    InvalidUtf8,
    UnexpectedImport,
}

impl ParseError {
    fn context(self, ctx: ParseContext) -> Self {
        ParseError::Context(ctx, Box::new(self))
    }
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
            _ => Err(ParseError::FailedExpectedToken),
        }
    }

    fn accept_next_token(&mut self) -> Option<&'t Token> {
        let (t, rest) = self.tokens.split_first()?;
        self.tokens = rest;
        println!("token: {:?}", t);
        Some(t)
    }

    fn expect_any_atom(&mut self) -> ParseResult<&str> {
        let (t, rest) = self.tokens.split_first().ok_or(ParseError::UnexpectedEot)?;
        match t {
            Token::Atom(string) => {
                self.tokens = rest;
                Ok(&string)
            }
            _ => Err(ParseError::FailedExpectedToken),
        }
    }

    fn accept_token(&mut self, f: impl FnOnce(&Token) -> bool) -> Option<&'t Token> {
        let (t, rest) = self.tokens.split_first()?;
        if f(t) {
            self.tokens = rest;
            println!("token: {:?}", t);
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
            return Some(ValType::I32);
        }
        if self.accept_atom("i64") {
            return Some(ValType::I64);
        }
        if self.accept_atom("f32") {
            return Some(ValType::F32);
        }
        if self.accept_atom("f64") {
            return Some(ValType::F64);
        }
        if self.accept_atom("v128") {
            return Some(ValType::V128);
        }
        if self.accept_atom("funcref") {
            return Some(ValType::FuncRef);
        }
        if self.accept_atom("externref") {
            return Some(ValType::ExternRef);
        }
        if self.accept_atom("func") {
            return Some(ValType::FuncRef);
        }
        if self.accept_atom("extern") {
            return Some(ValType::ExternRef);
        }
        None
    }

    fn expect_lparen(&mut self) -> ParseResult<()> {
        if self.accept_lparen() {
            Ok(())
        } else {
            Err(ParseError::ExpectedLparen)
        }
    }

    fn expect_rparen(&mut self) -> ParseResult<()> {
        if self.accept_rparen() {
            Ok(())
        } else {
            Err(ParseError::ExpectedRparen)
        }
    }

    fn accept_name(&mut self) -> Option<&'t str> {
        let (t, rest) = self.tokens.split_first()?;
        match t {
            Token::Name(string) => {
                self.tokens = rest;
                Some(&string)
            }
            _ => None,
        }
    }

    fn accept_param(&mut self) -> ParseResult<Option<Vec<ValType>>> {
        let Ok("param") = self.peek_decl() else { return Ok(None) };
        self.expect_lparen()?;
        self.expect_atom("param")?;
        let mut types = vec![];
        while let Some(typ) = self.accept_valtype() {
            types.push(typ);
        }
        self.expect_rparen()?;
        Ok(Some(types))
    }

    fn accept_result(&mut self) -> ParseResult<Option<Vec<ValType>>> {
        let Ok("result") = self.peek_decl() else { return Ok(None) };
        self.expect_lparen()?;
        self.expect_atom("result")?;
        let mut types = vec![];
        while let Some(typ) = self.accept_valtype() {
            types.push(typ);
        }
        self.expect_rparen()?;
        Ok(Some(types))
    }

    fn accept_params(&mut self) -> ParseResult<ResultType> {
        let mut types = vec![];
        while let Some(params) = self.accept_param()? {
            types.extend(params);
        }
        Ok(ResultType { types })
    }

    fn accept_results(&mut self) -> ParseResult<ResultType> {
        let mut types = vec![];
        while let Some(params) = self.accept_result()? {
            types.extend(params);
        }
        Ok(ResultType { types })
    }

    fn expect_functype(&mut self) -> ParseResult<FuncType> {
        self.expect_lparen()?;
        self.expect_atom("func")?;
        let params = self
            .accept_params()
            .map_err(|e| e.context(ParseContext::Params))?;
        let result = self
            .accept_results()
            .map_err(|e| e.context(ParseContext::Result))?;
        self.expect_rparen()?;
        Ok(FuncType {
            from: params,
            to: result,
        })
    }

    fn expect_type(&mut self, ctx: &mut IdentifierContext) -> ParseResult<FuncType> {
        self.expect_lparen()?;
        self.expect_atom("type")?;
        if let Some(name) = self.accept_name() {
            ctx.register_func(name)?;
        }
        let ft = self
            .expect_functype()
            .map_err(|e| e.context(ParseContext::FuncType))?;
        self.expect_rparen()?;
        Ok(ft)
    }

    fn expect_text(&mut self) -> ParseResult<&'t TextToken> {
        let t = self.accept_next_token().ok_or(ParseError::UnexpectedEot)?;
        match t {
            Token::Text(t) => Ok(t),
            _ => Err(ParseError::FailedExpectedToken),
        }
    }

    fn expect_name(&mut self) -> ParseResult<String> {
        let text = self.expect_text()?;
        let Ok(string) = text.try_string() else { return Err(ParseError::InvalidUtf8)};
        Ok(string)
    }

    fn expect_typeidx(&mut self) -> ParseResult<TypeIdx> {
        let Some((t, rest)) = self.tokens.split_first() else { return Err(ParseError::UnexpectedEot) };
        let typidx = match t {
            Token::Nat(n) => TypeIdx(*n as u32),
            _ => return Err(ParseError::FailedExpectedToken),
        };
        self.accept_next_token();
        Ok(typidx)
    }

    fn expect_typeuse(&mut self) -> ParseResult<TypeIdx> {
        let decl = self.peek_decl()?;
        match decl {
            "type" => {
                self.expect_lparen()?;
                self.expect_atom("type")?;
                let typidx = self.expect_typeidx()?;
                self.expect_rparen()?;
                Ok(typidx)
            }
            "param" => todo!(),
            "result" => todo!(),
            _ => return Err(ParseError::FailedExpectedToken),
        }
    }

    fn expect_importdesc_func(&mut self, ctx: &mut IdentifierContext) -> ParseResult<ImportDesc> {
        self.expect_lparen()?;
        self.expect_atom("func")?;
        let id = self.accept_name();
        let typ = self.expect_typeuse()?;
        self.expect_rparen()?;
        Ok(ImportDesc::Func(typ))
    }

    fn expect_importdesc(&mut self, ctx: &mut IdentifierContext) -> ParseResult<ImportDesc> {
        let decl = self.peek_decl()?;
        match decl {
            "func" => self.expect_importdesc_func(ctx),
            "table" => todo!("import table"),
            "memory" => todo!("import memory"),
            "global" => todo!("import global"),
            _ => return Err(ParseError::UnexpectedImport),
        }
    }

    fn expect_import(&mut self, ctx: &mut IdentifierContext) -> ParseResult<Import> {
        self.expect_lparen()?;
        self.expect_atom("import")?;
        let modname = self.expect_name()?;
        let nm = self.expect_name()?;
        let desc = self.expect_importdesc(ctx)?;
        self.expect_rparen()?;
        Ok(Import {
            module: modname,
            nm,
            desc,
        })
    }

    pub(super) fn module(&mut self) -> ParseResult<Module> {
        self.expect_lparen()?;
        self.expect_atom("module")?;
        let mut module = Module::default();
        let mut ctx = IdentifierContext {};
        loop {
            if self.accept_rparen() {
                return Ok(module);
            }
            let decl = self.peek_decl()?;
            println!("decl: {}", &decl);
            match decl {
                "type" => {
                    let typ = self
                        .expect_type(&mut ctx)
                        .map_err(|e| e.context(ParseContext::Type))?;
                    module.types.push(typ);
                }
                "import" => {
                    let import = self.expect_import(&mut ctx)?;
                    module.imports.push(import);
                }
                "func" => todo!("func"),
                "table" => todo!("table"),
                "mem" => todo!("mem"),
                "global" => todo!("global"),
                "export" => todo!("export"),
                "start" => todo!("start"),
                "elem" => todo!("elem"),
                "data" => todo!("data"),
                x => return Err(ParseError::InvalidModulefield(x.to_string())),
            }
        }
    }
}
