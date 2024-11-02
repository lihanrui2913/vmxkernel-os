use alloc::{ffi::CString, string::String, vec::Vec};

use crate::{header::errno::EOPNOTSUPP, io::Write};
use core::{arch::asm, num::NonZero, ptr};

use super::{types::*, Pal, ERRNO};
use crate::{
    c_str::CStr,
    header::{
        dirent::dirent,
        errno::EINVAL,
        signal::SIGCHLD,
        sys_resource::{rlimit, rusage},
        sys_stat::{stat, S_IFIFO},
        sys_statvfs::statvfs,
        sys_time::{timeval, timezone},
        unistd::SEEK_SET,
    },
};
// use header::sys_times::tms;
use crate::{
    error::{Errno, Result},
    header::{errno::ENOMEM, sys_utsname::utsname, time::timespec},
};

mod epoll;
mod ptrace;
mod signal;
mod socket;

const AT_FDCWD: c_int = -100;
const AT_EMPTY_PATH: c_int = 0x1000;
const AT_REMOVEDIR: c_int = 0x200;

const SYS_CLONE: usize = 56;
const CLONE_VM: usize = 0x0100;
const CLONE_FS: usize = 0x0200;
const CLONE_FILES: usize = 0x0400;
const CLONE_SIGHAND: usize = 0x0800;
const CLONE_THREAD: usize = 0x00010000;

#[repr(C)]
#[derive(Default)]
struct linux_statfs {
    f_type: c_long,       /* type of file system (see below) */
    f_bsize: c_long,      /* optimal transfer block size */
    f_blocks: fsblkcnt_t, /* total data blocks in file system */
    f_bfree: fsblkcnt_t,  /* free blocks in fs */
    f_bavail: fsblkcnt_t, /* free blocks available to unprivileged user */
    f_files: fsfilcnt_t,  /* total file nodes in file system */
    f_ffree: fsfilcnt_t,  /* free file nodes in fs */
    f_fsid: c_long,       /* file system id */
    f_namelen: c_long,    /* maximum length of filenames */
    f_frsize: c_long,     /* fragment size (since Linux 2.6) */
    f_flags: c_long,
    f_spare: [c_long; 4],
}

// TODO
const ERRNO_MAX: usize = 4095;

pub fn e_raw(sys: usize) -> Result<usize> {
    if sys > ERRNO_MAX.wrapping_neg() {
        Err(Errno(sys.wrapping_neg() as _))
    } else {
        Ok(sys)
    }
}

pub struct Sys;

impl Sys {
    pub unsafe fn ioctl(fd: c_int, request: c_ulong, out: *mut c_void) -> Result<c_int> {
        // TODO: Somehow support varargs to syscall??
        unimplemented!()
    }

    // fn times(out: *mut tms) -> clock_t {
    //     unsafe { syscall!(TIMES, out) as clock_t }
    // }
}

pub unsafe fn copy_to_user(dest: usize, src: &[u8]) -> Result<usize, ()> {
    let p = dest as *mut u8;
    // 拷贝数据
    p.copy_from_nonoverlapping(src.as_ptr(), src.len());
    return Ok(src.len());
}

/// 从用户空间拷贝数据到内核空间
pub unsafe fn copy_from_user(dst: &mut [u8], src: usize) -> Result<usize, ()> {
    let src: &[u8] = core::slice::from_raw_parts(src as *const u8, dst.len());
    // 拷贝数据
    dst.copy_from_slice(src);

    return Ok(dst.len());
}

pub fn check_and_clone_cstr(user: *const u8, max_length: Option<usize>) -> Result<CString, ()> {
    if user.is_null() {
        return Err(());
    }

    // 从用户态读取，直到遇到空字符 '\0' 或者达到最大长度
    let mut buffer = Vec::new();
    for i in 0.. {
        if max_length.is_some() && max_length.as_ref().unwrap() <= &i {
            break;
        }

        let addr = unsafe { user.add(i) };
        let mut c = [0u8; 1];
        unsafe {
            copy_from_user(&mut c, addr as usize)?;
        }
        if c[0] == 0 {
            break;
        }
        buffer.push(NonZero::new(c[0]).ok_or(())?);
    }

    let cstr = CString::from(buffer);

    return Ok(cstr);
}

