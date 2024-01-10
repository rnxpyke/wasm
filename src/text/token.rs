use std::{f64::NAN, str::FromStr};

#[derive(Debug, Clone)]
pub struct TextToken(Vec<u8>);

#[derive(Debug, Clone)]
pub enum Token {
    LeftParen,
    RightParen,
    Atom(String),
    Name(String),
    Text(TextToken),
    Nat(usize),
    Int(isize),
    Float(f64),
    Equal,
    Comment(String),
    Whitespace,
}

#[derive(Debug)]
pub enum Sign {
    Positive,
    Negative,
}

#[derive(Debug)]
pub enum TokenizeError {
    UnknownError,
    FailedExpectedToken,
    UnexpectedNextChar(char),
    UnexpectedEof,
}

#[derive(Copy, Clone)]
pub struct Lexer<'s> {
    input: &'s str,
}

type LexResult<T> = Result<T, TokenizeError>;
type LexFn<'s, T> = fn(lexer: &mut Lexer<'s>) -> LexResult<T>;

fn parse_longest<'s, T: core::fmt::Debug>(
    lexer: &mut Lexer<'s>,
    fns: &[LexFn<'s, T>],
) -> LexResult<T> {
    let conts = fns.into_iter().map(|f| {
        let mut l = lexer.clone();
        let res = f(&mut l);
        (l.input, res)
    });
    let (rest, val) = conts.min_by_key(|x| x.0.len()).unwrap();
    lexer.input = rest;
    return val;
}

impl<'s> Lexer<'s> {
    fn peek_next_char(&self) -> Option<char> {
        self.input.chars().next()
    }

    fn accept_next_char(&mut self) -> Option<char> {
        let mut chars = self.input.chars();
        let char = chars.next()?;
        self.input = chars.as_str();
        return Some(char);
    }

    fn accept_char(&mut self, c: char) -> bool {
        let Some(char) = self.peek_next_char() else { return false };
        if char == c {
            self.accept_next_char();
            return true;
        }
        false
    }

    fn accept_string(&mut self, s: &str) -> bool {
        match self.input.strip_prefix(s) {
            Some(rest) => {
                self.input = rest;
                true
            }
            None => false,
        }
    }

    fn expect_char(&mut self, c: char) -> LexResult<()> {
        let char = self.peek_next_char().ok_or(TokenizeError::UnexpectedEof)?;
        if char == c {
            self.accept_next_char();
            return Ok(());
        }
        return Err(TokenizeError::UnexpectedNextChar(char));
    }

    fn expect_string(&mut self, s: &str) -> LexResult<()> {
        if self.accept_string(s) {
            Ok(())
        } else {
            Err(TokenizeError::FailedExpectedToken)
        }
    }

    fn lparen(&mut self) -> LexResult<Token> {
        self.expect_char('(')?;
        Ok(Token::LeftParen)
    }

    fn rparen(&mut self) -> LexResult<Token> {
        self.expect_char(')')?;
        Ok(Token::RightParen)
    }

    fn string(&mut self) -> LexResult<Token> {
        self.expect_char('"')?;
        let mut text: Vec<u8> = vec![];
        loop {
            if self.accept_char('"') {
                break;
            }
            if self.accept_char('\\') {
                if self.accept_char('n') {
                    text.extend("\n".as_bytes());
                    continue;
                }
                if self.accept_char('t') {
                    text.extend("\t".as_bytes());
                    continue;
                }
                if self.accept_char('\\') {
                    text.extend("\\".as_bytes());
                    continue;
                }
                if self.accept_char('\'') {
                    text.extend("'".as_bytes());
                    continue;
                }
                if self.accept_char('"') {
                    text.extend("\"".as_bytes());
                    continue;
                }
                if self.accept_char('u') {
                    self.expect_char('{')?;
                    let num = self.hexnum()?;
                    self.expect_char('}')?;
                    todo!("no idea what to do with hexnum");
                    continue;
                }
                let Some(a) = self.accept_hexdigit() else { return Err(TokenizeError::FailedExpectedToken) };
                let Some(b) = self.accept_hexdigit() else { return Err(TokenizeError::FailedExpectedToken) };
                text.push(a as u8 * 16 + b as u8);
            } else {
                if let Some(c) = self.accept_next_char() {
                    text.extend(c.encode_utf8(&mut [0, 0, 0, 0]).as_bytes());
                } else {
                    return Err(TokenizeError::UnexpectedEof);
                }
            }
        }
        Ok(Token::Text(TextToken(text)))
    }

