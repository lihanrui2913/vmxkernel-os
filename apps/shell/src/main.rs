#![no_std]
#![no_main]

use vstd::println;

extern crate alloc;

#[no_mangle]
pub fn main() -> usize {
    println!("Shell started");

    0
}
