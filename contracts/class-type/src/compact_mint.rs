use alloc::vec::Vec;
use ckb_std::{
    ckb_types::{bytes::Bytes, packed::*, prelude::*},
    dynamic_loading_c_impl::CKBDLContext,
};
use core::result::Result;
use nft_smt::{mint::MintCompactNFTEntries, smt::blake2b_256};
use script_utils::{
    class::Class,
    constants::{BYTE32_ZEROS, BYTE4_ZEROS},
    error::Error,
    helper::u32_from_slice,
    smt::LibCKBSmt,
};

pub fn check_compact_nft_mint(
    input_class: Class,
    output_class: Class,
    witness_args: WitnessArgs,
    class_args: Bytes,
) -> Result<(), Error> {
    // Parse witness_args.input_type to get smt leaves and proof to verify smt proof
    if let Some(mint_witness_type) = witness_args.input_type().to_opt() {
        if output_class.nft_smt_root.is_none() || input_class.nft_smt_root.is_none() {
            return Err(Error::ClassCompactMintSmtRootError);
        }
        let witness_type_bytes: Bytes = mint_witness_type.unpack();
        let mint_entries = MintCompactNFTEntries::from_slice(&witness_type_bytes[..])
            .map_err(|_e| Error::WitnessTypeParseError)?;

        if output_class.issued != input_class.issued + mint_entries.nft_keys().len() as u32 {
            return Err(Error::ClassIssuedInvalid);
        }

        let mut smt_keys: Vec<u8> = Vec::new();
        let mut token_ids: Vec<u32> = Vec::new();

        for nft_id in mint_entries.nft_keys() {
            if nft_id.issuer_id().as_slice() != &class_args.to_vec()[0..20]
                || nft_id.class_id().as_slice() != &class_args.to_vec()[20..]
            {
                return Err(Error::CompactIssuerIdOrClassIdInvalid);
            }
            let token_id = u32_from_slice(nft_id.token_id().as_slice());
            if token_id < input_class.issued || token_id >= output_class.issued {
                return Err(Error::ClassIssuedInvalid);
            }
            token_ids.push(token_id);

            smt_keys.extend(&BYTE4_ZEROS);
            smt_keys.extend(nft_id.as_slice());
        }
        let mut class_cell_token_ids = Vec::new();
        for token_id in input_class.issued..output_class.issued {
            class_cell_token_ids.push(token_id);
        }
        if token_ids != class_cell_token_ids {
            return Err(Error::NFTTokenIdIncreaseError);
        }

        let mut smt_values: Vec<u8> = Vec::new();
        for nft_value in mint_entries.nft_values() {
            if nft_value.nft_info().configure().as_slice()[0] != input_class.configure {
                return Err(Error::NFTAndClassConfigureNotSame);
            }
            smt_values.extend(&blake2b_256(nft_value.as_slice()));
        }

        let proof: Vec<u8> = mint_entries.proof().raw_data().to_vec();

        let mut context = unsafe { CKBDLContext::<[u8; 128 * 1024]>::new() };
        let lib_ckb_smt = LibCKBSmt::load(&mut context);

        if let Some(mint_smt_root) = output_class.nft_smt_root {
            lib_ckb_smt
                .smt_verify(
                    &mint_smt_root[..],
                    &smt_keys[..],
                    &smt_values[..],
                    &proof[..],
                )
                .map_err(|_| Error::SMTProofVerifyFailed)?;
        }

        if let Some(input_class_smt_root) = input_class.nft_smt_root {
            smt_values.clear();
            for _ in mint_entries.nft_values() {
                smt_values.extend(&BYTE32_ZEROS);
            }
            lib_ckb_smt
                .smt_verify(
                    &input_class_smt_root[..],
                    &smt_keys[..],
                    &smt_values[..],
                    &proof[..],
                )
                .map_err(|_| Error::SMTProofVerifyFailed)?;
        }
    }
    Ok(())
}
