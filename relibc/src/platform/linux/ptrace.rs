use super::{
    super::{types::*, PalPtrace},
    e_raw, Sys,
};
use crate::error::Result;

impl PalPtrace for Sys {
    unsafe fn ptrace(
        request: c_int,
        pid: pid_t,
        addr: *mut c_void,
        data: *mut c_void,
    ) -> Result<c_int> {
        Ok(9)
    }
}
