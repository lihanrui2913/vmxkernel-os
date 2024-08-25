use crate::println;

pub fn exit(code: usize) -> ! {
    const EXIT_SYSCALL_ID: u64 = 3;
    crate::syscall(EXIT_SYSCALL_ID, code, 0, 0, 0, 0);

    loop {}
}

pub fn execve(buf: &[u8]) -> usize {
    const EXECVE_SYSCALL_ID: u64 = 10;
    crate::syscall(EXECVE_SYSCALL_ID, buf.as_ptr() as usize, buf.len(), 0, 0, 0)
}

pub fn wait(pid: usize) -> usize {
    const IS_EXITED_SYSCALL_ID: u64 = 11;

    loop {
        let is_exited = crate::syscall(IS_EXITED_SYSCALL_ID, pid, 0, 0, 0, 0);
        if is_exited != 0 {
            break;
        }
    }

    println!("OK");

    0
}
