pub type Key = i64;
pub type Value = Vec<u8>;
pub type LSN = u64;
pub type PageID = usize;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Internal,
    Leaf,
}