    fn whitespace(&mut self) -> LexResult<Token> {
        let char = self.peek_next_char().ok_or(TokenizeError::UnexpectedEof)?;
        if !char.is_whitespace() {
            return Err(TokenizeError::UnexpectedNextChar(char));
        }

        self.accept_next_char();
        self.input = self.input.trim_start();
        return Ok(Token::Whitespace);
    }

    fn accept_name_char(&mut self) -> Option<char> {
        let Some(c) = self.peek_next_char() else { return None };
        if !c.is_ascii() {
            return None;
        }
        if !c.is_ascii_graphic() {
            return None;
        }
        match c {
            ' ' => return None,
            '\'' => return None,
            '"' => return None,
            ',' => return None,
            ';' => return None,
            '(' => return None,
            ')' => return None,
            '[' => return None,
            ']' => return None,
            '{' => return None,
            '}' => return None,
            _ => return self.accept_next_char(),
        };
    }

    fn name(&mut self) -> Result<Token, TokenizeError> {
        self.expect_char('$')?;
        let mut name = String::new();
        while let Some(char) = self.accept_name_char() {
            name.push(char);
        }
        Ok(Token::Name(name))
    }

    fn linecomment(&mut self) -> Result<Token, TokenizeError> {
        self.expect_string(";;")?;
        match self.input.split_once('\n') {
            Some((comment, rest)) => {
                self.input = rest;
                return Ok(Token::Comment(comment.into()));
            }
            None => {
                let comment = self.input;
                self.input = "";
                return Ok(Token::Comment(comment.into()));
            }
        }
    }

    fn accept_atom_char(&mut self) -> Option<char> {
        let char = self.peek_next_char()?;
        if char.is_ascii_alphanumeric() {
            return self.accept_next_char();
        }
        match char {
            '_' => self.accept_next_char(),
            '.' => self.accept_next_char(),
            ':' => self.accept_next_char(),
            _ => None,
        }
    }

    fn accept_digit(&mut self) -> Option<u32> {
        let char = self.peek_next_char()?;
        match char.to_digit(10) {
            Some(d) => {
                self.accept_next_char().unwrap();
                Some(d)
            }
            None => None,
        }
    }

    fn accept_hexdigit(&mut self) -> Option<u32> {
        let char = self.peek_next_char()?;
        match char.to_digit(16) {
            Some(d) => {
                self.accept_next_char().unwrap();
                Some(d)
            }
            None => None,
        }
    }

    fn num(&mut self) -> LexResult<usize> {
        let mut num: usize = 0;
        num += self
            .accept_digit()
            .ok_or(TokenizeError::FailedExpectedToken)? as usize;
        loop {
            self.accept_char('_');
            let Some(digit) = self.accept_digit() else { break };
            num = num.wrapping_mul(10);
            num = num.wrapping_add(digit as usize);
        }
        Ok(num)
    }

    fn hexnum(&mut self) -> LexResult<usize> {
        let mut num: usize = 0;
        num += self
            .accept_hexdigit()
            .ok_or(TokenizeError::FailedExpectedToken)? as usize;
        loop {
            self.accept_char('_');
            let Some(digit) = self.accept_digit() else { break };
            num = num.wrapping_mul(16);
            num = num.wrapping_add(digit as usize);
        }
        Ok(num)
    }

    fn expect_nat(&mut self) -> LexResult<usize> {
        if self.accept_string("0x") {
            self.hexnum()
        } else {
            self.num()
        }
    }

    fn nat(&mut self) -> LexResult<Token> {
        let num = self.expect_nat()?;
        Ok(Token::Nat(num))
    }

    fn accept_sign(&mut self) -> Option<Sign> {
        if self.accept_char('+') {
            return Some(Sign::Positive);
        }
        if self.accept_char('-') {
            return Some(Sign::Negative);
        }
        return None;
    }

    fn sign(&mut self) -> LexResult<Sign> {
        return self.accept_sign().ok_or(TokenizeError::FailedExpectedToken);
    }

    fn int(&mut self) -> LexResult<Token> {
        let sign = self.sign()?;
        let num = self.expect_nat()?;
        match sign {
            Sign::Positive => Ok(Token::Int(num as isize)),
            Sign::Negative => Ok(Token::Int((num as isize).overflowing_neg().0)),
        }
    }

