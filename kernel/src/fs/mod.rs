use alloc::string::ToString;
use ext2::Ext2Volume;
use fat32::Fat32Volume;
use limine::request::KernelFileRequest;
use spin::{Lazy, Mutex};
use uuid::Uuid;
use vfs::{
    dev::ROOT_PARTITION,
    inode::{mount_to, InodeRef},
    root::RootFS,
};

mod ext2;
mod fat32;
pub mod operation;
pub mod vfs;

pub static ROOT: Lazy<Mutex<InodeRef>> = Lazy::new(|| Mutex::new(RootFS::new()));

#[used]
static KERNEL_FILE_REQUEST: KernelFileRequest = KernelFileRequest::new();

pub fn get_root_partition_uuid() -> Uuid {
    let kernel_file_response = KERNEL_FILE_REQUEST.get_response().unwrap();
    Uuid::from(kernel_file_response.file().gpt_partition_id().unwrap())
}

pub fn init() {
    ROOT.lock().write().when_mounted("/".to_string(), None);

    vfs::dev::init();
    vfs::mnt::init();

    let root_partition = ROOT_PARTITION.lock().clone().unwrap().clone();
    let mut root_fs = Fat32Volume::new(root_partition.clone());
    if root_fs.is_none() {
        root_fs = Ext2Volume::new(root_partition.clone());
    }

    let root_fs = root_fs.unwrap();

    mount_to(root_fs.clone(), ROOT.lock().clone(), "root".to_string());
}
