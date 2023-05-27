use lightning_persister::FilesystemPersister; // import LDK sample persist module

pub fn persister(ldk_data_dir_path: &str) -> FilesystemPersister {
    FilesystemPersister::new(ldk_data_dir_path.to_string())
}
