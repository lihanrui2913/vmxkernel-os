use core::{
    sync::atomic::{AtomicUsize, Ordering},
    usize,
};

use crate::{ref_to_mut, task::process::ProcessId};
use alloc::{collections::BTreeMap, string::String, sync::Arc, vec::Vec};
use spin::Mutex;

use crate::task::get_current_process_id;

use super::{
    fat32::Fat32Volume,
    vfs::inode::{mount_to, FileInfo, InodeRef, InodeTy},
    ROOT,
};

static FILE_DESCRIPTOR_MANAGERS: Mutex<BTreeMap<ProcessId, Arc<FileDescriptorManager>>> =
    Mutex::new(BTreeMap::new());

pub enum OpenMode {
    Read,
    Write,
}

impl From<usize> for OpenMode {
    fn from(mode: usize) -> Self {
        match mode {
            0 => Self::Read,
            1 => Self::Write,
            _ => panic!("Unknown open mode!!!"),
        }
    }
}

type FileDescriptor = usize;
type FileTuple = (InodeRef, OpenMode, usize);

struct FileDescriptorManager {
    file_descriptors: BTreeMap<FileDescriptor, FileTuple>,
    file_descriptor_allocator: AtomicUsize,
    cwd: Mutex<InodeRef>,
}

impl FileDescriptorManager {
    pub fn new(file_descriptors: BTreeMap<FileDescriptor, FileTuple>) -> Self {
        Self {
            file_descriptors,
            file_descriptor_allocator: AtomicUsize::new(3), // 0, 1, and 2 are reserved for stdin, stdout, and stderr
            cwd: Mutex::new(ROOT.lock().clone()),
        }
    }

    pub fn get_new_fd(&self) -> FileDescriptor {
        self.file_descriptor_allocator
            .fetch_add(1, Ordering::SeqCst)
    }

    pub fn add_inode(&self, inode: InodeRef, mode: OpenMode) -> FileDescriptor {
        let new_fd = self.get_new_fd();
        ref_to_mut(self)
            .file_descriptors
            .insert(new_fd, (inode, mode, 0));
        new_fd
    }

    pub fn change_cwd(&self, path: String) {
        if let Some(inode) = get_inode_by_path(path) {
            if inode.read().inode_type() == InodeTy::Dir {
                *self.cwd.lock() = inode;
            }
        }
    }

    pub fn get_cwd(&self) -> String {
        self.cwd.lock().read().get_path()
    }
}

fn get_file_descriptor_manager<'a>() -> Option<Arc<FileDescriptorManager>> {
    let pid = get_current_process_id();

    FILE_DESCRIPTOR_MANAGERS.lock().get_mut(&pid).cloned()
}

pub fn init_file_descriptor_manager(pid: ProcessId) {
    let mut file_descriptor_managers = FILE_DESCRIPTOR_MANAGERS.lock();
    file_descriptor_managers.insert(pid, Arc::new(FileDescriptorManager::new(BTreeMap::new())));
}

pub fn init_file_descriptor_manager_with_stdin_stdout(
    pid: ProcessId,
    stdin: InodeRef,
    stdout: InodeRef,
) {
    let mut file_descriptor_managers = FILE_DESCRIPTOR_MANAGERS.lock();

    let mut file_descriptors = BTreeMap::new();
    file_descriptors.insert(0, (stdin.clone(), OpenMode::Read, 0));
    file_descriptors.insert(1, (stdout.clone(), OpenMode::Write, 0));

    file_descriptor_managers.insert(pid, Arc::new(FileDescriptorManager::new(file_descriptors)));
}

fn get_inode_by_path(path: String) -> Option<InodeRef> {
    let root = ROOT.lock().clone();

    let path = path.split("/");

    let node = root;

    for path_node in path {
        if path_node.len() > 0 {
            if let Some(child) = node.read().open(String::from(path_node)) {
                core::mem::drop(core::mem::replace(ref_to_mut(&node), child));
            } else {
                return None;
            }
        }
    }

    Some(node.clone())
}

pub fn kernel_open(path: String) -> Option<InodeRef> {
    get_inode_by_path(path)
}

pub fn get_inode_by_fd(file_descriptor: usize) -> Option<InodeRef> {
    let current_file_descriptor_manager = get_file_descriptor_manager()?;

    let (inode, _, _) = current_file_descriptor_manager
        .file_descriptors
        .get(&file_descriptor)?;

    Some(inode.clone())
}

pub fn open(path: String, open_mode: OpenMode) -> Option<usize> {
    let current_file_descriptor_manager = get_file_descriptor_manager()?;

    let inode = if path.starts_with("/") {
        get_inode_by_path(path.clone())?
    } else {
        get_inode_by_path(alloc::format!(
            "{}{}",
            current_file_descriptor_manager.get_cwd(),
            path.clone()
        ))?
    };

    let file_descriptor = current_file_descriptor_manager.add_inode(inode, open_mode);

    Some(file_descriptor)
}

pub fn read(fd: FileDescriptor, buf: &mut [u8]) -> usize {
    let current_file_descriptor_manager = get_file_descriptor_manager();
    if let None = current_file_descriptor_manager {
        return 0;
    }
    let current_file_descriptor_manager = current_file_descriptor_manager.unwrap();

    if let Some((inode, _, offset)) = current_file_descriptor_manager.file_descriptors.get(&fd) {
        inode.read().read_at(*offset, buf)
    } else {
        0
    }
}

