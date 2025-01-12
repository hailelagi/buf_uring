use bytes::Bytes;
use derive_with::With;
use std::sync::atomic::AtomicU32;

pub const PAGE_SIZE: usize = 4096;

#[derive(Debug, With)]
pub struct Page {
    pub page_id: u32,
    pub data: Bytes,
    pub is_dirty: bool,
    pub ref_count: AtomicU32,
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
