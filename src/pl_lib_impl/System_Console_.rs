use crate::pl_lib_impl::System_Console_::console_pal::ConsoleFile;
use encoding_rs::Encoding;
use enumflags2::{bitflags, BitFlags};
use global::errors::{RuntimeError, RuntimeMayBeInvalidOperation};
use global::getset::CopyGetters;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::sync::{Mutex, OnceLock};

#[cfg_attr(unix, path = "./System_Console_/console_pal_unix.rs")]
#[cfg_attr(windows, path = "./System_Console_/console_pal_windows.rs")]
mod console_pal;

#[repr(i16)]
#[allow(unused)]
#[derive(Debug, Copy, Clone)]
pub enum ConsoleColor {
    Black = 0,
    DarkBlue = 1,
    DarkGreen = 2,
    DarkCyan = 3,
    DarkRed = 4,
    DarkMagenta = 5,
    DarkYellow = 6,
    Gray = 7,
    DarkGray = 8,
    Blue = 9,
    Green = 10,
    Cyan = 11,
    Red = 12,
    Magenta = 13,
    Yellow = 14,
    White = 15,

    UnknownColor = -1,
}

impl ConsoleColor {
    pub fn name(&self) -> &'static str {
        macro match_self($e:expr, $($i:ident),* $(,)?) {
            match $e {
                $(Self::$i => stringify!($i),)*
            }
        }
        match_self!(
            self,
            Black,
            DarkBlue,
            DarkGreen,
            DarkCyan,
            DarkRed,
            DarkMagenta,
            DarkYellow,
            Gray,
            DarkGray,
            Blue,
            Green,
            Cyan,
            Red,
            Magenta,
            Yellow,
            White,
            UnknownColor,
        )
    }
}

#[allow(unused)]
static SYNC_OBJECT: Mutex<()> = Mutex::new(());

static _IS_STDIN_REDIRECTED: OnceLock<bool> = OnceLock::new();
static _IS_STDOUT_REDIRECTED: OnceLock<bool> = OnceLock::new();
static _IS_STDERR_REDIRECTED: OnceLock<bool> = OnceLock::new();

static mut _INPUT_ENCODING: OnceLock<&'static Encoding> = OnceLock::new();
static mut _OUTPUT_ENCODING: OnceLock<&'static Encoding> = OnceLock::new();

static mut _IN: OnceLock<ConsoleFile> = OnceLock::new();
static mut _OUT: OnceLock<ConsoleFile> = OnceLock::new();
static mut _ERROR: OnceLock<ConsoleFile> = OnceLock::new();

#[allow(unused)]
pub fn input_encoding() -> &'static Encoding {
    unsafe {
        _INPUT_ENCODING.get_or_init(|| {
            let _lock = SYNC_OBJECT.lock().unwrap();
            console_pal::input_encoding()
        })
    }
}

#[allow(unused)]
pub fn output_encoding() -> &'static Encoding {
    unsafe {
        _OUTPUT_ENCODING.get_or_init(|| {
            let _lock = SYNC_OBJECT.lock().unwrap();
            console_pal::output_encoding()
        })
    }
}

#[allow(unused)]
pub fn set_input_encoding(encoding: &'static Encoding) -> global::Result<()> {
    let _lock = SYNC_OBJECT.lock().unwrap();
    unsafe {
        console_pal::set_console_input_encoding(encoding)?;
        _INPUT_ENCODING = OnceLock::from(encoding);
        _IN = OnceLock::new();
    }
    Ok(())
}

#[allow(unused)]
pub fn set_output_encoding(encoding: &'static Encoding) -> global::Result<()> {
    let _lock = SYNC_OBJECT.lock().unwrap();
    unsafe {
        console_pal::set_console_output_encoding(encoding)?;
        _OUTPUT_ENCODING = OnceLock::from(encoding);
        if _OUT.get().is_some() && !is_output_redirected() {
            _OUT.get_mut().unwrap().flush()?;
            _OUT = OnceLock::new();
        }
        if _ERROR.get().is_some() && !is_error_redirected() {
            _ERROR.get_mut().unwrap().flush()?;
            _ERROR = OnceLock::new();
        }
        _OUTPUT_ENCODING = OnceLock::from(encoding);
    }
    Ok(())
}

