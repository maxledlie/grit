// Show information about files in the index and the working tree

use std::{env, fs};
use anyhow::Result;
use clap::Args;

use crate::{GlobalOpts, repo_find, git_dir_name, index::Index};

#[derive(Args)]
pub struct LsFilesArgs {
}

pub fn cmd_ls_files(_args: LsFilesArgs, global_opts: GlobalOpts) -> Result<()> {
    let path = env::current_dir().unwrap_or_else(|_| { panic!() });
    let root = repo_find(&path, global_opts).unwrap_or_else(|| {
        panic!("fatal: not a grit repository");
    });

    let index_path = root.join(format!("{}/index", git_dir_name(global_opts)));
    let index_bytes = fs::read(index_path)?;
    let index = Index::deserialize(index_bytes)?;

    for item in index.items {
        println!("{}", item.path.to_string_lossy());
    }

    Ok(())
}