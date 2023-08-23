use proc_macro::TokenStream;

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
