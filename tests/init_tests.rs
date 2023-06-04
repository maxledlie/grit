use std::{fs, path::{PathBuf}};

use grit::{GlobalOpts, cmd_init};


struct TempDir {
    root: PathBuf
}

impl TempDir {
    fn new(root: &String) -> TempDir {
        fs::create_dir_all(&root).unwrap();
        TempDir {
            root: PathBuf::from(root)
        }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

fn temp_dir() -> TempDir {
    TempDir::new(&String::from("test_repo"))
}


#[test]
fn init_creates_git_structure() {
    let tempdir = temp_dir();

    let global_opts = GlobalOpts {
        git_mode: false
    };

    cmd_init(Some(tempdir.root.to_string_lossy().into_owned()), global_opts)
        .unwrap_or_else(|e| println!("{}", e));

    // A .grit folder should have been created with the default structure
    let expected_paths: Vec<PathBuf> = vec![
        ".grit/HEAD",
        ".grit/branches",
        ".grit/config",
        ".grit/hooks",
        ".grit/index",
        ".grit/info",
        ".grit/logs",
        ".grit/objects",
        ".grit/refs/heads",
        ".grit/refs/tags"
    ].into_iter().map(|s| tempdir.root.join(s)).collect();

    for path in expected_paths {
        println!("{}", path.to_string_lossy());
        assert!(path.exists());
    }

    // A branch named "master" should exist

}
