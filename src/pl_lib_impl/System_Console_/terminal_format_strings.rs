use std::collections::HashMap;

use crate::pl_lib_impl::System_Console_::ConsoleKeyInfo;

/// Provides format strings and related information for use with the current terminal.
pub struct TerminalFormatStrings {
    /// The format string to use to change the foreground color.
    pub foreground: Option<String>,
    /// The format string to use to change the background color.
    pub background: Option<String>,
    /// The format string to use to reset the foreground and background colors.
    pub reset: Option<String>,
    /// The maximum number of colors supported by the terminal.
    pub max_colors: i32,
    /// The number of columns in a format.
    pub columns: i32,
    /// The number of lines in a format.
    pub lines: i32,
    /// The format string to use to make cursor visible.
    pub cursor_visible: Option<String>,
    /// The format string to use to make cursor invisible.
    pub cursor_invisible: Option<String>,
    /// The format string to use to set the window title.
    pub title: Option<String>,
    /// The format string to use for an audible bell.
    pub bell: Option<String>,
    /// The format string to use to clear the terminal.
    /// # Remarks
    ///
    /// If supported, this includes
    /// the format string for first clearing the terminal scrollback buffer.
    pub clear: Option<String>,
    /// The format string to use to set the position of the cursor.
    pub cursor_address: Option<String>,
    /// The format string to use to move the cursor to the left.
    pub cursor_left: Option<String>,
    /// The format string to use to clear to the end of line.
    pub clr_eol: Option<String>,
    /* cSpell: disable */
    /// The dictionary of keystring to ConsoleKeyInfo.
    /// Only some members of the ConsoleKeyInfo are used; in particular, the actual char is ignored.
    pub key_format_to_console_key: HashMap<String, ConsoleKeyInfo>,
    /* cSpell: enable */
    /// Max key length
    pub max_key_format_length: i32,
    /// Min key length
    pub min_key_format_length: i32,
    /// The ANSI string used to enter "application" / "keypad transmit" mode.
    pub keypad_x_mit: Option<String>,
    /// Indicates that it was created out of rxvt TERM
    pub is_rxvt_term: bool,
}

impl TerminalFormatStrings {
    /// The ANSI-compatible string for the Cursor Position report request.
    ///
    /// # Remarks
    ///
    /// This should really be in user string 7 in the terminfo file, but some terminfo databases
    /// are missing it.  As this is defined to be supported by any ANSI-compatible terminal,
    /// we assume it's available; doing so means CursorTop/Left will work even if the terminfo database
    /// doesn't contain it (as appears to be the case with e.g. screen and tmux on Ubuntu), at the risk
    /// of outputting the sequence on some terminal that's not compatible.
    pub const CURSOR_POSITION_REPORT: &str = "\u{001b}[6n";
}
