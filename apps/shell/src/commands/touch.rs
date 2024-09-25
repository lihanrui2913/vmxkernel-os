use alloc::{string::String, vec::Vec};
use vstd::println;

pub fn touch(args: Vec<String>) {
    if args.len() != 2 {
        println!("Usage: touch <path>\n");
        return;
    }

    let path = args[1].clone();

    let fd = vstd::fs::create(path.clone(), vstd::fs::OpenMode::Read);
    if fd == usize::MAX {
        println!("Cannot touch file: {}", path.clone());
    }
}
