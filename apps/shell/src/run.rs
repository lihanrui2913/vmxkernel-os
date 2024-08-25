use alloc::{string::String, vec};
use vstd::{
    fs::{close, fsize, open, read, OpenMode},
    task::{execve, wait},
};

pub fn try_run(path: String) -> usize {
    let fd = open(path, OpenMode::Read);
    let mut buf = vec![0; fsize(fd)];
    read(fd, &mut buf);
    close(fd);

    let pid = execve(&buf);
    wait(pid);

    0
}
