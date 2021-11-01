use ckb_std::high_level::load_cell_data;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, packed::*, prelude::*},
    high_level::{load_cell, load_cell_lock_hash, load_cell_type, load_script},
};
use core::result::Result;
use nft_smt::transfer::{ClaimCompactNFTEntries, CompactNFTId};
use script_utils::compact_nft::CompactNft;
use script_utils::{
    error::Error,
    helper::{load_class_type_with_args, load_group_input_witness_args_with_type},
};

const TYPE_ARGS_LEN: usize = 20;
const MINT_CLAIM: u8 = 1;
const TRANSFER_WITHDRAW: u8 = 2;
const TRANSFER_CLAIM: u8 = 3;
const COMPACT_NFT_ID_LEN: usize = 29;

fn check_type_args_not_equal_lock_hash(type_: &Script, source: Source) -> Result<bool, Error> {
    let lock_hash = load_cell_lock_hash(0, source)?;
    let type_args: Bytes = type_.args().unpack();
    Ok(type_args[..] != lock_hash[0..TYPE_ARGS_LEN])
}

fn check_class_cell_dep(nft_id: &CompactNFTId) -> Result<(), Error> {
    if nft_id.as_slice().len() != COMPACT_NFT_ID_LEN {
        return Err(Error::CompactIssuerIdOrClassIdInvalid);
    }
    let class_args = Bytes::from(&nft_id.as_slice()[1..25]);
    let class_cell_dep = load_cell(0, Source::CellDep)?;
    let class_type = load_class_type_with_args(&class_args);
    if let Some(dep_class_type) = class_cell_dep.type_().to_opt() {
        if dep_class_type.as_slice() == class_type.as_slice() {
            return Ok(());
        }
        return Err(Error::CompactNFTClassDepError);
    }
    Err(Error::CompactNFTClassDepError)
}

fn validate_type_and_verify_smt(compact_nft_type: &Script) -> Result<(), Error> {
    // Outputs[0] must be compact_nft_cell whose type_args must be equal the lock_hash[0..20]
    let output_compact_type = load_cell_type(0, Source::Output).map_err(|_e| Error::Encoding)?;
    match output_compact_type {
        Some(type_) => {
            if compact_nft_type.as_slice() != type_.as_slice() {
                return Err(Error::CompactCellPositionError);
            }
            if check_type_args_not_equal_lock_hash(&type_, Source::Output)? {
                return Err(Error::CompactTypeArgsNotEqualLockHash);
            }
        }
        None => return Err(Error::CompactCellPositionError),
    }

    let compact_nft = CompactNft::from_data(&load_cell_data(0, Source::Output)?[..])?;
    let witness_args = load_group_input_witness_args_with_type(&compact_nft_type)?;
    if let Some(witness_args_type) = witness_args.input_type().to_opt() {
        let witness_args_input_type: Bytes = witness_args_type.unpack();
        if compact_nft.nft_smt_root.is_none() {
            return Err(Error::CompactNFTSmtRootError);
        }
        match u8::from(witness_args_input_type[0]) {
            MINT_CLAIM => {
                let claim_entries =
                    ClaimCompactNFTEntries::from_slice(&witness_args_input_type[1..])
                        .map_err(|_e| Error::WitnessTypeParseError)?;
                let owned_nft_ids = claim_entries.owned_nft_ids();
                let nft_id = owned_nft_ids
                    .get(0)
                    .ok_or(Error::Encoding)
                    .map_err(|_e| Error::Encoding)?;
                check_class_cell_dep(&nft_id)?;
            }
            _ => {}
        }
    }

    // If the inputs[0] is compact_nft_cell, then its type_args must be equal to
    // lock_hash[0..20] and its lock script must be equal to outputs.compact_nft_cell.
    if let Some(type_) = load_cell_type(0, Source::Input)? {
        if compact_nft_type.as_slice() != type_.as_slice() {
            return Ok(());
        }
        if check_type_args_not_equal_lock_hash(&type_, Source::Input)? {
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

    validate_type_and_verify_smt(&script)?;

    Ok(())
}
