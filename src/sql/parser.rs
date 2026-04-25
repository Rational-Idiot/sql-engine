use std::fmt;

use crate::sql::{ast::*, lex::Token};

#[derive(Debug)]
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
            Token::Insert => Ok(Stmt::Insert(self.parse_insert()?)),
            Token::Update => Ok(Stmt::Update(self.parse_update()?)),
            Token::Delete => Ok(Stmt::Delete(self.parse_delete()?)),
            Token::Create => Ok(Stmt::Create(self.parse_create()?)),
            Token::Drop => Ok(Stmt::Drop(self.parse_drop()?)),

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

        let joins = if from.is_some() {
            self.parse_joins()?
        } else {
            vec![]
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
            joins,
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
        if self.eat(&Token::LParen) {
            let query = self.parse_select()?;
            self.expect(&Token::RParen)?;
            self.eat(&Token::As);
            let alias = self.expect_ident()?;
            return Ok(TableRef::Subquery {
                query: Box::new(query),
                alias,
            });
        }

        let name = self.expect_ident()?;
        let alias = self.parse_alias()?;
        Ok(TableRef::Named { name, alias })
    }

    fn parse_joins(&mut self) -> Result<Vec<JoinClause>> {
        let mut joins = vec![];

        loop {
            let kind = match self.peek() {
                Token::Join => {
                    self.advance();
                    JoinKind::Inner
                }

                Token::Inner => {
                    self.advance();
                    self.expect(&Token::Join)?;
                    JoinKind::Inner
                }

                Token::Left => {
                    self.advance();
                    self.eat(&Token::Outer);
                    self.expect(&Token::Join)?;
                    JoinKind::Left
                }

                Token::Right => {
                    self.advance();
                    self.eat(&Token::Outer);
                    self.expect(&Token::Join)?;
                    JoinKind::Right
                }

                Token::Full => {
                    self.advance();
                    self.eat(&Token::Outer);
                    self.expect(&Token::Join)?;
                    JoinKind::Outer
                }

                Token::Cross => {
                    self.advance();
                    self.expect(&Token::Join)?;
                    let table = self.parse_table()?;

                    joins.push(JoinClause {
                        kind: JoinKind::Cross,
                        table,
                        constraint: JoinConstraint::Natural,
                    });

                    continue;
                }

                Token::Natural => {
                    self.advance();
                    self.expect(&Token::Join)?;
                    let table = self.parse_table()?;

                    joins.push(JoinClause {
                        kind: JoinKind::Inner,
                        table,
                        constraint: JoinConstraint::Natural,
                    });

                    continue;
                }

                _ => break,
            };

            let table = self.parse_table()?;
            let constraint = match self.peek() {
                Token::On => {
                    self.advance();
                    JoinConstraint::On(self.parse_expr(0)?)
                }

                Token::Using => {
                    self.advance();
                    self.expect(&Token::LParen)?;
                    let cols = self.comma_sep(|p| p.expect_ident())?;
                    self.expect(&Token::RParen)?;
                    JoinConstraint::Using(cols)
                }

                got => {
                    return Err(ParseError::UnexpectedToken {
                        got: got.clone(),
                        expected: "ON | USING",
                    });
                }
            };

            joins.push(JoinClause {
                kind,
                table,
                constraint,
            });
        }
        Ok(joins)
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

    fn parse_insert(&mut self) -> Result<InsertStmt> {
        self.expect(&Token::Insert)?;
        self.expect(&Token::Into)?;
        let table = self.expect_ident()?;

        let columns = if self.eat(&Token::LParen) {
            let cols = self.comma_sep(|p| p.expect_ident())?;
            self.expect(&Token::RParen)?;
            cols
        } else {
            vec![]
        };

        let source = match self.peek() {
            Token::Values => {
                self.eat(&Token::Values);
                let rows = self.comma_sep(|p| {
                    p.expect(&Token::LParen)?;

                    let row = p.comma_sep(|p2| p2.parse_expr(0))?;

                    p.expect(&Token::RParen)?;
                    Ok(row)
                })?;
                InsertSource::Values(rows)
            }

            Token::Select => {
                let select = self.parse_select()?;
                InsertSource::Select(Box::new(select))
            }

            t => {
                return Err(ParseError::UnexpectedToken {
                    got: t.clone(),
                    expected: "VALUES or SELECT",
                });
            }
        };

        Ok(InsertStmt {
            table,
            columns,
            source,
        })
    }

    fn parse_update(&mut self) -> Result<UpdateStmt> {
        self.expect(&Token::Update)?;
        let table = self.parse_table()?;
        self.expect(&Token::Set)?;

        let assign = self.comma_sep(|p| {
            let column = p.expect_ident()?;
            p.expect(&Token::Equal)?;
            let value = p.parse_expr(0)?;

            Ok(Assignment { column, value })
        })?;

        let where_clause = if self.eat(&Token::Where) {
            Some(self.parse_expr(0)?)
        } else {
            None
        };

        Ok(UpdateStmt {
            table,
            assign,
            where_clause,
        })
    }

    fn parse_delete(&mut self) -> Result<DeleteStmt> {
        self.expect(&Token::Delete)?;
        self.expect(&Token::From)?;

        let table = self.parse_table()?;

        let where_clause = if self.eat(&Token::Where) {
            Some(self.parse_expr(0)?)
        } else {
            None
        };

        Ok(DeleteStmt {
            table,
            where_clause,
        })
    }

    fn parse_create(&mut self) -> Result<CreateStmt> {
        self.expect(&Token::Create)?;

        match self.peek() {
            Token::Table => Ok(CreateStmt::Table(self.parse_create_table()?)),
            //TODO: Add views and stuff
            got => Err(ParseError::UnexpectedToken {
                got: got.clone(),
                expected: "TABLE",
            }),
        }
    }

    fn parse_create_table(&mut self) -> Result<CreateTableStmt> {
        self.expect(&Token::Table)?;

        let flag = if self.eat(&Token::If) {
            self.expect(&Token::Not)?;
            self.expect(&Token::Exists)?;
            true
        } else {
            false
        };

        let name = self.expect_ident()?;
        self.expect(&Token::LParen)?;
        let columns = self.comma_sep(Self::parse_column_def)?;
        self.expect(&Token::RParen)?;

        Ok(CreateTableStmt {
            name,
            columns,
            flag,
        })
    }

    fn parse_column_def(&mut self) -> Result<ColumnDef> {
        let name = self.expect_ident()?;
        let data_type = self.parse_data_type()?;

        let mut constraints = vec![];
        loop {
            match self.peek() {
                Token::Primary => {
                    self.advance();
                    self.expect(&Token::Key)?;
                    constraints.push(ColumnConstraint::PrimaryKey);
                }

                Token::Not => {
                    self.advance();
                    self.expect(&Token::Null)?;
                    constraints.push(ColumnConstraint::NotNull);
                }

                Token::Unique => {
                    self.advance();
                    constraints.push(ColumnConstraint::Unique);
                }
                Token::Default => {
                    self.advance();
                    constraints.push(ColumnConstraint::Default(self.parse_expr(0)?));
                }

                _ => break,
            }
        }

        Ok(ColumnDef {
            name,
            data_type,
            constraints,
        })
    }

    fn parse_drop(&mut self) -> Result<DropStmt> {
        self.expect(&Token::Drop)?;
        match self.advance() {
            Token::Table => {
                let f = if self.eat(&Token::If) {
                    self.expect(&Token::Exists)?;
                    true
                } else {
                    false
                };

                Ok(DropStmt::Table {
                    name: self.expect_ident()?,
                    if_exists: f,
                })
            }

            got => Err(ParseError::UnexpectedToken {
                got,
                expected: "TABLE",
            }),
        }
    }

    // A pratt parser inspired by core dumped https://www.youtube.com/watch?v=0c8b7YfsBKs&t=658s
    // and this article for bp reference https://www.youtube.com/redirect?event=video_description&redir_token=QUFFLUhqbE5UajJUTjBDdzFtMHlFLURweGJLTk90eXJ1UXxBQ3Jtc0trVXl2aTE5MU1BR0M3aWtQaG5hTjJmRnVFdUhJQy1veDZYb3ViSVdqVEZVdWNwYW5VRHdqV0RwNGs0dXRTUUtTODl5Y3lfVHctZFltbllhOWJiTE85QUE2Um9wWWNMdzFlRWdfRG5nTU1TbjBqY2FpWQ&q=https%3A%2F%2Fmatklad.github.io%2F2020%2F04%2F13%2Fsimple-but-powerful-pratt-parsing.html&v=0c8b7YfsBKs
    fn parse_expr(&mut self, min_bp: u8) -> Result<Expr> {
        let mut lhs = match self.peek() {
            Token::Not => {
                self.eat(&Token::Not);

                if self.peek() == &Token::Exists {
                    self.advance();

                    self.expect(&Token::LParen)?;
                    let q = self.parse_select()?;
                    self.expect(&Token::RParen)?;

                    Expr::Exists {
                        query: Box::new(q),
                        neg: true,
                    }
                } else {
                    Expr::UnaryOp {
                        op: UnaryOp::Not,
                        expr: Box::new(self.parse_expr(5)?),
                    }
                }
            }

            Token::Minus => {
                self.advance();
                Expr::UnaryOp {
                    op: UnaryOp::Neg,
                    expr: Box::new(self.parse_expr(13)?),
                }
            }

            Token::Exists => {
                self.advance();
                self.expect(&Token::LParen)?;
                let q = self.parse_select()?;
                self.expect(&Token::RParen)?;
                Expr::Exists {
                    query: Box::new(q),
                    neg: false,
                }
            }

            _ => self.parse_primary()?,
        };

        loop {
            // IS [NOT] NULL
            if self.peek() == &Token::Is {
                if 6 < min_bp {
                    break;
                }
                self.advance();

                let neg = self.eat(&Token::Not);
                self.expect(&Token::Null)?;
                lhs = Expr::IsNull {
                    expr: Box::new(lhs),
                    neg,
                };
                continue;
            }

            // [NOT] BETWEEN l and h
            let flag = self.peek() == &Token::Not && self.peek_ahead(1) == &Token::Between;
            if self.peek() == &Token::Between || flag {
                if min_bp > 6 {
                    break;
                }
                let neg = self.eat(&Token::Not);
                self.eat(&Token::Between);

                let l = self.parse_expr(7)?;
                self.expect(&Token::And)?;
                let h = self.parse_expr(7)?;

                lhs = Expr::Between {
                    expr: Box::new(lhs),
                    negated: neg,
                    low: Box::new(l),
                    high: Box::new(h),
                };
                continue;
            }

            // [NOT] IN (list | subquery)
            let flag = self.peek() == &Token::Not && self.peek_ahead(1) == &Token::In;
            if self.peek() == &Token::In || flag {
                if min_bp > 6 {
                    break;
                }
                let neg = self.eat(&Token::Not);
                self.eat(&Token::In);

                self.expect(&Token::LParen)?;
                lhs = if self.peek() == &Token::Select {
                    let q = self.parse_select()?;
                    self.expect(&Token::RParen)?;
                    Expr::InSubquery {
                        expr: Box::new(lhs),
                        query: Box::new(q),
                        neg,
                    }
                } else {
                    let list = self.comma_sep(|p| p.parse_expr(0))?;
                    self.expect(&Token::RParen)?;
                    Expr::InList {
                        expr: Box::new(lhs),
                        list,
                        neg,
                    }
                };
                continue;
            }

            // [NOT] LIKE/ILIKE
            let flag = self.peek() == &Token::Not
                && matches!(self.peek_ahead(1), Token::Like | Token::Ilike);
            if matches!(self.peek(), Token::Like | Token::Ilike) || flag {
                if min_bp > 6 {
                    break;
                }
                let neg = self.eat(&Token::Not);
                let insensitive = self.peek() == &Token::Ilike;
                self.advance(); // LIKE or ILIKE
                let pattern = self.parse_expr(7)?;
                lhs = Expr::Like {
                    expr: Box::new(lhs),
                    pattern: Box::new(pattern),
                    neg,
                    insensitive: insensitive,
                };
                continue;
            }

            // The binding powers
            let (op, l_bp, r_bp) = match self.peek() {
                Token::Or => (BinaryOp::Or, 1u8, 2u8),
                Token::And => (BinaryOp::And, 3, 4),
                Token::Equal => (BinaryOp::Eq, 6, 7),
                Token::NotEqual => (BinaryOp::Ne, 6, 7),
                Token::Less => (BinaryOp::Lt, 6, 7),
                Token::LessEqual => (BinaryOp::Le, 6, 7),
                Token::Greater => (BinaryOp::Gt, 6, 7),
                Token::GreaterEqual => (BinaryOp::Ge, 6, 7),
                Token::Plus => (BinaryOp::Add, 9, 10),
                Token::Minus => (BinaryOp::Sub, 9, 10),
                Token::Star => (BinaryOp::Mul, 11, 12),
                Token::Divide => (BinaryOp::Div, 11, 12),
                Token::Percent => (BinaryOp::Percent, 11, 12),
                _ => break,
            };

            if l_bp < min_bp {
                break;
            }
            self.advance(); //Consume Operator
            let rhs = self.parse_expr(r_bp)?;

            lhs = Expr::BinaryOp {
                left: Box::new(lhs),
                op,
                right: Box::new(rhs),
            };
        }

        Ok(lhs)
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        match self.advance() {
            Token::Number(n) => Ok(Expr::Literal(Literal::Number(n))),
            Token::String(n) => Ok(Expr::Literal(Literal::String(n))),
            Token::True => Ok(Expr::Literal(Literal::Bool(true))),
            Token::False => Ok(Expr::Literal(Literal::Bool(false))),
            Token::Null => Ok(Expr::Literal(Literal::Null)),

            // SO that COUNT(*) works
            Token::Star => Ok(Expr::Glob),

            // CAST(expr AS type)
            Token::Cast => {
                self.expect(&Token::LParen)?;
                let e = self.parse_expr(0)?;
                self.expect(&Token::As)?;
                let data_type = self.parse_data_type()?;
                self.expect(&Token::RParen)?;
                Ok(Expr::Cast {
                    expr: Box::new(e),
                    data_type,
                })
            }

            // Scalar subquery or (expr)
            Token::LParen => {
                if self.peek() == &Token::Select {
                    let q = self.parse_select()?;
                    self.expect(&Token::RParen)?;
                    Ok(Expr::SubQuery(Box::new(q)))
                } else {
                    let e = self.parse_expr(0)?;
                    self.expect(&Token::RParen)?;
                    Ok(e)
                }
            }

            Token::Id(id) => {
                // table.col or table.*
                if self.eat(&Token::Dot) {
                    if self.eat(&Token::Star) {
                        return Ok(Expr::QualifiedGlob(Ident(id)));
                    }
                    let col = self.expect_ident()?;
                    return Ok(Expr::Identifier(Ident(format!("{id}.{}", col.0))));
                }

                // Must be a function name(...)
                if self.eat(&Token::LParen) {
                    return self.parse_call(id);
                }

                Ok(Expr::Identifier(Ident(id)))
            }

            got => Err(ParseError::UnexpectedToken {
                got,
                expected: "Expression",
            }),
        }
    }

    fn parse_call(&mut self, id: String) -> Result<Expr> {
        // COUNT(*)
        if self.peek() == &Token::Star {
            self.advance();
            self.expect(&Token::RParen)?;
            let f = self.parse_filter()?;

            return Ok(Expr::Function(Call {
                name: Ident(id),
                args: Args::Star,
                distinct: false,
                filter: f,
            }));
        }

        // f()
        if self.eat(&Token::RParen) {
            let f = self.parse_filter()?;
            return Ok(Expr::Function(Call {
                name: Ident(id),
                args: Args::List(vec![]),
                distinct: false,
                filter: f,
            }));
        }

        // f([disticnt] a1, a3, ...)
        let distinct = self.eat(&Token::Distinct);
        let args = self.comma_sep(|p| p.parse_expr(0))?;
        self.expect(&Token::RParen)?;
        let f = self.parse_filter()?;

        Ok(Expr::Function(Call {
            name: Ident(id),
            args: Args::List(args),
            distinct,
            filter: f,
        }))
    }

    fn parse_filter(&mut self) -> Result<Option<Box<Expr>>> {
        if self.eat(&Token::Filter) {
            self.expect(&Token::LParen)?;
            self.expect(&Token::Where)?;
            let e = self.parse_expr(0)?;
            self.expect(&Token::RParen)?;
            Ok(Some(Box::new(e)))
        } else {
            Ok(None)
        }
    }

    fn parse_data_type(&mut self) -> Result<DataType> {
        match self.advance() {
            Token::Integer => Ok(DataType::Integer),
            Token::Float => Ok(DataType::Float),
            Token::Bool => Ok(DataType::Bool),
            Token::Text => Ok(DataType::String),

            got => Err(ParseError::UnexpectedToken {
                got,
                expected: "INTEGER | FLOAT | BOOL | TEXT",
            }),
        }
    }
}

