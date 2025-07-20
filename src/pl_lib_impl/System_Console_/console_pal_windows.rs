use crate::pl_lib_impl::System_Console_::{ConsoleColor, ConsoleKey, ConsoleKeyInfo};
use derive_more::Display;
use encoding_rs::Encoding;
use enumflags2::BitFlag;
use global::ThreadSafe;
use global::errors::{EncodingError, RuntimeError, RuntimeMayBeInvalidOperation};
use std::alloc::Layout;
use std::io::{Read, Write};
use std::mem::MaybeUninit;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};
use std::sync::{LazyLock, Mutex, RwLock};
use windows::Win32::Foundation::{
    ERROR_BROKEN_PIPE, ERROR_INVALID_ACCESS, ERROR_INVALID_HANDLE, ERROR_NO_DATA, ERROR_SUCCESS,
    HANDLE,
};
use windows::Win32::Storage::FileSystem;
use windows::Win32::Storage::FileSystem::FILE_TYPE_CHAR;
use windows::Win32::System::Console;
use windows::Win32::System::Console::{
    CONSOLE_CHARACTER_ATTRIBUTES, CONSOLE_MODE, COORD, INPUT_RECORD, STD_ERROR_HANDLE, STD_HANDLE,
    STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
};
use windows::core::Free;

#[derive(ThreadSafe)]
pub struct ConsoleFile {
    handle: HANDLE,
    use_file_apis: bool,
}

impl ConsoleFile {
    pub fn new(handle: HANDLE, use_file_apis: bool) -> Self {
        Self {
            handle,
            use_file_apis,
        }
    }
}

