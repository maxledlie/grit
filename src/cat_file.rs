use std::env;
use anyhow::{bail, Result};
use clap::Args;

use crate::{ObjectType, GlobalOpts, repo_find};
use crate::objects::{Object, search_object};


#[derive(Args)]
pub struct CatFileArgs {
    #[arg(value_enum)]
    r#type: ObjectType,
    object: String,
}

pub fn cmd_cat_file(args: CatFileArgs, global_opts: GlobalOpts) -> Result<()>{
    let path = env::current_dir().unwrap_or_else(|_| { panic!() });
    let root = repo_find(&path, global_opts).unwrap_or_else(|| {
        panic!("fatal: not a grit repository");
    });

    let hash_bytes = hex::decode(&args.object)?;
    let hash: [u8; 20] = hash_bytes.try_into().expect("invalid object hash");

    let object = match search_object(&root, &hash, global_opts.git_mode) {
        Ok(None) => bail!("object {} not found in store", args.object),
        Err(e) => return Err(e),
        Ok(Some(x)) => x
    };

    // Check that object has expected type
    match (&object, &args.r#type) {
        (Object::Blob(_), ObjectType::Blob) | 
        (Object::Commit(_), ObjectType::Commit) | 
        (Object::Tree(_), ObjectType::Tree) | 
        (Object::Tag, ObjectType::Tag) => (),
        _ => {
            let hash_str = hex::encode(&hash);
            bail!("fatal: git cat-file {}: bad file", hash_str);
        }
    }

    println!("{}", object);
    Ok(())
}
