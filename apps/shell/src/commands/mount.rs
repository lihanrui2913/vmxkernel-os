use alloc::{string::String, vec::Vec};
use vstd::println;

pub fn mount(args: Vec<String>) {
    if args.len() != 3 {
        println!("Usage: mount <path> <partition>\n");
        return;
    }

    let path = args[2].clone();
    let partition = args[1].clone();

    vstd::fs::mount(path.clone(), partition.clone()).unwrap_or_else(|_| {
        println!("Failed to mount {} to {}\n", partition, path);
    });
}
