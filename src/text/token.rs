#[derive(Debug, Clone)]
pub struct TextToken(Vec<u8>);

#[derive(Debug, Clone)]
pub enum Token {
    LeftParen,
    RightParen,
    Atom(String),
    Text(TextToken),
    Number(String),
    Name(String),
    Comment(String),
}

#[derive(Debug)]
pub enum TokenizeError {
    UnknownError,
    FailedExpectedToken,
    UnexpectedNextChar(char),
}

pub struct Tokenizer<'s> {
    input: &'s str,
}

pub struct GostyleTokenizer<'s> {
    input: &'s str,
    pos: usize,
}

impl<'s> GostyleTokenizer<'s> {
    fn emit(&mut self) -> &'s str {
        let (tok, rest) = self.input.split_at(self.pos);
        self.input = rest;
        self.pos = 0;
        return tok;
    }

    fn skip(&mut self) {
        let (_, rest) = self.input.split_at(self.pos);
        self.input = rest;
        self.pos = 0;
    }

    fn current(&mut self) -> &'s str {
        let (_, cur) = self.input.split_at(self.pos);
        return cur;
    }

    fn expect_char(&mut self, c: char) -> Result<(), TokenizeError> {
        if self.accept_char(c) {
            Ok(())
        } else {
            Err(TokenizeError::FailedExpectedToken)
        }
    }

    fn accept_char(&mut self, c: char) -> bool {
        if self.current().starts_with(c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn accept(&mut self, variants: &[char]) -> bool {
        for variant in variants {
            if self.accept_char(*variant) {
                return true;
            }
        }
        return false;
    }

    fn expect(&mut self, variants: &[char]) -> Result<(), TokenizeError> {
        if self.accept(variants) {
            Ok(())
        } else {
            Err(TokenizeError::FailedExpectedToken)
        }
    }

    fn accept_digit(&mut self) -> bool {
        const DIGITS: &'static [char] = &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
        return self.accept(DIGITS);
    }

    fn accept_letter(&mut self) -> bool {
        let Some(c) = self.current().chars().nth(1) else { return false };
        if c.is_alphabetic() {
            self.pos += 1;
            return true;
        }
        return false;
    }

    fn accept_name_char(&mut self) -> bool {
        let Some(c) = self.current().chars().next() else { return false };
        if !c.is_ascii() {
            return false;
        }
        if !c.is_ascii_graphic() {
            return false;
        }
        match c {
            ' ' => return false,
            '\'' => return false,
            '"' => return false,
            ',' => return false,
            ';' => return false,
            '(' => return false,
            ')' => return false,
            '[' => return false,
            ']' => return false,
            '{' => return false,
            '}' => return false,
            _ => {}
        };
        self.pos += c.len_utf8();
        return true;
    }

    fn expect_name(&mut self) -> Result<(), TokenizeError> {
        self.expect_char('$')?;
        loop {
            if !self.accept_name_char() {
                break;
            }
        }
        Ok(())
    }

    fn expect_number(&mut self) -> Result<(), TokenizeError> {
        const DIGITS: &'static [char] = &['_', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
        self.accept(&['+', '-']);
        if self.accept_char('0') {
            if self.accept_char('x') {
                // parsing hex digint
            }
        }
        loop {
            let had_digit = self.accept(DIGITS);
            if !had_digit {
                break;
            }
        }
        if self.accept_char('.') {
            //number is fractional
            loop {
                let had_digit = self.accept(DIGITS);
                if !had_digit {
                    break;
                }
            }
        }
        if self.accept(&['e', 'E']) {
            // accept exponent
            loop {
                let had_digit = self.accept(DIGITS);
                if !had_digit {
                    break;
                }
            }
        }
        if self.pos == 0 {
            return Err(TokenizeError::FailedExpectedToken);
        }
        Ok(())
    }

    fn accept_string(&mut self, s: &str) -> bool {
        if self.current().starts_with(s) {
            self.pos += s.len();
            return true;
        }
        return false;
    }

    fn accept_any_char(&mut self) -> bool {
        if let Some(c) = self.current().chars().next() {
            self.pos += c.len_utf8();
            return true;
        }
        return false;
    }

    fn expect_string(&mut self) -> Result<(), TokenizeError> {
        self.expect_char('"')?;
        loop {
            if self.accept_char('"') {
                break;
            }
            if self.accept_string("\\n") {
                continue;
            }
            if self.accept_string("\\t") {
                continue;
            }
            if self.accept_string("\\\"") {
                continue;
            }
            if self.accept_string("\'") {
                continue;
            }
            if self.accept_string("\\\\") {
                continue;
            }
            if !self.accept_any_char() {
                return Err(TokenizeError::FailedExpectedToken);
            }
        }
        Ok(())
    }
}

impl Tokenizer<'_> {
    fn skip_whitespace(&mut self) {
        self.input = self.input.trim_start();
    }

    fn expect(&mut self, pat: &'static str) -> Result<(), TokenizeError> {
        match self.input.strip_prefix(pat) {
            Some(s) => self.input = s,
            None => return Err(TokenizeError::FailedExpectedToken),
        }
        Ok(())
    }

    fn try_left_paren(&mut self) -> Result<Token, TokenizeError> {
        self.expect("(")?;
        return Ok(Token::LeftParen);
    }

    fn try_right_paren(&mut self) -> Result<Token, TokenizeError> {
        self.expect(")")?;
        return Ok(Token::RightParen);
    }

    fn try_name(&mut self) -> Result<Token, TokenizeError> {
        let mut gostyle = GostyleTokenizer {
            input: self.input,
            pos: 0,
        };
        gostyle.expect_name()?;
        let name = gostyle.emit();
        self.input = gostyle.input;
        return Ok(Token::Name(name.into()));
    }

    fn try_string(&mut self) -> Result<Token, TokenizeError> {
        let mut gostyle = GostyleTokenizer {
            input: self.input,
            pos: 0,
        };
        gostyle.expect_string()?;
        let text = gostyle.emit();
        self.input = gostyle.input;
        return Ok(Token::Text(TextToken(text.as_bytes().into())));
    }

    fn try_atom(&mut self) -> Result<Token, TokenizeError> {
        let mut pos = 0;
        let mut it = self.input.char_indices();
        let Some((_, char)) = it.next() else { return Err(TokenizeError::FailedExpectedToken) };
        if !char.is_ascii_alphabetic() {
            return Err(TokenizeError::FailedExpectedToken);
        }
        loop {
            let Some((idx, char)) = it.next() else { break };
            if char.is_ascii_alphanumeric() {
                pos = idx;
                continue;
            }
            match char {
                '_' => {}
                '.' => {}
                '=' => {
                    pos = idx;
                    break;
                }
                _ => break,
            };
            pos = idx;
        }
        let (atom, rest) = self.input.split_at(pos + 1);
        self.input = rest;
        return Ok(Token::Atom(atom.into()));
    }

    fn try_number(&mut self) -> Result<Token, TokenizeError> {
        let mut gostyle = GostyleTokenizer {
            input: self.input,
            pos: 0,
        };
        gostyle.expect_number()?;
        let num = gostyle.emit();
        self.input = gostyle.input;
        return Ok(Token::Number(num.into()));
    }

    fn try_comment(&mut self) -> Result<Token, TokenizeError> {
        self.expect(";")?;
        if self.input.starts_with(';') {
            self.input = &self.input[1..];
            // line comment
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
        } else {
            // block comment
            match self.input.split_once(';') {
                Some((comment, rest)) => {
                    self.input = rest;
                    return Ok(Token::Comment(comment.into()));
                }
                None => return Err(TokenizeError::FailedExpectedToken),
            }
        }
    }

    fn next_token(&mut self) -> Result<Option<Token>, TokenizeError> {
        self.skip_whitespace();
        let Some(next_char) = self.input.chars().nth(0) else { return Ok(None) };
        match next_char {
            '(' => self.try_left_paren().map(Some),
            ')' => self.try_right_paren().map(Some),
            '$' => self.try_name().map(Some),
            '"' => self.try_string().map(Some),
            ';' => self.try_comment().map(Some),
            '+' => self.try_number().map(Some),
            '-' => self.try_number().map(Some),
            a if a.is_alphabetic() => self.try_atom().map(Some),
            d if d.is_ascii_digit() => self.try_number().map(Some),
            _ => Err(TokenizeError::UnexpectedNextChar(next_char)),
        }
    }
}

pub fn tokenize_script(input: &str) -> Result<Vec<Token>, TokenizeError> {
    let mut tokens = vec![];
    let mut tokenizer = Tokenizer { input };
    loop {
        let Some(token) = tokenizer.next_token()? else { return Ok(tokens) };
        tokens.push(token);
    }
}

pub fn tokenize_script_without_ws(input: &str) -> Result<Vec<Token>, TokenizeError> {
    let mut tokens = vec![];
    let mut tokenizer = Tokenizer { input };
    loop {
        let Some(token) = tokenizer.next_token()? else { return Ok(tokens) };
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
