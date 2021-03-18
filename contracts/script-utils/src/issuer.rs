use alloc::vec::Vec;
use core::result::Result;

use crate::error::Error;

const ISSUER_MIN_LEN: usize = 11;
pub const ISSUER_TYPE_ARGS_LEN: usize = 20;

pub struct Issuer {
    pub version:     u8,
    pub class_count: u32,
    pub set_count:   u32,
    pub info_size:   u16,
    pub info:        Vec<u8>,
}

impl Issuer {
    pub fn from_data(data: &[u8]) -> Result<Self, Error> {
        if data.len() < ISSUER_MIN_LEN {
            return Err(Error::IssuerDataInvalid);
        }

        let mut info_size_slice = [0u8; 2];
        info_size_slice.copy_from_slice(&data[9..11]);
        let info_size = u16::from_be_bytes(info_size_slice);
        let info_slice = &data[11..];
        if (info_size as usize) != info_slice.len() {
            return Err(Error::IssuerDataInvalid);
        }
        let info = info_slice.to_vec();

        let version: u8 = data[0];
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
            info_size,
            info,
        })
    }
}
