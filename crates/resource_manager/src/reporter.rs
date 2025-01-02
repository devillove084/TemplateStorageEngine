use tokio::sync::Notify;

use crate::StorageEventHappen;


// ! push model
pub struct InfoReporter<I: StorageEventHappen> {
    queue: crossbeam_channel::Sender<I>,
}

// ! pull model
pub struct InfoGather {}