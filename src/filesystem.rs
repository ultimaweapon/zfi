use crate::{EfiStr, Guid, Owned, Status, Time};
use alloc::alloc::{alloc, dealloc, handle_alloc_error};
use alloc::borrow::ToOwned;
use bitflags::bitflags;
use core::alloc::Layout;
use core::borrow::{Borrow, BorrowMut};
use core::fmt::{Display, Formatter};
use core::mem::zeroed;
use core::ops::{Deref, DerefMut};
use core::ptr::{null_mut, read, slice_from_raw_parts, slice_from_raw_parts_mut};

/// Represents an `EFI_SIMPLE_FILE_SYSTEM_PROTOCOL`.
#[repr(C)]
pub struct SimpleFileSystem {
    revision: u64,
    open_volume: unsafe extern "efiapi" fn(&Self, *mut *mut File) -> Status,
}

impl SimpleFileSystem {
    pub const ID: Guid = Guid::new(
        0x0964e5b22,
        0x6459,
        0x11d2,
        [0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
    );

    /// Opens the root directory on a volume.
    pub fn open(&self) -> Result<Owned<File>, Status> {
        let mut root = null_mut();
        let status = unsafe { (self.open_volume)(self, &mut root) };

        if status != Status::SUCCESS {
            Err(status)
        } else {
            Ok(unsafe { Owned::new(root, File::dtor) })
        }
    }
}

/// Represents an `EFI_FILE_PROTOCOL`.
#[repr(C)]
pub struct File {
    revision: u64,
    open: unsafe extern "efiapi" fn(
        &Self,
        *mut *mut Self,
        *const u16,
        FileModes,
        FileAttributes,
    ) -> Status,
    close: unsafe extern "efiapi" fn(*mut Self) -> Status,
    delete: fn(),
    read: unsafe extern "efiapi" fn(&Self, *mut usize, *mut u8) -> Status,
    write: unsafe extern "efiapi" fn(&Self, *mut usize, *const u8) -> Status,
    get_position: fn(),
    set_position: extern "efiapi" fn(&Self, u64) -> Status,
    get_info: unsafe extern "efiapi" fn(&Self, *const Guid, *mut usize, *mut u8) -> Status,
    set_info: unsafe extern "efiapi" fn(&Self, *const Guid, usize, *const u8) -> Status,
    flush: extern "efiapi" fn(&Self) -> Status,
}

impl File {
    /// Creates a file relative to the current file's location.
    ///
    /// This function will create a file if it does not exist, and will truncate it if it does. If
    /// the filename starts with a `\` the relative location is the root directory that the current
    /// file resides on.
    pub fn create<N: AsRef<EfiStr>>(
        &self,
        name: N,
        attrs: FileAttributes,
    ) -> Result<Owned<Self>, FileCreateError> {
        // Create the file.
        let mut file = match self.open(
            name,
            FileModes::READ | FileModes::WRITE | FileModes::CREATE,
            attrs,
        ) {
            Ok(v) => v,
            Err(e) => return Err(FileCreateError::CreateFailed(e)),
        };

        // Truncate the file.
        if let Err(e) = file.set_len(0) {
            return Err(FileCreateError::TruncateFailed(e));
        }

        Ok(file)
    }

    /// Opens a file relative to the current file's location.
    ///
    /// If the filename starts with a `\` the relative location is the root directory that the
    /// current file resides on.
    pub fn open<N: AsRef<EfiStr>>(
        &self,
        name: N,
        modes: FileModes,
        attrs: FileAttributes,
    ) -> Result<Owned<Self>, Status> {
        let mut out = null_mut();
        let status = unsafe { (self.open)(self, &mut out, name.as_ref().as_ptr(), modes, attrs) };

        if status != Status::SUCCESS {
            Err(status)
        } else {
            Ok(unsafe { Owned::new(out, Self::dtor) })
        }
    }

    /// Reads data from the file.
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, Status> {
        let mut len = buf.len();
        let status = unsafe { (self.read)(self, &mut len, buf.as_mut_ptr()) };

        if status != Status::SUCCESS {
            Err(status)
        } else {
            Ok(len)
        }
    }

