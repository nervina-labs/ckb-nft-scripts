use crate::error::Error;
use crate::helper::{parse_dyn_vec_len, u32_from_slice, DYN_MIN_LEN};
use core::result::Result;

const FIXED_LEN: usize = 9;
// FIXED_LEN + DYN_MIN_LEN
const ISSUER_DATA_MIN_LEN: usize = 11;
pub const ISSUER_TYPE_ARGS_LEN: usize = 20;

/// Issuer cell data structure
/// This structure contains the following information:
/// 1) version: u8
/// 2) class_count: u32
/// 3) set_count: u32
/// 4) info: <size: u16> + <content>
#[derive(Debug, Clone)]
pub struct Issuer {
    pub version:     u8,
    pub class_count: u32,
    pub set_count:   u32,
}

impl Issuer {
    pub fn from_data(data: &[u8]) -> Result<Self, Error> {
        if data.len() < ISSUER_DATA_MIN_LEN {
            return Err(Error::IssuerDataInvalid);
        }

        let version: u8 = data[0];
        if version != 0 {
            return Err(Error::VersionInvalid);
        }

        let class_count = u32_from_slice(&data[1..5]);
        let set_count = u32_from_slice(&data[5..FIXED_LEN]);

        let info_len = parse_dyn_vec_len(&data[FIXED_LEN..(FIXED_LEN + DYN_MIN_LEN)]);
        if data.len() < info_len + FIXED_LEN {
            return Err(Error::IssuerDataInvalid);
        }

        Ok(Issuer {
            version,
            class_count,
            set_count,
        })
    }
}
