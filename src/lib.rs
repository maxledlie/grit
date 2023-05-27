use clap::Args;
use clap::{Parser, Subcommand, ValueEnum};
use flate2::Compression;
use flate2::bufread::ZlibDecoder;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::env;
use std::fs::{self, File};
use configparser::ini::Ini;
use sha1::{Sha1, Digest};
use flate2::write::ZlibEncoder;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Init { path: Option<String> },
    HashObject(HashObjectArgs),
    CatFile(CatFileArgs),
    Log(LogArgs)
}

#[derive(Args)]
pub struct HashObjectArgs {
    path: String,
    #[arg(short, long, default_value_t = String::from("blob"))]
    r#type: String,
    #[arg(short)]
    write: bool,
    #[arg(short, long)]
    git_mode: bool
}

#[derive(Args)]
pub struct CatFileArgs {
    #[arg(value_enum)]
    r#type: ObjectType,
    object: String,
    #[arg(short, long)]
    git_mode: bool
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
    #[arg(short, long)]
    git_mode: bool
}

pub fn cmd_init(path: Option<String>) {
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
}

pub fn cmd_hash_object(args: HashObjectArgs) {
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
        return;
    }

    let path = env::current_dir().unwrap_or_else(|_| { panic!() });
    let root = repo_find(&path, args.git_mode).unwrap_or_else(|| {
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
        println!("Failed to write compressed file: {e}");
    }
}

pub fn cmd_cat_file(args: CatFileArgs) {
    let path = env::current_dir().unwrap_or_else(|_| { panic!() });
    let root = repo_find(&path, args.git_mode).unwrap_or_else(|| {
        panic!("fatal: not a grit repository");
    });

    if let Some(contents) = read_object(&root, &args.object, args.git_mode) {
        println!("{}", contents);
    } else {
        println!("fatal: Not a valid object name {}", args.object);
    }
}

pub fn cmd_log(args: LogArgs) {
    let path = env::current_dir().unwrap_or_else(|_| { panic!() });
    let root = repo_find(&path, args.git_mode).unwrap_or_else(|| {
        panic!("fatal: not a grit repository");
    });

    if let Some(commit_text) = read_object(&root, &args.commit_hash, args.git_mode) {
        println!("{}", commit_text);
    } else {
        println!("fatal: Not a valid object name {}", args.commit_hash)
    }
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

fn read_object(root: &Path, hash: &String, git_mode: bool) -> Option<String> {
    if hash.len() < 3 {
        return None;
    }

    let git_dir = if git_mode { ".git" } else { ".grit" };

    let full_path = root.join(format!("{}/objects/{}/{}", git_dir, &hash[..2], &hash[2..]));
    if !full_path.exists() {
        return None;
    }

    // Read and decompress the requested file
    if let Ok(contents) = fs::read(full_path).and_then(|bytes| {
        let mut z = ZlibDecoder::new(&bytes[..]);
        let mut s = String::new();
        z.read_to_string(&mut s)?;
        Ok(s)
    }) {
        return Some(contents)
    } else {
        return None
    }
}

struct Commit {
    hash: String,
    author: String,
    date: String,
    message: String,
    parent: Option<String>,
    tree: Option<String>
}