use std::{
    cell::RefCell,
    mem, ptr,
    sync::{
        Arc,
        atomic::{AtomicPtr, AtomicUsize, Ordering},
    },
};

const BLOCK_SIZE: usize = 4096;

pub trait Arena {
    /// Return the start pointer to a newly allocated memory block of 'chunk' bytes .
    ///
    /// # Safety
    ///
    /// The `*mut T` might be unmatched with the size and align
    unsafe fn allocate<T>(&self, chunk: usize, align: usize) -> *mut T;

    /// Return the size of memory that has been allocated.
    fn memory_used(&self) -> usize;
}

struct OffsetArenaInner {
    len: AtomicUsize,
    cap: usize,
    ptr: *mut u8,
}

#[derive(Clone)]
pub struct OffsetArena {
    inner: Arc<OffsetArenaInner>,
}

impl Drop for OffsetArenaInner {
    fn drop(&mut self) {
        // manully drop ArenaInner
        if !self.ptr.is_null() {
            unsafe {
                let ptr = self.ptr as *mut u64;
                let cap = self.cap / 8;
                Vec::from_raw_parts(ptr, 0, cap);
            }
        }
    }
}

impl Arena for OffsetArena {
    unsafe fn allocate<T>(&self, chunk: usize, align: usize) -> *mut T {
        let offset = self.alloc(align, chunk);
        unsafe { self.get_mut(offset) }
    }

    /// Return the size of memory that has been allocated.
    fn memory_used(&self) -> usize {
        self.inner.len.load(Ordering::SeqCst)
    }
}

unsafe impl Send for OffsetArenaInner {}
unsafe impl Sync for OffsetArenaInner {}

impl OffsetArena {
    // The real cap will be aligned with 8
    pub fn with_capacity(cap: usize) -> Self {
        let mut buf: Vec<u64> = Vec::with_capacity(cap / 8);
        let ptr = buf.as_mut_ptr() as *mut u8;
        let cap = buf.capacity() * 8;
        mem::forget(buf);
        OffsetArena {
            inner: Arc::new(OffsetArenaInner {
                len: AtomicUsize::new(1),
                cap,
                ptr,
            }),
        }
    }

    // Allocates `size` bytes aligned with `align`
    fn alloc(&self, align: usize, size: usize) -> usize {
        let align_mask = align - 1;
        // Leave enough padding for align.
        let size = size + align_mask;
        let offset = self.inner.len.fetch_add(size, Ordering::SeqCst);
        // (offset + align_mask) / align * align.
        let ptr_offset = (offset + align_mask) & !align_mask;
        assert!(
            offset + size <= self.inner.cap,
            "current {}, cap {}",
            offset + size,
            self.inner.cap
        );
        ptr_offset
    }

    // Returns a raw pointer with given arena offset
    unsafe fn get_mut<N>(&self, offset: usize) -> *mut N {
        if offset == 0 {
            return ptr::null_mut();
        }
        unsafe { self.inner.ptr.add(offset) as _} 
    }
}

/// `BlockArena` is a memory pool for allocating and handling Node memory dynamically.
/// It's caller's responsibility to ensure the room before allocating.
///
/// # NOTICE:
///
/// `BlockArena` must only be used with single thread writing since we use `RefCell` when
/// allocating new blocks.
#[derive(Default)]
pub struct BlockArena {
    ptr: AtomicPtr<u8>,
    bytes_remaining: AtomicUsize,
    blocks: RefCell<Vec<Vec<u8>>>,
    // Total memory usage of the arena.
    memory_usage: AtomicUsize,
}

impl BlockArena {
    fn allocate_fallback(&self, size: usize) -> *mut u8 {
        if size > BLOCK_SIZE / 4 {
            // Object is more than a quarter of our block size.  Allocate it separately
            // to avoid wasting too much space in leftover bytes.
            return self.allocate_new_block(size);
        }
        // create a new full block
        let new_block_ptr = self.allocate_new_block(BLOCK_SIZE);
        unsafe {
            let ptr = new_block_ptr.add(size);
            self.ptr.store(ptr, Ordering::Release);
        };
        self.bytes_remaining
            .store(BLOCK_SIZE - size, Ordering::Release);
        new_block_ptr
    }

