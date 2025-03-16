#![deny(clippy::all)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::module_inception)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::question_mark)]
#![feature(allocator_api)]
#![allow(clippy::rc_buffer)]
#[macro_use]
extern crate log;
extern crate crc32fast;
extern crate crossbeam_channel;
extern crate crossbeam_utils;
extern crate slog;
extern crate slog_async;
extern crate slog_term;
#[macro_use]
extern crate num_derive;
extern crate bytes;
extern crate quick_error;
extern crate rand;
extern crate snap;
extern crate thiserror;

#[macro_use]
pub mod error;

pub mod cache;
pub mod compaction;
pub mod db_impl;
pub mod db_trait;
pub mod iterator;
mod logger;
pub mod manager;
pub mod memtable;
pub mod operator;
pub mod options;
pub mod sstable;
pub mod storage;
pub mod util;
pub mod wal;
pub mod instance;
pub mod metric;