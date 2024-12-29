use std::sync::atomic::AtomicU32;

use bytes::Bytes;
pub const PAGE_SIZE: usize = 4096;

#[derive(Debug, Clone, With)]
pub struct Page {
    pub page_id: u32,
    data: Bytes,
    pub ref_count: AtomicU32,
    pub is_dirty: bool,
}

impl Page {
    pub fn new() -> Self {
        Self {
            page_id: 0,
            data: Bytes::new(),
            ref_count: AtomicU32::new(0),
            is_dirty: false,
        }
    }
}
