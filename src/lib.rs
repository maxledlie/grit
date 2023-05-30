use clap::Args;
use clap::{Parser, Subcommand, ValueEnum};
use flate2::Compression;
use objects::Commit;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::env;
use std::fs::{self, File};
use configparser::ini::Ini;
use sha1::{Sha1, Digest};
use flate2::write::ZlibEncoder;

use crate::objects::{self as obj, parse_commit};

pub mod objects;

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
    Log(LogArgs)
}

#[derive(Args)]
pub struct GlobalOpts {
    #[arg(short, long, global = true)]
    git_mode: bool
}

#[derive(Args)]
pub struct HashObjectArgs {
    path: String,
    #[arg(short, long, default_value_t = String::from("blob"))]
    r#type: String,
    #[arg(short)]
    write: bool,
}

#[derive(Args)]
pub struct CatFileArgs {
    #[arg(value_enum)]
    r#type: ObjectType,
    object: String,
}

#[derive(Copy, Clone, Eq, PartialEq, ValueEnum)]
enum ObjectType {
    Blob,
    Tree,
    Commit,
    Tag
}

#[derive(Args)]
pub struct LogArgs {
    commit_hash: String,
}

pub fn cmd_init(path: Option<String>, _global_opts: GlobalOpts) -> Result<(), String> {
    let worktree = path
        .map(|p| Path::new(&p).to_path_buf())
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|e| {
            grit_err("A path was not provided and the current working directory is invalid", Some(e));
            Path::new("").to_path_buf()
        }));

    let gitdir = worktree.join(".grit"); 

    // Create the folder if it does not exist
    if !gitdir.exists() {
        std::fs::create_dir_all(&gitdir).unwrap_or_else(|e| {
            grit_err("Directory does not exist and could not be created", Some(e));
        });
    }

    // Create a default config file
    let config = repo_default_config();
    let config_path = gitdir.join("config");
    config.write(config_path).unwrap_or_else(|e| {
        grit_err("Failed to write config", Some(e));
    });

    // Create objects directory
    let objects_path = gitdir.join("objects");
    std::fs::create_dir(&objects_path).unwrap_or_else(|e| {
        grit_err("Failed to create file {objects_path}", Some(e));
    });

    println!("Initialized empty Grit repository in {}", gitdir.to_string_lossy());
    Ok(())
}

pub fn cmd_hash_object(args: HashObjectArgs, global_opts: GlobalOpts) -> Result<(), String> {
    let mut hasher: Sha1 = Sha1::new();

    // Read the file at the given path
    let Ok(content_bytes) = fs::read(&args.path) else { panic!() };

    // Prepend header: the file type and size
    let header_str = args.r#type + " " + &content_bytes.len().to_string() + "\0";
    let header_bytes = header_str.as_bytes();

    let bytes = [header_bytes, &content_bytes].concat();

    hasher.update(bytes);
    let hash_bytes = hasher.finalize();
    let hash_str = hex::encode(hash_bytes);
    println!("{}", hash_str);

    if !args.write {
        return Ok(());
    }

    let path = env::current_dir().unwrap_or_else(|_| { panic!() });
    let root = repo_find(&path, global_opts.git_mode).unwrap_or_else(|| {
        panic!("fatal: not a grit repository");
    });

    // Compress the file contents
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&content_bytes).unwrap_or_else(|e| {
        panic!("Compression of object failed: {e}");
    });

    let compressed_bytes = encoder.finish().unwrap_or_else(|e| {
        panic!("Compression of object failed: {e}");
    });

    // The first two characters of the SHA1 hash are used to name a directory. The remaining 14 name the file within
    // that directory. This is just for practical reasons, because most operating systems slow down on directories
    // with loads of files.
    let dir_name = &hash_str[..2];
    let file_name = &hash_str[2..];

    let dir = root.join(format!(".grit/objects/{}", dir_name));
    let result = fs::create_dir_all(&dir).and_then(|()| {
        File::create(dir.join(file_name))
    }).and_then(|mut f| {
        f.write_all(&compressed_bytes)
    });

    if let Err(e) = result {
        Err(format!("Failed to write compressed file: {}", e))
    } else {
        Ok(())
    }
}

pub fn cmd_cat_file(args: CatFileArgs, global_opts: GlobalOpts) -> Result<(), String>{
    let path = env::current_dir().unwrap_or_else(|_| { panic!() });
    let root = repo_find(&path, global_opts.git_mode).unwrap_or_else(|| {
        panic!("fatal: not a grit repository");
    });

    if let Some(contents) = obj::read_text(&root, &args.object, global_opts.git_mode) {
        println!("{}", contents);
        return Ok(());
    } else {
        return Err(format!("Not a valid object name {}", args.object));
    }
}

pub fn cmd_log(args: LogArgs, global_opts: GlobalOpts) -> Result<(), String> {
    let path = env::current_dir().unwrap_or_else(|_| { panic!() });
    let root = repo_find(&path, global_opts.git_mode).unwrap_or_else(|| {
        panic!("fatal: not a grit repository");
    });

    let mut current_hash = Some(args.commit_hash.clone());
    while let Some(hash) = current_hash {
        if let Some(commit_text) = obj::read_text(&root, &hash, global_opts.git_mode) {
            let commit = parse_commit(&commit_text)?;
            print_commit(&commit, &args.commit_hash);

            // TODO: Handle multiple parents due to merges
            current_hash = commit.parent;
        } else {
            return Err(format!("Not a valid object name {}", args.commit_hash))
        }
    }
    Ok(())
}

fn print_commit(commit: &Commit, hash: &String) {
    println!("commit {}", hash);
    println!("Author: {}", commit.committer);
    if let Some(date) = &commit.date {
        println!("Date: {}", date);
    }
    println!();
    println!("\t{}", commit.message);
    println!();
}

fn repo_default_config() -> Ini {
    let mut config = Ini::new();
    config.set("core", "repositoryformatversion", Some(String::from("0")));
    config.set("core", "filemode", Some(String::from("false")));
    config.set("core", "bare", Some(String::from("false")));

    config
}

fn grit_err<E: std::error::Error>(text: &str, inner_err: Option<E>) {
    println!("ERR: {text}:");
    if let Some(e) = inner_err {
        println!("{e}");
    }
    panic!()
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
