#![allow(dead_code)]
use core::fmt;
use std::collections::HashMap;

use crate::ast::DataType;

#[derive(Debug, PartialEq)]
pub enum CatalogError {
    TableNotFound(String),
    TableAlreadyExiste(String),
    NoColumns(String),
    DuplicateColumn(String, String),
}

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TableNotFound(t) => write!(f, "Table '{t}' does not exists"),
            Self::TableAlreadyExiste(t) => write!(f, "Table '{t}' already not exists"),
            Self::NoColumns(t) => write!(f, "Table '{t}' has no columns"),
            Self::DuplicateColumn(t, c) => write!(f, "Table '{t}' already has column '{c}'"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Column {
    pub id: usize,
    pub name: String,

    // pre-computed for fast case insensistive lookup
    pub name_lower: String,

    pub data_type: DataType,
    pub nullable: bool,
    pub primary_key: bool,
    pub unique: bool,
}

#[derive(Debug, Clone)]
pub struct Table {
    pub name: String,
    cols: Vec<Column>,
}

impl Table {
    pub fn column(&self, name_lower: &str) -> Option<&Column> {
        self.cols.iter().find(|c| c.name_lower == name_lower)
    }

    pub fn primary_key(&self) -> Option<Vec<&Column>> {
        let res: Vec<&Column> = self.cols.iter().filter(|c| c.primary_key).collect();
        if res.is_empty() { None } else { Some(res) }
    }
}

#[derive(Debug)]
pub struct Catalog {
    tables: HashMap<String, Table>, // The key is name_lower
}

impl Catalog {
    pub fn new() -> Self {
        Self {
            tables: HashMap::new(),
        }
    }

    pub fn table(&self, name_lower: &str) -> Option<&Table> {
        self.tables.get(name_lower)
    }

    pub fn exists(&self, name_lower: &str) -> bool {
        self.tables.contains_key(name_lower)
    }

    pub fn table_names(&self) -> impl Iterator<Item = &str> {
        self.tables.keys().map(String::as_str)
    }
}
