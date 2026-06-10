use core::fmt;
use std::{cmp::Ordering, fmt::Display, str::from_utf8};

use crate::{
    catalog::Column,
    sql::ast::DataType,
    storage::page::{NULL_PAGE, PAGE_SIZE, PageId, Pagetag},
};

pub const KEY_SIZE: usize = 9;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct F64Key(pub f64);

impl Eq for F64Key {}
impl PartialOrd for F64Key {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for F64Key {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

// Key - [tag: 1][payload: 8]
//
// The tag identifies the key variant and determines how the payload is
// interpreted
// Int and Float keys are in little endian
// Bool keys store 0x00 for false or 0x01 for true. The remaining payload
// bytes are zeroed
// Text keys store the first 8 UTF-8 bytes of the string and pad any unused
// bytes with zeros
// Inf is a sentinel key represented by tag 0xFF and has no payload
#[derive(Debug, Clone)]
pub enum Key {
    Int(i64),
    Float(f64),
    Bool(bool),
    Text(String),

    // A sentinel value always greater than real keys with
    Inf,
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Key::Int(a), Key::Int(b)) => a == b,
            (Key::Float(a), Key::Float(b)) => a.total_cmp(b) == Ordering::Equal,
            (Key::Bool(a), Key::Bool(b)) => a == b,
            (Key::Text(a), Key::Text(b)) => a == b,
            (Key::Inf, Key::Inf) => true,
            _ => false,
        }
    }
}

impl Eq for Key {}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Key::Inf, Key::Inf) => Ordering::Equal,
            (_, Key::Inf) => Ordering::Less,
            (Key::Inf, _) => Ordering::Greater,

            (Key::Int(a), Key::Int(b)) => a.cmp(b),
            (Key::Float(a), Key::Float(b)) => a.total_cmp(b),
            (Key::Bool(a), Key::Bool(b)) => a.cmp(b),
            (Key::Text(a), Key::Text(b)) => a.cmp(b),
            _ => panic!("compared keys of different types"),
        }
    }
}

impl Key {
    pub fn serialize(&self) -> [u8; KEY_SIZE] {
        let mut buf = [0u8; KEY_SIZE];
        match self {
            Key::Int(i) => {
                buf[0] = 0;
                buf[1..9].copy_from_slice(&i.to_le_bytes());
            }

            Key::Float(f) => {
                buf[0] = 1;
                buf[1..9].copy_from_slice(&f.to_le_bytes());
            }

            Key::Bool(b) => {
                buf[0] = 2;
                buf[1] = *b as u8;
            }

            Key::Text(s) => {
                buf[0] = 3;
                let bytes = s.as_bytes();
                let n = bytes.len().min(8);
                buf[1..1 + n].copy_from_slice(&bytes[..n]);
            }

            Key::Inf => {
                buf[0] = 0xFF;
                //No Payload for sentinel key
            }
        }
        buf
    }

    pub fn deseriablize(buf: [u8; KEY_SIZE]) -> Result<Self, StorageError> {
        let payload = buf[1..9]
            .try_into()
            .expect("The payload should always be 8 bytes");
        match buf[0] {
            0 => Ok(Key::Int(i64::from_le_bytes(payload))),
            1 => Ok(Key::Float(f64::from_le_bytes(payload))),
            2 => Ok(Key::Bool(buf[1] == 1)),
            3 => {
                let end = payload.iter().position(|&b| b == 0).unwrap_or(8);
                let text = from_utf8(&payload[..end]).map_err(|e| {
                    StorageError::InavlidFormat(format!("invalid UTF-8 in text key: {e}"))
                })?;

                Ok(Key::Text(text.to_owned()))
            }
            0xFF => Ok(Key::Inf),
            t => panic!("Invalid key tag: {t:#x}"),
        }
    }
}

#[derive(Debug)]
pub enum StorageError {
    InavlidFormat(String),
    InvalidCall(String, String),
    AllocationError(String),
    /// Expected, Got
    SizeMismatch(usize, usize),
    WrongTag(Pagetag, Pagetag),
    FullPage,
    DuplicateRow,
    RowNotFound(u64),
}

