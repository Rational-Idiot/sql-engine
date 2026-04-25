#![allow(dead_code)]

use crate::{
    analyzer::resolved::{RExpr, RJoinConstraint, ROrder, RSelectItem},
    sql::ast::JoinKind,
};

// Relational Algebra model
pub enum LogicalPlan {
    Scan {
        table: String,
    },
    Values {
        rows: Vec<Vec<RExpr>>,
    },

    Filter {
        predicate: RExpr,
        input: Box<LogicalPlan>,
    },
    Project {
        cols: Vec<RSelectItem>,
        input: Box<LogicalPlan>,
    },
    Sort {
        keys: Vec<ROrder>,
        input: Box<LogicalPlan>,
    },
    Limit {
        limit: Option<u64>,
        offset: Option<u64>,
        input: Box<LogicalPlan>,
    },
    Distinct {
        input: Box<LogicalPlan>,
    },

    Aggregate {
        keys: Vec<RExpr>,
        aggs: Vec<RSelectItem>,
        input: Box<LogicalPlan>,
    },

    Join {
        kind: JoinKind,
        left: Box<LogicalPlan>,
        right: Box<LogicalPlan>,
        constraint: RJoinConstraint,
    },

    Insert {
        table: String,
        col_idxs: Vec<usize>,
        input: Box<LogicalPlan>,
    },
    Update {
        table: String,
        assign: Vec<(usize, RExpr)>,
        input: Box<LogicalPlan>,
    },
    Delete {
        table: String,
        input: Box<LogicalPlan>,
    },
}
