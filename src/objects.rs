// This module should encapsulate knowledge about how objects are represented in Git.
// Callers should only need to know that objects are identified by a hash.

use std::{path::PathBuf, fs::{self, File}, io::{Read, Write}, collections::HashMap, fmt};

use flate2::{bufread::ZlibDecoder, write::ZlibEncoder, Compression};
use sha1::{Sha1, Digest};

use crate::{CmdError, git_dir_name, GlobalOpts};

pub enum Object {
    Blob(Vec<u8>),
    Commit(Commit),
    Tree(Tree),
    Tag
}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Object::Blob(bytes) => write!(f, "{}", String::from_utf8_lossy(&bytes)),
            Object::Commit(c) => write!(f, "{}", c),
            Object::Tree(t) => write!(f, "{}", t),
            Object::Tag => write!(f, "TODO: IMPL DISPLAY FOR TAG")
        }
    }
}

pub struct Blob {
    pub bytes: Vec<u8>
}

impl Blob {
    pub fn hash(&self) -> [u8; 20] {
        let mut hasher: Sha1 = Sha1::new();
        // Prepend header: the file type and size
        let header_str = String::from("blob ") + &self.bytes.len().to_string() + "\0";
        let header_bytes = header_str.as_bytes();

        let bytes = [header_bytes, &self.bytes].concat();

        hasher.update(&bytes);
        hasher.finalize().into()
    }

    pub fn compress(&self) -> Vec<u8> {
        // Prepend header: the file type and size
        let header_str = String::from("blob ") + &self.bytes.len().to_string() + "\0";
        let header_bytes = header_str.as_bytes();

        let bytes = [header_bytes, &self.bytes].concat();

        // Compress the file contents.
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(&bytes).unwrap_or_else(|e| {
            panic!("Compression of object failed: {e}");
        });

        encoder.finish().unwrap_or_else(|e| {
            panic!("Compression of object failed: {e}");
        })
    }

    pub fn write(&self, repo_root: &PathBuf, global_opts: GlobalOpts) -> Result<(), CmdError> {
        let hash = self.hash();
        let compressed_bytes = self.compress();

        // The first two characters of the SHA1 hash are used to name a directory. The remaining 14 name the file within
        // that directory. This is just for practical reasons, because most operating systems slow down on directories
        // with loads of files.
        let hash_str = hex::encode(hash);
        let dir_name = &hash_str[..2];
        let file_name = &hash_str[2..];

        let dir = repo_root.join(format!("{}/objects/{}", git_dir_name(global_opts), dir_name));

        fs::create_dir_all(&dir).and_then(|()| {
            File::create(dir.join(file_name))
        }).and_then(|mut f| {
            f.write_all(&compressed_bytes)
        }).map_err(CmdError::IOError)?;

        Ok(())
    }
}

pub struct Commit {
    /// The SHA1 hash of the tree describing the directory contents at this commit
    pub tree: [u8; 20],
    pub author: String,
    pub committer: String,
    pub date: Option<String>,
    /// The SHA1 hash of the commit's parent if it has one
    pub parent: Option<[u8; 20]>,
    pub message: String,
}

impl fmt::Display for Commit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "tree: {}", hex::encode(&self.tree))?;
        if let Some(parent) = &self.parent {
            writeln!(f, "parent: {}", hex::encode(parent))?;
        } 
        writeln!(f, "author: {}", &self.author)?;
        writeln!(f, "committer: {}", &self.committer)?;
        writeln!(f, "")?;
        writeln!(f, "{}", &self.message)
    }
}

pub struct Tree {
    pub leaves: Vec<GitTreeLeaf>
}

impl fmt::Display for Tree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for leaf in &self.leaves {
            writeln!(f, "{}", leaf)?;
        }
        Ok(())
    }
}


pub struct GitTreeLeaf {
    /// The unix file mode
    pub mode: Vec<u8>,
    /// The path to the file
    pub path: PathBuf,
    /// The SHA1 hash of the file contents
    pub hash: [u8; 20]
}

impl fmt::Display for GitTreeLeaf {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mode = String::from_utf8_lossy(&self.mode);
        let hash = hex::encode(&self.hash);
        write!(f, "{} {} {}", mode, hash, &self.path.to_string_lossy())
    }
}

/// Attempts to interpret the given string as a 20-byte SHA1 hash
pub fn parse_hash(hash: &String) -> Result<[u8; 20], CmdError> {
    let bytes = hex::decode(hash).map_err(|e| CmdError::Fatal(e.to_string()))?;
    Ok(bytes.try_into().map_err(|_| CmdError::Fatal(String::from("invalid hash")))?)
}

pub fn search_object(root: &PathBuf, hash: &[u8; 20], git_mode: bool) -> Result<Option<Object>, CmdError> {
    match read_object_raw(root, hash, git_mode) {
        Ok(Some(bytes)) => {
            let type_end = bytes.iter().position(|x| x == &b' ')
                .ok_or(CmdError::Fatal(String::from("error parsing object: `type` field not terminated")))?;

            let file_size_end = (type_end + 1) + bytes[type_end+1..].iter().position(|x| x == &0)
                .ok_or(CmdError::Fatal(String::from("error parsing object: `size` field not terminated")))?;

            let object_type = &bytes[..type_end];
            let _file_size = &bytes[type_end+1..file_size_end];
            let contents = &bytes[file_size_end+1..];

            match object_type {
                b"blob" => Ok(Some(Object::Blob(contents.to_vec()))),
                b"tree" => {
                    match parse_tree(contents) {
                        Ok(t) => Ok(Some(Object::Tree(t))),
                        Err(e) => Err(e)
                    }
                }
                b"tag" => Ok(Some(Object::Tag)),
                b"commit" => {
                    match parse_commit(&String::from_utf8_lossy(&contents).to_string()) {
                        Ok(c) => Ok(Some(Object::Commit(c))),
                        Err(e) => Err(e)
                    }
                }
                _ => Err(CmdError::Fatal(format!("unrecognised object type")))
            }
        },
        Ok(None) => Ok(None),
        Err(e) => Err(e)
    }
}

