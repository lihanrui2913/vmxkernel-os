#![no_std]
#![no_main]

use core::panic::PanicInfo;
use kernel::alloc::string::String;
use kernel::device::hpet::HPET;
use kernel::device::keyboard::print_keypresses;
use kernel::device::rtc::RtcDateTime;
use kernel::device::terminal::terminal_manual_flush;
use kernel::fs::operation::{init_file_descriptor_manager, kernel_open};
use kernel::task::process::Process;
use kernel::task::thread::Thread;
use kernel::START_SCHEDULE;
use limine::BaseRevision;

#[used]
#[link_section = ".requests"]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[no_mangle]
extern "C" fn _start() -> ! {
    kernel::init();
    log::info!("HPET elapsed: {} ns", HPET.elapsed_ns());

    Thread::new_kernel_thread(print_keypresses);
    Thread::new_kernel_thread(terminal_manual_flush);

    let ansi_red_test_string = "\x1b[31mRed\x1b[0m";
    log::info!("ANSI red test string: {}", ansi_red_test_string);

    (40..=47).for_each(|index| kernel::print!("\x1b[{}m   \x1b[0m", index));
    kernel::println!();
    (100..=107).for_each(|index| kernel::print!("\x1b[{}m   \x1b[0m", index));
    kernel::println!();

    let current_time = RtcDateTime::new().to_datetime().unwrap();
    log::info!("Current time: {}", current_time);

    kernel::fs::init();

    let inode = kernel_open(String::from("/init.elf")).expect("Cannot open init.elf");
    let size = inode.read().size();
    let buf = kernel::alloc::vec![0u8; size].leak();
    inode.read().read_at(0, buf);
    let process = Process::new_user_process("init", buf);
    init_file_descriptor_manager(process.read().id);

    START_SCHEDULE.store(true, core::sync::atomic::Ordering::SeqCst);
    x86_64::instructions::interrupts::enable();

    loop {
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(panic_info: &PanicInfo<'_>) -> ! {
    log::error!("{}", panic_info);
    loop {
        x86_64::instructions::hlt();
    }
}
