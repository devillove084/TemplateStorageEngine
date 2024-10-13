use super::DeltaNode;
use super::{Key, NodeType, PageID, Value};
use std::sync::{Arc, Mutex};

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
