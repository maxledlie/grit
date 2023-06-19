use std::{fs::{self, File}, env, io::Write};

use clap::{arg, Args};
use flate2::{write::ZlibEncoder, Compression};
use sha1::{Digest, Sha1};

use crate::{GlobalOpts, CmdError, repo_find};

#[derive(Args)]
pub struct HashObjectArgs {
    pub path: String,
    #[arg(short, long, default_value_t = String::from("blob"))]
    pub r#type: String,
    #[arg(short)]
    pub write: bool,
}

pub fn cmd_hash_object(args: HashObjectArgs, global_opts: GlobalOpts) -> Result<(), CmdError> {
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

    fs::create_dir_all(&dir).and_then(|()| {
        File::create(dir.join(file_name))
    }).and_then(|mut f| {
        f.write_all(&compressed_bytes)
    }).map_err(CmdError::IOError)?;

    Ok(())
}