fn token_desc(t: &Token) -> &'static str {
    match t {
        Token::Create => "CREATE",
        Token::Table => "TABLE",
        Token::Insert => "INSERT",
        Token::Into => "INTO",
        Token::Values => "VALUES",
        Token::Select => "SELECT",
        Token::From => "FROM",
        Token::Where => "WHERE",
        Token::Delete => "DELETE",
        Token::Drop => "DROP",
        Token::Update => "UPDATE",
        Token::Set => "SET",
        Token::Distinct => "DISTINCT",
        Token::All => "ALL",
        Token::Null => "NULL",
        Token::And => "AND",
        Token::Or => "OR",
        Token::Not => "NOT",
        Token::True => "TRUE",
        Token::False => "FALSE",
        Token::As => "AS",
        Token::Is => "IS",
        Token::Between => "BETWEEN",
        Token::In => "IN",
        Token::Like => "LIKE",
        Token::Ilike => "ILIKE",
        Token::Exists => "EXISTS",
        Token::Cast => "CAST",
        Token::Filter => "FILTER",
        Token::If => "IF",
        Token::Join => "JOIN",
        Token::Inner => "INNER",
        Token::Outer => "OUTER",
        Token::Left => "LEFT",
        Token::Right => "RIGHT",
        Token::Natural => "NATURAL",
        Token::Full => "FULL",
        Token::Cross => "CROSS",
        Token::On => "ON",
        Token::Using => "USING",
        Token::Order => "ORDER",
        Token::By => "BY",
        Token::Group => "GROUP",
        Token::Having => "HAVING",
        Token::Limit => "LIMIT",
        Token::Offset => "OFFSET",
        Token::Asc => "ASC",
        Token::Desc => "DESC",
        Token::Nulls => "NULLS",
        Token::First => "FIRST",
        Token::Last => "LAST",
        Token::Primary => "PRIMARY",
        Token::Key => "KEY",
        Token::Unique => "UNIQUE",
        Token::Default => "DEFAULT",
        Token::Integer => "INTEGER",
        Token::Float => "FLOAT",
        Token::Bool => "BOOLEAN",
        Token::Text => "TEXT",

        Token::LParen => "(",
        Token::RParen => ")",
        Token::Comma => ",",
        Token::Semicolon => ";",
        Token::Star => "*",
        Token::Plus => "+",
        Token::Minus => "-",
        Token::Divide => "/",
        Token::Percent => "%",
        Token::Dot => ".",
        Token::Equal => "=",
        Token::NotEqual => "!=",
        Token::Less => "<",
        Token::LessEqual => "<=",
        Token::Greater => ">",
        Token::GreaterEqual => ">=",

        Token::Id(_) => "identifier",
        Token::Number(_) => "number",
        Token::String(_) => "string literal",

        Token::EOF => "end of input",
    }
}

