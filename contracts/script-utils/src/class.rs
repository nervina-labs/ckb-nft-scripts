use crate::error::Error;
use crate::helper::{parse_dyn_vec_len, u32_from_slice, DYN_MIN_LEN};
use alloc::vec::Vec;
use core::result::Result;

const FIXED_LEN: usize = 10;

// FIXED_LEN + DYN_MIN_LEN * 3
const CLASS_DATA_MIN_LEN: usize = 16;
pub const CLASS_TYPE_ARGS_LEN: usize = 24;

/// Class cell data structure
/// This structure contains the following information:
/// 1) version: u8
/// 2) total: u32
/// 3) issued: u32
/// 4) configure: u8
/// 5) name: <size: u16> + <content>
/// 6) description: <size: u16> + <content>
/// 7) renderer: <size: u16> + <content>
/// 8) extinfo_data: <size: u16> + <content>
/// The fields of 1), 2), 4), 5) and 6) cannot be changed after they are set and they cannot be
/// missing. The fields of 3) and 7) can be changed and it cannot be missing.
/// The filed of 8) can be changed and it also can be missing and it will not be validated.
#[derive(Debug, Clone)]
pub struct Class {
    pub version:     u8,
    pub total:       u32,
    pub issued:      u32,
    pub configure:   u8,
    pub name:        Vec<u8>,
    pub description: Vec<u8>,
}

impl Class {
    pub fn from_data(data: &[u8]) -> Result<Self, Error> {
        if data.len() < CLASS_DATA_MIN_LEN {
            return Err(Error::ClassDataInvalid);
        }

        let version: u8 = data[0];
        if version != 0 {
            return Err(Error::VersionInvalid);
        }

        let total = u32_from_slice(&data[1..5]);
        let issued = u32_from_slice(&data[5..9]);

        if total > 0 && issued > total {
            return Err(Error::ClassTotalSmallerThanIssued);
        }

        let configure: u8 = data[9];

        let name_len = parse_dyn_vec_len(&data[FIXED_LEN..(FIXED_LEN + DYN_MIN_LEN)]);
        // DYN_MIN_LEN: the min length of description
        if data.len() < FIXED_LEN + name_len + DYN_MIN_LEN {
            return Err(Error::ClassDataInvalid);
        }
        let name = data[FIXED_LEN..(FIXED_LEN + name_len)].to_vec();

        let description_index = FIXED_LEN + name_len;
        let description_len =
            parse_dyn_vec_len(&data[description_index..(description_index + DYN_MIN_LEN)]);
        // DYN_MIN_LEN: the min length of renderer
        if data.len() < description_index + description_len + DYN_MIN_LEN {
            return Err(Error::ClassDataInvalid);
        }
        let description = data[description_index..(description_index + description_len)].to_vec();

        let renderer_index = FIXED_LEN + name_len + description_len;
        let renderer_len = parse_dyn_vec_len(&data[renderer_index..(renderer_index + DYN_MIN_LEN)]);

        if data.len() < renderer_index + renderer_len {
            return Err(Error::ClassDataInvalid);
        }

        Ok(Class {
            version,
            total,
            issued,
            configure,
            name,
            description,
        })
    }

    pub fn immutable_equal(&self, other: &Class) -> bool {
        self.total == other.total
            && self.configure == other.configure
            && self.name == other.name
            && self.description == other.description
    }
}
