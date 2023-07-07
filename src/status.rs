use std::{fs, env};
use anyhow::Result;
use clap::Args;

use crate::{GlobalOpts, repo_find, index::Index};


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
    for item in index.items {
        println!("\tnew file:   {}", item.path.to_string_lossy());
    }

    Ok(())
}