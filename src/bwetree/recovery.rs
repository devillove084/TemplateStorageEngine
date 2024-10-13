use super::{Key, Value, LSN};

pub enum RequestType {
    Insert(Key, Value, LSN),
    Delete(Key, LSN),
    Split,
    // 其他请求类型
}

pub struct SuspendedRequest {
    pub request_type: RequestType,
}