impl Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::InavlidFormat(s) => write!(f, "Invalid Conversion: {s}"),
            StorageError::AllocationError(s) => write!(f, "Allocation Error: {s}"),
            StorageError::InvalidCall(fun, t) => {
                write!(f, "Called function: {fun} with invalid type: {t}")
            }
            StorageError::SizeMismatch(a, b) => {
                write!(f, "Expected buffer to be {a} long but is only {b} long")
            }
            StorageError::WrongTag(e, t) => {
                write!(f, "Expected the Tag to be {e} but found {t}")
            }
            StorageError::FullPage => write!(f, "The Page is full and cannot hold more entries"),
            StorageError::DuplicateRow => write!(f, "The same row_id already exists"),
            StorageError::RowNotFound(r) => write!(f, "The row {r} does not exist"),
        }
    }
}

// Thanks Claude ;) {Just for the structure BTW}
// Internal Nodes
//  Layout :
//   [0]        tag:       u8
//   [1..3]     key_count: u16
//   [3]        padding:   u8
//   [4 .. 4 + MAX_INTERNAL_KEYS * KEY_SIZE]                         keys
//   [.. end of used child area]    children (PageId u64)
//
// Capacity:  4 + N*9 + (N+1)*8 ≤ 4096
//            17N ≤ 4084  →  N = 240

pub const MAX_INTERNAL_KEYS: usize = 240;
pub const INTERNAL_KEYS_OFF: usize = 4;
pub const INTERNAL_CHILDREN_OFF: usize = INTERNAL_KEYS_OFF + MAX_INTERNAL_KEYS * KEY_SIZE;

pub struct InternalNode {
    pub keys: Vec<Key>,        // len == n
    pub children: Vec<PageId>, // len == n + 1
}

impl InternalNode {
    /// Index of the child that should contain key.
    pub fn find_child(&self, key: &Key) -> usize {
        match self.keys.binary_search(key) {
            Ok(i) => i + 1, // key == keys[i] - go to right subtree
            Err(i) => i,    // key < keys[i] - go to child i
        }
    }

    pub fn serialize(&self) -> [u8; PAGE_SIZE] {
        let mut buf = [0u8; PAGE_SIZE];
        buf[0] = Pagetag::INTERNAL as u8;
        buf[1..3].copy_from_slice(&(self.keys.len() as u16).to_le_bytes());

        for (i, key) in self.keys.iter().enumerate() {
            let off = INTERNAL_KEYS_OFF + i * KEY_SIZE;
            buf[off..off + KEY_SIZE].copy_from_slice(&key.serialize());
        }
        for (i, &child) in self.children.iter().enumerate() {
            let off = INTERNAL_CHILDREN_OFF + i * 8;
            buf[off..off + 8].copy_from_slice(&child.to_le_bytes());
        }
        buf
    }

    pub fn deserialize(buf: &[u8; PAGE_SIZE]) -> Result<Self, StorageError> {
        let n = u16::from_le_bytes(
            buf[1..3]
                .try_into()
                .expect("u16 header field occupies exactly 2 vytes"),
        ) as usize;

        let mut keys = Vec::with_capacity(n);
        for i in 0..n {
            let off = INTERNAL_KEYS_OFF + i * KEY_SIZE;
            keys.push(Key::deseriablize(
                buf[off..off + KEY_SIZE]
                    .try_into()
                    .expect("Should be guaranteed by the architecture"),
            )?);
        }

        let mut children = Vec::with_capacity(n + 1);
        for i in 0..=n {
            let off = INTERNAL_CHILDREN_OFF + i * 8;
            children.push(u64::from_le_bytes(buf[off..off + 8].try_into().unwrap()));
        }
        Ok(InternalNode { keys, children })
    }
}

