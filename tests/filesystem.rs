use zfi_testing::qemu;

#[test]
#[qemu]
fn file_info() {
    use zfi::{FileAttributes, FileModes, Image};

    let image = Image::current().proto();
    let fs = image.device().file_system().unwrap();
    let root = fs.open().unwrap();
    let file = root
        .open(
            image.file_path().to_media_file_path().unwrap(),
            FileModes::READ,
            FileAttributes::empty(),
        )
        .unwrap();
    let info = file.info().unwrap();

    assert_ne!(info.file_size(), 0);
    assert_ne!(info.physical_size(), 0);
    assert_eq!(info.attributes().contains(FileAttributes::DIRECTORY), false);
}
