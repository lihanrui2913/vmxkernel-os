#![no_std]
#![feature(naked_functions)]
#![feature(variant_count)]

pub extern crate alloc;

pub mod fs;
pub mod mm;
pub mod task;

pub use syscall_index::*;

#[naked]
extern "C" fn syscall(
    _id: u64,
    _arg1: usize,
    _arg2: usize,
    _arg3: usize,
    _arg4: usize,
    _arg5: usize,
) -> usize {
    unsafe {
        core::arch::asm!(
            "mov rax, rdi",
            "mov rdi, rsi",
            "mov rsi, rdx",
            "mov rdx, rcx",
            "mov r10, r8",
            "mov r8, r9",
            "syscall",
            "ret",
            options(noreturn)
        )
    }
}
