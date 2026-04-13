#![allow(dead_code)]
use std::fmt;

use crate::{catalog::catalog::Catalog, sql::ast::Stmt};

#[derive(Debug)]
pub enum AnalyzerError {
    UnknownType(String),
}

impl fmt::Display for AnalyzerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnalyzerError::UnknownType(t) => write!(f, "Unkown Type: '{t}'"),
        }
    }
}

impl std::error::Error for AnalyzerError {}
type Result<T> = std::result::Result<T, AnalyzerError>;

pub struct Analyzer<'c> {
    catalog: &'c mut Catalog,
}

impl<'c> Analyzer<'c> {
    pub fn new(catalog: &'c mut Catalog) -> Self {
        Self { catalog }
    }
}
