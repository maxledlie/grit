use std::{env, fs, path::PathBuf};

use anyhow::Result;
use clap::Args;

use crate::{GlobalOpts, repo_find, git_dir_name, index::Index, cmd_status, StatusArgs, write_tree::write_tree, objects::Commit};


#[derive(Args)]
pub struct CommitArgs {
    #[arg(short)]
    pub message: String
}

pub fn cmd_commit(args: CommitArgs, global_opts: GlobalOpts) -> Result<()> {
    let path = env::current_dir().unwrap_or_else(|_| { panic!() });
    let root = repo_find(&path, global_opts).unwrap_or_else(|| {
        panic!("fatal: not a grit repository");
    });

    let index = read_index(&root, global_opts)?;

    // If nothing is staged, run `status` instead to prompt the user to `add` files
    if index.items.len() == 0 {
        let status_args = StatusArgs { untracked_files: None };
        return cmd_status(status_args, global_opts);
    }

    let tree = write_tree(index, &root, global_opts)?;

    let branch = "master"; // TODO
    let parent = "root-commit"; // TODO
    let hash = ""; // TODO
    
    println!("[{} ({}) {}] {}", branch, parent, hash, args.message);

    // Print summary of changes

    Ok(())
}

// Returns the current index, or an empty index if one does not exist
fn read_index(repo_root: &PathBuf, global_opts: GlobalOpts) -> Result<Index> {
    let index_path = repo_root.join(format!("{}/index", git_dir_name(global_opts)));
    if index_path.exists() {
        let index_bytes = fs::read(index_path)?;
        return Index::deserialize(index_bytes);
    } else {
        return Ok(Index { version: 2, items: Vec::new() });
    }
}