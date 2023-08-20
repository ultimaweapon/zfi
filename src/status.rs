use core::fmt::{Display, Formatter};

/// Represents an `EFI_STATUS`.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct Status(usize);

impl Status {
    pub const SUCCESS: Self = Self(0);
    pub const UNSUPPORTED: Self = Self::error(3);
    pub const BUFFER_TOO_SMALL: Self = Self::error(5);
    pub const ABORTED: Self = Self::error(21);

    #[cfg(target_pointer_width = "32")]
    const fn error(v: usize) -> Self {
        Self(0x80000000 | v)
    }

    #[cfg(target_pointer_width = "64")]
    const fn error(v: usize) -> Self {
        Self(0x8000000000000000 | v)
    }

    pub fn err_or<T>(self, success: T) -> Result<T, Self> {
        if self == Self::SUCCESS {
            Ok(success)
        } else {
            Err(self)
        }
    }

    pub fn is_success(self) -> bool {
        self == Self::SUCCESS
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::SUCCESS => f.write_str("the operation completed successfully"),
            Self::UNSUPPORTED => f.write_str("the operation is not supported"),
            Self::BUFFER_TOO_SMALL => f.write_str("the buffer is not large enough"),
            Self::ABORTED => f.write_str("the operation was aborted"),
            v => write!(f, "{:#x}", v.0),
        }
    }
}
