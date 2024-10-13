use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};

use super::PageID;

pub struct PendingRequest {
    pub condvar: Condvar,
}

pub struct ConcurrencyManager {
    pending_requests: Mutex<HashMap<PageID, Vec<Arc<PendingRequest>>>>,
}

impl ConcurrencyManager {
    pub fn new() -> Self {
        Self {
            pending_requests: Mutex::new(HashMap::new()),
        }
    }

    pub fn suspend_request(&self, page_id: PageID, request: Arc<PendingRequest>) {
        let mut pending = self.pending_requests.lock().unwrap();
        pending
            .entry(page_id)
            .or_insert_with(Vec::new)
            .push(request);
    }

    pub fn resume_requests(&self, page_id: &PageID) {
        let mut pending = self.pending_requests.lock().unwrap();
        if let Some(requests) = pending.remove(page_id) {
            for req in requests {
                req.condvar.notify_one();
            }
        }
    }
}