#[allow(unused)]
pub fn is_input_redirected() -> bool {
    *_IS_STDIN_REDIRECTED.get_or_init(console_pal::is_input_redirected_core)
}

#[allow(unused)]
pub fn is_output_redirected() -> bool {
    *_IS_STDOUT_REDIRECTED.get_or_init(console_pal::is_output_redirected_core)
}

#[allow(unused)]
pub fn is_error_redirected() -> bool {
    *_IS_STDERR_REDIRECTED.get_or_init(console_pal::is_error_redirected_core)
}

#[allow(unused)]
pub fn get_in() -> &'static mut ConsoleFile {
    unsafe {
        _IN.get_mut_or_init(|| {
            let _lock = SYNC_OBJECT.lock().unwrap();
            console_pal::open_standard_input().unwrap()
        })
    }
}

#[allow(unused)]
pub fn get_out() -> &'static mut ConsoleFile {
    unsafe {
        _OUT.get_mut_or_init(|| {
            let _lock = SYNC_OBJECT.lock().unwrap();
            console_pal::open_standard_output().unwrap()
        })
    }
}

#[allow(unused)]
pub fn get_error() -> &'static mut ConsoleFile {
    unsafe {
        _ERROR.get_mut_or_init(|| {
            let _lock = SYNC_OBJECT.lock().unwrap();
            console_pal::open_standard_error().unwrap()
        })
    }
}

#[allow(unused)]
pub fn key_available() -> global::Result<bool> {
    if is_input_redirected() {
        Err(
            RuntimeError::InvalidOperation(RuntimeMayBeInvalidOperation::ConsoleKeyAvailableOnFile)
                .into(),
        )
    } else {
        console_pal::key_available()
    }
}

pub fn background_color() -> global::Result<ConsoleColor> {
    console_pal::background_color()
}

#[allow(unused)]
pub fn set_background_color(color: ConsoleColor) -> global::Result<()> {
    console_pal::set_background_color(color)
}

#[allow(unused)]
pub fn foreground_color() -> global::Result<ConsoleColor> {
    console_pal::foreground_color()
}

#[allow(unused)]
pub fn set_foreground_color(color: ConsoleColor) -> global::Result<()> {
    console_pal::set_foreground_color(color)
}

#[allow(unused)]
pub fn reset_color() -> global::Result<()> {
    console_pal::reset_color()
}

#[allow(unused)]
pub fn buffer_width() -> global::Result<isize> {
    console_pal::buffer_width()
}

#[allow(unused)]
pub fn buffer_height() -> global::Result<isize> {
    console_pal::buffer_height()
}

#[allow(unused)]
#[cfg(windows)]
pub fn set_buffer_width(width: isize) -> global::Result<()> {
    console_pal::set_buffer_width(width)
}

#[allow(unused)]
#[cfg(windows)]
pub fn set_buffer_height(height: isize) -> global::Result<()> {
    console_pal::set_buffer_height(height)
}

#[allow(unused)]
#[cfg(windows)]
pub fn set_buffer_size(width: isize, height: isize) -> global::Result<()> {
    console_pal::set_buffer_size(width, height)
}

#[allow(unused)]
#[derive(Copy, Clone, Debug, CopyGetters, PartialEq, Eq)]
#[get_copy = "pub"]
pub struct ConsoleKeyInfo {
    key_char: char,
    key: ConsoleKey,
    modifiers: BitFlags<ConsoleModifiers>,
}

