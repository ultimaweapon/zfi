use proc_macro2::{Delimiter, Group, Span, TokenStream, TokenTree};
use quote::quote_spanned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env::VarError;
use std::fmt::Write;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use syn::Error;

pub fn parse_qemu_attribute(item: TokenStream) -> Result<TokenStream, Error> {
    // Get the function body.
    let mut output = Vec::new();
    let mut state = State::Start;
    let mut name = None;
    let mut body = None;

    for t in item {
        match &state {
            State::Start => match &t {
                TokenTree::Ident(i) if i == "fn" => state = State::Fn,
                _ => {}
            },
            State::Fn => match &t {
                TokenTree::Ident(i) => {
                    name = Some(i.to_string());
                    state = State::FnName;
                }
                _ => unreachable!(),
            },
            State::FnName => match &t {
                TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => {
                    state = State::Params
                }
                _ => {}
            },
            State::Params => match t {
                TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
                    body = Some(g);
                    break;
                }
                _ => {}
            },
        }

        output.push(t);
    }

    // Get path for the integration test data.
    let item = body.unwrap();
    let root = match std::env::var("CARGO_TARGET_TMPDIR") {
        Ok(v) => PathBuf::from(v),
        Err(e) => match e {
            VarError::NotPresent => {
                // We are not running by the integration test, keep the original function body.
                output.push(TokenTree::Group(item));

                return Ok(TokenStream::from_iter(output));
            }
            VarError::NotUnicode(_) => {
                return Err(Error::new(
                    Span::call_site(),
                    "non-unicode value in CARGO_TARGET_TMPDIR",
                ))
            }
        },
    };

    // Generate a test project.
    let name = name.unwrap();
    let span = item.span();

    generate_test(root.join("project"), &name, &span.source_text().unwrap())?;

    // Construct a new body.
    let root = root.to_str().unwrap();
    let body = Group::new(
        Delimiter::Brace,
        quote_spanned!(span=> ::zfi_testing::run_qemu_test(::std::path::Path::new(#root));),
    );

    output.push(TokenTree::Group(body));

    Ok(TokenStream::from_iter(output))
}

fn generate_test<P: AsRef<Path>>(dir: P, name: &str, body: &str) -> Result<(), Error> {
    // Create project directory.
    let dir = dir.as_ref();

    create_dir_all(dir).unwrap();

    // Get path to the project.
    let proj = match std::env::var("CARGO_MANIFEST_DIR") {
        Ok(v) => PathBuf::from(v),
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

    // Copy application dependencies.
    let cargo = std::fs::read_to_string(proj.join("Cargo.toml")).unwrap();
    let mut cargo = toml::from_str::<Cargo>(&cargo).unwrap();

    cargo.dev_dependencies.remove("zfi-testing");
    cargo.dependencies.extend(cargo.dev_dependencies.drain());

    // Fix relative path.
    for dep in cargo.dependencies.values_mut() {
        let path = match dep {
            Dependency::Complex { version: _, path } => match path {
                Some(v) => v,
                None => continue,
            },
            _ => continue,
        };

        if !Path::new(path.as_str()).is_relative() {
            continue;
        }

        *path = proj
            .join(path.as_str())
            .canonicalize()
            .unwrap()
            .into_os_string()
            .into_string()
            .unwrap();
    }

    // Check if the test being run is our test.
    if cargo.package.take().unwrap().name == "zfi" {
        cargo.dependencies.remove("zfi-macros");
        cargo.dependencies.insert(
            String::from("zfi"),
            Dependency::Complex {
                version: None,
                path: Some(proj.into_os_string().into_string().unwrap()),
            },
        );
    }

    // Create Cargo.toml.
    let mut data = String::new();

    writeln!(data, r#"[package]"#).unwrap();
    writeln!(data, r#"name = "{name}""#).unwrap();
    writeln!(data, r#"version = "0.1.0""#).unwrap();
    writeln!(data, r#"edition = "2021""#).unwrap();
    writeln!(data).unwrap();
    data.push_str(&toml::to_string_pretty(&cargo).unwrap());
    writeln!(data).unwrap();
    writeln!(data, r#"[workspace]"#).unwrap();
    writeln!(data, r#"members = []"#).unwrap();

    std::fs::write(dir.join("Cargo.toml"), data).unwrap();

    // Create src directory.
    let mut path = dir.join("src");

    create_dir_all(&path).unwrap();

    // Generate src/main.rs.
    let mut data = String::new();

    writeln!(data, r#"#![no_std]"#).unwrap();
    writeln!(data, r#"#![no_main]"#).unwrap();
    writeln!(data).unwrap();
    writeln!(data, r#"extern crate alloc;"#).unwrap();
    writeln!(data).unwrap();
    writeln!(data, r#"#[no_mangle]"#).unwrap();
    writeln!(data, r#"extern "efiapi" fn efi_main(image: &'static ::zfi::Image, st: &'static ::zfi::SystemTable) -> ::zfi::Status {{"#).unwrap();
    writeln!(data, r#"    unsafe {{ ::zfi::init(image, st, None) }};"#).unwrap();
    writeln!(data, r#"{}"#, &body[1..(body.len() - 1)]).unwrap();
    writeln!(data, r#"    ::zfi::println!("zfi:ok");"#).unwrap();
    writeln!(data, r#"    loop {{}}"#).unwrap();
    writeln!(data, r#"}}"#).unwrap();
    writeln!(data).unwrap();
    writeln!(data, r#"#[panic_handler]"#).unwrap();
    writeln!(
        data,
        r#"fn panic_handler(i: &::core::panic::PanicInfo) -> ! {{"#
    )
    .unwrap();
    writeln!(data, r#"    let l = i.location().unwrap();"#).unwrap();
    writeln!(data).unwrap();
    writeln!(
        data,
        r#"    ::zfi::println!("zfi:panic:{{}}:{{}}:{{}}", l.file(), l.line(), l.column());"#
    )
    .unwrap();
    writeln!(data).unwrap();
    writeln!(
        data,
        r#"    if let Some(&p) = i.payload().downcast_ref::<&str>() {{"#
    )
    .unwrap();
    writeln!(data, r#"        ::zfi::println!("{{p}}");"#).unwrap();
    writeln!(
        data,
        r#"    }} else if let Some(p) = i.payload().downcast_ref::<::alloc::string::String>() {{"#
    )
    .unwrap();
    writeln!(data, r#"        ::zfi::println!("{{p}}");"#).unwrap();
    writeln!(data, r#"    }} else {{"#).unwrap();
    writeln!(data, r#"        ::zfi::println!("{{i}}");"#).unwrap();
    writeln!(data, r#"    }}"#).unwrap();
    writeln!(data).unwrap();
    writeln!(data, r#"    ::zfi::println!("zfi:end");"#).unwrap();
    writeln!(data).unwrap();
    writeln!(data, r#"    loop {{}}"#).unwrap();
    writeln!(data, r#"}}"#).unwrap();
    writeln!(data).unwrap();
    writeln!(data, r#"#[global_allocator]"#).unwrap();
    writeln!(
        data,
        r#"static ALLOCATOR: ::zfi::PoolAllocator = ::zfi::PoolAllocator;"#
    )
    .unwrap();

    // Write src/main.rs.
    path.push("main.rs");

    std::fs::write(&path, data).unwrap();

    Ok(())
}

enum State {
    Start,
    Fn,
    FnName,
    Params,
}

#[derive(Serialize, Deserialize)]
struct Cargo {
    package: Option<Package>,
    dependencies: HashMap<String, Dependency>,

    #[serde(rename = "dev-dependencies")]
    dev_dependencies: HashMap<String, Dependency>,
}

#[derive(Serialize, Deserialize)]
struct Package {
    name: String,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum Dependency {
    Simple(String),
    Complex {
        version: Option<String>,
        path: Option<String>,
    },
}
