use std::{fs, env, ffi::CString, mem, path::PathBuf};

use clap::{arg, Args};

use crate::{GlobalOpts, CmdError, index::{Index, IndexItem}, repo_find, git_dir_name, objects::Blob};

#[derive(Args)]
pub struct AddArgs {
    #[arg(short, long)]
    verbose: bool,
    pathspec: String,
}

pub fn cmd_add(args: AddArgs, global_opts: GlobalOpts) -> Result<(), CmdError> {
    let cwd = env::current_dir().unwrap_or_else(|_| { panic!() });
    let root = repo_find(&cwd, global_opts).unwrap_or_else(|| {
        panic!("fatal: not a {} repository", git_dir_name(global_opts));
    });

    // For now, we assume the pathspec is the literal name of a single file

    // Hash the object and write it to the store
    let path = PathBuf::from(args.pathspec);
    let bytes = fs::read(&path).map_err(CmdError::IOError)?;
    let blob = Blob { bytes };
    blob.write(&root, global_opts)?;

    let item: IndexItem;

    // Get status information on the file by calling the C standard library
    let c_path = CString::new(path.to_string_lossy().as_bytes()).map_err(|_| CmdError::Fatal(String::from("Could not interpret path as CString")))?;
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
            path
        }
    }

    let index = Index {
        version: 2,
        items: vec![item]
    };

    let index_bytes = index.serialize()?;
    let index_path = root.join(format!("{}/index", git_dir_name(global_opts)));
    fs::write(index_path, index_bytes).map_err(CmdError::IOError)?;

    Ok(())
}