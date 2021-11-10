use crate::error::Error;
use core::result::Result;

type Byte32Opt = Option<[u8; 32]>;

/// Global compact registry cell data structure
/// This structure contains the following information:
/// 1) version: u8
/// 2) registry_smt_root: [u8; 32]
#[derive(Debug, Clone)]
pub struct Registry {
    pub version:           u8,
    pub registry_smt_root: Byte32Opt,
}

impl Registry {
    pub fn from_data(data: &[u8]) -> Result<Self, Error> {
        if data.len() < 1 {
            return Err(Error::RegistryDataInvalid);
        }

        let version: u8 = data[0];
        if version != 0 {
            return Err(Error::VersionInvalid);
        }

        let registry_smt_root = if data.len() < 33 {
            None
        } else {
            let mut root = [0u8; 32];
            root.copy_from_slice(&data[1..33]);
            Some(root)
        };

        Ok(Registry {
            version,
            registry_smt_root,
        })
    }
}
