use alloc::vec::Vec;
use ckb_std::high_level::load_cell_data;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, packed::*, prelude::*},
    high_level::{load_cell, load_cell_lock_hash, load_cell_type, load_script},
};
use core::result::Result;
use nft_smt::transfer::{ClaimCompactNFTEntries, ClaimCompactNFTEntriesBuilder};
use script_utils::compact_nft::CompactNft;
use script_utils::{
    error::Error,
    helper::{load_class_type, load_group_input_witness_args_with_type},
};

const TYPE_ARGS_LEN: usize = 20;
const MINT_CLAIM: u8 = 1;
const TRANSFER_WITHDRAW: u8 = 2;
const TRANSFER_CLAIM: u8 = 3;

fn check_type_args_not_equal_lock_hash(type_: &Script, source: Source) -> bool {
    let lock_hash = load_cell_lock_hash(0, source)?;
    let type_args: Bytes = type_.args().unpack();
    type_args[..] != lock_hash[0..TYPE_ARGS_LEN]
}

fn check_type_args_equal_lock_hash(compact_nft_type: &Script) -> Result<(), Error> {
    // Outputs[0] must be compact_nft_cell whose type_args must be equal the lock_hash[0..20]
    match load_cell_type(0, Source::Output)? {
        Some(type_) => {
            if compact_nft_type.as_slice() != type_.as_slice() {
                return Err(Error::CompactCellPositionError);
            }
            if check_type_args_not_equal_lock_hash(&type_, Source::Output) {
                return Err(Error::CompactTypeArgsNotEqualLockHash);
            }
        }
        None => return Err(Error::CompactCellPositionError),
    }

    // If the inputs[0] is compact_nft_cell, then its type_args must be equal to
    // lock_hash[0..20] and its lock script must be equal to outputs.compact_nft_cell.
    if let Some(type_) = load_cell_type(0, Source::Input)? {
        if compact_nft_type.as_slice() != type_.as_slice() {
            return Ok(());
        }
        if check_type_args_not_equal_lock_hash(&type_, Source::Input) {
            return Err(Error::CompactTypeArgsNotEqualLockHash);
        }
    };

    Ok(())
}

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let type_args: Bytes = script.args().unpack();
    if type_args.len() != TYPE_ARGS_LEN {
        return Err(Error::TypeArgsInvalid);
    }

    check_type_args_equal_lock_hash(&script)?;

    let compact_nft = CompactNft::from_data(&load_cell_data(0, Source::Output)?[..])?;
    let witness_args = load_group_input_witness_args_with_type(&script)?;
    if let Some(witness_args_type) = witness_args.input_type().to_opt() {
        if compact_nft.nft_smt_root.is_none() {
            return Err(Error::CompactNFTSmtRootError);
        }
        match u8::from(witness_args_type[0]) {
            MINT_CLAIM => {
                let _claim_entries = ClaimCompactNFTEntries::from_slice(witness_args_type[1..])
                    .map_err(|_e| Error::WitnessTypeParseError)?;
                Ok(())
            }
            _ => {}
        }
    }

    Ok(())
}
