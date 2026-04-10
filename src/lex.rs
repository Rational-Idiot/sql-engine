use std::collections::HashMap;
use std::sync::OnceLock;

pub struct Lex {
    pub input: Vec<char>,
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
    Dot,

    And,
    Or,
    Not,
    True,
    False,

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
    Default,

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

static KEYWORDS: OnceLock<HashMap<&'static str, Token>> = OnceLock::new();

fn keyword_map() -> &'static HashMap<&'static str, Token> {
    KEYWORDS.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert("SELECT", Token::Select);
        m.insert("TABLE", Token::Table);
        m.insert("INSERT", Token::Insert);
        m.insert("INTO", Token::Into);
        m.insert("FROM", Token::From);
        m.insert("WHERE", Token::Where);

        m.insert("DELETE", Token::Delete);
        m.insert("DROP", Token::Drop);

        m.insert("UPDATE", Token::Update);
        m.insert("SET", Token::Set);
        m.insert("DISTINCT", Token::Distinct);
        m.insert("ALL", Token::All);

        m.insert("CREATE", Token::Create);
        m.insert("VALUES", Token::Values);
        m.insert("NULL", Token::Null);

        m.insert("AND", Token::And);
        m.insert("OR", Token::Or);
        m.insert("NOT", Token::Not);
        m.insert("TRUE", Token::True);
        m.insert("FALSE", Token::False);

        m.insert("AS", Token::As);
        m.insert("IS", Token::Is);
        m.insert("BETWEEN", Token::Between);
        m.insert("IN", Token::In);
        m.insert("LIKE", Token::Like);
        m.insert("ILIKE", Token::Ilike);
        m.insert("EXISTS", Token::Exists);
        m.insert("CAST", Token::Cast);
        m.insert("FILTER", Token::Filter);
        m.insert("IF", Token::If);

        m.insert("JOIN", Token::Join);
        m.insert("INNER", Token::Inner);
        m.insert("OUTER", Token::Outer);
        m.insert("LEFT", Token::Left);
        m.insert("RIGHT", Token::Right);
        m.insert("NATURAL", Token::Natural);
        m.insert("FULL", Token::Full);
        m.insert("CROSS", Token::Cross);
        m.insert("ON", Token::On);
        m.insert("USING", Token::Using);

        m.insert("ORDER", Token::Order);
        m.insert("BY", Token::By);
        m.insert("GROUP", Token::Group);
        m.insert("HAVING", Token::Having);
        m.insert("LIMIT", Token::Limit);
        m.insert("OFFSET", Token::Offset);
        m.insert("ASC", Token::Asc);
        m.insert("DESC", Token::Desc);
        m.insert("NULLS", Token::Nulls);
        m.insert("FIRST", Token::First);
        m.insert("LAST", Token::Last);

        m.insert("PRIMARY", Token::Primary);
        m.insert("KEY", Token::Key);
        m.insert("UNIQUE", Token::Unique);
        m.insert("DEFAULT", Token::Default);

        m.insert("INT", Token::Integer);
        m.insert("INTEGER", Token::Integer);
        m.insert("FLOAT", Token::Float);
        m.insert("REAL", Token::Float);
        m.insert("DOUBLE", Token::Float);
        m.insert("BOOL", Token::Bool);
        m.insert("BOOLEAN", Token::Bool);
        m.insert("TEXT", Token::Text);
        m.insert("STRING", Token::Text);
        m.insert("CHAR", Token::Text);
        m.insert("VARCHAR", Token::Text);

        m
    })
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

                '.' => {
                    self.advance();
                    return Ok(Token::Dot);
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
                    return Ok(Self::extract_keyword(s.as_str()));
                }

                _ => return Err(format!("Unexpected Character: {}", c)),
            }
        }
        Ok(Token::EOF)
    }

    pub fn extract_keyword(s: &str) -> Token {
        let upper = s.to_ascii_uppercase();

        keyword_map()
            .get(upper.as_str())
            .cloned()
            .unwrap_or(Token::Id(s.to_string()))
    }
}

impl Iterator for Lex {
    type Item = Result<Token, String>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_token())
    }
}

use std::fmt;

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Create => write!(f, "CREATE"),
            Token::Table => write!(f, "TABLE"),
            Token::Insert => write!(f, "INSERT"),
            Token::Into => write!(f, "INTO"),
            Token::Values => write!(f, "VALUES"),
            Token::Select => write!(f, "SELECT"),
            Token::From => write!(f, "FROM"),
            Token::Where => write!(f, "WHERE"),
            Token::Delete => write!(f, "DELETE"),
            Token::Drop => write!(f, "DROP"),
            Token::Update => write!(f, "UPDATE"),
            Token::Set => write!(f, "SET"),
            Token::Distinct => write!(f, "DISTINCT"),
            Token::All => write!(f, "ALL"),
            Token::Null => write!(f, "NULL"),

            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::Comma => write!(f, ","),
            Token::Semicolon => write!(f, ";"),
            Token::Star => write!(f, "*"),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Divide => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::Dot => write!(f, "."),

            Token::And => write!(f, "AND"),
            Token::Or => write!(f, "OR"),
            Token::Not => write!(f, "NOT"),

            Token::As => write!(f, "AS"),
            Token::Is => write!(f, "IS"),
            Token::Between => write!(f, "BETWEEN"),
            Token::In => write!(f, "IN"),

            Token::Like => write!(f, "LIKE"),
            Token::Ilike => write!(f, "ILIKE"),
            Token::Exists => write!(f, "EXISTS"),
            Token::Cast => write!(f, "CAST"),
            Token::Filter => write!(f, "FILTER"),
            Token::If => write!(f, "IF"),

            Token::Join => write!(f, "JOIN"),
            Token::Inner => write!(f, "INNER"),
            Token::Outer => write!(f, "OUTER"),
            Token::Left => write!(f, "LEFT"),
            Token::Right => write!(f, "RIGHT"),
            Token::Natural => write!(f, "NATURAL"),
            Token::Full => write!(f, "FULL"),
            Token::Cross => write!(f, "CROSS"),
            Token::On => write!(f, "ON"),
            Token::Using => write!(f, "USING"),

            Token::Order => write!(f, "ORDER"),
            Token::By => write!(f, "BY"),
            Token::Group => write!(f, "GROUP"),
            Token::Having => write!(f, "HAVING"),
            Token::Limit => write!(f, "LIMIT"),
            Token::Offset => write!(f, "OFFSET"),
            Token::Asc => write!(f, "ASC"),
            Token::Desc => write!(f, "DESC"),
            Token::Nulls => write!(f, "NULLS"),
            Token::First => write!(f, "FIRST"),
            Token::Last => write!(f, "LAST"),

            Token::Primary => write!(f, "PRIMARY"),
            Token::Key => write!(f, "KEY"),
            Token::Unique => write!(f, "UNIQUE"),
            Token::Default => write!(f, "DEFAULT"),

            Token::Integer => write!(f, "INTEGER"),
            Token::Float => write!(f, "FLOAT"),
            Token::Bool => write!(f, "BOOLEAN"),
            Token::Text => write!(f, "TEXT"),

            Token::Equal => write!(f, "="),
            Token::NotEqual => write!(f, "<>"),
            Token::Less => write!(f, "<"),
            Token::LessEqual => write!(f, "<="),
            Token::Greater => write!(f, ">"),
            Token::GreaterEqual => write!(f, ">="),

            Token::Id(s) => write!(f, "{}", s),
            Token::Number(n) => write!(f, "{}", n),
            Token::String(s) => write!(f, "'{}'", s),

            Token::EOF => write!(f, "EOF"),
        }
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
