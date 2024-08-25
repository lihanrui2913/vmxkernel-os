use core::arch::asm;
use core::mem::{transmute, variant_count};

use super::operations::*;

#[derive(Debug)]
#[allow(dead_code)]
enum SyscallIndex {
    Null,
    Print,
    Malloc,
    Exit,
    Free,
    Open,
    Close,
    Read,
    Write,
    Fsize,
    Execve,
}

impl From<usize> for SyscallIndex {
    fn from(number: usize) -> Self {
        let syscall_length = variant_count::<Self>();
        if number >= syscall_length {
            panic!("Invalid syscall index: {}", number);
        }
        unsafe { transmute(number as u8) }
    }
}

#[allow(unused_variables)]
pub extern "C" fn syscall_matcher(
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
) -> usize {
    let syscall_number_raw: usize;
    unsafe { asm!("mov {0}, rax", out(reg) syscall_number_raw) };

    match SyscallIndex::from(syscall_number_raw) {
        SyscallIndex::Null => unimplemented!(),
        SyscallIndex::Print => print(arg1 as *const u8, arg2),
        SyscallIndex::Malloc => malloc(arg1, arg2),
        SyscallIndex::Exit => exit(),
        SyscallIndex::Free => free(arg1, arg2, arg3),
        SyscallIndex::Open => open(arg1, arg2, arg3),
        SyscallIndex::Close => close(arg1),
        SyscallIndex::Read => read(arg1, arg2, arg3),
        SyscallIndex::Write => write(arg1, arg2, arg3),
        SyscallIndex::Fsize => fsize(arg1),
        SyscallIndex::Execve => execve(arg1, arg2),
    }
}
