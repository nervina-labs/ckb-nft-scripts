use alloc::vec::Vec;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, packed::*, prelude::*},
    dynamic_loading_c_impl::CKBDLContext,
    high_level::{load_cell_data, load_cell_type, load_script, load_witness_args},
};
use core::result::Result;
use nft_smt::registry::CompactNFTRegistryEntries;
use script_utils::helper::{
    check_compact_nft_exist, check_type_args_not_equal_lock_hash,
    load_output_compact_nft_lock_hashes,
};
use script_utils::registry::Registry;
use script_utils::{constants::BYTE32_ZEROS, error::Error, smt::LibCKBSmt};

const TYPE_ARGS_LEN: usize = 20;

fn check_registry_output_type(registry_type: &Script) -> Result<(), Error> {
    match load_cell_type(0, Source::Output)? {
        Some(type_) => {
            if registry_type.as_slice() != type_.as_slice() {
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

fn check_output_registry_data() -> Result<(), Error> {
    let output_registry_data = load_cell_data(0, Source::Output)?;
    // Registry cell data only has version
    if output_registry_data.len() != 1 {
        return Err(Error::RegistryDataInvalid);
    }
    Ok(())
}

fn check_input_registry_exist(registry_type: &Script) -> Result<bool, Error> {
    if let Some(type_) = load_cell_type(0, Source::Input)? {
        if registry_type.as_slice() != type_.as_slice() {
            return Ok(false);
        }
        if check_type_args_not_equal_lock_hash(&type_, Source::Input)? {
            return Err(Error::CompactTypeArgsNotEqualLockHash);
        }
        return Ok(true);
    };
    Ok(false)
}

fn validate_type_and_verify_smt() -> Result<(), Error> {
    // Parse cell data to get registry smt root hash
    let output_registry = Registry::from_data(&load_cell_data(0, Source::Output)?[..])?;
    let input_registry = Registry::from_data(&load_cell_data(0, Source::Input)?[..])?;
    if output_registry.registry_smt_root.is_none() {
        return Err(Error::RegistryCellSMTRootError);
    }

    // Parse witness_args.input_type to get smt leaves and proof to verify smt proof
    let registry_witness_type = load_witness_args(0, Source::Input)?.input_type();
    let registry_entries = registry_witness_type
        .to_opt()
        .ok_or(Error::ItemMissing)
        .map(|witness_type| {
            let witness_type_: Bytes = witness_type.unpack();
            CompactNFTRegistryEntries::from_slice(&witness_type_[..])
        })?
        .map_err(|_| Error::WitnessTypeParseError)?;

    let mut context = unsafe { CKBDLContext::<[u8; 128 * 1024]>::new() };

    let mut registry_keys: Vec<[u8; 32]> = Vec::new();
    let mut registry_key_bytes = [0u8; 32];
    let mut keys: Vec<u8> = Vec::new();
    let mut values: Vec<u8> = Vec::new();
    for kv in registry_entries.kv_state() {
        keys.extend(kv.k().as_slice());
        values.extend(kv.v().as_slice());
        registry_key_bytes.copy_from_slice(kv.k().as_slice());
        registry_keys.push(registry_key_bytes);
    }

    let proof: Vec<u8> = registry_entries.kv_proof().raw_data().to_vec();

    let lib_ckb_smt = LibCKBSmt::load(&mut context);

    if let Some(smt_root) = output_registry.registry_smt_root {
        lib_ckb_smt
            .smt_verify(&smt_root, &keys[..], &values[..], &proof[..])
            .map_err(|_| Error::SMTProofVerifyFailed)?;
    }

    if let Some(smt_root) = input_registry.registry_smt_root {
        values.clear();
        for _ in registry_entries.kv_state() {
            values.extend(&BYTE32_ZEROS);
        }
        lib_ckb_smt
            .smt_verify(&smt_root[..], &keys[..], &values[..], &proof[..])
            .map_err(|_| Error::SMTProofVerifyFailed)?;
    }

    if check_compact_nft_exist(Source::Input) || !check_compact_nft_exist(Source::Output) {
        return Err(Error::RegistryCompactNFTExistError);
    }

    let compact_nft_lock_hashes = load_output_compact_nft_lock_hashes();
    registry_keys.sort_unstable();
    if registry_keys != compact_nft_lock_hashes {
        return Err(Error::RegistryKeysNotEqualLockHashes);
    }

    Ok(())
}

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let type_args: Bytes = script.args().unpack();
    if type_args.len() != TYPE_ARGS_LEN {
        return Err(Error::TypeArgsInvalid);
    }

    check_registry_output_type(&script)?;

    if check_input_registry_exist(&script)? {
        validate_type_and_verify_smt()?;
    } else {
        check_output_registry_data()?;
    }
    Ok(())
}
