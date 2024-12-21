use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec::Vec;
use core::borrow::Borrow;
use core::fmt::{Display, Formatter};
use core::mem::transmute;
use core::slice::{from_raw_parts, IterMut};
use core::str::FromStr;

/// A borrowed EFI string. The string is always have NUL at the end.
///
/// You can use [str](crate::str) macro to create a value of this type.
#[repr(transparent)]
#[derive(Debug, PartialEq, Eq)]
pub struct EfiStr([u16]);

impl EfiStr {
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

    pub const fn as_ptr(&self) -> *const u16 {
        self.0.as_ptr()
    }

    pub const fn len(&self) -> usize {
        self.0.len() - 1
    }

    pub const fn is_empty(&self) -> bool {
        self.len() == 0
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

impl Display for EfiStr {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let d = &self.0[..(self.0.len() - 1)]; // Exclude NUL.
        let s = String::from_utf16(d).unwrap();

        f.write_str(&s)
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

impl AsRef<EfiStr> for EfiString {
    fn as_ref(&self) -> &EfiStr {
        self.borrow()
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

impl Borrow<EfiStr> for EfiString {
    fn borrow(&self) -> &EfiStr {
        unsafe { EfiStr::new_unchecked(&self.0) }
    }
}

impl Display for EfiString {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        self.as_ref().fmt(f)
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
