#![no_std]
#![no_main]

use alloc::{string::String, vec::Vec};
use vstd::println;

extern crate alloc;

#[no_mangle]
pub fn main(args: Vec<String>) -> usize {
    println!("init process is running!!! args = {:?}", args);

    let fd = vstd::fs::open(String::from("/shell.elf"), vstd::fs::OpenMode::Read);
    let fsize = vstd::fs::fsize(fd);
    let buf = alloc::vec![0u8; fsize].leak();
    vstd::fs::read(fd, buf);

    let args = "/shell.elf --ls";
    let addr = alloc::vec![0u8; args.len()].leak();
    addr.copy_from_slice(args.as_bytes());
    let pid = vstd::task::execve(buf, addr.as_ptr() as usize, addr.len());
    vstd::task::wait(pid);

    println!("shell done");

    0
}
