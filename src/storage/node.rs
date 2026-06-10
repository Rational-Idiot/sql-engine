// Keys
// Disk encoding: fixed KEY_SIZE = 9 bytes
//   byte 0:    discriminant (0=Int, 1=Float, 2=Bool, 3=Text)
//   bytes 1-8: payload
//     Int   - i64 little-endian
//     Float - f64 little-endian
//     Bool  - 0x00 or 0x01, rest zeroed
//     Text  - first 8 UTF-8 bytes, zero-padded

use core::fmt;
use std::{cmp::Ordering, fmt::Display, str::from_utf8};

use crate::{
    catalog::Column,
    sql::ast::DataType,
    storage::page::{NULL_PAGE, PAGE_SIZE, PageId, tag},
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    Int(i64),
    Float(F64Key),
    Bool(bool),
    Text(String),
}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Key::Int(a), Key::Int(b)) => a.cmp(b),
            (Key::Float(a), Key::Float(b)) => a.cmp(b),
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
                buf[1..9].copy_from_slice(&f.0.to_le_bytes());
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
        }
        buf
    }

    pub fn deseriablize(buf: [u8; KEY_SIZE]) -> Result<Self, StorageError> {
        let payload = buf[1..9]
            .try_into()
            .expect("The payload should always be 8 bytes");
        match buf[0] {
            0 => Ok(Key::Int(i64::from_le_bytes(payload))),
            1 => Ok(Key::Float(F64Key(f64::from_le_bytes(payload)))),
            2 => Ok(Key::Bool(buf[1] == 1)),
            3 => {
                let end = payload.iter().position(|&b| b == 0).unwrap_or(8);
                let text = from_utf8(&payload[..end]).map_err(|e| {
                    StorageError::InavlidFormat(format!("invalid UTF-8 in text key: {e}"))
                })?;

                Ok(Key::Text(text.to_owned()))
            }
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
                write!(f, "Expected buffer to be {} long but is only {} long", a, b)
            }
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
        buf[0] = tag::INTERNAL;
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
    page[0] = tag::OVERFLOW;
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

    if page[0] != tag::OVERFLOW {
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

pub struct LeafNode {
    pub right_sibling: PageId,
    /// first row_id of right sibling
    /// happen concurrently, example -
    /// [10, 20, 30, 40] splits into
    /// A [10, 20] high_key = 30
    /// B [30, 40] high_key +inf
    /// Now if an old instance redirects the search to A for 35 then we can know that 35 belongs to
    /// the right sibling and therefore the search does not fail outright
    ///
    /// Derived from Lehman-Yao B-link trees
    pub high_key: Key,
    pub entries: Vec<LeafVal>,
}
