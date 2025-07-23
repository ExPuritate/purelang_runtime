use std::io::{Read, Write};
use std::os::fd::{FromRawFd, RawFd};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
#[path = "./console_pal_unix/pal_io.rs"]
mod pal_io;
#[path = "./console_pal_unix/pal_signal.rs"]
mod pal_signal;
#[path = "./console_pal_unix/util.rs"]
mod util;

use crate::pl_lib_impl::System_Console_::ConsoleColor;
use crate::pl_lib_impl::System_Console_::console_pal::util::{
    Handle, check_io, initialize_terminal_and_signal_handling, sys, uninitialize_terminal,
};
use encoding_rs::Encoding;
use global::errors::RuntimeError;
use std::ffi::c_void;
use std::os::fd::{AsRawFd, OwnedFd};

static mut TERMINAL_HANDLE: Handle = Handle(std::ptr::null_mut());

pub struct ConsoleFile {
    handle: Handle,
    use_read_line: bool,
}

impl ConsoleFile {
    pub fn new(handle: Handle, use_read_line: bool) -> Self {
        Self {
            handle,
            use_read_line,
        }
    }
}

impl Read for ConsoleFile {
    #[allow(unused)]
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.use_read_line {}
        todo!()
    }
}
#[allow(unused)]
impl Write for ConsoleFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        todo!()
    }

    fn flush(&mut self) -> std::io::Result<()> {
        todo!()
    }
}

impl Drop for ConsoleFile {
    fn drop(&mut self) {
        unsafe {
            libc::free(self.handle.0);
        }
    }
}

static INITIALIZED: AtomicBool = AtomicBool::new(false);

pub(crate) fn ensure_console_initialized() -> global::Result<()> {
    if !INITIALIZED.load(Ordering::SeqCst) {
        unsafe { ensure_initialized_core() }
    } else {
        Ok(())
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn ensure_initialized_core() -> global::Result<()> {
    let out = super::get_out().lock().unwrap();

    if !INITIALIZED.load(Ordering::SeqCst) {
        #[allow(irrefutable_let_patterns)]
        if let x = initialize_terminal_and_signal_handling()
            && x == 0
        {
            return Err(RuntimeError::LibcError(x).into());
        }
        libc::atexit(uninitialize_terminal);
        TERMINAL_HANDLE = if !super::is_output_redirected() {
            open_console_file_handle(libc::STDOUT_FILENO)?
        } else {
            if !super::is_input_redirected() {
                open_console_file_handle(libc::STDIN_FILENO)?
            } else {
                Handle(std::ptr::null_mut())
            }
        };

        if !TERMINAL_HANDLE.0.is_null() && todo!() {
            todo!()
        }
    }
    todo!()
}

fn open_console_file_handle(fd: RawFd) -> global::Result<Handle> {
    unsafe {
        Ok(Handle(
            check_io(sys::dup(OwnedFd::from_raw_fd(fd)).as_raw_fd())?.as_raw_fd() as *mut c_void,
        ))
    }
}

fn open_console_file(fd: RawFd, use_read_line: bool) -> global::Result<ConsoleFile> {
    Ok(ConsoleFile::new(
        open_console_file_handle(fd)?,
        use_read_line,
    ))
}

pub fn open_standard_input() -> global::Result<ConsoleFile> {
    open_console_file(libc::STDIN_FILENO, !super::is_input_redirected())
}

pub fn open_standard_output() -> global::Result<ConsoleFile> {
    open_console_file(libc::STDOUT_FILENO, false)
}

pub fn open_standard_error() -> global::Result<ConsoleFile> {
    open_console_file(libc::STDERR_FILENO, false)
}

const LOCALE_ENV_VARS: [&str; 3] = ["LC_ALL", "LC_MESSAGES", "LANG"];

fn get_encoding_from_charset() -> Option<&'static Encoding> {
    get_charset().and_then(|x| Encoding::for_label(x.as_bytes()))
}

pub fn input_encoding() -> &'static Encoding {
    get_console_encoding()
}
pub fn output_encoding() -> &'static Encoding {
    get_console_encoding()
}

pub fn set_console_input_encoding(encoding: &'static Encoding) -> global::Result<()> {
    // No-op.
    // There is no good way to set the terminal console encoding.
    // From Microsoft
    // https://github.com/dotnet/runtime/blob/main/src/libraries/System.Console/src/System/ConsolePal.Unix.cs#L724
    Ok(())
}

pub fn set_console_output_encoding(_e: &'static Encoding) -> global::Result<()> {
    // No-op.
    // There is no good way to set the terminal console encoding.
    // From Microsoft
    // https://github.com/dotnet/runtime/blob/main/src/libraries/System.Console/src/System/ConsolePal.Unix.cs#L730
    Ok(())
}

fn get_console_encoding() -> &'static Encoding {
    get_encoding_from_charset().unwrap_or(encoding_rs::UTF_8)
}

fn get_charset() -> Option<String> {
    let mut locale = None;
    for v in LOCALE_ENV_VARS {
        locale = std::env::var(v).ok();
        match &locale {
            Some(x) if !x.is_empty() => break,
            _ => (),
        }
    }

    if let Some(locale) = locale {
        let Some(mut dot_pos) = locale.bytes().position(|x| x == b'.') else {
            return None;
        };

        dot_pos += 1;
        let at_pos = locale[(dot_pos + 1)..]
            .bytes()
            .position(|x| x == b'@')
            .map(|x| x as isize)
            .unwrap_or(-1);
        let charset = if at_pos < dot_pos as isize {
            &locale[dot_pos..]
        } else {
            &locale[dot_pos..(at_pos as usize)]
        };
        Some(charset.to_lowercase())
    } else {
        None
    }
}

pub fn is_input_redirected_core() -> bool {
    todo!()
}

pub fn is_output_redirected_core() -> bool {
    todo!()
}

pub fn is_error_redirected_core() -> bool {
    todo!()
}

pub fn key_available() -> global::Result<bool> {
    todo!()
}

pub fn background_color() -> global::Result<ConsoleColor> {
    todo!()
}

pub fn set_background_color(value: ConsoleColor) -> global::Result<()> {
    todo!()
}

pub fn foreground_color() -> global::Result<ConsoleColor> {
    todo!()
}

pub fn set_foreground_color(value: ConsoleColor) -> global::Result<()> {
    todo!()
}

pub fn reset_color() -> global::Result<()> {
    todo!()
}

pub fn buffer_width() -> global::Result<isize> {
    todo!()
}

pub fn buffer_height() -> global::Result<isize> {
    todo!()
}
