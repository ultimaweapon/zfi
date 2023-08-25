use proc_macro2::TokenStream;
use quote::quote_spanned;
use syn::{Error, LitStr};

pub fn parse_str(arg: LitStr) -> Result<TokenStream, Error> {
    let span = arg.span();
    let input = arg.value();
    let mut output: Vec<u16> = Vec::with_capacity(input.len() + 1);

    for c in input.chars() {
        match c {
            '\0' => return Err(Error::new(span, "expect a string with non-NUL character")),
            '\u{10000}'.. => {
                return Err(Error::new(
                    span,
                    "the string contains unsupported character",
                ));
            }
            c => output.push(c.encode_utf16(&mut [0; 1])[0]),
        }
    }

    output.push(0);

    Ok(quote_spanned!(span=> unsafe { ::zfi::EfiStr::new_unchecked(&[#(#output),*]) }))
}
