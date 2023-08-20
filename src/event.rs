/// Represents an `EFI_EVENT`.
///
/// The reason this type is not exposed is because it is likely to be changing in the future.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub(crate) struct Event(usize);
