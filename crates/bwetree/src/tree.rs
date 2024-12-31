// bwe_tree.rs

use super::{
    DataDelta, DeleteDelta, DeltaNode, GarbageCollector, IndexDelta, Key, LSN, NodeType, PageID,
    RequestType, SplitDelta, StorageManager, Value,
};
use super::{MappingTable, MappingTableEntry};
use super::{Page, SuspendedRequest};
use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};

pub struct BweTree {
    pub mapping_table: Arc<MappingTable>,
    pub root_page_id: Mutex<PageID>,
    suspended_requests: Mutex<HashMap<PageID, Vec<SuspendedRequest>>>,
    request_condvar: Condvar,
    next_page_id: Mutex<PageID>,
    garbage_collector: GarbageCollector,
    storage_manager: Arc<StorageManager>,
}

impl BweTree {
    pub fn new(path: &str) -> Self {
        let mapping_table = Arc::new(MappingTable::new());

        // Initialize root page as a leaf page
        let root_page_id = 0;
        let root_page = Arc::new(Page::new(root_page_id, NodeType::Leaf, Key::MIN, Key::MAX));
        let root_entry = MappingTableEntry {
            page: root_page,
            pending_alloc: false,
            pending_dealloc: false,
            under_smo: false,
        };
        mapping_table.update_entry(root_page_id, root_entry);

        let storage_mgr = Arc::new(StorageManager::new(path));

        Self {
            mapping_table,
            root_page_id: Mutex::new(root_page_id),
            suspended_requests: Mutex::new(HashMap::new()),
            request_condvar: Condvar::new(),
            next_page_id: Mutex::new(1),
            garbage_collector: GarbageCollector::new(storage_mgr.clone()),
            storage_manager: storage_mgr,
        }
    }

    fn allocate_page_id(&self) -> PageID {
        let mut next_id = self.next_page_id.lock().unwrap();
        let page_id = *next_id;
        *next_id += 1;
        page_id
    }
}

impl BweTree {
    // Suspend request of action on a page
    fn suspend_request(&self, page_id: PageID, request: SuspendedRequest) {
        let mut suspended = self.suspended_requests.lock().unwrap();
        suspended.entry(page_id).or_default().push(request);
        self.request_condvar.wait(suspended).unwrap();
    }

    // Wake requests to process action
    fn wake_up_suspended_requests(&self, page_id: PageID) {
        let mut suspended = self.suspended_requests.lock().unwrap();
        if let Some(requests) = suspended.remove(&page_id) {
            // Notify for action
            self.request_condvar.notify_all();
            // Simplify to process
            for req in requests {
                match req.request_type {
                    RequestType::Insert(key, value, lsn) => {
                        self.insert(key, value, lsn);
                    }
                    RequestType::Delete(key, lsn) => {
                        self.delete(key, lsn);
                    }
                    // Process other actions
                    _ => {}
                }
            }
        }
    }
}

impl BweTree {
    fn is_under_smo(&self, page_id: &PageID) -> bool {
        self.mapping_table.is_under_smo(page_id)
    }

    fn set_under_smo(&self, page_id: PageID) {
        self.mapping_table.set_under_smo(page_id);
    }

    fn clear_under_smo(&self, page_id: PageID) {
        self.mapping_table.clear_under_smo(page_id);
    }
}

impl BweTree {
    pub fn insert(&self, key: Key, value: Value, lsn: LSN) {
        loop {
            // 1. Find leaf page containing this key along with parent path
            let (leaf_entry, parents) = match self.find_leaf_page_with_parents(key) {
                Some(result) => result,
                None => return, // Handle error
            };

            let page = leaf_entry.page.clone();
            let page_id = page.page_id;

            // 2. Check if this page is under SMO
            if self.is_under_smo(&page_id) {
                // Suspend this request
                let request = SuspendedRequest {
                    request_type: RequestType::Insert(key, value.clone(), lsn),
                };
                self.suspend_request(page_id, request);
                return;
            }

            // 3. Create DataDelta and add it to the delta chain
            let data_delta = DataDelta {
                lsn,
                record: (key, value.clone()),
                next: None,
            };

            // TODO: Should use atomic operations!!
            {
                let mut delta_chain = page.delta_chain.lock().unwrap();
                let original_chain = delta_chain.clone();
                let mut new_delta = DeltaNode::DataDelta(data_delta);
                new_delta.set_next(original_chain);

                *delta_chain = Some(Arc::new(new_delta));
            }

            // 4. Check if page needs to split
            if self.need_split(&page) {
                self.handle_split(&leaf_entry, parents);
                // Retry insertion after split
                continue;
            }

            // Done!
            break;
        }
    }

