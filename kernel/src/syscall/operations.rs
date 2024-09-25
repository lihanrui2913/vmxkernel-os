use crate::fs::operation::{get_inode_by_fd, OpenMode};
use crate::fs::vfs::inode::{FileInfo, InodeTy};
use crate::memory::{addr_to_mut_ref, write_for_syscall};
use crate::task::process::{is_process_exited, ProcessId};
use crate::task::scheduler::SCHEDULER;
use crate::task::{get_current_process, get_current_process_id};
use alloc::alloc::{alloc, dealloc};
use alloc::collections::btree_map::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::alloc::Layout;
use core::{slice, str, usize};
use spin::{Lazy, Mutex};
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

pub fn execve(buf_addr: usize, buf_len: usize, args_ptr: usize, args_len: usize) -> usize {
    let buffer = unsafe { slice::from_raw_parts(buf_addr as _, buf_len) };
    let new_process = crate::task::process::Process::new_user_process("task", buffer);
    if new_process.is_none() {
        return usize::MAX;
    }
    let new_process = new_process.unwrap();
    new_process.write().args_value = args_ptr;
    new_process.write().args_len = args_len;
    let ret = new_process.read().id.0 as usize;
    ret
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

pub fn list_dir(path_addr: usize, path_len: usize, buf_addr: usize) -> usize {
    let buf = unsafe { slice::from_raw_parts(path_addr as _, path_len) };
    let path = String::from(core::str::from_utf8(buf).unwrap());

    #[derive(Clone)]
    #[allow(dead_code)]
    struct TemporyInfo {
        name: &'static [u8],
        ty: InodeTy,
    }

    let file_infos: Vec<TemporyInfo> = {
        let infos = crate::fs::operation::list_dir(path);
        let mut new_infos = Vec::new();
        for info in infos.iter() {
            let FileInfo { name, ty } = info;
            let new_name = alloc::vec![0u8; name.len()].leak();
            new_name[..name.len()].copy_from_slice(name.as_bytes());
            new_infos.push(TemporyInfo {
                name: new_name,
                ty: *ty,
            });
        }
        new_infos
    };

    write_for_syscall(VirtAddr::new(buf_addr as u64), file_infos.as_slice());

    0
}

pub fn dir_item_num(path_addr: usize, path_len: usize) -> usize {
    let buf = unsafe { slice::from_raw_parts(path_addr as _, path_len) };
    let path = String::from(core::str::from_utf8(buf).unwrap());

    crate::fs::operation::list_dir(path).len()
}

pub fn ioctl(fd: usize, cmd: usize, arg: usize) -> usize {
    crate::fs::operation::ioctl(fd, cmd, arg)
}

pub fn get_args() -> usize {
    let current_process = get_current_process();
    let current_args_value = current_process.read().args_value;
    let current_args_len = current_process.read().args_len;

    let ret_struct_ptr = alloc::vec![0u8; 16].leak().as_ptr() as u64;
    let value_ptr = addr_to_mut_ref(VirtAddr::new(ret_struct_ptr));
    *value_ptr = current_args_value;
    let len_ptr = addr_to_mut_ref(VirtAddr::new(ret_struct_ptr + 8));
    *len_ptr = current_args_len;
    ret_struct_ptr as usize
}

pub fn get_pid() -> usize {
    get_current_process_id().0 as usize
}

pub fn lseek(fd: usize, offset: usize) -> usize {
    let res = crate::fs::operation::lseek(fd, offset);
    if res.is_some() {
        return 0;
    }
    return usize::MAX;
}

pub fn kill_process(pid: usize) -> usize {
    {
        let mut scheduler = SCHEDULER.lock();
        let thread = scheduler.find(ProcessId::from(pid as u64));

        if let Some(thread) = thread {
            scheduler.remove(thread.clone());
            if let Some(thread) = thread.upgrade() {
                let thread = thread.read();
                if let Some(process) = thread.process.upgrade() {
                    process.read().exit_process();
                }
            }
        }
    }

    0
}

static C_ALLOCATION_MAP: Lazy<Mutex<BTreeMap<VirtAddr, (VirtAddr, usize, usize)>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));

pub fn sbrk(size: usize) -> usize {
    let size = size as isize;
    if size > 0 {
        let size = size as usize;

        let space: Vec<u8> = alloc::vec![0u8; size];

        assert!(space.len() == size);
        let (ptr, len, cap) = space.into_raw_parts();
        if !ptr.is_null() {
            let vaddr = VirtAddr::new(ptr as u64);
            let mut guard = C_ALLOCATION_MAP.lock();
            if guard.contains_key(&vaddr) {
                drop(guard);
                unsafe {
                    drop(Vec::from_raw_parts(vaddr.as_mut_ptr() as *mut u8, len, cap));
                }
                panic!(
                    "sbrk(): vaddr {:?} already exists in C Allocation Map, query size: {size}",
                    vaddr
                );
            }
            guard.insert(vaddr, (vaddr, len, cap));
            return vaddr.as_u64() as usize;
        } else {
            return usize::MAX;
        }
    } else {
        return usize::MAX;
    }
}

pub fn create(path: usize, path_len: usize, mode: usize) -> usize {
    let path = String::from(
        str::from_utf8(unsafe { core::slice::from_raw_parts(path as *const u8, path_len) })
            .expect("Cannot from utf8"),
    );

    let fd = crate::fs::operation::create(path, InodeTy::File, OpenMode::from(mode));
    fd.unwrap_or(usize::MAX)
}

pub fn mount(
    path_addr: usize,
    path_len: usize,
    partition_addr: usize,
    partition_len: usize,
) -> usize {
    let path = String::from(
        str::from_utf8(unsafe { core::slice::from_raw_parts(path_addr as *const u8, path_len) })
            .expect("Cannot from utf8"),
    );

    let partition = String::from(
        str::from_utf8(unsafe {
            core::slice::from_raw_parts(partition_addr as *const u8, partition_len)
        })
        .expect("Cannot from utf8"),
    );

    let ret = crate::fs::operation::mount(path, partition);

    if ret.is_none() {
        return usize::MAX;
    }

    0
}
