#![allow(dead_code)]
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
    // joins: Vec<JoinClause>,
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
    table: TableRef,
    assign: Vec<Assignment>,
    where_clause: Option<Expr>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Assignment {
    column: Ident,
    value: Expr,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DeleteStmt {
    table: TableRef,
    where_clause: Option<Expr>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CreateStmt {
    Table(CreateTableStmt),
}

#[derive(Debug, PartialEq, Eq)]
pub struct CreateTableStmt {
    name: Ident,
    columns: Vec<ColumnDef>,
    flag: bool, //If not exists clause
}

#[derive(Debug, PartialEq, Eq)]
pub struct ColumnDef {
    name: Ident,
    data_type: DataType,
    constraints: Vec<ColumnConstraint>,
}

#[derive(Debug, PartialEq, Eq)]
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
    kind: JoinKind,
    table: TableRef,
    constraint: JoinConstraint,
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
        case_insensitive: bool, // LIKE vs ILIKE
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
    name: Ident,
    args: Args,
    distinct: bool,
    filter: Option<Box<Expr>>, // aggregate
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

#[derive(Debug, PartialEq, Eq)]
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
