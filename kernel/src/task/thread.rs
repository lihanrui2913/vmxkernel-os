use alloc::boxed::Box;
use alloc::sync::{Arc, Weak};
use core::fmt::Debug;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::RwLock;
use x86_64::instructions::interrupts;
use x86_64::registers::control::{Cr4, Cr4Flags};

use super::context::Context;
use super::process::{WeakSharedProcess, KERNEL_PROCESS};
use super::scheduler::SCHEDULER;
use super::stack::{KernelStack, UserStack};
use crate::arch::gdt::Selectors;
use crate::device::fpu::FpState;
use crate::memory::{ExtendedPageTable, KERNEL_PAGE_TABLE};

pub(super) type SharedThread = Arc<RwLock<Box<Thread>>>;
pub(super) type WeakSharedThread = Weak<RwLock<Box<Thread>>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ThreadId(pub u64);

impl ThreadId {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        ThreadId(NEXT_ID.fetch_add(1, Ordering::SeqCst))
    }
}

pub struct Thread {
    pub id: ThreadId,
    pub kernel_stack: KernelStack,
    pub context: Context,
    pub process: WeakSharedProcess,
    pub fpu_context: FpState,
    pub fsbase: usize,
}

impl Thread {
    pub fn new(process: WeakSharedProcess) -> Self {
        let thread = Thread {
            id: ThreadId::new(),
            context: Context::default(),
            kernel_stack: KernelStack::new(),
            process,
            fpu_context: FpState::new(),
            fsbase: 0,
        };

        thread
    }

    pub fn get_init_thread() -> WeakSharedThread {
        let thread = Self::new(Arc::downgrade(&KERNEL_PROCESS));
        let thread = Arc::new(RwLock::new(Box::new(thread)));
        KERNEL_PROCESS.write().threads.push(thread.clone());
        Arc::downgrade(&thread)
    }

    pub fn new_kernel_thread(function: fn()) {
        let mut thread = Self::new(Arc::downgrade(&KERNEL_PROCESS));

        thread.context.init(
            function as usize,
            thread.kernel_stack.end_address(),
            KERNEL_PAGE_TABLE.lock().physical_address(),
            Selectors::get_kernel_segments(),
        );

        let thread = Arc::new(RwLock::new(Box::new(thread)));
        KERNEL_PROCESS.write().threads.push(thread.clone());

        interrupts::without_interrupts(|| {
            SCHEDULER.lock().add(Arc::downgrade(&thread));
        });
    }

    pub fn new_user_thread(process: WeakSharedProcess, entry_point: usize) {
        let mut thread = Self::new(process.clone());
        let process = process.upgrade().unwrap();
        let mut process = process.write();
        let user_stack = UserStack::new(&mut process.page_table);

        thread.context.init(
            entry_point,
            user_stack.end_address,
            process.page_table.physical_address(),
            Selectors::get_user_segments(),
        );

        thread.save_fsbase();

        let thread = Arc::new(RwLock::new(Box::new(thread)));
        process.threads.push(thread.clone());

        SCHEDULER.lock().add(Arc::downgrade(&thread));
    }

    pub fn save_fsbase(&mut self) {
        unsafe {
            if Cr4::read().contains(Cr4Flags::FSGSBASE) {
                self.fsbase = x86::current::segmentation::rdfsbase() as usize;
            } else {
                self.fsbase = x86::msr::rdmsr(x86::msr::IA32_FS_BASE) as usize;
            }
        }
    }

    pub fn restore_fsbase(&self) {
        unsafe {
            if Cr4::read().contains(Cr4Flags::FSGSBASE) {
                x86::current::segmentation::wrfsbase(self.fsbase as u64);
            } else {
                x86::msr::wrmsr(x86::msr::IA32_FS_BASE, self.fsbase as u64);
            }
        }
    }
}
