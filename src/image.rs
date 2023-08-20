use crate::{Device, Guid, OpenProtocolAttributes, Path, SystemTable, IMAGE};
use core::mem::transmute;
use core::ptr::null;

/// Represents an `EFI_HANDLE` for the image.
pub struct Image(());

impl Image {
    pub fn current() -> &'static Self {
        // SAFETY: This is safe because the only place that write IMAGE is our init function.
        unsafe { IMAGE.unwrap() }
    }

    /// Gets the `EFI_LOADED_IMAGE_PROTOCOL` from this image.
    pub fn proto(&self) -> &LoadedImage {
        static ID: Guid = Guid::new(
            0x5B1B31A1,
            0x9562,
            0x11d2,
            [0x8E, 0x3F, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B],
        );

        let st = SystemTable::current();
        let bs = st.boot_services();
        let proto = unsafe {
            bs.open_protocol(
                transmute(self),
                &ID,
                transmute(Self::current()),
                null(),
                OpenProtocolAttributes::GET_PROTOCOL,
            )
            .unwrap()
        };

        unsafe { transmute(proto) }
    }
}

/// Represents an `EFI_LOADED_IMAGE_PROTOCOL`.
#[repr(C)]
pub struct LoadedImage {
    revision: u32,
    parent_handle: *const (),
    system_table: *const SystemTable,
    device_handle: *const (),
    file_path: *const Path,
    reserved: *const (),
    load_options_size: u32,
    load_options: *const (),
    image_base: *const u8,
}

impl LoadedImage {
    pub fn device(&self) -> &Device {
        unsafe { transmute(self.device_handle) }
    }

    pub fn file_path(&self) -> &Path {
        unsafe { transmute(self.file_path) }
    }

    pub fn image_base(&self) -> *const u8 {
        self.image_base
    }
}
