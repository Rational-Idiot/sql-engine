#![allow(dead_code)]
use core::fmt;
use std::collections::HashMap;

use crate::ast::{ColumnConstraint, CreateTableStmt, DataType};

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

impl std::error::Error for CatalogError {}
pub type Result<T> = std::result::Result<T, CatalogError>;

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
    pub name_lower: String,
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

    pub fn create_table(&mut self, stmt: CreateTableStmt) -> Result<()> {
        let name_lower = stmt.name.0.to_lowercase();

        let mut cols = Vec::with_capacity(stmt.columns.len());

        for (id, def) in stmt.columns.iter().enumerate() {
            let col_lower = def.name.0.to_lowercase();
            let primary_key = def
                .constraints
                .iter()
                .any(|c| matches!(c, ColumnConstraint::PrimaryKey));
            let unique = primary_key
                || def
                    .constraints
                    .iter()
                    .any(|c| matches!(c, ColumnConstraint::Unique));
            let nullable = !primary_key
                && !def
                    .constraints
                    .iter()
                    .any(|c| matches!(c, ColumnConstraint::NotNull));

            cols.push(Column {
                id,
                name: def.name.0.clone(),
                name_lower: col_lower,
                data_type: def.data_type.clone(),
                nullable,
                primary_key,
                unique,
            });
        }

        self.tables.insert(
            name_lower.clone(),
            Table {
                name: stmt.name.0,
                name_lower,
                cols,
            },
        );
        Ok(())
    }
}
