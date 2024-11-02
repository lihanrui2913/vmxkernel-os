use crate::header::signal::sigval;
use core::{mem, ptr::addr_of};

use super::{
    super::{types::*, PalSignal},
    e_raw, Sys,
};
use crate::{
    error::{Errno, Result},
    header::{
        signal::{sigaction, siginfo_t, sigset_t, stack_t, NSIG, SA_RESTORER, SI_QUEUE},
        sys_time::itimerval,
        time::timespec,
    },
};

impl PalSignal for Sys {
    unsafe fn getitimer(which: c_int, out: *mut itimerval) -> Result<()> {
        unimplemented!()
    }

    fn kill(pid: pid_t, sig: c_int) -> Result<()> {
        unimplemented!()
    }
    fn sigqueue(pid: pid_t, sig: c_int, val: sigval) -> Result<()> {
        unimplemented!()
    }

    fn killpg(pgrp: pid_t, sig: c_int) -> Result<()> {
        unimplemented!()
    }

    fn raise(sig: c_int) -> Result<()> {
        unimplemented!()
    }

    unsafe fn setitimer(which: c_int, new: *const itimerval, old: *mut itimerval) -> Result<()> {
        unimplemented!()
    }

    fn sigaction(
        sig: c_int,
        act: Option<&sigaction>,
        oact: Option<&mut sigaction>,
    ) -> Result<(), Errno> {
        unimplemented!()
    }

    unsafe fn sigaltstack(ss: Option<&stack_t>, old_ss: Option<&mut stack_t>) -> Result<()> {
        unimplemented!()
    }

    fn sigpending(set: &mut sigset_t) -> Result<()> {
        unimplemented!()
    }

    fn sigprocmask(how: c_int, set: Option<&sigset_t>, oset: Option<&mut sigset_t>) -> Result<()> {
        unimplemented!()
    }

    fn sigsuspend(mask: &sigset_t) -> Errno {
        unimplemented!()
    }

    fn sigtimedwait(
        set: &sigset_t,
        sig: Option<&mut siginfo_t>,
        tp: Option<&timespec>,
    ) -> Result<()> {
        unimplemented!()
    }
}
