use crate::task::scheduler::SCHEDULER;
use alloc::alloc::{alloc, dealloc};
use core::alloc::Layout;
use core::{slice, str};

pub fn write(buffer: *const u8, length: usize) -> usize {
    if length == 0 {
        return 0;
    }

    if let Ok(string) = unsafe {
        let slice = slice::from_raw_parts(buffer, length);
        str::from_utf8(slice)
    } {
        crate::print!("{}", string);
    };

    0
}

pub fn exit() -> ! {
    {
        let current_thread = {
            let mut scheduler = SCHEDULER.lock();
            let thread = scheduler.current_thread();
            scheduler.remove(thread.clone());
            thread
        };

        if let Some(current_thread) = current_thread.upgrade() {
            let current_thread = current_thread.read();
            if let Some(process) = current_thread.process.upgrade() {
                process.read().exit_process();
            }
        }
    }

    loop {
        unsafe {
            core::arch::asm!("sti", "2:", "hlt", "jmp 2b");
        }
    }
}

pub fn malloc(size: usize, align: usize) -> usize {
    let layout = Layout::from_size_align(size, align);
    if let Ok(layout) = layout {
        let addr = unsafe { alloc(layout) };
        addr as usize
    } else {
        0
    }
}

pub fn free(addr: usize, size: usize, align: usize) -> usize {
    let layout = Layout::from_size_align(size, align);
    if let Ok(layout) = layout {
        unsafe { dealloc(addr as _, layout) }
        return 0;
    }

    0
}
