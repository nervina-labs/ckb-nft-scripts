use super::claim_mint::verify_claim_mint_smt;
use super::withdraw_transfer::verify_withdraw_transfer_smt;
use crate::claim_transfer::verify_claim_transfer_smt;
use ckb_std::high_level::load_cell_data;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, packed::*, prelude::*},
    high_level::{load_cell_lock_hash, load_cell_type, load_script},
};
use core::result::Result;
use script_utils::helper::{check_compact_nft_cells_only_one, check_registry_cells_exist};
use script_utils::{
    compact_nft::CompactNft, error::Error, helper::load_group_input_witness_args_with_type,
};

const TYPE_ARGS_LEN: usize = 20;

const CLAIM_MINT: u8 = 1;
const WITHDRAW_TRANSFER: u8 = 2;
const CLAIM_TRANSFER: u8 = 3;

fn check_type_args_not_equal_lock_hash(type_: &Script, source: Source) -> Result<bool, Error> {
    let lock_hash = load_cell_lock_hash(0, source)?;
    let type_args: Bytes = type_.args().unpack();
    Ok(type_args[..] != lock_hash[0..TYPE_ARGS_LEN])
}

fn check_output_compact_nft_type(compact_nft_type: &Script) -> Result<(), Error> {
    match load_cell_type(0, Source::GroupOutput).map_err(|_e| Error::CompactCellPositionError)? {
        Some(type_) => {
            if compact_nft_type.as_slice() != type_.as_slice() {
                return Err(Error::CompactCellPositionError);
            }
            if check_type_args_not_equal_lock_hash(&type_, Source::Output)? {
                return Err(Error::CompactTypeArgsNotEqualLockHash);
            }
            Ok(())
        }
        None => return Err(Error::CompactCellPositionError),
    }
}

fn check_registry_cells() -> Result<(), Error> {
    if !check_registry_cells_exist()? {
        return Err(Error::CompactRegistryCellExistError);
    }
    let output_compact_nft_data = load_cell_data(0, Source::GroupOutput)?;
    // Compact nft cell data only has version
    if output_compact_nft_data.len() != 1 {
        return Err(Error::CompactNFTDataInvalid);
    }
    Ok(())
}

fn check_input_compact_nft_exist(compact_nft_type: &Script) -> Result<bool, Error> {
    if let Ok(input_compact_nft_type) = load_cell_type(0, Source::GroupInput) {
        if let Some(type_) = input_compact_nft_type {
            if compact_nft_type.as_slice() != type_.as_slice() {
                return Ok(false);
            }
            if check_type_args_not_equal_lock_hash(&type_, Source::Input)? {
                return Err(Error::CompactTypeArgsNotEqualLockHash);
            }
            return Ok(true);
        };
        return Ok(false);
    }
    Ok(false)
}

fn verify_compact_nft_smt(compact_nft_type: &Script) -> Result<(), Error> {
    if !check_compact_nft_cells_only_one(compact_nft_type) {
        return Err(Error::CompactNFTCellsCountError);
    }
    let witness_args = load_group_input_witness_args_with_type(&compact_nft_type)?;

    if let Some(witness_args_type) = witness_args.input_type().to_opt() {
        let witness_args_input_type: Bytes = witness_args_type.unpack();
        let output_compact_nft = CompactNft::from_data(&load_cell_data(0, Source::Output)?[..])?;
        if output_compact_nft.nft_smt_root.is_none() {
            return Err(Error::CompactNFTSMTRootError);
        }
        match u8::from(witness_args_input_type[0]) {
            CLAIM_MINT => verify_claim_mint_smt(witness_args_input_type)?,
            WITHDRAW_TRANSFER => verify_withdraw_transfer_smt(witness_args_input_type)?,
            CLAIM_TRANSFER => verify_claim_transfer_smt(compact_nft_type, witness_args_input_type)?,
            _ => return Err(Error::WitnessTypeParseError),
        }
    } else {
        return Err(Error::WitnessTypeParseError);
    }

    Ok(())
}

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let type_args: Bytes = script.args().unpack();
    if type_args.len() != TYPE_ARGS_LEN {
        return Err(Error::TypeArgsInvalid);
    }

    check_output_compact_nft_type(&script)?;

    if check_input_compact_nft_exist(&script)? {
        verify_compact_nft_smt(&script)?;
    } else {
        check_registry_cells()?;
    }

    Ok(())
}
