use std::{path::{PathBuf, Path}, env, fs};

use configparser::ini::Ini;

use crate::{GlobalOpts, CmdError, git_dir_name, program_name};


pub fn cmd_init(path: Option<String>, global_opts: GlobalOpts) -> Result<(), CmdError> {
    let git_dir_name = git_dir_name(&global_opts);

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

    let gitdir = root.join(git_dir_name); 
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

    // Add trailing slash if a directory name to match Git
    let mut gitdir_str: String = gitdir.to_string_lossy().into();
    if gitdir.is_dir() {
        gitdir_str.push('/');
    }

    println!("Initialized empty {} repository in {}", program_name(&global_opts), gitdir_str);
    eprintln!("hint: Using 'master' as the name for the initial branch. This default branch name");
    eprintln!("hint: is subject to change. To configure the initial branch name to use in all");
    eprintln!("hint: of your new repositories, which will suppress this warning, call:");
    eprintln!("hint: ");
    eprintln!("hint: \tgit config --global init.defaultBranch <name>");
    eprintln!("hint: ");
    eprintln!("hint: Names commonly chosen instead of 'master' are 'main', 'trunk' and");
    eprintln!("hint: 'development'. The just-created branch can be renamed via this command:");
    eprintln!("hint: ");
    eprintln!("hint: \tgit branch -m <name>");
    Ok(())
}

fn repo_default_config() -> Ini {
    let mut config = Ini::new();
    config.set("core", "repositoryformatversion", Some(String::from("0")));
    config.set("core", "filemode", Some(String::from("false")));
    config.set("core", "bare", Some(String::from("false")));

    config
}
