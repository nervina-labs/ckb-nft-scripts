#![allow(dead_code)]

pub const TYPE: u8 = 1;
pub const CLASS_TYPE_CODE_HASH: [u8; 32] = [
    75, 72, 142, 8, 147, 183, 171, 166, 84, 50, 130, 190, 116, 165, 142, 238, 31, 216, 3, 156, 27,
    49, 204, 36, 129, 37, 122, 215, 219, 130, 89, 245,
];

pub const COMPACT_NFT_TYPE_CODE_HASH: [u8; 32] = [
    220, 167, 40, 178, 34, 13, 64, 38, 174, 66, 149, 145, 92, 163, 223, 181, 134, 189, 247, 93,
    171, 123, 241, 75, 32, 55, 56, 153, 88, 141, 134, 137,
];

pub const BYTE32_ZEROS: [u8; 32] = [0u8; 32];
pub const BYTE22_ZEROS: [u8; 22] = [0u8; 22];
pub const BYTE4_ZEROS: [u8; 4] = [0u8; 4];
pub const BYTE3_ZEROS: [u8; 3] = [0u8; 3];

pub const OWNED_SMT_TYPE: u8 = 1u8;
pub const WITHDRAWAL_SMT_TYPE: u8 = 2u8;
pub const CLAIMED_SMT_TYPE: u8 = 3u8;
