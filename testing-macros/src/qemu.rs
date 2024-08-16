use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env::VarError;
use std::fmt::Write;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use syn::{parse_quote, Error, ItemFn};

pub fn parse_qemu_attribute(mut item: ItemFn) -> Result<TokenStream, Error> {
    // Get path for the integration test data.
    let root = match std::env::var("CARGO_TARGET_TMPDIR") {
        Ok(v) => PathBuf::from(v),
        Err(e) => match e {
            VarError::NotPresent => {
                // We are not running by the integration test, keep the original function body.
                return Ok(item.into_token_stream());
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
    let name = item.sig.ident.to_string();
    let body = item.block.brace_token.span.join().source_text().unwrap();

    generate_test(root.join("project"), &name, &body)?;

    // Construct a new body.
    let root = root.to_str().unwrap();

    item.block = Box::new(parse_quote!({
        ::zfi_testing::run_qemu_test(::std::path::Path::new(#root));
    }));

    Ok(item.into_token_stream())
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
            Dependency::Complex {
                version: _,
                path: Some(v),
            } => v,
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
    writeln!(data, r#"#[::zfi::main(no_ph)]"#).unwrap();
    writeln!(data, r#"fn main() -> ::zfi::Status {{"#).unwrap();
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

    // Write src/main.rs.
    path.push("main.rs");

    std::fs::write(&path, data).unwrap();

    Ok(())
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
