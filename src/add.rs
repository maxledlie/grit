use std::{fs, env, ffi::CString, mem, path::PathBuf};
use anyhow::{Result, anyhow};
use clap::{arg, Args};

use crate::{GlobalOpts, index::{Index, IndexItem}, repo_find, git_dir_name, objects::{Blob, GitObject}};

#[derive(Args)]
pub struct AddArgs {
    #[arg(short, long)]
    verbose: bool,
    pathspec: String,
}

pub fn cmd_add(args: AddArgs, global_opts: GlobalOpts) -> Result<()> {
    let cwd = env::current_dir().unwrap_or_else(|_| { panic!() });
    let root = repo_find(&cwd, global_opts).unwrap_or_else(|| {
        panic!("fatal: not a {} repository", git_dir_name(global_opts));
    });

    // For now, we assume the pathspec is a single file
    // The provided path may be relative or absolute
    let provided_path = PathBuf::from(args.pathspec);
    let index_item_path = rebase_path(&provided_path, &root)?;

    // Hash the object and write it to the store
    let bytes = fs::read(provided_path)?;

    let blob = Blob { bytes };
    blob.write(&root, global_opts)?;

    let item: IndexItem;

    // Get status information on the file by calling the C standard library
    let c_path = CString::new(index_item_path.to_string_lossy().as_bytes())?;
    unsafe {
        let mut stat: libc::stat = mem::zeroed();
        libc::stat(c_path.as_ptr(), &mut stat);

        item = IndexItem {
            ctime: u32::try_from(stat.st_ctime).unwrap(),
            ctime_nsec: u32::try_from(stat.st_ctime_nsec).unwrap(),
            mtime: u32::try_from(stat.st_mtime).unwrap(),
            mtime_nsec: u32::try_from(stat.st_mtime_nsec).unwrap(),
            dev: u32::try_from(stat.st_dev).unwrap(),
            ino: u32::try_from(stat.st_ino).unwrap(),
            mode: u32::try_from(stat.st_mode).unwrap(),
            uid: u32::try_from(stat.st_uid).unwrap(),
            gid: u32::try_from(stat.st_gid).unwrap(),
            size: u32::try_from(stat.st_size).unwrap(),
            hash: blob.hash(),
            path: index_item_path
        }
    }

    let index_path = root.join(format!("{}/index", git_dir_name(global_opts)));
    let mut index: Index;
    if index_path.exists() {
        let index_bytes = fs::read(&index_path)?;
        index = Index::deserialize(index_bytes)?;

        // Remove any existing entry for this path
        index.items.retain(|x| x.path != item.path);

        // Find position to insert this item in that will preserve ordering by path name
        let new_path_str = item.path.to_string_lossy();
        let new_path_bytes = new_path_str.as_bytes();

        let mut inserted = false;
        for i in 0..index.items.len() {
            let current_path_str = index.items[i].path.to_string_lossy();
            let current_path_bytes = current_path_str.as_bytes();
            if mem_cmp(new_path_bytes, current_path_bytes) > 0 {
                index.items.insert(i, item.clone());
                inserted = true;
                break;
            }
        }

        if !inserted {
            index.items.push(item.clone());
        }
    } else {
        index = Index {
            version: 2,
            items: vec![item]
        };
    }

    let index_bytes = index.serialize()?;
    fs::write(index_path, index_bytes)?;

    Ok(())
}

/// Paths may be provided as absolute or relative to the current working directory.
/// When written to the index, they are stored relative to the repository root.
/// This fuction returns the path relative to the repository root, if the provided path is within the repository.
/// Otherwise returns an error.
fn rebase_path(path: &PathBuf, root: &PathBuf) -> Result<PathBuf> {
    let path = path.canonicalize().map_err(|_| anyhow!("Invalid path {:?}", path))?;
    let rel_path = path.strip_prefix(root)
        .map_err(|_| anyhow!("{:?} is outside repository at {:?}", path, root))?;

    Ok(rel_path.to_path_buf())
}

// Compares the byte arrays as a string of unsigned bytes. Returns -1 if left is greater, 0 if equal, 1 if right is greater.
fn mem_cmp(left: &[u8], right: &[u8]) -> isize {
    let min_len: usize = std::cmp::min(left.len(), right.len());
    for i in 0..min_len {
        if left[i] < right[i] {
            return 1;
        }
        if left[i] > right[i] {
            return -1;
        }
    }

    // All aligned bytes were equal: the larger string is the longer one
    if left.len() > right.len() {
        return -1;
    }
    if left.len() < right.len() {
        return 1;
    }

    return 0;
}