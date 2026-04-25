#![allow(dead_code)]

use super::logical::LogicalPlan::{self, *};
use super::physical::{PhysicalPlan, lower};
use crate::{
    analyzer::resolved::{
        FnKind, RDelete, RExpr, RInsert, RInsertSource, RSelect, RStmt, RTableRef, RUpdate,
    },
    sql::ast::SetQuantifier,
};

pub struct Planner;

impl Planner {
    pub fn new() -> Self {
        Self
    }

    pub fn plan(&self, stmt: RStmt) -> PhysicalPlan {
        lower(self.plan_logical(stmt))
    }

    fn plan_logical(&self, stmt: RStmt) -> LogicalPlan {
        match stmt {
            RStmt::Select(s) => self.plan_select(s),
            RStmt::Insert(s) => self.plan_insert(s),
            RStmt::Update(s) => self.plan_update(s),
            RStmt::Delete(s) => self.plan_delete(s),
        }
    }

    fn plan_select(&self, s: RSelect) -> LogicalPlan {
        let mut plan = match s.from {
            None => Values { rows: vec![vec![]] },
            Some(tr) => self.plan_table_ref(tr),
        };

        for join in s.joins {
            let right = self.plan_table_ref(join.table);
            plan = Join {
                kind: join.kind,
                left: Box::new(plan),
                right: Box::new(right),
                constraint: join.constraint,
            };
        }

        if let Some(pred) = s.where_clause {
            plan = Filter {
                predicate: pred,
                input: Box::new(plan),
            };
        }

        let has_agg = !s.group_by.is_empty() || s.col.iter().any(|c| expr_has_agg(&c.expr));

        if has_agg {
            let agg_items = s
                .col
                .iter()
                .filter(|c| expr_has_agg(&c.expr))
                .cloned()
                .collect::<Vec<_>>();

            plan = Aggregate {
                keys: s.group_by,
                aggs: agg_items,
                input: Box::new(plan),
            };
        }

        if let Some(pred) = s.having {
            plan = Filter {
                predicate: pred,
                input: Box::new(plan),
            };
        }

        plan = Project {
            cols: s.col,
            input: Box::new(plan),
        };

        if matches!(s.quantifier, SetQuantifier::Distinct) {
            plan = Distinct {
                input: Box::new(plan),
            };
        }

        if !s.order_by.is_empty() {
            plan = Sort {
                keys: s.order_by,
                input: Box::new(plan),
            };
        }

        if s.limit.is_some() || s.offset.is_some() {
            plan = Limit {
                limit: s.limit,
                offset: s.offset,
                input: Box::new(plan),
            };
        }

        plan
    }

    fn plan_table_ref(&self, tr: RTableRef) -> LogicalPlan {
        match tr {
            RTableRef::Named { table_name, .. } => Scan { table: table_name },
            RTableRef::Subquery { query, .. } => self.plan_select(*query),
        }
    }

    fn plan_insert(&self, s: RInsert) -> LogicalPlan {
        let input = match s.source {
            RInsertSource::Values(rows) => Values { rows },
            RInsertSource::Select(query) => self.plan_select(*query),
        };
        Insert {
            table: s.table,
            col_idxs: s.col_idx,
            input: Box::new(input),
        }
    }

    fn plan_update(&self, s: RUpdate) -> LogicalPlan {
        let table_name = match &s.table {
            RTableRef::Named { table_name, .. } => table_name.clone(),
            RTableRef::Subquery { .. } => unreachable!("analyzer rejects subquery UPDATE targets"),
        };

        let mut input = Scan {
            table: table_name.clone(),
        };
        if let Some(pred) = s.where_clause {
            input = Filter {
                predicate: pred,
                input: Box::new(input),
            };
        }

        Update {
            table: table_name,
            assign: s.assign,
            input: Box::new(input),
        }
    }

    fn plan_delete(&self, s: RDelete) -> LogicalPlan {
        let table_name = match &s.table {
            RTableRef::Named { table_name, .. } => table_name.clone(),
            RTableRef::Subquery { .. } => unreachable!("analyzer rejects subquery DELETE targets"),
        };

        let mut input = Scan {
            table: table_name.clone(),
        };
        if let Some(pred) = s.where_clause {
            input = Filter {
                predicate: pred,
                input: Box::new(input),
            };
        }

        Delete {
            table: table_name,
            input: Box::new(input),
        }
    }
}

