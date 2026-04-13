#![allow(dead_code)]

//Structure is insipred by stoolap - https://github.com/stoolap/stoolap/blob/main/src/parser/ast.rs

use core::fmt;
#[derive(Debug, PartialEq, Eq)]
pub enum Stmt {
    Select(SelectStmt),
    Insert(InsertStmt),
    Update(UpdateStmt),
    Delete(DeleteStmt),
    Create(CreateStmt),
    Drop(DropStmt),
}

#[derive(Debug, PartialEq, Eq)]
pub struct SelectStmt {
    pub col: Vec<SelectItem>,
    pub quantifier: SetQuantifier,
    pub from: Option<TableRef>,
    pub joins: Vec<JoinClause>,
    pub where_clause: Option<Expr>,
    pub group_by: Vec<Expr>,
    pub having: Option<Expr>,
    pub order_by: Vec<Order>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct InsertStmt {
    pub table: Ident,
    pub columns: Vec<Ident>, // empty => All
    pub source: InsertSource,
}

#[derive(Debug, PartialEq, Eq)]
pub enum InsertSource {
    Values(Vec<Vec<Expr>>),
    Select(Box<SelectStmt>),
}

#[derive(Debug, PartialEq, Eq)]
pub struct UpdateStmt {
    pub table: TableRef,
    pub assign: Vec<Assignment>,
    pub where_clause: Option<Expr>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Assignment {
    pub column: Ident,
    pub value: Expr,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DeleteStmt {
    pub table: TableRef,
    pub where_clause: Option<Expr>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CreateStmt {
    Table(CreateTableStmt),
}

#[derive(Debug, PartialEq, Eq)]
pub struct CreateTableStmt {
    pub name: Ident,
    pub columns: Vec<ColumnDef>,
    pub flag: bool, //If not exists clause
}

#[derive(Debug, PartialEq, Eq)]
pub struct ColumnDef {
    pub name: Ident,
    pub data_type: DataType,
    pub constraints: Vec<ColumnConstraint>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DataType {
    Integer,
    Float,
    Bool,
    String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ColumnConstraint {
    PrimaryKey,
    NotNull,
    Unique,
    Default(Expr),
}

#[derive(Debug, PartialEq, Eq)]
pub enum DropStmt {
    Table { name: Ident, if_exists: bool },
    //TODO: Index, View
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ident(pub String);

#[derive(Debug, PartialEq, Eq)]
pub struct Order {
    pub expr: Expr,
    pub dir: SortType,
    // nulls: NullOrdering,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SortType {
    Asc,
    Desc,
}

#[derive(Debug, PartialEq, Eq)]
pub enum NullOrdering {
    First,
    Last,
    Default,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SetQuantifier {
    All,
    Distinct,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TableRef {
    Named {
        name: Ident,
        alias: Option<Ident>,
    },
    Subquery {
        query: Box<SelectStmt>,
        alias: Ident,
    },
    //TODO: TableFunction{name, args, alias}
}

#[derive(Debug, PartialEq, Eq)]
pub struct SelectItem {
    pub expr: Expr,
    pub alias: Option<Ident>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct JoinClause {
    pub kind: JoinKind,
    pub table: TableRef,
    pub constraint: JoinConstraint,
}

#[derive(Debug, PartialEq, Eq)]
pub enum JoinKind {
    Inner,
    Left,
    Right,
    Outer,
    Cross,
    //TODO: LeftSemi LeftAnit bhaang bhosda
}

#[derive(Debug, PartialEq, Eq)]
pub enum JoinConstraint {
    Natural,
    On(Expr),
    Using(Vec<Ident>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Expr {
    Literal(Literal),
    Identifier(Ident),
    Glob,
    QualifiedGlob(Ident),

    BinaryOp {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },

    UnaryOp {
        op: UnaryOp,
        expr: Box<Expr>,
    },

    IsNull {
        expr: Box<Expr>,
        neg: bool, // IS NULL | IS NOT NULL
    },

    Between {
        expr: Box<Expr>,
        negated: bool,
        low: Box<Expr>,
        high: Box<Expr>,
    },

    InList {
        expr: Box<Expr>,
        list: Vec<Expr>,
        neg: bool,
    },

    InSubquery {
        expr: Box<Expr>,
        query: Box<SelectStmt>,
        neg: bool,
    },

    Like {
        expr: Box<Expr>,
        pattern: Box<Expr>,
        neg: bool,
        insensitive: bool, // LIKE vs ILIKE
    },

    SubQuery(Box<SelectStmt>),
    Exists {
        query: Box<SelectStmt>,
        neg: bool,
    },

    Function(Call),
    Cast {
        expr: Box<Expr>,
        data_type: DataType,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub struct Call {
    pub name: Ident,
    pub args: Args,
    pub distinct: bool,
    pub filter: Option<Box<Expr>>, // aggregate
}

#[derive(Debug, PartialEq, Eq)]
pub enum Args {
    Star, // COUNT(*)
    List(Vec<Expr>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Percent,

    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,

    And,
    Or,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Literal {
    Number(String),
    String(String),
    Bool(bool),
    Null,
}

// Thank you ChatGPT for the Displays
impl fmt::Display for Stmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Stmt::Select(s) => write!(f, "{s}"),
            Stmt::Insert(i) => write!(f, "{i}"),
            Stmt::Update(u) => write!(f, "{u}"),
            Stmt::Delete(d) => write!(f, "{d}"),
            Stmt::Create(c) => write!(f, "{c}"),
            Stmt::Drop(d) => write!(f, "{d}"),
        }
    }
}

impl fmt::Display for SelectStmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SELECT ")?;

        if self.quantifier == SetQuantifier::Distinct {
            write!(f, "DISTINCT ")?;
        }

        for (i, col) in self.col.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{col}")?;
        }

        if let Some(from) = &self.from {
            write!(f, "\nFROM {from}")?;
        }

        if let Some(w) = &self.where_clause {
            write!(f, "\nWHERE ")?;
            w.fmt_with_indent(f, 1, 0)?;
        }

        if !self.order_by.is_empty() {
            write!(f, "\nORDER BY ")?;
            for (i, o) in self.order_by.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{o}")?;
            }
        }

        if let Some(limit) = self.limit {
            write!(f, "\nLIMIT {limit}")?;
        }

        if let Some(offset) = self.offset {
            write!(f, "\nOFFSET {offset}")?;
        }

        Ok(())
    }
}

impl fmt::Display for SelectItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.expr {
            Expr::Glob => write!(f, "*")?,
            _ => write!(f, "{}", self.expr)?,
        }

        if let Some(alias) = &self.alias {
            write!(f, " AS {}", alias.0)?;
        }

        Ok(())
    }
}

impl fmt::Display for TableRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TableRef::Named { name, alias } => {
                write!(f, "{}", name.0)?;
                if let Some(a) = alias {
                    write!(f, " {}", a.0)?;
                }
                Ok(())
            }
            TableRef::Subquery { query, alias } => write!(f, "({}) {}", query, alias.0),
        }
    }
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Percent => "%",
            BinaryOp::Eq => "=",
            BinaryOp::Ne => "<>",
            BinaryOp::Lt => "<",
            BinaryOp::Gt => ">",
            BinaryOp::Le => "<=",
            BinaryOp::Ge => ">=",
            BinaryOp::And => "AND",
            BinaryOp::Or => "OR",
        };
        write!(f, "{s}")
    }
}

impl fmt::Display for InsertStmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "INSERT INTO {}", self.table.0)?;

