#![allow(dead_code)]

use super::logical::LogicalPlan;
use crate::{
    analyzer::resolved::{RExpr, RJoinConstraint, ROrder, RSelectItem},
    sql::ast::JoinKind,
};

// Every node names its concrete execution strategy.
// Right now there is only one strategy per node type.
pub enum PhysicalPlan {
    SeqScan {
        table: String,
    },
    Values {
        rows: Vec<Vec<RExpr>>,
    },

    Filter {
        predicate: RExpr,
        input: Box<PhysicalPlan>,
    },
    Project {
        cols: Vec<RSelectItem>,
        input: Box<PhysicalPlan>,
    },
    Sort {
        keys: Vec<ROrder>,
        input: Box<PhysicalPlan>,
    },
    Limit {
        limit: Option<u64>,
        offset: Option<u64>,
        input: Box<PhysicalPlan>,
    },
    Distinct {
        input: Box<PhysicalPlan>,
    },

    HashAggregate {
        keys: Vec<RExpr>,
        aggs: Vec<RSelectItem>,
        input: Box<PhysicalPlan>,
    },

    NestedLoopJoin {
        kind: JoinKind,
        left: Box<PhysicalPlan>,
        right: Box<PhysicalPlan>,
        constraint: RJoinConstraint,
    },

    Insert {
        table: String,
        col_idxs: Vec<usize>,
        input: Box<PhysicalPlan>,
    },
    Update {
        table: String,
        assign: Vec<(usize, RExpr)>,
        input: Box<PhysicalPlan>,
    },
    Delete {
        table: String,
        input: Box<PhysicalPlan>,
    },
}

// Currrently Trivial one logical node becomes one physical node.
// An optimizer pass will rewrite the LogicalPlan before this runs,
// which is the only thing that needs to change when optimizations are added.
pub fn lower(plan: LogicalPlan) -> PhysicalPlan {
    match plan {
        LogicalPlan::Scan { table } => PhysicalPlan::SeqScan { table },
        LogicalPlan::Values { rows } => PhysicalPlan::Values { rows },

        LogicalPlan::Filter { predicate, input } => PhysicalPlan::Filter {
            predicate,
            input: Box::new(lower(*input)),
        },
        LogicalPlan::Project { cols, input } => PhysicalPlan::Project {
            cols,
            input: Box::new(lower(*input)),
        },
        LogicalPlan::Sort { keys, input } => PhysicalPlan::Sort {
            keys,
            input: Box::new(lower(*input)),
        },
        LogicalPlan::Distinct { input } => PhysicalPlan::Distinct {
            input: Box::new(lower(*input)),
        },
        LogicalPlan::Limit {
            limit,
            offset,
            input,
        } => PhysicalPlan::Limit {
            limit,
            offset,
            input: Box::new(lower(*input)),
        },

        LogicalPlan::Aggregate { keys, aggs, input } => PhysicalPlan::HashAggregate {
            keys,
            aggs,
            input: Box::new(lower(*input)),
        },

        LogicalPlan::Join {
            kind,
            left,
            right,
            constraint,
        } => PhysicalPlan::NestedLoopJoin {
            kind,
            left: Box::new(lower(*left)),
            right: Box::new(lower(*right)),
            constraint,
        },

        LogicalPlan::Insert {
            table,
            col_idxs,
            input,
        } => PhysicalPlan::Insert {
            table,
            col_idxs,
            input: Box::new(lower(*input)),
        },
        LogicalPlan::Update {
            table,
            assign,
            input,
        } => PhysicalPlan::Update {
            table,
            assign,
            input: Box::new(lower(*input)),
        },
        LogicalPlan::Delete { table, input } => PhysicalPlan::Delete {
            table,
            input: Box::new(lower(*input)),
        },
    }
}
