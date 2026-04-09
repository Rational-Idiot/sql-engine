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

    Identifier(String),
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
}
