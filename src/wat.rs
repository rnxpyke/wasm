use std::str::FromStr;

#[derive(Debug)]
pub enum Token {
    LeftParen,
    RightParen,
    Atom(String),
    Text(String),
    Number(isize),
    Name(String),
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

impl Tokenizer<'_> {
    fn skip_whitespace(&mut self) {
        self.input = self.input.trim_start();
    }

    fn try_left_paren(&mut self) -> Result<Token, TokenizeError> {
        self.input.starts_with('(').then_some(()).ok_or(TokenizeError::FailedExpectedToken)?;
        self.input = &self.input[1..];
        return Ok(Token::LeftParen)
    }

    fn try_right_paren(&mut self) -> Result<Token, TokenizeError> {
        self.input.starts_with(')').then_some(()).ok_or(TokenizeError::FailedExpectedToken)?;
        self.input = &self.input[1..];
        return Ok(Token::RightParen)
    }

    fn try_name(&mut self) -> Result<Token, TokenizeError> {
        self.input.starts_with('$').then_some(()).ok_or(TokenizeError::FailedExpectedToken)?;
        self.input = &self.input[1..];
        let Ok(Token::Atom(s)) = self.try_atom() else { return Err(TokenizeError::FailedExpectedToken) };
        return Ok(Token::Name(s));
    }

    fn try_string(&mut self) -> Result<Token, TokenizeError> {
        self.input.starts_with('"').then_some(()).ok_or(TokenizeError::FailedExpectedToken)?;
        self.input = &self.input[1..];
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
        let mut pos = 0;
        let mut num: isize = 0;
        let mut it = self.input.char_indices();

        loop {
            let Some((idx, char)) = it.next() else { break };
            let Some(digit) = char.to_digit(10) else { break };
            num *= 10;
            num += digit as isize;
            pos = idx;
        }
        let (_, rest) = self.input.split_at(pos+1);
        self.input = rest;
        return Ok(Token::Number(num))
    }

    fn next_token(&mut self) -> Result<Option<Token>, TokenizeError> {
        self.skip_whitespace();
        let Some(next_char) = self.input.chars().nth(0) else { return Ok(None) };
        match next_char {
            '(' => self.try_left_paren().map(Some),
            ')' => self.try_right_paren().map(Some),
            '$' => self.try_name().map(Some),
            '"' => self.try_string().map(Some),
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
        let Some(token) = tokenizer.next_token()? else { return Ok(tokens) };
        tokens.push(token);
    }
}
