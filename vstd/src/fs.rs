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
