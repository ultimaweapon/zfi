# ZFI – Zero-cost and safe interface to UEFI firmware
[![CI](https://github.com/ultimicro/zfi/actions/workflows/ci.yml/badge.svg)](https://github.com/ultimicro/zfi/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/zfi)](https://crates.io/crates/zfi)

ZFI is a Rust crate for writing a UEFI application with the following goals:

- Provides base APIs that are almost identical to the UEFI specifications.
- Provides additional APIs that build on top of the base APIs.
- Base APIs are zero-cost abstraction over UEFI API.
- Safe and easy to use.
- Work on stable Rust.

ZFI supports only single-thread environment, which is the same as UEFI specifications.

## Example

```rust
#![no_std]
#![no_main]

use alloc::boxed::Box;
use zfi::{pause, println, DebugFile, Image, Status, SystemTable};

extern crate alloc;

#[no_mangle]
extern "efiapi" fn efi_main(image: &'static Image, st: &'static SystemTable) -> Status {
    // This is the only place you need to use unsafe. This must be done immediately after landing
    // here.
    unsafe {
        zfi::init(
            image,
            st,
            Some(|| Box::new(DebugFile::next_to_image("log").unwrap())),
        )
    };

    // Any EFI_HANDLE will be represents by a reference to a Rust type (e.g. image here is a type of
    // Image). Each type that represents EFI_HANDLE provides the methods to access any protocols it
    // is capable for (e.g. you can do image.proto() here to get an EFI_LOADED_IMAGE_PROTOCOL from
    // it). You can download the UEFI specifications for free here: https://uefi.org/specifications
    println!("Hello, world!");
    pause();

    Status::SUCCESS
}

#[cfg(not(test))]
#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    zfi::eprintln!("{info}");
    loop {}
}

#[cfg(not(test))]
#[global_allocator]
static ALLOCATOR: zfi::PoolAllocator = zfi:PoolAllocator;
```

You can use `zfi::main` macro if you prefer a less boilerplate:

```rust
#![no_std]
#![no_main]

use zfi::{pause, println, Status};

// zfi::main will not enable the debug writer by default. See its documentation to see how to enable
// the debug writer.
#[zfi::main]
fn main() -> Status {
    // Use Image::current() to get the image handle.
    println!("Hello, world!");
    pause();

    Status::SUCCESS
}
```

To build the above example you need to add a UEFI target to Rust:

```sh
rustup target add x86_64-unknown-uefi
```

Then build with the following command:

```sh
cargo build --target x86_64-unknown-uefi
```

You can grab the EFI file in `target/x86_64-unknown-uefi/debug` and boot it on a compatible machine.

## Integration Testing

ZFI provide [zfi-testing](https://crates.io/crates/zfi-testing) crate to help you write the
[integration tests](https://doc.rust-lang.org/rust-by-example/testing/integration_testing.html).
This crate must be added as a
[development dependency](https://doc.rust-lang.org/rust-by-example/testing/dev_dependencies.html),
not a standard dependency. You need to install the following tools before you can run the
integration tests that use `zfi-testing`:

- [QEMU](https://www.qemu.org)
- [OVMF](https://github.com/tianocore/tianocore.github.io/wiki/OVMF)

Once ready create `zfi.toml` in the root of your package (the same location as `Cargo.toml`) with
the following content:

```toml
[qemu.RUST_TARGET]
bin = "QEMU_BIN"
firmware = "OVMF_CODE"
nvram = "OVMF_VARS"
```

This file should not commit to the version control because it is specific to your machine. Replace
the following placeholders with the appropriate value:

- `RUST_TARGET`: name of Rust target you want to run on the QEMU (e.g. `x86_64-unknown-uefi`).
- `QEMU_BIN`: path to the QEMU binary to run your tests. The binary must have the same CPU type as
  `RUST_TARGET`. You don't need to specify a full path if the binary can be found in the `PATH`
  environment variable (e.g. `qemu-system-x86_64`).
- `OVMF_CODE`: path to `OVMF_CODE.fd` from OVMF. File must have the same CPU type as `RUST_TARGET`
  (e.g. `/usr/share/edk2/x64/OVMF_CODE.fd`).
- `OVMF_VARS`: path to `OVMF_VARS.fd` from OVMF. File must have the same CPU type as `RUST_TARGET`
  (e.g. `/usr/share/edk2/x64/OVMF_VARS.fd`).

Example:

```toml
[qemu.aarch64-unknown-uefi]
bin = "qemu-system-aarch64"
firmware = "/usr/share/AAVMF/AAVMF_CODE.fd"
nvram = "/usr/share/AAVMF/AAVMF_VARS.fd"

[qemu.i686-unknown-uefi]
bin = "qemu-system-i386"
firmware = "/usr/share/edk2/ia32/OVMF_CODE.fd"
nvram = "/usr/share/edk2/ia32/OVMF_VARS.fd"

[qemu.x86_64-unknown-uefi]
bin = "qemu-system-x86_64"
firmware = "/usr/share/edk2/x64/OVMF_CODE.fd"
nvram = "/usr/share/edk2/x64/OVMF_VARS.fd"
```

### Writing Tests

To write an integration test to run on QEMU, put `zfi_testing::qemu` attribute to your integration
test:

```rust
use zfi_testing::qemu;

#[test]
#[qemu]
fn proto() {
    use zfi::{str, Image, PathBuf};

    let proto = Image::current().proto();
    let mut path = PathBuf::new();

    if cfg!(target_arch = "x86_64") {
        path.push_media_file_path(str!(r"\EFI\BOOT\BOOTX64.EFI"));
    } else {
        todo!("path for non-x86-64");
    }

    assert_eq!(proto.device().file_system().is_some(), true);
    assert_eq!(*proto.file_path(), path);
}
```

The code in the function that has `zfi_testing::qemu` attribute will run on the QEMU. This test can
be run in the same way as normal integration tests:

```sh
cargo test
```

Keep in mind that you need to put everything your test needed in the same function because what
`qemu` attribute does is moving your function body into `efi_main` and run it on QEMU.

### Known Issues

- Any panic (including assertion failed) in your integration test will be show as `src/main.rs:L:C`.
  This is a limitation on stable Rust for [now](https://github.com/rust-lang/rust/issues/54725).
- rust-analyzer not report any syntax error. The reason is because `qemu` attribute replace the
  whole function body, which mean what rust-analyzer see when running syntax check is the replaced
  function, not the origial function. Right now there is no way to check if our proc macro being run
  by rust-analyzer until this [issue](https://github.com/rust-lang/rust-analyzer/issues/13731) has
  been resolved.

## Breaking Changes

### 0.1 to 0.2

- `Path` is changed from sized type to unsized type. Any code that cast `Path` to a raw pointer need
  to update otherwise you will got a fat pointer, which is Rust specific. You can get a pointer to
  `EFI_DEVICE_PATH_PROTOCOL` via `Path::as_bytes()`.
- `FileInfo` is changed from sized type to unsized type in the same way as `Path`.
- `File::info()` now return `Box<FileInfo>` instead of `Owned<FileInfo>` when success.
- The second parameter of `Owned::new()` is changed to `Dtor`.

## Example Projects

- [TCG Boot](https://github.com/ultimaweapon/tcg-boot)

## License

MIT