impl ConsoleKeyInfo {
    #[allow(unused)]
    pub fn new(key_char: char, key: ConsoleKey, shift: bool, alt: bool, control: bool) -> Self {
        Self {
            key_char,
            key,
            modifiers: {
                let mut m = BitFlags::empty();
                if shift {
                    m.insert(ConsoleModifiers::Shift);
                }
                if alt {
                    m.insert(ConsoleModifiers::Alt);
                }
                if control {
                    m.insert(ConsoleModifiers::Control);
                }
                m
            },
        }
    }
}

impl Hash for ConsoleKeyInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u32(
            self.key_char as u32
                | ((self.key as u32) << 16)
                | ((self.modifiers.bits() as u32) << 24),
        );
    }
}

#[repr(u16)]
#[allow(unused)]
#[derive(PartialEq, PartialOrd, Eq, Ord, Copy, Clone, Debug)]
pub enum ConsoleKey {
    None = 0x0,
    Backspace = 0x8,
    Tab = 0x9,
    Clear = 0xC,
    Enter = 0xD,
    Pause = 0x13,
    Escape = 0x1B,
    Spacebar = 0x20,
    PageUp = 0x21,
    PageDown = 0x22,
    End = 0x23,
    Home = 0x24,
    LeftArrow = 0x25,
    UpArrow = 0x26,
    RightArrow = 0x27,
    DownArrow = 0x28,
    Select = 0x29,
    Print = 0x2A,
    Execute = 0x2B,
    PrintScreen = 0x2C,
    Insert = 0x2D,
    Delete = 0x2E,
    Help = 0x2F,
    D0 = 0x30, // 0 through 9
    D1 = 0x31,
    D2 = 0x32,
    D3 = 0x33,
    D4 = 0x34,
    D5 = 0x35,
    D6 = 0x36,
    D7 = 0x37,
    D8 = 0x38,
    D9 = 0x39,
    A = 0x41,
    B = 0x42,
    C = 0x43,
    D = 0x44,
    E = 0x45,
    F = 0x46,
    G = 0x47,
    H = 0x48,
    I = 0x49,
    J = 0x4A,
    K = 0x4B,
    L = 0x4C,
    M = 0x4D,
    N = 0x4E,
    O = 0x4F,
    P = 0x50,
    Q = 0x51,
    R = 0x52,
    S = 0x53,
    T = 0x54,
    U = 0x55,
    V = 0x56,
    W = 0x57,
    X = 0x58,
    Y = 0x59,
    Z = 0x5A,
    LeftWindows = 0x5B,  // Microsoft Natural keyboard
    RightWindows = 0x5C, // Microsoft Natural keyboard
    Applications = 0x5D, // Microsoft Natural keyboard
    Sleep = 0x5F,
    NumPad0 = 0x60,
    NumPad1 = 0x61,
    NumPad2 = 0x62,
    NumPad3 = 0x63,
    NumPad4 = 0x64,
    NumPad5 = 0x65,
    NumPad6 = 0x66,
    NumPad7 = 0x67,
    NumPad8 = 0x68,
    NumPad9 = 0x69,
    Multiply = 0x6A,
    Add = 0x6B,
    Separator = 0x6C,
    Subtract = 0x6D,
    Decimal = 0x6E,
    Divide = 0x6F,
    F1 = 0x70,
    F2 = 0x71,
    F3 = 0x72,
    F4 = 0x73,
    F5 = 0x74,
    F6 = 0x75,
    F7 = 0x76,
    F8 = 0x77,
    F9 = 0x78,
    F10 = 0x79,
    F11 = 0x7A,
    F12 = 0x7B,
    F13 = 0x7C,
    F14 = 0x7D,
    F15 = 0x7E,
    F16 = 0x7F,
    F17 = 0x80,
    F18 = 0x81,
    F19 = 0x82,
    F20 = 0x83,
    F21 = 0x84,
    F22 = 0x85,
    F23 = 0x86,
    F24 = 0x87,
    BrowserBack = 0xA6,       // Windows 2000/XP
    BrowserForward = 0xA7,    // Windows 2000/XP
    BrowserRefresh = 0xA8,    // Windows 2000/XP
    BrowserStop = 0xA9,       // Windows 2000/XP
    BrowserSearch = 0xAA,     // Windows 2000/XP
    BrowserFavorites = 0xAB,  // Windows 2000/XP
    BrowserHome = 0xAC,       // Windows 2000/XP
    VolumeMute = 0xAD,        // Windows 2000/XP
    VolumeDown = 0xAE,        // Windows 2000/XP
    VolumeUp = 0xAF,          // Windows 2000/XP
    MediaNext = 0xB0,         // Windows 2000/XP
    MediaPrevious = 0xB1,     // Windows 2000/XP
    MediaStop = 0xB2,         // Windows 2000/XP
    MediaPlay = 0xB3,         // Windows 2000/XP
    LaunchMail = 0xB4,        // Windows 2000/XP
    LaunchMediaSelect = 0xB5, // Windows 2000/XP
    LaunchApp1 = 0xB6,        // Windows 2000/XP
    LaunchApp2 = 0xB7,        // Windows 2000/XP
    Oem1 = 0xBA,
    OemPlus = 0xBB,
    OemComma = 0xBC,
    OemMinus = 0xBD,
    OemPeriod = 0xBE,
    Oem2 = 0xBF,
    Oem3 = 0xC0,
    Oem4 = 0xDB,
    Oem5 = 0xDC,
    Oem6 = 0xDD,
    Oem7 = 0xDE,
    Oem8 = 0xDF,
    Oem102 = 0xE2,  // Win2K/XP: Either angle or backslash on RT 102-key keyboard
    Process = 0xE5, // Windows: IME Process Key
    Packet = 0xE7,  // Win2K/XP: Used to pass Unicode chars as if keystrokes
    Attention = 0xF6,
    CrSel = 0xF7,
    ExSel = 0xF8,
    EraseEndOfFile = 0xF9,
    Play = 0xFA,
    Zoom = 0xFB,
    NoName = 0xFC, // Reserved
    Pa1 = 0xFD,
    OemClear = 0xFE,
}

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ConsoleModifiers {
    Alt = 1,
    Shift = 2,
    Control = 4,
}

