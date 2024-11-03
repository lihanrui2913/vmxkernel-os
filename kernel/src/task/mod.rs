use alloc::{boxed::Box, sync::Arc};
use process::{Process, ProcessId};
use scheduler::SCHEDULER;
use spin::RwLock;
use thread::Thread;

pub mod context;
pub mod process;
pub mod scheduler;
pub mod stack;
pub mod thread;

pub use self::scheduler::init;

#[inline]
pub fn get_current_thread() -> Arc<RwLock<Box<Thread>>> {
    SCHEDULER.lock().current_thread().upgrade().unwrap()
}

#[inline]
pub fn get_current_process() -> Arc<RwLock<Box<Process>>> {
    get_current_thread().read().process.upgrade().unwrap()
}

#[inline]
pub fn get_current_process_id() -> ProcessId {
    get_current_process().read().id
}
