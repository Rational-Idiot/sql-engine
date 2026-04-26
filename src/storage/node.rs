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