pub fn check_and_clone_cstr_array(user: *const *const u8) -> Result<Vec<CString>, ()> {
    if user.is_null() {
        Ok(Vec::new())
    } else {
        // debug!("check_and_clone_cstr_array: {:p}\n", user);
        let mut buffer = Vec::new();
        for i in 0.. {
            let addr = unsafe { user.add(i) };
            let str_ptr: *const u8;
            // 读取这个地址的值（这个值也是一个指针）
            unsafe {
                let dst = [0usize; 1];
                let mut dst =
                    core::mem::transmute::<[usize; 1], [u8; core::mem::size_of::<usize>()]>(dst);
                copy_from_user(&mut dst, addr as usize)?;
                let dst =
                    core::mem::transmute::<[u8; core::mem::size_of::<usize>()], [usize; 1]>(dst);
                str_ptr = dst[0] as *const u8;
            }

            if str_ptr.is_null() {
                break;
            }
            let string = check_and_clone_cstr(str_ptr, None)?;
            buffer.push(string);
        }
        return Ok(buffer);
    }
}

static mut BRK_CUR: *mut c_void = ptr::null_mut();
static mut BRK_END: *mut c_void = ptr::null_mut();

impl Pal for Sys {
    fn access(path: CStr, mode: c_int) -> Result<()> {
        unimplemented!()
    }

    unsafe fn brk(addr: *mut c_void) -> Result<*mut c_void> {
        // On first invocation, allocate a buffer for brk
        if BRK_CUR.is_null() {
            // 4 megabytes of RAM ought to be enough for anybody
            const BRK_MAX_SIZE: usize = 4 * 1024 * 1024;

            let allocated = Self::mmap(ptr::null_mut(), 0, 0, 0, 0, 0)?;

            BRK_CUR = allocated;
            BRK_END = (allocated as *mut u8).add(BRK_MAX_SIZE) as *mut c_void;
        }

        if addr.is_null() {
            // Lookup what previous brk() invocations have set the address to
            Ok(BRK_CUR)
        } else if BRK_CUR <= addr && addr < BRK_END {
            // It's inside buffer, return
            BRK_CUR = addr;
            Ok(addr)
        } else {
            // It was outside of valid range
            Err(Errno(ENOMEM))
        }
    }

    fn chdir(path: CStr) -> Result<()> {
        vsc::fs::change_cwd(String::from(unsafe { path.to_str() }.unwrap()));
        Ok(())
    }
    fn set_default_scheme(scheme: CStr) -> Result<()> {
        Err(Errno(EOPNOTSUPP))
    }

    fn chmod(path: CStr, mode: mode_t) -> Result<()> {
        unimplemented!()
    }

    fn chown(path: CStr, owner: uid_t, group: gid_t) -> Result<()> {
        unimplemented!()
    }

    unsafe fn clock_getres(clk_id: clockid_t, tp: *mut timespec) -> Result<()> {
        unimplemented!()
    }

    unsafe fn clock_gettime(clk_id: clockid_t, tp: *mut timespec) -> Result<()> {
        unimplemented!()
    }

    unsafe fn clock_settime(clk_id: clockid_t, tp: *const timespec) -> Result<()> {
        unimplemented!()
    }

    fn close(fildes: c_int) -> Result<()> {
        vsc::fs::close(fildes as usize);
        Ok(())
    }

    fn dup(fildes: c_int) -> Result<c_int> {
        unimplemented!()
    }

    fn dup2(fildes: c_int, fildes2: c_int) -> Result<c_int> {
        unimplemented!()
    }

    unsafe fn execve(path: CStr, argv: *const *mut c_char, envp: *const *mut c_char) -> Result<()> {
        let fd = vsc::fs::open(
            String::from(unsafe { path.to_str() }.unwrap()),
            vsc::fs::OpenMode::Read,
        );

        let buf = alloc::vec![0u8; vsc::fs::fsize(fd)].leak();
        vsc::fs::read(fd, buf);
        let argc = check_and_clone_cstr_array(argv as *const *const u8)
            .expect("Cannot get argc")
            .len();
        vsc::task::execve(buf, argv as usize, argc);
        Ok(())
    }
    unsafe fn fexecve(
        fildes: c_int,
        argv: *const *mut c_char,
        envp: *const *mut c_char,
    ) -> Result<()> {
        let fd = fildes as usize;

        let buf = alloc::vec![0u8; vsc::fs::fsize(fd)].leak();
        vsc::fs::read(fd, buf);
        let argc = check_and_clone_cstr_array(argv as *const *const u8)
            .expect("Cannot get argc")
            .len();
        vsc::task::execve(buf, argv as usize, argc);
        Ok(())
    }

    fn exit(status: c_int) -> ! {
        vsc::task::exit(status as usize)
    }
    unsafe fn exit_thread(_stack_base: *mut (), _stack_size: usize) -> ! {
        Self::exit(0)
    }

    fn fchdir(fildes: c_int) -> Result<()> {
        unimplemented!()
    }

    fn fchmod(fildes: c_int, mode: mode_t) -> Result<()> {
        unimplemented!()
    }

    fn fchown(fildes: c_int, owner: uid_t, group: gid_t) -> Result<()> {
        unimplemented!()
    }

