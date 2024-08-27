#![no_std]
#![no_main]

use alloc::string::String;
use vstd::{
    fs::{ioctl, open, write},
    FbDevIoctlCommand,
};

extern crate alloc;

pub fn box_fill(
    buf: &mut [u32],
    width: usize,
    color: u32,
    startx: usize,
    starty: usize,
    endx: usize,
    endy: usize,
) {
    for y in starty..endy {
        for x in startx..endx {
            buf[y * width + x] = color;
        }
    }
}

pub fn flush_fb(fbfd: usize, buf: &mut [u32]) {
    let buf_ptr = buf.as_ptr() as _;
    let buf_len = buf.len() * 4;
    let buf = unsafe { core::slice::from_raw_parts(buf_ptr, buf_len) };

    write(fbfd, buf);
}

pub fn init_screen(buf: &mut [u32], width: usize, height: usize) {
    box_fill(buf, width, 0x00008484, 0, 0, width, height - 29);
    box_fill(buf, width, 0x00C6C6C6, 0, height - 28, width, height - 28);
    box_fill(buf, width, 0x00FFFFFF, 0, height - 27, width, height - 27);
    box_fill(buf, width, 0x00C6C6C6, 0, height - 26, width, height);

    box_fill(buf, width, 0x00FFFFFF, 3, height - 24, 59, height - 24);
    box_fill(buf, width, 0x00FFFFFF, 2, height - 24, 2, height - 4);
    box_fill(buf, width, 0x00848484, 3, height - 4, 59, height - 4);
    box_fill(buf, width, 0x00848484, 59, height - 23, 59, height - 5);
    box_fill(buf, width, 0x00000000, 2, height - 3, 59, height - 3);
    box_fill(buf, width, 0x00000000, 60, height - 24, 60, height - 3);

    box_fill(
        buf,
        width,
        0x00848484,
        width - 47,
        height - 24,
        width - 4,
        height - 24,
    );
    box_fill(
        buf,
        width,
        0x00848484,
        width - 47,
        height - 23,
        width - 47,
        height - 4,
    );
    box_fill(
        buf,
        width,
        0x00FFFFFF,
        width - 47,
        height - 3,
        width - 4,
        height - 3,
    );
    box_fill(
        buf,
        width,
        0x00FFFFFF,
        width - 3,
        height - 24,
        width - 3,
        height - 3,
    );
}

#[no_mangle]
pub fn main() -> usize {
    let fbfd = open(String::from("/dev/fb"), vstd::fs::OpenMode::Write);
    let width = ioctl(fbfd, FbDevIoctlCommand::GetWidth as usize, 0);
    let height = ioctl(fbfd, FbDevIoctlCommand::GetHeight as usize, 0);
    let buf = alloc::vec![0u32; width * height].leak();

    loop {
        init_screen(buf, width, height);

        flush_fb(fbfd, buf)
    }
}
