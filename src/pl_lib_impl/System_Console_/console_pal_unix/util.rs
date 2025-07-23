/* cSpell:disable */
#![allow(unexpected_cfgs)]

use global::ThreadSafe;
use global::errors::RuntimeError;
use libc::{PTHREAD_MUTEX_INITIALIZER, pthread_mutex_t, sigaction, termios};
use std::ffi::c_void;
use std::os::fd::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::pl_lib_impl::System_Console_::console_pal::pal_io::{PAL_O_CLOEXEC, pipe};
use crate::pl_lib_impl::System_Console_::console_pal::pal_signal::{
    G_HANDLER_IS_INSTALLED, G_HAS_POSIX_SIGNAL_REGISTERATIONS, G_ORIG_SIG_HANDLER, G_PID,
    G_SIGNAL_PIPE, close_signal_handling_pipe, get_signal_max, install_signal_handler,
    install_ttou_handler_for_console, uninstall_ttou_handler_for_console,
};

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

static mut G_LOCK: pthread_mutex_t = PTHREAD_MUTEX_INITIALIZER;

static mut G_SIGNAL_FOR_BREAK: bool = true;

static mut G_INIT_TERMIOS: termios = unsafe { std::mem::zeroed() };
static mut G_HAS_CURRENT_TERMIOS: bool = false;
static mut G_CURRENT_TERMIOS: termios = unsafe { std::mem::zeroed() };

static mut G_READING: bool = false;
static mut G_CHILD_USED_TERMINAL: bool = false;
static mut G_TERMINAL_UNINITIALIZED: bool = false;
static mut G_TERMINAL_CONFIGURED: bool = false;
static mut G_HAS_TTY: bool = false;

static G_RECEIVED_SIG_TTOU: AtomicBool = AtomicBool::new(false);

/// Translated from
/// https://github.com/dotnet/runtime/blob/b306971348daefc65862fc446272a832b17aa71f/src/native/libs/System.Native/pal_console.c#L459
pub fn initialize_terminal_and_signal_handling() -> i32 {
    static mut INITIALIZED: i32 = 0;

    if unsafe { libc::pthread_mutex_lock(&mut G_LOCK) } == 0 {
        unsafe {
            if INITIALIZED == 0 {
                initialize_terminal_core();
                INITIALIZED = initialize_signal_handling_core();
            }
            libc::pthread_mutex_unlock(&mut G_LOCK);
        }
    }
    unsafe { INITIALIZED }
}

fn initialize_signal_handling_core() -> i32 {
    let signal_max = get_signal_max() as usize;
    G_ORIG_SIG_HANDLER.store(
        unsafe { libc::calloc(size_of::<sigaction>(), signal_max) }.cast(),
        Ordering::SeqCst,
    );
    G_HANDLER_IS_INSTALLED.store(
        unsafe { libc::calloc(size_of::<bool>(), signal_max) }.cast(),
        Ordering::SeqCst,
    );
    G_HAS_POSIX_SIGNAL_REGISTERATIONS.store(
        unsafe { libc::calloc(size_of::<bool>(), signal_max) }.cast(),
        Ordering::SeqCst,
    );
    if G_ORIG_SIG_HANDLER.load(Ordering::SeqCst).is_null()
        || G_HANDLER_IS_INSTALLED.load(Ordering::SeqCst).is_null()
        || G_HAS_POSIX_SIGNAL_REGISTERATIONS
            .load(Ordering::SeqCst)
            .is_null()
    {
        unsafe {
            libc::free(G_ORIG_SIG_HANDLER.load(Ordering::SeqCst).cast());
            libc::free(G_HANDLER_IS_INSTALLED.load(Ordering::SeqCst).cast());
            libc::free(
                G_HAS_POSIX_SIGNAL_REGISTERATIONS.load(Ordering::SeqCst) as usize as *mut c_void,
            );
            G_ORIG_SIG_HANDLER.store(std::ptr::null_mut(), Ordering::SeqCst);
            G_HANDLER_IS_INSTALLED.store(std::ptr::null_mut(), Ordering::SeqCst);
            G_HAS_POSIX_SIGNAL_REGISTERATIONS.store(std::ptr::null_mut(), Ordering::SeqCst);
            *errno_location() = libc::ENOMEM;
        }
        return 0;
    }
    unsafe { G_PID = libc::getpid() };

    if unsafe { pipe(&mut G_SIGNAL_PIPE, PAL_O_CLOEXEC) } != 0 {
        return 0;
    }
    unsafe {
        assert!(G_SIGNAL_PIPE[0] >= 0);
        assert!(G_SIGNAL_PIPE[1] >= 0);
    }

    let read_fd_ptr = unsafe { libc::malloc(size_of::<libc::c_int>()) }.cast::<libc::c_int>();
    if read_fd_ptr.is_null() {
        close_signal_handling_pipe();
        unsafe { *errno_location() = libc::ENOMEM };
        return 0;
    }
    unsafe { *read_fd_ptr = G_SIGNAL_PIPE[0] };
    unsafe {
        assert! {
            install_signal_handler(libc::SIGINT, libc::SA_RESTART) &&
            install_signal_handler(libc::SIGQUIT, libc::SA_RESTART) &&
            install_signal_handler(libc::SIGCONT, libc::SA_RESTART)
        }
    }
    1
}

