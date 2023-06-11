use std::{fs, path::PathBuf, env};

use clap::Args;

use crate::{GlobalOpts, repo_find, obj::{get_object, Commit, Object, GitTreeLeaf, search_object}, CmdError};

#[derive(Args)]
pub struct CheckoutArgs {
    /// The commit or tree to checkout
    pub commit: String,
    /// The EMPTY directory to checkout on
    pub directory: String
}

pub fn cmd_checkout(args: CheckoutArgs, global_opts: GlobalOpts) -> Result<(), CmdError> {
    // Fail if the given directory is not empty
    let path = PathBuf::from(args.directory);
    let contents = fs::read_dir(&path).map_err(CmdError::IOError)?;
    
    if contents.into_iter().count() > 0 {
        return Err(CmdError::Fatal("Destination directory is not empty!".to_owned()));
    }

    let path = env::current_dir().unwrap_or_else(|_| { panic!() });
    let root = repo_find(&path, global_opts.git_mode).unwrap_or_else(|| {
        panic!("fatal: not a grit repository");
    });

    // Parse the given commit object
    match search_object(&root, &args.commit, global_opts.git_mode) {
        Ok(Some(Object::Commit(c))) => checkout_commit(&root, c, global_opts.git_mode),
        Ok(Some(_)) => Err(CmdError::Fatal(String::from("Requested object is not a commit"))),
        Ok(None) => Err(CmdError::Fatal(String::from("Commit object does not exist"))),
        Err(e) => Err(e)
    }
}

fn checkout_commit(root: &PathBuf, commit: Commit, git_mode: bool) -> Result<(), CmdError> {
    match get_object(root, &commit.tree, git_mode) {
        Ok(Object::Tree(t)) => checkout_tree(root, &t, git_mode),
        Ok(_) => Err(CmdError::Fatal(String::from("Commit references a tree that is not actually a tree"))),
        Err(e) => Err(e)
    }
}

fn checkout_tree(root: &PathBuf, tree: &Vec<GitTreeLeaf>, git_mode: bool) -> Result<(), CmdError> {
    for leaf in tree.into_iter() {
        match get_object(root, &leaf.hash, git_mode) {
            Ok(Object::Blob) => {}, // TODO: Write the file
            Ok(Object::Tree(_)) => {}, // TODO: Recurse on the subtree
            Ok(_) => return Err(CmdError::Fatal(String::from("Unexpected object found in tree. Expecting only blobs or trees"))),
            Err(e) => return Err(e)
        }
    }

    Ok(())
}