// Thank you ChatGPT for the Test Suite :yum
#[cfg(test)]
mod tests {
    use crate::sql::{
        ast::*,
        lex::{Lex, Token},
        parser::Parser,
    };
    #[test]
    fn parse_select() {
        use crate::sql::ast::*;
        use crate::sql::lex::{Lex, Token};
        use crate::sql::parser::Parser;

        let sql = "
        SELECT DISTINCT 
            u.id,
            u.name AS username,
            COUNT(*) AS cnt,
            SUM(o.amount) + 10 * 2 AS total,
            NOT (u.age > 18 OR u.banned = 1) AS flag
        FROM users u
        INNER JOIN orders o ON u.id = o.user_id
        WHERE 
            (u.age BETWEEN 18 AND 30 OR u.id IN (1, 2, 3))
            AND u.name LIKE 'A%'
            AND u.deleted IS NOT NULL
        GROUP BY u.id, u.name
        HAVING SUM(o.amount) > 100
        ORDER BY u.id DESC, total ASC
        LIMIT 10 OFFSET 5;
    ";

        let mut lex = Lex::new();
        lex.input = sql.chars().collect();

        let tokens: Vec<Token> = lex
            .map(|t| t.unwrap())
            .take_while(|t| *t != Token::EOF)
            .chain(std::iter::once(Token::EOF))
            .collect();

        let mut parser = Parser::new(tokens);
        let stmt = parser.parse().unwrap();

        // Ensure full consumption (VERY important)
        assert_eq!(parser.peek(), &Token::EOF);

        match stmt {
            Stmt::Select(s) => {
                assert_eq!(s.col.len(), 5);
                assert_eq!(s.joins.len(), 1);
                assert_eq!(s.group_by.len(), 2);
                assert!(s.having.is_some());
                assert_eq!(s.order_by.len(), 2);
                assert_eq!(s.limit, Some(10));
                assert_eq!(s.offset, Some(5));

                // WHERE should be AND at top level
                match s.where_clause.unwrap() {
                    Expr::BinaryOp {
                        op: BinaryOp::And, ..
                    } => {}
                    _ => panic!("Expected AND at top-level WHERE"),
                }

                // HAVING should be comparison
                match s.having.unwrap() {
                    Expr::BinaryOp {
                        op: BinaryOp::Gt, ..
                    } => {}
                    _ => panic!("Expected comparison in HAVING"),
                }

                // ORDER BY correctness
                assert_eq!(s.order_by[0].dir, SortType::Desc);
                assert_eq!(s.order_by[1].dir, SortType::Asc);
            }
            _ => panic!("Expected SELECT"),
        }
    }

