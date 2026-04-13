#![allow(dead_code)]
use std::fmt;

use crate::{
    catalog::{Column, Table},
    sql::ast::DataType,
};

#[derive(Clone)]
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

    pub fn add(&mut self, alias: String, tbael: &'catalog Table) -> Result<()> {
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

    fn resolve_col(&self, col_lower: &str) -> Result<ColRef> {
        let mut found: Vec<ColRef> = Vec::new();

        for entry in &self.entries {
            if let Some(c) = entry.table.column(col_lower) {
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
