use alloc::vec::Vec;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    dynamic_loading_c_impl::CKBDLContext,
    high_level::{load_cell_data, load_cell_lock_hash, load_input_out_point},
};
use core::result::Result;
use nft_smt::{smt::blake2b_256, transfer::WithdrawCompactNFTEntries};
use script_utils::{
    compact_nft::CompactNft,
    constants::{BYTE22_ZEROS, BYTE32_ZEROS, BYTE3_ZEROS},
    error::Error,
    smt::LibCKBSmt,
};

pub fn verify_withdraw_transfer_smt(witness_args_input_type: Bytes) -> Result<(), Error> {
    let compact_nft = CompactNft::from_data(&load_cell_data(0, Source::Output)?[..])?;
    let compact_input_out_point = load_input_out_point(0, Source::Input)?;

    let lock_hash = load_cell_lock_hash(0, Source::Output)?;

    let withdraw_entries = WithdrawCompactNFTEntries::from_slice(&witness_args_input_type[1..])
        .map_err(|_e| Error::WitnessTypeParseError)?;
    let owned_nft_keys = withdraw_entries.owned_nft_keys();

    let mut withdrawal_keys: Vec<u8> = Vec::new();
    let mut withdrawal_values: Vec<u8> = Vec::new();
    let mut withdrawal_old_values: Vec<u8> = Vec::new();

    for index in 0..owned_nft_keys.len() {
        let withdrawal_nft_value = withdraw_entries
            .withdrawal_nft_values()
            .get(index)
            .ok_or(Error::Encoding)
            .map_err(|_e| Error::Encoding)?;
        if &compact_input_out_point.as_slice()[12..] != withdrawal_nft_value.out_point().as_slice()
        {
            return Err(Error::CompactNFTOutPointInvalid);
        }
        if &lock_hash[0..20] != withdrawal_nft_value.to().as_slice() {
            return Err(Error::WithdrawCompactToNotEqualLockHash);
        }
        let owned_nft_value = withdraw_entries
            .owned_nft_values()
            .get(index)
            .ok_or(Error::Encoding)
            .map_err(|_e| Error::Encoding)?;
        if owned_nft_value.as_slice() != withdrawal_nft_value.nft_info().as_slice() {
            return Err(Error::WithdrawCompactNFTInfoNotSame);
        }

        // Generate owned and withdrawal smt kv pairs
        withdrawal_values.extend(&BYTE32_ZEROS);
        withdrawal_values.extend(&blake2b_256(withdrawal_nft_value.as_slice()));

        withdrawal_old_values.extend(&BYTE22_ZEROS);
        withdrawal_old_values.extend(owned_nft_value.as_slice());
        withdrawal_old_values.extend(&BYTE32_ZEROS);

        let owned_nft_key = owned_nft_keys
            .get(index)
            .ok_or(Error::Encoding)
            .map_err(|_e| Error::Encoding)?;
        let withdrawal_nft_key = withdraw_entries
            .withdrawal_nft_keys()
            .get(index)
            .ok_or(Error::Encoding)
            .map_err(|_e| Error::Encoding)?;
        withdrawal_keys.extend(&BYTE3_ZEROS);
        withdrawal_keys.extend(owned_nft_key.as_slice());
        withdrawal_keys.extend(&BYTE3_ZEROS);
        withdrawal_keys.extend(withdrawal_nft_key.as_slice());
    }

    let mut context = unsafe { CKBDLContext::<[u8; 128 * 1024]>::new() };
    let lib_ckb_smt = LibCKBSmt::load(&mut context);

    // Verify withdrawal smt proof of compact nft output
    let proof = withdraw_entries.proof().raw_data().to_vec();
    if let Some(compact_smt_root) = compact_nft.nft_smt_root {
        lib_ckb_smt
            .smt_verify(
                &compact_smt_root[..],
                &withdrawal_keys[..],
                &withdrawal_values[..],
                &proof[..],
            )
            .map_err(|_| Error::SMTProofVerifyFailed)?;
    }

    // Verify withdrawal smt proof of compact nft input
    let input_compact_nft = CompactNft::from_data(&load_cell_data(0, Source::Input)?[..])?;
    if let Some(compact_smt_root) = input_compact_nft.nft_smt_root {
        lib_ckb_smt
            .smt_verify(
                &compact_smt_root[..],
                &withdrawal_keys[..],
                &withdrawal_old_values[..],
                &proof[..],
            )
            .map_err(|_| Error::SMTProofVerifyFailed)?;
    }
    Ok(())
}
