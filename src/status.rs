use std::{fs, env, path::PathBuf};
use anyhow::Result;
use clap::Args;

use crate::{GlobalOpts, repo_find, index::Index, git_dir_name};


#[derive(Args)]
pub struct StatusArgs {
}

pub fn cmd_status(_args: StatusArgs, global_opts: GlobalOpts) -> Result<()> {
    let path = env::current_dir().unwrap_or_else(|_| { panic!() });
    let root = repo_find(&path, global_opts).unwrap_or_else(|| {
        panic!("fatal: not a grit repository");
    });

    // TODO: Handle different branches
    println!("On branch master");
    println!();

    // TODO: Check log to determine if there have been commits
    println!("No commits yet");
    println!();

    let index_bytes = fs::read(root.join(".git/index"))?;
    let index = Index::deserialize(index_bytes)?;

    // Currently assuming all files are uncommitted.
    // Once `commit` is implemented, only report files that are not in the HEAD tree
    println!("Changes to be committed:");
    println!("  (use \"git rm --cached <file>...\" to unstage)");
    for item in &index.items {
        println!("\tnew file:   {}", item.path.to_string_lossy());
    }
    println!();

    // Find untracked files - those in the working directory but
    // not listed in the index.
    println!("Untracked files:");
    println!("  (use \"git add <file>...\" to include in what will be committed)");
    for path in walk_worktree(&root, &git_dir_name(global_opts))? {
        let rel_path = path.strip_prefix(&root)?;
        if !index.items.iter().any(|x| x.path == rel_path) {
            println!("\t{}", rel_path.to_string_lossy());
        }
    }

    Ok(())
}

fn walk_worktree(path: &PathBuf, git_dir_name: &str) -> Result<Vec<PathBuf>> {
    let mut ret = Vec::new();
    for entry in fs::read_dir(&path)? {
        let entry = entry?;
        let entry_path = path.join(entry.file_name());
        if entry.file_type()?.is_file() {
            ret.push(entry_path);
        } else if entry.file_type()?.is_dir() && entry.file_name() != git_dir_name {
            let mut dir_files = walk_worktree(&entry_path, git_dir_name)?;
            ret.append(&mut dir_files);
        }
    } 

    Ok(ret)
}