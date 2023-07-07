use std::path::PathBuf;
use anyhow::Result;
use sha1::{Sha1, Digest};

pub struct Index {
    pub version: u32,

    // These should be stored in ascending order on the name field.
    // Entries with the same name are sorted by their stage field.
    pub items: Vec<IndexItem>
}

#[derive(Clone)]
pub struct IndexItem {
    pub ctime: u32,
    pub ctime_nsec: u32,
    pub mtime: u32,
    pub mtime_nsec: u32,
    pub dev: u32,
    pub ino: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u32,
    pub hash: [u8; 20],
    pub path: PathBuf
}

impl Index {
    pub fn deserialize(bytes: Vec<u8>) -> Result<Index> {
        let signature = String::from_utf8(bytes[..4].to_vec())?;
        let mut pos = 4;
        let version = read_u32(&bytes, &mut pos);
        let num_entries = read_u32(&bytes, &mut pos);

        let mut items = Vec::new();
        for _ in 0..num_entries {
            let mut item_pos = 0;
            let item_bytes = &bytes[pos..];
            let ctime = read_u32(item_bytes, &mut item_pos);
            let ctime_nsec = read_u32(item_bytes, &mut item_pos);
            let mtime = read_u32(item_bytes, &mut item_pos);
            let mtime_nsec = read_u32(item_bytes, &mut item_pos);
            let dev = read_u32(item_bytes, &mut item_pos);
            let ino = read_u32(item_bytes, &mut item_pos);
            let mode = read_u32(item_bytes, &mut item_pos);
            let uid = read_u32(item_bytes, &mut item_pos);
            let gid = read_u32(item_bytes, &mut item_pos);
            let size = read_u32(item_bytes, &mut item_pos);
            let hash = read_hash(item_bytes, &mut item_pos);

            let flags = u16::from_be_bytes(item_bytes[item_pos..(item_pos+2)].try_into().unwrap());
            item_pos += 2;

            let path_len: usize = (0xFFF & flags).into();
            let path_bytes = item_bytes[item_pos..(item_pos+path_len)].into();
            let path_str = String::from_utf8_lossy(path_bytes).to_string();
            let path = PathBuf::from(&path_str);
            item_pos += path_len;

            // Shift pos to account for NUL-padding of path name
            let npad = 8 - ((item_pos) % 8);
            let item_len = item_pos + npad;
            pos += item_len;

            items.push(IndexItem {
                ctime,
                ctime_nsec,
                mtime,
                mtime_nsec,
                dev,
                ino,
                mode,
                uid,
                gid,
                size,
                hash,
                path
            });
        }

        Ok(Index{version, items})
    }
    

    pub fn serialize(&self) -> Result<Vec<u8>> {
        let mut bytes = Vec::<u8>::new();

        append_string(&mut bytes, String::from("DIRC"));
        append_u32(&mut bytes, self.version.try_into().unwrap());

        let num_entries = self.items.len().try_into()?;
        append_u32(&mut bytes, num_entries);

        for item in &self.items {
            let mut entry_bytes = Vec::<u8>::new();

            append_u32(&mut entry_bytes, u32::try_from(item.ctime).unwrap());
            append_u32(&mut entry_bytes, u32::try_from(item.ctime_nsec).unwrap());
            append_u32(&mut entry_bytes, u32::try_from(item.mtime).unwrap());
            append_u32(&mut entry_bytes, u32::try_from(item.mtime_nsec).unwrap());
            append_u32(&mut entry_bytes, u32::try_from(item.dev).unwrap());
            append_u32(&mut entry_bytes, u32::try_from(item.ino).unwrap());
            append_u32(&mut entry_bytes, item.mode);
            append_u32(&mut entry_bytes, item.uid);
            append_u32(&mut entry_bytes, item.gid);
            append_u32(&mut entry_bytes, u32::try_from(item.size).unwrap());
            entry_bytes.append(&mut item.hash.into());

            let path_str = item.path.to_string_lossy();
            let path_bytes = path_str.as_bytes();

            // TODO: Handle "assume-valid" flag
            let flags: u16 = std::cmp::min(0xFFF, path_bytes.len()).try_into().unwrap();
            entry_bytes.append(&mut u16::to_be_bytes(flags).to_vec());
            entry_bytes.append(&mut path_bytes.into());

            // Pad with 1-8 NUL bytes so total length is a multiple of 8.
            let npad = 8 - (entry_bytes.len() % 8);
            entry_bytes.append(&mut vec![0; npad]);

            bytes.append(&mut entry_bytes);
        }

        // Extension data goes here

        // Append checksum
        let mut hasher: Sha1 = Sha1::new();
        hasher.update(&bytes);
        let checksum: [u8; 20] = hasher.finalize().into();
        bytes.append(&mut checksum.to_vec());

        Ok(bytes)
    }
}

fn read_u32(bytes: &[u8], pos: &mut usize) -> u32 {
    let val = u32::from_be_bytes(bytes[*pos..(*pos+4)].try_into().unwrap());
    *pos += 4;
    val
}

fn read_hash(bytes: &[u8], pos: &mut usize) -> [u8; 20] {
    let val: [u8; 20] = bytes[*pos..(*pos+20)].try_into().unwrap();
    *pos += 20;
    val
}

fn append_string(current: &mut Vec::<u8>, val: String) {
    let mut bytes = val.into_bytes();
    current.append(&mut bytes);
}

fn append_u32(current: &mut Vec::<u8>, val: u32) {
    let mut bytes = u32::to_be_bytes(val).to_vec();
    current.append(&mut bytes);
}