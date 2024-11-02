use super::{e_raw, Sys};
use crate::{
    error::Result,
    header::{signal::sigset_t, sys_epoll::epoll_event},
    platform::{types::*, PalEpoll},
};

impl PalEpoll for Sys {
    fn epoll_create1(flags: c_int) -> Result<c_int> {
        Ok(0)
    }

    unsafe fn epoll_ctl(epfd: c_int, op: c_int, fd: c_int, event: *mut epoll_event) -> Result<()> {
        Ok(())
    }

    unsafe fn epoll_pwait(
        epfd: c_int,
        events: *mut epoll_event,
        maxevents: c_int,
        timeout: c_int,
        sigmask: *const sigset_t,
    ) -> Result<usize> {
        Ok(0)
    }
}
