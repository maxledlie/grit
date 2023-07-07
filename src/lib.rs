// INTERFACE

pub mod objects;

pub use crate::add::{AddArgs, cmd_add};
pub use crate::checkout::{CheckoutArgs, cmd_checkout};
pub use crate::cat_file::{CatFileArgs, cmd_cat_file};
pub use crate::hash_object::{HashObjectArgs, cmd_hash_object};
pub use crate::init::cmd_init;
pub use crate::log::{LogArgs, cmd_log};
pub use crate::ls_files::{LsFilesArgs, cmd_ls_files};
pub use crate::status::{StatusArgs, cmd_status};

// END INTERFACE

mod add;
mod cat_file;
mod checkout;
mod hash_object;
mod index;
mod init;
mod log;
mod ls_files;
mod status;

use clap::Args;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::{Path, PathBuf};
use std::fmt;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[clap(flatten)]
    pub global_opts: GlobalOpts,

    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Add(AddArgs),
    Init { path: Option<String> },
    HashObject(HashObjectArgs),
    CatFile(CatFileArgs),
    Log(LogArgs),
    LsFiles(LsFilesArgs),
    Checkout(CheckoutArgs),
    Status(StatusArgs)
}

#[derive(Args, Clone, Copy)]
pub struct GlobalOpts {
    #[arg(short, long, global = true)]
    pub git_mode: bool
}

#[derive(Copy, Clone, Eq, PartialEq, ValueEnum)]
enum ObjectType {
    Blob,
    Tree,
    Commit,
    Tag
}

pub enum CmdError {
    IOError(std::io::Error),
    Fatal(String)
}

impl fmt::Display for CmdError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CmdError::IOError(e) => write!(f, "{}", e.to_string()),
            CmdError::Fatal(e) => write!(f, "{}", e)
        }
    }
}

// Returns the path to the root of the repository at the given path.
fn repo_find(path: &Path, global_opts: GlobalOpts) -> Option<PathBuf> {
    let git_dir = git_dir_name(global_opts);

    if path.join(git_dir).exists() {
        return Some(path.to_path_buf());
    }

    let parent = path.parent();
    if parent == None || parent == Some(Path::new("")) {
        return None
    }

    repo_find(parent.unwrap(), global_opts)
}

pub fn git_dir_name(global_opts: GlobalOpts) -> String {
    if global_opts.git_mode { String::from(".git") } else { String::from(".grit") }
}

pub fn program_name(global_opts: GlobalOpts) -> String { 
    if global_opts.git_mode { String::from("Git") } else { String::from("Grit") }
}