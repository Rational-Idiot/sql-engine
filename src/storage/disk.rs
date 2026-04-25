// Header Layout
// [0..8]   magic: u64
// [8..12]  version: u32
// [12..16] page_size: u32
// [16..24] commit_root:PageId
// [24..32] free_head: PageId of head of on-disk free-list chain
// [32..40] page_count: u64
// [40..]   reserved / zeroed

const MAGIC: u64 = 0x4442_5452_4545_0001; // "DBTREE\0\1"
const VERSION: u32 = 1;

// Free-list page layout
// [0]      tag:   u8      (FREELIST)
// [1..5]   count: u32     — number of ids stored in this page
// [5..13]  next:  PageId  — next free-list page, or NULL_PAGE
// [13..]   ids:   [PageId; count]