    /// Writes data to the file.
    ///
    /// Partial writes only occur when there has been a data error during the write attempt (such as
    /// "file space full").
    pub fn write(&mut self, buf: &[u8]) -> Result<usize, Status> {
        let mut len = buf.len();

        unsafe { (self.write)(self, &mut len, buf.as_ptr()).err_or(len) }
    }

    /// Sets a file's current position.
    pub fn set_position(&mut self, position: u64) -> Result<(), Status> {
        let status = (self.set_position)(self, position);

        if status != Status::SUCCESS {
            Err(status)
        } else {
            Ok(())
        }
    }

    pub fn info(&self) -> Result<FileInfoBuf, Status> {
        // Try until the buffer is enought.
        let mut layout = Layout::from_size_align(0x52, 8).unwrap();
        let info = loop {
            // Allocate a buffer.
            let mut len = layout.size();
            let info = unsafe { alloc(layout) };

            if info.is_null() {
                handle_alloc_error(layout);
            }

            // Get info.
            let status = unsafe { (self.get_info)(self, &FileInfo::ID, &mut len, info) };

            if status == Status::SUCCESS {
                break info;
            }

            // Check if we need to try again.
            unsafe { dealloc(info, layout) };

            if status != Status::BUFFER_TOO_SMALL {
                return Err(status);
            }

            // Update memory layout and try again.
            layout = Layout::from_size_align(len, 8).unwrap();
        };

        Ok(FileInfoBuf {
            buf: info,
            len: layout.size(),
        })
    }

    pub fn set_len(&mut self, len: u64) -> Result<(), FileSetLenError> {
        // Load current info.
        let mut info = match self.info() {
            Ok(v) => v,
            Err(e) => return Err(FileSetLenError::GetInfoFailed(e)),
        };

        if info.attributes().contains(FileAttributes::DIRECTORY) {
            return Err(FileSetLenError::FileIsDirectory);
        }

        // Update the info.
        *info.file_size_mut() = len;
        *info.create_time_mut() = unsafe { zeroed() };
        *info.last_accessed_mut() = unsafe { zeroed() };
        *info.last_modified_mut() = unsafe { zeroed() };

        // Set the info.
        let status = unsafe { (self.set_info)(self, &FileInfo::ID, info.0.len(), info.0.as_ptr()) };

        if status != Status::SUCCESS {
            Err(FileSetLenError::SetInfoFailed(status))
        } else {
            Ok(())
        }
    }

    /// Flushes all modified data associated with a file to a device.
    pub fn flush(&mut self) -> Result<(), Status> {
        (self.flush)(self).err_or(())
    }

