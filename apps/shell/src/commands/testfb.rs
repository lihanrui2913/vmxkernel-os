use alloc::{string::String, vec::Vec};
use vstd::{
    fs::{ioctl, open, write},
    FbDevIoctlCommand,
};

pub fn testfb(_args: Vec<String>) {
    let fbfd = open(String::from("/dev/fb"), vstd::fs::OpenMode::Write);
    let width = ioctl(fbfd, FbDevIoctlCommand::GetWidth as usize, 0);
    let height = ioctl(fbfd, FbDevIoctlCommand::GetHeight as usize, 0);
    let buf = alloc::vec![0u32; width * height].leak();

    for x in 0..width {
        for y in 0..height {
            buf[y * width + x] = 0x00C6C6C6;
        }
    }

    let buf_ptr = buf.as_ptr() as _;
    let buf_len = buf.len() * 4;
    let buf = unsafe { core::slice::from_raw_parts(buf_ptr, buf_len) };

    write(fbfd, buf);
}
