use crate::error::Error;
use core::result::Result;

const ISSUER_MIN_LEN: usize = 11;
pub const ISSUER_TYPE_ARGS_LEN: usize = 20;

pub struct Issuer {
    pub version:     u8,
    pub class_count: u32,
    pub set_count:   u32,
}

impl Issuer {
    pub fn from_data(data: &[u8]) -> Result<Self, Error> {
        if data.len() < ISSUER_MIN_LEN {
            return Err(Error::IssuerDataInvalid);
        }

        let version: u8 = data[0];
        if version != 0 {
            return Err(Error::VersionInvalid);
        }

        let mut class_count_slice = [0u8; 4];
        let mut set_count_slice = [0u8; 4];
        class_count_slice.copy_from_slice(&data[1..5]);
        set_count_slice.copy_from_slice(&data[5..9]);
        let class_count = u32::from_be_bytes(class_count_slice);
        let set_count = u32::from_be_bytes(set_count_slice);

        Ok(Issuer {
            version,
            class_count,
            set_count,
        })
    }
}
