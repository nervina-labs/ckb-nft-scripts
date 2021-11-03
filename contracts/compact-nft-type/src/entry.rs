use alloc::vec::Vec;
use ckb_std::high_level::{load_cell_data, load_cell_lock, load_input_out_point};
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, packed::*, prelude::*},
    debug,
    dynamic_loading_c_impl::CKBDLContext,
    high_level::{load_cell, load_cell_lock_hash, load_cell_type, load_script},
};
use core::result::Result;
use nft_smt::mint::MintCompactNFTValueBuilder;
use nft_smt::{
    common::{self},
    smt::blake2b_256,
    transfer::{ClaimMintCompactNFTEntries, CompactNFTKey},
};
use script_utils::{
    class::Class,
    compact_nft::CompactNft,
    error::Error,
    helper::{load_class_type_with_args, load_group_input_witness_args_with_type, ALL_ZEROS},
    smt::LibCKBSmt,
};

const TYPE_ARGS_LEN: usize = 20;
const COMPACT_NFT_ID_LEN: usize = 29;

const CLAIM_MINT: u8 = 1;
const WITHDRAW_TRANSFER: u8 = 2;
const CLAIM_TRANSFER: u8 = 3;

const MINT_RESERVED: [u8; 4] = [0u8; 4];
const ID_RESERVED: [u8; 3] = [0u8; 3];

fn check_type_args_not_equal_lock_hash(type_: &Script, source: Source) -> Result<bool, Error> {
    let lock_hash = load_cell_lock_hash(0, source)?;
    let type_args: Bytes = type_.args().unpack();
    Ok(type_args[..] != lock_hash[0..TYPE_ARGS_LEN])
}

fn check_output_compact_nft_type(compact_nft_type: &Script) -> Result<(), Error> {
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
            Ok(())
        }
        None => return Err(Error::CompactCellPositionError),
    }
}

fn check_input_compact_nft_exist(compact_nft_type: &Script) -> Result<bool, Error> {
    // If the inputs[0] is compact_nft_cell, then its type_args must be equal to
    // lock_hash[0..20].
    if let Some(type_) = load_cell_type(0, Source::Input)? {
        if compact_nft_type.as_slice() != type_.as_slice() {
            return Ok(false);
        }
        if check_type_args_not_equal_lock_hash(&type_, Source::Input)? {
            return Err(Error::CompactTypeArgsNotEqualLockHash);
        }
        return Ok(true);
    };
    Ok(false)
}

fn load_smt_root_from_class_cell_dep(nft_key: &CompactNFTKey) -> Result<[u8; 32], Error> {
    if nft_key.as_slice().len() != COMPACT_NFT_ID_LEN {
        return Err(Error::CompactIssuerIdOrClassIdInvalid);
    }
    let class_args = Bytes::from(&nft_key.as_slice()[1..25]);
    let class_cell_dep = load_cell(0, Source::CellDep)?;
    let class_type = load_class_type_with_args(&class_args);
    if let Some(dep_class_type) = class_cell_dep.type_().to_opt() {
        if dep_class_type.as_slice() == class_type.as_slice() {
            let class_data = load_cell_data(0, Source::CellDep).map_err(|_e| Error::Encoding)?;
            let class = Class::from_data(&class_data)?;
            return Ok(class.nft_smt_root.ok_or(Error::Encoding)?);
        }
        return Err(Error::CompactNFTClassDepError);
    }
    Err(Error::CompactNFTClassDepError)
}

