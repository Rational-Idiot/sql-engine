use crate::{
    analyzer::scope::ColRef,
    sql::ast::{BinaryOp, JoinKind, Literal, SetQuantifier, SortType, UnaryOp},
};

#[derive(PartialEq, Clone, Copy)]
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

pub struct RCall {
    pub name: String,
    pub args: RArgs,
    pub distinct: bool,
    pub filter: Option<Box<RExpr>>,
    pub return_ty: Ty,
    pub kind: FnKind,
}

pub enum FnKind {
    Aggregate, // COUNT, SUM, AVG, MIN, MAX
    Scalar,    // UPPER, LOWER, LENGTH, ABS
}

pub enum RArgs {
    Star, // COUNT(*)
    List(Vec<RExpr>),
}

pub struct RSelect {
    pub col: Vec<RSelectItem>,
    pub quantifier: SetQuantifier,
    pub from: Option<SetQuantifier>,
    pub joins: Vec<RJoin>,
    pub where_clause: Option<RExpr>,
    pub group_by: Vec<RExpr>,
    pub having: Option<RExpr>,
    pub order_by: Vec<ROrder>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

pub struct ROrder {
    pub expr: RExpr,
    pub dir: SortType,
    // nulls: NullOrdering,
}

pub struct RSelectItem {
    pub expr: RExpr,
    pub label: String, // AS alias, col name, synthesised "col_N"
}

pub struct RJoin {
    pub kind: JoinKind,
    pub table: RTableRef,
    pub constraint: RJoinConstraint,
}

pub enum RJoinConstraint {
    Natural,
    On(RExpr),
    Using(Vec<String>),
}

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
