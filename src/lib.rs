// INTERFACE

pub mod objects;

pub use crate::checkout::{CheckoutArgs, cmd_checkout};
pub use crate::cat_file::{CatFileArgs, cmd_cat_file};
pub use crate::hash_object::{HashObjectArgs, cmd_hash_object};
pub use crate::init::cmd_init;
pub use crate::log::{LogArgs, cmd_log};

// END INTERFACE

mod cat_file;
mod checkout;
mod hash_object;
mod init;
mod log;

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
    Init { path: Option<String> },
    HashObject(HashObjectArgs),
    CatFile(CatFileArgs),
    Log(LogArgs),
    Checkout(CheckoutArgs)
}

#[derive(Args)]
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
            CmdError::IOError(e) => write!(f, "fatal: {}", e.to_string()),
            CmdError::Fatal(e) => write!(f, "fatal: {}", e)
        }
    }
}

// Returns the path to the root of the repository at the given path.
fn repo_find(path: &Path, git_mode: bool) -> Option<PathBuf> {
    let git_dir = if git_mode { ".git" } else { ".grit" };

    if path.join(git_dir).exists() {
        return Some(path.to_path_buf());
    }

    let parent = path.parent();
    if parent == None || parent == Some(Path::new("")) {
        return None
    }

    repo_find(parent.unwrap(), git_mode)
}
