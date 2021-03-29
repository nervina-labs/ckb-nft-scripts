use crate::error::Error;
use core::result::Result;

const NFT_DATA_MIN_LEN: usize = 11;
pub const NFT_TYPE_ARGS_LEN: usize = 28;

/// NFT cell data structure
/// This structure contains the following information:
/// 1) version: u8
/// 2) characteristic: [u8; 8]
/// 3) configure: u8
/// 4) state: u8
/// 5) extinfo_data: <size: u16> + <vartext>
/// The filed of 8) can be changed and it also can be missing and it will not be validated.
pub struct Nft {
    pub version:     u8,
    pub characteristic: [u8; 8],
    pub configure:   u8,
    pub state: u8,
}

impl Nft {
    pub fn from_data(data: &[u8]) -> Result<Self, Error> {
        if data.len() < NFT_DATA_MIN_LEN {
            return Err(Error::NFTDataInvalid);
        }

        let version: u8 = data[0];
        if version != 0 {
            return Err(Error::VersionInvalid);
        }

        let mut characteristic = [0u8; 8];
        characteristic.copy_from_slice(&data[1..9]);

        let configure: u8 = data[9];
        let state: u8 = data[10];

        Ok(Nft {
            version,
            characteristic,
            configure,
            state,
        })
    }
}
