#![allow(dead_code)]
use std::fmt;

use crate::{
    analyzer::{
        resolved::{
            FnKind, RExpr, RJoin, RJoinConstraint, RSelect, RSelectItem, RStmt, RTableRef, Ty,
        },
        scope::{Scope, ScopeError},
    },
    catalog::catalog::Catalog,
    sql::ast::{
        DataType, Expr, JoinClause, JoinConstraint, SelectItem, SelectStmt, Stmt, TableRef,
    },
};

#[derive(Debug)]
pub enum AnalyzerError {
    UnknownType(String),
    TableNotFound(String),
    Scope(ScopeError),
}

impl fmt::Display for AnalyzerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnalyzerError::UnknownType(t) => write!(f, "Unkown Type: '{t}'"),
            AnalyzerError::TableNotFound(t) => write!(f, "table '{t}' not found"),
            AnalyzerError::Scope(e) => write!(f, "ScopeError: '{e}'"),
        }
    }
}

impl std::error::Error for AnalyzerError {}
type Result<T> = std::result::Result<T, AnalyzerError>;

pub struct Analyzer<'c> {
    catalog: &'c Catalog,
}

fn dt_to_ty(dt: &DataType) -> Ty {
    match dt {
        DataType::Float => Ty::Float,
        DataType::Integer => Ty::Int,
        DataType::String => Ty::Text,
        DataType::Bool => Ty::Bool,
    }
}

fn auto_label(e: &RExpr) -> String {
    match e {
        RExpr::Column(cr, _) => cr.col_name.clone(),
        RExpr::Literal(lit, _) => format!("{lit}"),
        RExpr::Function(c) => c.name.clone(),
        RExpr::Cast { .. } => "cast".into(),
        _ => "?column?".into(),
    }
}

impl<'c> Analyzer<'c> {
    pub fn new(catalog: &'c mut Catalog) -> Self {
        Self { catalog }
    }

    pub fn analyze(&self, stmt: Stmt) -> Result<RStmt> {
        match stmt {
            Stmt::Select(s) => Ok(RStmt::Select(self.analyze_select(s)?)),
            _ => todo!(),
        }
    }

    pub fn analyze_select(&self, stmt: SelectStmt) -> Result<RSelect> {
        let mut scope = Scope::new();
        let from = match stmt.from {
            Some(tr) => Some(self.add_to_scope(tr, &mut scope)?),
            None => None,
        };

        let where_clause = match stmt.where_clause {
            Some(e) => Some(self.analyze_expr(e, &scope)?),
            None => None,
        };

        let having = match stmt.having {
            Some(e) => Some(self.analyze_expr(e, &scope)?),
            None => None,
        };

        let joins = stmt
            .joins
            .into_iter()
            .map(|j| self.analyze_join(j, &mut scope))
            .collect::<Result<Vec<_>>>()?;

        let group_by = stmt
            .group_by
            .into_iter()
            .map(|e| self.analyze_expr(e, &scope))
            .collect::<Result<Vec<_>>>()?;

        let cols = stmt
            .col
            .iter()
            .map(|i| self.expand_select_item(i, &scope))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        if !group_by.is_empty() {
            for item in &cols {
                let flag = match &item.expr {
                    RExpr::Literal(_, _) => true,
                    RExpr::Function(c) => matches!(c.kind, FnKind::Aggregate),
                    _ => false,
                };

                if !flag {
                    let cov = group_by.iter().any(|g| {

                    };
                    if !cov {
                        return Err(AnalyzerError);
                    }
                }
            }
        }

        todo!()
    }

    pub fn expand_select_item(&self, item: &SelectItem, scope: &Scope) -> Result<Vec<RSelectItem>> {
        match &item.expr {
            Expr::Glob => {
                let cols = scope
                    .resolve_star()
                    .into_iter()
                    .map(|cr| {
                        let ty = dt_to_ty(&cr.data_type);
                        let label = cr.col_name.clone();

                        RSelectItem {
                            expr: RExpr::Column(cr, ty),
                            label,
                        }
                    })
                    .collect();

                Ok(cols)
            }

            Expr::QualifiedGlob(id) => {
                let alias_lower = id.0.to_lowercase();
                let cols = scope
                    .resolve_table_star(&alias_lower)
                    .ok_or_else(|| AnalyzerError::Scope(ScopeError::UnknownTable(id.0.clone())))?;

                let cols = cols
                    .into_iter()
                    .map(|cr| {
                        let ty = dt_to_ty(&cr.data_type);
                        let label = cr.col_name.clone();

                        RSelectItem {
                            expr: RExpr::Column(cr, ty),
                            label,
                        }
                    })
                    .collect();

                Ok(cols)
            }

            expr => {
                let rexpr = self.analyze_expr(expr.clone(), scope)?;
                let label = item
                    .alias
                    .as_ref()
                    .map(|a| a.0.clone())
                    .unwrap_or_else(|| auto_label(&rexpr));

                Ok(vec![RSelectItem { expr: rexpr, label }])
            }
        }
    }

    pub fn analyze_join(&self, j: JoinClause, scope: &mut Scope<'_, 'c>) -> Result<RJoin> {
        let table = self.add_to_scope(j.table, scope)?;
        let constraint = match j.constraint {
            JoinConstraint::On(e) => RJoinConstraint::On(self.analyze_expr(e, scope)?),
            JoinConstraint::Natural => RJoinConstraint::Natural,
            JoinConstraint::Using(v) => {
                RJoinConstraint::Using(v.into_iter().map(|i| i.0.to_lowercase()).collect())
            }
        };

        Ok(RJoin {
            kind: j.kind,
            table,
            constraint,
        })
    }

    pub fn analyze_expr(&self, e: Expr, scope: &Scope<'_, '_>) -> Result<RExpr> {
        todo!()
    }

    pub fn add_to_scope(
        &self,
        table_ref: TableRef,
        scope: &mut Scope<'_, 'c>,
    ) -> Result<RTableRef> {
        match table_ref {
            TableRef::Named { name, alias } => {
                let name_lower = name.0.to_lowercase();
                let alias_lower = alias
                    .as_ref()
                    .map(|a| a.0.to_lowercase())
                    .unwrap_or_else(|| name_lower.clone());
                let table = self
                    .catalog
                    .table(&name_lower)
                    .ok_or_else(|| AnalyzerError::TableNotFound(name.0.clone()))?;
                scope
                    .add(alias_lower.clone(), table)
                    .map_err(AnalyzerError::Scope)?;

                Ok(RTableRef::Named {
                    table_name: name_lower,
                    alias: Some(alias_lower),
                })
            }

            TableRef::Subquery { query, alias } => {
                let alias_lower = alias.0.to_lowercase();
                let rq = self.analyze_select(*query)?;
                Ok(RTableRef::Subquery {
                    query: Box::new(rq),
                    alias: alias_lower,
                })
            }
        }
    }
}