pub enum ColValue {
    Int(i64),   // 8 Bytes
    Float(f64), // 8 bytes
    Bool(bool), // 1 bytes
    /// Inline the text in a node upto 64 bytes
    /// Excess data is chained with overflow pages
    /// overflow is NULL Page when string < 64 bytes
    Text {
        // 64 bytes inline + 8 byte overfow PageID
        inline: Box<[u8; 64]>,
        len: u8,
        overflow: PageId,
    },
    Null,
}

impl ColValue {
    pub fn serialize(&self, buf: &mut [u8]) {
        match self {
            ColValue::Int(i) => buf[..8].copy_from_slice(&i.to_le_bytes()),
            ColValue::Float(f) => buf[..8].copy_from_slice(&f.to_le_bytes()),
            ColValue::Bool(b) => buf[0] = *b as u8,
            ColValue::Text {
                inline,
                len,
                overflow,
            } => {
                // buf[0..64] - Inline Text
                // buf[64] - Length of the inline text
                // buf[65..72] - 7 bit PageId, which gives us 2^56 - 1 possible pages
                buf[..64].copy_from_slice(inline.as_ref());
                buf[64] = *len;
                // We only store the fist 7 bytes => THe highest byte is always 0 and can be ommitted
                buf[65..72].copy_from_slice(&overflow.to_le_bytes()[..7]);
            }
            ColValue::Null => {
                // IT should be zeroed out by default
            }
        }
    }

    pub fn deserialize(buf: &[u8], dt: &DataType, is_null: bool) -> Result<Self, StorageError> {
        if is_null {
            return Ok(Self::Null);
        }

        match dt {
            DataType::Integer => {
                let arr: [u8; 8] = buf[..8].try_into().map_err(|_| {
                    StorageError::InavlidFormat("Integer column requires 8 bytes".into())
                })?;

                Ok(ColValue::Int(i64::from_le_bytes(arr)))
            }

            DataType::Float => {
                let arr: [u8; 8] = buf[..8].try_into().map_err(|_| {
                    StorageError::InavlidFormat("Float column requires 8 bytes".into())
                })?;

                Ok(ColValue::Float(f64::from_le_bytes(arr)))
            }

            DataType::Bool => Ok(ColValue::Bool(buf[0] == 1)),

            DataType::String => {
                if buf.len() < 64 + 8 {
                    return Err(StorageError::InavlidFormat(
                        "Colvalus::Text - Buffer is too short".into(),
                    ));
                }

                let len = buf[64];
                if len as usize > 64 {
                    return Err(StorageError::InavlidFormat(format!(
                        "Colvalue::TExt- inline length {len} > Maximum inline length of 64 ",
                    )));
                }

                let mut inline = Box::new([0u8; 64]);
                inline.copy_from_slice(&buf[..64]);

                let mut ov = [0u8; 8];
                ov[..7].copy_from_slice(&buf[65..72]);

                // The Null page has the sentinel of all 7 bytes being FF
                // highest byte will always be 00 by design
                let overflow = if PageId::from_le_bytes(ov) == 0x00FF_FFFF_FFFF_FFFF {
                    NULL_PAGE
                } else {
                    PageId::from_le_bytes(ov)
                };

                Ok(ColValue::Text {
                    inline,
                    len,
                    overflow,
                })
            }
        }
    }

    pub fn text_value<F>(&self, mut get_bytes: F) -> Result<String, StorageError>
    where
        F: FnMut(PageId) -> Result<Vec<u8>, StorageError>,
    {
        match self {
            ColValue::Text {
                inline,
                len,
                overflow,
            } => {
                let mut bytes = inline[..*len as usize].to_vec();
                let mut nxt = *overflow;
                while nxt != NULL_PAGE {
                    let page = get_bytes(nxt)?;
                    let (chain_nxt, chunk) = parse_overflow(&page)?;
                    bytes.extend_from_slice(chunk);
                    nxt = chain_nxt;
                }
                String::from_utf8(bytes).map_err(|e| {
                    StorageError::InavlidFormat(format!(
                        "ColValue::text_value : inavlid UTF-8 - {e}"
                    ))
                })
            }

            ColValue::Null => Err(StorageError::InvalidCall(
                "ColValue::text_value".into(),
                "Null".into(),
            )),

            _ => Err(StorageError::InvalidCall(
                "ColValue::text_value".into(),
                "Non-Text".into(),
            )),
        }
    }