    pub fn delete(&self, key: Key, lsn: LSN) {
        loop {
            // Find the leaf page containing the key along with parent path
            let (leaf_entry, parents) = match self.find_leaf_page_with_parents(key) {
                Some(result) => result,
                None => return, // Key not found
            };

            let page = leaf_entry.page.clone();
            let page_id = page.page_id;

            // Check if the page is under SMO
            if self.is_under_smo(&page_id) {
                let request = SuspendedRequest {
                    request_type: RequestType::Delete(key, lsn),
                };
                self.suspend_request(page_id, request);
                return;
            }

            // Create DeleteDelta and add it to the delta chain
            let delete_delta = DeleteDelta {
                lsn,
                key,
                next: None,
            };

            {
                let mut delta_chain = page.delta_chain.lock().unwrap();
                let original_chain = delta_chain.clone();
                let mut new_delta = DeltaNode::DeleteDelta(delete_delta);
                new_delta.set_next(original_chain);

                *delta_chain = Some(Arc::new(new_delta));
            }

            // Check if page needs to merge
            if self.need_merge(&page) {
                self.handle_merge(&leaf_entry, parents);
                // Retry deletion after merge
                continue;
            }

            // Deletion successful
            break;
        }
    }
}

impl BweTree {
    fn find_leaf_page(&self, key: Key) -> Option<MappingTableEntry> {
        let root_page_id = *self.root_page_id.lock().unwrap();
        let mut current_page_id = root_page_id;

        loop {
            let entry = self.mapping_table.get_entry(&current_page_id)?;
            let page = entry.page.clone();

            let page_state = self.consolidate_page(&page);

            match page_state.node_type {
                NodeType::Leaf => return Some(entry),
                NodeType::Internal => {
                    let child_page_id = self.find_child_in_internal_node(&page_state, key)?;
                    current_page_id = child_page_id;
                }
            }
        }
    }

    // Modified to track parent nodes
    fn find_leaf_page_with_parents(&self, key: Key) -> Option<(MappingTableEntry, Vec<PageID>)> {
        let root_page_id = *self.root_page_id.lock().unwrap();
        let mut current_page_id = root_page_id;
        let mut parents = Vec::new();

        loop {
            let entry = self.mapping_table.get_entry(&current_page_id)?;
            let page = entry.page.clone();

            let page_state = self.consolidate_page(&page);

            match page_state.node_type {
                NodeType::Leaf => return Some((entry, parents)),
                NodeType::Internal => {
                    parents.push(current_page_id);
                    let child_page_id = self.find_child_in_internal_node(&page_state, key)?;
                    current_page_id = child_page_id;
                }
            }
        }
    }

    fn find_child_in_internal_node(&self, page_state: &PageState, key: Key) -> Option<PageID> {
        for (index_key, child_page_id) in &page_state.index_entries {
            if key < *index_key {
                return Some(*child_page_id);
            }
        }
        page_state.index_entries.last().map(|(_, pid)| *pid)
    }
}

struct PageState {
    pub node_type: NodeType,
    pub low_key: Key,
    pub high_key: Key,
    pub records: Vec<(Key, Value)>,
    pub index_entries: Vec<(Key, PageID)>,
    pub right_sibling: Option<PageID>,
}

