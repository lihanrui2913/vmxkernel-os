#![no_std]
#![no_main]

use vstd::println;

extern crate alloc;

#[no_mangle]
pub fn main() {
    (0..1000).for_each(|i| {
        println!("Waiting {}", i);
    });
    println!("Shell done.");
}
