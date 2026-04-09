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
    Star,
    Plus,
    Minus,
    Divide,
    Percent,

    And,
    Or,
    Not,

    As,
    Is,
    Between,
    In,

    Like,
    Ilike,
    Exists,
    Cast,
    Filter,
    If,

    Join,
    Inner,
    Outer,
    Left,
    Right,
    Natural,
    Full,
    Cross,
    On,
    Using,

    Order,
    By,
    Group,
    Having,
    Limit,
    Offset,
    Asc,
    Desc,
    Nulls,
    First,
    Last,

    Primary,
    Key,
    Unique,
    Defualt,

    Integer,
    Float,
    Bool,
    Text,

    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,

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
                ' ' | '\t' | '\n' | '\r' => self.advance(),
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
                ';' => {
                    self.advance();
                    return Ok(Token::Semicolon);
                }

                '*' => {
                    self.advance();
                    return Ok(Token::Star);
                }

                '+' => {
                    self.advance();
                    return Ok(Token::Plus);
                }
                '-' => {
                    self.advance();
                    return Ok(Token::Minus);
                }
                '/' => {
                    self.advance();
                    return Ok(Token::Divide);
                }
                '%' => {
                    self.advance();
                    return Ok(Token::Percent);
                }

                '=' => {
                    self.advance();
                    return Ok(Token::Equal);
                }

                '<' => {
                    self.advance();
                    match self.peek() {
                        Some('>') => {
                            self.advance();
                            return Ok(Token::NotEqual);
                        }
                        Some('=') => {
                            self.advance();
                            return Ok(Token::LessEqual);
                        }
                        _ => return Ok(Token::Less),
                    }
                }

                '>' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        return Ok(Token::GreaterEqual);
                    }
                    return Ok(Token::Greater);
                }

                '\'' => {
                    self.advance();
                    let s = self.eat_while(|c| c != '\'');
                    if self.peek() != Some('\'') {
                        return Err("Unterminated string literal".into());
                    }
                    self.advance();
                    return Ok(Token::String(s));
                }

                c if c.is_ascii_digit() => {
                    let num = self.eat_while(|c| c.is_ascii_digit());
                    return Ok(Token::Number(num));
                }

                c if c.is_ascii_alphabetic() || c == '_' => {
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

            "AND" => Token::And,
            "OR" => Token::Or,
            "NOT" => Token::Not,

            "AS" => Token::As,
            "IS" => Token::Is,
            "BETWEEN" => Token::Between,
            "IN" => Token::In,

            "LIKE" => Token::Like,
            "ILIKE" => Token::Ilike,
            "EXISTS" => Token::Exists,
            "CAST" => Token::Cast,
            "FILTER" => Token::Filter,
            "IF" => Token::If,

            "JOIN" => Token::Join,
            "INNER" => Token::Inner,
            "OUTER" => Token::Outer,
            "LEFT" => Token::Left,
            "RIGHT" => Token::Right,
            "NATURAL" => Token::Natural,
            "FULL" => Token::Full,
            "CROSS" => Token::Cross,
            "ON" => Token::On,
            "USING" => Token::Using,

            "ORDER" => Token::Order,
            "BY" => Token::By,
            "GROUP" => Token::Group,
            "HAVING" => Token::Having,
            "LIMIT" => Token::Limit,
            "OFFSET" => Token::Offset,
            "ASC" => Token::Asc,
            "DESC" => Token::Desc,
            "NULLS" => Token::Nulls,
            "FIRST" => Token::First,
            "LAST" => Token::Last,

            "PRIMARY" => Token::Primary,
            "KEY" => Token::Key,
            "UNIQUE" => Token::Unique,
            "DEFAULT" => Token::Defualt,

            "INT" | "INTEGER" => Token::Integer,
            "FLOAT" | "REAL" | "DOUBLE" => Token::Float,
            "BOOL" | "BOOLEAN" => Token::Bool,
            "TEXT" | "STRING" | "CHAR" | "VARCHAR" => Token::Text,

            _ => Token::Id(s.to_string()),
        }
    }
}

impl Iterator for Lex {
    type Item = Result<Token, String>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_token())
    }
}

#[cfg(test)]
mod tests {
    use crate::lex::{Lex, Token};

    #[test]
    fn tokenise_paren() {
        let mut lexer = Lex::new();
        lexer.input = vec!['(', ')'];
        let mut res: Vec<Token> = Vec::new();

        while let Ok(t) = lexer.next_token() {
            if t == Token::EOF {
                res.push(t);
                break;
            }
            res.push(t);
        }
        assert_eq!(res, vec![Token::LParen, Token::RParen, Token::EOF])
    }

    #[test]
    fn tokenise_create_table_query() {
        let mut lexer = Lex::new();
        lexer.input = "CREATE TABLE users (id, name);".chars().collect();
        let mut res: Vec<Token> = Vec::new();

        for tok in lexer {
            if let Ok(t) = tok {
                if t == Token::EOF {
                    res.push(t);
                    break;
                }
                res.push(t);
            }
        }

        assert_eq!(
            res,
            vec![
                Token::Create,
                Token::Table,
                Token::Id("users".to_string()),
                Token::LParen,
                Token::Id("id".to_string()),
                Token::Comma,
                Token::Id("name".to_string()),
                Token::RParen,
                Token::Semicolon,
                Token::EOF,
            ]
        );
    }
}
