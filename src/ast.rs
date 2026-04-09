#![allow(dead_code)]
pub enum Stmt {
    Select(SelectStmt),
    Insert(InsertStmt),
    Update(UpdateStmt),
    Delete(DeleteStmt),
    Create(CreateStmt),
    Drop(DropStmt),
}

pub struct SelectStmt {
    columns: Vec<SelectItem>,
    quantifier: SetQuantifier,
    from: Option<TableRef>,
    joins: Vec<JoinClause>,
    where_clause: Option<Expr>,
    group_by: Vec<Expr>,
    having: Option<Expr>,
    order_by: Vec<Order>,
    limit: Option<u64>,
    offset: Option<u64>,
}

pub struct InsertStmt {
    table: Ident,
    columns: Vec<Ident>, // empty => All
    source: InsertSource,
}

pub enum InsertSource {
    Values(Vec<Vec<Expr>>),
    Select(Box<SelectStmt>),
}

pub struct UpdateStmt {
    table: TableRef,
    assign: Vec<Assignment>,
    where_clause: Option<Expr>,
}

pub struct Assignment {
    column: Ident,
    value: Expr,
}

pub struct DeleteStmt {
    table: TableRef,
    where_clause: Option<Expr>,
}

pub enum CreateStmt {
    Table(CreateTableStmt),
}

pub struct CreateTableStmt {
    name: Ident,
    columns: Vec<ColumnDef>,
    flag: bool, //If not exists clause
}

pub struct ColumnDef {
    name: Ident,
    data_type: DataType,
    constraints: Vec<ColumnConstraint>,
}

pub enum DataType {
    Integer,
    Float,
    Bool,
    String,
}

pub enum ColumnConstraint {
    PrimaryKey,
    NotNull,
    Unique,
    Default(Expr),
}

pub enum DropStmt {
    Table { name: Ident, if_exists: bool },
    //TODO: Index, View
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ident(pub String);

pub struct Order {
    expr: Expr,
    direction: SortType,
    nulls: NullOrdering,
}

pub enum SortType {
    Asc,
    Desc,
}

pub enum NullOrdering {
    First,
    Last,
    Default,
}

pub enum SetQuantifier {
    All,
    Distinct,
}

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

pub struct SelectItem {
    pub expr: Expr,
    pub alias: Option<Ident>,
}

pub struct JoinClause {
    kind: JoinKind,
    table: TableRef,
    constraint: JoinConstraint,
}

pub enum JoinKind {
    Inner,
    Left,
    Right,
    Outer,
    Cross,
    //TODO: LeftSemi LeftAnit bhaang bhosda
}

pub enum JoinConstraint {
    Natural,
    On(Expr),
    Using(Vec<Ident>),
}

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

pub struct Call {
    name: Ident,
    args: Args,
    distinct: bool,
    filter: Option<Box<Expr>>, // aggregate
}

pub enum Args {
    Star, // COUNT(*)
    List(Vec<Expr>),
}

pub enum UnaryOp {
    Neg,
    Not,
}

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

pub enum Literal {
    Number(String),
    String(String),
    Bool(bool),
    Null,
}
