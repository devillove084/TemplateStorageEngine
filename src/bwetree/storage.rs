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

    // 写入页面片段到存储
    pub fn write_page_fragment(&self, data: &[u8]) -> StorageLocation {
        let mut file = self.file.lock().unwrap();
        let mut block_number = self.next_block_number.lock().unwrap();

        let location = StorageLocation {
            block_number: *block_number,
            offset: 0, // 假设每个块只存储一个页面片段
        };

        file.seek(SeekFrom::End(0)).unwrap();
        file.write_all(data).unwrap();

        *block_number += 1;

        location
    }

    // 从存储读取页面片段
    pub fn read_page_fragment(&self, location: &StorageLocation) -> Vec<u8> {
        let mut file = self.file.lock().unwrap();

        // 计算偏移量
        let offset = location.block_number * BLOCK_SIZE + location.offset;

        file.seek(SeekFrom::Start(offset)).unwrap();
        let mut buffer = vec![0u8; BLOCK_SIZE as usize];
        file.read_exact(&mut buffer).unwrap();

        buffer
    }

    // 其他存储管理方法
}

const BLOCK_SIZE: u64 = 4096;
