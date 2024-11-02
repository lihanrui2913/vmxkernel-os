use crate::SyscallIndex;

pub fn sbrk(size: usize) -> usize {
    crate::syscall(SyscallIndex::SBrk as u64, size, 0, 0, 0, 0)
}

pub fn mmap(addr: usize, len: usize, prot: usize, flags: usize) -> usize {
    crate::syscall(SyscallIndex::Mmap as u64, addr, len, prot, flags, 0)
}
