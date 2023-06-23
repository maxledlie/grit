use crate::CmdError;

pub struct Index {
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
        dbg!(version);

        let num_entries_bytes: [u8; 4] = bytes[8..12].try_into().map_err(|_| corrupt())?;
        let num_entries = u32::from_be_bytes(num_entries_bytes);
        dbg!(num_entries);


        Ok(Index{})
    }
}