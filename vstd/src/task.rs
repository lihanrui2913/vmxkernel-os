pub fn exit(code: usize) -> ! {
    const EXIT_SYSCALL_ID: u64 = 3;
    crate::syscall(EXIT_SYSCALL_ID, code, 0, 0, 0, 0);

    loop {}
}

pub fn execve(buf: &[u8]) -> usize {
    const EXECVE_SYSCALL_ID: u64 = 10;
    crate::syscall(EXECVE_SYSCALL_ID, buf.as_ptr() as usize, buf.len(), 0, 0, 0)
}
