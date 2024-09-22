#![no_std]
#![feature(naked_functions)]
#![feature(variant_count)]

pub extern crate alloc;

pub mod debug;
pub mod fs;
pub mod mm;
pub mod task;

use core::panic::PanicInfo;
use core::usize;

use alloc::string::String;
use alloc::vec::Vec;
pub use syscall_index::*;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("User Panic:{}", info);
    task::exit(usize::MAX)
}

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

extern "Rust" {
    fn main(args: Vec<String>) -> usize;
}

#[no_mangle]
pub unsafe extern "sysv64" fn _start() -> ! {
    let args = task::get_args();
    task::exit(main(args));
}
