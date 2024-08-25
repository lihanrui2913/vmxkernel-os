pub fn exit(code: usize) -> ! {
    const EXIT_SYSCALL_ID: u64 = 3;
    crate::syscall(EXIT_SYSCALL_ID, code, 0, 0, 0, 0);

    loop {}
}
