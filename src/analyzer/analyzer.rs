#![allow(dead_code)]
use std::fmt;

use crate::{
    analyzer::{
        resolved::{
            FnKind, RArgs, RCall, RDelete, RExpr, RInsert, RInsertSource, RJoin, RJoinConstraint,
            ROrder, RSelect, RSelectItem, RStmt, RTableRef, RUpdate, Ty,
        },
        scope::{Scope, ScopeError},
    },
    catalog::catalog::Catalog,
    sql::ast::{
        self, Args, BinaryOp, Call, DataType, DeleteStmt, Expr, InsertSource, InsertStmt,
        JoinClause, JoinConstraint, Literal, Order, SelectItem, SelectStmt, Stmt, TableRef,
        UnaryOp, UpdateStmt,
    },
};

#[derive(Debug)]
pub enum AnalyzerError {
    UnknownType(String),
    UnknownFunction(String),
    TableNotFound(String),
    ColumnNotFound { table: String, col: String },
    Scope(ScopeError),
    NonAggregateInSelect(String),
    AggNotAllowed(String),
    GlobNotAllowed,
    CannotUnify(Ty, Ty),
    StarArgNotAllowed(String),
    ColumnMismatch { expected: usize, got: usize },
    PrimaryKeyUpdate(String),
}

impl fmt::Display for AnalyzerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnalyzerError::UnknownType(t) => write!(f, "Unkown Type: '{t}'"),
            AnalyzerError::UnknownFunction(t) => write!(f, "Unkown Type: '{t}'"),
            AnalyzerError::TableNotFound(t) => write!(f, "table '{t}' not found"),
            AnalyzerError::ColumnNotFound { table: t, col: c } => {
                write!(f, "Column '{c}' not fouond in table '{t}'")
            }
            AnalyzerError::CannotUnify(t1, t2) => {
                write!(f, "Types '{t1}' and '{t2}' cannnot be unified")
            }
            AnalyzerError::GlobNotAllowed => write!(f, "Globbing (*) is not allowed in these Expr"),
            AnalyzerError::Scope(e) => write!(f, "ScopeError: '{e}'"),
            AnalyzerError::NonAggregateInSelect(label) => {
                write!(
                    f,
                    "column '{label}' must appear in GROUP BY or be used inside an aggregate"
                )
            }
            AnalyzerError::AggNotAllowed(a) => {
                write!(f, "aggregate function '{a}' is not allowed here")
            }
            AnalyzerError::StarArgNotAllowed(n) => {
                write!(f, "'{n}' does not accept a star (*) argument")
            }
            AnalyzerError::ColumnMismatch { expected, got } => {
                write!(
                    f,
                    "column count mismatch: expected {expected} value(s), got {got}"
                )
            }
            AnalyzerError::PrimaryKeyUpdate(col) => {
                write!(f, "cannot update primary key column '{col}'")
            }
        }
    }
}

impl std::error::Error for AnalyzerError {}
type Result<T> = std::result::Result<T, AnalyzerError>;

fn lookup_function(name: &str) -> Option<(FnKind, fn(&[Ty]) -> Ty)> {
    match name {
        "count" => Some((FnKind::Aggregate, |_| Ty::Int)),
        "sum" => Some((FnKind::Aggregate, |a| {
            a.first().cloned().unwrap_or(Ty::Null)
        })),
        "avg" => Some((FnKind::Aggregate, |_| Ty::Float)),
        "min" | "max" => Some((FnKind::Aggregate, |a| {
            a.first().cloned().unwrap_or(Ty::Null)
        })),

        "upper" | "lower" => Some((FnKind::Scalar, |_| Ty::Text)),
        "length" => Some((FnKind::Scalar, |_| Ty::Int)),
        "abs" => Some((FnKind::Scalar, |a| a.first().cloned().unwrap_or(Ty::Null))),

        _ => None,
    }
}

#[derive(Clone, Copy)]
pub struct Analyzer<'c> {
    catalog: &'c Catalog,
    allow_agg: bool,
}

fn check_assignable(got: Ty, expected: &Ty) -> Result<()> {
    if matches!(got, Ty::Null) {
        return Ok(());
    }
    Ty::unify(&got, expected).ok_or_else(|| AnalyzerError::CannotUnify(got, expected.clone()))?;
    Ok(())
}

