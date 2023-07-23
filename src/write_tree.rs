use std::{env, fs, path::PathBuf};

use anyhow::Result;
use crate::{GlobalOpts, index::{Index, IndexItem}, objects::{GitObject, Tree, TreeEntry}, repo_find, git_dir_name};


pub fn cmd_write_tree(global_opts: GlobalOpts) -> Result<()> {
    let path = env::current_dir().unwrap_or_else(|_| { panic!() });
    let root = repo_find(&path, global_opts).unwrap_or_else(|| {
        panic!("fatal: not a grit repository");
    });

    let index_path = root.join(format!("{}/index", git_dir_name(global_opts)));
    let index_bytes = fs::read(index_path)?;
    let index = Index::deserialize(index_bytes)?;

    let tree = write_tree(index, &root, global_opts)?;
    println!("{}", hex::encode(tree.hash()));
    Ok(())
}


pub fn write_tree(index: Index, repo_root: &PathBuf, global_opts: GlobalOpts) -> Result<Tree> {
    /*
    Creates a tree object using the current index. The name of the new tree object is printed to standard output.
    The index must be in a fully merged state.
    Conceptually, git write-tree syncs the current index contents into a set of tree files.
    In order to have that match what is actually in your directory right now, you need to have done a git update-index
    phase before you did the git write-tree.
    */
    write_subtree(0, &index.items, repo_root, global_opts)
}


fn write_subtree(depth: usize, index: &[IndexItem], repo_root: &PathBuf, global_opts: GlobalOpts) -> Result<Tree> {
    let mut children = Vec::new();
    let mut pos = 0;
    while pos < index.len() {
        let first = &index[pos];
        if first.path.is_file() {
            // Handle blob
            children.push(TreeEntry {
                mode: first.mode,
                path: first.path.clone(),
                hash: first.hash
            });
            pos += 1;
        } else {
            // We are at the start of a subtree. Find index items in the subtree and recurse on them
            let subtree_path = PathBuf::from_iter(first.path.components().take(depth + 1));
            let subtree_end = index[pos..].iter().position(|x| !x.path.starts_with(&subtree_path));
            let subtree_items = if let Some(end) = subtree_end { 
                &index[pos..end]
            } else {
                &index[pos..]
            };
            
            let subtree = write_subtree(depth + 1, subtree_items, repo_root, global_opts)?;
            children.push(TreeEntry {
                mode: 40000,
                path: subtree_path.clone(),
                hash: subtree.hash()
            });
            
            pos = subtree_end.unwrap_or(index.len());
        }
    }

    let tree = Tree { children };
    tree.write(repo_root, global_opts)?;

    Ok(tree)
}
