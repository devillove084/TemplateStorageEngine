use super::Page;
use super::PageID;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct MappingTableEntry {
    pub page: Arc<Page>,
    pub pending_alloc: bool,
    pub pending_dealloc: bool,
    pub under_smo: bool,
}

pub struct MappingTable {
    table: RwLock<HashMap<PageID, MappingTableEntry>>,
}

impl MappingTable {
    pub fn new() -> Self {
        Self {
            table: RwLock::new(HashMap::new()),
        }
    }

    pub fn get_entry(&self, page_id: &PageID) -> Option<MappingTableEntry> {
        let table = self.table.read().unwrap();
        table.get(page_id).cloned()
    }

    pub fn update_entry(&self, page_id: PageID, entry: MappingTableEntry) {
        let mut table = self.table.write().unwrap();
        table.insert(page_id, entry);
    }

    pub fn set_under_smo(&self, page_id: PageID) {
        let mut table = self.table.write().unwrap();
        if let Some(entry) = table.get_mut(&page_id) {
            entry.under_smo = true;
        }
    }

    pub fn clear_under_smo(&self, page_id: PageID) {
        let mut table = self.table.write().unwrap();
        if let Some(entry) = table.get_mut(&page_id) {
            entry.under_smo = false;
        }
    }

    pub fn is_under_smo(&self, page_id: &PageID) -> bool {
        let table = self.table.read().unwrap();
        if let Some(entry) = table.get(page_id) {
            entry.under_smo
        } else {
            false
        }
    }

    pub fn set_pending_alloc(&self, page_id: PageID) {
        let mut table = self.table.write().unwrap();
        if let Some(entry) = table.get_mut(&page_id) {
            entry.pending_alloc = true;
        }
    }

    pub fn clear_pending_alloc(&self, page_id: PageID) {
        let mut table = self.table.write().unwrap();
        if let Some(entry) = table.get_mut(&page_id) {
            entry.pending_alloc = false;
        }
    }
}