        if !self.columns.is_empty() {
            write!(f, " (")?;
            for (i, col) in self.columns.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", col.0)?;
            }
            write!(f, ")")?;
        }

        match &self.source {
            InsertSource::Values(rows) => {
                write!(f, "\nVALUES ")?;
                for (i, row) in rows.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "(")?;
                    for (j, expr) in row.iter().enumerate() {
                        if j > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{expr}")?;
                    }
                    write!(f, ")")?;
                }
            }
            InsertSource::Select(q) => {
                write!(f, "\n{q}")?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for UpdateStmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UPDATE {}", self.table)?;

        write!(f, "\nSET ")?;
        for (i, assign) in self.assign.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{} = {}", assign.column.0, assign.value)?;
        }

        if let Some(w) = &self.where_clause {
            write!(f, "\nWHERE {w}")?;
        }

        Ok(())
    }
}

impl fmt::Display for DeleteStmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DELETE FROM {}", self.table)?;

        if let Some(w) = &self.where_clause {
            write!(f, "\nWHERE {w}")?;
        }

        Ok(())
    }
}

impl fmt::Display for CreateStmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CreateStmt::Table(t) => write!(f, "{t}"),
        }
    }
}

impl fmt::Display for CreateTableStmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CREATE TABLE ")?;

        if self.flag {
            write!(f, "IF NOT EXISTS ")?;
        }

        write!(f, "{} (\n", self.name.0)?;

        for (i, col) in self.columns.iter().enumerate() {
            write!(f, "  {col}")?;
            if i < self.columns.len() - 1 {
                write!(f, ",")?;
            }
            write!(f, "\n")?;
        }

        write!(f, ")")
    }
}

