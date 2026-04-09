pub struct Lex {
    input: Vec<char>,
    pos: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Create,
    Table,
    Insert,
    Into,
    Values,
    Select,
    From,
    Where,
    Delete,
    Drop,
    Update,
    Set,
    Distinct,
    All,
    Null,

    LParen,
    RParen,
    Comma,
    Semicolon,
    Equal,

    Id(String),
    Number(String),
    String(String),

    EOF,
}

impl Lex {
    pub fn new() -> Self {
        Self {
            input: Vec::new(),
            pos: 0,
        }
    }

    pub fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn eat_while<F: Fn(char) -> bool>(&mut self, cond: F) -> String {
        let mut res = String::new();

        while let Some(c) = self.peek() {
            if cond(c) {
                res.push(c);
                self.advance();
            } else {
                break;
            }
        }
        res
    }

    pub fn next_token(&mut self) -> Result<Token, String> {
        while let Some(c) = self.peek() {
            match c {
                ' ' | '\t' | '\n' => self.advance(),
                '(' => {
                    self.advance();
                    return Ok(Token::LParen);
                }
                ')' => {
                    self.advance();
                    return Ok(Token::RParen);
                }
                ',' => {
                    self.advance();
                    return Ok(Token::Comma);
                }
                '=' => {
                    self.advance();
                    return Ok(Token::Equal);
                }
                '\'' => {
                    self.advance();
                    let s = self.eat_while(|c| c != '\'');
                    self.advance();
                    return Ok(Token::String(s));
                }
                c if c.is_ascii_digit() => {
                    let num = self.eat_while(|c| c.is_ascii_digit());
                    return Ok(Token::Number(num));
                }
                c if c.is_ascii_alphabetic() => {
                    let s = self.eat_while(|c| c.is_alphanumeric() || c == '_');
                    return Ok(Self::extract_keyword(s));
                }

                _ => return Err(format!("Unexpected Character: {}", c)),
            }
        }
        Ok(Token::EOF)
    }

    pub fn extract_keyword(s: String) -> Token {
        match s.to_ascii_uppercase().as_str() {
            "SELECT" => Token::Select,
            "TABLE" => Token::Table,
            "INSERT" => Token::Insert,
            "INTO" => Token::Into,
            "FROM" => Token::From,
            "WHERE" => Token::Where,
            "DELETE" => Token::Delete,
            "DROP" => Token::Drop,
            "UPDATE" => Token::Update,
            "SET" => Token::Set,
            "DISTINCT" => Token::Distinct,
            "ALL" => Token::All,
            "CREATE" => Token::Create,
            "VALUES" => Token::Values,
            "NULL" => Token::Null,

            _ => Token::Id(s),
        }
    }
}
