use core::str;

use alloc::{string::String, vec::Vec};

use crate::SyscallIndex;

pub fn exit(code: usize) -> ! {
    crate::syscall(SyscallIndex::Exit as u64, code, 0, 0, 0, 0);

    loop {}
}

pub fn execve(buf: &[u8], args_ptr: usize, args_len: usize) -> usize {
    crate::syscall(
        SyscallIndex::Execve as u64,
        buf.as_ptr() as usize,
        buf.len(),
        args_ptr,
        args_len,
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

pub fn get_args() -> Vec<String> {
    let mut vec = Vec::new();

    let ptr = crate::syscall(SyscallIndex::GetArgs as u64, 0, 0, 0, 0, 0);
    let args_buf_ptr = unsafe { (ptr as *const u64).read() };
    let args_buf_len = unsafe { (ptr as *const usize).add(1).read() };
    let args_buf = unsafe { core::slice::from_raw_parts(args_buf_ptr as *const u8, args_buf_len) };

    let args = str::from_utf8(args_buf).unwrap();
    let args_vec = args.split(" ");
    for arg in args_vec {
        vec.push(String::from(arg));
    }

    vec
}
