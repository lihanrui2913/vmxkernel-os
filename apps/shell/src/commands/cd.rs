use alloc::{string::String, vec::Vec};
use vstd::{
    fs::{change_cwd, close, ftype, open, InodeTy, OpenMode},
    println,
};

pub fn cd(args: Vec<String>) {
    if args.len() != 2 {
        println!("Usage: cd <folder>\n");
        return;
    }

    let path = args[1].clone();

    let k = open(path.clone(), OpenMode::Read);
    if k == usize::MAX {
        println!("cd: {}: No such directory", path);
        return;
    }

    if ftype(k) == InodeTy::File {
        println!("cd: {}: No such directory", path);
        return;
    }

    close(k);

    change_cwd(path.clone());
}
