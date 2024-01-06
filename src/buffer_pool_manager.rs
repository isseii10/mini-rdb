use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io;
use std::ops::{Index, IndexMut};
use std::rc::Rc;

use create::DiskManager::{DiskManager, PageId, PAGE_SIZE};

pub type Page = [u8; PAGE_SIZE];

pub struct Buffer {
    pub page_id: PageId,
    pub page: RefCell<Page>,
    pub is_dirty: Cell<bool>,
}

pub struct Frame {
    usage_count: u64,
    buffer: Rc<Buffer>,
}

pub struct BufferPool {
    buffers: Vec<Frame>,
    next_victim_id: BufferId,
}

impl BufferPool {
    pub fn new(pool_size: usize) -> self {
        let mut buffers = vec![];
        buffers.resize_with(pool_size, Default::Default);
        let next_victim_id = BufferId::default();
        Self {
            buffers,
            next_victim_id,
        }
    }

    fn size(&self) -> usize {
        self.buffers.len()
    }

    fn evict(&mut self) -> Option<BufferId> {
        // clock-sweep algorithm
        let pool_size = self.size();
        let mut consecutive_pinned = 0;
        let victim_id = loop {
            let next_victim_id = self.next_victim_id;
            let frame = &mut self[next_victim_id];
            if frame.usage_count == 0 {
                break self.next_victim_id;
            }
            if Rc::get_mut(&mut frame.buffer).is_some() {
                frame.usage_count -= 1;
                consecutive_pinned = 0;
            } else {
                consecutive_pinned += 1;
                if consecutive_pinned >= pool_size {
                    return None;
                }
            }
            self.next_victim_id = self.increment_id(self.next_victim_id);
        };
        Some(next_victim_id);
    }

    fn increment_id(&self, buffer_id: BufferId) -> BufferId {
        BufferId((buffer_id.0 + 1) % self.size())
    }
}

pub struct BufferPoolManager {
    disk: DiskManager,
    pool: BufferPool,
    page_table: HashMap<PageId, BufferId>,
}

impl BufferPoolManager {
    pub fn new(disk: DiskManager, pool: BufferPool) -> self {
        let page_table = HashMap::new();
        Self {
            disk,
            pool,
            page_table,
        }
    }

    pub fn fetch_page(&mut self, page_id: PageId) -> Result<Rc<Buffer>, Error> {
        // pageがBufferPoolにある場合
        if let Some(&buffer_id) = self.page_table.get(&page_id) {
            let frame = &mut self[page_id];
            frame.usage_count += 1;
            return Ok(frame.buffer.clone());
        }
        // pageがBufferPoolにない場合
        let buffer_id = self.pool.evict().ok_or(Error::NoFreeBuffer)?;
        let frame = &mut self.pool[buffer_id];
        let evict_page_id = frame.buffer.page_id;
        {}
        let page = Rc::clone(&frame.buffer);
        self.page_table.remove(&evict_page_id);
        self.page_table.insert(page_id, buffer_id);
    }
}
