use alloc::{string::String, vec::Vec};
use vstd::{fs::OpenMode, println};

pub fn write(args: Vec<String>) {
    if args.len() < 3 {
        println!("Usage: write <file> <content>\n");
        return;
    }

    let file_path = args[1].clone();
    let content = args[2..].join(" ");

    let fd = vstd::fs::open(file_path.clone(), OpenMode::Write);
    if fd != usize::MAX {
        vstd::fs::write(fd, content.as_bytes());
    } else {
        println!("Can't find {}.", file_path);
    }
}
