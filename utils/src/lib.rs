use std::{path::PathBuf, fs};

use uuid::Uuid;

pub struct TempDir {
    pub root: PathBuf
}

impl TempDir {
    fn new(parent: &PathBuf) -> TempDir {
        let dir_name = Uuid::new_v4().to_string();
        let temp_dir = parent.join(&dir_name);
        fs::create_dir_all(&temp_dir).unwrap();
        TempDir {
            root: PathBuf::from(temp_dir)
        }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

pub fn testbed() -> TempDir {
    TempDir::new(&PathBuf::from("__TEST__"))
}