pub fn write(fd: FileDescriptor, buf: &[u8]) -> usize {
    if let Some(current_file_descriptor_manager) = get_file_descriptor_manager() {
        if let Some((inode, mode, offset)) =
            current_file_descriptor_manager.file_descriptors.get(&fd)
        {
            match mode {
                OpenMode::Write => inode.read().write_at(*offset, buf),

                _ => 0,
            }
        } else {
            0
        }
    } else {
        0
    }
}

pub fn lseek(fd: FileDescriptor, offset: usize) -> Option<()> {
    let current_file_descriptor_manager = get_file_descriptor_manager()?;

    let (_, _, old_offset) = ref_to_mut(current_file_descriptor_manager.as_ref())
        .file_descriptors
        .get_mut(&fd)?;
    *old_offset = offset;

    Some(())
}

pub fn close(fd: FileDescriptor) -> Option<()> {
    let current_file_descriptor_manager = get_file_descriptor_manager()?;
    ref_to_mut(current_file_descriptor_manager.as_ref())
        .file_descriptors
        .remove(&fd)?;
    Some(())
}

pub fn fsize(fd: FileDescriptor) -> Option<usize> {
    let current_file_descriptor_manager = get_file_descriptor_manager()?;

    let (inode, _, _) = ref_to_mut(current_file_descriptor_manager.as_ref())
        .file_descriptors
        .get_mut(&fd)?;

    let size = inode.read().size();

    Some(size)
}

pub fn list_dir(path: String) -> Vec<FileInfo> {
    if let Some(inode) = get_inode_by_path(path) {
        if inode.read().inode_type() == InodeTy::Dir {
            let mut list = inode.read().list();
            list.sort();

            let mut slow = 0;
            for fast in 0..list.len() {
                if list[fast] != list[slow] && fast != slow {
                    list[slow] = list[fast].clone();
                    slow += 1;
                }
                if slow == 0 {
                    slow += 1;
                }
            }

            let mut new = list[0..slow].to_vec();
            new.sort();

            return new;
        }
    }
    Vec::new()
}

pub fn change_cwd(path: String) {
    if let Some(current_file_descriptor_manager) = get_file_descriptor_manager() {
        if path.starts_with("/") {
            current_file_descriptor_manager.change_cwd(path);
        } else {
            let current = current_file_descriptor_manager.get_cwd();
            let new = alloc::format!("{}{}", current, path);
            current_file_descriptor_manager.change_cwd(new);
        }
    }
}

pub fn get_cwd() -> String {
    if let Some(current_file_descriptor_manager) = get_file_descriptor_manager() {
        current_file_descriptor_manager.get_cwd()
    } else {
        String::from("/")
    }
}

pub fn create(path: String, ty: InodeTy) -> Option<FileDescriptor> {
    if let Some(current_file_descriptor_manager) = get_file_descriptor_manager() {
        if path.starts_with("/") {
            let mut name = String::new();
            let parrent_path = {
                let mut path = path.clone();
                while !path.ends_with("/") {
                    name.push(path.pop().unwrap());
                }
                path
            };
            let parent = get_inode_by_path(parrent_path)?;
            parent.read().create(name.clone(), ty)?;
            open(path, OpenMode::Write)
        } else {
            let cwd = current_file_descriptor_manager.get_cwd();
            let parent = get_inode_by_path(cwd.clone())?;
            parent.read().create(path.clone(), ty)?;
            open(cwd + path.as_str(), OpenMode::Write)
        }
    } else {
        None
    }
}

pub fn get_type(fd: FileDescriptor) -> Option<InodeTy> {
    if let Some(current_file_descriptor_manager) = get_file_descriptor_manager() {
        let (inode, _, _) = current_file_descriptor_manager.file_descriptors.get(&fd)?;
        Some(inode.read().inode_type())
    } else {
        None
    }
}

pub fn mount(to: String, partition_path: String) -> Option<()> {
    let partition_inode = get_inode_by_path(partition_path)?;
    let to_father_path = {
        let mut path = to.clone();
        if path.ends_with("/") {
            path.pop().unwrap();
        }

        while !path.ends_with("/") {
            path.pop().unwrap();
        }
        path
    };
    let to_father = get_inode_by_path(to_father_path)?;
    let to = get_inode_by_path(to)?;

    let to_name = {
        let mut path = to.read().get_path();
        let mut name = String::new();
        if path.ends_with("/") {
            path.pop().unwrap();
        }
        while !path.ends_with("/") {
            name.push(path.pop().unwrap());
        }
        name.chars().rev().collect()
    };

    let volumne = Fat32Volume::new(partition_inode.clone());
    mount_to(volumne, to_father, to_name);
    Some(())
}

pub fn ioctl(fd: FileDescriptor, cmd: usize, arg: usize) -> usize {
    let inode = get_inode_by_fd(fd);
    if let Some(inode) = inode {
        return inode.read().ioctl(cmd, arg);
    }

    return usize::MAX;
}
