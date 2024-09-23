use alloc::string::ToString;

use crate::fs::ROOT;

use super::{inode::mount_to, root::RootFS};

pub fn init() {
    let mnt_fs = RootFS::new();
    mount_to(mnt_fs.clone(), ROOT.lock().clone(), "mnt".to_string());
}
