use clap::Args;
use clap::{Parser, Subcommand};
use std::path::Path;
use std::env;
use std::fs;
use configparser::ini::Ini;
use sha1::{Sha1, Digest};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init { path: Option<String> },
    HashObject(HashObjectArgs) 
}

#[derive(Args)]
struct HashObjectArgs {
    path: String,
    #[arg(short, long, default_value_t = String::from("blob"))]
    r#type: String,
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Init { path } => cmd_init(path),
        Commands::HashObject(args) => cmd_hash_object(args)
    }
}

fn cmd_init(path: Option<String>) {
    let worktree = path
        .map(|p| Path::new(&p).to_path_buf())
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|e| {
            grit_err("A path was not provided and the current working directory is invalid", Some(e));
            Path::new("").to_path_buf()
        }));

    let gitdir = worktree.join(".git"); 

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

fn cmd_hash_object(args: HashObjectArgs) {
    let mut hasher: Sha1 = Sha1::new();

    // Read the file at the given path
    let Ok(content_bytes) = fs::read(args.path) else { panic!() };

    // Prepend header: the file type and size
    let header_str = args.r#type + " " + &content_bytes.len().to_string() + "\0";
    let header_bytes = header_str.as_bytes();

    let bytes = [header_bytes, &content_bytes].concat();

    hasher.update(bytes);
    let hash = hasher.finalize();
    let hash_str = hex::encode(hash);
    println!("{}", hash_str);
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