    fn fdatasync(fildes: c_int) -> Result<()> {
        unimplemented!()
    }

    fn flock(fd: c_int, operation: c_int) -> Result<()> {
        unimplemented!()
    }

    unsafe fn fstat(fildes: c_int, buf: *mut stat) -> Result<()> {
        unimplemented!()
    }

    unsafe fn fstatvfs(fildes: c_int, buf: *mut statvfs) -> Result<()> {
        unimplemented!()
    }

    fn fcntl(fildes: c_int, cmd: c_int, arg: c_ulonglong) -> Result<c_int> {
        unimplemented!()
    }

    unsafe fn fork() -> Result<pid_t> {
        unimplemented!()
    }

    fn fpath(fildes: c_int, out: &mut [u8]) -> Result<usize> {
        unimplemented!()
    }

    fn fsync(fildes: c_int) -> Result<()> {
        unimplemented!()
    }

    fn ftruncate(fildes: c_int, length: off_t) -> Result<()> {
        unimplemented!()
    }

    #[inline]
    unsafe fn futex_wait(addr: *mut u32, val: u32, deadline: Option<&timespec>) -> Result<()> {
        unimplemented!()
    }
    #[inline]
    unsafe fn futex_wake(addr: *mut u32, num: u32) -> Result<u32> {
        unimplemented!()
    }

    unsafe fn futimens(fd: c_int, times: *const timespec) -> Result<()> {
        unimplemented!()
    }

    unsafe fn utimens(path: CStr, times: *const timespec) -> Result<()> {
        unimplemented!()
    }

    unsafe fn getcwd(buf: *mut c_char, size: size_t) -> Result<()> {
        let cwd = vsc::fs::get_cwd();

        buf.copy_from(cwd.as_ptr() as *const c_char, cwd.len());

        Ok(())
    }

    fn getdents(fd: c_int, buf: &mut [u8], _off: u64) -> Result<usize> {
        unimplemented!()
    }
    fn dir_seek(fd: c_int, off: u64) -> Result<()> {
        unimplemented!()
    }
    unsafe fn dent_reclen_offset(this_dent: &[u8], offset: usize) -> Option<(u16, u64)> {
        unimplemented!()
    }

    fn getegid() -> gid_t {
        unimplemented!()
    }

    fn geteuid() -> uid_t {
        unimplemented!()
    }

    fn getgid() -> gid_t {
        unimplemented!()
    }

    unsafe fn getgroups(size: c_int, list: *mut gid_t) -> Result<c_int> {
        unimplemented!()
    }

    fn getpagesize() -> usize {
        4096
    }

    fn getpgid(pid: pid_t) -> Result<pid_t> {
        unimplemented!()
    }

    fn getpid() -> pid_t {
        vsc::task::getpid() as pid_t
    }

    fn getppid() -> pid_t {
        unimplemented!()
    }

    fn getpriority(which: c_int, who: id_t) -> Result<c_int> {
        unimplemented!()
    }

    fn getrandom(buf: &mut [u8], flags: c_uint) -> Result<usize> {
        unimplemented!()
    }

    unsafe fn getrlimit(resource: c_int, rlim: *mut rlimit) -> Result<()> {
        unimplemented!()
    }

    unsafe fn setrlimit(resource: c_int, rlimit: *const rlimit) -> Result<()> {
        unimplemented!()
    }

    fn getrusage(who: c_int, r_usage: &mut rusage) -> Result<()> {
        unimplemented!()
    }

    fn getsid(pid: pid_t) -> Result<pid_t> {
        unimplemented!()
    }

    fn gettid() -> pid_t {
        unimplemented!()
    }

    unsafe fn gettimeofday(tp: *mut timeval, tzp: *mut timezone) -> Result<()> {
        unimplemented!()
    }

    fn getuid() -> uid_t {
        unimplemented!()
    }

    fn lchown(path: CStr, owner: uid_t, group: gid_t) -> Result<()> {
        unimplemented!()
    }

    fn link(path1: CStr, path2: CStr) -> Result<()> {
        unimplemented!()
    }

    fn lseek(fildes: c_int, offset: off_t, whence: c_int) -> Result<off_t> {
        let off = vsc::fs::lseek(fildes as usize, offset as usize) as off_t;
        Ok(off)
    }

    fn mkdir(path: CStr, mode: mode_t) -> Result<()> {
        unimplemented!()
    }

    fn mknodat(dir_fildes: c_int, path: CStr, mode: mode_t, dev: dev_t) -> Result<()> {
        unimplemented!()
    }

    fn mknod(path: CStr, mode: mode_t, dev: dev_t) -> Result<()> {
        unimplemented!()
    }

    fn mkfifo(path: CStr, mode: mode_t) -> Result<()> {
        unimplemented!()
    }

