use std::{path::PathBuf, fs::{self, File}, io::{Write, Read}, collections::HashMap, fmt};
use anyhow::{anyhow, bail, Result};
use flate2::{bufread::ZlibDecoder, write::ZlibEncoder, Compression};
use sha1::{Sha1, Digest};

use crate::{git_dir_name, GlobalOpts};

// All object types implement this trait which provides common functionality.
// All objects can be hashed, compressed, and written to the object store.
pub trait GitObject {
    fn type_name(&self) -> String;
    fn content_bytes(&self) -> Vec<u8>;

    fn content_with_header(&self) -> Vec<u8> {
        let content = self.content_bytes();
        let header_str = self.type_name() + " " + &self.content_bytes().len().to_string() + "\0";
        let header_bytes = header_str.as_bytes();
        let bytes = [header_bytes, &content].concat();
        bytes
    }

    fn compress(&self) -> Result<Vec<u8>> {
        let bytes = self.content_with_header();
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(&bytes).map_err(|_| anyhow!("Object compression failed"))?;
        let compressed_bytes = encoder.finish().map_err(|_| anyhow!("Object compression failed"))?;
        Ok(compressed_bytes)
    }

    fn hash(&self) -> [u8; 20] {
        let bytes = self.content_with_header();
        let mut hasher: Sha1 = Sha1::new();
        hasher.update(&bytes);
        hasher.finalize().into()
    }

    fn write(&self, repo_root: &PathBuf, global_opts: GlobalOpts) -> Result<()> {
        let hash = self.hash();
        let compressed_bytes = self.compress()?;

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
        })?;

        Ok(())
    }
}




pub struct Blob {
    pub bytes: Vec<u8>
}

impl GitObject for Blob {
    fn type_name(&self) -> String {
        String::from("blob")
    }
    fn content_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
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

impl GitObject for Commit {
    fn type_name(&self) -> String {
        String::from("commit")
    }
    fn content_bytes(&self) -> Vec<u8> {
        // TODO
        vec![0]
    }
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


#[derive(Clone, Debug)]
pub struct Tree {
    pub children: Vec<TreeEntry>
}

#[derive(Clone, Debug)]
pub struct TreeEntry {
    /// The unix file mode
    pub mode: u32,
    /// The name of the file or directory
    pub name: String,
    /// The SHA1 hash of the file contents
    pub hash: [u8; 20]
}

impl GitObject for Tree {
    fn type_name(&self) -> String {
        String::from("tree")
    }
    fn content_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for child in &self.children {
            // Convert mode from integer to an ASCII representation of the octal value
            let mode_str = format!("{:o}", child.mode);
            let mut mode = mode_str.as_bytes().to_vec();
            let mut name = child.name.as_bytes().to_vec();
            let mut hash = child.hash.to_vec();

            bytes.append(&mut mode);
            bytes.push(b' ');
            bytes.append(&mut name);
            bytes.push(b'\0');
            bytes.append(&mut hash);
        }

        bytes
    }
}


pub struct Tag {
    name: String
}

impl GitObject for Tag {
    fn type_name(&self) -> String {
        String::from("tag")
    }
    fn content_bytes(&self) -> Vec<u8> {
        self.name.as_bytes().to_vec()
    }
}


// To pass around an object of unknown type, use this enum.
pub enum Object {
    Blob(Blob),
    Commit(Commit),
    Tree(Tree),
    Tag(Tag)
}

impl GitObject for Object {
    fn type_name(&self) -> String {
        match self {
            Object::Blob(x) => x.type_name(),
            Object::Commit(x) => x.type_name(),
            Object::Tree(x) => x.type_name(),
            Object::Tag(x) => x.type_name(),
        }
    }

