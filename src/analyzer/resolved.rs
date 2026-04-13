use crate::{
    analyzer::scope::ColRef,
    sql::ast::{BinaryOp, Literal, UnaryOp},
};

pub enum Ty {
    Int,
    Float,
    Bool,
    Text,
    Null,
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
}
