#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(allocator_api)]
#![feature(naked_functions)]
#![feature(unsigned_is_multiple_of)]
#![feature(vec_into_raw_parts)]

use core::sync::atomic::AtomicBool;

pub mod arch;
pub mod device;
pub mod fs;
pub mod memory;
pub mod syscall;
pub mod task;
pub mod virt;

pub extern crate alloc;

pub static START_SCHEDULE: AtomicBool = AtomicBool::new(false);

pub fn init() {
    memory::init_heap();
    device::log::init();
    arch::smp::CPUS.write().init_bsp();
    arch::interrupts::IDT.load();
    arch::smp::CPUS.write().init_ap();
    arch::apic::init();
    arch::interrupts::init();
    device::mouse::init();
    device::pci::init();
    device::nvme::init();
    device::xhci::init();
    device::fpu::init();
    virt::kvm::init();
    syscall::init();
    task::init();
}

pub fn addr_of<T>(reffer: &T) -> usize {
    reffer as *const T as usize
}

pub fn ref_to_mut<T>(reffer: &T) -> &mut T {
    unsafe { &mut *(addr_of(reffer) as *const T as *mut T) }
}

pub fn ref_to_static<T>(reffer: &T) -> &'static T {
    unsafe { &*(addr_of(reffer) as *const T) }
}

#[macro_export]
macro_rules! unsafe_trait_impl {
    ($struct: ident, $trait: ident) => {
        unsafe impl $trait for $struct {}
    };
    ($struct: ident, $trait: ident, $life: tt) => {
        unsafe impl<$life> $trait for $struct<$life> {}
    };
}
