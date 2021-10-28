use alloc::vec::Vec;
use ckb_std::{
    ckb_types::{bytes::Bytes, packed::*, prelude::*},
    debug,
    dynamic_loading_c_impl::CKBDLContext,
};
use core::result::Result;
use nft_smt::{mint::CompactNFTMintEntries, smt::blake2b_256};
use script_utils::{class::Class, error::Error, helper::u32_from_slice, smt::LibCKBSmt};

const RESERVED: [u8; 4] = [0u8; 4];

pub fn check_compact_nft_mint(
    input_class: Class,
    output_class: Class,
    witness_args: WitnessArgs,
    class_args: Bytes,
) -> Result<(), Error> {
    // Parse witness_args.input_type to get smt leaves and proof to verify smt proof
    if let Some(mint_witness_type) = witness_args.input_type().to_opt() {
        if output_class.nft_smt_root.is_none() {
            return Err(Error::ClassCompactSmtRootError);
        }
        let witness_type_bytes: Bytes = mint_witness_type.unpack();
        let mint_entries = CompactNFTMintEntries::from_slice(&witness_type_bytes[..])
            .map_err(|_e| Error::WitnessTypeParseError)?;

        if output_class.issued != input_class.issued + mint_entries.nft_ids().len() as u32 {
            return Err(Error::ClassIssuedInvalid);
        }

        let mut keys: Vec<u8> = Vec::new();
        let mut token_ids: Vec<u32> = Vec::new();
        for nft_id in mint_entries.nft_ids() {
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

            keys.extend(&RESERVED);
            keys.extend(nft_id.as_slice());
        }
        let mut class_cell_token_ids = Vec::new();
        for token_id in input_class.issued..output_class.issued {
            class_cell_token_ids.push(token_id);
        }
        debug!("token_ids: {:?}", token_ids);
        debug!("class_cell_token_ids: {:?}", token_ids);
        if token_ids != class_cell_token_ids {
            return Err(Error::NFTTokenIdIncreaseError);
        }

        let mut values: Vec<u8> = Vec::new();
        for nft_info in mint_entries.nft_infos() {
            if nft_info.configure().as_slice()[0] != input_class.configure {
                return Err(Error::NFTAndClassConfigureNotSame);
            }
            values.extend(Vec::from(blake2b_256(nft_info.as_slice())));
        }

        let proof: Vec<u8> = mint_entries.proof().raw_data().to_vec();

        let mut context = unsafe { CKBDLContext::<[u8; 128 * 1024]>::new() };
        let lib_ckb_smt = LibCKBSmt::load(&mut context);

        if let Some(mint_smt_root) = output_class.nft_smt_root {
            lib_ckb_smt
                .smt_verify(&mint_smt_root[..], &keys[..], &values[..], &proof[..])
                .map_err(|_| Error::SMTProofVerifyFailed)?;
        }
    }
    Ok(())
}
