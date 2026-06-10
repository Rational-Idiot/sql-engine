use std::fmt;

pub const PAGE_SIZE: usize = 4096;
pub type PageId = u64; // 8 Bytes
pub const NULL_PAGE: PageId = u64::MAX;

#[repr(u8)]
#[derive(Debug)]
pub enum Pagetag {
    INTERNAL = 1,
    LEAF = 2,
    OVERFLOW = 3,
    FREELIST = 4,
    COMMIT = 5,
    CATALOG = 6,
}

impl fmt::Display for Pagetag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Pagetag::INTERNAL => write!(f, "PageTag::Internal"),
            Pagetag::LEAF => write!(f, "PageTag::Leaf"),
            Pagetag::OVERFLOW => write!(f, "PageTag::Overflow"),
            Pagetag::FREELIST => write!(f, "PageTag::Freelist"),
            Pagetag::COMMIT => write!(f, "PageTag::Commit"),
            Pagetag::CATALOG => write!(f, "PageTag::Catalog"),
        }
    }
}

impl Pagetag {
    pub fn get_tag(val: u8) -> Option<Self> {
        match val {
            1 => Some(Pagetag::INTERNAL),
            2 => Some(Pagetag::LEAF),
            3 => Some(Pagetag::OVERFLOW),
            4 => Some(Pagetag::FREELIST),
            5 => Some(Pagetag::COMMIT),
            6 => Some(Pagetag::CATALOG),
            _ => None,
        }
    }
}
