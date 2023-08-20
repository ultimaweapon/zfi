use crate::{AllocateType, MemoryDescriptor, MemoryType, Status, SystemTable};
use alloc::vec::Vec;
use core::ops::{Deref, DerefMut};

pub const PAGE_SIZE: usize = 4096; // Getting from EFI_BOOT_SERVICES.FreePages() specs.

/// Gets how many pages required for a specified number of bytes.
pub fn page_count(bytes: usize) -> usize {
    (bytes / PAGE_SIZE) + if bytes % PAGE_SIZE == 0 { 0 } else { 1 }
}

/// A shortcut to [`super::BootServices::allocate_pages()`].
pub fn allocate_pages(
    at: AllocateType,
    mt: MemoryType,
    pages: usize,
    addr: u64,
) -> Result<Pages, Status> {
    SystemTable::current()
        .boot_services()
        .allocate_pages(at, mt, pages, addr)
}

/// Just a shortcut to [`super::BootServices::get_memory_map()`]. Do not discard the returned map if
/// you want the key to use with [`super::BootServices::exit_boot_services()`].
pub fn get_memory_map() -> Result<(Vec<MemoryDescriptor>, usize), Status> {
    SystemTable::current().boot_services().get_memory_map()
}

/// Encapsulate a pointer to one or more memory pages.
pub struct Pages {
    ptr: *mut u8,
    len: usize, // In bytes.
}

impl Pages {
    /// # Safety
    /// `ptr` must be valid and was allocated with [`super::BootServices::allocate_pages()`].
    pub unsafe fn new(ptr: *mut u8, len: usize) -> Self {
        Self { ptr, len }
    }

    pub fn addr(&self) -> usize {
        self.ptr as _
    }
}

impl Drop for Pages {
    fn drop(&mut self) {
        unsafe {
            SystemTable::current()
                .boot_services()
                .free_pages(self.ptr, self.len / PAGE_SIZE)
                .unwrap()
        };
    }
}

impl Deref for Pages {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { core::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl DerefMut for Pages {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { core::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}
