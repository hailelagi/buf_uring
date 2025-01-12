// Buffer pool.rs

use crossbeam_skiplist::SkipMap;
use io_uring::{opcode, IoUring};
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct BufferPool {
    pages: SkipMap<u64, RwLock<Vec<u8>>>,
    meta: SkipMap<u64, RwLock<Page>>,

    page_size: usize,
    k_value: usize,
    capacity: usize,

    ring: IoUring,
}

impl BufferPool {
    pub fn new(capacity: usize, page_size: usize, k_value: usize) -> std::io::Result<Self> {
        Ok(BufferPool {
            pages: SkipMap::new(),
            meta: SkipMap::new(),
            ring: IoUring::new(32)?,
            page_size,
            k_value,
            capacity,
        })
    }

    pub async fn get_page(&self, page_id: u64) -> std::io::Result<&RwLock<Vec<u8>>> {
        if let Some(page) = self.pages.get(&page_id) {
            return Ok(page.value());
        }

        // todo: should this be atomic?
        self.evict();
        self.load_page(page_id).await
    }

    fn evict(&self) {
        if self.pages.len() >= self.capacity {
            let victim = self.find_lru_k_victim();
            if let Some(page_id) = victim {
                self.evict_page(page_id);
            }
        }
    }

    fn find_lru_k_victim(&self) -> Option<u64> {
        todo!()
    }
}
