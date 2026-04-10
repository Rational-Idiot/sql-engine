#![allow(dead_code)]
use std::fmt;

use crate::{
    catalog::{Column, Table},
    sql::ast::DataType,
};

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
    DuplicateAlias(String),
}

impl std::error::Error for ScopeError {}
type Result<T> = std::result::Result<T, ScopeError>;

impl fmt::Display for ScopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScopeError::UnknownTable(t) => write!(f, "Unkown table or alias '{t}'"),
            ScopeError::UnknownColumn(c) => write!(f, "Unkown column '{c}'"),
            ScopeError::DuplicateAlias(a) => {
                write!(f, "alias '{a}' has already been used in this clause")
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
        let mut found: Vec<Column> = Vec::new();

        todo!()
    }
}
