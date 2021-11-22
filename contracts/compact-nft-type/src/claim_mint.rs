use alloc::vec::Vec;
use ckb_std::high_level::{load_cell_data, load_cell_lock, load_input_out_point};
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, packed::*, prelude::*},
    dynamic_loading_c_impl::CKBDLContext,
    high_level::load_cell,
};
use core::result::Result;
use nft_smt::mint::MintCompactNFTValueBuilder;
use nft_smt::{
    common::{self},
    smt::blake2b_256,
    transfer::{ClaimMintCompactNFTEntries, CompactNFTKey},
};
use script_utils::constants::{CLAIMED_SMT_TYPE, OWNED_SMT_TYPE};
use script_utils::{
    class::Class,
    compact_nft::CompactNft,
    constants::{BYTE22_ZEROS, BYTE32_ZEROS, BYTE3_ZEROS, BYTE4_ZEROS},
    error::Error,
    helper::load_class_type_with_args,
    smt::LibCKBSmt,
};

const COMPACT_NFT_KEY_LEN: usize = 29;

fn load_mint_smt_root_from_class_cell_dep(nft_key: &CompactNFTKey) -> Result<[u8; 32], Error> {
    if nft_key.as_slice().len() != COMPACT_NFT_KEY_LEN {
        return Err(Error::CompactNFTClassDepError);
    }
    let class_args = Bytes::from(&nft_key.as_slice()[1..25]);
    let class_cell_dep = load_cell(0, Source::CellDep)?;
    let class_type = load_class_type_with_args(&class_args);
    if let Some(dep_class_type) = class_cell_dep.type_().to_opt() {
        if dep_class_type.as_slice() == class_type.as_slice() {
            let class_data = load_cell_data(0, Source::CellDep)?;
            let class = Class::from_data(&class_data)?;
            return Ok(class.nft_smt_root.ok_or(Error::Encoding)?);
        }
        return Err(Error::CompactNFTClassDepError);
    }
    Err(Error::CompactNFTClassDepError)
}

pub fn verify_claim_mint_smt(witness_args_input_type: Bytes) -> Result<(), Error> {
    let lock_script: Vec<Byte> = load_cell_lock(0, Source::Output)?
        .as_slice()
        .iter()
        .map(|v| Byte::from(*v))
        .collect();
    let compact_nft = CompactNft::from_data(&load_cell_data(0, Source::Output)?[..])?;
    let compact_input_out_point = load_input_out_point(0, Source::Input)?;

    let claim_entries = ClaimMintCompactNFTEntries::from_slice(&witness_args_input_type[1..])
        .map_err(|_e| Error::WitnessTypeParseError)?;
    let owned_nft_keys = claim_entries.owned_nft_keys();
    let nft_key = owned_nft_keys.get(0).ok_or(Error::Encoding)?;
    let class_mint_smt_root = load_mint_smt_root_from_class_cell_dep(&nft_key)?;

    let mut mint_nft_keys: Vec<u8> = Vec::new();
    let mut mint_nft_values: Vec<u8> = Vec::new();
    let mut claimed_nft_keys: Vec<u8> = Vec::new();
    let mut claimed_nft_values: Vec<u8> = Vec::new();

    for index in 0..owned_nft_keys.len() {
        // Generate owned and claimed smt kv pairs
        let owned_nft_key = owned_nft_keys.get(index).ok_or(Error::Encoding)?;
        if let Some(smt_type) = owned_nft_key.smt_type().as_slice().get(index) {
            if smt_type != &OWNED_SMT_TYPE {
                return Err(Error::CompactNFTSmtTypeError);
            }
        }
        let claimed_nft_key = claim_entries
            .claimed_nft_keys()
            .get(index)
            .ok_or(Error::Encoding)?;
        if let Some(smt_type) = claimed_nft_key.nft_key().smt_type().as_slice().get(index) {
            if smt_type != &CLAIMED_SMT_TYPE {
                return Err(Error::CompactNFTSmtTypeError);
            }
        }

        if &compact_input_out_point.as_slice()[12..] != claimed_nft_key.out_point().as_slice() {
            return Err(Error::CompactNFTOutPointInvalid);
        }

        claimed_nft_keys.extend(&BYTE3_ZEROS);
        claimed_nft_keys.extend(owned_nft_key.as_slice());
        claimed_nft_keys.extend(&blake2b_256(claimed_nft_key.as_slice()));

        let owned_nft_value = claim_entries
            .owned_nft_values()
            .get(index)
            .ok_or(Error::Encoding)?;
        let claimed_nft_value = claim_entries
            .claimed_nft_values()
            .get(index)
            .ok_or(Error::Encoding)?;
        claimed_nft_values.extend(&BYTE22_ZEROS);
        claimed_nft_values.extend(owned_nft_value.as_slice());
        claimed_nft_values.extend(claimed_nft_value.as_slice());

        // Generate mint smt kv pairs
        mint_nft_keys.extend(&BYTE4_ZEROS);
        mint_nft_keys.extend(owned_nft_key.nft_id().as_slice());

        let lock = common::BytesBuilder::default()
            .set(lock_script.clone())
            .build();

        let mint_nft_value = MintCompactNFTValueBuilder::default()
            .nft_info(owned_nft_value)
            .receiver_lock(lock)
            .build();
        mint_nft_values.extend(&blake2b_256(mint_nft_value.as_slice()))
    }

    let mut context = unsafe { CKBDLContext::<[u8; 128 * 1024]>::new() };
    let lib_ckb_smt = LibCKBSmt::load(&mut context);

    // Verify claimed smt proof of compact nft output
    let proof = claim_entries.proof().raw_data().to_vec();
    if let Some(compact_smt_root) = compact_nft.nft_smt_root {
        lib_ckb_smt
            .smt_verify(
                &compact_smt_root[..],
                &claimed_nft_keys[..],
                &claimed_nft_values[..],
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
            .map_err(|_| Error::CompactClassMintSMTProofVerifyFailed)?;
    }

    // Verify claimed smt proof of compact nft input
    let input_compact_nft = CompactNft::from_data(&load_cell_data(0, Source::Input)?[..])?;
    if let Some(compact_smt_root) = input_compact_nft.nft_smt_root {
        claimed_nft_values.clear();
        for _ in 0..(claim_entries.owned_nft_values().len() * 2) {
            claimed_nft_values.extend(&BYTE32_ZEROS);
        }
        lib_ckb_smt
            .smt_verify(
                &compact_smt_root[..],
                &claimed_nft_keys[..],
                &claimed_nft_values[..],
                &proof[..],
            )
            .map_err(|_| Error::SMTProofVerifyFailed)?;
    }
    Ok(())
}