#[allow(unused)]
pub fn write_wchar(c: u16) {
    get_out().write_all(&c.to_le_bytes()).unwrap();
}

#[allow(nonstandard_style)]
pub mod to_vm {
    use crate::pl_lib_impl::System_Enum::System_Enum;
    use crate::pl_lib_impl::System_Object::System_Object;
    use crate::pl_lib_impl::System_Void::System_Void;
    use crate::pl_lib_impl::{ClassLoadToCore, StructLoadToCore, System_Console_};
    use crate::type_system::{
        Assembly, AssemblyManager, Class, CommonMethod, CommonMethodTable, Struct, TypeHandle,
    };
    use crate::value::Value;
    use crate::vm::CPU;
    use enumflags2::make_bitflags;
    use global::attrs::{
        ClassImplementationFlags, MethodAttr, MethodImplementationFlags, StructImplementationFlags,
        TypeAttr, TypeSpecificAttr, Visibility,
    };
    use global::errors::{DynamicCheckingItem, RuntimeError};
    use global::{
        indexmap, string_name, IndexMap, StringMethodReference, StringName, StringTypeReference,
    };
    use std::io::Write;
    use std::panic::Location;
    use std::sync::Arc;

    pub struct System_Console;

    impl System_Console {
        /// Sign: `.sctor()`
        fn sctor(
            _method: &CommonMethod<Class>,
            _cpu: Arc<CPU>,
            _this_val: &mut Value,
            _args: &mut [Value],
            _register_start: u64,
        ) -> global::Result<Value> {
            super::input_encoding();
            super::output_encoding();
            Ok(Value::Void)
        }
        fn get_BackgroundColor(
            method: &CommonMethod<Class>,
            cpu: Arc<CPU>,
            this_val: &mut Value,
            args: &mut [Value],
            _register_start: u64,
        ) -> global::Result<Value> {
            if cpu.vm().is_dynamic_checking_enabled() {
                if !args.is_empty() {
                    return Err(
                        RuntimeError::DynamicCheckingFailed(DynamicCheckingItem::ArgLen {
                            got: args.len(),
                            expected: 0,
                        })
                        .throw()
                        .into(),
                    );
                }
                let ty = this_val.ty(cpu.clone())?.string_reference();
                if ty != method.mt().ty().string_reference() {
                    return Err(
                        RuntimeError::DynamicCheckingFailed(DynamicCheckingItem::Type {
                            got: ty,
                            expected: method.mt().ty().string_reference(),
                        })
                        .throw()
                        .into(),
                    );
                }
            }
            let color = System_Console_::background_color()?;
            let name = color.name();
            cpu.vm().get_static_from_str(
                &StringTypeReference::make_static_single("!", "System.ConsoleColor"),
                name,
            )
        }