impl ConsoleFile {
    const BYTES_PER_WCHAR: u32 = 2;
    fn read_file_native(
        h_file: HANDLE,
        buffer: &mut [u8],
        bytes_read: &mut u32,
        use_file_apis: bool,
    ) -> windows::core::Result<()> {
        if buffer.is_empty() {
            *bytes_read = 0;
            return Err(ERROR_SUCCESS.into());
        }
        let res = unsafe {
            if use_file_apis {
                FileSystem::ReadFile(h_file, Some(buffer), Some(bytes_read), None)
            } else {
                let mut chars_read = 0;
                Console::ReadConsoleW(
                    h_file,
                    buffer.as_mut_ptr().cast(),
                    buffer.len() as u32 / Self::BYTES_PER_WCHAR,
                    &mut chars_read,
                    None,
                )
            }
        };
        match res {
            Ok(_) => Ok(()),
            Err(e)
                if e.code() == ERROR_NO_DATA.to_hresult()
                    || e.code() == ERROR_BROKEN_PIPE.to_hresult() =>
            {
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
    fn write_file_native(
        h_file: HANDLE,
        bytes: &[u8],
        use_file_apis: bool,
    ) -> windows::core::Result<()> {
        if bytes.is_empty() {
            return Err(ERROR_SUCCESS.into());
        }
        let res = unsafe {
            if use_file_apis {
                let mut num_bytes_written = 0;
                FileSystem::WriteFile(h_file, Some(bytes), Some(&mut num_bytes_written), None)
            } else {
                let mut chars_written = 0;
                let res = Console::WriteConsoleW(
                    h_file,
                    std::slice::from_raw_parts(
                        bytes.as_ptr().cast(),
                        bytes.len() / Self::BYTES_PER_WCHAR as usize,
                    ),
                    Some(&mut chars_written),
                    None,
                );
                debug_assert!(
                    res.is_err()
                        || bytes.len() / Self::BYTES_PER_WCHAR as usize == chars_written as usize
                );
                res
            }
        };
        match res {
            Ok(_) => Ok(()),
            Err(e)
                if e.code() == ERROR_NO_DATA.to_hresult()
                    || e.code() == ERROR_BROKEN_PIPE.to_hresult() =>
            {
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

impl Read for ConsoleFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut bytes_read = 0;
        Self::read_file_native(self.handle, buf, &mut bytes_read, self.use_file_apis)?;
        Ok(bytes_read as _)
    }
}

impl Write for ConsoleFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Self::write_file_native(self.handle, buf, self.use_file_apis)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if self.handle.is_invalid() {
            Err(std::io::ErrorKind::UnexpectedEof.into())
        } else {
            Ok(())
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        Self::write_file_native(self.handle, buf, self.use_file_apis).map_err(Into::into)
    }
}

impl Drop for ConsoleFile {
    fn drop(&mut self) {
        unsafe {
            self.handle.free();
        }
        self.handle = HANDLE::default();
    }
}

fn get_standard_file(handle_type: STD_HANDLE, use_file_apis: bool) -> global::Result<ConsoleFile> {
    unsafe {
        Ok(ConsoleFile::new(
            Console::GetStdHandle(handle_type)?,
            use_file_apis,
        ))
    }
}

pub fn open_standard_input() -> global::Result<ConsoleFile> {
    get_standard_file(
        STD_INPUT_HANDLE,
        codepage::from_encoding(super::input_encoding()).unwrap() != *UNICODE_CODE_PAGE
            || super::is_input_redirected(),
    )
}

pub fn open_standard_output() -> global::Result<ConsoleFile> {
    get_standard_file(
        STD_OUTPUT_HANDLE,
        codepage::from_encoding(super::output_encoding()).unwrap() != *UNICODE_CODE_PAGE
            || super::is_output_redirected(),
    )
}

pub fn open_standard_error() -> global::Result<ConsoleFile> {
    get_standard_file(
        STD_OUTPUT_HANDLE,
        codepage::from_encoding(super::output_encoding()).unwrap() != *UNICODE_CODE_PAGE
            || super::is_error_redirected(),
    )
}

pub fn is_input_redirected_core() -> bool {
    is_handle_redirected(get_input_handle())
}

pub fn is_output_redirected_core() -> bool {
    is_handle_redirected(get_output_handle())
}

pub fn is_error_redirected_core() -> bool {
    is_handle_redirected(get_error_handle())
}

fn is_handle_redirected(handle: HANDLE) -> bool {
    unsafe {
        let file_type = FileSystem::GetFileType(handle);
        if (file_type.0 & FILE_TYPE_CHAR.0) != FILE_TYPE_CHAR.0 {
            true
        } else {
            !is_get_console_mod_call_successful(handle)
        }
    }
}

fn is_get_console_mod_call_successful(handle: HANDLE) -> bool {
    unsafe {
        let mode = std::alloc::alloc(Layout::new::<CONSOLE_MODE>()) as *mut CONSOLE_MODE;
        Console::GetConsoleMode(handle, mode).is_ok()
    }
}

fn get_input_handle() -> HANDLE {
    unsafe { Console::GetStdHandle(STD_INPUT_HANDLE).unwrap() }
}

fn get_output_handle() -> HANDLE {
    unsafe { Console::GetStdHandle(STD_OUTPUT_HANDLE).unwrap() }
}

fn get_error_handle() -> HANDLE {
    unsafe { Console::GetStdHandle(STD_ERROR_HANDLE).unwrap() }
}

#[allow(unused)]
fn console_handle_is_writable(out_err_handle: HANDLE) -> bool {
    let junk_byte = 0x41;
    let mut bytes_written = 0;
    unsafe {
        FileSystem::WriteFile(
            out_err_handle,
            Some(&[junk_byte]),
            Some(&mut bytes_written),
            None,
        )
        .is_ok()
    }
}

static UNICODE_CODE_PAGE: LazyLock<u16> =
    LazyLock::new(|| codepage::from_encoding(encoding_rs::UTF_16LE).unwrap());

pub fn input_encoding() -> &'static Encoding {
    unsafe { codepage::to_encoding(Console::GetConsoleCP() as _).unwrap() }
}

pub fn output_encoding() -> &'static Encoding {
    unsafe { codepage::to_encoding(Console::GetConsoleOutputCP() as _).unwrap() }
}

pub fn set_console_input_encoding(encoding: &'static Encoding) -> global::Result<()> {
    let Some(code_page) = codepage::from_encoding(encoding) else {
        return Err(EncodingError::UnsupportedEncoding(encoding.name()).into());
    };
    if code_page != *UNICODE_CODE_PAGE {
        unsafe {
            let page = codepage::from_encoding(encoding).unwrap();
            handle_set_console_encoding_error(Console::SetConsoleCP(page as _))?;
        }
    }
    Ok(())
}

pub fn set_console_output_encoding(encoding: &'static Encoding) -> global::Result<()> {
    let Some(code_page) = codepage::from_encoding(encoding) else {
        return Err(EncodingError::UnsupportedEncoding(encoding.name()).into());
    };
    if code_page != *UNICODE_CODE_PAGE {
        unsafe {
            let page = codepage::from_encoding(encoding).unwrap();
            handle_set_console_encoding_error(Console::SetConsoleOutputCP(page as _))?;
        }
    }
    Ok(())
}

fn handle_set_console_encoding_error(res: windows::core::Result<()>) -> windows::core::Result<()> {
    match res {
        Ok(()) => Ok(()),
        Err(err)
            if err.code() == ERROR_INVALID_HANDLE.to_hresult()
                || err.code() == ERROR_INVALID_ACCESS.to_hresult() =>
        {
            Ok(())
        }
        Err(e) => Err(e),
    }
}

static mut CACHED_INPUT_RECORD: INPUT_RECORD = unsafe { core::mem::zeroed() };

pub fn key_available() -> global::Result<bool> {
    unsafe {
        if CACHED_INPUT_RECORD.EventType as u32 == Console::KEY_EVENT {
            return Ok(true);
        }
        loop {
            let mut ir: [INPUT_RECORD; 1] = core::mem::zeroed();
            let mut num_events_read = 0;
            let r = Console::PeekConsoleInputW(get_input_handle(), &mut ir, &mut num_events_read);
            if let Err(e) = r {
                return if e.code() == ERROR_INVALID_HANDLE.to_hresult() {
                    Err(RuntimeError::InvalidOperation(
                        RuntimeMayBeInvalidOperation::ConsoleKeyAvailableOnFile,
                    )
                    .into())
                } else {
                    Err(e.into())
                };
            }
            if num_events_read == 0 {
                return Ok(false);
            }
            if !is_read_key_event(&ir[0]) {
                let mut buffer: [INPUT_RECORD; 1] = core::mem::zeroed();
                let mut number_of_events_read = 0;
                Console::ReadConsoleInputW(
                    get_input_handle(),
                    &mut buffer,
                    &mut number_of_events_read,
                )?;
            } else {
                return Ok(true);
            }
        }
    }
}

#[allow(nonstandard_style)]
const AltVKCode: u16 = 0x12;

fn is_read_key_event(ir: &INPUT_RECORD) -> bool {
    if ir.EventType as u32 != Console::KEY_EVENT {
        return false;
    }
    unsafe {
        if ir.Event.KeyEvent.bKeyDown == windows::core::BOOL::from(false) {
            ir.Event.KeyEvent.wVirtualKeyCode == AltVKCode
                && std::mem::transmute::<_, u16>(ir.Event.KeyEvent.uChar) != 0
        } else {
            let key_code = ir.Event.KeyEvent.wVirtualKeyCode;
            if (0x10..=0x12).contains(&key_code) {
                return false;
            }
            if key_code == 0x14 || key_code == 0x90 || key_code == 0x91 {
                return false;
            }
            let key_state =
                ControlKeyState::from_bits_unchecked(ir.Event.KeyEvent.dwControlKeyState);
            if key_state.contains(ControlKeyState::LeftAltPressed)
                || key_state.contains(ControlKeyState::RightAltPressed)
            {
                let key = std::mem::transmute::<_, ConsoleKey>(key_code);
                if key >= ConsoleKey::NumPad0 && key <= ConsoleKey::NumPad9 {
                    return false;
                }
                if !key_state.contains(ControlKeyState::EnhancedKey) {
                    if key == ConsoleKey::Clear || key == ConsoleKey::Insert {
                        return false;
                    }
                    if key >= ConsoleKey::PageUp && key <= ConsoleKey::DownArrow {
                        return false;
                    }
                }
            }
            true
        }
    }
}

#[allow(unused)]
static READ_KEY_SYNC_OBJECT: Mutex<()> = Mutex::new(());

#[allow(unused)]
pub fn read_key(intercept: bool) -> global::Result<ConsoleKeyInfo> {
    let mut ir: [INPUT_RECORD; 1] = unsafe { core::mem::zeroed() };
    let mut r;
    {
        let _lock = READ_KEY_SYNC_OBJECT.lock();
        unsafe {
            if CACHED_INPUT_RECORD.EventType as u32 == Console::KEY_EVENT {
                ir[0] = CACHED_INPUT_RECORD;
                if CACHED_INPUT_RECORD.Event.KeyEvent.wRepeatCount == 0 {
                    CACHED_INPUT_RECORD.EventType = 0;
                } else {
                    CACHED_INPUT_RECORD.Event.KeyEvent.wRepeatCount -= 1;
                }
            } else {
                loop {
                    let mut num_events_read = 0;
                    r = Console::ReadConsoleInputW(
                        get_input_handle(),
                        &mut ir,
                        &mut num_events_read,
                    );
                    if r.is_err() {
                        return Err(RuntimeError::InvalidOperation(
                            RuntimeMayBeInvalidOperation::ConsoleKeyAvailableOnFile,
                        )
                        .into());
                    }
                    if num_events_read == 0 {
                        continue;
                    }
                    if !is_read_key_event(&ir[0]) {
                        continue;
                    }
                    if ir[0].Event.KeyEvent.wRepeatCount > 1 {
                        ir[0].Event.KeyEvent.wRepeatCount -= 1;
                        CACHED_INPUT_RECORD = ir[0];
                    }
                    break;
                }
            }
        }
    }
    unsafe {
        let state = ControlKeyState::from_bits_unchecked(ir[0].Event.KeyEvent.dwControlKeyState);
        let shift = state.contains(ControlKeyState::ShiftPressed);
        let alt = state.contains(ControlKeyState::LeftAltPressed)
            || state.contains(ControlKeyState::RightAltPressed);
        let control = state.contains(ControlKeyState::LeftCtrlPressed)
            || state.contains(ControlKeyState::RightCtrlPressed);
        let info = ConsoleKeyInfo::new(
            char::from_u32_unchecked(
                std::mem::transmute::<_, u16>(ir[0].Event.KeyEvent.uChar) as u32
            ),
            std::mem::transmute(ir[0].Event.KeyEvent.wVirtualKeyCode),
            shift,
            alt,
            control,
        );
        if !intercept {
            super::write_wchar(ir[0].Event.KeyEvent.uChar.UnicodeChar);
        }
        Ok(info)
    }
}

#[enumflags2::bitflags]
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ControlKeyState {
    RightAltPressed = 0x0001,
    LeftAltPressed = 0x0002,
    RightCtrlPressed = 0x0004,
    LeftCtrlPressed = 0x0008,
    ShiftPressed = 0x0010,
    NumLockOn = 0x0020,
    ScrollLockOn = 0x0040,
    CapsLockOn = 0x0080,
    EnhancedKey = 0x0100,
}

#[allow(unused)]
pub fn treat_control_c_as_input() -> global::Result<bool> {
    unsafe {
        let handle = get_input_handle();
        if !handle.is_invalid() {
            return Err(RuntimeError::NoConsole.into());
        }
        let mut mode = std::mem::zeroed();
        Console::GetConsoleMode(handle, &mut mode)?;
        Ok((mode & Console::ENABLE_PROCESSED_INPUT).0 == 0)
    }
}

#[allow(unused)]
pub fn set_treat_control_c_as_input(value: bool) -> global::Result<()> {
    let handle = get_input_handle();
    if !handle.is_invalid() {
        return Err(RuntimeError::NoConsole.into());
    }
    unsafe {
        let mut mode = std::mem::zeroed();
        Console::GetConsoleMode(handle, &mut mode)?;
        if value {
            mode &= !Console::ENABLE_PROCESSED_INPUT;
        } else {
            mode |= Console::ENABLE_PROCESSED_INPUT;
        }
        Console::SetConsoleMode(handle, mode)?;
    }
    Ok(())
}

pub fn background_color() -> global::Result<ConsoleColor> {
    let mut succeeded = false;
    let buffer_info = get_buffer_info_inner(false, &mut succeeded)?;

    unsafe {
        if succeeded {
            Ok(
                (std::mem::transmute::<_, Kernel32Color>(buffer_info.wAttributes)
                    & Kernel32Color::BackgroundMask)
                    .into(),
            )
        } else {
            Ok(ConsoleColor::Black)
        }
    }
}

pub fn set_background_color(value: ConsoleColor) -> global::Result<()> {
    let c = Kernel32Color::from_console_color(value, true);
    let mut succeeded = false;
    let buffer_info = get_buffer_info_inner(false, &mut succeeded)?;
    if !succeeded {
        return Ok(());
    }
    debug_assert!(
        *HAVE_READ_DEFAULT_COLORS.read().unwrap(),
        "Setting the background color before we've read the default foreground color!"
    );
    let mut attrs = buffer_info.wAttributes;
    attrs &= !CONSOLE_CHARACTER_ATTRIBUTES(Kernel32Color::BackgroundMask as u16);
    attrs = CONSOLE_CHARACTER_ATTRIBUTES((attrs.0 as u32 | c as u16 as u32) as u16);
    #[allow(unused_must_use)]
    unsafe {
        Console::SetConsoleTextAttribute(get_output_handle(), attrs);
    }
    Ok(())
}

pub fn foreground_color() -> global::Result<ConsoleColor> {
    let mut succeeded = false;
    let buffer_info = get_buffer_info_inner(false, &mut succeeded)?;

    unsafe {
        if succeeded {
            Ok(
                (std::mem::transmute::<_, Kernel32Color>(buffer_info.wAttributes)
                    & Kernel32Color::ForegroundMask)
                    .into(),
            )
        } else {
            Ok(ConsoleColor::Gray)
        }
    }
}

pub fn set_foreground_color(value: ConsoleColor) -> global::Result<()> {
    let c = Kernel32Color::from_console_color(value, true);
    let mut succeeded = false;
    let buffer_info = get_buffer_info_inner(false, &mut succeeded)?;
    if !succeeded {
        return Ok(());
    }
    debug_assert!(
        *HAVE_READ_DEFAULT_COLORS.read().unwrap(),
        "Setting the background color before we've read the default foreground color!"
    );
    let mut attrs = buffer_info.wAttributes;
    attrs &= !CONSOLE_CHARACTER_ATTRIBUTES(Kernel32Color::ForegroundMask as u16);
    attrs = CONSOLE_CHARACTER_ATTRIBUTES((attrs.0 as u32 | c as u16 as u32) as u16);
    #[allow(unused_must_use)]
    unsafe {
        Console::SetConsoleTextAttribute(get_output_handle(), attrs);
    }
    Ok(())
}

pub fn reset_color() -> global::Result<()> {
    if !*HAVE_READ_DEFAULT_COLORS.read().unwrap() {
        let mut succeeded = false;
        get_buffer_info_inner(false, &mut succeeded)?;
        if !succeeded {
            return Ok(());
        }
        debug_assert!(
            !*HAVE_READ_DEFAULT_COLORS.read().unwrap(),
            "Resetting color before we've read the default foreground color!"
        );
    }
    #[allow(unused_must_use)]
    unsafe {
        Console::SetConsoleTextAttribute(
            get_output_handle(),
            CONSOLE_CHARACTER_ATTRIBUTES(*DEFAULT_COLORS.read().unwrap() as u16),
        );
    }
    Ok(())
}

fn get_buffer_info() -> global::Result<Console::CONSOLE_SCREEN_BUFFER_INFO> {
    let mut x = false;
    get_buffer_info_inner(true, &mut x)
}

static HAVE_READ_DEFAULT_COLORS: RwLock<bool> = RwLock::new(false);
static DEFAULT_COLORS: RwLock<u8> = RwLock::new(0);

pub fn buffer_width() -> global::Result<isize> {
    Ok(get_buffer_info()?.dwSize.X as isize)
}

pub fn set_buffer_width(width: isize) -> global::Result<()> {
    set_buffer_size(width, buffer_height()?)
}

pub fn buffer_height() -> global::Result<isize> {
    Ok(get_buffer_info()?.dwSize.Y as isize)
}

pub fn set_buffer_height(height: isize) -> global::Result<()> {
    set_buffer_size(buffer_width()?, height)
}

pub fn set_buffer_size(width: isize, height: isize) -> global::Result<()> {
    let console_screen_buffer_info = get_buffer_info()?;
    let sr_window = console_screen_buffer_info.srWindow;
    if width < sr_window.Right as isize + 1 || width >= i16::MAX as isize {
        return Err(RuntimeError::ConsoleBufferLessThanWindowSize { is_width: true }.into());
    }
    if height < sr_window.Bottom as isize + 1 || height >= i16::MAX as isize {
        return Err(RuntimeError::ConsoleBufferLessThanWindowSize { is_width: false }.into());
    }
    let size = COORD {
        X: width as i16,
        Y: height as i16,
    };
    unsafe {
        Console::SetConsoleScreenBufferSize(get_output_handle(), size)?;
    }
    Ok(())
}

fn get_buffer_info_inner(
    err_on_no_console: bool,
    succeeded: &mut bool,
) -> global::Result<Console::CONSOLE_SCREEN_BUFFER_INFO> {
    *succeeded = false;
    unsafe {
        let output_handle = get_output_handle();
        if output_handle.is_invalid() {
            return if err_on_no_console {
                Err(RuntimeError::NoConsole.into())
            } else {
                Ok(Default::default())
            };
        }
        let mut console_screen_buffer_info = MaybeUninit::uninit();
        if Console::GetConsoleScreenBufferInfo(
            output_handle,
            console_screen_buffer_info.as_mut_ptr(),
        )
        .is_err()
            && Console::GetConsoleScreenBufferInfo(
                get_error_handle(),
                console_screen_buffer_info.as_mut_ptr(),
            )
            .is_err()
            && let Err(err) = Console::GetConsoleScreenBufferInfo(
                get_input_handle(),
                console_screen_buffer_info.as_mut_ptr(),
            )
        {
            return if err.code() == ERROR_INVALID_HANDLE.to_hresult() && !err_on_no_console {
                Ok(Default::default())
            } else {
                Err(err.into())
            };
        }
        let console_screen_buffer_info = console_screen_buffer_info.assume_init();
        if !*HAVE_READ_DEFAULT_COLORS.read().unwrap() {
            debug_assert_eq!(
                Kernel32Color::ColorMask as u16,
                0xff,
                "Make sure one byte is large enough to store a Console color value!"
            );
            DEFAULT_COLORS
                .set(
                    (console_screen_buffer_info.wAttributes.0 & Kernel32Color::ColorMask as u16)
                        .try_into()
                        .unwrap(),
                )
                .unwrap();
            HAVE_READ_DEFAULT_COLORS.set(true).unwrap();
        }
        *succeeded = true;
        Ok(console_screen_buffer_info)
    }
}

#[repr(u16)]
#[allow(unused)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Display)]
enum Kernel32Color {
    Black = 0,
    ForegroundBlue = 0x1,
    ForegroundGreen = 0x2,
    ForegroundRed = 0x4,
    ForegroundYellow = 0x6,
    ForegroundIntensity = 0x8,
    BackgroundBlue = 0x10,
    BackgroundGreen = 0x20,
    BackgroundRed = 0x40,
    BackgroundYellow = 0x60,
    BackgroundIntensity = 0x80,

    ForegroundMask = 0xf,
    BackgroundMask = 0xf0,
    ColorMask = 0xff,
}

impl BitOr for Kernel32Color {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        unsafe { std::mem::transmute((self as u16).bitor(rhs as u16)) }
    }
}

impl BitOrAssign for Kernel32Color {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = Self::bitor(*self, rhs);
    }
}

impl BitAnd for Kernel32Color {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        unsafe { std::mem::transmute((self as u16).bitand(rhs as u16)) }
    }
}

impl BitAndAssign for Kernel32Color {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = Self::bitand(*self, rhs);
    }
}

impl PartialEq<u16> for Kernel32Color {
    fn eq(&self, rhs: &u16) -> bool {
        *self as u16 == *rhs
    }
}

impl Kernel32Color {
    pub fn from_console_color(color: ConsoleColor, is_background: bool) -> Self {
        let mut c = unsafe { std::mem::transmute(color) };
        if is_background {
            c = unsafe { std::mem::transmute((c as u16) << 4) };
        }
        c
    }
}

impl From<Kernel32Color> for ConsoleColor {
    fn from(value: Kernel32Color) -> Self {
        let mut value = value;
        if (value & Kernel32Color::ColorMask) != 0 {
            unsafe {
                value = std::mem::transmute((value as u16) >> 4);
            }
        }
        unsafe { std::mem::transmute(value) }
    }
}
