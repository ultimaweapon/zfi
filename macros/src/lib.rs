use self::entry::parse_entry;
use self::string::parse_str;
use crate::entry::{EntryDebug, EntryOptions};
use proc_macro::TokenStream;
use syn::{parse_macro_input, Error, ItemFn, LitStr};

mod entry;
mod string;

/// Define the entry of EFI program, automatically import `alloc` crate, generate `global_allocator`
/// and `panic_handler`.
///
/// This macro will not enable the debug writer by default. To enable the debug writer, specify one
/// of the following options:
///
/// - `debug_extension`: A string literal that specify the extension of the log file that will be
///   created next to the application image. If you specify `#[zfi::main(debug_extension = "log")]`,
///   ZFI will create a file `PATH\TO\YOUR\APP.EFI.log`.
/// - `debug_writer`: A function identifier with zero parameter that return
///   `alloc::boxed::Box<dyn core::fmt::Write>`.
///
/// Other available options:
///
/// - `no_ph`: Do not generate `panic_handler`.
#[proc_macro_attribute]
pub fn main(arg: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemFn);
    let mut options = EntryOptions::default();
    let parser = syn::meta::parser(|m| {
        if m.path.is_ident("debug_extension") {
            options.debug = Some(EntryDebug::Extension(m.value()?.parse()?));
        } else if m.path.is_ident("debug_writer") {
            options.debug = Some(EntryDebug::Writer(m.value()?.parse()?));
        } else if m.path.is_ident("no_ph") {
            options.no_ph = true;
        } else {
            return Err(m.error("unknown option"));
        }

        Ok(())
    });

    parse_macro_input!(arg with parser);

    parse_entry(item, options)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

/// Construct an EFI string from a Rust string literal.
#[proc_macro]
pub fn str(arg: TokenStream) -> TokenStream {
    let arg = parse_macro_input!(arg as LitStr);

    parse_str(arg)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