fn validate_type_and_verify_smt(compact_nft_type: &Script) -> Result<(), Error> {
    let lock_script: Vec<Byte> = load_cell_lock(0, Source::Output)
        .map_err(|_e| Error::Encoding)?
        .as_slice()
        .iter()
        .map(|v| Byte::from(*v))
        .collect();

    let compact_nft = CompactNft::from_data(&load_cell_data(0, Source::Output)?[..])?;
    let first_input_out_point = load_input_out_point(0, Source::Input)?;
    let witness_args = load_group_input_witness_args_with_type(&compact_nft_type)?;

    let mut context = unsafe { CKBDLContext::<[u8; 128 * 1024]>::new() };
    let lib_ckb_smt = LibCKBSmt::load(&mut context);
    let mut transfer_keys: Vec<u8> = Vec::new();
    let mut transfer_values: Vec<u8> = Vec::new();
    let mut proof: Vec<u8> = Vec::new();

    if let Some(witness_args_type) = witness_args.input_type().to_opt() {
        let witness_args_input_type: Bytes = witness_args_type.unpack();
        if compact_nft.nft_smt_root.is_none() {
            return Err(Error::CompactNFTSmtRootError);
        }
        match u8::from(witness_args_input_type[0]) {
            CLAIM_MINT => {
                let claim_entries =
                    ClaimMintCompactNFTEntries::from_slice(&witness_args_input_type[1..])
                        .map_err(|_e| Error::WitnessTypeParseError)?;
                let owned_nft_keys = claim_entries.owned_nft_keys();
                let nft_key = owned_nft_keys
                    .get(0)
                    .ok_or(Error::Encoding)
                    .map_err(|_e| Error::Encoding)?;
                let class_mint_smt_root = load_smt_root_from_class_cell_dep(&nft_key)?;

                let mut mint_nft_keys: Vec<u8> = Vec::new();
                let mut mint_nft_values: Vec<u8> = Vec::new();

                for index in 0..owned_nft_keys.len() {
                    // Generate owned and claimed smt kv pairs
                    let owned_nft_key = owned_nft_keys
                        .get(index)
                        .ok_or(Error::Encoding)
                        .map_err(|_e| Error::Encoding)?;
                    let claimed_nft_key = claim_entries
                        .claimed_nft_keys()
                        .get(index)
                        .ok_or(Error::Encoding)
                        .map_err(|_e| Error::Encoding)?;

                    if &first_input_out_point.as_slice()[12..]
                        != claimed_nft_key.out_point().as_slice()
                    {
                        return Err(Error::CompactNFTOutPointInvalid);
                    }

                    transfer_keys.extend(&ID_RESERVED);
                    transfer_keys.extend(owned_nft_key.as_slice());
                    transfer_keys.extend(&blake2b_256(claimed_nft_key.as_slice()));

                    let owned_nft_value = claim_entries
                        .owned_nft_values()
                        .get(index)
                        .ok_or(Error::Encoding)
                        .map_err(|_e| Error::Encoding)?;
                    let claimed_nft_value = claim_entries
                        .claimed_nft_values()
                        .get(index)
                        .ok_or(Error::Encoding)
                        .map_err(|_e| Error::Encoding)?;
                    transfer_values.extend(&blake2b_256(owned_nft_value.as_slice()));
                    transfer_values.extend(claimed_nft_value.as_slice());

                    // Generate mint smt kv pairs
                    mint_nft_keys.extend(&MINT_RESERVED);
                    mint_nft_keys.extend(&owned_nft_key.as_slice()[1..]);

                    let lock = common::BytesBuilder::default()
                        .set(lock_script.clone())
                        .build();

                    let mint_nft_value = MintCompactNFTValueBuilder::default()
                        .nft_info(owned_nft_value)
                        .receiver_lock(lock)
                        .build();
                    mint_nft_values.extend(&blake2b_256(mint_nft_value.as_slice()))
                }

                // Verify claim smt proof of compact nft output
                proof = claim_entries.proof().raw_data().to_vec();
                if let Some(compact_smt_root) = compact_nft.nft_smt_root {
                    lib_ckb_smt
                        .smt_verify(
                            &compact_smt_root[..],
                            &transfer_keys[..],
                            &transfer_values[..],
                            &proof[..],
                        )
                        .map_err(|_| Error::SMTProofVerifyFailed)?;
                }
                // Verify mint smt proof of class cell_dep
                if !mint_nft_keys.is_empty() {
                    let mint_proof = claim_entries.mint_proof().raw_data().to_vec();
                    lib_ckb_smt
                        .smt_verify(
                            &class_mint_smt_root[..],
                            &mint_nft_keys[..],
                            &mint_nft_values[..],
                            &mint_proof[..],
                        )
                        .map_err(|_| Error::SMTProofVerifyFailed)?;
                }

                // Verify claim smt proof of compact nft input
                transfer_values.clear();
                for _ in 0..(claim_entries.owned_nft_values().len() * 2) {
                    transfer_values.extend(&ALL_ZEROS);
                }
                let input_compact_nft =
                    CompactNft::from_data(&load_cell_data(0, Source::Input)?[..])?;
                if let Some(compact_smt_root) = input_compact_nft.nft_smt_root {
                    lib_ckb_smt
                        .smt_verify(
                            &compact_smt_root[..],
                            &transfer_keys[..],
                            &transfer_values[..],
                            &proof[..],
                        )
                        .map_err(|_| Error::SMTProofVerifyFailed)?;
                }
            }
            _ => {}
        }
    }

    Ok(())
}

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let type_args: Bytes = script.args().unpack();
    if type_args.len() != TYPE_ARGS_LEN {
        return Err(Error::TypeArgsInvalid);
    }

    if !check_input_compact_nft_exist(&script)? {
        return Ok(());
    }

    check_output_compact_nft_type(&script)?;
    validate_type_and_verify_smt(&script)?;

    Ok(())
}
