use alloc::string::String;
use syscall_index::KvmDevIoctlCommand;

use crate::{fs::vfs::inode::Inode, virt::kvm::run_vm};

pub struct KVMInode {
    path: String,
}

impl KVMInode {
    pub fn new() -> Self {
        Self {
            path: String::new(),
        }
    }
}

impl Inode for KVMInode {
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

    fn get_path(&self) -> String {
        self.path.clone()
    }

    fn ioctl(&self, cmd: usize, arg: usize) -> usize {
        match KvmDevIoctlCommand::from(cmd) {
            KvmDevIoctlCommand::KvmRun => run_vm(arg),
            KvmDevIoctlCommand::KvmGetRegs => unimplemented!(),
        }
    }
}