        /// Sign: `WriteLine([!]System.String)`
        fn WriteLine__System_String(
            _method: &CommonMethod<Class>,
            cpu: Arc<CPU>,
            this_val: &mut Value,
            args: &mut [Value],
            _register_start: u64,
        ) -> global::Result<Value> {
            if cpu.vm().is_dynamic_checking_enabled() {
                if args.len() != 1 {
                    return Err(
                        RuntimeError::DynamicCheckingFailed(DynamicCheckingItem::ArgLen {
                            got: args.len(),
                            expected: 1,
                        })
                        .throw()
                        .into(),
                    );
                }
                let ty = this_val.string_type_reference();
                if ty != StringTypeReference::core_static_single_type("System.Void") {
                    return Err(
                        RuntimeError::DynamicCheckingFailed(DynamicCheckingItem::Type {
                            got: ty,
                            expected: StringTypeReference::core_static_single_type("System.Void"),
                        })
                        .throw()
                        .into(),
                    );
                }
            }
            let Some(arg0) = args.first() else {
                eprintln!("Abnormally return {}", Location::caller());
                return Ok(Value::Void);
            };
            if cpu.vm().is_dynamic_checking_enabled() {
                let ty = arg0.string_type_reference();
                if ty != StringTypeReference::core_static_single_type("System.String") {
                    return Err(
                        RuntimeError::DynamicCheckingFailed(DynamicCheckingItem::Type {
                            got: ty,
                            expected: StringTypeReference::core_static_single_type("System.String"),
                        })
                        .throw()
                        .into(),
                    );
                }
            }
            let Ok((arg0,)) = arg0.unwrap_reference_ref() else {
                eprintln!("Abnormally return {}", Location::caller());
                return Ok(Value::Void);
            };
            let Ok((arg0,)) = arg0.unwrap_string_ref() else {
                eprintln!("Abnormally return {}", Location::caller());
                return Ok(Value::Void);
            };
            let mut output = arg0.get().as_bytes().to_vec();
            output.push(b'\n');
            super::get_out().write_all(output.as_slice())?;
            Ok(Value::Void)
        }
    }

