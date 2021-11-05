use alloc::vec::Vec;
use ckb_std::high_level::{load_cell_data, load_cell_lock_hash};
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, packed::*, prelude::*},
    dynamic_loading_c_impl::CKBDLContext,
    high_level::load_cell,
};
use core::result::Result;
use nft_smt::{
    common::LockHashBuilder,
    smt::blake2b_256,
    transfer::{ClaimTransferCompactNFTEntries, WithdrawCompactNFTValueBuilder},
};
use script_utils::{
    compact_nft::CompactNft,
    constants::{BYTE22_ZEROS, BYTE32_ZEROS, BYTE3_ZEROS},
    error::Error,
    smt::LibCKBSmt,
};

fn load_withdrawal_smt_root_from_compact_cell_dep(
    compact_nft_type: &Script,
) -> Result<[u8; 32], Error> {
    let withdrawal_compact_cell_dep = load_cell(0, Source::CellDep)?;
    if let Some(dep_compact_type) = withdrawal_compact_cell_dep.type_().to_opt() {
        if dep_compact_type.code_hash().as_slice() == compact_nft_type.code_hash().as_slice()
            && dep_compact_type.hash_type() == compact_nft_type.hash_type()
        {
            let compact_nft_data = load_cell_data(0, Source::CellDep)?;
            let compact_nft = CompactNft::from_data(&compact_nft_data)?;
            return Ok(compact_nft.nft_smt_root.ok_or(Error::Encoding)?);
        }
        return Err(Error::CompactNFTWithdrawalDepError);
    }
    Err(Error::CompactNFTWithdrawalDepError)
}

pub fn verify_claim_transfer_smt(
    compact_nft_type: &Script,
    witness_args_input_type: Bytes,
) -> Result<(), Error> {
    let mut lock_hash_160 = [Byte::from(0u8); 20];
    let lock_hash: Vec<Byte> = load_cell_lock_hash(0, Source::Output)?[12..]
        .iter()
        .map(|v| Byte::from(*v))
        .collect();
    lock_hash_160.copy_from_slice(&lock_hash);
    let compact_nft = CompactNft::from_data(&load_cell_data(0, Source::Output)?[..])?;

    let claim_entries = ClaimTransferCompactNFTEntries::from_slice(&witness_args_input_type[1..])
        .map_err(|_e| Error::WitnessTypeParseError)?;
    let withdrawal_compact_smt_root =
        load_withdrawal_smt_root_from_compact_cell_dep(&compact_nft_type)?;

    let mut withdrawal_nft_keys: Vec<u8> = Vec::new();
    let mut withdrawal_nft_values: Vec<u8> = Vec::new();
    let mut claimed_nft_keys: Vec<u8> = Vec::new();
    let mut claimed_nft_values: Vec<u8> = Vec::new();

    for index in 0..claim_entries.owned_nft_keys().len() {
        // Generate owned and claimed smt kv pairs
        let owned_nft_key = claim_entries
            .owned_nft_keys()
            .get(index)
            .ok_or(Error::Encoding)
            .map_err(|_e| Error::Encoding)?;
        let claimed_nft_key = claim_entries
            .claimed_nft_keys()
            .get(index)
            .ok_or(Error::Encoding)
            .map_err(|_e| Error::Encoding)?;

        claimed_nft_keys.extend(&BYTE3_ZEROS);
        claimed_nft_keys.extend(owned_nft_key.as_slice());
        claimed_nft_keys.extend(&blake2b_256(claimed_nft_key.as_slice()));

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
        claimed_nft_values.extend(&BYTE22_ZEROS);
        claimed_nft_values.extend(owned_nft_value.as_slice());
        claimed_nft_values.extend(claimed_nft_value.as_slice());

        // Generate withdrawal smt kv pairs
        let withdrawal_smt_type = [2u8];
        withdrawal_nft_keys.extend(&BYTE3_ZEROS);
        withdrawal_nft_keys.extend(&withdrawal_smt_type);
        withdrawal_nft_keys.extend(owned_nft_key.nft_id().as_slice());

        let withdrawal_nft_value = WithdrawCompactNFTValueBuilder::default()
            .nft_info(owned_nft_value)
            .to(LockHashBuilder::default().set(lock_hash_160).build())
            .out_point(claimed_nft_key.out_point())
            .build();
        withdrawal_nft_values.extend(&blake2b_256(withdrawal_nft_value.as_slice()));
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
    // Verify withdrawal smt proof of compact cell_dep
    if !withdrawal_nft_keys.is_empty() {
        let withdrawal_proof = claim_entries.withdrawal_proof().raw_data().to_vec();
        lib_ckb_smt
            .smt_verify(
                &withdrawal_compact_smt_root[..],
                &withdrawal_nft_keys[..],
                &withdrawal_nft_values[..],
                &withdrawal_proof[..],
            )
            .map_err(|_| Error::ClaimedCompactWithdrawalSMTProofVerifyFailed)?;
    }

    // Verify claimed smt proof of compact nft input
    claimed_nft_values.clear();
    for _ in 0..(claim_entries.owned_nft_values().len() * 2) {
        claimed_nft_values.extend(&BYTE32_ZEROS);
    }
    let input_compact_nft = CompactNft::from_data(&load_cell_data(0, Source::Input)?[..])?;
    if let Some(compact_smt_root) = input_compact_nft.nft_smt_root {
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
