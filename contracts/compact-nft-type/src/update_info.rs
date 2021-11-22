use alloc::vec::Vec;
use ckb_std::high_level::load_cell_data;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    dynamic_loading_c_impl::CKBDLContext,
};
use core::result::Result;
use nft_smt::update::UpdateCompactNFTEntries;
use script_utils::constants::OWNED_SMT_TYPE;
use script_utils::{
    compact_nft::CompactNft,
    constants::{BYTE22_ZEROS, BYTE3_ZEROS},
    error::Error,
    smt::LibCKBSmt,
};

pub fn verify_update_nft_info_smt(witness_args_input_type: Bytes) -> Result<(), Error> {
    let compact_nft = CompactNft::from_data(&load_cell_data(0, Source::Output)?[..])?;

    let update_entries = UpdateCompactNFTEntries::from_slice(&witness_args_input_type[1..])
        .map_err(|_e| Error::WitnessTypeParseError)?;
    let owned_nft_keys = update_entries.owned_nft_keys();

    let mut update_nft_keys: Vec<u8> = Vec::new();
    let mut update_nft_values: Vec<u8> = Vec::new();
    let mut update_old_nft_values: Vec<u8> = Vec::new();

    for index in 0..owned_nft_keys.len() {
        // Generate owned smt kv pairs
        let owned_nft_key = owned_nft_keys.get(index).ok_or(Error::Encoding)?;
        if let Some(smt_type) = owned_nft_key.smt_type().as_slice().get(index) {
            if smt_type != &OWNED_SMT_TYPE {
                return Err(Error::CompactNFTSmtTypeError);
            }
        }

        update_nft_keys.extend(&BYTE3_ZEROS);
        update_nft_keys.extend(owned_nft_key.as_slice());

        let new_owned_nft_value = update_entries
            .new_nft_values()
            .get(index)
            .ok_or(Error::Encoding)?;
        update_nft_values.extend(&BYTE22_ZEROS);
        update_nft_values.extend(new_owned_nft_value.as_slice());

        let old_owned_nft_value = update_entries
            .old_nft_values()
            .get(index)
            .ok_or(Error::Encoding)?;
        update_old_nft_values.extend(&BYTE22_ZEROS);
        update_old_nft_values.extend(old_owned_nft_value.as_slice());
    }

    let mut context = unsafe { CKBDLContext::<[u8; 128 * 1024]>::new() };
    let lib_ckb_smt = LibCKBSmt::load(&mut context);

    // Verify update smt proof of compact nft output
    let proof = update_entries.proof().raw_data().to_vec();
    if let Some(compact_smt_root) = compact_nft.nft_smt_root {
        lib_ckb_smt
            .smt_verify(
                &compact_smt_root[..],
                &update_nft_keys[..],
                &update_nft_values[..],
                &proof[..],
            )
            .map_err(|_| Error::SMTProofVerifyFailed)?;
    }

    // Verify update smt proof of compact nft input
    let input_compact_nft = CompactNft::from_data(&load_cell_data(0, Source::Input)?[..])?;
    if let Some(compact_smt_root) = input_compact_nft.nft_smt_root {
        lib_ckb_smt
            .smt_verify(
                &compact_smt_root[..],
                &update_nft_keys[..],
                &update_old_nft_values[..],
                &proof[..],
            )
            .map_err(|_| Error::SMTProofVerifyFailed)?;
    }
    Ok(())
}
