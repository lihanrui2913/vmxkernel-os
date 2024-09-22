use alloc::{string::String, vec::Vec};

use crate::SyscallIndex;

#[repr(C)]
pub enum OpenMode {
    Read = 0,
    Write = 1,
}

pub fn open(path: String, mode: OpenMode) -> usize {
    crate::syscall(
        SyscallIndex::Open as u64,
        path.as_ptr() as usize,
        path.len(),
        mode as usize,
        0,
        0,
    )
}

pub fn close(fd: usize) -> usize {
    crate::syscall(SyscallIndex::Close as u64, fd, 0, 0, 0, 0)
}

pub fn read(fd: usize, buf: &mut [u8]) -> usize {
    crate::syscall(
        SyscallIndex::Read as u64,
        fd,
        buf.as_mut_ptr() as usize,
        buf.len(),
        0,
        0,
    )
}

pub fn write(fd: usize, buf: &[u8]) -> usize {
    crate::syscall(
        SyscallIndex::Write as u64,
        fd,
        buf.as_ptr() as usize,
        buf.len(),
        0,
        0,
    )
}

pub fn fsize(fd: usize) -> usize {
    crate::syscall(SyscallIndex::Fsize as u64, fd, 0, 0, 0, 0)
}

pub fn change_cwd(path: String) {
    crate::syscall(
        SyscallIndex::ChangeCwd as u64,
        path.as_ptr() as usize,
        path.len(),
        0,
        0,
        0,
    );
}

pub fn get_cwd() -> String {
    let ptr = crate::syscall(SyscallIndex::GetCwd as u64, 0, 0, 0, 0, 0);
    let path_buf_ptr = unsafe { (ptr as *const u64).read() };
    let path_buf_len = unsafe { (ptr as *const usize).add(1).read() };
    let path_buf = unsafe { core::slice::from_raw_parts(path_buf_ptr as *const u8, path_buf_len) };
    String::from_utf8(path_buf.to_vec()).unwrap()
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum InodeTy {
    Dir = 0,
    File = 1,
}

impl From<usize> for InodeTy {
    fn from(data: usize) -> Self {
        match data {
            0 => return Self::Dir,
            1 => return Self::File,
            _ => panic!("Unknown inode type"),
        }
    }
}

impl Default for InodeTy {
    fn default() -> Self {
        Self::Dir
    }
}

pub fn ftype(fd: usize) -> InodeTy {
    let ty = crate::syscall(SyscallIndex::FType as u64, fd, 0, 0, 0, 0);
    if ty == usize::MAX {
        return InodeTy::File;
    }
    InodeTy::from(ty)
}

pub struct FileInfo {
    pub name: String,
    pub ty: InodeTy,
}

pub fn list_dir(path: String) -> Vec<FileInfo> {
    fn dir_item_num(path: String) -> usize {
        crate::syscall(
            SyscallIndex::DirItemNum as u64,
            path.as_ptr() as usize,
            path.len(),
            0,
            0,
            0,
        )
    }

    #[derive(Default, Clone)]
    struct TemporyInfo {
        name: &'static [u8],
        ty: InodeTy,
    }

    let len = dir_item_num(path.clone());
    let buf = alloc::vec![TemporyInfo::default(); len];

    crate::syscall(
        SyscallIndex::ListDir as u64,
        path.as_ptr() as usize,
        path.len(),
        buf.as_ptr() as usize,
        0,
        0,
    );

    let mut infos = Vec::new();
    for info in buf.iter() {
        infos.push(FileInfo {
            name: String::from_utf8(info.name.to_vec()).unwrap(),
            ty: info.ty,
        })
    }
    infos
}

pub fn ioctl(fd: usize, cmd: usize, arg: usize) -> usize {
    crate::syscall(SyscallIndex::IoCtl as u64, fd, cmd, arg, 0, 0)
}

pub fn lseek(fd: usize, ptr: usize) -> usize {
    crate::syscall(SyscallIndex::LSeek as u64, fd, ptr, 0, 0, 0)
}
