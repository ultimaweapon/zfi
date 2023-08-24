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
[qemu.x86_64-unknown-uefi]
bin = "qemu-system-x86_64"
firmware = "/usr/share/edk2/x64/OVMF_CODE.fd"
nvram = "/usr/share/edk2/x64/OVMF_VARS.fd"
```

## Example Projects

- [TCG Boot](https://github.com/ultimaweapon/tcg-boot)

## Commercial Support

Please contact hello@ultima.inc if you need commercial support.

## License

MIT
