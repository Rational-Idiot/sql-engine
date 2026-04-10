use std::fmt;

use crate::{catalog::Table, sql::ast::DataType};

pub struct ColRef {
    pub table_name: String,
    pub table_alias: String,
    pub col_name: String,
    pub col_idx: usize,
    pub data_type: DataType,
    pub nullable: bool,
}

pub enum ScopeError {
    UnknownTable(String),
    UnknownColumn(String),
    DuplicateAlias(String),
}

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
