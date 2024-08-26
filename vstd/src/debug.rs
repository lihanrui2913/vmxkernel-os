use core::fmt::{self, Write};

use spin::Mutex;

use crate::SyscallIndex;

pub fn print(str: &str) -> usize {
    crate::syscall(SyscallIndex::Print as u64, str.as_ptr() as usize, str.len(), 0, 0, 0)
}

struct AppOutputStream;

impl Write for AppOutputStream {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        print(s);
        Ok(())
    }
}

static OOS: Mutex<AppOutputStream> = Mutex::new(AppOutputStream);

#[inline]
pub fn _print(args: fmt::Arguments) {
    OOS.lock().write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => (
        $crate::debug::_print(
            format_args!($($arg)*)
        )
    )
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)))
}