/// Translated from
/// https://github.com/dotnet/runtime/blob/b306971348daefc65862fc446272a832b17aa71f/src/native/libs/System.Native/pal_console.c#L440
pub fn initialize_terminal_core() {
    let have_init_termios =
        unsafe { libc::tcgetattr(libc::STDIN_FILENO, &mut G_INIT_TERMIOS) } >= 0;
    if have_init_termios {
        unsafe {
            G_HAS_TTY = true;
            G_HAS_CURRENT_TERMIOS = true;
            G_CURRENT_TERMIOS = G_INIT_TERMIOS;
            G_SIGNAL_FOR_BREAK = (G_INIT_TERMIOS.c_cflag & libc::ISIG) > 0;
            libc::atexit(uninitialize_terminal);
        }
    } else {
        unsafe { G_SIGNAL_FOR_BREAK = true };
    }
}

pub fn tc_set_attr(termios: &mut termios, block_if_background: bool) -> bool {
    if unsafe { G_TERMINAL_UNINITIALIZED } {
        return true;
    }
    if !block_if_background {
        install_ttou_handler_for_console(ttou_handler);
        G_RECEIVED_SIG_TTOU.store(false, Ordering::SeqCst);
    }

    let mut rv = unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, termios) } >= 0;

    if !block_if_background {
        if (!rv)
            && (std::io::Error::last_os_error().raw_os_error().unwrap() == libc::EINTR
                && G_RECEIVED_SIG_TTOU.load(Ordering::SeqCst))
        {
            rv = true;
        }
        uninstall_ttou_handler_for_console();
    }

    unsafe {
        if rv {
            G_TERMINAL_CONFIGURED = true;
            G_HAS_CURRENT_TERMIOS = true;
            G_CURRENT_TERMIOS = *termios;
        }
    }
    rv
}

pub extern "C" fn ttou_handler() {
    G_RECEIVED_SIG_TTOU.store(true, Ordering::SeqCst);
}

pub extern "C" fn uninitialize_terminal() {
    if unsafe { libc::pthread_mutex_lock(&mut G_LOCK) } == 0 {
        if !unsafe { G_TERMINAL_UNINITIALIZED } {
            if unsafe { G_TERMINAL_CONFIGURED } {
                unsafe { tc_set_attr(&mut G_INIT_TERMIOS, false) };
            }
            unsafe { G_TERMINAL_UNINITIALIZED = true };
        }
        unsafe { libc::pthread_mutex_unlock(&mut G_LOCK) };
    }
}

pub mod sys {
    use std::os::fd::FromRawFd;
    use std::os::fd::{AsRawFd, OwnedFd};

    pub fn dup(old_fd: OwnedFd) -> OwnedFd {
        unsafe { OwnedFd::from_raw_fd(libc::dup(old_fd.as_raw_fd())) }
    }
}

cfg_if::cfg_if! {
    if #[cfg(any(target_os = "freebsd", apple_targets))] {
        pub unsafe fn errno_location() -> *mut libc::c_int {
            unsafe { libc::__error() }
        }
    } else if #[cfg(any(target_os = "android", netbsdlike, target_os = "cygwin"))] {
        pub unsafe fn errno_location() -> *mut libc::c_int {
            unsafe { libc::__errno() }
        }
    } else if #[cfg(any(target_os = "linux",
                        target_os = "redox",
                        target_os = "dragonfly",
                        target_os = "fuchsia",
                        target_os = "hurd",
                        target_os = "emscripten"))] {
        pub unsafe fn errno_location() -> *mut libc::c_int {
            unsafe { libc::__errno_location() }
        }
    } else if #[cfg(solarish)] {
        pub unsafe fn errno_location() -> *mut libc::c_int {
            unsafe { libc::___errno() }
        }
    } else if #[cfg(any(target_os = "haiku",))] {
        pub unsafe fn errno_location() -> *mut libc::c_int {
            unsafe { libc::_errnop() }
        }
    } else if #[cfg(any(target_os = "aix"))] {
        pub unsafe fn errno_location() -> *mut libc::c_int {
            unsafe { libc::_Errno() }
        }
    }
}