impl BweTree {
    fn consolidate_page(&self, page: &Arc<Page>) -> PageState {
        let mut page_state = PageState {
            node_type: page.node_type,
            low_key: page.low_key,
            high_key: *page.high_key.lock().unwrap(),
            records: vec![],
            index_entries: vec![],
            right_sibling: *page.right_sibling.lock().unwrap(),
        };

        if page.node_type == NodeType::Leaf {
            page_state.records = page.get_base_data();
        } else {
            page_state.index_entries = page.get_index_entries();
        }

        let mut delta_opt = page.get_delta_chain();

        while let Some(delta_arc) = delta_opt {
            match &*delta_arc {
                DeltaNode::DataDelta(data_delta) => {
                    if page_state.node_type == NodeType::Leaf {
                        page_state.records.push(data_delta.record.clone());
                    }
                    delta_opt = data_delta.next.clone();
                }
                DeltaNode::DeleteDelta(delete_delta) => {
                    if page_state.node_type == NodeType::Leaf {
                        page_state.records.retain(|(k, _)| *k != delete_delta.key);
                    }
                    delta_opt = delete_delta.next.clone();
                }
                DeltaNode::IndexDelta(index_delta) => {
                    if page_state.node_type == NodeType::Internal {
                        page_state
                            .index_entries
                            .extend(index_delta.index_entries.clone());
                    }
                    delta_opt = index_delta.next.clone();
                }
                DeltaNode::SplitDelta(split_delta) => {
                    page_state.high_key = split_delta.split_key;
                    page_state.right_sibling = Some(split_delta.right_page_id);
                    delta_opt = split_delta.next.clone();
                }
                DeltaNode::MergeDelta(merge_delta) => {
                    page_state.low_key = merge_delta.merge_key;
                    delta_opt = merge_delta.next.clone();
                }
                DeltaNode::LinkDelta(link_delta) => {
                    delta_opt = link_delta.next.clone();
                }
                DeltaNode::FlushDelta(flush_delta) => {
                    delta_opt = flush_delta.next.clone();
                }
            }
        }

        if page_state.node_type == NodeType::Leaf {
            page_state.records.sort_by(|a, b| a.0.cmp(&b.0));
        } else {
            page_state.index_entries.sort_by(|a, b| a.0.cmp(&b.0));
        }

        page_state
    }
}

impl BweTree {
    fn need_split(&self, page: &Arc<Page>) -> bool {
        let logical_size = self.calculate_logical_size(page);
        logical_size > self.smo_threshold() && self.can_split(page)
    }

    fn calculate_logical_size(&self, page: &Arc<Page>) -> usize {
        let page_state = self.consolidate_page(page);

        if page_state.node_type == NodeType::Leaf {
            page_state
                .records
                .iter()
                .map(|(k, v)| std::mem::size_of_val(k) + v.len())
                .sum()
        } else {
            page_state.index_entries.len()
                * (std::mem::size_of::<Key>() + std::mem::size_of::<PageID>())
        }
    }

    fn can_split(&self, page: &Arc<Page>) -> bool {
        let logical_size = self.calculate_logical_size(page);
        logical_size / 2 >= self.smo_threshold() / 4 // right/all == 1/4
    }

    fn smo_threshold(&self) -> usize {
        4 * 1024
    }

