use self::string::parse_str;
use proc_macro::TokenStream;
use syn::{parse_macro_input, Error, LitStr};

mod string;

/// Define the entry of efi program, automatically import alloc crate and generate global_allocator
#[proc_macro_attribute]
pub fn main(_: TokenStream, main: TokenStream) -> TokenStream {
    let main = syn::parse_macro_input!(main as syn::ItemFn);
    let fn_name = &main.sig.ident;
    quote::quote! {
        extern crate alloc;
        #[global_allocator] static ALLOCATOR: ::zfi::PoolAllocator = ::zfi::PoolAllocator;
        #[no_mangle] extern "efiapi" fn efi_main(image: &'static ::zfi::Image, st: &'static ::zfi::SystemTable) -> ::zfi::Status {
            unsafe { ::zfi::init(image, st, Some(|| { ::alloc::boxed::Box::new(::zfi::DebugFile::next_to_image("log").unwrap()) })) };
            let start: fn(image: &'static ::zfi::Image, st: &'static ::zfi::SystemTable) -> ::zfi::Status = #fn_name;
            start(image, st)
        }
        #main
    }.into()
}

/// Construct an EFI string from a Rust string literal.
#[proc_macro]
pub fn str(arg: TokenStream) -> TokenStream {
    let arg = parse_macro_input!(arg as LitStr);

    parse_str(arg)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
