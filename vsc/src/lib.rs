#![no_std]
#![feature(c_size_t)]

mod c_str;

extern crate alloc;

use alloc::string::String;
use core::ffi::{c_char, c_int, c_void};
use vstd::fs::OpenMode;

use c_str::CStr;

#[no_mangle]
pub extern "C" fn open(path: *const c_char, mode: c_int) -> c_int {
    vstd::fs::open(
        String::from(unsafe { CStr::from_ptr(path).to_str().unwrap() }),
        OpenMode::from(mode as usize),
    ) as c_int
}

#[no_mangle]
pub extern "C" fn close(fd: c_int) -> c_int {
    vstd::fs::close(fd as usize) as c_int
}

#[no_mangle]
pub extern "C" fn fstat(_fd: c_int, _st: *mut c_void) -> c_int {
    0
}

#[no_mangle]
pub extern "C" fn getpid() -> c_int {
    vstd::task::getpid() as c_int
}

#[no_mangle]
pub extern "C" fn _exit(code: c_int) -> ! {
    vstd::task::exit(code as usize)
}

#[no_mangle]
pub extern "C" fn isatty() -> c_int {
    0
}

#[no_mangle]
pub extern "C" fn kill(pid: c_int) -> c_int {
    vstd::task::kill(pid as usize) as c_int
}

#[no_mangle]
pub extern "C" fn lseek(fd: c_int, ptr: c_int, _dir: c_int) -> c_int {
    vstd::fs::lseek(fd as usize, ptr as usize) as c_int
}

#[no_mangle]
pub extern "C" fn read(fd: c_int, ptr: *mut c_void, len: c_int) -> c_int {
    let buf = unsafe { core::slice::from_raw_parts_mut(ptr as *mut u8, len as usize) };
    vstd::fs::read(fd as usize, buf) as c_int
}

#[no_mangle]
pub extern "C" fn write(fd: c_int, ptr: *const c_void, len: c_int) -> c_int {
    let buf = unsafe { core::slice::from_raw_parts(ptr as *const u8, len as usize) };
    vstd::fs::write(fd as usize, buf) as c_int
}

#[no_mangle]
pub extern "C" fn sbrk(size: c_int) -> c_int {
    vstd::mm::sbrk(size as usize) as c_int
}
