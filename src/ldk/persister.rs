use lightning_persister::FilesystemPersister; // import LDK sample persist module

pub fn persister(ldk_data_dir_path: String) -> FilesystemPersister {
    FilesystemPersister::new(ldk_data_dir_path)
}
