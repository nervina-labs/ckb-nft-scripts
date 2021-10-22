use crate::error::Error;
use core::result::Result;

pub const NFT_DATA_MIN_LEN: usize = 11;
pub const NFT_TYPE_ARGS_LEN: usize = 28;

/// NFT cell data structure
/// This structure contains the following information:
/// 1) version: u8
/// 2) characteristic: [u8; 8]
/// 3) configure: u8
/// 4) state: u8
#[derive(Debug, Clone)]
pub struct Nft {
    pub version:        u8,
    pub characteristic: [u8; 8],
    pub configure:      u8,
    pub state:          u8,
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

    pub fn allow_claim(&self) -> bool {
        self.configure & 0b0000_0001 == 0b0000_0000
    }

    pub fn allow_lock(&self) -> bool {
        self.configure & 0b0000_0010 == 0b0000_0000
    }

    pub fn allow_update_characteristic(&self) -> bool {
        self.configure & 0b0000_1000 == 0b0000_0000
    }

    pub fn allow_transfer_before_claim(&self) -> bool {
        self.configure & 0b0001_0000 == 0b0000_0000
    }

    pub fn allow_transfer_after_claim(&self) -> bool {
        self.configure & 0b0010_0000 == 0b0000_0000
    }

    pub fn allow_destroying_before_claim(&self) -> bool {
        self.configure & 0b0100_0000 == 0b0000_0000
    }

    pub fn allow_destroying_after_claim(&self) -> bool {
        self.configure & 0b1000_0000 == 0b0000_0000
    }

    pub fn is_claimed(&self) -> bool {
        self.state & 0b0000_0001 == 0b0000_0001
    }

    pub fn is_locked(&self) -> bool {
        self.state & 0b0000_0010 == 0b0000_0010
    }
}
