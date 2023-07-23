use std::{fs, env};
use anyhow::Result;
use clap::{arg, Args};

use crate::{GlobalOpts, repo_find, objects::{Blob, GitObject}};

#[derive(Args)]
pub struct HashObjectArgs {
    pub path: String,
    #[arg(short, long, default_value_t = String::from("blob"))]
    pub r#type: String,
    #[arg(short)]
    pub write: bool,
}

pub fn cmd_hash_object(args: HashObjectArgs, global_opts: GlobalOpts) -> Result<()> {
    // Read the file at the given path
    let Ok(content_bytes) = fs::read(&args.path) else { panic!() };
    
    // Assume the object is a blob for now
    let blob = Blob { bytes: content_bytes };
    let hash = blob.hash();

    let hash_str = hex::encode(hash);
    println!("{}", hash_str);

    if args.write {
        let path = env::current_dir().unwrap_or_else(|_| { panic!() });
        let root = repo_find(&path, global_opts).unwrap_or_else(|| {
            panic!("fatal: not a grit repository");
        });

        blob.write(&root, global_opts)?;
    }

    Ok(())
}
