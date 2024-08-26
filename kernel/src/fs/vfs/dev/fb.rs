use alloc::string::String;
use syscall_index::FbDevIoctlCommand;

use crate::{device::display::FRAMEBUFFER_RESPONSE, fs::vfs::inode::Inode, ref_to_mut};

pub struct FrameBuffer {
    path: String,
    fbaddr: &'static mut [u32],
    width: usize,
    height: usize,
    bpp: usize,
}

impl FrameBuffer {
    pub fn new() -> Self {
        let frame_buffer = FRAMEBUFFER_RESPONSE.framebuffers().next().take().unwrap();

        let width = frame_buffer.width() as usize;
        let height = frame_buffer.height() as usize;
        let bpp = frame_buffer.bpp() as usize;

        let buffer =
            unsafe { core::slice::from_raw_parts_mut(frame_buffer.addr() as _, width * height) };

        Self {
            path: String::new(),
            fbaddr: buffer,
            width,
            height,
            bpp,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn bpp(&self) -> usize {
        self.bpp
    }
}

impl Inode for FrameBuffer {
    fn when_mounted(
        &mut self,
        path: alloc::string::String,
        _father: Option<crate::fs::vfs::inode::InodeRef>,
    ) {
        self.path.clear();
        self.path.push_str(path.as_str());
    }

    fn when_umounted(&mut self) {
        self.path.clear();
    }

    fn size(&self) -> usize {
        self.fbaddr.len()
    }

    fn get_path(&self) -> String {
        self.path.clone()
    }

    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> usize {
        let read_buf_ptr = buf.as_ptr();
        let read_buf_len = buf.len();
        let read_buf: &mut [u32] =
            unsafe { core::slice::from_raw_parts_mut(read_buf_ptr as _, read_buf_len / 4) };

        for index in 0..read_buf.len() {
            read_buf[index] = self.fbaddr[index];
        }

        buf.len()
    }

    fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let real_buf_len = self.size() / self.bpp() - offset;

        if buf.len() > real_buf_len || !buf.len().is_multiple_of(4) {
            return 0;
        }

        let write_buf_ptr = buf.as_ptr() as _;
        let write_buf_len = buf.len();
        let write_buf: &[u32] =
            unsafe { core::slice::from_raw_parts(write_buf_ptr, write_buf_len / 4) };

        for index in 0..write_buf.len() {
            ref_to_mut(self).fbaddr[index] = write_buf[index];
        }

        buf.len()
    }

    fn ioctl(&self, cmd: usize, _arg: usize) -> usize {
        match FbDevIoctlCommand::from(cmd) {
            FbDevIoctlCommand::GetWidth => self.width(),
            FbDevIoctlCommand::GetHeight => self.height(),
        }
    }
}
