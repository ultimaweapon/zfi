use crate::event::Event;
use crate::{system_table, EfiStr, Status};
use alloc::vec::Vec;
use core::fmt::Write;

/// Prints to the standard output, with a newline.
#[macro_export]
macro_rules! println {
    ($($args:tt)*) => {{
        use ::core::fmt::Write;

        let mut dev = $crate::system_table().stdout();

        dev.write_fmt(::core::format_args!($($args)*)).unwrap();
        dev.write_eol().unwrap();
    }};
}

/// Prints to the standard error, with a newline.
#[macro_export]
macro_rules! eprintln {
    ($($args:tt)*) => {{
        use ::core::fmt::Write;

        let mut dev = $crate::system_table().stderr();

        dev.write_fmt(::core::format_args!($($args)*)).unwrap();
        dev.write_eol().unwrap();
    }};
}

/// Wait for a key stroke.
pub fn pause() {
    let stdin = system_table().stdin();

    system_table()
        .boot_services()
        .wait_for_event(&[stdin.wait_for_key])
        .unwrap();
}

/// Represents an `EFI_SIMPLE_TEXT_INPUT_PROTOCOL`.
#[repr(C)]
pub struct SimpleTextInput {
    reset: fn(),
    read_key_stroke: fn(),
    wait_for_key: Event,
}

/// Represents an `EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL`.
#[repr(C)]
pub struct SimpleTextOutput {
    reset: fn(),
    output_string: unsafe extern "efiapi" fn(&Self, s: *const u16) -> Status,
}

impl SimpleTextOutput {
    pub fn write_eol(&self) -> Result<(), Status> {
        let eol = [0x0D, 0x0A, 0x00];

        // SAFETY: This is safe because eol has NUL at the end.
        unsafe { (self.output_string)(self, eol.as_ptr()).err_or(()) }
    }

    pub fn output_string(&self, s: &EfiStr) -> Result<(), Status> {
        // SAFETY: This is safe because EfiStr has NUL at the end.
        unsafe { (self.output_string)(self, s.as_ptr()).err_or(()) }
    }
}

impl Write for &SimpleTextOutput {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        // Encode Rust string to UCS-2.
        let mut buf = Vec::with_capacity(s.len() + 1);
        let mut prev = 0;

        for c in s.encode_utf16() {
            match c {
                0x0000 | 0xD800..=0xDFFF => return Err(core::fmt::Error),
                0x000A => {
                    // Prepend \r before \n if required.
                    if prev != 0x000D {
                        buf.push(0x000D);
                    }
                }
                _ => {}
            }

            buf.push(c);
            prev = c;
        }

        buf.push(0);

        // SAFETY: This is safe because we just push NUL at the end by the above statement.
        let status = unsafe { (self.output_string)(self, buf.as_ptr()) };

        if status.is_success() {
            Ok(())
        } else {
            Err(core::fmt::Error)
        }
    }

    fn write_char(&mut self, c: char) -> core::fmt::Result {
        // Encode to UCS-2 (not UTF-16).
        let mut buf = [0; 2];

        match c {
            '\0' | '\u{10000}'.. => return Err(core::fmt::Error),
            c => c.encode_utf16(&mut buf),
        };

        assert_eq!(buf[1], 0);

        // SAFETY: This is safe because we just ensure the second element is NUL by the above
        // statement.
        let status = unsafe { (self.output_string)(self, buf.as_ptr()) };

        if status.is_success() {
            Ok(())
        } else {
            Err(core::fmt::Error)
        }
    }
}
