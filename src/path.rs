use crate::EfiStr;
use alloc::borrow::{Cow, ToOwned};
use core::borrow::Borrow;
use core::fmt::Formatter;
use core::ops::Deref;
use core::ptr::read_unaligned;
use core::slice::from_raw_parts;

/// Represents one or more `EFI_DEVICE_PATH_PROTOCOL`.
#[repr(transparent)]
#[derive(Debug, PartialEq)]
pub struct Path([u8]);

impl Path {
    pub const EMPTY: &'static Path = unsafe { Self::new_unchecked(&[0x7F, 0xFF, 0x04, 0x00]) };

    /// # Safety
    /// `data` must be a valid device path.
    pub const unsafe fn new_unchecked(data: &[u8]) -> &Self {
        // SAFETY: This is safe because Path is #[repr(transparent)].
        &*(data as *const [u8] as *const Self)
    }

    /// # Safety
    /// `ptr` must be a valid device path.
    pub unsafe fn from_ptr<'a>(ptr: *const u8) -> &'a Self {
        let mut p = ptr;
        let mut t = 0;
        let mut l: usize;

        while *p != 0x7F || *p.add(1) != 0xFF {
            l = read_unaligned::<u16>(p.add(2) as _).into();
            t += l;
            p = p.add(l);
        }

        t += 4;

        Self::new_unchecked(from_raw_parts(ptr, t))
    }

    pub fn join_media_file_path<F: AsRef<EfiStr>>(&self, file: F) -> PathBuf {
        let mut buf = self.to_owned();
        buf.push_media_file_path(file);
        buf
    }

    pub fn to_media_file_path(&self) -> Option<&EfiStr> {
        match self.read() {
            PathNode::MediaFilePath(v) => Some(v),
        }
    }

    pub fn read(&self) -> PathNode<'_> {
        let p = &self.0[4..];

        match (self.0[0], self.0[1]) {
            (4, 4) => PathNode::MediaFilePath(unsafe { EfiStr::from_ptr(p.as_ptr() as _) }),
            (t, s) => todo!("device path with type {t:#x}:{s:#x}"),
        }
    }

    pub const fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub const fn display(&self) -> impl core::fmt::Display + '_ {
        Display(self)
    }
}

impl PartialEq<PathBuf> for Path {
    fn eq(&self, other: &PathBuf) -> bool {
        self == Borrow::<Self>::borrow(other)
    }
}

impl ToOwned for Path {
    type Owned = PathBuf;

    fn to_owned(&self) -> Self::Owned {
        PathBuf(Cow::Owned(self.0.to_vec()))
    }
}

impl<'a> IntoIterator for &'a Path {
    type Item = &'a Path;
    type IntoIter = PathNodes<'a>;

    fn into_iter(self) -> Self::IntoIter {
        PathNodes(&self.0)
    }
}

/// Provides [`core::fmt::Display`] implementation to print [`Path`].
struct Display<'a>(&'a Path);

impl core::fmt::Display for Display<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        for (i, n) in self.0.into_iter().enumerate() {
            if i != 0 {
                f.write_str("/")?;
            }

            match n.read() {
                PathNode::MediaFilePath(v) => write!(f, "{}", v.display())?,
            }
        }

        Ok(())
    }
}

/// An owned version of [`Path`].
#[derive(Debug)]
pub struct PathBuf(Cow<'static, [u8]>);

impl PathBuf {
    pub const fn new() -> Self {
        Self(Cow::Borrowed(Path::EMPTY.as_bytes()))
    }

    pub fn push_media_file_path<F: AsRef<EfiStr>>(&mut self, file: F) {
        unsafe { self.push(4, 4, file.as_ref().as_ref()) };
    }

    /// # Safety
    /// This method don't check if the combination of parameters form a valid device path.
    unsafe fn push(&mut self, ty: u8, sub: u8, data: &[u8]) {
        // Change type on the last node.
        let len: u16 = (data.len() + 4).try_into().unwrap();
        let len = len.to_ne_bytes();
        let path = self.0.to_mut();
        let off = path.len() - 4;
        let last = &mut path[off..];

        last[0] = ty;
        last[1] = sub;
        last[2] = len[0];
        last[3] = len[1];

        // Push data.
        path.extend(data);

        // Push Hardware Device Path with End Entire Device Path.
        path.push(0x7F);
        path.push(0xFF);
        path.extend(4u16.to_ne_bytes());
    }
}

impl Default for PathBuf {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for PathBuf {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.borrow()
    }
}

impl Borrow<Path> for PathBuf {
    fn borrow(&self) -> &Path {
        unsafe { Path::new_unchecked(&self.0) }
    }
}

/// Contains the data that read from a device path node.
pub enum PathNode<'a> {
    MediaFilePath(&'a EfiStr),
}

/// An iterator over device path nodes.
pub struct PathNodes<'a>(&'a [u8]);

impl<'a> Iterator for PathNodes<'a> {
    type Item = &'a Path;

    fn next(&mut self) -> Option<Self::Item> {
        // Do nothing if the current node is End of Hardware Device Path with End Entire Device
        // Path.
        let p = self.0;

        if p[0] == 0x7F && p[1] == 0xFF {
            return None;
        }

        // Move to next node.
        let l: usize = u16::from_ne_bytes(p[2..4].try_into().unwrap()).into();

        self.0 = &p[l..];

        Some(unsafe { Path::new_unchecked(p) })
    }
}
