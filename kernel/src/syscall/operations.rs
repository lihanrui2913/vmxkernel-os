use crate::fs::operation::{get_inode_by_fd, OpenMode};
use crate::memory::addr_to_mut_ref;
use crate::task::process::is_process_exited;
use crate::task::scheduler::SCHEDULER;
use alloc::alloc::{alloc, dealloc};
use alloc::string::String;
use core::alloc::Layout;
use core::{slice, str, usize};
use x86_64::VirtAddr;

pub fn print(buffer: *const u8, length: usize) -> usize {
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

pub fn exit() -> usize {
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

    0
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

pub fn open(path: usize, path_len: usize, mode: usize) -> usize {
    let slice = unsafe { core::slice::from_raw_parts(path as _, path_len) };
    let path = String::from(str::from_utf8(slice).expect("Cannot from utf8"));

    if let Some(ret) = crate::fs::operation::open(path.clone(), OpenMode::from(mode)) {
        return ret;
    }
    usize::MAX
}

pub fn close(fd: usize) -> usize {
    crate::fs::operation::close(fd);
    0
}

pub fn read(fd: usize, buf: usize, buf_size: usize) -> usize {
    let buffer = unsafe { slice::from_raw_parts_mut(buf as _, buf_size) };
    crate::fs::operation::read(fd, buffer)
}

pub fn write(fd: usize, buf: usize, buf_size: usize) -> usize {
    let buffer = unsafe { slice::from_raw_parts(buf as _, buf_size) };
    crate::fs::operation::write(fd, buffer)
}

pub fn fsize(fd: usize) -> usize {
    crate::fs::operation::fsize(fd).unwrap()
}

pub fn execve(buf_addr: usize, buf_len: usize) -> usize {
    let buffer = unsafe { slice::from_raw_parts(buf_addr as _, buf_len) };
    crate::task::process::Process::new_user_process("task", buffer)
        .read()
        .id
        .0 as usize
}

pub fn is_exited(pid: usize) -> usize {
    is_process_exited(pid) as usize
}

pub fn change_cwd(path_addr: usize, path_len: usize) -> usize {
    let buf = unsafe { slice::from_raw_parts(path_addr as _, path_len) };
    let path = String::from(core::str::from_utf8(buf).unwrap());

    crate::fs::operation::change_cwd(path);

    0
}

pub fn get_cwd() -> usize {
    let path = crate::fs::operation::get_cwd();
    let new_path = alloc::vec![0u8;path.len()].leak();
    new_path[..path.len()].copy_from_slice(path.as_bytes());
    let ret_struct_ptr = alloc::vec![0u8; 16].leak().as_ptr() as u64;
    let path_ptr = addr_to_mut_ref(VirtAddr::new(ret_struct_ptr));
    *path_ptr = new_path;
    let len_ptr = addr_to_mut_ref(VirtAddr::new(ret_struct_ptr + 8));
    *len_ptr = path.len();
    ret_struct_ptr as usize
}

pub fn ftype(fd: usize) -> usize {
    let inode = get_inode_by_fd(fd);

    if let Some(inode) = inode {
        return inode.read().inode_type() as usize;
    }

    usize::MAX
}
