use alloc::borrow::ToOwned;
use alloc::vec::Vec;
use core::borrow::Borrow;
use core::fmt::{Formatter, Write};
use core::mem::transmute;
use core::ops::Deref;
use core::slice::{from_raw_parts, IterMut};
use core::str::FromStr;

/// A borrowed EFI string. The string is always have NUL at the end.
///
/// You can use [str](crate::str) macro to create a value of this type.
#[repr(transparent)]
#[derive(Debug, PartialEq, Eq)]
pub struct EfiStr([u16]);

impl EfiStr {
    /// An empty string with only NUL character.
    pub const EMPTY: &'static Self = unsafe { Self::new_unchecked(&[0]) };

    /// # Safety
    /// `data` must be:
    ///
    /// - NUL-terminated.
    /// - Not have any NULs in the middle.
    /// - Valid UCS-2 (not UTF-16).
    pub const unsafe fn new_unchecked(data: &[u16]) -> &Self {
        // SAFETY: This is safe because EfiStr is #[repr(transparent)].
        &*(data as *const [u16] as *const Self)
    }

    /// # Safety
    /// `ptr` must be a valid UCS-2 (not UTF-16) and NUL-terminated.
    pub unsafe fn from_ptr<'a>(ptr: *const u16) -> &'a Self {
        let mut len = 0;

        while *ptr.add(len) != 0 {
            len += 1;
        }

        Self::new_unchecked(from_raw_parts(ptr, len + 1))
    }

    /// Returnes a pointer to the first character.
    pub const fn as_ptr(&self) -> *const u16 {
        self.0.as_ptr()
    }

    /// Returnes length of this string without NUL, in character.
    pub const fn len(&self) -> usize {
        self.0.len() - 1
    }

    /// Returns `true` if this string contains only NUL character.
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns object that implement [`core::fmt::Display`] for safely printing string that may
    /// contain non-Unicode data.
    pub const fn display(&self) -> impl core::fmt::Display + '_ {
        Display(self)
    }
}

impl AsRef<EfiStr> for EfiStr {
    fn as_ref(&self) -> &EfiStr {
        self
    }
}

impl AsRef<[u8]> for EfiStr {
    fn as_ref(&self) -> &[u8] {
        let ptr = self.0.as_ptr().cast();
        let len = self.0.len() * 2;

        // SAFETY: This is safe because any alignment of u16 is a valid alignment fo u8.
        unsafe { from_raw_parts(ptr, len) }
    }
}

impl AsRef<[u16]> for EfiStr {
    fn as_ref(&self) -> &[u16] {
        &self.0
    }
}

impl ToOwned for EfiStr {
    type Owned = EfiString;

    fn to_owned(&self) -> Self::Owned {
        EfiString(self.0.to_vec())
    }
}

/// Provides [`core::fmt::Display`] to display [`EfiStr`] lossy.
struct Display<'a>(&'a EfiStr);

impl<'a> core::fmt::Display for Display<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        // SAFETY: EfiStr guarantee to have NUL at the end.
        let mut ptr = self.0.as_ptr();

        while unsafe { *ptr != 0 } {
            let c = unsafe { *ptr };
            let c = char::from_u32(c.into()).unwrap_or(char::REPLACEMENT_CHARACTER);

            f.write_char(c)?;

            unsafe { ptr = ptr.add(1) };
        }

        Ok(())
    }
}

/// An owned version of [`EfiStr`]. The string is always have NUL at the end.
#[derive(Debug)]
pub struct EfiString(Vec<u16>);

impl EfiString {
    pub fn push(&mut self, c: EfiChar) {
        self.0.pop();
        self.0.push(c.0);
        self.0.push(0);
    }

    pub fn push_str<S: AsRef<str>>(&mut self, s: S) -> Result<(), EfiStringError> {
        let s = s.as_ref();
        let l = self.0.len();

        self.0.pop();

        for (i, c) in s.chars().enumerate() {
            let e = match c {
                '\0' => EfiStringError::HasNul(i),
                '\u{10000}'.. => EfiStringError::UnsupportedChar(i, c),
                c => {
                    self.0.push(c.encode_utf16(&mut [0; 1])[0]);
                    continue;
                }
            };

            unsafe { self.0.set_len(l) };
            self.0[l - 1] = 0;

            return Err(e);
        }

        self.0.push(0);

        Ok(())
    }
}

impl FromStr for EfiString {
    type Err = EfiStringError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut v = Self(Vec::with_capacity(s.len() + 1));
        v.0.push(0);
        v.push_str(s)?;
        Ok(v)
    }
}

impl Deref for EfiString {
    type Target = EfiStr;

    fn deref(&self) -> &Self::Target {
        unsafe { EfiStr::new_unchecked(&self.0) }
    }
}

impl AsRef<EfiStr> for EfiString {
    fn as_ref(&self) -> &EfiStr {
        self.deref()
    }
}

impl Borrow<EfiStr> for EfiString {
    fn borrow(&self) -> &EfiStr {
        self.deref()
    }
}

impl<'a> IntoIterator for &'a mut EfiString {
    type Item = &'a mut EfiChar;
    type IntoIter = IterMut<'a, EfiChar>;

    fn into_iter(self) -> Self::IntoIter {
        let len = self.0.len() - 1; // Exclude NUL.

        // SAFETY: This is safe because EfiChar is #[repr(transparent)].
        unsafe { transmute(self.0[..len].iter_mut()) }
    }
}

/// A non-NUL character in the EFI string.
#[repr(transparent)]
pub struct EfiChar(u16);

impl EfiChar {
    pub const FULL_STOP: Self = Self(b'.' as u16);
    pub const REVERSE_SOLIDUS: Self = Self(b'\\' as u16);
}

impl PartialEq<u8> for EfiChar {
    fn eq(&self, other: &u8) -> bool {
        self.0 == (*other).into()
    }
}

/// Represents an error when an [`EfiString`] cnostruction is failed.
#[derive(Debug)]
pub enum EfiStringError {
    HasNul(usize),
    UnsupportedChar(usize, char),
}
