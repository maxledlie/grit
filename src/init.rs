use std::{path::{PathBuf, Path}, env, fs};

use configparser::ini::Ini;

use crate::{GlobalOpts, CmdError};


pub fn cmd_init(path: Option<String>, _global_opts: GlobalOpts) -> Result<(), CmdError> {
    let git_dirs: Vec<PathBuf> = vec![
        "branches",
        "hooks",
        "index",
        "info",
        "logs",
        "objects",
        "refs/heads",
        "refs/tags"
    ].into_iter().map(|s| PathBuf::from(s)).collect();

    let root = path
        .map(|p| Path::new(&p).to_path_buf())
        .unwrap_or(env::current_dir().unwrap());

    let gitdir = root.join(".grit"); 
    for p in git_dirs {
        let path = gitdir.join(&p);
        fs::create_dir_all(&path).map_err(CmdError::IOError)?;
    }

    // Create a default config file
    let config = repo_default_config();
    let config_path = gitdir.join("config");
    config.write(config_path).map_err(CmdError::IOError)?;

    // Create a HEAD file pointing to the master branch
    let head_path = gitdir.join("HEAD");
    let head_contents = "ref: refs/heads/master";
    fs::write(head_path, head_contents).map_err(CmdError::IOError)?;

    println!("Initialized empty Grit repository in {}", gitdir.to_string_lossy());
    Ok(())
}

fn repo_default_config() -> Ini {
    let mut config = Ini::new();
    config.set("core", "repositoryformatversion", Some(String::from("0")));
    config.set("core", "filemode", Some(String::from("false")));
    config.set("core", "bare", Some(String::from("false")));

    config
}
