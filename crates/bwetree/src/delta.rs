use crate::{Page, PageLocation};

use super::{Key, LSN, PageID, Value};
use std::sync::{atomic::AtomicPtr, Arc};

#[derive(Debug)]
pub enum DeltaNode {
    DataDelta(DataDelta),
    IndexDelta(IndexDelta),
    SplitDelta(SplitDelta),
    MergeDelta(MergeDelta),
    LinkDelta(LinkDelta),
    FlushDelta(FlushDelta),
    DeleteDelta(DeleteDelta),
}

impl DeltaNode {
    pub fn next(&self) -> Option<Arc<DeltaNode>> {
        match self {
            DeltaNode::DataDelta(d) => d.next.clone(),
            DeltaNode::IndexDelta(d) => d.next.clone(),
            DeltaNode::SplitDelta(d) => d.next.clone(),
            DeltaNode::MergeDelta(d) => d.next.clone(),
            DeltaNode::LinkDelta(d) => d.next.clone(),
            DeltaNode::FlushDelta(d) => d.next.clone(),
            DeltaNode::DeleteDelta(d) => d.next.clone(),
        }
    }

    pub fn set_next(&mut self, next: Option<Arc<DeltaNode>>) {
        match self {
            DeltaNode::DataDelta(d) => d.next = next,
            DeltaNode::IndexDelta(d) => d.next = next,
            DeltaNode::SplitDelta(d) => d.next = next,
            DeltaNode::MergeDelta(d) => d.next = next,
            DeltaNode::LinkDelta(d) => d.next = next,
            DeltaNode::FlushDelta(d) => d.next = next,
            DeltaNode::DeleteDelta(d) => d.next = next,
        }
    }
}

#[derive(Debug)]
pub struct DataDelta {
    pub lsn: LSN,
    pub record: (Key, Value),
    pub next: Option<Arc<DeltaNode>>,
}

#[derive(Debug)]
pub struct IndexDelta {
    pub lsn: LSN,
    pub index_entries: Vec<(Key, PageID)>,
    pub next: Option<Arc<DeltaNode>>,
}

#[derive(Debug)]
pub struct SplitDelta {
    pub lsn: LSN,
    pub split_key: Key,
    pub right_page_id: PageID,
    pub next: Option<Arc<DeltaNode>>,
}

#[derive(Debug)]
pub struct MergeDelta {
    pub lsn: LSN,
    pub merge_key: Key,
    pub merged_page_id: PageID,
    pub next: Option<Arc<DeltaNode>>,
}

#[derive(Debug)]
pub struct LinkDelta {
    pub lsn: LSN,
    pub data_delta_count: usize,
    pub next: Option<Arc<DeltaNode>>,
}

#[derive(Debug)]
pub struct FlushDelta {
    pub storage_location: usize,
    pub next: Option<Arc<DeltaNode>>,
}

#[derive(Debug)]
pub struct DeleteDelta {
    pub lsn: LSN,
    pub key: Key,
    pub next: Option<Arc<DeltaNode>>,
}

#[derive(Debug)]
pub struct DeltaChain {
    own_base_page: Box<PageLocation>,
    next_delta_record: AtomicPtr<DeltaNode>,
}

impl DeltaChain {
    pub fn new(location: PageLocation) -> Self {
        Self {
            own_base_page: Box::new(location),
            next_delta_record: AtomicPtr::default(),
        }
    }

    pub fn get_last_delta_node_address(&self) -> Option<*mut DeltaNode> {
        todo!()
    }

    pub fn consolidate_with_base_page(&mut self) -> Option<Page> {
        todo!()
    }
}


mod delta_chain_unit_test {

}