    fn dtor(f: *mut Self) {
        unsafe { assert_eq!(((*f).close)(f), Status::SUCCESS) };
    }
}

bitflags! {
    /// Flags to control how to open a [`File`].
    ///
    /// The only valid combinations that the file may be opened with are: read, read/write, or
    /// create/read/write.
    #[repr(transparent)]
    pub struct FileModes: u64 {
        const READ = 0x0000000000000001;
        const WRITE = 0x0000000000000002;
        const CREATE = 0x8000000000000000;
    }
}

bitflags! {
    /// Attributes of the file to create.
    #[repr(transparent)]
    pub struct FileAttributes: u64 {
        const READ_ONLY = 0x0000000000000001;
        const HIDDEN = 0x0000000000000002;
        const SYSTEM = 0x0000000000000004;
        const RESERVED = 0x0000000000000008;
        const DIRECTORY = 0x0000000000000010;
        const ARCHIVE = 0x0000000000000020;
    }
}

/// A borrowed `EFI_FILE_INFO`.
///
/// Do not depend on a transparent representation as a slice because it will be removed in the
/// future when a Dynamically Sized Type can be safely construct on stable Rust.
#[repr(transparent)]
pub struct FileInfo([u8]);

impl FileInfo {
    pub const ID: Guid = Guid::new(
        0x09576e92,
        0x6d3f,
        0x11d2,
        [0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
    );

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn file_size(&self) -> u64 {
        unsafe { read(self.0.as_ptr().add(0x08) as _) }
    }

    pub fn file_size_mut(&mut self) -> &mut u64 {
        unsafe { &mut *(self.0.as_mut_ptr().add(0x08) as *mut u64) }
    }

    pub fn physical_size(&self) -> u64 {
        unsafe { read(self.0.as_ptr().add(0x10) as _) }
    }

    pub fn create_time(&self) -> &Time {
        unsafe { &*(self.0.as_ptr().add(0x18) as *const Time) }
    }

    pub fn create_time_mut(&mut self) -> &mut Time {
        unsafe { &mut *(self.0.as_mut_ptr().add(0x18) as *mut Time) }
    }

    pub fn last_accessed(&self) -> &Time {
        unsafe { &*(self.0.as_ptr().add(0x28) as *const Time) }
    }

    pub fn last_accessed_mut(&mut self) -> &mut Time {
        unsafe { &mut *(self.0.as_mut_ptr().add(0x28) as *mut Time) }
    }

    pub fn last_modified(&self) -> &Time {
        unsafe { &*(self.0.as_ptr().add(0x38) as *const Time) }
    }

    pub fn last_modified_mut(&mut self) -> &mut Time {
        unsafe { &mut *(self.0.as_mut_ptr().add(0x38) as *mut Time) }
    }

    pub fn attributes(&self) -> FileAttributes {
        unsafe { read(self.0.as_ptr().add(0x48) as _) }
    }

    pub fn attributes_mut(&mut self) -> &mut FileAttributes {
        unsafe { &mut *(self.0.as_mut_ptr().add(0x48) as *mut FileAttributes) }
    }

    pub fn file_name(&self) -> &EfiStr {
        unsafe { EfiStr::from_ptr(self.0.as_ptr().add(0x50) as _) }
    }
}

impl ToOwned for FileInfo {
    type Owned = FileInfoBuf;

    fn to_owned(&self) -> Self::Owned {
        let len = self.0.len();
        let layout = Layout::from_size_align(len, 8).unwrap();
        let buf = unsafe { alloc(layout) };

        if buf.is_null() {
            handle_alloc_error(layout);
        }

        unsafe { buf.copy_from_nonoverlapping(self.0.as_ptr(), len) };

        FileInfoBuf { buf, len }
    }
}

/// An owned version of [`FileInfo`].
pub struct FileInfoBuf {
    buf: *mut u8, // Must be 8 bytes aligment.
    len: usize,
}

impl Drop for FileInfoBuf {
    fn drop(&mut self) {
        unsafe { dealloc(self.buf, Layout::from_size_align(self.len, 8).unwrap()) };
    }
}

impl Deref for FileInfoBuf {
    type Target = FileInfo;

    fn deref(&self) -> &Self::Target {
        self.borrow()
    }
}

impl DerefMut for FileInfoBuf {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.borrow_mut()
    }
}

impl Borrow<FileInfo> for FileInfoBuf {
    fn borrow(&self) -> &FileInfo {
        unsafe { &*(slice_from_raw_parts(self.buf, self.len) as *const FileInfo) }
    }
}

impl BorrowMut<FileInfo> for FileInfoBuf {
    fn borrow_mut(&mut self) -> &mut FileInfo {
        unsafe { &mut *(slice_from_raw_parts_mut(self.buf, self.len) as *mut FileInfo) }
    }
}

/// Represents an error when [`File::create()`] is failed.
#[derive(Debug)]
pub enum FileCreateError {
    CreateFailed(Status),
    TruncateFailed(FileSetLenError),
}

impl Display for FileCreateError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CreateFailed(e) => e.fmt(f),
            Self::TruncateFailed(e) => write!(f, "cannot truncate the file -> {e}"),
        }
    }
}

/// Represents an error when [`File::set_len()`] is failed.
#[derive(Debug)]
pub enum FileSetLenError {
    GetInfoFailed(Status),
    FileIsDirectory,
    SetInfoFailed(Status),
}

impl Display for FileSetLenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::GetInfoFailed(e) => write!(f, "cannot get the current info -> {e}"),
            Self::FileIsDirectory => f.write_str("file is a directory"),
            Self::SetInfoFailed(e) => write!(f, "cannot set file info -> {e}"),
        }
    }
}
