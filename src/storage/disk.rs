#![allow(dead_code)]
use crate::storage::page::{NULL_PAGE, PAGE_SIZE, PageId};
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::Path,
};

// Header Layout
// [0..8]   magic:           u64
// [8..12]  version:         u32
// [12..16] page_size:       u32
// [16..24] commit_root:     PageId
// [24..32] free_head:       PageId of head of on-disk free-list chain
// [32..40] page_count:      u64
// [40..]   reserved / zeroed

const MAGIC: u64 = 0x4442_5452_4545_0001; // "DBTREE\0\1"
const VERSION: u32 = 1;

// Free-list page layout
// [0]      tag:   u8
// [1..5]   count: u32     number of ids stored in this page
// [5..13]  next:  PageId  next free-list page, or NULL_PAGE
// [13..]   ids:   [PageId; count]

const FL_IDS_PER_PAGE: usize = (PAGE_SIZE - 13) / 8; // PAGE_SIZE - (Freelist header Size = 13 bytes) / (size of PageId: u64 = 8 bytes)

fn page_offset(id: PageId) -> u64 {
    id * PAGE_SIZE as u64
}

pub struct DiskManager {
    file: File,
    free_list: Vec<PageId>,
    page_count: u64,
    commit_root: PageId,
}

impl DiskManager {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let mut file = OpenOptions::new().read(true).write(true).open(path)?;
        file.seek(io::SeekFrom::Start(0))?; // Init the seek at 0

        let mut buf = [0u8; PAGE_SIZE];
        file.read_exact(&mut buf)?; // Read a page from memory

        let magic = u64::from_le_bytes(buf[0..8].try_into().unwrap());
        if magic != MAGIC {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Not Ma File"));
        }

        let stored_ps = u64::from_le_bytes(buf[12..16].try_into().unwrap());
        if stored_ps as usize != PAGE_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "page size mismatch",
            ));
        }

        let commit_root = u64::from_le_bytes(buf[16..24].try_into().unwrap());
        let free_head = u64::from_le_bytes(buf[24..32].try_into().unwrap());
        let page_count = u64::from_le_bytes(buf[32..40].try_into().unwrap());

        let free_list = Self::load_fl(&mut file, free_head)?;
        Ok(Self {
            file,
            free_list,
            page_count,
            commit_root,
        })
    }

    fn load_fl(file: &mut File, mut head: PageId) -> io::Result<Vec<PageId>> {
        let mut list = Vec::new();
        while head != NULL_PAGE {
            let mut buf = [0u8; PAGE_SIZE];
            file.seek(io::SeekFrom::Start(page_offset(head)))?;
            file.read_exact(&mut buf)?;

            let count = u32::from_le_bytes(buf[1..5].try_into().unwrap()) as usize;
            let next = u64::from_le_bytes(buf[5..13].try_into().unwrap());

            for i in 0..count {
                let off = 13 + i * 8; // Start at offset 13 and each PageID of size 8 bytes
                list.push(u64::from_le_bytes(buf[off..off + 8].try_into().unwrap()));
            }
            head = next;
        }
        Ok(list)
    }

    pub fn create(path: impl AsRef<Path>) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(true)
            .open(path)?;

        let mut dm = Self {
            file,
            free_list: Vec::new(),
            page_count: 1,
            commit_root: NULL_PAGE,
        };

        dm.write_header(NULL_PAGE)?; // Initial Head
        Ok(dm)
    }

    fn write_header(&mut self, free_head: PageId) -> io::Result<()> {
        let mut buf = [0u8; PAGE_SIZE];
        buf[0..8].copy_from_slice(&MAGIC.to_le_bytes());
        buf[8..12].copy_from_slice(&VERSION.to_le_bytes());
        buf[12..16].copy_from_slice(&(PAGE_SIZE as u32).to_le_bytes());
        buf[16..24].copy_from_slice(&self.commit_root.to_le_bytes());
        buf[24..32].copy_from_slice(&free_head.to_le_bytes());
        buf[32..40].copy_from_slice(&self.page_count.to_le_bytes());
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(&buf)
    }

    fn read_page(&mut self, id: PageId) -> io::Result<[u8; PAGE_SIZE]> {
        debug_assert!(id != NULL_PAGE, "read NULL_PAGE");
        let mut buf = [0u8; PAGE_SIZE];
        self.file.seek(SeekFrom::Start(page_offset(id)))?;
        self.file.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn write_page(&mut self, id: PageId, data: &[u8; PAGE_SIZE]) -> io::Result<()> {
        debug_assert!(id != NULL_PAGE, "write NULL_PAGE");
        self.file.seek(SeekFrom::Start(page_offset(id)))?;
        self.file.write_all(data)
    }

    pub fn allocate(&mut self) -> io::Result<PageId> {
        if let Some(id) = self.free_list.pop() {
            return Ok(id);
        }

        // Extend the page count by writing a zero page
        let id = self.page_count;
        self.page_count += 1;
        let buf = [0u8; PAGE_SIZE];
        self.file.seek(SeekFrom::Start(page_offset(id)))?;
        self.file.write_all(&buf)?;
        Ok(id)
    }

    pub fn free_page(&mut self, id: PageId) {
        self.free_list.push(id);
    }
}
