#![allow(unused_doc_comments)]
//! Translated From
//! https://github.com/dotnet/runtime/blob/main/src/native/libs/System.Native/pal_io.h
//! and
//! https://github.com/dotnet/runtime/blob/main/src/native/libs/System.Native/pal_io.c

/* cSpell:disable */

use crate::pl_lib_impl::System_Console_::console_pal::util::errno_location;

macro _flags(
$t:ty:
    $($i:ident = $e:expr),+ $(,)?
) {$(
    #[allow(unused)]
    pub const $i: $t = $e;
)+}

/**
 * Constants for interpreting the flags passed to Open or ShmOpen.
 * There are several other values defined by POSIX but not implemented
 * everywhere. The set below is restricted to the current needs of
 * COREFX, which increases portability and speeds up conversion. We
 * can add more as needed.
 */
_flags! {
i32:
    // Access modes (mutually exclusive).
    PAL_O_RDONLY = 0x0000, // Open for read-only
    PAL_O_WRONLY = 0x0001, // Open for write-only
    PAL_O_RDWR = 0x0002,   // Open for read-write

    // Mask to get just the access mode. Some room is left for more.
    // POSIX also defines O_SEARCH and O_EXEC that are not available
    // everywhere.
    PAL_O_ACCESS_MODE_MASK = 0x000F,

    // Flags (combinable)
    // These numeric values are not defined by POSIX and vary across targets.
    PAL_O_CLOEXEC = 0x0010,  // Close-on-exec
    PAL_O_CREAT = 0x0020,    // Create file if it doesn't already exist
    PAL_O_EXCL = 0x0040,     // When combined with CREAT, fails if file already exists
    PAL_O_TRUNC = 0x0080,    // Truncate file to length 0 if it already exists
    PAL_O_SYNC = 0x0100,     // Block writes call will block until physically written
    PAL_O_NOFOLLOW = 0x0200, // Fails to open the target if it's a symlink, parent symlinks are allowed
}

pub fn pipe(pipe_fds: &mut [i32; 2], mut flags: i32) -> i32 {
    match flags {
        0 => (),
        PAL_O_CLOEXEC => flags = libc::O_CLOEXEC,
        _ => {
            assert!(false, "Unknown pipe flag {}", flags);
            unsafe { *errno_location() = libc::EINVAL };
            return -1;
        }
    }
    let mut result = unsafe { libc::pipe2(pipe_fds.as_mut_ptr().cast(), flags) };
    while result < 0 && unsafe { *errno_location() } == libc::EINTR {
        result = unsafe { libc::pipe2(pipe_fds.as_mut_ptr().cast(), flags) };
    }
    result
}
