use std::os::fd::{FromRawFd, RawFd};
#[path = "./console_pal_unix/util.rs"]
mod util;

use crate::pl_lib_impl::System_Console_::console_pal::util::{check_io, sys, Handle};
use encoding_rs::Encoding;
use std::ffi::c_void;
use std::os::fd::{AsRawFd, OwnedFd};

static TERMINAL_HANDLE: Handle = Handle(std::ptr::null_mut());

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

impl Drop for ConsoleFile {
    fn drop(&mut self) {
        unsafe {
            libc::free(self.handle.0);
        }
    }
}

fn open_console_file(fd: RawFd, use_read_line: bool) -> global::Result<ConsoleFile> {
    unsafe {
        Ok(ConsoleFile::new(
            Handle(
                check_io(sys::dup(OwnedFd::from_raw_fd(fd)).as_raw_fd())?.as_raw_fd()
                    as *mut c_void,
            ),
            use_read_line,
        ))
    }
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

const LOCALE_ENV_VARS: [&'static str; 3] = ["LC_ALL", "LC_MESSAGES", "LANG"];

fn get_encoding_from_charset() -> Option<&'static Encoding> {
    todo!()
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
