use super::{Key, LSN, Value};

pub enum RequestType {
    Insert(Key, Value, LSN),
    Delete(Key, LSN),
    Split,
}

pub struct SuspendedRequest {
    pub request_type: RequestType,
}
