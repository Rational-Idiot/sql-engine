use crate::lex::Token;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}