    fn handle_split(&self, entry: &MappingTableEntry, parents: Vec<PageID>) {
        let page = entry.page.clone();
        let page_id = page.page_id;

        if self.is_under_smo(&page_id) {
            let request = SuspendedRequest {
                request_type: RequestType::Split,
            };
            self.suspend_request(page_id, request);
            return;
        }

        self.set_under_smo(page_id);

        // Lock delta chain with page
        let delta_chain_lock = page.delta_chain.lock().unwrap();

        // 1. Allocate a new page, save the right half
        let new_page_id = self.allocate_page_id();
        let split_key = self.choose_split_key(&page);

        let new_page = Arc::new(Page::new(
            new_page_id,
            page.node_type,
            split_key,
            *page.high_key.lock().unwrap(),
        ));
        new_page
            .right_sibling
            .lock()
            .unwrap()
            .replace(page.right_sibling.lock().unwrap().unwrap());

        // Set PendingAlloc flag for new page
        let new_entry = MappingTableEntry {
            page: new_page.clone(),
            pending_alloc: true,
            pending_dealloc: false,
            under_smo: false,
        };
        self.mapping_table.update_entry(new_page_id, new_entry);

        // Simulate immediate flush to storage (omitted)

        // 2. Update original page
        {
            (*page).update_high_key(split_key);
            *page.right_sibling.lock().unwrap() = Some(new_page_id);

            // Add SplitDelta to delta chain
            let split_delta = SplitDelta {
                lsn: 0, // Set appropriate LSN
                split_key,
                right_page_id: new_page_id,
                next: page.get_delta_chain(),
            };
            page.add_delta(DeltaNode::SplitDelta(split_delta));
        }

        // Simulate flush to storage (omitted)

        // Clear PendingAlloc flag for new page
        self.mapping_table.clear_pending_alloc(new_page_id);

        // 3. Update parent node index entries
        self.split_index_entry_with_parents(page.clone(), split_key, new_page_id, parents);

        // Clear UnderSMO flag for the page
        self.clear_under_smo(page_id);

        // Wake up suspended requests
        self.wake_up_suspended_requests(page_id);
    }

    fn choose_split_key(&self, page: &Arc<Page>) -> Key {
        let page_state = self.consolidate_page(page);

        if page_state.node_type == NodeType::Leaf {
            let mid = page_state.records.len() / 2;
            page_state.records[mid].0
        } else {
            let mid = page_state.index_entries.len() / 2;
            page_state.index_entries[mid].0
        }
    }
}

impl BweTree {
    fn split_index_entry_with_parents(
        &self,
        old_page: Arc<Page>,
        split_key: Key,
        new_page_id: PageID,
        mut parents: Vec<PageID>,
    ) {
        if let Some(parent_page_id) = parents.pop() {
            let parent_entry = self.mapping_table.get_entry(&parent_page_id).unwrap();
            let parent_page = parent_entry.page.clone();

            // Insert index delta into parent page
            let index_delta = IndexDelta {
                lsn: 0, // Set appropriate LSN
                index_entries: vec![(split_key, new_page_id)],
                next: parent_page.get_delta_chain(),
            };
            parent_page.add_delta(DeltaNode::IndexDelta(index_delta));

            // Check if parent page needs splitting
            if self.need_split(&parent_page) {
                self.handle_split(&parent_entry, parents);
            }
        } else {
            // Create a new root node as before
            let new_root_page_id = self.allocate_page_id();
            let new_root_page = Arc::new(Page::new(
                new_root_page_id,
                NodeType::Internal,
                Key::MIN,
                Key::MAX,
            ));

            // Add index entries to the new root node
            new_root_page.add_index_entry(*old_page.high_key.lock().unwrap(), old_page.page_id);
            new_root_page.add_index_entry(split_key, new_page_id);

            let new_root_entry = MappingTableEntry {
                page: new_root_page.clone(),
                pending_alloc: false,
                pending_dealloc: false,
                under_smo: false,
            };
            self.mapping_table
                .update_entry(new_root_page_id, new_root_entry);

            // Update root node ID
            *self.root_page_id.lock().unwrap() = new_root_page_id;
        }
    }

    fn need_merge(&self, page: &Arc<Page>) -> bool {
        let logical_size = self.calculate_logical_size(page);
        logical_size < self.merge_threshold()
    }

    fn merge_threshold(&self) -> usize {
        self.smo_threshold() / 4 // For example, a quarter of the SMO threshold
    }

    // Find parent page ID
    fn find_parent_page_id(&self, _page: &Arc<Page>) -> Option<PageID> {
        todo!()
    }
}

