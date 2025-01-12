use crate::event::Event;
use crate::{
    current_image, Device, Guid, Image, Pages, Path, Status, TableHeader, IMAGE, PAGE_SIZE,
};
use alloc::vec::Vec;
use bitflags::bitflags;
use core::mem::size_of;
use core::ptr::{null, null_mut};

/// Represents an `EFI_BOOT_SERVICES`.
#[repr(C)]
pub struct BootServices {
    hdr: TableHeader,
    raise_tpl: fn(),
    restore_tpl: fn(),
    allocate_pages: unsafe extern "efiapi" fn(AllocateType, MemoryType, usize, *mut u64) -> Status,
    free_pages: unsafe extern "efiapi" fn(u64, usize) -> Status,
    get_memory_map: unsafe extern "efiapi" fn(
        *mut usize,
        *mut MemoryDescriptor,
        *mut usize,
        *mut usize,
        *mut u32,
    ) -> Status,
    allocate_pool: unsafe extern "efiapi" fn(MemoryType, usize, *mut *mut u8) -> Status,
    free_pool: unsafe extern "efiapi" fn(*mut u8) -> Status,
    create_event: fn(),
    set_timer: fn(),
    wait_for_event: unsafe extern "efiapi" fn(usize, *const Event, *mut usize) -> Status,
    signal_event: fn(),
    close_event: fn(),
    check_event: fn(),
    install_protocol_interface: fn(),
    reinstall_protocol_interface: fn(),
    uninstall_protocol_interface: fn(),
    handle_protocol: fn(),
    reserved: usize,
    register_protocol_notify: fn(),
    locate_handle: fn(),
    locate_device_path:
        unsafe extern "efiapi" fn(*const Guid, *mut *const u8, *mut *const ()) -> Status,
    install_configuration_table: fn(),
    load_image: fn(),
    start_image: fn(),
    exit: fn(),
    unload_image: fn(),
    exit_boot_services: extern "efiapi" fn(&Image, usize) -> Status,
    get_next_monotonic_count: fn(),
    stall: fn(),
    set_watchdog_timer: fn(),
    connect_controller: fn(),
    disconnect_controller: fn(),
    open_protocol: unsafe extern "efiapi" fn(
        *const (),
        *const Guid,
        *mut *const (),
        *const (),
        *const (),
        OpenProtocolAttributes,
    ) -> Status,
}

impl BootServices {
    /// Allocates memory pages from the system.
    pub fn allocate_pages(
        &self,
        at: AllocateType,
        mt: MemoryType,
        pages: usize,
        mut addr: u64,
    ) -> Result<Pages, Status> {
        let status = unsafe { (self.allocate_pages)(at, mt, pages, &mut addr) };

        if status != Status::SUCCESS {
            Err(status)
        } else {
            Ok(unsafe { Pages::new(addr as _, pages * PAGE_SIZE) })
        }
    }

    /// # Safety
    /// `base` must be allocated with [`Self::allocate_pages()`].
    pub unsafe fn free_pages(&self, base: *mut u8, pages: usize) -> Result<(), Status> {
        let status = (self.free_pages)(base as _, pages);

        if status != Status::SUCCESS {
            Err(status)
        } else {
            Ok(())
        }
    }

    /// Returns the current memory map. A common mistake when using this method to get a key to
    /// invoke [`Self::exit_boot_services()`] is discarding the result, which will cause the vector
    /// to drop and memory map will be changed.
    pub fn get_memory_map(&self) -> Result<(Vec<MemoryDescriptor>, usize), Status> {
        let mut len = 1;

        loop {
            let mut size = len * size_of::<MemoryDescriptor>();
            let mut map: Vec<MemoryDescriptor> = Vec::with_capacity(len);
            let mut key = 0;
            let mut dsize = 0;
            let mut dver = 0;
            let status = unsafe {
                (self.get_memory_map)(
                    &mut size,
                    map.spare_capacity_mut().as_mut_ptr() as _,
                    &mut key,
                    &mut dsize,
                    &mut dver,
                )
            };

            len = size / size_of::<MemoryDescriptor>();

            match status {
                Status::SUCCESS => {
                    assert_eq!(dsize, size_of::<MemoryDescriptor>());
                    assert_eq!(dver, 1);

                    unsafe { map.set_len(len) };

                    break Ok((map, key));
                }
                Status::BUFFER_TOO_SMALL => continue,
                v => break Err(v),
            }
        }
    }

