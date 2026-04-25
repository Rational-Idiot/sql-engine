pub const PAGE_SIZE: usize = 4096;
pub type PageId = u64; // 8 Bytes
pub const NULL_PAGE: PageId = u64::MAX;

pub mod tag {
    pub const INTERNAL: u8 = 0x01;
    pub const LEAF: u8 = 0x02;
    pub const OVERFLOW: u8 = 0x03;
    pub const FREELIST: u8 = 0x04;
    pub const COMMIT: u8 = 0x05;
    pub const CATALOG: u8 = 0x06;
}