fn dt_to_ty(dt: &DataType) -> Ty {
    match dt {
        DataType::Float => Ty::Float,
        DataType::Integer => Ty::Int,
        DataType::String => Ty::Text,
        DataType::Bool => Ty::Bool,
    }
}

fn lit_ty(lit: &Literal) -> Ty {
    match lit {
        Literal::Null => Ty::Null,
        Literal::Number(s) => {
            if s.contains('.') {
                Ty::Float
            } else {
                Ty::Int
            }
        }
        Literal::Bool(_) => Ty::Bool,
        Literal::String(_) => Ty::Text,
    }
}

fn rexpr_matches(sel: &RExpr, key: &RExpr) -> bool {
    match (sel, key) {
        (RExpr::Column(a, _), RExpr::Column(b, _)) => {
            a.table_name == b.table_name && a.col_idx == b.col_idx
        }
        _ => false,
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
        Self {
            catalog,
            allow_agg: true,
        }
    }

    fn forbid_agg(self) -> Self {
        Self {
            allow_agg: false,
            ..self
        }
    }

    pub fn analyze(&self, stmt: Stmt) -> Result<RStmt> {
        match stmt {
            Stmt::Select(s) => Ok(RStmt::Select(self.analyze_select(s)?)),
            Stmt::Insert(s) => Ok(RStmt::Insert(self.analyze_insert(s)?)),
            Stmt::Update(s) => Ok(RStmt::Update(self.analyze_update(s)?)),
            _ => todo!(),
        }
    }

    pub fn analyze_select(&self, stmt: SelectStmt) -> Result<RSelect> {
        let mut scope = Scope::new();

        let from = match stmt.from {
            Some(tr) => Some(self.add_to_scope(tr, &mut scope)?),
            None => None,
        };

        let where_clause = stmt
            .where_clause
            .map(|e| self.forbid_agg().analyze_expr(e, &scope))
            .transpose()?;

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
            .map(|e| self.forbid_agg().analyze_expr(e, &scope))
            .collect::<Result<Vec<_>>>()?;

        let col = stmt
            .col
            .iter()
            .map(|i| self.expand_select_item(i, &scope))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        if !group_by.is_empty() {
            for item in &col {
                let flag = match &item.expr {
                    RExpr::Literal(_, _) => true,
                    RExpr::Function(c) => matches!(c.kind, FnKind::Aggregate),
                    _ => false,
                };

                if !flag {
                    let cov = group_by.iter().any(|g| rexpr_matches(&item.expr, g));
                    if !cov {
                        return Err(AnalyzerError::NonAggregateInSelect(item.label.clone()));
                    }
                }
            }
        }

        let order_by = stmt
            .order_by
            .into_iter()
            .map(|o| {
                let expr = self.analyze_expr(o.expr, &scope)?;
                Ok(ROrder { expr, dir: o.dir })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(RSelect {
            quantifier: stmt.quantifier,
            col,
            from,
            joins,
            where_clause,
            group_by,
            having,
            order_by,
            limit: stmt.limit,
            offset: stmt.offset,
        })
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

    pub fn analyze_insert(&self, s: InsertStmt) -> Result<RInsert> {
        let name_lower = s.table.0.to_lowercase();
        let table = self
            .catalog
            .table(&name_lower)
            .ok_or_else(|| AnalyzerError::TableNotFound(s.table.0))?;

        let coli = if s.columns.is_empty() {
            table.cols.iter().map(|c| c.id).collect()
        } else {
            s.columns
                .iter()
                .map(|i| {
                    let lower = i.0.to_lowercase();
                    table.find_column(&lower).map(|c| c.id).ok_or_else(|| {
                        AnalyzerError::ColumnNotFound {
                            table: name_lower.clone(),
                            col: lower,
                        }
                    })
                })
                .collect::<Result<Vec<_>>>()?
        };

        let es = Scope::new();
        let source = match s.source {
            InsertSource::Values(vr) => {
                let res = vr
                    .into_iter()
                    .map(|r| {
                        if r.len() != coli.len() {
                            return Err(AnalyzerError::ColumnMismatch {
                                expected: coli.len(),
                                got: r.len(),
                            });
                        }

                        r.into_iter()
                            .zip(coli.iter())
                            .map(|(e, &i)| {
                                let rexpr = self.analyze_expr(e, &es)?;
                                let ex = dt_to_ty(&table.cols[i].data_type);
                                check_assignable(rexpr.ty(), &ex)?;
                                Ok(rexpr)
                            })
                            .collect::<Result<Vec<_>>>()
                    })
                    .collect::<Result<Vec<_>>>()?;

                RInsertSource::Values(res)
            }

            InsertSource::Select(s) => RInsertSource::Select(Box::new(self.analyze_select(*s)?)),
        };

        Ok(RInsert {
            table: name_lower,
            col_idx: coli,
            source,
        })
    }

    pub fn analyze_update(&self, s: UpdateStmt) -> Result<RUpdate> {
        let mut scope = Scope::new();
        let where_clause = s
            .where_clause
            .map(|e| self.analyze_expr(e, &scope))
            .transpose()?;

        let rtable = self.add_to_scope(s.table, &mut scope)?;
        let name_lwoer = match &rtable {
            RTableRef::Named { table_name, .. } => table_name.clone(),
            RTableRef::Subquery { .. } => {
                return Err(AnalyzerError::TableNotFound(
                    "(subquery is not a valid UPDATE target)".into(),
                ));
            }
        };

        let table = self
            .catalog
            .table(&name_lwoer)
            .ok_or_else(|| AnalyzerError::TableNotFound(name_lwoer.clone()))?;

        let assign =
            s.assign
                .into_iter()
                .map(|a| {
                    let col_lower = a.column.0.to_lowercase();
                    let col = table.find_column(&col_lower).ok_or_else(|| {
                        AnalyzerError::ColumnNotFound {
                            table: name_lwoer.clone(),
                            col: col_lower.clone(),
                        }
                    })?;

                    if col.primary_key {
                        return Err(AnalyzerError::PrimaryKeyUpdate(col_lower));
                    }

                    let rexpr = self.analyze_expr(a.value, &scope)?;
                    check_assignable(rexpr.ty(), &dt_to_ty(&col.data_type))?;
                    Ok((col.id, rexpr))
                })
                .collect::<Result<Vec<_>>>()?;

        Ok(RUpdate {
            table: rtable,
            assign,
            where_clause,
        })
    }

    pub fn analyze_delete(&self, stmt: DeleteStmt) -> Result<RDelete> {
        let mut scope = Scope::new();
        let rtable = self.add_to_scope(stmt.table, &mut scope)?;

        let where_clause = stmt
            .where_clause
            .map(|e| self.analyze_expr(e, &scope))
            .transpose()?;

        Ok(RDelete {
            table: rtable,
            where_clause,
        })
    }

    pub fn analyze_expr(&self, e: Expr, scope: &Scope<'_, '_>) -> Result<RExpr> {
        match e {
            Expr::Literal(l) => {
                let ty = lit_ty(&l);
                Ok(RExpr::Literal(l, ty))
            }

            Expr::Identifier(ident) => {
                let s = ident.0.to_lowercase();
                let cr = if let Some(dot) = s.find('.') {
                    scope
                        .resolve_qualified(&s[..dot], &s[dot + 1..])
                        .map_err(AnalyzerError::Scope)?
                } else {
                    scope.resolve_col(&s).map_err(AnalyzerError::Scope)?
                };
                let ty = dt_to_ty(&cr.data_type);
                Ok(RExpr::Column(cr, ty))
            }

            Expr::Glob | Expr::QualifiedGlob(_) => Err(AnalyzerError::GlobNotAllowed),

            Expr::BinaryOp { left, op, right } => {
                let rleft = self.analyze_expr(*left, scope)?;
                let rright = self.analyze_expr(*right, scope)?;

                let ty = match op {
                    BinaryOp::Eq
                    | BinaryOp::Ne
                    | BinaryOp::Gt
                    | BinaryOp::Ge
                    | BinaryOp::Lt
                    | BinaryOp::Le => {
                        Ty::unify(&rleft.ty(), &rright.ty())
                            .ok_or_else(|| AnalyzerError::CannotUnify(rleft.ty(), rright.ty()))?;
                        Ty::Bool
                    }

                    BinaryOp::And | BinaryOp::Or => Ty::Bool,

                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Percent => Ty::unify(&rleft.ty(), &rright.ty())
                        .ok_or_else(|| AnalyzerError::CannotUnify(rleft.ty(), rright.ty()))?,
                };

                Ok(RExpr::BinaryOp {
                    op,
                    lhs: Box::new(rleft),
                    rhs: Box::new(rright),
                    ty,
                })
            }

            Expr::UnaryOp { op, expr } => {
                let rexpr = self.analyze_expr(*expr, scope)?;
                let ty = match op {
                    UnaryOp::Not => Ty::Bool,
                    UnaryOp::Neg => rexpr.ty(),
                };

                Ok(RExpr::UnaryOp {
                    op,
                    expr: Box::new(rexpr),
                    ty,
                })
            }

            Expr::IsNull { expr, neg } => {
                let rexpr = self.analyze_expr(*expr, scope)?;
                Ok(RExpr::IsNull {
                    expr: Box::new(rexpr),
                    neg,
                })
            }

            Expr::Between {
                expr,
                negated,
                low,
                high,
            } => {
                let rexpr = self.analyze_expr(*expr, scope)?;
                let rlow = self.analyze_expr(*low, scope)?;
                let rhigh = self.analyze_expr(*high, scope)?;

                Ty::unify(&rexpr.ty(), &rlow.ty())
                    .ok_or_else(|| AnalyzerError::CannotUnify(rexpr.ty(), rlow.ty()))?;
                Ty::unify(&rexpr.ty(), &rhigh.ty())
                    .ok_or_else(|| AnalyzerError::CannotUnify(rexpr.ty(), rhigh.ty()))?;
                Ok(RExpr::Between {
                    expr: Box::new(rexpr),
                    negated,
                    low: Box::new(rlow),
                    high: Box::new(rhigh),
                })
            }

            Expr::InList { expr, list, neg } => {
                let rexpr = self.analyze_expr(*expr, scope)?;
                let rlist = list
                    .into_iter()
                    .map(|e| self.analyze_expr(e, scope))
                    .collect::<Result<Vec<_>>>()?;

                Ok(RExpr::InList {
                    expr: Box::new(rexpr),
                    list: rlist,
                    neg,
                })
            }

            Expr::InSubquery { expr, query, neg } => {
                let rexpr = self.analyze_expr(*expr, scope)?;
                let rq = self.analyze_select(*query)?;

                Ok(RExpr::InSubquery {
                    expr: Box::new(rexpr),
                    query: Box::new(rq),
                    neg,
                })
            }

            Expr::Like {
                expr,
                pattern,
                neg,
                insensitive,
            } => {
                let rexpr = self.analyze_expr(*expr, scope)?;
                let rpat = self.analyze_expr(*pattern, scope)?;

                Ok(RExpr::Like {
                    expr: Box::new(rexpr),
                    pattern: Box::new(rpat),
                    neg,
                    insensitive,
                })
            }

            Expr::SubQuery(q) => Ok(RExpr::SubQuery(Box::new(self.analyze_select(*q)?))),

            Expr::Exists { query, neg } => Ok(RExpr::Exists {
                query: Box::new(self.analyze_select(*query)?),
                neg,
            }),

            Expr::Function(c) => {
                let name_lower = c.name.0.to_lowercase();
                let (kind, return_ty_fn) = lookup_function(&name_lower)
                    .ok_or_else(|| AnalyzerError::UnknownFunction(name_lower.clone()))?;

                if matches!(kind, FnKind::Aggregate) && !self.allow_agg {
                    return Err(AnalyzerError::AggNotAllowed(name_lower));
                }

                let (rargs, arg_tys) = match c.args {
                    Args::Star => {
                        if name_lower != "count" {
                            return Err(AnalyzerError::StarArgNotAllowed(name_lower));
                        }
                        (RArgs::Star, vec![])
                    }

                    Args::List(v) => {
                        let res = v
                            .into_iter()
                            .map(|e| self.analyze_expr(e, scope))
                            .collect::<Result<Vec<_>>>()?;

                        let tys = res.iter().map(|e| e.ty()).collect::<Vec<_>>();
                        (RArgs::List(res), tys)
                    }
                };

                let return_ty = return_ty_fn(&arg_tys);

                let filter = c
                    .filter
                    .map(|e| self.forbid_agg().analyze_expr(*e, scope))
                    .transpose()?
                    .map(Box::new);

                Ok(RExpr::Function(RCall {
                    name: name_lower,
                    args: rargs,
                    distinct: c.distinct,
                    filter,
                    return_ty,
                    kind,
                }))
            }

            Expr::Cast { expr, data_type } => {
                let rexpr = self.analyze_expr(*expr, scope)?;
                let ty = dt_to_ty(&data_type);
                Ok(RExpr::Cast {
                    expr: Box::new(rexpr),
                    data_type: ty,
                })
            }
        }
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