    /// All allocations are eight-byte aligned.
    pub fn allocate_pool(&self, ty: MemoryType, size: usize) -> Result<*mut u8, Status> {
        let mut mem = null_mut();
        let status = unsafe { (self.allocate_pool)(ty, size, &mut mem) };

        if status != Status::SUCCESS {
            Err(status)
        } else {
            Ok(mem)
        }
    }

    /// # Safety
    /// `mem` must be allocated by [`Self::allocate_pool()`].
    pub unsafe fn free_pool(&self, mem: *mut u8) -> Result<(), Status> {
        (self.free_pool)(mem).err_or(())
    }

    /// Stops execution until an event is signaled.
    pub(crate) fn wait_for_event(&self, events: &[Event]) -> Result<usize, Status> {
        let mut index = 0;
        let status = unsafe { (self.wait_for_event)(events.len(), events.as_ptr(), &mut index) };

        if status != Status::SUCCESS {
            Err(status)
        } else {
            Ok(index)
        }
    }

    /// Locates the handle to a device on the device path that supports the specified protocol.
    pub fn locate_device_path<'a>(
        &self,
        proto: &Guid,
        path: &'a Path,
    ) -> Result<(&'static Device, &'a Path), Status> {
        let mut path = path.as_bytes().as_ptr();
        let mut device = null();
        let status = unsafe { (self.locate_device_path)(proto, &mut path, &mut device) };

        if status != Status::SUCCESS {
            Err(status)
        } else {
            Ok(unsafe { (&*(device as *const Device), Path::from_ptr(path)) })
        }
    }

    /// Terminates all boot services.
    ///
    /// # Safety
    /// Once this method is returned any functions provided by ZFI will not be usable. Beware of any
    /// functions that are automatically called by Rust (e.g. when the value is dropped)! Usually
    /// this method will be called right before transfering the control to the OS kernel.
    pub unsafe fn exit_boot_services(&self, map_key: usize) -> Result<(), Status> {
        let status = (self.exit_boot_services)(current_image(), map_key);

        if status != Status::SUCCESS {
            Err(status)
        } else {
            Ok(())
        }
    }

    /// # Safety
    /// This method don't check anything so the caller is responsible to make sure all arguments is
    /// valid for `EFI_BOOT_SERVICES.OpenProtocol()`.
    pub unsafe fn get_protocol(&self, handle: *const (), proto: &Guid) -> Option<*const ()> {
        let agent = IMAGE.cast();
        let attrs = OpenProtocolAttributes::GET_PROTOCOL;

        match self.open_protocol(handle, proto, agent, null(), attrs) {
            Ok(v) => Some(v),
            Err(e) => {
                assert_eq!(e, Status::UNSUPPORTED);
                None
            }
        }
    }

    /// # Safety
    /// This method don't check anything so the caller is responsible to make sure all arguments is
    /// valid for `EFI_BOOT_SERVICES.OpenProtocol()`.
    pub unsafe fn open_protocol(
        &self,
        handle: *const (),
        proto: &Guid,
        agent: *const (),
        controller: *const (),
        attrs: OpenProtocolAttributes,
    ) -> Result<*const (), Status> {
        let mut interface = null();
        let status = (self.open_protocol)(handle, proto, &mut interface, agent, controller, attrs);

        if status != Status::SUCCESS {
            Err(status)
        } else {
            Ok(interface)
        }
    }
}

/// Represents an `EFI_ALLOCATE_TYPE`.
#[repr(C)]
pub enum AllocateType {
    AnyPages,
    MaxAddress,
    Address,
}

/// Represents an `EFI_MEMORY_TYPE`.
#[repr(C)]
pub enum MemoryType {
    /// Not usable.
    Reserved,

    /// The code portions of a loaded UEFI application.
    LoaderCode,

    /// The data portions of a loaded UEFI application and the default data allocation type used by
    /// a UEFI application to allocate pool memory.
    LoaderData,
    BootServicesCode,
    BootServicesData,
    RuntimeServicesCode,
    RuntimeServicesData,
    ConventionalMemory,
    UnusableMemory,
    AcpiReclaimMemory,
    AcpiMemoryNvs,
    MemoryMappedIo,
    MemoryMappedIoPortSpace,
    PalCode,
    PersistentMemory,
    Unaccepted,
}

/// Represents an `EFI_MEMORY_DESCRIPTOR`.
#[repr(C)]
pub struct MemoryDescriptor {
    ty: u32,
    physical_start: u64,
    virtual_start: u64,
    number_of_pages: u64,
    attribute: u64,
}

bitflags! {
    /// Attributes of [`BootServices::open_protocol()`].
    #[repr(transparent)]
    pub struct OpenProtocolAttributes: u32 {
        const GET_PROTOCOL = 0x00000002;
    }
}
