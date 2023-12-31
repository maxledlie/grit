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

    // Currently assuming all files are uncommitted.
    // Once `commit` is implemented, only report files that are not in the HEAD tree
    let mut staged = Vec::new();
    let index_path = root.join(format!("{}/index", git_dir_name(global_opts)));
    if index_path.exists() {
        let index_bytes = fs::read(index_path)?;
        let index = Index::deserialize(index_bytes)?;
        for item in &index.items {
            staged.push(item.path.to_string_lossy().to_string());
        }
    }

    // Find untracked files - those in the working directory but
    // not listed in the index.
    let mut untracked = Vec::new();
    let mut untracked_paths: Vec<String> = walk_worktree(&root, &git_dir_name(global_opts))?.iter()
        .map(|x| x.strip_prefix(&root).unwrap().to_string_lossy().to_string())
        .collect();
    untracked_paths.sort();

    for path in untracked_paths {
        if !staged.iter().any(|x| x == &path) {
            untracked.push(path);
        }
    }

    if staged.len() > 0 {
        println!("Changes to be committed:");
        println!("  (use \"git rm --cached <file>...\" to unstage)");
        for path in &staged {
            println!("\tnew file:   {}", path);
        }
        println!();
    }

    if untracked.len() > 0 {
        println!("Untracked files:");
        println!("  (use \"git add <file>...\" to include in what will be committed)");
        for x in &untracked {
            println!("\t{}", x);
        }
        println!();
    }

    if untracked.len() > 0 && staged.len() == 0 {
        println!("nothing added to commit but untracked files present (use \"git add\" to track)");
    }

    if untracked.len() == 0 && staged.len() == 0 {
        println!("nothing to commit (create/copy files and use \"git add\" to track)");
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