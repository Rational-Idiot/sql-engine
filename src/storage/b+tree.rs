// Implement a B+ Link Tree as proposed by this lecture - https://www.youtube.com/watch?v=u7ii_Lvm9rM

use core::fmt;
use std::io;

use crate::{
    catalog::Column,
    storage::{
        disk::DiskManager,
        node::{ColValue, LeafNode, LeafVal, StorageError},
        page::NULL_PAGE,
    },
};

pub struct BTree<'a> {
    disk: &'a mut DiskManager,
    schema: &'a [Column],
}

#[derive(Debug)]
pub enum BTreeError {
    InvalidInput(usize, usize),
    StorageError(StorageError),
    IOError(io::Error),
}

impl fmt::Display for BTreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BTreeError::InvalidInput(e, g) => {
                write!(f, "Expected buffer to be {e} long but is only {g} long")
            }
            BTreeError::StorageError(e) => write!(f, "{e}"),
            BTreeError::IOError(e) => write!(f, "{e}"),
        }
    }
}

impl<'a> BTree<'a> {
    pub fn new(disk: &'a mut DiskManager, schema: &'a [Column]) -> Self {
        Self { disk, schema }
    }

    pub fn insert(&mut self, values: Vec<ColValue>) -> Result<u64, BTreeError> {
        if values.len() > self.schema.len() {
            return Err(BTreeError::InvalidInput(values.len(), self.schema.len()));
        }

        let row = self.disk.next_row_id();
        let entry = LeafVal::new(row, values);

        let new_root = if self.disk.commit_root == NULL_PAGE {
            // Inserts the first page in a tree
            let mut leaf = LeafNode::new();

            leaf.insert(entry, self.schema)
                .map_err(BTreeError::StorageError)?;

            let id = self.disk.allocate().map_err(BTreeError::IOError)?;

            self.disk
                .write_page(
                    id,
                    &leaf
                        .serialize(self.schema)
                        .map_err(BTreeError::StorageError)?,
                )
                .map_err(BTreeError::IOError)?;

            id
        } else {
            // Recursively insert by finding the correct point
            let root = self.disk.commit_root;
            todo!()
        };

        todo!()
    }
}
