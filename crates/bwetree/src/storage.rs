use bytes::Bytes;

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::Mutex;

struct FlushBuffer {
    data: Bytes,
    capacity: usize,
}

pub struct StorageLocation {
    block_number: u64,
    offset: u64,
}

pub struct StorageManager {
    file: Mutex<File>,
    next_block_number: Mutex<u64>,
}

impl StorageManager {
    pub fn new(file_path: &str) -> Self {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(file_path)
            .unwrap();

        Self {
            file: Mutex::new(file),
            next_block_number: Mutex::new(0),
        }
    }

    pub fn write_page_fragment(&self, data: &[u8]) -> StorageLocation {
        todo!()
    }

    pub fn read_page_fragment(&self, location: &StorageLocation) -> Vec<u8> {
        todo!()
    }
}

const BLOCK_SIZE: u64 = 4096;