impl fmt::Display for ColumnDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.name.0, self.data_type)?;

        for c in &self.constraints {
            write!(f, " {c}")?;
        }

        Ok(())
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            DataType::Integer => "INTEGER",
            DataType::Float => "FLOAT",
            DataType::Bool => "BOOLEAN",
            DataType::String => "TEXT",
        };
        write!(f, "{s}")
    }
}

impl fmt::Display for ColumnConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColumnConstraint::PrimaryKey => write!(f, "PRIMARY KEY"),
            ColumnConstraint::NotNull => write!(f, "NOT NULL"),
            ColumnConstraint::Unique => write!(f, "UNIQUE"),
            ColumnConstraint::Default(expr) => write!(f, "DEFAULT {}", expr),
        }
    }
}

impl fmt::Display for DropStmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DropStmt::Table { name, if_exists } => {
                write!(f, "DROP TABLE ")?;
                if *if_exists {
                    write!(f, "IF EXISTS ")?;
                }
                write!(f, "{}", name.0)
            }
        }
    }
}

impl fmt::Display for Order {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.dir {
            SortType::Asc => write!(f, "{} ASC", self.expr),
            SortType::Desc => write!(f, "{} DESC", self.expr),
        }
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::Number(n) => write!(f, "{n}"),
            Literal::String(s) => write!(f, "'{}'", s),
            Literal::Bool(b) => write!(f, "{}", if *b { "TRUE" } else { "FALSE" }),
            Literal::Null => write!(f, "NULL"),
        }
    }
}

fn op_prec(op: &BinaryOp) -> u8 {
    match op {
        BinaryOp::Or => 1,
        BinaryOp::And => 2,
        BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
            3
        }
        BinaryOp::Add | BinaryOp::Sub => 4,
        BinaryOp::Mul | BinaryOp::Div | BinaryOp::Percent => 5,
    }
}

fn indent(f: &mut fmt::Formatter<'_>, n: usize) -> fmt::Result {
    for _ in 0..n {
        write!(f, "  ")?;
    }
    Ok(())
}

