use core::usize;

use alloc::{
    collections::btree_map::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use efs::{
    dev::Device,
    file::{Directory, TypeWithFile},
    fs::{
        ext2::{
            error::Ext2Error,
            file::{Directory as FileDirectory, Regular as FileRegular},
            Ext2Fs,
        },
        FileSystem,
    },
    io::{Base, Read, Seek, SeekFrom, Write},
    path::UnixStr,
};
use spin::RwLock;

use crate::ref_to_mut;

use super::{
    operation::kernel_open,
    vfs::inode::{FileInfo, Inode, InodeRef, InodeTy},
};

pub struct Ext2InodeIO {
    offset: usize,
    inode: InodeRef,
}

impl Ext2InodeIO {
    pub fn new(inode: InodeRef) -> Self {
        Self { offset: 0, inode }
    }
}

impl Base for Ext2InodeIO {
    type FsError = Ext2Error;
}

impl Read for Ext2InodeIO {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, efs::error::Error<Self::FsError>> {
        self.inode.read().read_at(self.offset, buf);
        self.seek(SeekFrom::Current(buf.len() as i64))?;
        Ok(buf.len())
    }
}

impl Write for Ext2InodeIO {
    fn write(&mut self, buf: &[u8]) -> Result<usize, efs::error::Error<Self::FsError>> {
        self.inode.read().write_at(self.offset, buf);
        self.seek(SeekFrom::Current(buf.len() as i64))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), efs::error::Error<Self::FsError>> {
        Ok(())
    }
}

impl Seek for Ext2InodeIO {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, efs::error::Error<Self::FsError>> {
        match pos {
            SeekFrom::Current(i) => self.offset = (self.offset as i64 + i) as usize,
            SeekFrom::Start(i) => self.offset = i as usize,
            SeekFrom::End(i) => {
                let size = self.inode.read().size();
                self.offset = size - i as usize;
            }
        }
        Ok(self.offset as u64)
    }
}

pub struct Ext2Volume {
    volume: Arc<Ext2Fs<Ext2InodeIO>>,
    virtual_inodes: BTreeMap<String, InodeRef>,
    path: String,
}

impl Ext2Volume {
    pub fn new(dev: InodeRef) -> Option<InodeRef> {
        let block_device = Ext2InodeIO::new(dev);
        let volume = Arc::new(Ext2Fs::new(block_device, 0, false).ok()?);
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

impl Inode for Ext2Volume {
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
            let entry = self
                .volume
                .root()
                .ok()?
                .entry(UnixStr::new(name.as_str()).unwrap())
                .ok()??;

            if entry.is_directory() {
                if let TypeWithFile::Directory(dir) = entry {
                    let dir = Ext2Dir::new(dir);
                    dir.write()
                        .when_mounted(self.get_path() + name.as_str() + "/", self_inode);
                    return Some(dir);
                } else {
                    return None;
                }
            } else {
                if let TypeWithFile::Regular(file) = entry {
                    let file = Ext2File::new(file);
                    file.write()
                        .when_mounted(self.get_path() + name.as_str() + "/", self_inode);
                    return Some(file);
                } else {
                    return None;
                }
            }
        }
    }

    fn create(&self, name: String, ty: InodeTy) -> Option<InodeRef> {
        self.volume
            .root()
            .ok()?
            .add_entry(
                UnixStr::new(name.as_str()).unwrap(),
                match ty {
                    InodeTy::Dir => efs::file::Type::Directory,
                    InodeTy::File => efs::file::Type::Regular,
                },
                efs::permissions::Permissions::empty(),
                efs::types::Uid(0),
                efs::types::Gid(0),
            )
            .ok()?;
        self.open(name)
    }

    fn inode_type(&self) -> InodeTy {
        InodeTy::Dir
    }

    fn list(&self) -> Vec<FileInfo> {
        let mut vec = Vec::new();
        for (name, inode) in self.virtual_inodes.iter() {
            vec.push(FileInfo::new(name.clone(), inode.read().inode_type()));
        }
        for entry in self.volume.root().unwrap().entries().unwrap() {
            if entry.filename.to_string() != ".".to_string()
                && entry.filename.to_string() != "..".to_string()
            {
                if entry.file.is_directory() {
                    vec.push(FileInfo::new(entry.filename.to_string(), InodeTy::Dir));
                } else {
                    vec.push(FileInfo::new(entry.filename.to_string(), InodeTy::File));
                }
            }
        }
        vec
    }
}

pub struct Ext2Dir {
    dir: FileDirectory<Ext2InodeIO>,
    virtual_inodes: BTreeMap<String, InodeRef>,
    path: String,
}

impl Ext2Dir {
    pub fn new(dir: FileDirectory<Ext2InodeIO>) -> InodeRef {
        let i = Self {
            dir,
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

impl Inode for Ext2Dir {
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

        if let Some(inode) = self.virtual_inodes.get(&name) {
            return Some(inode.clone());
        } else {
            let entry = self
                .dir
                .entry(UnixStr::new(name.as_str()).unwrap())
                .ok()??;

            if entry.is_directory() {
                if let TypeWithFile::Directory(dir) = entry {
                    let dir = Ext2Dir::new(dir);
                    dir.write()
                        .when_mounted(self.get_path() + name.as_str() + "/", self_inode);
                    return Some(dir);
                } else {
                    return None;
                }
            } else {
                if let TypeWithFile::Regular(file) = entry {
                    let file = Ext2File::new(file);
                    file.write()
                        .when_mounted(self.get_path() + name.as_str() + "/", self_inode);
                    return Some(file);
                } else {
                    return None;
                }
            }
        }
    }

    fn create(&self, name: String, ty: InodeTy) -> Option<InodeRef> {
        ref_to_mut(self)
            .dir
            .add_entry(
                UnixStr::new(name.as_str()).unwrap(),
                match ty {
                    InodeTy::Dir => efs::file::Type::Directory,
                    InodeTy::File => efs::file::Type::Regular,
                },
                efs::permissions::Permissions::empty(),
                efs::types::Uid(0),
                efs::types::Gid(0),
            )
            .ok()?;
        self.open(name)
    }

    fn inode_type(&self) -> InodeTy {
        InodeTy::Dir
    }

    fn list(&self) -> Vec<FileInfo> {
        let mut vec = Vec::new();
        for (name, inode) in self.virtual_inodes.iter() {
            vec.push(FileInfo::new(name.clone(), inode.read().inode_type()));
        }
        for entry in self.dir.entries().unwrap() {
            if entry.filename.to_string() != ".".to_string()
                && entry.filename.to_string() != "..".to_string()
            {
                if entry.file.is_directory() {
                    vec.push(FileInfo::new(entry.filename.to_string(), InodeTy::Dir));
                } else {
                    vec.push(FileInfo::new(entry.filename.to_string(), InodeTy::File));
                }
            }
        }
        vec
    }
}

pub struct Ext2File {
    file: FileRegular<Ext2InodeIO>,
    path: String,
}

impl Ext2File {
    pub fn new(file: FileRegular<Ext2InodeIO>) -> InodeRef {
        let i = Self {
            file,
            path: String::new(),
        };
        Arc::new(RwLock::new(i))
    }
}

impl Inode for Ext2File {
    fn when_mounted(&mut self, path: String, _father: Option<InodeRef>) {
        self.path.clear();
        self.path.push_str(path.as_str());
    }

    fn when_umounted(&mut self) {}

    fn get_path(&self) -> String {
        self.path.clone()
    }

    fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let res = ref_to_mut(self).file.seek(SeekFrom::Start(offset as u64));
        if res.is_err() {
            return 0;
        }
        let res = ref_to_mut(self).file.read(buf);
        if res.is_err() {
            return 0;
        }
        let res = ref_to_mut(self).file.seek(SeekFrom::Start(0));
        if res.is_err() {
            return 0;
        }
        buf.len()
    }

    fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let res = ref_to_mut(self).file.seek(SeekFrom::Start(offset as u64));
        if res.is_err() {
            return 0;
        }
        let res = ref_to_mut(self).file.write(buf);
        if res.is_err() {
            return 0;
        }
        let res = ref_to_mut(self).file.seek(SeekFrom::Start(0));
        if res.is_err() {
            return 0;
        }

        buf.len()
    }

    fn size(&self) -> usize {
        ref_to_mut(self).file.size().0 as usize
    }

    fn inode_type(&self) -> InodeTy {
        InodeTy::File
    }
}