    unsafe fn mlock(addr: *const c_void, len: usize) -> Result<()> {
        unimplemented!()
    }

    unsafe fn mlockall(flags: c_int) -> Result<()> {
        unimplemented!()
    }

    unsafe fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: c_int,
        flags: c_int,
        fildes: c_int,
        off: off_t,
    ) -> Result<*mut c_void> {
        Ok(vsc::mm::mmap(addr as usize, len, prot as usize, flags as usize) as *mut c_void)
    }

    unsafe fn mremap(
        addr: *mut c_void,
        len: usize,
        new_len: usize,
        flags: c_int,
        args: *mut c_void,
    ) -> Result<*mut c_void> {
        unimplemented!()
    }

    unsafe fn mprotect(addr: *mut c_void, len: usize, prot: c_int) -> Result<()> {
        unimplemented!()
    }

    unsafe fn msync(addr: *mut c_void, len: usize, flags: c_int) -> Result<()> {
        unimplemented!()
    }

    unsafe fn munlock(addr: *const c_void, len: usize) -> Result<()> {
        unimplemented!()
    }

    unsafe fn munlockall() -> Result<()> {
        unimplemented!()
    }

    unsafe fn munmap(addr: *mut c_void, len: usize) -> Result<()> {
        unimplemented!()
    }

    unsafe fn madvise(addr: *mut c_void, len: usize, flags: c_int) -> Result<()> {
        unimplemented!()
    }

    unsafe fn nanosleep(rqtp: *const timespec, rmtp: *mut timespec) -> Result<()> {
        unimplemented!()
    }

    fn open(path: CStr, oflag: c_int, mode: mode_t) -> Result<c_int> {
        Ok(vsc::fs::open(
            String::from(unsafe { path.to_str() }.unwrap()),
            // default
            vsc::fs::OpenMode::ReadWrite,
        ) as c_int)
    }

    fn pipe2(fildes: &mut [c_int], flags: c_int) -> Result<()> {
        unimplemented!()
    }

    #[cfg(target_arch = "x86_64")]
    unsafe fn rlct_clone(stack: *mut usize) -> Result<crate::pthread::OsTid> {
        unimplemented!()
    }
    unsafe fn rlct_kill(os_tid: crate::pthread::OsTid, signal: usize) -> Result<()> {
        unimplemented!()
    }
    fn current_os_tid() -> crate::pthread::OsTid {
        unimplemented!()
    }

    fn read(fildes: c_int, buf: &mut [u8]) -> Result<usize> {
        Ok(vsc::fs::read(fildes as usize, buf))
    }
    fn pread(fildes: c_int, buf: &mut [u8], off: off_t) -> Result<usize> {
        unimplemented!()
    }

    fn readlink(pathname: CStr, out: &mut [u8]) -> Result<usize> {
        unimplemented!()
    }

    fn rename(old: CStr, new: CStr) -> Result<()> {
        unimplemented!()
    }

    fn rmdir(path: CStr) -> Result<()> {
        unimplemented!()
    }

    fn sched_yield() -> Result<()> {
        unimplemented!()
    }

    unsafe fn setgroups(size: size_t, list: *const gid_t) -> Result<()> {
        unimplemented!()
    }

    fn setpgid(pid: pid_t, pgid: pid_t) -> Result<()> {
        unimplemented!()
    }

    fn setpriority(which: c_int, who: id_t, prio: c_int) -> Result<()> {
        unimplemented!()
    }

    fn setresgid(rgid: gid_t, egid: gid_t, sgid: gid_t) -> Result<()> {
        unimplemented!()
    }

    fn setresuid(ruid: uid_t, euid: uid_t, suid: uid_t) -> Result<()> {
        unimplemented!()
    }

    fn setsid() -> Result<()> {
        unimplemented!()
    }

    fn symlink(path1: CStr, path2: CStr) -> Result<()> {
        unimplemented!()
    }

    fn sync() -> Result<()> {
        unimplemented!()
    }

    fn umask(mask: mode_t) -> mode_t {
        unimplemented!()
    }

    unsafe fn uname(utsname: *mut utsname) -> Result<()> {
        unimplemented!()
    }

    fn unlink(path: CStr) -> Result<()> {
        unimplemented!()
    }

    unsafe fn waitpid(pid: pid_t, stat_loc: *mut c_int, options: c_int) -> Result<pid_t> {
        vsc::task::wait(pid as usize);
        Ok(pid)
    }

    fn write(fildes: c_int, buf: &[u8]) -> Result<usize> {
        Ok(vsc::fs::write(fildes as usize, buf))
    }
    fn pwrite(fildes: c_int, buf: &[u8], off: off_t) -> Result<usize> {
        unimplemented!()
    }

    fn verify() -> bool {
        true
    }
}
