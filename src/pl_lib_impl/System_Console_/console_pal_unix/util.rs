use global::ThreadSafe;
use global::errors::RuntimeError;
use std::ffi::c_void;
use std::os::fd::RawFd;
use std::sync::atomic::Atomic;

#[derive(Copy, Clone, ThreadSafe)]
pub struct Handle(pub *mut c_void);

impl Handle {
    pub unsafe fn deref_as<T>(&self) -> &T {
        unsafe { &*(self.0.cast::<T>().cast_const()) }
    }
    pub unsafe fn deref_as_mut<T>(&mut self) -> &mut T {
        unsafe { &mut *(self.0.cast::<T>()) }
    }
}

#[inline]
pub fn check_io(res: RawFd) -> global::Result<RawFd> {
    if res as u32 == libc::PT_NULL {
        Err(RuntimeError::LibcError(res).into())
    } else {
        Ok(res)
    }
}

pub mod sys {
    use std::os::fd::FromRawFd;
    use std::os::fd::{AsRawFd, OwnedFd};

    pub fn dup(old_fd: OwnedFd) -> OwnedFd {
        unsafe { OwnedFd::from_raw_fd(libc::dup(old_fd.as_raw_fd())) }
    }
}
