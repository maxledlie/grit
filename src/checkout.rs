use std::{fs, path::PathBuf, env};

use clap::Args;

use crate::{GlobalOpts, repo_find, CmdError};
use crate::objects::{get_object, Commit, Object, search_object, parse_hash, Tree};

#[derive(Args)]
pub struct CheckoutArgs {
    /// The commit or tree to checkout
    pub commit: String,
    /// The EMPTY directory to checkout on
    pub directory: String
}

pub fn cmd_checkout(args: CheckoutArgs, global_opts: GlobalOpts) -> Result<(), CmdError> {
    // Fail if the given directory is not empty
    let destination = PathBuf::from(args.directory);
    let contents = fs::read_dir(&destination).map_err(CmdError::IOError)?;
    
    if contents.into_iter().count() > 0 {
        return Err(CmdError::Fatal("Destination directory is not empty!".to_owned()));
    }

    let path = env::current_dir().unwrap_or_else(|_| { panic!() });
    let root = repo_find(&path, global_opts).unwrap_or_else(|| {
        panic!("fatal: not a grit repository");
    });

    let hash = parse_hash(&args.commit)?;

    // Parse the given commit object
    match search_object(&root, &hash, global_opts.git_mode) {
        Ok(Some(Object::Commit(c))) => checkout_commit(&root, c, &destination, global_opts.git_mode),
        Ok(Some(_)) => Err(CmdError::Fatal(String::from("Requested object is not a commit"))),
        Ok(None) => Err(CmdError::Fatal(String::from("Commit object does not exist"))),
        Err(e) => Err(e)
    }
}

fn checkout_commit(root: &PathBuf, commit: Commit, destination: &PathBuf, git_mode: bool) -> Result<(), CmdError> {
    match get_object(root, &commit.tree, git_mode) {
        Ok(Object::Tree(t)) => checkout_tree(root, t, destination, git_mode),
        Ok(_) => Err(CmdError::Fatal(String::from("Commit references a tree that is not actually a tree"))),
        Err(e) => Err(e)
    }
}

fn checkout_tree(root: &PathBuf, tree: Tree, destination: &PathBuf, git_mode: bool) -> Result<(), CmdError> {
    for leaf in tree.leaves.into_iter() {
        println!("Checking out following tree node...");
        println!("{}", leaf);

        let output_path = destination.join(&leaf.path);

        match get_object(root, &leaf.hash, git_mode) {
            Ok(Object::Blob(bytes)) => { fs::write(output_path, bytes).map_err(CmdError::IOError)?; },
            Ok(Object::Tree(_)) => {}, // TODO: Recurse on the subtree
            Ok(_) => return Err(CmdError::Fatal(String::from("Unexpected object found in tree. Expecting only blobs or trees"))),
            Err(e) => return Err(e)
        }
    }

    Ok(())
}