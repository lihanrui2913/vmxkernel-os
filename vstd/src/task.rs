use crate::SyscallIndex;

pub fn exit(code: usize) -> ! {
    crate::syscall(SyscallIndex::Exit as u64, code, 0, 0, 0, 0);

    loop {}
}

pub fn execve(buf: &[u8]) -> usize {
    crate::syscall(
        SyscallIndex::Execve as u64,
        buf.as_ptr() as usize,
        buf.len(),
        0,
        0,
        0,
    )
}

pub fn wait(pid: usize) -> usize {
    loop {
        let is_exited = crate::syscall(SyscallIndex::IsExited as u64, pid, 0, 0, 0, 0);
        if is_exited != 0 {
            break;
        }
        unsafe { core::arch::asm!("pause") };
    }

    0
}

pub fn run_vm(address: usize) -> usize {
    crate::syscall(SyscallIndex::RunVM as u64, address, 0, 0, 0, 0)
}
