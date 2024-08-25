#![no_std]
#![no_main]

use alloc::string::String;
use vstd::println;

extern crate alloc;

#[no_mangle]
pub fn main() {
    let fd = vstd::fs::open(String::from("/shell.elf"), vstd::fs::OpenMode::Read);
    println!("fd = {}", fd);
    let fsize = vstd::fs::fsize(fd);
    println!("fsize = {}", fsize);
    let buf = alloc::vec![0u8; fsize].leak();
    vstd::fs::read(fd, buf);
    println!("Read done.");

    let pid = vstd::task::execve(buf);
}
