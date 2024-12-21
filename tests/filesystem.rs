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

#[test]
#[qemu]
fn create() {
    use zfi::{str, FileAttributes, Image};

    let image = Image::current().proto();
    let fs = image.device().file_system().unwrap();
    let root = fs.open().unwrap();

    // Create non-empty file to see if the second call truncate the file.
    let path = str!("\\test-file.txt");
    let mut file = root.create(path, FileAttributes::empty()).unwrap();
    let mut data = b"Hello, world!".to_vec();

    assert_eq!(file.write(&data).unwrap(), data.len());

    // Create the same file again to see if it is truncated.
    let mut file = root.create(path, FileAttributes::empty()).unwrap();

    assert_eq!(file.read(&mut data).unwrap(), 0);
}
