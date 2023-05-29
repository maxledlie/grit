// This module should encapsulate knowledge about how objects are represented in Git.
// Callers should only need to know that objects are identified by a hash.

use std::{path::Path, fs, io::Read, collections::HashMap};

use flate2::bufread::ZlibDecoder;

// Returns the decompressed contents of the object with the given hash, or None
// if the object does not exist.
pub fn read_text(root: &Path, hash: &String, git_mode: bool) -> Option<String> {
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

enum ParseState {
    BeforeKey,
    InKey,
    BeforeValue,
    InValue,
    InMessage
}

pub fn parse_commit(commit_text: &String) -> Result<Commit, String> {
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
                        return Err(String::from("unexpected new line in commit text"));
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
                            return Err(String::from("invalid commit text"));
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
        tree: tags.get("tree").cloned(),
        message: message,
    })
}

pub struct Commit {
    pub tree: Option<String>,
    pub author: String,
    pub committer: String,
    pub date: Option<String>,
    pub parent: Option<String>,
    pub message: String,
}