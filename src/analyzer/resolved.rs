use crate::{
    analyzer::scope::ColRef,
    sql::ast::{BinaryOp, JoinKind, Literal, SetQuantifier, SortType, UnaryOp},
};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Ty {
    Int,
    Float,
    Bool,
    Text,
    Null,
}

impl Ty {
    pub fn unify(a: &Ty, b: &Ty) -> Option<Ty> {
        match (a, b) {
            (Ty::Null, other) | (other, Ty::Null) => Some(*other),
            (Ty::Text, _) | (_, Ty::Text) => Some(Ty::Text),
            (Ty::Int, Ty::Float) | (Ty::Float, Ty::Int) => Some(Ty::Float),
            _ if a == b => Some(*a),
            _ => None,
        }
    }
}

impl RExpr {
    pub fn ty(&self) -> Ty {
        match self {
            RExpr::Literal(_, ty) => ty.clone(),
            RExpr::Column(_, ty) => ty.clone(),
            RExpr::BinaryOp { ty, .. } => ty.clone(),
            RExpr::UnaryOp { ty, .. } => ty.clone(),
            RExpr::Cast { data_type, .. } => data_type.clone(),
            RExpr::Function(call) => call.return_ty.clone(),
            RExpr::IsNull { .. } => Ty::Bool,
            RExpr::Between { .. } => Ty::Bool,
            RExpr::InList { .. } => Ty::Bool,
            RExpr::InSubquery { .. } => Ty::Bool,
            RExpr::Like { .. } => Ty::Bool,
            RExpr::Exists { .. } => Ty::Bool,
            RExpr::SubQuery(s) => s.col.first().map(|c| c.expr.ty()).unwrap_or(Ty::Null),
        }
    }
}

impl std::fmt::Display for Ty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ty::Int => write!(f, "INTEGER"),
            Ty::Float => write!(f, "FLOAT"),
            Ty::Bool => write!(f, "BOOL"),
            Ty::Text => write!(f, "TEXT"),
            Ty::Null => write!(f, "NULL"),
        }
    }
}

pub enum RStmt {
    Select(RSelect),
    Insert(RInsert),
    Update(RUpdate),
    Delete(RDelete),
}

#[derive(Clone)]
pub enum RExpr {
    Literal(Literal, Ty),
    Column(ColRef, Ty),

    UnaryOp {
        op: UnaryOp,
        expr: Box<RExpr>,
        ty: Ty,
    },
    BinaryOp {
        op: BinaryOp,
        lhs: Box<RExpr>,
        rhs: Box<RExpr>,
        ty: Ty,
    },

    IsNull {
        expr: Box<RExpr>,
        neg: bool,
    },

    Between {
        expr: Box<RExpr>,
        negated: bool,
        low: Box<RExpr>,
        high: Box<RExpr>,
    },

    InList {
        expr: Box<RExpr>,
        list: Vec<RExpr>,
        neg: bool,
    },

    InSubquery {
        expr: Box<RExpr>,
        query: Box<RSelect>,
        neg: bool,
    },

    Like {
        expr: Box<RExpr>,
        pattern: Box<RExpr>,
        neg: bool,
        insensitive: bool, // LIKE vs ILIKE
    },
    Cast {
        expr: Box<RExpr>,
        data_type: Ty,
    },

    Exists {
        query: Box<RSelect>,
        neg: bool,
    },
    SubQuery(Box<RSelect>),
    Function(RCall),
}

#[derive(Clone)]
pub struct RCall {
    pub name: String,
    pub args: RArgs,
    pub distinct: bool,
    pub filter: Option<Box<RExpr>>,
    pub return_ty: Ty,
    pub kind: FnKind,
}

#[derive(Clone)]
pub enum FnKind {
    Aggregate, // COUNT, SUM, AVG, MIN, MAX
    Scalar,    // UPPER, LOWER, LENGTH, ABS
}

#[derive(Clone)]
pub enum RArgs {
    Star, // COUNT(*)
    List(Vec<RExpr>),
}

#[derive(Clone)]
pub struct RSelect {
    pub col: Vec<RSelectItem>,
    pub quantifier: SetQuantifier,
    pub from: Option<RTableRef>,
    pub joins: Vec<RJoin>,
    pub where_clause: Option<RExpr>,
    pub group_by: Vec<RExpr>,
    pub having: Option<RExpr>,
    pub order_by: Vec<ROrder>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Clone)]
pub struct ROrder {
    pub expr: RExpr,
    pub dir: SortType,
    // nulls: NullOrdering,
}

#[derive(Clone)]
pub struct RSelectItem {
    pub expr: RExpr,
    pub label: String, // AS alias, col name, synthesised "col_N"
}

#[derive(Clone)]
pub struct RJoin {
    pub kind: JoinKind,
    pub table: RTableRef,
    pub constraint: RJoinConstraint,
}

#[derive(Clone)]
pub enum RJoinConstraint {
    Natural,
    On(RExpr),
    Using(Vec<String>),
}

#[derive(Clone)]
pub enum RTableRef {
    Named {
        // The real one from catalog
        table_name: String,
        alias: Option<String>,
    },
    Subquery {
        query: Box<RSelect>,
        alias: String,
    },
}

pub struct RInsert {
    pub table: String,
    pub col_idx: Vec<usize>, // Columns to write in order
    pub source: RInsertSource,
}

pub enum RInsertSource {
    Values(Vec<Vec<RExpr>>),
    Select(Box<RSelect>),
}

pub struct RUpdate {
    pub table: RTableRef,
    pub assign: Vec<(usize, RExpr)>, // (idx, expr)
    pub where_clause: Option<RExpr>,
}

pub struct RDelete {
    pub table: RTableRef,
    pub where_clause: Option<RExpr>,
}
