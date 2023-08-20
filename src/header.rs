/// Represents an `EFI_TABLE_HEADER`.
#[repr(C)]
pub struct TableHeader {
    signature: u64,
    revision: TableRevision,
    header_size: u32,
    crc32: u32,
    reserved: u32,
}

impl TableHeader {
    pub fn revision(&self) -> TableRevision {
        self.revision
    }
}

/// Represents a Revision field in the `EFI_TABLE_HEADER`.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TableRevision(u32);

impl TableRevision {
    pub const fn new(major: u16, minor: u16) -> Self {
        Self(((major as u32) << 16) | minor as u32)
    }
}
