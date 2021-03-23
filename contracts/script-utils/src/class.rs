use crate::error::Error;
use core::result::Result;

const CLASS_MIN_LEN: usize = 16;
pub const CLASS_TYPE_ARGS_LEN: usize = 52;

pub struct Class {
    pub version:   u8,
    pub total:     u32,
    pub issued:    u32,
    pub configure: u8,
}

impl Class {
    pub fn from_data(data: &[u8]) -> Result<Self, Error> {
        if data.len() < CLASS_MIN_LEN {
            return Err(Error::ClassDataInvalid);
        }

        let version: u8 = data[0];
        if version != 0 {
            return Err(Error::VersionInvalid);
        }

        let mut total_list = [0u8; 4];
        let mut issued_list = [0u8; 4];
        total_list.copy_from_slice(&data[1..5]);
        issued_list.copy_from_slice(&data[5..9]);
        let total = u32::from_be_bytes(total_list);
        let issued = u32::from_be_bytes(issued_list);

        if total > 0 && issued >= total {
            return Err(Error::ClassTotalSmallerThanIssued);
        }

        let configure: u8 = data[9];

        Ok(Class {
            version,
            total,
            issued,
            configure,
        })
    }
}