    impl ClassLoadToCore for System_Console {
        const STRING_TYPE_REFERENCE: StringTypeReference =
            StringTypeReference::core_static_single_type("System.Console");
        fn load_class(core_assembly: &Arc<Assembly>, _: &AssemblyManager) {
            let class = Class::new(
                core_assembly,
                TypeAttr::new(
                    Visibility::Public,
                    TypeSpecificAttr::Class(make_bitflags!(ClassImplementationFlags::{Static})),
                ),
                Self::STRING_TYPE_REFERENCE.unwrap_single_name_ref().clone(),
                |class| {
                    CommonMethodTable::new(
                        |mt_ptr| {
                            indexmap! {
                                string_name!(".ctor()") => CommonMethod::native(
                                    string_name!(".ctor()"),
                                    MethodAttr::new(Visibility::Public, make_bitflags!(MethodImplementationFlags::{}), 0),
                                    mt_ptr,
                                    core_assembly.get_type(&AssemblyManager::System_Void_STRUCT_REF).unwrap(),
                                    vec![],
                                    Default::default(),
                                    |_, _, _, _, _| Err(RuntimeError::ConstructStaticClass.into()),
                                ),
                                StringMethodReference::STATIC_CTOR_REF.unwrap_single() => CommonMethod::native(
                                    StringMethodReference::STATIC_CTOR_REF.unwrap_single(),
                                    MethodAttr::new(Visibility::Public, make_bitflags!(MethodImplementationFlags::{}), 0),
                                    mt_ptr,
                                    TypeHandle::Unloaded(AssemblyManager::System_Void_STRUCT_REF),
                                    vec![],
                                    Default::default(),
                                    Self::sctor,
                                ),
                                string_name!("get_BackgroundColor()") => CommonMethod::native(
                                    string_name!("get_BackgroundColor()"),
                                    MethodAttr::new(Visibility::Public, make_bitflags!(MethodImplementationFlags::{Static}), 0),
                                    mt_ptr,
                                    TypeHandle::Unloaded(StringTypeReference::make_static_single("!", "System.ConsoleColor")),
                                    vec![],
                                    Default::default(),
                                    Self::get_BackgroundColor,
                                ),
                                string_name!("WriteLine([!]System.String)") => CommonMethod::native(
                                    string_name!("WriteLine([!]System.String)"),
                                    MethodAttr::new(Visibility::Public, make_bitflags!(MethodImplementationFlags::{Static}), 0),
                                    mt_ptr,
                                    TypeHandle::Unloaded(StringTypeReference::core_static_single_type("System.Void")),
                                    vec![],
                                    Default::default(),
                                    Self::WriteLine__System_String,
                                ),
                            }
                        },
                        &class,
                        Some(
                            core_assembly
                                .get_type(&System_Object::STRING_TYPE_REFERENCE)
                                .unwrap(),
                        ),
                    )
                },
                IndexMap::new(),
            );
            core_assembly.add_type(TypeHandle::Class(class));
        }
    }

