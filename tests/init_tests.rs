use std::{fs, path::PathBuf};
use uuid::Uuid;

use grit::{GlobalOpts, cmd_init};


struct TempDir {
    root: PathBuf
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

fn temp_dir() -> TempDir {
    TempDir::new(&PathBuf::from("__TEST__"))
}


fn repo_created(root: &PathBuf) -> bool {
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
    ].into_iter().map(|s| root.join(s)).collect();

    for path in expected_paths {
        if !path.exists() {
            return false;
        }
    }

    // refs/heads should initially be empty
    let heads_dir = root.join(".grit/refs/heads");
    let heads = fs::read_dir(heads_dir).unwrap();
    return heads.into_iter().count() == 0;
}


#[test]
fn creates_repo_in_provided_path() {
    let tempdir = temp_dir();

    let global_opts = GlobalOpts {
        git_mode: false
    };

    cmd_init(Some(tempdir.root.to_string_lossy().into_owned()), global_opts)
        .unwrap_or_else(|e| println!("{}", e));

    assert!(repo_created(&tempdir.root));
}
