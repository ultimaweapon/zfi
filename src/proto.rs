use crate::{system_table, Guid, OpenProtocolAttributes, Status};

/// Invokes `EFI_BOOT_SERVICES.OpenProtocol`.
///
/// # Safety
/// This function don't check anything so the caller is responsible to make sure all arguments is
/// valid for `EFI_BOOT_SERVICES.OpenProtocol()`.
pub unsafe fn get_protocol(handle: *const (), proto: &Guid) -> Option<*const ()> {
    system_table().boot_services().get_protocol(handle, proto)
}

/// Invokes `EFI_BOOT_SERVICES.OpenProtocol`.
///
/// # Safety
/// This method don't check anything so the caller is responsible to make sure all arguments is
/// valid for `EFI_BOOT_SERVICES.OpenProtocol()`.
pub unsafe fn open_protocol(
    handle: *const (),
    proto: &Guid,
    agent: *const (),
    controller: *const (),
    attrs: OpenProtocolAttributes,
) -> Result<*const (), Status> {
    system_table()
        .boot_services()
        .open_protocol(handle, proto, agent, controller, attrs)
}