    pub fn size(dt: &DataType) -> usize {
        match dt {
            DataType::Integer => 8,
            DataType::Float => 8,
            DataType::Bool => 1,
            DataType::String => 64 + 8,
        }
    }
}

// Overflow Page - [tag: 1][nxt: 8][len: 4][data: till PAGE_SIZE - 13]

pub fn build_overflow_page(next: PageId, data: &[u8]) -> Result<[u8; PAGE_SIZE], StorageError> {
    if data.len() > PAGE_SIZE - 13 {
        return Err(StorageError::InavlidFormat(format!(
            "Overflow Page: data : {} exceeds maximum: {}",
            data.len(),
            PAGE_SIZE - 13
        )));
    }
    let mut page = [0u8; PAGE_SIZE];
    page[0] = Pagetag::OVERFLOW as u8;
    page[1..9].copy_from_slice(&next.to_le_bytes());
    page[9..13].copy_from_slice(&data.len().to_le_bytes());
    page[13..13 + data.len()].copy_from_slice(data);

    Ok(page)
}

/// allocate returns the fresh PageID for the next pages
/// The function returns the first PageID to store in the leaf node and the list of pages that were
/// created (PageID, page_bytes) to be written to disk
pub fn build_overflow_chain<F>(
    data: &[u8],
    mut allocate: F,
) -> Result<(PageId, Vec<(PageId, [u8; PAGE_SIZE])>), StorageError>
where
    F: FnMut() -> Result<PageId, String>,
{
    let chunks: Vec<&[u8]> = data.chunks(PAGE_SIZE - 13).collect();
    if chunks.is_empty() {
        return Err(StorageError::InavlidFormat(
            "Build Overflow Chain: empty data".into(),
        ));
    }

    // Allocate all PageIDs
    let ids: Vec<PageId> = (0..chunks.len())
        .map(|_| allocate())
        .collect::<Result<_, _>>()
        .map_err(StorageError::AllocationError)?;

    let pages: Vec<(PageId, [u8; PAGE_SIZE])> = ids
        .iter()
        .zip(chunks.iter())
        .enumerate()
        .map(|(i, (&id, chunk))| {
            let next = if i + 1 < ids.len() {
                ids[i + 1]
            } else {
                NULL_PAGE
            };

            let page = build_overflow_page(next, chunk)?;
            Ok((id, page))
        })
        .collect::<Result<_, StorageError>>()?;

    Ok((ids[0], pages))
}

pub fn parse_overflow(page: &[u8]) -> Result<(PageId, &[u8]), StorageError> {
    // tag + next (8 bytes) + len (4 bytes)
    if page.len() < 1 + 8 + 4 {
        return Err(StorageError::InavlidFormat(
            "Overflow page too short".into(),
        ));
    }

    if page[0] != Pagetag::OVERFLOW as u8 {
        return Err(StorageError::InavlidFormat(format!(
            "Overflow Page: bad tag 0x{:02X}",
            page[0]
        )));
    }

    let nxt = PageId::from_le_bytes(page[1..9].try_into().unwrap());
    let len = u32::from_le_bytes(page[9..13].try_into().unwrap()) as usize;
    if page.len() < 13 + len {
        return Err(StorageError::InavlidFormat(format!(
            "Overflow page: len {len} is greater than page boundary"
        )));
    }

    Ok((nxt, &page[13..13 + len]))
}

// Leaval - [tombstone: 1][row_id: 8] then each column in order - if nullable [null flag: 1]
// 0x00 = live, 0x01 = null
// [col values]
pub struct LeafVal {
    pub tombstone: bool,
    pub row_id: u64,
    pub values: Vec<ColValue>,
}

