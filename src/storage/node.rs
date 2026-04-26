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
