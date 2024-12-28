use crate::{system_table, MemoryType};
use core::alloc::{GlobalAlloc, Layout};
use core::mem::size_of;
use core::ptr::{null_mut, read_unaligned, write_unaligned};

/// An implementation of [`GlobalAlloc`] using EFI memory pool.
pub struct PoolAllocator;

unsafe impl GlobalAlloc for PoolAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Calculate allocation size to include a spare room for adjusting alignment.
        let mut size = if layout.align() <= 8 {
            layout.size()
        } else {
            layout.size() + (layout.align() - 8)
        };

        // We will store how many bytes that we have shifted in the beginning at the end.
        size += size_of::<usize>();

        // Do allocation.
        let mem = system_table()
            .boot_services()
            .allocate_pool(MemoryType::LoaderData, size)
            .unwrap_or(null_mut());

        if mem.is_null() {
            return null_mut();
        }

        // Get number of bytes to shift so the alignment is correct.
        let misaligned = (mem as usize) % layout.align();
        let adjust = if misaligned == 0 {
            0
        } else {
            layout.align() - misaligned
        };

        // Store how many bytes have been shifted.
        let mem = mem.add(adjust);

        write_unaligned(mem.add(layout.size()) as *mut usize, adjust);

        mem
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // Get original address before alignment.
        let adjusted = read_unaligned(ptr.add(layout.size()) as *const usize);
        let ptr = ptr.sub(adjusted);

        // Free the memory.
        system_table().boot_services().free_pool(ptr).unwrap();
    }
}
