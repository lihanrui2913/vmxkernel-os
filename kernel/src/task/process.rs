use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::fmt::Debug;
use core::sync::atomic::{AtomicU64, Ordering};
use object::{File, Object, ObjectSegment};
use spin::{Lazy, RwLock};
use x86_64::instructions::interrupts;
use x86_64::structures::paging::OffsetPageTable;
use x86_64::VirtAddr;

use super::thread::{SharedThread, Thread};
use crate::fs::operation::init_file_descriptor_manager_with_stdin_stdout;
use crate::fs::vfs::dev::terminal::Terminal;
use crate::memory::KERNEL_PAGE_TABLE;
use crate::memory::{ExtendedPageTable, MappingType, MemoryManager};

pub(super) type SharedProcess = Arc<RwLock<Box<Process>>>;
pub(super) type WeakSharedProcess = Weak<RwLock<Box<Process>>>;

static PROCESSES: RwLock<VecDeque<SharedProcess>> = RwLock::new(VecDeque::new());
pub static KERNEL_PROCESS: Lazy<SharedProcess> = Lazy::new(|| Process::new_kernel_process());

const KERNEL_PROCESS_NAME: &str = "kernel";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProcessId(pub u64);

impl ProcessId {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        ProcessId(NEXT_ID.fetch_add(1, Ordering::SeqCst))
    }
}

impl From<u64> for ProcessId {
    fn from(value: u64) -> Self {
        ProcessId(value)
    }
}

#[allow(dead_code)]
pub struct Process {
    pub id: ProcessId,
    pub name: String,
    pub page_table: OffsetPageTable<'static>,
    pub threads: Vec<SharedThread>,
    pub args_value: usize,
    pub args_len: usize,
}

impl Process {
    pub fn new(name: &str) -> Self {
        let process = Process {
            id: ProcessId::new(),
            name: String::from(name),
            page_table: unsafe { KERNEL_PAGE_TABLE.lock().deep_copy() },
            threads: Default::default(),
            args_value: 0,
            args_len: 0,
        };

        process
    }

    pub fn new_kernel_process() -> SharedProcess {
        let process = Arc::new(RwLock::new(Box::new(Self::new(KERNEL_PROCESS_NAME))));
        PROCESSES.write().push_back(process.clone());
        process
    }

    pub fn new_user_process(name: &str, elf_data: &'static [u8]) -> Option<SharedProcess> {
        let binary = ProcessBinary::parse(elf_data)?;
        let process = interrupts::without_interrupts(|| {
            let process = Arc::new(RwLock::new(Box::new(Self::new(name))));
            ProcessBinary::map_segments(&binary, &mut process.write().page_table);
            Thread::new_user_thread(Arc::downgrade(&process), binary.entry() as usize);
            PROCESSES.write().push_back(process.clone());
            process
        });
        init_file_descriptor_manager_with_stdin_stdout(
            process.read().id,
            Arc::new(RwLock::new(Terminal::new())),
            Arc::new(RwLock::new(Terminal::new())),
        );

        return Some(process);
    }

    pub fn exit_process(&self) {
        let mut processes = PROCESSES.write();
        if let Some(index) = processes
            .iter()
            .position(|process| process.read().id == self.id)
        {
            processes.remove(index);
        }
    }
}

struct ProcessBinary;

impl ProcessBinary {
    fn parse(bin: &'static [u8]) -> Option<File<'static>> {
        let file = File::parse(bin);
        if file.is_err() {
            return None;
        }
        Some(file.expect("Failed to parse ELF binary"))
    }

    fn map_segments(elf_file: &File, page_table: &mut OffsetPageTable<'static>) {
        for segment in elf_file.segments() {
            MemoryManager::alloc_range(
                VirtAddr::new(segment.address() as u64),
                segment.size(),
                MappingType::UserCode.flags(),
                page_table,
            )
            .expect("Failed to allocate memory for ELF segment");

            if let Ok(data) = segment.data() {
                page_table.write_to_mapped_address(data, VirtAddr::new(segment.address()));
            }
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        unsafe { self.page_table.free_user_page_table() };
    }
}

pub fn is_process_exited(pid: usize) -> bool {
    for process in PROCESSES.read().iter() {
        if process.read().id == ProcessId::from(pid as u64) {
            return false;
        }
    }

    return true;
}
