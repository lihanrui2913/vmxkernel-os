use core::usize;

use alloc::{
    collections::btree_map::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use ext4_rs::{BlockDevice, Ext4, BLOCK_SIZE};
use spin::RwLock;

use crate::ref_to_mut;

use super::{
    operation::kernel_open,
    vfs::inode::{FileInfo, Inode, InodeRef, InodeTy},
};

pub struct Ext4InodeIO {
    inode: InodeRef,
}

impl BlockDevice for Ext4InodeIO {
    fn read_offset(&self, offset: usize) -> alloc::vec::Vec<u8> {
        let mut buf = alloc::vec![0u8; BLOCK_SIZE];
        self.inode.read().read_at(offset, &mut buf);
        buf
    }

    fn write_offset(&self, offset: usize, data: &[u8]) {
        self.inode.read().write_at(offset, data);
    }
}

pub struct Ext4Volume {
    volume: Arc<Ext4>,
    virtual_inodes: BTreeMap<String, InodeRef>,
    path: String,
}

impl Ext4Volume {
    pub fn new(dev: InodeRef) -> Option<InodeRef> {
        let block_device = Arc::new(Ext4InodeIO { inode: dev });
        let volume = Arc::new(Ext4::open(block_device));
        let inode = Self {
            volume,
            virtual_inodes: BTreeMap::new(),
            path: String::new(),
        };

        let inode_ref = Arc::new(RwLock::new(inode));
        inode_ref
            .write()
            .virtual_inodes
            .insert(".".to_string(), inode_ref.clone());

        Some(inode_ref)
    }
}

impl Inode for Ext4Volume {
    fn when_mounted(&mut self, path: String, father: Option<InodeRef>) {
        self.path.clear();
        self.path.push_str(path.as_str());
        if let Some(father) = father {
            self.virtual_inodes.insert("..".to_string(), father);
        }
    }

    fn when_umounted(&mut self) {}

    fn get_path(&self) -> String {
        self.path.clone()
    }

    fn mount(&self, node: InodeRef, name: String) {
        ref_to_mut(self)
            .virtual_inodes
            .insert(name.clone(), node.clone());
    }

    fn open(&self, name: String) -> Option<InodeRef> {
        let self_inode = kernel_open(self.get_path());

        if let Some(inode) = self.virtual_inodes.get(&name) {
            return Some(inode.clone());
        } else {
            return self
                .volume
                .generic_open(name.as_str(), &mut 2, false, 0, &mut 0)
                .ok()
                .map_or_else(
                    || None,
                    |x| {
                        let ty = self.volume.dir_has_entry(x);
                        if ty {
                            let inode = Ext4Dir::new(self.volume.clone(), x);
                            inode
                                .write()
                                .when_mounted(self.get_path() + name.as_str() + "/", self_inode);
                            return Some(inode);
                        } else {
                            let inode = Ext4File::new(self.volume.clone(), x);
                            inode
                                .write()
                                .when_mounted(self.get_path() + name.as_str() + "/", self_inode);
                            return Some(inode);
                        }
                    },
                );
        }
    }

    fn create(&self, name: String, ty: InodeTy) -> Option<InodeRef> {
        let self_inode = kernel_open(self.get_path());

        let inode = self.volume.create(
            2,
            name.as_str(),
            match ty {
                InodeTy::Dir => 0x4000,
                InodeTy::File => 0x8000,
            },
        );

        if inode.is_err() {
            return None;
        }

        let inode_id = inode.unwrap().inode_num;

        match ty {
            InodeTy::Dir => {
                let inode = Ext4Dir::new(self.volume.clone(), inode_id);
                inode
                    .write()
                    .when_mounted(self.get_path() + name.as_str() + "/", self_inode);
                return Some(inode);
            }
            InodeTy::File => {
                let inode = Ext4File::new(self.volume.clone(), inode_id);
                inode
                    .write()
                    .when_mounted(self.get_path() + name.as_str() + "/", self_inode);
                return Some(inode);
            }
        }
    }

    fn inode_type(&self) -> InodeTy {
        InodeTy::Dir
    }

    fn list(&self) -> Vec<FileInfo> {
        let mut vec = Vec::new();
        for (name, inode) in self.virtual_inodes.iter() {
            vec.push(FileInfo::new(name.clone(), inode.read().inode_type()));
        }
        for entry in self.volume.dir_get_entries(2).iter() {
            if entry.get_name() != ".".to_string() && entry.get_name() != "..".to_string() {
                vec.push(FileInfo::new(
                    entry.get_name().clone(),
                    if entry.get_de_type() != 2 {
                        InodeTy::File
                    } else {
                        InodeTy::Dir
                    },
                ))
            }
        }
        vec
    }
}

pub struct Ext4Dir {
    volume: Arc<Ext4>,
    inode_id: u32,
    virtual_inodes: BTreeMap<String, InodeRef>,
    path: String,
}

impl Ext4Dir {
    pub fn new(volume: Arc<Ext4>, inode_id: u32) -> InodeRef {
        let i = Self {
            volume,
            inode_id,
            path: String::new(),
            virtual_inodes: BTreeMap::new(),
        };
        let inode_ref = Arc::new(RwLock::new(i));
        inode_ref
            .write()
            .virtual_inodes
            .insert(".".into(), inode_ref.clone());
        inode_ref
    }
}

impl Inode for Ext4Dir {
    fn when_mounted(&mut self, path: String, father: Option<InodeRef>) {
        self.path.clear();
        self.path.push_str(path.as_str());
        if let Some(father) = father {
            self.virtual_inodes.insert("..".into(), father);
        }
    }

    fn when_umounted(&mut self) {}

    fn get_path(&self) -> String {
        self.path.clone()
    }

    fn mount(&self, node: InodeRef, name: String) {
        ref_to_mut(self)
            .virtual_inodes
            .insert(name.clone(), node.clone());
    }

    fn open(&self, name: String) -> Option<InodeRef> {
        let self_inode = kernel_open(self.get_path());

        if let Some(node) = self.virtual_inodes.get(&name) {
            return Some(node.clone());
        } else {
            return self
                .volume
                .generic_open(
                    name.as_str(),
                    &mut ref_to_mut(self).inode_id,
                    false,
                    0,
                    &mut 0,
                )
                .ok()
                .map_or_else(
                    || None,
                    |x| {
                        let ty = self.volume.dir_has_entry(x);
                        if ty {
                            let inode = Ext4Dir::new(self.volume.clone(), x);
                            inode
                                .write()
                                .when_mounted(self.get_path() + name.as_str() + "/", self_inode);
                            return Some(inode);
                        } else {
                            let inode = Ext4File::new(self.volume.clone(), x);
                            inode
                                .write()
                                .when_mounted(self.get_path() + name.as_str() + "/", self_inode);
                            return Some(inode);
                        }
                    },
                );
        }
    }

    fn create(&self, name: String, ty: InodeTy) -> Option<InodeRef> {
        let self_inode = kernel_open(self.get_path());

        let inode = self.volume.create(
            self.inode_id,
            name.as_str(),
            match ty {
                InodeTy::Dir => 0x4000,
                InodeTy::File => 0x8000,
            },
        );

        if inode.is_err() {
            return None;
        }

        let inode_id = inode.unwrap().inode_num;

        match ty {
            InodeTy::Dir => {
                let inode = Ext4Dir::new(self.volume.clone(), inode_id);
                inode
                    .write()
                    .when_mounted(self.get_path() + name.as_str() + "/", self_inode);
                return Some(inode);
            }
            InodeTy::File => {
                let inode = Ext4File::new(self.volume.clone(), inode_id);
                inode
                    .write()
                    .when_mounted(self.get_path() + name.as_str() + "/", self_inode);
                return Some(inode);
            }
        }
    }

    fn inode_type(&self) -> InodeTy {
        InodeTy::Dir
    }

    fn list(&self) -> Vec<FileInfo> {
        let mut vec = Vec::new();
        for (name, inode) in self.virtual_inodes.iter() {
            vec.push(FileInfo::new(name.clone(), inode.read().inode_type()));
        }
        for entry in self.volume.dir_get_entries(self.inode_id).iter() {
            if entry.get_name() != ".".to_string() && entry.get_name() != "..".to_string() {
                vec.push(FileInfo::new(
                    entry.get_name().clone(),
                    if entry.get_de_type() != 2 {
                        InodeTy::File
                    } else {
                        InodeTy::Dir
                    },
                ))
            }
        }
        vec
    }
}

pub struct Ext4File {
    volume: Arc<Ext4>,
    inode_id: u32,
    path: String,
}

impl Ext4File {
    pub fn new(volume: Arc<Ext4>, inode_id: u32) -> InodeRef {
        let i = Self {
            volume,
            inode_id,
            path: String::new(),
        };
        Arc::new(RwLock::new(i))
    }
}

impl Inode for Ext4File {
    fn when_mounted(&mut self, path: String, _father: Option<InodeRef>) {
        self.path.clear();
        self.path.push_str(path.as_str());
    }

    fn when_umounted(&mut self) {}

    fn get_path(&self) -> String {
        self.path.clone()
    }

    fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        self.volume.read_at(self.inode_id, offset, buf).unwrap_or(0)
    }

    fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        self.volume
            .write_at(self.inode_id, offset, buf)
            .unwrap_or(0)
    }

    fn size(&self) -> usize {
        let ext4_inode = self.volume.get_inode_ref(self.inode_id).inode;
        ext4_inode.size() as usize
    }

    fn inode_type(&self) -> InodeTy {
        InodeTy::File
    }
}
