use super::operations::*;
use core::arch::asm;
use syscall_index::SyscallIndex;

#[allow(unused_variables)]
pub extern "C" fn syscall_matcher(
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
) -> usize {
    let syscall_number_raw: usize;
    unsafe { asm!("mov {0}, rax", out(reg) syscall_number_raw) };

    match SyscallIndex::from(syscall_number_raw) {
        SyscallIndex::Null => unimplemented!(),
        SyscallIndex::Print => print(arg1 as *const u8, arg2),
        SyscallIndex::Malloc => malloc(arg1, arg2),
        SyscallIndex::Exit => exit(),
        SyscallIndex::Free => free(arg1, arg2, arg3),
        SyscallIndex::Open => open(arg1, arg2, arg3),
        SyscallIndex::Close => close(arg1),
        SyscallIndex::Read => read(arg1, arg2, arg3),
        SyscallIndex::Write => write(arg1, arg2, arg3),
        SyscallIndex::Fsize => fsize(arg1),
        SyscallIndex::Execve => execve(arg1, arg2, arg3, arg4),
        SyscallIndex::IsExited => is_exited(arg1),
        SyscallIndex::ChangeCwd => change_cwd(arg1, arg2),
        SyscallIndex::GetCwd => get_cwd(),
        SyscallIndex::FType => ftype(arg1),
        SyscallIndex::ListDir => list_dir(arg1, arg2, arg3),
        SyscallIndex::DirItemNum => dir_item_num(arg1, arg2),
        SyscallIndex::IoCtl => ioctl(arg1, arg2, arg3),
        SyscallIndex::GetArgs => get_args(),
        SyscallIndex::GetPid => get_pid(),
        SyscallIndex::LSeek => lseek(arg1, arg2),
        SyscallIndex::Kill => kill_process(arg1),
        SyscallIndex::SBrk => sbrk(arg1),
        SyscallIndex::Create => create(arg1, arg2, arg3),
        SyscallIndex::Mount => mount(arg1, arg2, arg3, arg4),
        SyscallIndex::Mmap => mmap(arg1, arg2, arg3, arg4),
    }
}
