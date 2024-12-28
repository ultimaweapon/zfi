use crate::{system_table, Device, Guid, OpenProtocolAttributes, Path, SystemTable, IMAGE};
use core::ptr::null;

/// Represents an `EFI_HANDLE` for the image.
pub struct Image(());

impl Image {
    /// Gets the `EFI_LOADED_IMAGE_PROTOCOL` from this image.
    pub fn proto(&self) -> &LoadedImage {
        static ID: Guid = Guid::new(
            0x5B1B31A1,
            0x9562,
            0x11d2,
            [0x8E, 0x3F, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B],
        );

        let proto = unsafe {
            system_table()
                .boot_services()
                .open_protocol(
                    self as *const Image as *const (),
                    &ID,
                    IMAGE.cast(),
                    null(),
                    OpenProtocolAttributes::GET_PROTOCOL,
                )
                .unwrap()
        };

        unsafe { &*(proto as *const LoadedImage) }
    }
}

/// Represents an `EFI_LOADED_IMAGE_PROTOCOL`.
#[repr(C)]
pub struct LoadedImage {
    revision: u32,
    parent_handle: *const (),
    system_table: *const SystemTable,
    device_handle: *const (),
    file_path: *const u8,
    reserved: *const (),
    load_options_size: u32,
    load_options: *const (),
    image_base: *const u8,
}

impl LoadedImage {
    pub fn device(&self) -> &Device {
        unsafe { &*(self.device_handle as *const Device) }
    }

    pub fn file_path(&self) -> &Path {
        unsafe { Path::from_ptr(self.file_path) }
    }

    pub fn image_base(&self) -> *const u8 {
        self.image_base
    }
}
