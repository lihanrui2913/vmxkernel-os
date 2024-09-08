use alloc::{string::String, vec};
use vstd::{
    fs::{close, fsize, open, read, OpenMode},
    task::{execve, wait},
};

pub fn try_run(path: String, args: &str) -> Option<()> {
    if path.len() == 0 {
        return None;
    }

    let fd = open(path, OpenMode::Read);
    if fd == usize::MAX {
        return None;
    }
    let mut buf = vec![0; fsize(fd)];
    read(fd, &mut buf);
    close(fd);

    let addr = alloc::vec![0u8; args.len()].leak();
    addr.copy_from_slice(args.as_bytes());
    let pid = execve(&buf, addr.as_ptr() as usize, addr.len());
    wait(pid);

    Some(())
}
