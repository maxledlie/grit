use std::{env, fs};

use anyhow::Result;
use clap::Args;

use crate::{GlobalOpts, repo_find, git_dir_name, index::Index, cmd_status, StatusArgs};


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

    let mut staged = Vec::new();
    let index_path = root.join(format!("{}/index", git_dir_name(global_opts)));
    if index_path.exists() {
        let index_bytes = fs::read(index_path)?;
        let index = Index::deserialize(index_bytes)?;
        for item in &index.items {
            staged.push(item.path.to_string_lossy().to_string());
        }
    }

    // If nothing is staged, run `status` instead to prompt the user to `add` files
    if staged.len() == 0 {
        let status_args = StatusArgs { };
        return cmd_status(status_args, global_opts);
    }

    let branch = "master"; // TODO
    let parent = "root-commit"; // TODO
    let hash = ""; // TODO
    
    println!("[{} ({}) {}] {}", branch, parent, hash, args.message);

    // Print summary of changes

    Ok(())
}
