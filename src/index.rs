use std::path::PathBuf;

use sha1::{Sha1, Digest};

use crate::CmdError;

pub struct Index {
    pub version: u32,
    pub items: Vec<IndexItem>
}

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

fn corrupt() -> CmdError {
    CmdError::Fatal(String::from("Corrupt index"))
}

impl Index {
    pub fn deserialize(bytes: Vec<u8>) -> Result<Index, CmdError> {
        let signature = String::from_utf8(bytes[..4].to_vec());
        if signature != Ok(String::from("DIRC")) {
            return Err(corrupt());
        }
        
        let mut pos = 4;
        let version = read_u32(&bytes, &mut pos);
        let num_entries = read_u32(&bytes, &mut pos);

        let mut items = Vec::new();
        for _ in [..num_entries] {
            let ctime = read_u32(&bytes, &mut pos);
            let ctime_nsec = read_u32(&bytes, &mut pos);
            let mtime = read_u32(&bytes, &mut pos);
            let mtime_nsec = read_u32(&bytes, &mut pos);
            let dev = read_u32(&bytes, &mut pos);
            let ino = read_u32(&bytes, &mut pos);
            let mode = read_u32(&bytes, &mut pos);
            let uid = read_u32(&bytes, &mut pos);
            let gid = read_u32(&bytes, &mut pos);
            let size = read_u32(&bytes, &mut pos);
            let hash = read_hash(&bytes, &mut pos);

            let flags = u16::from_be_bytes(bytes[pos..(pos+2)].try_into().unwrap());
            pos += 2;

            let path_len: usize = (0xFFF & flags).into();
            let path_bytes = bytes[pos..(pos+path_len)].into();
            let path = PathBuf::from(String::from_utf8_lossy(path_bytes).to_string());

            let item = IndexItem {
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
            };
            items.push(item);
        }

        Ok(Index{version, items})
    }
    

    pub fn serialize(&self) -> Result<Vec<u8>, CmdError> {
        let mut bytes = Vec::<u8>::new();

        append_string(&mut bytes, String::from("DIRC"));
        append_u32(&mut bytes, self.version.try_into().unwrap());

        let num_entries = self.items.len()
            .try_into()
            .map_err(|_| CmdError::Fatal(String::from("You've staged > 4 billion files. Are you okay?")))?;
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

fn read_u32(bytes: &Vec<u8>, pos: &mut usize) -> u32 {
    let val = u32::from_be_bytes(bytes[*pos..(*pos+4)].try_into().unwrap());
    *pos += 4;
    val
}

fn read_hash(bytes: &Vec<u8>, pos: &mut usize) -> [u8; 20] {
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