impl BweTree {
    pub fn range_query(&self, start_key: Key, end_key: Key) -> Vec<(Key, Value)> {
        let mut results = Vec::new();

        // Find the starting leaf page
        let (mut entry, _) = match self.find_leaf_page_with_parents(start_key) {
            Some(result) => result,
            None => return results, // Start key not found
        };

        loop {
            let page = entry.page.clone();
            let page_state = self.consolidate_page(&page);

            // Collect keys within the range
            for (key, value) in page_state.records {
                if key >= start_key && key <= end_key {
                    results.push((key, value));
                }
            }

            // Check if we need to move to the right sibling
            if let Some(right_sibling_id) = page_state.right_sibling {
                if start_key <= end_key {
                    entry = self.mapping_table.get_entry(&right_sibling_id).unwrap();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        results
    }
}

impl BweTree {
    fn handle_merge(&self, entry: &MappingTableEntry, parents: Vec<PageID>) {
        let page = entry.page.clone();
        let page_id = page.page_id;

        // Set UnderSMO flag
        self.set_under_smo(page_id);

        // Lock delta chain
        let delta_chain_lock = page.delta_chain.lock().unwrap();

        // Find the left sibling
        let left_sibling_id = self.find_left_sibling(&page);
        if left_sibling_id.is_none() {
            // Cannot merge if there's no left sibling
            self.clear_under_smo(page_id);
            return;
        }
        let left_sibling_id = left_sibling_id.unwrap();
        let left_entry = self.mapping_table.get_entry(&left_sibling_id).unwrap();
        let left_page = left_entry.page.clone();

        // Merge page into left sibling
        {
            let mut left_base_data = left_page.base_data.lock().unwrap();
            let page_state = self.consolidate_page(&page);

            if page_state.node_type == NodeType::Leaf {
                left_base_data.extend(page_state.records);
                left_base_data.sort_by(|a, b| a.0.cmp(&b.0));
            } else {
                let mut left_index_entries = left_page.index_entries.lock().unwrap();
                left_index_entries.extend(page_state.index_entries);
                left_index_entries.sort_by(|a, b| a.0.cmp(&b.0));
            }

            // Update left page's high key and right sibling
            left_page.update_high_key(*page.high_key.lock().unwrap());
            *left_page.right_sibling.lock().unwrap() = *page.right_sibling.lock().unwrap();
        }

        // Set PendingDealloc flag for the merged page
        self.mapping_table.set_pending_alloc(page_id);

        // Update parent node index entries
        self.merge_index_entry_with_parents(page_id, parents);

        // Clear UnderSMO flag
        self.clear_under_smo(page_id);

        // Wake up suspended requests
        self.wake_up_suspended_requests(page_id);
    }

    fn find_left_sibling(&self, page: &Arc<Page>) -> Option<PageID> {
        // Implement logic to find the left sibling of the given page
        // This may involve traversing the parent node's index entries
        None // Placeholder
    }

    fn merge_index_entry_with_parents(&self, merged_page_id: PageID, mut parents: Vec<PageID>) {
        if let Some(parent_page_id) = parents.pop() {
            let parent_entry = self.mapping_table.get_entry(&parent_page_id).unwrap();
            let parent_page = parent_entry.page.clone();

            // Remove index entry pointing to the merged page
            let mut index_entries = parent_page.index_entries.lock().unwrap();
            index_entries.retain(|(_, pid)| *pid != merged_page_id);

            // Check if parent page needs merging
            if self.need_merge(&parent_page) {
                self.handle_merge(&parent_entry, parents);
            }
        }
    }
}

#[cfg(test)]
mod bwe_tree_test {

    use super::*;

    #[test]
    fn test_bwe_tree_basic_read_write() {
        let tree = BweTree::new("/tmp/bwe_tree_test");

        let test_data = vec![
            (1, b"Value1".to_vec()),
            (2, b"Value2".to_vec()),
            (3, b"Value3".to_vec()),
            (4, b"Value4".to_vec()),
            (5, b"Value5".to_vec()),
        ];

        for (key, value) in &test_data {
            tree.insert(*key, value.clone(), 0);
        }

        for (key, expected_value) in &test_data {
            let result = tree.range_query(*key, *key);
            assert_eq!(result[0].1, expected_value.clone());
        }

        // delete operation
        tree.delete(3, 0); // Delete key 3

        // key 3 is deleted
        let result = tree.range_query(3, 3);
        assert_eq!(result.len(), 0);
    }
}