fn expr_has_agg(e: &RExpr) -> bool {
    match e {
        RExpr::Function(c) => matches!(c.kind, FnKind::Aggregate),

        RExpr::BinaryOp { lhs, rhs, .. } => expr_has_agg(lhs) || expr_has_agg(rhs),
        RExpr::UnaryOp { expr, .. } => expr_has_agg(expr),
        RExpr::IsNull { expr, .. } => expr_has_agg(expr),
        RExpr::Cast { expr, .. } => expr_has_agg(expr),
        RExpr::Like { expr, pattern, .. } => expr_has_agg(expr) || expr_has_agg(pattern),
        RExpr::Between {
            expr, low, high, ..
        } => expr_has_agg(expr) || expr_has_agg(low) || expr_has_agg(high),
        RExpr::InList { expr, list, .. } => expr_has_agg(expr) || list.iter().any(expr_has_agg),

        RExpr::Literal(..)
        | RExpr::Column(..)
        | RExpr::SubQuery(..)
        | RExpr::Exists { .. }
        | RExpr::InSubquery { .. } => false,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        analyzer::analyzer::*,
        test_utils::{mock_catalog, parse},
    };
    #[test]
    fn test_planner_select_everything_pipeline() {
        let stmt = parse(
            "SELECT DISTINCT u.id, COUNT(*) AS cnt, SUM(o.amount) \
         FROM users u \
         INNER JOIN orders o ON u.id = o.user_id \
         WHERE u.age > 18 AND u.name LIKE 'A%' \
         GROUP BY u.id \
         HAVING SUM(o.amount) > 100.0 \
         ORDER BY u.id DESC \
         LIMIT 10 OFFSET 2",
        );

        let mut catalog = mock_catalog();
        let analyzer = Analyzer::new(&mut catalog);
        let analyzed = analyzer.analyze(stmt).unwrap();

        let planner = Planner::new();
        let logical = planner.plan_logical(analyzed);

        use LogicalPlan::*;

        // Top-down assertions (DON'T try full ==, too brittle)
        match logical {
            Limit {
                limit,
                offset,
                input,
            } => {
                assert_eq!(limit, Some(10));
                assert_eq!(offset, Some(2));

                match *input {
                    Sort { keys, input } => {
                        assert_eq!(keys.len(), 1);

                        match *input {
                            Distinct { input } => {
                                match *input {
                                    Project { cols, input } => {
                                        assert_eq!(cols.len(), 3);

                                        match *input {
                                            Filter { input, .. } => {
                                                // HAVING

                                                match *input {
                                                    Aggregate { keys, aggs, input } => {
                                                        assert_eq!(keys.len(), 1);
                                                        assert!(!aggs.is_empty());

                                                        match *input {
                                                            Filter { input, .. } => {
                                                                // WHERE

                                                                match *input {
                                                                    Join {
                                                                        left, right, ..
                                                                    } => match (*left, *right) {
                                                                        (
                                                                            Scan { table: l },
                                                                            Scan { table: r },
                                                                        ) => {
                                                                            assert_eq!(l, "users");
                                                                            assert_eq!(r, "orders");
                                                                        }
                                                                        _ => panic!(
                                                                            "Expected scans under join"
                                                                        ),
                                                                    },
                                                                    _ => panic!("Expected Join"),
                                                                }
                                                            }
                                                            _ => panic!("Expected WHERE Filter"),
                                                        }
                                                    }
                                                    _ => panic!("Expected Aggregate"),
                                                }
                                            }
                                            _ => panic!("Expected HAVING Filter"),
                                        }
                                    }
                                    _ => panic!("Expected Project"),
                                }
                            }
                            _ => panic!("Expected Distinct"),
                        }
                    }
                    _ => panic!("Expected Sort"),
                }
            }
            _ => panic!("Expected Limit at top"),
        }
    }

    #[test]
    fn test_planner_write_pipeline() {
        let mut catalog = mock_catalog();
        let analyzer = Analyzer::new(&mut catalog);
        let planner = Planner::new();

        use LogicalPlan::*;

        // ---------------- INSERT ----------------

        let insert_stmt = parse(
            "INSERT INTO users (id, age, score) \
         SELECT u.id, u.age, COUNT(*) \
         FROM users u \
         INNER JOIN orders o ON u.id = o.user_id \
         WHERE u.active = true \
         GROUP BY u.id, u.age",
        );

        let analyzed_insert = analyzer.analyze(insert_stmt).unwrap();
        let logical_insert = planner.plan_logical(analyzed_insert);

        match logical_insert {
            Insert {
                table,
                col_idxs,
                input,
            } => {
                assert_eq!(table, "users");
                assert_eq!(col_idxs.len(), 3);

                match *input {
                    Project { input, .. } => match *input {
                        Aggregate { keys, input, .. } => {
                            assert_eq!(keys.len(), 2);

                            match *input {
                                Filter { input, .. } => match *input {
                                    Join { left, right, .. } => match (*left, *right) {
                                        (Scan { table: l }, Scan { table: r }) => {
                                            assert_eq!(l, "users");
                                            assert_eq!(r, "orders");
                                        }
                                        _ => panic!("Expected scans"),
                                    },
                                    _ => panic!("Expected Join"),
                                },
                                _ => panic!("Expected WHERE"),
                            }
                        }
                        _ => panic!("Expected Aggregate"),
                    },
                    _ => panic!("Expected Project"),
                }
            }
            _ => panic!("Expected Insert"),
        }

        // ---------------- UPDATE ----------------

        let update_stmt = parse(
            "UPDATE users SET age = age + 1, score = score + 10 \
         WHERE active = true AND deleted = false",
        );

        let analyzed_update = analyzer.analyze(update_stmt).unwrap();
        let logical_update = planner.plan_logical(analyzed_update);

        match logical_update {
            Update {
                table,
                assign,
                input,
            } => {
                assert_eq!(table, "users");
                assert_eq!(assign.len(), 2);

                match *input {
                    Filter { input, .. } => match *input {
                        Scan { table } => {
                            assert_eq!(table, "users");
                        }
                        _ => panic!("Expected Scan"),
                    },
                    _ => panic!("Expected Filter"),
                }
            }
            _ => panic!("Expected Update"),
        }

        // ---------------- DELETE ----------------

        let delete_stmt = parse("DELETE FROM users WHERE age < 18 OR banned > 0");

        let analyzed_delete = analyzer.analyze(delete_stmt).unwrap();
        let logical_delete = planner.plan_logical(analyzed_delete);

        match logical_delete {
            Delete { table, input } => {
                assert_eq!(table, "users");

                match *input {
                    Filter { input, .. } => match *input {
                        Scan { table } => {
                            assert_eq!(table, "users");
                        }
                        _ => panic!("Expected Scan"),
                    },
                    _ => panic!("Expected Filter"),
                }
            }
            _ => panic!("Expected Delete"),
        }
    }
}
