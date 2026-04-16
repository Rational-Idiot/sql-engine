#![allow(dead_code)]
use std::fmt;

use crate::{
    catalog::{Column, Table},
    sql::ast::DataType,
};

#[derive(Debug, Clone)]
pub struct ColRef {
    pub table_name: String,
    pub table_alias: String,
    pub col_name: String,
    pub col_idx: usize,
    pub data_type: DataType,
    pub nullable: bool,
}

#[derive(Debug)]
pub enum ScopeError {
    UnknownTable(String),
    UnknownColumn(String),
    UnknownQualifiedColumn(String, String), // table, col
    DuplicateAlias(String),
    AmbiguousColumn(String, String), // col, (String of table aliases)
}

impl std::error::Error for ScopeError {}
type Result<T> = std::result::Result<T, ScopeError>;

impl fmt::Display for ScopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScopeError::UnknownTable(t) => write!(f, "Unkown table or alias '{t}'"),
            ScopeError::UnknownColumn(c) => write!(f, "Unkown column '{c}'"),
            ScopeError::UnknownQualifiedColumn(c, t) => {
                write!(f, "Unkown Qualified column '{c}' for table '{t}'")
            }
            ScopeError::DuplicateAlias(a) => {
                write!(f, "alias '{a}' has already been used in this clause")
            }

            ScopeError::AmbiguousColumn(c, t) => {
                write!(f, "Column '{c}' is ambiguous, found in: {t}")
            }
        }
    }
}

struct Entry<'a> {
    alias: String,
    alias_lower: String,
    table: &'a Table,
}

pub struct Scope<'parent, 'catalog> {
    entries: Vec<Entry<'catalog>>,
    parent: Option<&'parent Scope<'parent, 'catalog>>,
}

impl<'parent, 'catalog> Scope<'parent, 'catalog> {
    pub fn new() -> Self {
        Scope {
            entries: Vec::new(),
            parent: None,
        }
    }

    pub fn add(&mut self, alias: String, table: &'catalog Table) -> Result<()> {
        let alias_lower = alias.to_ascii_lowercase();

        if self.entries.iter().any(|e| e.alias_lower == alias_lower) {
            return Err(ScopeError::DuplicateAlias(alias_lower));
        }

        self.entries.push(Entry {
            alias,
            alias_lower,
            table,
        });
        Ok(())
    }

    pub fn with_parent(parent: &'parent Scope<'parent, 'catalog>) -> Self {
        Scope {
            entries: Vec::new(),
            parent: Some(parent),
        }
    }

    fn make_ref(alias: &str, table: &Table, col: &Column) -> ColRef {
        ColRef {
            table_name: table.name.clone(),
            table_alias: alias.to_string(),
            col_name: col.name.clone(),
            col_idx: col.id,
            data_type: col.data_type.clone(),
            nullable: col.nullable,
        }
    }

    pub fn resolve_col(&self, col_lower: &str) -> Result<ColRef> {
        let mut found: Vec<ColRef> = Vec::new();

        for entry in &self.entries {
            if let Some(c) = entry.table.find_column(col_lower) {
                found.push(Self::make_ref(&entry.alias, entry.table, c));
            }
        }

        if found.is_empty() {
            return match self.parent {
                Some(p) => p.resolve_col(col_lower),
                None => Err(ScopeError::UnknownColumn(col_lower.to_string())),
            };
        }

        if found.len() > 1 {
            let tables = found
                .iter()
                .map(|t| t.table_alias.as_str())
                .collect::<Vec<_>>()
                .join(", ");

            return Err(ScopeError::AmbiguousColumn(col_lower.to_string(), tables));
        }

        return Ok(found.into_iter().next().unwrap()); // Return without moving
    }