impl LeafVal {
    pub fn size(schema: &[Column]) -> usize {
        // tombstone + row_id
        let mut sz = 1 + 8;
        for col in schema {
            if col.nullable {
                sz += 1;
            }

            sz += ColValue::size(&col.data_type);
        }

        sz
    }

    pub fn serialize(&self, buf: &mut [u8], schema: &[Column]) -> Result<(), StorageError> {
        //buf must be exactly the size of schema
        if self.values.len() != schema.len() {
            return Err(StorageError::SizeMismatch(self.values.len(), schema.len()));
        }

        let expected = Self::size(schema);
        if buf.len() != expected {
            return Err(StorageError::SizeMismatch(expected, buf.len()));
        }

        // Zero out hte buffers
        buf.fill(0);

        let mut off = 0;
        buf[off] = self.tombstone as u8;
        off += 1;

        buf[off..off + 8].copy_from_slice(&self.row_id.to_le_bytes());
        off += 8;

        for (col, val) in schema.iter().zip(self.values.iter()) {
            let sz = ColValue::size(&col.data_type);
            if col.nullable {
                buf[off] = matches!(val, ColValue::Null) as u8;
                off += 1;
            }

            val.serialize(&mut buf[off..off + sz]);
            off += sz;
        }

        Ok(())
    }

    pub fn deserialize(buf: &[u8], schema: &[Column]) -> Result<Self, StorageError> {
        let expected = Self::size(schema);
        if buf.len() < expected {
            return Err(StorageError::SizeMismatch(expected, buf.len()));
        }

        let mut off = 1;
        let tombstone = buf[off] != 0;
        off += 1;

        let row_id: u64 = u64::from_le_bytes(buf[off..off + 8].try_into().unwrap());
        off += 8;

        let mut values: Vec<ColValue> = Vec::with_capacity(schema.len());
        for col in schema {
            let sz = ColValue::size(&col.data_type);
            let null = if col.nullable {
                let flag = buf[off] != 0;
                off += 1;
                flag
            } else {
                false
            };

            let val = ColValue::deserialize(&buf[off..off + sz], &col.data_type, null)?;
            values.push(val);
            off += sz;
        }

        Ok(Self {
            tombstone,
            row_id,
            values,
        })
    }
}

// LeafNode - [tag :1][entry count: 2][pad: 1][right sibling: 8][high key: 8] then each LeafVal in order
//
// The Tag specifies it to be a LeafNode to distinguish from internal nodes
// The padding aligns the right sibling pointer to a 4 byte and rounds the struct to 20 instead of 19, always 0
// right sibling carries the PageID or NULL_PAGE if rightmost
// high key contains the smallesst key of the right sibling
pub struct LeafNode {
    pub right_sibling: PageId,
    // first row_id of right sibling
    // happen concurrently, example -
    // [10, 20, 30, 40] splits into
    // A [10, 20] high_key = 30
    // B [30, 40] high_key +inf
    // Now if an old instance redirects the search to A for 35 then we can know that 35 belongs to
    // the right sibling and therefore the search does not fail outright
    //
    // Derived from Lehman-Yao B-link trees - https://www.cs.utexas.edu/~dsb/cs386d/Readings/ConcurrencyControl/Lehman-Yao.pdf
    pub high_key: Key,
    pub entries: Vec<LeafVal>,
}

impl LeafNode {
    pub fn new() -> Self {
        Self {
            right_sibling: NULL_PAGE,
            high_key: Key::Inf,
            entries: Vec::new(),
        }
    }

    fn max_entries(schema: &[Column]) -> usize {
        let sz = LeafVal::size(schema);
        if sz == 0 {
            return 0;
        }

        // 20 is the size of the header
        (PAGE_SIZE - 20) / sz
    }