    pub struct System_ConsoleColor;
    impl System_ConsoleColor {
        fn sctor(
            _: &CommonMethod<Struct>,
            cpu: Arc<CPU>,
            this_val: &mut Value,
            args: &mut [Value],
            _register_start: u64,
        ) -> global::Result<Value> {
            if cpu.vm().is_dynamic_checking_enabled() {
                if !args.is_empty() {
                    return Err(
                        RuntimeError::DynamicCheckingFailed(DynamicCheckingItem::ArgLen {
                            got: args.len(),
                            expected: 0,
                        })
                        .throw()
                        .into(),
                    );
                }
                let ty = this_val.ty(cpu)?.string_reference();
                if ty != StringTypeReference::core_static_single_type("System.ConsoleColor") {
                    return Err(
                        RuntimeError::DynamicCheckingFailed(DynamicCheckingItem::Type {
                            got: ty,
                            expected: StringTypeReference::core_static_single_type(
                                "System.ConsoleColor",
                            ),
                        })
                        .throw()
                        .into(),
                    );
                }
            }
            let (this_val,) = this_val.unwrap_struct_mut()?;
            this_val.get_mut_field("Black")?.set_val(&Value::UInt8(0));
            this_val
                .get_mut_field("DarkBlue")?
                .set_val(&Value::UInt8(1));
            this_val
                .get_mut_field("DarkGreen")?
                .set_val(&Value::UInt8(2));
            this_val
                .get_mut_field("DarkCyan")?
                .set_val(&Value::UInt8(3));
            this_val.get_mut_field("DarkRed")?.set_val(&Value::UInt8(4));
            this_val
                .get_mut_field("DarkMagenta")?
                .set_val(&Value::UInt8(5));
            this_val
                .get_mut_field("DarkYellow")?
                .set_val(&Value::UInt8(6));
            this_val.get_mut_field("Gray")?.set_val(&Value::UInt8(7));
            this_val
                .get_mut_field("DarkGray")?
                .set_val(&Value::UInt8(8));
            this_val.get_mut_field("Blue")?.set_val(&Value::UInt8(9));
            this_val.get_mut_field("Green")?.set_val(&Value::UInt8(10));
            this_val.get_mut_field("Cyan")?.set_val(&Value::UInt8(11));
            this_val.get_mut_field("Red")?.set_val(&Value::UInt8(12));
            this_val
                .get_mut_field("Magenta")?
                .set_val(&Value::UInt8(13));
            this_val.get_mut_field("Yellow")?.set_val(&Value::UInt8(14));
            this_val.get_mut_field("White")?.set_val(&Value::UInt8(15));
            Ok(Value::Void)
        }
    }
    impl StructLoadToCore for System_ConsoleColor {
        const STRING_TYPE_REFERENCE: StringTypeReference =
            StringTypeReference::core_single_type(Self::TYPE_NAME);
        fn load_struct(core_assembly: &Arc<Assembly>, _: &AssemblyManager) {
            use global::attrs::FieldImplementationFlags;
            macro make_field($name:literal) {
                $crate::type_system::StructField::new(
                    ::global::StringName::from_static_str($name),
                    ::global::attrs::FieldAttr::new(global::attrs::Visibility::Public, make_bitflags!(FieldImplementationFlags::{Static})),
                    $crate::type_system::TypeHandle::Unloaded(StringTypeReference::core_static_single_type("System.UInt8")),
                )
            }
            let r#struct = Struct::new(
                core_assembly,
                TypeAttr::new(
                    Visibility::Public,
                    TypeSpecificAttr::Struct(make_bitflags!(StructImplementationFlags::{})),
                ),
                string_name!("System.ConsoleColor"),
                |s| {
                    CommonMethodTable::new(
                        |mt_ptr| {
                            indexmap! {
                                StringMethodReference::STATIC_CTOR_REF.unwrap_single() => CommonMethod::native(
                                    StringMethodReference::STATIC_CTOR_REF.unwrap_single(),
                                    MethodAttr::new(Visibility::Public, make_bitflags!(MethodImplementationFlags::{}), 0),
                                    mt_ptr,
                                    core_assembly.get_type(&System_Void::STRING_TYPE_REFERENCE).unwrap(),
                                    vec![],
                                    Default::default(),
                                    Self::sctor,
                                )
                            }
                        },
                        &s,
                        Some(
                            core_assembly
                                .get_type(&System_Enum::STRING_TYPE_REFERENCE)
                                .unwrap(),
                        ),
                    )
                },
                indexmap! {
                    string_name!("Black") => make_field!("Black"),
                    string_name!("DarkBlue") => make_field!("DarkBlue"),
                    string_name!("DarkGreen") => make_field!("DarkGreen"),
                    string_name!("DarkCyan") => make_field!("DarkCyan"),
                    string_name!("DarkRed") => make_field!("DarkRed"),
                    string_name!("DarkMagenta") => make_field!("DarkMagenta"),
                    string_name!("DarkYellow") => make_field!("DarkYellow"),
                    string_name!("Gray") => make_field!("Gray"),
                    string_name!("DarkGray") => make_field!("DarkGray"),
                    string_name!("Blue") => make_field!("Blue"),
                    string_name!("Green") => make_field!("Green"),
                    string_name!("Cyan") => make_field!("Cyan"),
                    string_name!("Red") => make_field!("Red"),
                    string_name!("Magenta") => make_field!("Magenta"),
                    string_name!("Yellow") => make_field!("Yellow"),
                    string_name!("White") => make_field!("White"),
                },
            );
            core_assembly.add_type(TypeHandle::Struct(r#struct));
        }
    }
    impl System_ConsoleColor {
        pub const TYPE_NAME: StringName = string_name!("System.ConsoleColor");
    }
}
