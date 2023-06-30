use std::env;

use clap::Args;

use crate::{GlobalOpts, repo_find, objects::{parse_hash, parse_commit, Commit, read_object_raw}, CmdError};


#[derive(Args)]
pub struct LogArgs {
    commit_hash: String,
}

pub fn cmd_log(args: LogArgs, global_opts: GlobalOpts) -> Result<(), CmdError> {
    let path = env::current_dir().unwrap_or_else(|_| { panic!() });
    let root = repo_find(&path, global_opts).unwrap_or_else(|| {
        panic!("fatal: not a grit repository");
    });

    let mut current_hash = Some(parse_hash(&args.commit_hash)?);
    while let Some(hash) = current_hash {
        match read_object_raw(&root, &hash, global_opts.git_mode) {
            Ok(Some(bytes)) => {
                let commit_text = String::from_utf8_lossy(&bytes).to_string();
                let commit = parse_commit(&commit_text)?;
                print_commit(&commit, &args.commit_hash);

                // TODO: Handle multiple parents due to merges
                current_hash = commit.parent;
            },
            Ok(None) => { return Err(CmdError::Fatal(format!("object {} not found in store", args.commit_hash))); },
            Err(e) => { return Err(e) }
        }
    }
    Ok(())
}

fn print_commit(commit: &Commit, hash: &String) {
    println!("commit {}", hash);
    println!("Author: {}", commit.committer);
    if let Some(date) = &commit.date {
        println!("Date: {}", date);
    }
    println!();
    println!("\t{}", commit.message);
    println!();
}
