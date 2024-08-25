#![no_std]
#![feature(naked_functions)]

pub extern crate alloc;

pub mod debug;
pub mod fs;
pub mod mm;
pub mod task;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("User Panic:{}", info);
    loop {}
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

extern "C" {
    fn main() -> usize;
}

#[no_mangle]
pub unsafe extern "sysv64" fn _start() -> ! {
    task::exit(main());
}
