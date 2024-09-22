use core::{
    ffi::{c_char, c_size_t},
    marker::PhantomData,
    ptr::NonNull,
    str::Utf8Error,
    usize,
};

unsafe fn strlen(s: *const c_char) -> c_size_t {
    strnlen(s, usize::MAX)
}

unsafe fn strnlen(s: *const c_char, size: c_size_t) -> c_size_t {
    let mut i = 0;
    while i < size {
        if *s.add(i) == 0 {
            break;
        }
        i += 1;
    }
    i as c_size_t
}

/// C string wrapper, guaranteed to be
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct CStr<'a> {
    ptr: NonNull<c_char>,
    _marker: PhantomData<&'a [u8]>,
}

impl<'a> CStr<'a> {
    /// Safety
    ///
    /// The ptr must be valid up to and including the first NUL byte from the base ptr.
    pub const unsafe fn from_ptr(ptr: *const c_char) -> Self {
        Self {
            ptr: NonNull::new_unchecked(ptr as *mut c_char),
            _marker: PhantomData,
        }
    }
    pub fn to_bytes_with_nul(self) -> &'a [u8] {
        unsafe {
            // SAFETY: The string must be valid at least until (and including) the NUL byte.
            let len = strlen(self.ptr.as_ptr());
            core::slice::from_raw_parts(self.ptr.as_ptr().cast(), len + 1)
        }
    }
    pub fn to_bytes(self) -> &'a [u8] {
        let s = self.to_bytes_with_nul();
        &s[..s.len() - 1]
    }
    pub fn to_str(self) -> Result<&'a str, Utf8Error> {
        core::str::from_utf8(self.to_bytes())
    }
}

unsafe impl Send for CStr<'_> {}
unsafe impl Sync for CStr<'_> {}
