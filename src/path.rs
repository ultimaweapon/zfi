use crate::EfiStr;
use alloc::borrow::ToOwned;
use alloc::vec::Vec;
use core::borrow::Borrow;
use core::fmt::{Display, Formatter};
use core::marker::PhantomData;
use core::mem::transmute;
use core::ops::Deref;
use core::ptr::read_unaligned;

/// Represents one or more `EFI_DEVICE_PATH_PROTOCOL`.
#[repr(C)]
#[derive(Debug)]
pub struct Path {
    ty: u8,
    sub: u8,
    len: [u8; 2], // Cannot be u16 because the EFI_DEVICE_PATH_PROTOCOL is one byte alignment.
    data: [u8; 0],
}

impl Path {
    #[cfg(target_endian = "little")]
    pub const EMPTY: Path = unsafe { transmute([0x7Fu8, 0xFF, 0x04, 0x00]) };

    #[cfg(target_endian = "big")]
    pub const EMPTY: Path = unsafe { transmute([0x7Fu8, 0xFF, 0x00, 0x04]) };

    pub fn join_media_file_path<F: AsRef<EfiStr>>(&self, file: F) -> PathBuf {
        let mut buf = self.to_owned();
        buf.push_media_file_path(file);
        buf
    }

    pub fn read(&self) -> PathNode<'_> {
        let data = self.data.as_ptr();

        match (self.ty, self.sub) {
            (4, 4) => PathNode::MediaFilePath(unsafe { EfiStr::from_ptr(data as _) }),
            (t, s) => todo!("device path with type {t:#x}:{s:#x}"),
        }
    }
}

impl ToOwned for Path {
    type Owned = PathBuf;

    fn to_owned(&self) -> Self::Owned {
        // Get total length.
        let mut len: usize = 0;

        for p in self {
            len += Into::<usize>::into(u16::from_ne_bytes(p.len));
        }

        len += 4; // End of Hardware Device Path with End Entire Device Path.

        // Copy nodes.
        let src: *const u8 = unsafe { transmute(self) };
        let mut dst = Vec::with_capacity(len.next_power_of_two());

        unsafe { src.copy_to_nonoverlapping(dst.as_mut_ptr(), len) };
        unsafe { dst.set_len(len) };

        PathBuf(dst)
    }
}

impl<'a> IntoIterator for &'a Path {
    type Item = &'a Path;
    type IntoIter = PathNodes<'a>;

    fn into_iter(self) -> Self::IntoIter {
        PathNodes {
            next: unsafe { transmute(self) },
            phantom: PhantomData,
        }
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        for (i, n) in self.into_iter().enumerate() {
            if i != 0 {
                f.write_str("/")?;
            }
            n.read().fmt(f)?;
        }

        Ok(())
    }
}

/// An owned version of [`Path`].
pub struct PathBuf(Vec<u8>);

impl PathBuf {
    pub fn push_media_file_path<F: AsRef<EfiStr>>(&mut self, file: F) {
        unsafe { self.push(4, 4, file.as_ref().as_ref()) };
    }

    /// # Safety
    /// This method don't check if the combination of parameters form a valid device path.
    unsafe fn push(&mut self, ty: u8, sub: u8, data: &[u8]) {
        // Change type on the last node.
        let len: u16 = (data.len() + 4).try_into().unwrap();
        let len = len.to_ne_bytes();
        let off = self.0.len() - 4;
        let last = &mut self.0[off..];

        last[0] = ty;
        last[1] = sub;
        last[2] = len[0];
        last[3] = len[1];

        // Push data.
        self.0.extend(data);

        // Push Hardware Device Path with End Entire Device Path.
        self.0.push(0x7F);
        self.0.push(0xFF);
        self.0.extend(4u16.to_ne_bytes());
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
        unsafe { transmute(self.0.as_ptr()) }
    }
}

/// Contains the data that read from a device path node.
pub enum PathNode<'a> {
    MediaFilePath(&'a EfiStr),
}

impl<'a> Display for PathNode<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            PathNode::MediaFilePath(p) => p.fmt(f),
        }
    }
}

/// An iterator over device path nodes.
pub struct PathNodes<'a> {
    next: *const u8,
    phantom: PhantomData<&'a [Path]>,
}

impl<'a> Iterator for PathNodes<'a> {
    type Item = &'a Path;

    fn next(&mut self) -> Option<Self::Item> {
        // Do nothing if the current node is End of Hardware Device Path with End Entire Device
        // Path.
        let p = self.next;

        if unsafe { *p == 0x7F && *p.add(1) == 0xFF } {
            return None;
        }

        // Move to next node.
        self.next = unsafe { p.add(read_unaligned::<u16>(p.add(2) as _).into()) };

        Some(unsafe { transmute(p) })
    }
}
