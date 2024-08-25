#![no_std]
#![no_main]

use alloc::string::String;
use vstd::println;

extern crate alloc;

#[no_mangle]
pub fn main() -> usize {
    let fd = vstd::fs::open(String::from("/shell.elf"), vstd::fs::OpenMode::Read);
    let fsize = vstd::fs::fsize(fd);
    let buf = alloc::vec![0u8; fsize].leak();
    vstd::fs::read(fd, buf);

    let pid = vstd::task::execve(buf);
    vstd::task::wait(pid);

    println!("shell done");

    0
}
