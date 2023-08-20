# ZFI – Zero-cost and safe interface to UEFI firmware

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

## Example Projects

- [TCG Boot](https://github.com/ultimaweapon/tcg-boot)

## Commercial Support

Please contact hello@ultima.inc if you need commercial support.

## License

MIT
