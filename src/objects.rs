// This module should encapsulate knowledge about how objects are represented in Git.
// Callers should only need to know that objects are identified by a hash.

use std::{path::PathBuf, fs, io::Read, collections::HashMap};

use flate2::bufread::ZlibDecoder;

use crate::CmdError;

pub enum Object {
    Blob,
    Commit(Commit),
    Tree(Vec<GitTreeLeaf>),
    Tag
}

pub struct Commit {
    pub tree: String,
    pub author: String,
    pub committer: String,
    pub date: Option<String>,
    pub parent: Option<String>,
    pub message: String,
}

pub struct GitTreeLeaf {
    pub mode: String,
    pub path: String,
    pub hash: String
}

pub fn search_object(root: &PathBuf, hash: &String, git_mode: bool) -> Result<Option<Object>, CmdError> {
    match read_object_raw(root, hash, git_mode) {
        Ok(Some(all_text)) => {
            let (object_type, contents_with_header) = all_text.split_once(' ')
                .ok_or(CmdError::Fatal(String::from("malformed object")))?;

            let (_file_size, contents) = contents_with_header.split_once('\u{0}')
                .ok_or(CmdError::Fatal(String::from("malformed object")))?;

            match object_type {
                "blob" => Ok(Some(Object::Blob)),
                "tree" => {
                    match parse_tree(&contents.to_string()) {
                        Ok(t) => Ok(Some(Object::Tree(t))),
                        Err(e) => Err(e)
                    }
                }
                "tag" => Ok(Some(Object::Tag)),
                "commit" => {
                    match parse_commit(&contents.to_string()) {
                        Ok(c) => Ok(Some(Object::Commit(c))),
                        Err(e) => Err(e)
                    }
                }
                x => Err(CmdError::Fatal(format!("unrecognised object type {}", x)))
            }
        },
        Ok(None) => Ok(None),
        Err(e) => Err(e)
    }
}

/// Retrieves the object with the given hash from the store, or an Err if it doesn't exist.
/// Use this when the object is referenced by a different object, so it's absence suggests the store is corrupted.
pub fn get_object(root: &PathBuf, hash: &String, git_mode: bool) -> Result<Object, CmdError> {
    match search_object(root, hash, git_mode) {
        Ok(Some(x)) => Ok(x),
        Ok(None) => Err(CmdError::Fatal(format!("Object {} not found in store", hash))),
        Err(e) => Err(e)
    }
}


// Returns the decompressed contents of the object with the given hash, or None
// if the object does not exist, or an error if the object exists but decompression fails
pub fn read_object_raw(root: &PathBuf, hash: &String, git_mode: bool) -> Result<Option<String>, CmdError> {
    if hash.len() < 3 {
        return Ok(None);
    }

    let git_dir = if git_mode { ".git" } else { ".grit" };

    let full_path = root.join(format!("{}/objects/{}/{}", git_dir, &hash[..2], &hash[2..]));
    println!("Looking for {}", full_path.to_string_lossy().to_string());
    if !full_path.exists() {
        return Ok(None);
    }

    // Read and decompress the requested file
    let bytes = fs::read(full_path).map_err(CmdError::IOError)?;
    let mut z = ZlibDecoder::new(&bytes[..]);
    let mut s = String::new();
    
    z.read_to_string(&mut s).map_err(CmdError::IOError)?;
    Ok(Some(s))
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

    // TODO: Investigate better ways of doing this. Macros?
    Ok(Commit {
        author: tags.get("author").unwrap().to_string(),
        committer: tags.get("committer").unwrap().to_string(),
        date: tags.get("date").cloned(),
        parent: tags.get("parent").cloned(),
        tree: tags.get("tree").unwrap().to_string(),
        message: message,
    })
}

fn parse_tree(tree_text: &String) -> Result<Vec<GitTreeLeaf>, CmdError> {
    let mut leaves = Vec::new();
    for line in tree_text.lines() {
        if let Ok(leaf) = parse_tree_leaf(&line.to_string()) {
            leaves.push(leaf);
        } else {
            return Err(CmdError::Fatal(String::from("Failed to parse tree")));
        }
    }

    Ok(leaves)
}

fn parse_tree_leaf(text: &String) -> Result<GitTreeLeaf, String> {
    let (mode, rest) = text.split_once(' ').ok_or(String::from("malformed tree object"))?;
    let (_object_type, rest) = rest.split_once(' ').ok_or(String::from("malformed tree object"))?;
    let (hash, rest) = rest.split_once(' ').ok_or(String::from("malformed tree object"))?;
    let path = rest.trim();

    Ok(GitTreeLeaf {
        mode: mode.to_string(),
        path: path.to_string(),
        hash: hash.to_string()
    })
}
