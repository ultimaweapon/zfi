use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use std::env::VarError;
use syn::{parse_quote, Error, ItemFn};

pub fn parse_qemu_attribute(mut item: ItemFn) -> Result<TokenStream, Error> {
    // Do nothing if we are not running by the integration test.
    if std::env::var("CARGO_TARGET_TMPDIR").is_err_and(|e| e == VarError::NotPresent) {
        // Keep the original function body.
        return Ok(item.into_token_stream());
    }

    // Get path to the project.
    let proj = match std::env::var("CARGO_MANIFEST_DIR") {
        Ok(v) => v,
        Err(e) => {
            return Err(match e {
                VarError::NotPresent => {
                    Error::new(Span::call_site(), "no CARGO_MANIFEST_DIR has been set")
                }
                VarError::NotUnicode(_) => {
                    Error::new(Span::call_site(), "non-unicode value in CARGO_MANIFEST_DIR")
                }
            });
        }
    };

    // Generate a test project.
    let name = item.sig.ident.to_string();
    let body = item.block.brace_token.span.join().source_text().unwrap();

    // Construct a new body.
    item.block = Box::new(parse_quote!({
        let proj = std::path::PathBuf::from(#proj);
        let dest = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
        let root = ::zfi_testing::gen_qemu_test(proj, dest, #name, #body);

        ::zfi_testing::run_qemu_test(root);
    }));

    Ok(item.into_token_stream())
}
