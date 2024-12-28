use zfi_testing::qemu;

#[test]
#[qemu]
fn proto() {
    use zfi::{current_image, str, PathBuf};

    let proto = current_image().proto();
    let mut path = PathBuf::new();

    path.push_media_file_path(if cfg!(target_arch = "x86_64") {
        str!(r"\EFI\BOOT\BOOTX64.EFI")
    } else if cfg!(target_arch = "x86") {
        str!(r"\EFI\BOOT\BOOTIA32.EFI")
    } else if cfg!(target_arch = "aarch64") {
        str!(r"\EFI\BOOT\BOOTAA64.EFI")
    } else {
        todo!("path for non-x86-64");
    });

    assert_eq!(proto.device().file_system().is_some(), true);
    assert_eq!(*proto.file_path(), path);
}
