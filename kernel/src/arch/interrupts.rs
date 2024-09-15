use alloc::collections::btree_map::BTreeMap;
use spin::Lazy;
use spin::Mutex;
use x86_64::instructions::port::PortReadOnly;
use x86_64::registers::control::Cr2;
use x86_64::structures::idt::InterruptDescriptorTable;
use x86_64::structures::idt::InterruptStackFrame;
use x86_64::structures::idt::PageFaultErrorCode;
use x86_64::VirtAddr;

use super::gdt::DOUBLE_FAULT_IST_INDEX;
use crate::arch::apic::LAPIC;
use crate::task::scheduler::SCHEDULER;

const INTERRUPT_INDEX_OFFSET: u8 = 32;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = INTERRUPT_INDEX_OFFSET,
    ApicError,
    ApicSpurious,
    Keyboard,
    Mouse,
}

macro_rules! interrupt_handler {
    ($k: expr) => {{
        extern "x86-interrupt" fn default(frame: InterruptStackFrame) {
            IRQ_HANDLER.lock()($k as usize, frame);
        }
        default
    }};
}

macro_rules! interrupt_handler10 {
    ($k: expr, $idt: expr) => {
        $idt[$k * 10 + 0 + INTERRUPT_INDEX_OFFSET].set_handler_fn(interrupt_handler!($k * 10 + 0));
        $idt[$k * 10 + 1 + INTERRUPT_INDEX_OFFSET].set_handler_fn(interrupt_handler!($k * 10 + 1));
        $idt[$k * 10 + 2 + INTERRUPT_INDEX_OFFSET].set_handler_fn(interrupt_handler!($k * 10 + 2));
        $idt[$k * 10 + 3 + INTERRUPT_INDEX_OFFSET].set_handler_fn(interrupt_handler!($k * 10 + 3));
        $idt[$k * 10 + 4 + INTERRUPT_INDEX_OFFSET].set_handler_fn(interrupt_handler!($k * 10 + 4));
        $idt[$k * 10 + 5 + INTERRUPT_INDEX_OFFSET].set_handler_fn(interrupt_handler!($k * 10 + 5));
        $idt[$k * 10 + 6 + INTERRUPT_INDEX_OFFSET].set_handler_fn(interrupt_handler!($k * 10 + 6));
        $idt[$k * 10 + 7 + INTERRUPT_INDEX_OFFSET].set_handler_fn(interrupt_handler!($k * 10 + 7));
        $idt[$k * 10 + 8 + INTERRUPT_INDEX_OFFSET].set_handler_fn(interrupt_handler!($k * 10 + 8));
        $idt[$k * 10 + 9 + INTERRUPT_INDEX_OFFSET].set_handler_fn(interrupt_handler!($k * 10 + 9));
    };
}

pub static IDT: Lazy<InterruptDescriptorTable> = Lazy::new(|| {
    let mut idt = InterruptDescriptorTable::new();

    idt.breakpoint.set_handler_fn(breakpoint);
    idt.segment_not_present.set_handler_fn(segment_not_present);
    idt.invalid_opcode.set_handler_fn(invalid_opcode);
    idt.page_fault.set_handler_fn(page_fault);
    idt.general_protection_fault
        .set_handler_fn(general_protection_fault);

    interrupt_handler10!(0, idt);
    interrupt_handler10!(1, idt);
    interrupt_handler10!(2, idt);
    interrupt_handler10!(3, idt);
    interrupt_handler10!(4, idt);
    interrupt_handler10!(5, idt);
    interrupt_handler10!(6, idt);
    interrupt_handler10!(7, idt);
    interrupt_handler10!(8, idt);
    interrupt_handler10!(9, idt);
    interrupt_handler10!(10, idt);
    interrupt_handler10!(11, idt);
    interrupt_handler10!(12, idt);
    interrupt_handler10!(13, idt);
    interrupt_handler10!(14, idt);
    interrupt_handler10!(15, idt);
    interrupt_handler10!(16, idt);
    interrupt_handler10!(17, idt);
    interrupt_handler10!(18, idt);
    interrupt_handler10!(19, idt);
    interrupt_handler10!(20, idt);
    interrupt_handler10!(21, idt);

    idt[InterruptIndex::Timer as u8].set_handler_fn(timer_interrupt);
    idt[InterruptIndex::ApicError as u8].set_handler_fn(lapic_error);
    idt[InterruptIndex::ApicSpurious as u8].set_handler_fn(spurious_interrupt);

    unsafe {
        idt.double_fault
            .set_handler_fn(double_fault)
            .set_stack_index(DOUBLE_FAULT_IST_INDEX as u16);
    }

    return idt;
});

pub fn init() {
    register_irq(InterruptIndex::Keyboard as usize, keyboard_interrupt);
    register_irq(InterruptIndex::Mouse as usize, mouse_interrupt);
}

