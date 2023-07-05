use std::path::PathBuf;

use sha1::{Sha1, Digest};

use crate::CmdError;

pub struct Index {
    pub version: u32,
    pub items: Vec<IndexItem>
}

pub struct IndexItem {
    pub stat: libc::stat,
    pub hash: [u8; 20],
    pub flags: u16,
    pub path: PathBuf
}

fn corrupt() -> CmdError {
    CmdError::Fatal(String::from("Corrupt index"))
}

impl Index {
    pub fn deserialize(bytes: Vec<u8>) -> Result<Index, CmdError> {
        // Check the 4-byte signature
        let signature = String::from_utf8(bytes[..4].to_vec());
        if signature != Ok(String::from("DIRC")) {
            return Err(corrupt());
        }

        let version_bytes: [u8; 4] = bytes[4..8].try_into().map_err(|_| corrupt())?;
        let version = u32::from_be_bytes(version_bytes);

        let num_entries_bytes: [u8; 4] = bytes[8..12].try_into().map_err(|_| corrupt())?;
        let num_entries = u32::from_be_bytes(num_entries_bytes);

        let mut items = Vec::new();
        for i in [..num_entries] {
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

            append_u32(&mut entry_bytes, u32::try_from(item.stat.st_ctime).unwrap());
            append_u32(&mut entry_bytes, u32::try_from(item.stat.st_ctime_nsec).unwrap());
            append_u32(&mut entry_bytes, u32::try_from(item.stat.st_mtime).unwrap());
            append_u32(&mut entry_bytes, u32::try_from(item.stat.st_mtime_nsec).unwrap());
            append_u32(&mut entry_bytes, u32::try_from(item.stat.st_dev).unwrap());
            append_u32(&mut entry_bytes, u32::try_from(item.stat.st_ino).unwrap());
            append_u32(&mut entry_bytes, item.stat.st_mode);
            append_u32(&mut entry_bytes, item.stat.st_uid);
            append_u32(&mut entry_bytes, item.stat.st_gid);
            append_u32(&mut entry_bytes, u32::try_from(item.stat.st_size).unwrap());
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

fn append_string(current: &mut Vec::<u8>, val: String) {
    let mut bytes = val.into_bytes();
    current.append(&mut bytes);
}

fn append_u32(current: &mut Vec::<u8>, val: u32) {
    let mut bytes = u32::to_be_bytes(val).to_vec();
    current.append(&mut bytes);
}