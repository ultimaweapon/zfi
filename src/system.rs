use crate::{BootServices, RuntimeServices, SimpleTextInput, SimpleTextOutput, TableHeader};

/// Represents an `EFI_SYSTEM_TABLE`.
#[repr(C)]
pub struct SystemTable {
    hdr: TableHeader,
    firmware_vendor: *const u16,
    firmware_revision: u32,
    console_in_handle: *const (),
    con_in: *const SimpleTextInput,
    console_out_handle: *const (),
    con_out: *const SimpleTextOutput,
    standard_error_handle: *const (),
    std_err: *const SimpleTextOutput,
    runtime_services: *const RuntimeServices,
    boot_services: *const BootServices,
}

impl SystemTable {
    pub fn hdr(&self) -> &TableHeader {
        &self.hdr
    }

    pub fn stdin(&self) -> &SimpleTextInput {
        // SAFETY: This is safe because we mark ExitBootServices() as unsafe.
        unsafe { &*self.con_in }
    }

    pub fn stdout(&self) -> &SimpleTextOutput {
        // SAFETY: This is safe because we mark ExitBootServices() as unsafe.
        unsafe { &*self.con_out }
    }

    pub fn stderr(&self) -> &SimpleTextOutput {
        // SAFETY: This is safe because we mark ExitBootServices() as unsafe.
        unsafe { &*self.std_err }
    }

    pub fn boot_services(&self) -> &BootServices {
        // SAFETY: This is safe because we mark ExitBootServices() as unsafe.
        unsafe { &*self.boot_services }
    }
}