#[naked]
extern "x86-interrupt" fn timer_interrupt(_frame: InterruptStackFrame) {
    fn timer_handler(context: VirtAddr) -> VirtAddr {
        super::apic::end_of_interrupt();
        let mut scheduler = SCHEDULER.lock();

        let address = scheduler.schedule(context);

        address
    }

    unsafe {
        core::arch::asm!(
            "cli",
            crate::push_context!(),
            "mov rdi, rsp",
            "call {timer_handler}",
            "mov rsp, rax",
            crate::pop_context!(),
            "sti",
            "iretq",
            timer_handler = sym timer_handler,
            options(noreturn)
        );
    }
}

extern "x86-interrupt" fn lapic_error(_frame: InterruptStackFrame) {
    log::error!("Local APIC error!");
    super::apic::end_of_interrupt();
}

extern "x86-interrupt" fn spurious_interrupt(_frame: InterruptStackFrame) {
    log::debug!("Received spurious interrupt!");
    super::apic::end_of_interrupt();
}

extern "x86-interrupt" fn segment_not_present(frame: InterruptStackFrame, error_code: u64) {
    log::error!("Exception: Segment Not Present\n{:#?}", frame);
    log::error!("Error Code: {:#x}", error_code);
    panic!("Unrecoverable fault occured, halting!");
}

extern "x86-interrupt" fn general_protection_fault(frame: InterruptStackFrame, error_code: u64) {
    log::error!("Exception: General Protection Fault\n{:#?}", frame);
    log::error!("Error Code: {:#x}", error_code);
    if (error_code & 0x1) != 0 {
        log::error!("The exception occurred during delivery of an event external to the program,such as an interrupt or an earlier exception.");
    }
    if (error_code & 0x2) != 0 {
        log::error!("Refers to a gate descriptor in the IDT")
    } else {
        if (error_code & 0x4) != 0 {
            log::error!("Refers to a segment or gate descriptor in the LDT");
        } else {
            log::error!("Refers to a segment or gate descriptor in the LDT");
        }
    }

    log::error!("Segment Selector Index: {}", error_code & 0xfff8);

    panic!();
}

extern "x86-interrupt" fn invalid_opcode(frame: InterruptStackFrame) {
    log::error!("Exception: Invalid Opcode\n{:#?}", frame);
    panic!();
}

extern "x86-interrupt" fn breakpoint(frame: InterruptStackFrame) {
    log::debug!("Exception: Breakpoint\n{:#?}", frame);
}

extern "x86-interrupt" fn double_fault(frame: InterruptStackFrame, error_code: u64) -> ! {
    log::error!("Exception: Double Fault\n{:#?}", frame);
    log::error!("Error Code: {:#x}", error_code);
    log::error!("Unrecoverable fault occured, halting!");
    loop {
        x86_64::instructions::hlt();
    }
}

fn keyboard_interrupt(_irq: usize, _frame: InterruptStackFrame) {
    let scancode: u8 = unsafe { PortReadOnly::new(0x60).read() };
    crate::device::keyboard::add_scancode(scancode);
    super::apic::end_of_interrupt();
}

fn mouse_interrupt(_irq: usize, _frame: InterruptStackFrame) {
    let packet = unsafe { PortReadOnly::new(0x60).read() };
    crate::device::mouse::MOUSE.lock().process_packet(packet);
    super::apic::end_of_interrupt();
}

extern "x86-interrupt" fn page_fault(frame: InterruptStackFrame, error_code: PageFaultErrorCode) {
    log::warn!("Processor: {}", unsafe { LAPIC.lock().id() });
    log::warn!("Exception: Page Fault\n{:#?}", frame);
    log::warn!("Error Code: {:?}", error_code);
    match Cr2::read() {
        Ok(address) => {
            log::warn!("Fault Address: {:#x}", address);
        }
        Err(error) => {
            log::warn!("Invalid virtual address: {:?}", error);
        }
    }
    panic!();
}

pub type IrqHandler = fn(irq: usize, frame: InterruptStackFrame);

pub static IRQ_HANDLER: Mutex<IrqHandler> = Mutex::new(do_irq);

pub static IRQ_HANDLERS_TABLE: Mutex<BTreeMap<usize, IrqHandler>> = Mutex::new(BTreeMap::new());

pub fn do_irq(irq: usize, frame: InterruptStackFrame) {
    let irq = irq + INTERRUPT_INDEX_OFFSET as usize;
    let handlers_table = IRQ_HANDLERS_TABLE.lock();
    let handler = handlers_table.get(&irq);
    if let Some(handler) = handler {
        handler(irq, frame);
    } else {
        log::warn!("Unhandled irq: {}", irq);
    }
    super::apic::end_of_interrupt();
}

pub fn register_irq(irq: usize, handler: IrqHandler) {
    IRQ_HANDLERS_TABLE.lock().insert(irq, handler);
}
