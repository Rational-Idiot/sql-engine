// Keys
// Disk encoding: fixed KEY_SIZE = 9 bytes
//   byte 0:    discriminant (0=Int, 1=Float, 2=Bool, 3=Text)
//   bytes 1-8: payload
//     Int   - i64 little-endian
//     Float - f64 little-endian
//     Bool  - 0x00 or 0x01, rest zeroed
//     Text  - first 8 UTF-8 bytes, zero-padded (prefix for navigation;
//              full string is kept in memory for exact leaf comparison)
//
// Known limitation: Text keys with same 8 byte prefix collid in internal nodes,
// Navigation still gets to correct subtree, exact checking happens at leaf nodes
// using the full string. Overflow pages for arbitraily long text keys are left for a later pass

use std::cmp::Ordering;

use crate::storage::page::{PAGE_SIZE, PageId, tag};

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

    pub fn deseriablize(buf: [u8; KEY_SIZE]) -> Self {
        match buf[0] {
            0 => Key::Int(i64::from_le_bytes(buf[1..9].try_into().unwrap())),
            1 => Key::Float(F64Key(f64::from_le_bytes(buf[1..9].try_into().unwrap()))),
            2 => Key::Bool(buf[1] == 1),
            3 => {
                let end = buf[1..9].iter().position(|&b| b == 0).unwrap_or(8);
                Key::Text(String::from_utf8_lossy(&buf[1..1 + end]).into_owned())
            }
            t => panic!("Invalid key tag: {t:#x}"),
        }
    }
}

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

    pub fn deserialize(buf: &[u8; PAGE_SIZE]) -> Self {
        let n = u16::from_le_bytes(buf[1..3].try_into().unwrap()) as usize;

        let mut keys = Vec::with_capacity(n);
        for i in 0..n {
            let off = INTERNAL_KEYS_OFF + i * KEY_SIZE;
            keys.push(Key::deseriablize(
                buf[off..off + KEY_SIZE].try_into().unwrap(),
            ));
        }
        let mut children = Vec::with_capacity(n + 1);
        for i in 0..=n {
            let off = INTERNAL_CHILDREN_OFF + i * 8;
            children.push(u64::from_le_bytes(buf[off..off + 8].try_into().unwrap()));
        }
        InternalNode { keys, children }
    }
}