    // Find the home for columns like table.column
    pub fn resolve_qualified(&self, table_lower: &str, col_lower: &str) -> Result<ColRef> {
        let entry = self
            .entries
            .iter()
            .find(|e| e.alias_lower == table_lower || e.table.name_lower == table_lower);

        match entry {
            Some(e) => {
                let c = e.table.find_column(col_lower).ok_or_else(|| {
                    ScopeError::UnknownQualifiedColumn(e.alias_lower.clone(), col_lower.to_string())
                })?;

                Ok(Self::make_ref(&e.alias, e.table, c))
            }
            None => match self.parent {
                Some(p) => p.resolve_qualified(table_lower, col_lower),
                None => Err(ScopeError::UnknownTable(table_lower.to_string())),
            },
        }
    }

    // make SELECT * return all columns
    pub fn resolve_star(&self) -> Vec<ColRef> {
        self.entries
            .iter()
            .flat_map(|e| {
                e.table
                    .cols
                    .iter()
                    .map(|c| Self::make_ref(&e.alias, e.table, c))
            })
            .collect()
    }

    pub fn resolve_table_star(&self, alias_lower: &str) -> Option<Vec<ColRef>> {
        let entry = self
            .entries
            .iter()
            .find(|e| e.alias_lower == alias_lower || e.table.name_lower == alias_lower)?;

        Some(
            entry
                .table
                .cols
                .iter()
                .map(|c| Self::make_ref(&entry.alias, entry.table, c))
                .collect(),
        )
    }
}

// ThankGPT
#[test]
fn integration_scope_with_catalog_and_constraints() {
    use crate::catalog::catalog::*;
    use crate::sql::ast::*;

    let mut catalog = Catalog::new();

    // CREATE TABLE users
    catalog
        .create_table(CreateTableStmt {
            name: Ident("users".into()),
            flag: false,
            columns: vec![
                ColumnDef {
                    name: Ident("id".into()),
                    data_type: DataType::Integer,
                    constraints: vec![ColumnConstraint::PrimaryKey],
                },
                ColumnDef {
                    name: Ident("email".into()),
                    data_type: DataType::Integer,
                    constraints: vec![ColumnConstraint::Unique],
                },
                ColumnDef {
                    name: Ident("name".into()),
                    data_type: DataType::Integer,
                    constraints: vec![],
                },
            ],
        })
        .unwrap();

    // CREATE TABLE orders
    catalog
        .create_table(CreateTableStmt {
            name: Ident("orders".into()),
            flag: false,
            columns: vec![
                ColumnDef {
                    name: Ident("id".into()),
                    data_type: DataType::Integer,
                    constraints: vec![ColumnConstraint::PrimaryKey],
                },
                ColumnDef {
                    name: Ident("user_id".into()),
                    data_type: DataType::Integer,
                    constraints: vec![ColumnConstraint::NotNull],
                },
            ],
        })
        .unwrap();

    let users = catalog.table("users").unwrap();
    let orders = catalog.table("orders").unwrap();

    let mut scope = Scope::new();
    scope.add("u".into(), users).unwrap();
    scope.add("o".into(), orders).unwrap();

    // ✅ Ambiguous column
    let err = scope.resolve_col("id").unwrap_err();
    match err {
        ScopeError::AmbiguousColumn(col, tables) => {
            assert_eq!(col, "id");
            assert!(tables.contains("u"));
            assert!(tables.contains("o"));
        }
        _ => panic!("Expected ambiguity"),
    }

    // ✅ Qualified works
    let col = scope.resolve_qualified("u", "id").unwrap();
    assert_eq!(col.table_alias, "u");
    assert_eq!(col.nullable, false); // PK → not nullable
    assert!(col.data_type == DataType::Integer);

    // ✅ Unique column still nullable unless NOT NULL
    let email = scope.resolve_qualified("users", "email").unwrap();
    assert_eq!(email.nullable, true);

    // ✅ NOT NULL propagation
    let user_id = scope.resolve_qualified("o", "user_id").unwrap();
    assert_eq!(user_id.nullable, false);

    // ✅ Star expansion
    let star = scope.resolve_star();
    assert_eq!(star.len(), 5); // 3 + 2 columns

    // Order matters → insertion order
    assert_eq!(star[0].table_alias, "u");
    assert_eq!(star[3].table_alias, "o");
}
