#![allow(dead_code)]
use std::fmt;

use crate::{
    analyzer::{
        resolved::{RExpr, RJoin, RJoinConstraint, RSelect, RStmt, RTableRef},
        scope::{Scope, ScopeError},
    },
    catalog::{Table, catalog::Catalog},
    sql::ast::{Expr, JoinClause, JoinConstraint, SelectStmt, Stmt, TableRef},
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
        let from = stmt
            .from
            .map(|tr| self.add_to_scope(tr, &mut scope))
            .transpose()?;

        let joins = stmt.joins.into_iter().map(|j| self.analyze_join(j, &mut scope)).collect()::<Result<Vec<_>>>()?;

        todo!()
    }

    pub fn analyze_join(&self, j: JoinClause, scope: &mut Scope<'_, 'c>) -> Result<RJoin> {
        let table = self.add_to_scope(j.table, scope)?; 
        let constraint = match j.constraint {
        JoinConstraint::On(e) => RJoinConstraint::On(self.analyze_expr(e, scope)?),
        JoinConstraint::Natural => RJoinConstraint::Natural,
        JoinConstraint::Using(v) => RJoinConstraint::Using(v.into_iter().map(|i| i.0.to_lowercase()).collect()),
    };

    Ok(RJoin { kind: j.kind, table, constraint })
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