/// Retrieves the object with the given hash from the store, or an Err if it doesn't exist.
/// Use this when the object is referenced by a different object, so it's absence suggests the store is corrupted.
pub fn get_object(root: &PathBuf, hash: &[u8; 20], git_mode: bool) -> Result<Object, CmdError> {
    match search_object(root, hash, git_mode) {
        Ok(Some(x)) => Ok(x),
        Ok(None) => Err(CmdError::Fatal(format!("Object {} not found in store", String::from_utf8_lossy(hash)))),
        Err(e) => Err(e)
    }
}


// Returns the decompressed contents of the object with the given hash, or None
// if the object does not exist, or an error if the object exists but decompression fails
pub fn read_object_raw(root: &PathBuf, hash: &[u8; 20], git_mode: bool) -> Result<Option<Vec<u8>>, CmdError> {
    if hash.len() < 3 {
        return Ok(None);
    }

    let git_dir = if git_mode { ".git" } else { ".grit" };

    let hash_str = hex::encode(&hash);

    let full_path = root.join(format!(
        "{}/objects/{}/{}", 
        git_dir, 
        &hash_str[0..2], 
        &hash_str[2..]
    ));

    if !full_path.exists() {
        return Ok(None);
    }

    // Read and decompress the requested file
    let bytes = fs::read(full_path).map_err(CmdError::IOError)?;
    let mut z = ZlibDecoder::new(&bytes[..]);
    
    let mut buf = Vec::<u8>::new();
    z.read_to_end(&mut buf).map_err(CmdError::IOError)?;

    Ok(Some(buf))
}

enum ParseState {
    BeforeKey,
    InKey,
    BeforeValue,
    InValue,
    InMessage
}

pub fn parse_commit(commit_text: &String) -> Result<Commit, CmdError> {
    let mut buffer = String::from("");
    let mut current_key: Option<String> = Some(String::from(""));
    let mut state = ParseState::InKey;

    let mut tags = HashMap::<String, String>::new();
    
    for c in commit_text.chars() {
        match state {
            ParseState::BeforeKey => {
                match c {
                    '\n' => {
                        buffer.clear();
                        state = ParseState::InMessage;
                    },
                    _ => {
                        buffer.clear();
                        buffer.push(c);
                        state = ParseState::InKey;
                    }
                }
            },
            ParseState::InKey => {
                match c {
                    ' ' => {
                        // End of key
                        current_key = Some(buffer.clone());
                        state = ParseState::BeforeValue;
                    }
                    _ => buffer.push(c)
                }
            },
            ParseState::BeforeValue => {
                match c {
                    '\n' => {
                        return Err(CmdError::Fatal(String::from("unexpected new line in commit text")));
                    },
                    c if c.is_whitespace() => {
                        continue;
                    },
                    c => {
                        buffer.clear();
                        buffer.push(c);
                        state = ParseState::InValue;
                    }
                }
            },
            ParseState::InValue => {
                match c {
                    '\n' => {
                        // End of value
                        if let Some(ref key) = current_key {
                            tags.insert(key.to_string(), buffer.clone());
                            state = ParseState::BeforeKey;
                        } else {
                            return Err(CmdError::Fatal(String::from("invalid commit text")));
                        }
                    },
                    _ => {
                        buffer.push(c);
                    }
                }
            },
            ParseState::InMessage => {
                buffer.push(c);
            }
        }
    }
    
    let message = buffer;

    let parent = match tags.get("parent") {
        Some(hash) => Some(parse_hash(hash)?),
        None => None
    };

    let tree = parse_hash(tags.get("tree").unwrap())?;

    // TODO: Investigate better ways of doing this. Macros?
    Ok(Commit {
        author: tags.get("author").unwrap().to_string(),
        committer: tags.get("committer").unwrap().to_string(),
        date: tags.get("date").cloned(),
        parent,
        tree,
        message,
    })
}

fn parse_tree(bytes: &[u8]) -> Result<Tree, CmdError> {
    let mut nodes = Vec::new();
    let mut pos: usize = 0;
    let max = bytes.len();
    
    while pos < max {
        let node = parse_tree_node(bytes, &mut pos)?; 
        nodes.push(node);
    }

    Ok(Tree { leaves: nodes })
}

fn parse_tree_node(bytes: &[u8], pos: &mut usize) -> Result<GitTreeLeaf, CmdError> {
    let remainder = &bytes[*pos..];

    // Find the space that terminates the file mode
    let mode_end = remainder.iter().position(|x| x == &b' ')
        .ok_or(CmdError::Fatal(String::from(
            "error parsing tree: missing space terminator for file mode"
        )))?;

    // Read the mode
    let mode = remainder[..mode_end].to_vec();

    // Find the NULL terminator of the path
    let path_end = remainder.iter().position(|x| x == &0)
        .ok_or(CmdError::Fatal(String::from(
            "error parsing tree: missing NULL terminator for path"
        )))?;

    let path_str = String::from_utf8(remainder[mode_end+1..path_end].to_vec())
        .map_err(|_| CmdError::Fatal(String::from(
            "error parsing tree: non-UTF8 character in path"
        )))?;
    let path = PathBuf::from(path_str);
    let hash: [u8; 20] = remainder[path_end+1..path_end+21].try_into().expect("array of incorrect length");

    *pos += path_end + 21;

    Ok(GitTreeLeaf {
        mode,
        path,
        hash
    })
}
