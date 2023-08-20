use crate::{Guid, OpenProtocolAttributes, Status, SystemTable};

/// # Safety
/// This function don't check anything so the caller is responsible to make sure all arguments is
/// valid for `EFI_BOOT_SERVICES.OpenProtocol()`.
pub unsafe fn get_protocol(handle: *const (), proto: &Guid) -> Option<*const ()> {
    SystemTable::current()
        .boot_services()
        .get_protocol(handle, proto)
}

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
    SystemTable::current()
        .boot_services()
        .open_protocol(handle, proto, agent, controller, attrs)
}
