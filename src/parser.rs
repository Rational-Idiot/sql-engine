use std::fmt;

use crate::{
    ast::{SelectStmt, SetQuantifier, Stmt},
    lex::Token,
};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

#[derive(Debug)]
pub enum ParseError {
    UnexpectedToken { got: Token, expected: &'static str },
    UnexpectedEOF,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedToken { got, expected } => {
                write!(f, "Expected: {expected}, Got: {got}")
            }
            Self::UnexpectedEOF => write!(f, "Unexpected End of Input"),
        }
    }
}

impl std::error::Error for ParseError {}
pub type Result<T> = std::result::Result<T, ParseError>;

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Parser {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::EOF)
    }

    fn peek_ahead(&self, offset: usize) -> &Token {
        self.tokens.get(self.pos + offset).unwrap_or(&Token::EOF)
    }

    fn advance(&mut self) -> Token {
        let t = self.tokens.get(self.pos).cloned().unwrap_or(Token::EOF);
        if self.pos < self.tokens.len() {
            self.pos += 1
        };
        t
    }

    fn eat(&mut self, token: &Token) -> bool {
        if self.peek() == token {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, token: &Token) -> Result<()> {
        if self.peek() == token {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken {
                got: self.peek().clone(),
                expected: token_desc(token),
            })
        }
    }

    pub fn parse(&mut self) -> Result<Stmt> {
        let s = self.parse_stmt()?;
        self.eat(&Token::Semicolon);
        Ok(s)
    }

    fn parse_stmt(&mut self) -> Result<Stmt> {
        match self.peek() {
            Token::Select => Ok(Stmt::Select(self.parse_select()?)),

            got => Err(ParseError::UnexpectedToken {
                got: got.clone(),
                expected: "SELECT",
            }),
        }
    }

    fn parse_select(&mut self) -> Result<SelectStmt> {
        self.expect(&Token::Select)?;

        let q = if self.eat(&Token::Distinct) {
            SetQuantifier::Distinct
        } else {
            SetQuantifier::All
        };

        todo!()
    }
}

fn token_desc(t: &Token) -> &'static str {
    match t {
        Token::Select => "SELECT",
        Token::From => "FROM",
        Token::Where => "WHERE",
        Token::Insert => "INSERT",
        Token::Into => "INTO",
        Token::Values => "VALUES",
        Token::Update => "UPDATE",
        Token::Set => "SET",
        Token::Delete => "DELETE",
        Token::Create => "CREATE",
        Token::Drop => "DROP",
        Token::Table => "TABLE",
        Token::Join => "JOIN",
        Token::On => "ON",
        Token::Using => "USING",
        Token::By => "BY",
        Token::And => "AND",
        Token::As => "AS",
        Token::Is => "IS",
        Token::Not => "NOT",
        Token::Null => "NULL",
        Token::Exists => "EXISTS",
        Token::Key => "KEY",
        Token::If => "IF",
        Token::LParen => "(",
        Token::RParen => ")",
        Token::Comma => ",",
        Token::Semicolon => ";",
        Token::Dot => ".",
        Token::Equal => "=",
        Token::EOF => "end of input",
        _ => "token",
    }
}
