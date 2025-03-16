use crate::PageIOError;

use super::DeltaNode;
use super::{Key, NodeType, PageID, Value};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct Page {
    pub page_id: PageID,
    pub node_type: NodeType,
    pub low_key: Key,
    pub high_key: Mutex<Key>,
    pub delta_chain: Mutex<Option<Arc<DeltaNode>>>,
    pub index_entries: Mutex<Vec<(Key, PageID)>>,
    pub base_data: Mutex<Vec<(Key, Value)>>,
    pub right_sibling: Mutex<Option<PageID>>,
}

impl Page {
    pub fn new(page_id: PageID, node_type: NodeType, low_key: Key, high_key: Key) -> Self {
        Self {
            page_id,
            node_type,
            low_key,
            high_key: Mutex::new(high_key),
            delta_chain: Mutex::new(None),
            index_entries: Mutex::new(Vec::new()),
            base_data: Mutex::new(Vec::new()),
            right_sibling: Mutex::new(None),
        }
    }

    pub fn add_delta(&self, delta: DeltaNode) {
        let mut delta_chain = self.delta_chain.lock().unwrap();
        let mut delta = delta;

        delta.set_next(delta_chain.clone());
        *delta_chain = Some(Arc::new(delta));
    }

    pub fn get_delta_chain(&self) -> Option<Arc<DeltaNode>> {
        let delta_chain = self.delta_chain.lock().unwrap();
        delta_chain.clone()
    }

    pub fn add_index_entry(&self, key: Key, child_page_id: PageID) {
        let mut entries = self.index_entries.lock().unwrap();
        entries.push((key, child_page_id));
        entries.sort_by(|a, b| a.0.cmp(&b.0));
    }

    pub fn get_index_entries(&self) -> Vec<(Key, PageID)> {
        let entries = self.index_entries.lock().unwrap();
        entries.clone()
    }

    pub fn get_base_data(&self) -> Vec<(Key, Value)> {
        let base_data = self.base_data.lock().unwrap();
        base_data.clone()
    }

    pub fn update_high_key(&self, new_high_key: Key) {
        let mut high_key = self.high_key.lock().unwrap();
        *high_key = new_high_key;
    }
}

pub struct PageReader {}

pub struct PageWriter {}

impl PageWriter {
    pub async fn submit_write_page(&self) -> crate::Result<PageIOError> {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageMemoryAddress(*const u8);

impl PageMemoryAddress {
    /// Create new page address(unsafe)
    /// # Safety
    /// Caller must keep pointer safe
    pub unsafe fn new(ptr: *const u8) -> Self {
        Self(ptr)
    }

    /// Access original pointer
    pub fn as_ptr(&self) -> *const u8 {
        self.0
    }

    /// To usize
    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageInFileOffset(u64);

impl PageInFileOffset {
    pub fn new(offset: u64) -> Self {
        assert!(offset <= u64::MAX, "File offset exceeds maximum value");
        Self(offset)
    }

    pub fn get(&self) -> u64 {
        self.0
    }

    pub fn checked_add(&self, rhs: u64) -> Option<Self> {
        self.0.checked_add(rhs).map(Self)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PageLocation {
    Memory(PageMemoryAddress),
    File(PageInFileOffset),
}

impl PageLocation {
    pub fn with_memory_page(addr: *const u8) -> Option<Self> {
        Some(Self::Memory(unsafe { PageMemoryAddress::new(addr) }))
    }

    pub fn with_file_offset(offset: u64) -> Option<Self> {
        Some(Self::File(PageInFileOffset::new(offset)))
    }

    pub fn as_memory(&self) -> Option<PageMemoryAddress> {
        if let Self::Memory(v) = self {
            Some(*v)
        } else {
            None
        }
    }


    pub fn as_file_offset(&self) -> Option<PageInFileOffset> {
        if let Self::File(v) = self {
            Some(*v)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod page_unit_test {

    #[test]
    fn create_page() {
        
    }
}