impl Expr {
    fn collect_logical_chain<'a>(&'a self, op: &BinaryOp, out: &mut Vec<&'a Expr>) {
        match self {
            Expr::BinaryOp {
                left,
                op: inner_op,
                right,
            } if inner_op == op => {
                left.collect_logical_chain(op, out);
                right.collect_logical_chain(op, out);
            }
            _ => out.push(self),
        }
    }

    fn fmt_with_indent(
        &self,
        f: &mut fmt::Formatter<'_>,
        indent_lvl: usize,
        parent_prec: u8,
    ) -> fmt::Result {
        match self {
            Expr::Literal(l) => write!(f, "{l}"),
            Expr::Identifier(id) => write!(f, "{}", id.0),
            Expr::Glob => write!(f, "*"),
            Expr::QualifiedGlob(id) => write!(f, "{}.*", id.0),

            Expr::UnaryOp { op, expr } => {
                let my_prec = 6;
                let needs_paren = my_prec < parent_prec;

                if needs_paren {
                    write!(f, "(")?;
                }

                match op {
                    UnaryOp::Neg => write!(f, "-")?,
                    UnaryOp::Not => write!(f, "NOT ")?,
                }
                expr.fmt_with_indent(f, indent_lvl, my_prec)?;

                if needs_paren {
                    write!(f, ")")?;
                }
                Ok(())
            }

            Expr::BinaryOp { left, op, right } => {
                let my_prec = op_prec(op);
                let needs_paren = my_prec < parent_prec;

                if needs_paren {
                    write!(f, "(")?;
                }

                left.fmt_with_indent(f, indent_lvl, my_prec)?;

                match op {
                    BinaryOp::And | BinaryOp::Or => {
                        writeln!(f)?;
                        indent(f, indent_lvl)?;
                        write!(f, "{op} ")?;
                    }
                    _ => {
                        write!(f, " {} ", op)?;
                    }
                }

                right.fmt_with_indent(f, indent_lvl, my_prec + 1)?;

                if needs_paren {
                    write!(f, ")")?;
                }

                Ok(())
            }

            Expr::IsNull { expr, neg } => {
                expr.fmt_with_indent(f, indent_lvl, 10)?;
                if *neg {
                    write!(f, " IS NOT NULL")
                } else {
                    write!(f, " IS NULL")
                }
            }

            Expr::Between {
                expr,
                negated,
                low,
                high,
            } => {
                expr.fmt_with_indent(f, indent_lvl, 10)?;
                if *negated {
                    write!(f, " NOT BETWEEN ")?;
                } else {
                    write!(f, " BETWEEN ")?;
                }
                low.fmt_with_indent(f, indent_lvl, 10)?;
                write!(f, " AND ")?;
                high.fmt_with_indent(f, indent_lvl, 10)
            }

            Expr::InList { expr, list, neg } => {
                expr.fmt_with_indent(f, indent_lvl, 10)?;
                if *neg {
                    write!(f, " NOT IN (")?;
                } else {
                    write!(f, " IN (")?;
                }

                for (i, e) in list.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    e.fmt_with_indent(f, indent_lvl, 0)?;
                }

                write!(f, ")")
            }

            Expr::InSubquery { expr, query, neg } => {
                expr.fmt_with_indent(f, indent_lvl, 10)?;
                if *neg {
                    writeln!(f, " NOT IN (")?;
                } else {
                    writeln!(f, " IN (")?;
                }

                let s = format!("{query}");
                for line in s.lines().filter(|l| !l.trim().is_empty()) {
                    indent(f, indent_lvl + 1)?;
                    writeln!(f, "{line}")?;
                }
                indent(f, indent_lvl)?;
                write!(f, ")")
            }

            Expr::Like {
                expr,
                pattern,
                neg,
                insensitive,
            } => {
                expr.fmt_with_indent(f, indent_lvl, 10)?;
                if *neg {
                    write!(f, " NOT ")?;
                } else {
                    write!(f, " ")?;
                }

                if *insensitive {
                    write!(f, "ILIKE ")?;
                } else {
                    write!(f, "LIKE ")?;
                }

                pattern.fmt_with_indent(f, indent_lvl, 10)
            }

            Expr::Exists { query, neg } => {
                if *neg {
                    writeln!(f, "NOT EXISTS (")?;
                } else {
                    writeln!(f, "EXISTS (")?;
                }

                let s = format!("{query}");
                for line in s.lines().filter(|l| !l.trim().is_empty()) {
                    indent(f, indent_lvl + 1)?;
                    writeln!(f, "{line}")?;
                }
                indent(f, indent_lvl)?;
                write!(f, ")")
            }

            Expr::Cast { expr, data_type } => {
                write!(f, "CAST(")?;
                expr.fmt_with_indent(f, indent_lvl, 0)?;
                write!(f, " AS {data_type})")
            }

            Expr::SubQuery(query) => {
                writeln!(f, "(")?;
                let s = format!("{query}");
                for line in s.lines().filter(|l| !l.trim().is_empty()) {
                    indent(f, indent_lvl + 1)?;
                    writeln!(f, "{line}")?;
                }
                indent(f, indent_lvl)?;
                write!(f, ")")
            }
            Expr::Function(call) => {
                write!(f, "{}", call)
            }
        }
    }
}

impl fmt::Display for Call {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(", self.name.0)?;

        // DISTINCT
        if self.distinct {
            write!(f, "DISTINCT ")?;
        }

        // Args
        match &self.args {
            Args::Star => {
                write!(f, "*")?;
            }
            Args::List(exprs) => {
                for (i, e) in exprs.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    e.fmt_with_indent(f, 0, 0)?;
                }
            }
        }

        write!(f, ")")?;

        // FILTER clause (for aggregates)
        if let Some(filter) = &self.filter {
            write!(f, " FILTER (WHERE ")?;
            filter.fmt_with_indent(f, 0, 0)?;
            write!(f, ")")?;
        }

        Ok(())
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_with_indent(f, 0, 0)
    }
}
