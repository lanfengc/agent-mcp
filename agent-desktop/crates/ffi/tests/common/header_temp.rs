use std::path::PathBuf;

pub struct HeaderTemp(pub PathBuf);

impl HeaderTemp {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(agent_desktop_core::temporary_file_name(
            std::ffi::OsStr::new("agent-desktop-header"),
        ));
        std::fs::DirBuilder::new()
            .create(&path)
            .expect("create private compiler directory");
        Self(path)
    }
}

impl Drop for HeaderTemp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
