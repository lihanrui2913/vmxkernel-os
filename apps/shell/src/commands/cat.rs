use core::str;

use alloc::{string::String, vec, vec::Vec};
use vstd::{
    fs::{fsize, open, read, OpenMode},
    println,
};

pub fn cat(args: Vec<String>) {
    if args.len() != 2 {
        println!("Usage: cat <file>");
        return;
    }

    let file_path = args[1].clone();
    let fd = open(file_path, OpenMode::Read);
    if fd == usize::MAX {
        println!("No such as file");
        return;
    }
    let mut buf = vec![0; fsize(fd)];
    read(fd, buf.as_mut_slice());
    let s = str::from_utf8(buf.as_slice());
    if let Ok(s) = s {
        println!("{}", s);
    } else {
        println!("The file content is incomprehensible");
    }
}
