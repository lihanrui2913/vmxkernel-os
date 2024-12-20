#![no_std]
#![feature(variant_count)]

use core::mem::{transmute, variant_count};

#[derive(Debug)]
#[allow(dead_code)]
pub enum SyscallIndex {
    Null,
    Print,
    Malloc,
    Exit,
    Free,
    Open,
    Close,
    Read,
    Write,
    Fsize,
    Execve,
    IsExited,
    ChangeCwd,
    GetCwd,
    FType,
    ListDir,
    DirItemNum,
    IoCtl,
    GetArgs,
    GetPid,
    LSeek,
    Kill,
    SBrk,
    Create,
    Mount,
    Mmap,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum FbDevIoctlCommand {
    GetWidth,
    GetHeight,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum KvmDevIoctlCommand {
    KvmRun,
    KvmGetRegs,
}

impl From<usize> for SyscallIndex {
    fn from(number: usize) -> Self {
        let syscall_length = variant_count::<Self>();
        if number >= syscall_length {
            panic!("Invalid syscall index: {}", number);
        }
        unsafe { transmute(number as u8) }
    }
}

impl From<usize> for FbDevIoctlCommand {
    fn from(number: usize) -> Self {
        let length = variant_count::<Self>();
        if number >= length {
            panic!("Invalid syscall index: {}", number);
        }
        unsafe { transmute(number as u8) }
    }
}

impl From<usize> for KvmDevIoctlCommand {
    fn from(number: usize) -> Self {
        let length = variant_count::<Self>();
        if number >= length {
            panic!("Invalid syscall index: {}", number);
        }
        unsafe { transmute(number as u8) }
    }
}
