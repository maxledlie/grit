use std::{fs, env};

use clap::Args;

use crate::{GlobalOpts, CmdError, repo_find, index::Index};


#[derive(Args)]
pub struct StatusArgs {
}

pub fn cmd_status(_args: StatusArgs, global_opts: GlobalOpts) -> Result<(), CmdError> {
    let path = env::current_dir().unwrap_or_else(|_| { panic!() });
    let root = repo_find(&path, global_opts).unwrap_or_else(|| {
        panic!("fatal: not a grit repository");
    });

    let index_bytes = fs::read(root.join(".git/index")).map_err(CmdError::IOError)?;
    let _index = Index::deserialize(index_bytes);

    Ok(())
}