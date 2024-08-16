use self::qemu::parse_qemu_attribute;
use proc_macro::TokenStream;
use syn::{parse_macro_input, Error, ItemFn};

mod qemu;

/// Attribute macro applied to a function to run it on QEMU.
///
/// This attribute will move the function body into `efi_main` to run it on QEMU. Which mean you
/// must put everything that are needed within this function.
#[proc_macro_attribute]
pub fn qemu(_: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemFn);

    parse_qemu_attribute(item)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
