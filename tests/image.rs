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