    fn allocate_new_block(&self, block_bytes: usize) -> *mut u8 {
        let mut new_block = vec![0; block_bytes];
        let p = new_block.as_mut_ptr();
        self.blocks.borrow_mut().push(new_block);
        self.memory_usage.fetch_add(block_bytes, Ordering::Relaxed);
        p
    }
}

impl Arena for BlockArena {
    unsafe fn allocate<T>(&self, chunk: usize, align: usize) -> *mut T {
        assert!(chunk > 0);
        let ptr_size = mem::size_of::<usize>();
        // the align should be a pow(2)
        assert_eq!(align & (align - 1), 0);

        let slop = {
            let current_mod = self.ptr.load(Ordering::Acquire) as usize & (align - 1);
            if current_mod == 0 {
                0
            } else {
                align - current_mod
            }
        };
        let needed = chunk + slop;
        let result = if needed <= self.bytes_remaining.load(Ordering::Acquire) {
            // padding to align
            let p = unsafe { self.ptr.load(Ordering::Acquire).add(slop) };
            self.ptr.store(unsafe { p.add(chunk) }, Ordering::Release);
            self.bytes_remaining.fetch_sub(needed, Ordering::SeqCst);
            p
        } else {
            self.allocate_fallback(chunk)
        };
        assert_eq!(
            result as usize & (align - 1),
            0,
            "allocated memory should be aligned with {}",
            ptr_size
        );
        result as *mut T
    }

    #[inline]
    fn memory_used(&self) -> usize {
        self.memory_usage.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use std::{ptr, sync::atomic::Ordering};

    use rand::Rng;

    use crate::memtable::arena::{Arena, BLOCK_SIZE, BlockArena};

    #[test]
    fn test_new_arena() {
        let a = BlockArena::default();
        assert_eq!(a.memory_used(), 0);
        assert_eq!(a.bytes_remaining.load(Ordering::Acquire), 0);
        assert_eq!(a.ptr.load(Ordering::Acquire), ptr::null_mut());
        assert_eq!(a.blocks.borrow().len(), 0);
    }

    #[test]
    #[should_panic]
    fn test_allocate_empty_should_panic() {
        let a = BlockArena::default();
        unsafe { a.allocate::<u8>(0, 0) };
    }

    #[test]
    fn test_allocate_new_block() {
        let a = BlockArena::default();
        let mut expect_size = 0;
        for (i, size) in [1, 128, 256, 1000, 4096, 10000].iter().enumerate() {
            a.allocate_new_block(*size);
            expect_size += *size;
            assert_eq!(a.memory_used(), expect_size, "memory used should match");
            assert_eq!(
                a.blocks.borrow().len(),
                i + 1,
                "number of blocks should match"
            )
        }
    }

    #[test]
    fn test_allocate_fallback() {
        let a = BlockArena::default();
        assert_eq!(a.memory_used(), 0);
        a.allocate_fallback(1);
        assert_eq!(a.memory_used(), BLOCK_SIZE);
        assert_eq!(a.bytes_remaining.load(Ordering::Acquire), BLOCK_SIZE - 1);
        a.allocate_fallback(BLOCK_SIZE / 4 + 1);
        assert_eq!(a.memory_used(), BLOCK_SIZE + BLOCK_SIZE / 4 + 1);
    }

    #[test]
    fn test_allocate_mixed() {
        let a = BlockArena::default();
        let mut allocated = vec![];
        let mut allocated_size = 0;
        let n = 10000;
        let mut r = rand::thread_rng();
        for i in 0..n {
            let size = if i % (n / 10) == 0 {
                if i == 0 {
                    continue;
                }
                i
            } else if i == 1 {
                1
            } else {
                r.gen_range(1..i)
            };
            let ptr = unsafe { a.allocate::<u8>(size, 8) };
            unsafe {
                for j in 0..size {
                    let np = ptr.add(j);
                    (*np) = (j % 256) as u8;
                }
            }
            allocated_size += size;
            allocated.push((ptr, size));
            assert!(
                a.memory_used() >= allocated_size,
                "the memory used {} should be greater or equal to expecting allocated {}",
                a.memory_used(),
                allocated_size
            );
        }
        for (ptr, size) in allocated.iter() {
            unsafe {
                for i in 0..*size {
                    let p = ptr.add(i);
                    assert_eq!(*p, (i % 256) as u8);
                }
            }
        }
    }
}
