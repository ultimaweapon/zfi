use crate::{
    EfiChar, EfiString, File, FileAttributes, FileCreateError, Image, Owned, Path, PathNode,
    Status, DEBUG_WRITER,
};
use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use core::cell::RefCell;
use core::fmt::{Display, Formatter, Write};

/// Prints to the debug log, with a newline.
///
/// This macro will do nothing if no debug writer specified during ZFI initialization. See
/// [`debug_writer()`] for more information.
#[macro_export]
macro_rules! debugln {
    ($($args:tt)*) => {
        if let Some(w) = $crate::debug_writer() {
            let mut w = w.borrow_mut();

            w.write_fmt(::core::format_args!($($args)*)).unwrap();
            w.write_char('\n').unwrap();
        }
    };
}

/// Gets the debug writer that was specified as an argument of [`crate::init()`].
///
/// If you are using [`crate::main`] macro no debug writer is enable by default. See [`crate::main`]
/// for how to enable the debug writer.
pub fn debug_writer() -> Option<&'static RefCell<Box<dyn Write>>> {
    // SAFETY: This is safe because the only place that write DEBUG_WRITER is our init function.
    #[allow(static_mut_refs)]
    unsafe {
        DEBUG_WRITER.as_ref()
    }
}

/// A debug writer that write the debug log to a text file.
pub struct DebugFile {
    file: Owned<File>,
}

impl DebugFile {
    /// `ext` is a file extension without leading dot.
    pub fn next_to_image(ext: &str) -> Result<Self, DebugFileError> {
        // Get FS on the device where the image is located.
        let im = Image::current().proto();
        let fs = match im.device().file_system() {
            Some(v) => v,
            None => return Err(DebugFileError::UnsupportedImageLocation),
        };

        // Open the root of volume.
        let root = match fs.open() {
            Ok(v) => v,
            Err(e) => return Err(DebugFileError::OpenRootFailed(im.file_path(), e)),
        };

        // Build file path.
        let mut path = match im.file_path().read() {
            PathNode::MediaFilePath(v) => v.to_owned(),
        };

        path.push(EfiChar::FULL_STOP);

        if path.push_str(ext).is_err() {
            return Err(DebugFileError::UnsupportedExtension);
        }

        // Create the file.
        let file = match root.create(&path, FileAttributes::empty()) {
            Ok(v) => v,
            Err(e) => return Err(DebugFileError::CreateFileFailed(path, e)),
        };

        Ok(Self { file })
    }
}

impl Write for DebugFile {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.file
            .write(s.as_bytes())
            .and_then(|_| self.file.flush())
            .map_err(|_| core::fmt::Error)
    }
}

/// Represents an error when [`DebugFile`] constructing is failed.
#[derive(Debug)]
pub enum DebugFileError {
    UnsupportedImageLocation,
    OpenRootFailed(&'static Path, Status),
    UnsupportedExtension,
    CreateFileFailed(EfiString, FileCreateError),
}

impl Display for DebugFileError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnsupportedImageLocation => {
                f.write_str("the location of the current image is not supported")
            }
            Self::OpenRootFailed(p, e) => write!(f, "cannot open the root directory of {p} -> {e}"),
            Self::UnsupportedExtension => {
                f.write_str("file extension contains unsupported character")
            }
            Self::CreateFileFailed(p, e) => write!(f, "cannot create {p} -> {e}"),
        }
    }
}