    // Requires the schema so the entry size can be calculated
    pub fn serialize(&self, schema: &[Column]) -> Result<[u8; PAGE_SIZE], StorageError> {
        let max = Self::max_entries(schema);

        if self.entries.len() > max {
            return Err(StorageError::SizeMismatch(self.entries.len(), max));
        }

        let sz = LeafVal::size(schema);
        let mut page = [0u8; PAGE_SIZE];

        page[0] = Pagetag::LEAF as u8;
        page[1..3].copy_from_slice(&sz.to_le_bytes());
        // page[3] is pad and should be default 0
        page[4..12].copy_from_slice(&self.right_sibling.to_le_bytes());
        page[12..21].copy_from_slice(&self.high_key.serialize());

        let mut off = 20;
        for entry in &self.entries {
            entry.serialize(&mut page[off..off + sz], schema)?;
            off += sz;
        }

        Ok(page)
    }

    pub fn deserialize(page: &[u8; PAGE_SIZE], schema: &[Column]) -> Result<Self, StorageError> {
        if page[0] != Pagetag::LEAF as u8 {
            return Err(StorageError::WrongTag(
                Pagetag::LEAF,
                Pagetag::get_tag(page[0]).unwrap(),
            ));
        }

        let count = u16::from_le_bytes(page[1..3].try_into().unwrap()) as usize;
        let max = Self::max_entries(schema);

        if count > max {
            return Err(StorageError::SizeMismatch(count, max));
        }

        let right_sibling = PageId::from_le_bytes(page[4..12].try_into().unwrap());
        let high_key = Key::deseriablize(page[12..21].try_into().unwrap())?;

        let mut entries: Vec<LeafVal> = Vec::with_capacity(count);

        let sz = LeafVal::size(schema);
        let mut off = 20;
        for _ in 0..count {
            let entry = LeafVal::deserialize(&page[off..off + sz], schema)?;
            entries.push(entry);
            off += sz;
        }

        Ok(Self {
            right_sibling,
            high_key,
            entries,
        })
    }

    pub fn insert(&mut self, entry: LeafVal, schema: &[Column]) -> Result<(), StorageError> {
        if self.entries.len() >= Self::max_entries(schema) {
            return Err(StorageError::FullPage);
        }

        // TO insert in a sorted order;
        let pos = self.entries.partition_point(|e| e.row_id < entry.row_id);
        if pos < self.entries.len() && entry.row_id == self.entries[pos].row_id {
            return Err(StorageError::DuplicateRow);
        }
        self.entries.insert(pos, entry);

        Ok(())
    }

    pub fn delete(&mut self, row_id: u64) -> Result<(), StorageError> {
        match self.entries.binary_search_by_key(&row_id, |e| e.row_id) {
            Ok(idx) => {
                self.entries[idx].tombstone = true;
                Ok(())
            }

            Err(_) => Err(StorageError::RowNotFound(row_id)),
        }
    }

    pub fn get(&self, row_id: u64) -> Option<&LeafVal> {
        match self.entries.binary_search_by_key(&row_id, |e| e.row_id) {
            Ok(idx) if !self.entries[idx].tombstone => Some(&self.entries[idx]),
            _ => None,
        }
    }

    // It is used when an insert would overflow the page
    // Produces two leaf nodes where left holds the lower half keys and right holds the upper half keys
    // The call site must -
    // Set the right sibling of the left node
    // update the parent node with the split key
    // write both nodes to disk with allocated PageIds
    //
    // The return value is (left, right, split_key)
    pub fn split(self) -> (LeafNode, LeafNode, u64) {
        let mid = self.entries.len() / 2;
        let split = self.entries[mid].row_id;

        let (le, re) = {
            let mut all = self.entries;
            let right = all.split_off(mid);
            (all, right)
        };

        let left = LeafNode {
            // Filled at call site
            right_sibling: NULL_PAGE,
            high_key: Key::Int(split as i64),
            entries: le,
        };

        let right = LeafNode {
            right_sibling: self.right_sibling,
            high_key: self.high_key,
            entries: re,
        };

        (left, right, split)
    }
}
