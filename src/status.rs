use std::{collections::HashSet, env, fs, path::{Path, PathBuf}};
use anyhow::{Result, anyhow};
use clap::Args;

use crate::{GlobalOpts, repo_find, index::Index, git_dir_name};

pub enum UntrackedMode {
    No,
    Normal,
    All
}

#[derive(Args)]
pub struct StatusArgs {
    #[arg(short, long)]
    pub untracked_files: Option<String>
}

pub fn cmd_status(args: StatusArgs, global_opts: GlobalOpts) -> Result<()> {
    let untracked_mode = parse_untracked_mode(&args)?;

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
    // Build a list of tracked directories.
    let mut staged = Vec::new();
    let mut tracked_dirs = HashSet::<&Path>::new();

    let index_path = root.join(format!("{}/index", git_dir_name(global_opts)));
    if index_path.exists() {
        let index_bytes = fs::read(index_path)?;
        let index = Index::deserialize(index_bytes)?;
        for item in &index.items {
            staged.push(item.path.to_string_lossy().to_string());
            if let Some(parent) = item.path.parent() {
                tracked_dirs.insert(parent);
            }
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

fn parse_untracked_mode(args: &StatusArgs) -> Result<UntrackedMode> {
    if let Some(u) = &args.untracked_files {
        match u.as_str() {
            "no" => Ok(UntrackedMode::No),
            "normal" => Ok(UntrackedMode::Normal),
            "all" => Ok(UntrackedMode::All),
            _ => Err(anyhow!("fatal: Invalid untracked files mode '{}'", u))
        }
    } else {
        Ok(UntrackedMode::Normal)
    }
}