use alloc::string::String;

#[repr(C)]
pub enum OpenMode {
    Read = 0,
    Write = 1,
}

pub fn open(path: String, mode: OpenMode) -> usize {
    const OPEN_SYSCALL_ID: u64 = 5;
    crate::syscall(
        OPEN_SYSCALL_ID,
        path.as_ptr() as usize,
        path.len(),
        mode as usize,
        0,
        0,
    )
}

pub fn close(fd: usize) -> usize {
    const CLOSE_SYSCALL_ID: u64 = 6;
    crate::syscall(CLOSE_SYSCALL_ID, fd, 0, 0, 0, 0)
}

pub fn read(fd: usize, buf: &mut [u8]) -> usize {
    const READ_SYSCALL_ID: u64 = 7;
    crate::syscall(
        READ_SYSCALL_ID,
        fd,
        buf.as_mut_ptr() as usize,
        buf.len(),
        0,
        0,
    )
}

pub fn write(fd: usize, buf: &[u8]) -> usize {
    const WRITE_SYSCALL_ID: u64 = 8;
    crate::syscall(WRITE_SYSCALL_ID, fd, buf.as_ptr() as usize, buf.len(), 0, 0)
}

pub fn fsize(fd: usize) -> usize {
    const FSIZE_SYSCALL_ID: u64 = 9;
    crate::syscall(FSIZE_SYSCALL_ID, fd, 0, 0, 0, 0)
}

pub fn change_cwd(path: String) {
    const CHANGE_CWD_SYSCALL: u64 = 12;
    crate::syscall(
        CHANGE_CWD_SYSCALL,
        path.as_ptr() as usize,
        path.len(),
        0,
        0,
        0,
    );
}

pub fn get_cwd() -> String {
    const GET_CWD_SYSCALL: u64 = 13;
    let ptr = crate::syscall(GET_CWD_SYSCALL, 0, 0, 0, 0, 0);
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

impl InodeTy {
    pub fn from(data: usize) -> Self {
        match data {
            0 => return Self::Dir,
            1 => return Self::File,
            _ => panic!("Unknown inode type"),
        }
    }
}

pub fn ftype(fd: usize) -> InodeTy {
    const FTYPE_SYSCALL_ID: u64 = 14;
    let ty = crate::syscall(FTYPE_SYSCALL_ID, fd, 0, 0, 0, 0);
    if ty == usize::MAX {
        return InodeTy::File;
    }
    InodeTy::from(ty)
}
