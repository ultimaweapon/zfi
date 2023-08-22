/// Declare program entry
#[macro_export]
macro_rules! entry {
    ($func:ident) => {
        extern crate alloc;

        #[global_allocator]
        static ALLOCATOR: $crate::PoolAllocator = $crate::PoolAllocator;

        #[no_mangle]
        extern "efiapi" fn efi_main(
            image: &'static $crate::Image,
            st: &'static $crate::SystemTable,
        ) -> $crate::Status {
            unsafe {
                $crate::init(
                    image,
                    st,
                    Some(|| {
                        ::alloc::boxed::Box::new($crate::DebugFile::next_to_image("log").unwrap())
                    }),
                )
            };
            let main: fn(
                image: &'static $crate::Image,
                st: &'static $crate::SystemTable,
            ) -> $crate::Status = $func;
            main(image, st)
        }
    };
}