    #[test]
    fn parse_mutations() {
        use crate::sql::ast::*;
        use crate::sql::lex::{Lex, Token};
        use crate::sql::parser::Parser;

        // ---------- INSERT ----------
        let insert_sql = "
        INSERT INTO users (id, score, active)
        SELECT 
            u.id,
            u.score * 2 + 5,
            NOT (u.age > 18 AND u.banned = 0)
        FROM users u
        WHERE u.id IN (1, 2, 3) AND u.name LIKE 'A%';
    ";

        let mut lex = Lex::new();
        lex.input = insert_sql.chars().collect();

        let tokens: Vec<Token> = lex
            .map(|t| t.unwrap())
            .take_while(|t| *t != Token::EOF)
            .chain(std::iter::once(Token::EOF))
            .collect();

        let mut parser = Parser::new(tokens);
        let stmt = parser.parse().unwrap();

        match stmt {
            Stmt::Insert(insert) => match insert.source {
                InsertSource::Select(sel) => {
                    assert_eq!(sel.col.len(), 3);
                    assert!(sel.where_clause.is_some());

                    match sel.where_clause.unwrap() {
                        Expr::BinaryOp {
                            op: BinaryOp::And, ..
                        } => {}
                        _ => panic!("Expected AND in INSERT SELECT WHERE"),
                    }
                }
                _ => panic!("Expected SELECT source"),
            },
            _ => panic!("Expected INSERT"),
        }

        // ---------- UPDATE ----------
        let update_sql = "
        UPDATE users u
        SET 
            score = score * 2 + 5,
            active = NOT (age > 18 OR banned = 1),
            name = name
        WHERE 
            (score BETWEEN 10 AND 20 OR id IN (1, 2, 3))
            AND NOT deleted
            OR name LIKE 'A%';
    ";

        let mut lex = Lex::new();
        lex.input = update_sql.chars().collect();

        let tokens: Vec<Token> = lex
            .map(|t| t.unwrap())
            .take_while(|t| *t != Token::EOF)
            .chain(std::iter::once(Token::EOF))
            .collect();

        let mut parser = Parser::new(tokens);
        let stmt = parser.parse().unwrap();

        match stmt {
            Stmt::Update(update) => {
                assert_eq!(update.assign.len(), 3);

                let where_expr = update.where_clause.unwrap();

                match where_expr {
                    Expr::BinaryOp {
                        op: BinaryOp::Or, ..
                    } => {}
                    _ => panic!("Expected OR at top-level WHERE"),
                }
            }
            _ => panic!("Expected UPDATE"),
        }

        // ---------- DELETE ----------
        let delete_sql = "
        DELETE FROM users
        WHERE 
            (age > 18 AND active = true)
            OR name LIKE 'A%'
            AND id NOT IN (SELECT id FROM users);
    ";

        let mut lex = Lex::new();
        lex.input = delete_sql.chars().collect();

        let tokens: Vec<Token> = lex
            .map(|t| t.unwrap())
            .take_while(|t| *t != Token::EOF)
            .chain(std::iter::once(Token::EOF))
            .collect();

        let mut parser = Parser::new(tokens);
        let stmt = parser.parse().unwrap();

        match stmt {
            Stmt::Delete(del) => {
                let expr = del.where_clause.unwrap();

                match expr {
                    Expr::BinaryOp {
                        op: BinaryOp::Or, ..
                    } => {}
                    _ => panic!("Expected OR at top-level DELETE WHERE"),
                }
            }
            _ => panic!("Expected DELETE"),
        }
    }
}
