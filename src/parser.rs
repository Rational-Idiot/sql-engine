use std::fmt;

use crate::{
    ast::{Expr, Ident, Order, SelectItem, SelectStmt, SetQuantifier, SortType, Stmt, TableRef},
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
    InvalidInteger(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedToken { got, expected } => {
                write!(f, "Expected: {expected}, Got: {got}")
            }
            Self::UnexpectedEOF => write!(f, "Unexpected End of Input"),
            Self::InvalidInteger(s) => write!(f, "Cannot parse {s:?} as an Integer"),
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

    fn expect_ident(&mut self) -> Result<Ident> {
        match self.advance() {
            Token::Id(s) => Ok(Ident(s)),
            got => Err(ParseError::UnexpectedToken {
                got,
                expected: "Identifier",
            }),
        }
    }

    fn expect_num(&mut self) -> Result<u64> {
        match self.advance() {
            Token::Number(n) => n.parse().map_err(|_| ParseError::InvalidInteger(n)),
            got => Err(ParseError::UnexpectedToken {
                got,
                expected: "Integer Literal",
            }),
        }
    }

    fn comma_sep<T, F>(&mut self, mut f: F) -> Result<Vec<T>>
    where
        F: FnMut(&mut Self) -> Result<T>,
    {
        let first = f(self)?;
        let mut o = vec![first];
        while self.eat(&Token::Comma) {
            o.push(f(self)?);
        }
        Ok(o)
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

        let quantifier = if self.eat(&Token::Distinct) {
            SetQuantifier::Distinct
        } else {
            self.eat(&Token::All);
            SetQuantifier::All
        };

        let col = self.parse_sel_list()?;
        let from = if self.eat(&Token::From) {
            Some(self.parse_table()?)
        } else {
            None
        };

        let where_clause = if self.eat(&Token::Where) {
            Some(self.parse_expr(0)?)
        } else {
            None
        };

        let group_by = if self.eat(&Token::Group) {
            self.expect(&Token::By)?;
            self.comma_sep(|p| p.parse_expr(0))?
        } else {
            vec![]
        };

        let having = if self.eat(&Token::Having) {
            Some(self.parse_expr(0)?)
        } else {
            None
        };

        let order_by = if self.eat(&Token::Order) {
            self.expect(&Token::By)?;
            self.comma_sep(Self::parse_order)?
        } else {
            vec![]
        };

        let limit = if self.eat(&Token::Limit) {
            Some(self.expect_num()?)
        } else {
            None
        };

        let offset = if self.eat(&Token::Offset) {
            Some(self.expect_num()?)
        } else {
            None
        };

        Ok(SelectStmt {
            col,
            quantifier,
            from,
            where_clause,
            group_by,
            having,
            order_by,
            limit,
            offset,
        })
    }

    fn parse_order(&mut self) -> Result<Order> {
        let expr = self.parse_expr(0)?;
        let dir = if self.eat(&Token::Desc) {
            SortType::Desc
        } else {
            self.eat(&Token::Asc);
            SortType::Asc
        };

        Ok(Order { expr, dir })
    }

    fn parse_table(&mut self) -> Result<TableRef> {
        let name = self.expect_ident()?;
        let alias = self.parse_alias()?;
        Ok(TableRef::Named { name, alias })
    }

    fn parse_sel_list(&mut self) -> Result<Vec<SelectItem>> {
        self.comma_sep(|p| {
            if p.peek() == &Token::Star {
                p.advance();
                return Ok(SelectItem {
                    expr: Expr::Glob,
                    alias: None,
                });
            }

            let expr = p.parse_expr(0)?;
            let alias = p.parse_alias()?;
            Ok(SelectItem { expr, alias })
        })
    }

    fn parse_alias(&mut self) -> Result<Option<Ident>> {
        if self.eat(&Token::As) {
            return Ok(Some(self.expect_ident()?));
        }

        if let Token::Id(_) = self.peek() {
            return Ok(Some(self.expect_ident()?));
        }

        Ok(None)
    }

    fn parse_expr(&mut self, min_bp: u8) -> Result<Expr> {
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

#[cfg(test)]
mod tests {
    use crate::{
        ast::{Expr, Ident, SelectItem, SelectStmt, SetQuantifier, Stmt, TableRef},
        lex::{Lex, Token},
        parser::Parser,
    };

    #[test]
    fn parse_select_statement() {
        let mut lexer = Lex::new();
        lexer.input = "SELECT * FROM gay".chars().collect();

        let tokens: Vec<Token> = lexer
            .map(|t| t.unwrap())
            .take_while(|t| *t != Token::EOF)
            .chain(std::iter::once(Token::EOF))
            .collect();

        let mut parser = Parser::new(tokens);
        let stmt = parser.parse().unwrap();

        assert_eq!(
            stmt,
            Stmt::Select(SelectStmt {
                col: vec![SelectItem {
                    expr: Expr::Glob,
                    alias: None,
                }],
                quantifier: SetQuantifier::All,
                from: Some(TableRef::Named {
                    name: Ident("gay".to_string()),
                    alias: None,
                }),
                where_clause: None,
                group_by: vec![],
                having: None,
                order_by: vec![],
                limit: None,
                offset: None,
            })
        );
    }
}
