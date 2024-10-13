use std::sync::{atomic::AtomicUsize, Arc};

use super::StorageManager;

pub struct GarbageCollector {
    invalidated_size: AtomicUsize,
    data_start_point: AtomicUsize,
    storage_manager: Arc<StorageManager>,
}

impl GarbageCollector {
    pub fn new(storage_manager: Arc<StorageManager>) -> Self {
        Self {
            invalidated_size: AtomicUsize::new(0),
            data_start_point: AtomicUsize::new(0),
            storage_manager,
        }
    }

    pub fn run(&self) {
        todo!()
    }

    pub fn collect(&self) {
        todo!()
    }
}
