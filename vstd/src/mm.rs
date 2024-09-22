use core::alloc::{GlobalAlloc, Layout};

use crate::SyscallIndex;

fn malloc(layout: Layout) -> Result<u64, ()> {
    let addr = crate::syscall(
        SyscallIndex::Malloc as u64,
        layout.size(),
        layout.align(),
        0,
        0,
        0,
    );

    if addr == 0 {
        Err(())
    } else {
        Ok(addr as u64)
    }
}

fn free(addr: u64, layout: Layout) {
    crate::syscall(
        SyscallIndex::Free as u64,
        addr as usize,
        layout.size(),
        layout.align(),
        0,
        0,
    );
}

pub fn sbrk(size: usize) -> usize {
    crate::syscall(SyscallIndex::SBrk as u64, size, 0, 0, 0, 0)
}

struct MemoryAllocator;

unsafe impl GlobalAlloc for MemoryAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        malloc(layout).unwrap() as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        free(ptr as u64, layout)
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: MemoryAllocator = MemoryAllocator;
