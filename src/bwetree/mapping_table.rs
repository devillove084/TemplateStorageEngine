// mapping_table.rs

use super::Page;
use super::PageID;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct MappingTableEntry {
    pub page: Arc<Page>,
    pub pending_alloc: bool,
    pub pending_dealloc: bool,
    pub under_smo: bool, // 标记页面是否正在进行 SMO
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

    // 获取页面条目
    pub fn get_entry(&self, page_id: &PageID) -> Option<MappingTableEntry> {
        let table = self.table.read().unwrap();
        table.get(page_id).cloned()
    }

    // 更新页面条目
    pub fn update_entry(&self, page_id: PageID, entry: MappingTableEntry) {
        let mut table = self.table.write().unwrap();
        table.insert(page_id, entry);
    }

    // 设置页面的 UnderSMO 标志
    pub fn set_under_smo(&self, page_id: PageID) {
        let mut table = self.table.write().unwrap();
        if let Some(entry) = table.get_mut(&page_id) {
            entry.under_smo = true;
        }
    }

    // 清除页面的 UnderSMO 标志
    pub fn clear_under_smo(&self, page_id: PageID) {
        let mut table = self.table.write().unwrap();
        if let Some(entry) = table.get_mut(&page_id) {
            entry.under_smo = false;
        }
    }

    // 检查页面是否正在进行 SMO
    pub fn is_under_smo(&self, page_id: &PageID) -> bool {
        let table = self.table.read().unwrap();
        if let Some(entry) = table.get(page_id) {
            entry.under_smo
        } else {
            false
        }
    }

    // 设置 PendingAlloc 标志
    pub fn set_pending_alloc(&self, page_id: PageID) {
        let mut table = self.table.write().unwrap();
        if let Some(entry) = table.get_mut(&page_id) {
            entry.pending_alloc = true;
        }
    }

    // 清除 PendingAlloc 标志
    pub fn clear_pending_alloc(&self, page_id: PageID) {
        let mut table = self.table.write().unwrap();
        if let Some(entry) = table.get_mut(&page_id) {
            entry.pending_alloc = false;
        }
    }

    // 其他方法，例如设置/清除 PendingDealloc 标志
}