    fn content_bytes(&self) -> Vec<u8> {
        match self {
            Object::Blob(x) => x.content_bytes(),
            Object::Commit(x) => x.content_bytes(),
            Object::Tree(x) => x.content_bytes(),
            Object::Tag(x) => x.content_bytes(),
        }
    }
}

/// Attempts to interpret the given string as a 20-byte SHA1 hash
pub fn parse_hash(hash: &String) -> Result<[u8; 20]> {
    let bytes = hex::decode(hash)?;
    let result: [u8; 20] = bytes.as_slice().try_into()?;
    Ok(result)
}

pub fn search_object(root: &PathBuf, hash: &[u8; 20], git_mode: bool) -> Result<Option<Object>> {
    match read_object_raw(root, hash, git_mode) {
        Ok(Some(bytes)) => {
            let type_end = bytes.iter().position(|x| x == &b' ')
                .ok_or(anyhow!("error parsing object: `type` field not terminated"))?;

            let file_size_end = (type_end + 1) + bytes[type_end+1..].iter().position(|x| x == &0)
                .ok_or(anyhow!("error parsing object: `size` field not terminated"))?;

            let object_type = &bytes[..type_end];
            let _file_size = &bytes[type_end+1..file_size_end];
            let contents = &bytes[file_size_end+1..];

            match object_type {
                b"blob" => Ok(Some(Object::Blob(Blob { bytes: contents.to_vec() }))),
                b"tree" => {
                    match parse_tree(contents) {
                        Ok(t) => Ok(Some(Object::Tree(t))),
                        Err(e) => Err(e)
                    }
                }
                b"tag" => Ok(Some(Object::Tag(Tag { name: String::from("TODO: Read name")}))),
                b"commit" => {
                    match parse_commit(&String::from_utf8_lossy(&contents).to_string()) {
                        Ok(c) => Ok(Some(Object::Commit(c))),
                        Err(e) => Err(e)
                    }
                }
                _ => bail!("unrecognised object type")
            }
        },
        Ok(None) => Ok(None),
        Err(e) => Err(e)
    }
}

/// Retrieves the object with the given hash from the store, or an Err if it doesn't exist.
/// Use this when the object is referenced by a different object, so it's absence suggests the store is corrupted.
pub fn get_object(root: &PathBuf, hash: &[u8; 20], git_mode: bool) -> Result<Object> {
    match search_object(root, hash, git_mode) {
        Ok(Some(x)) => Ok(x),
        Ok(None) => bail!("Object {} not found in store", String::from_utf8_lossy(hash)),
        Err(e) => Err(e)
    }
}


// Returns the decompressed contents of the object with the given hash, or None
// if the object does not exist, or an error if the object exists but decompression fails
pub fn read_object_raw(root: &PathBuf, hash: &[u8; 20], git_mode: bool) -> Result<Option<Vec<u8>>> {
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
    let bytes = fs::read(full_path)?;
    let mut z = ZlibDecoder::new(&bytes[..]);
    
    let mut buf = Vec::<u8>::new();
    z.read_to_end(&mut buf)?;

    Ok(Some(buf))
}

enum ParseState {
    BeforeKey,
    InKey,
    BeforeValue,
    InValue,
    InMessage
}

pub fn parse_commit(commit_text: &String) -> Result<Commit> {
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
                    '\n' => bail!("unexpected new line in commit text"),
                    c if c.is_whitespace() => continue,
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
                            bail!("invalid commit text");
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

fn parse_tree(bytes: &[u8]) -> Result<Tree> {
    let mut nodes = Vec::new();
    let mut pos: usize = 0;
    let max = bytes.len();
    
    while pos < max {
        let node = parse_tree_node(bytes, &mut pos)?; 
        nodes.push(node);
    }

    Ok(Tree { children: nodes })
}

fn parse_tree_node(bytes: &[u8], pos: &mut usize) -> Result<TreeEntry> {
    let remainder = &bytes[*pos..];

    // Find the space that terminates the file mode
    let mode_end = remainder.iter().position(|x| x == &b' ')
        .ok_or(anyhow!(
            "error parsing tree: missing space terminator for file mode"
        ))?;

    // Read the mode
    let mode = u32::from_be_bytes(remainder[..mode_end].try_into().unwrap());

    // Find the NULL terminator of the path
    let path_end = remainder.iter().position(|x| x == &0)
        .ok_or(anyhow!(
            "error parsing tree: missing NULL terminator for path"
        ))?;

    let path_str = String::from_utf8(remainder[mode_end+1..path_end].to_vec())
        .map_err(|_| anyhow!( 
            "error parsing tree: non-UTF8 character in path"
        ))?;
    let hash: [u8; 20] = remainder[path_end+1..path_end+21].try_into().expect("array of incorrect length");

    *pos += path_end + 21;

    Ok(TreeEntry {
        mode,
        name: path_str,
        hash
    })
}