    fn float(&mut self) -> LexResult<Token> {
        // TODO: exponents
        let sign = self.accept_sign();
        if self.accept_string("0x") {
            let dec = self.hexnum()?;
            self.expect_char('.')?;
            let frac = self.hexnum().ok();
            let floatstr = format!("{}.{}", dec, frac.unwrap_or(0));
            let float = f64::from_str(&floatstr).unwrap();
            Ok(Token::Float(float))
        } else {
            let dec = self.num()?;
            self.expect_char('.')?;
            let frac = self.num().ok();
            let floatstr = format!("{}.{}", dec, frac.unwrap_or(0));
            let float = f64::from_str(&floatstr).unwrap();
            Ok(Token::Float(float))
        }
    }

    fn float_inf(&mut self) -> LexResult<Token> {
        let sign = self.accept_sign().unwrap_or(Sign::Positive);
        self.expect_string("inf")?;
        match sign {
            Sign::Positive => Ok(Token::Float(f64::INFINITY)),
            Sign::Negative => Ok(Token::Float(f64::NEG_INFINITY)),
        }
    }

    fn float_nan(&mut self) -> LexResult<Token> {
        let sign = self.accept_sign();
        self.expect_string("nan")?;
        Ok(Token::Float(f64::NAN))
    }

    fn float_nan_hex(&mut self) -> LexResult<Token> {
        let sign = self.accept_sign();
        self.expect_string("nan:0x")?;
        let num = self.hexnum()?;
        // TODO: change nan pattern
        Ok(Token::Float(f64::NAN))
    }

    fn atom(&mut self) -> LexResult<Token> {
        let mut atom = String::new();
        let Some(char) = self.peek_next_char() else  { return Err(TokenizeError::UnexpectedEof) };
        if !char.is_ascii_alphabetic() {
            return Err(TokenizeError::FailedExpectedToken);
        }
        atom.push(self.accept_next_char().unwrap());
        loop {
            let Some(char) = self.accept_atom_char() else { break; };
            atom.push(char);
        }
        Ok(Token::Atom(atom))
    }

    fn equal(&mut self) -> LexResult<Token> {
        self.expect_char('=')?;
        Ok(Token::Equal)
    }

    fn blockcomment(&mut self) -> LexResult<Token> {
        // TODO: not to spec
        self.expect_string("(;")?;
        let mut comment = String::new();
        let mut depth = 1;
        loop {
            if self.accept_string(";)") {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                comment.push_str(";)");
                continue;
            }
            if self.accept_string("(;") {
                depth += 1;
                comment.push_str("(;");
                continue;
            }
            let Some(char) = self.accept_next_char() else { return Err(TokenizeError::UnexpectedEof) };
            comment.push(char);
        }
        Ok(Token::Comment(comment.to_string()))
    }

    fn token(&mut self) -> LexResult<Option<Token>> {
        if self.input.len() == 0 {
            return Ok(None);
        }

        if self.input.starts_with("(;") {
            return self.blockcomment().map(Some);
        }
        let res = match self.peek_next_char().unwrap() {
            '(' => self.lparen(),
            ')' => self.rparen(),
            '=' => self.equal(),
            '$' => self.name(),
            ';' => self.linecomment(),
            '"' => self.string(),

            c if c.is_whitespace() => self.whitespace(),
            c if c.is_ascii_alphabetic() => self.atom(),
            _ => parse_longest(
                self,
                &[
                    Lexer::nat,
                    Lexer::int,
                    Lexer::float,
                    Lexer::float_inf,
                    Lexer::float_nan,
                    Lexer::float_nan_hex,
                ],
            ),
        };

        //println!("res: {:?}, {:?}", &res, self.input.chars().take(25).collect::<String>());

        return res.map(Some);
    }
}

pub fn tokenize_script(input: &str) -> Result<Vec<Token>, TokenizeError> {
    let mut tokens = vec![];
    let mut tokenizer = Lexer { input };
    loop {
        let Some(token) = tokenizer.token()? else { return Ok(tokens) };
        tokens.push(token);
    }
}

pub fn tokenize_script_without_ws(input: &str) -> Result<Vec<Token>, TokenizeError> {
    let mut tokens = vec![];
    let mut tokenizer = Lexer { input };
    loop {
        let Some(token) = tokenizer.token()? else { return Ok(tokens) };
        match token {
            Token::Comment(_) => continue,
            _ => {}
        };
        tokens.push(token);
    }
}

#[cfg(test)]
mod tests {
    use crate::text::token::Token;

    use super::tokenize_script;

    #[test]
    fn tokenize_string() {
        let tokens = tokenize_script("\"abc\"").unwrap();
        assert!(tokens.len() == 1);
        let token = &tokens[0];
        assert!(matches!(token, Token::Text(_)));
    }
}
