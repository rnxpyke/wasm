use std::str::FromStr;

#[derive(Debug)]
pub enum Token {
    LeftParen,
    RightParen,
    Atom(String),
    Text(String),
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
    input: &'s str
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

    fn expectChar(&mut self, c: char) -> Result<(), TokenizeError> {
        if self.acceptChar(c) {
            Ok(())
        } else {
            Err(TokenizeError::FailedExpectedToken)
        }
    }

    fn acceptChar(&mut self, c: char) -> bool {
        if self.current().starts_with(c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn accept(&mut self, variants: &[char]) -> bool {
        for variant in variants {
            if self.acceptChar(*variant) {
                return true
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

    fn acceptDigit(&mut self) -> bool {
        const DIGITS: &'static [char] = &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
        return self.accept(DIGITS)
    }

    fn acceptLetter(&mut self) -> bool {
        let Some(c) = self.current().chars().nth(1) else { return false };
        if c.is_alphabetic() {
            self.pos += 1;
            return true;
        }
        return false;
    }

    fn acceptNameChar(&mut self) -> bool {
        const SPECIALS: &'static [char] = &['_', '.', '+', '-', '*', '/'];
        if self.accept(SPECIALS) {
            return true;
        }
        if self.acceptLetter() {
            return true;
        }
        if self.acceptDigit() {
            return true;
        }
        return false;
    }

    fn expectName(&mut self) -> Result<(), TokenizeError> {
        self.expectChar('$')?;
        loop {
            if !self.acceptNameChar() { break }
        }
        Ok(())
    }

    fn expectNumber(&mut self) -> Result<(), TokenizeError> {
        const DIGITS: &'static [char] = &['_', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
        self.accept(&['+', '-']);
        if self.acceptChar('0') {
            if self.acceptChar('x') {
                // parsing hex digint
            }
        }
        loop {
            let had_digit = self.accept(DIGITS);
            if !had_digit { break; }
        }
        if self.acceptChar('.') {
            //number is fractional
            loop {
                let had_digit = self.accept(DIGITS);
                if !had_digit { break; }
            }
        }
        if self.accept(&['e', 'E']) {
            // accept exponent
            loop {
                let had_digit = self.accept(DIGITS);
                if !had_digit { break; }
            }
        }
        if self.pos == 0 {
            return Err(TokenizeError::FailedExpectedToken);
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
        return Ok(Token::LeftParen)
    }

    fn try_right_paren(&mut self) -> Result<Token, TokenizeError> {
        self.expect(")")?;
        return Ok(Token::RightParen)
    }

    fn try_name(&mut self) -> Result<Token, TokenizeError> {
        let mut gostyle = GostyleTokenizer { input: self.input, pos: 0 };
        gostyle.expectName()?;
        let name = gostyle.emit();
        self.input = gostyle.input;
        return Ok(Token::Name(name.into()));
    }

    fn try_string(&mut self) -> Result<Token, TokenizeError> {
        self.expect("\"")?;
        let mut it = self.input.char_indices();
        let mut pos = 0;
        loop {
            let Some((idx, char)) = it.next() else { return Err(TokenizeError::FailedExpectedToken) };
            if char == '"' {
                pos = idx;
                break;
            }
        }
        let (escaped, rest) = self.input.split_at(pos);
        self.input = rest;
        self.input.starts_with('"').then_some(()).ok_or(TokenizeError::FailedExpectedToken)?;
        self.input = &self.input[1..];
       return Ok(Token::Text(FromStr::from_str(escaped).unwrap()))
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
                '_' => {},
                '.' => {},
                _ => break
            };
            pos = idx;
        }
        let (atom, rest) = self.input.split_at(pos+1);
        self.input = rest;
        return Ok(Token::Atom(atom.into()))
    }

    fn try_number(&mut self) -> Result<Token, TokenizeError> {
        let mut gostyle = GostyleTokenizer { input: self.input, pos: 0 };
        gostyle.expectNumber()?;
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
                },
                None => {
                    let comment = self.input;
                    self.input = "";
                    return Ok(Token::Comment(comment.into()));
                },
            }
        } else {
            // block comment
            match self.input.split_once(';') {
                Some((comment, rest)) => {
                    self.input = rest;
                    return Ok(Token::Comment(comment.into()))
                },
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
    let mut tokens =  vec![];
    let mut tokenizer = Tokenizer { input };
    loop {
        println!("input: {:?}", &tokenizer.input[0..tokenizer.input.len().min(10)]);
        let Some(token) = tokenizer.next_token()? else { return Ok(tokens) };
        println!("tok: {:?}", &token);
        tokens.push(token);
    }
}
