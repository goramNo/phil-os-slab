#![no_std]

use core::ptr::null_mut;

#[derive(Clone, Copy)]
struct FreeNode {
    next: *mut FreeNode,
}

pub struct SlabCache {
    size: usize,
    free_list: *mut FreeNode,
}

impl SlabCache {
    pub const fn new(size: usize) -> Self {
        Self {
            size,
            free_list: null_mut(),
        }
    }

    pub unsafe fn alloc(&mut self) -> *mut u8 {
        if self.free_list.is_null() {
            self.refill();
        }

        let node = self.free_list;
        if node.is_null() {
            return null_mut();
        }

        self.free_list = (*node).next;
        node as *mut u8
    }

    pub unsafe fn dealloc(&mut self, ptr: *mut u8) {
        let node = ptr as *mut FreeNode;
        (*node).next = self.free_list;
        self.free_list = node;
    }

    unsafe fn refill(&mut self) {
        static mut PAGE: [u8; 4096] = [0; 4096];

        let mut offset = 0;
        while offset + self.size <= 4096 {
            let node = PAGE.as_mut_ptr().add(offset) as *mut FreeNode;
            (*node).next = self.free_list;
            self.free_list = node;
            offset += self.size;
        }
    }
}

pub struct SlabAllocator {
    caches: [SlabCache; 8],
}

impl SlabAllocator {
    pub const fn new() -> Self {
        Self {
            caches: [
                SlabCache::new(8),
                SlabCache::new(16),
                SlabCache::new(32),
                SlabCache::new(64),
                SlabCache::new(128),
                SlabCache::new(256),
                SlabCache::new(512),
                SlabCache::new(1024),
            ],
        }
    }

    pub unsafe fn alloc(&mut self, size: usize) -> *mut u8 {
        for cache in self.caches.iter_mut() {
            if size <= cache.size {
                return cache.alloc();
            }
        }
        null_mut()
    }

    pub unsafe fn dealloc(&mut self, ptr: *mut u8, size: usize) {
        for cache in self.caches.iter_mut() {
            if size <= cache.size {
                cache.dealloc(ptr);
                return;
            }
        }
    }
}
