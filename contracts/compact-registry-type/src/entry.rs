use alloc::vec::Vec;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, packed::*, prelude::*},
    debug,
    dynamic_loading_c_impl::CKBDLContext,
    high_level::{
        load_cell_data, load_cell_lock_hash, load_cell_type, load_script, load_witness_args,
    },
};
use core::result::Result;
use nft_smt::registry::CompactNFTRegistryEntries;
use script_utils::{error::Error, smt::LibCKBSmt};

const TYPE_ARGS_LEN: usize = 20;
const REGISTRY_SMT_ROOT_HASH: usize = 32;

fn check_type_args_not_equal_lock_hash(type_: &Script, source: Source) -> bool {
    let lock_hash = load_cell_lock_hash(0, source)?;
    let type_args: Bytes = type_.args().unpack();
    type_args[..] != lock_hash[0..TYPE_ARGS_LEN]
}

fn check_type_args_equal_lock_hash(registry_type: &Script) -> Result<(), Error> {
    // Outputs[0] must be compact_registry_cell whose type_args must be equal the lock_hash[0..20]
    match load_cell_type(0, Source::Output)? {
        Some(type_) => {
            if registry_type.as_slice() != type_.as_slice() {
                return Err(Error::CompactCellPositionError);
            }
            if check_type_args_not_equal_lock_hash(&type_, Source::Output) {
                return Err(Error::CompactTypeArgsNotEqualLockHash);
            }
        }
        None => return Err(Error::CompactCellPositionError),
    }

    // If the inputs[0] is compact_registry_cell, then its type_args must be equal to
    // lock_hash[0..20] and its lock script must be equal to outputs.compact_registry_cell.
    if let Some(type_) = load_cell_type(0, Source::Input)? {
        if registry_type.as_slice() != type_.as_slice() {
            return Ok(());
        }
        if check_type_args_not_equal_lock_hash(&type_, Source::Input) {
            return Err(Error::CompactTypeArgsNotEqualLockHash);
        }
    };

    Ok(())
}

fn verify_smt_proof() -> Result<(), Error> {
    // Parse cell data to get registry smt root hash
    let registry_smt_root = load_cell_data(0, Source::Output).or(Err(Error::Encoding))?;
    if registry_smt_root.len() != REGISTRY_SMT_ROOT_HASH {
        return Err(Error::LengthNotEnough);
    }
    let mut registry_smt_root_hash = [0u8; 32];
    registry_smt_root_hash.copy_from_slice(&registry_smt_root);

    debug!("root hash: {:?}", registry_smt_root_hash);

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

    let mut keys: Vec<u8> = Vec::new();
    let mut values: Vec<u8> = Vec::new();
    for kv in registry_entries.kv_state() {
        keys.extend(kv.k().as_slice());
        values.extend(kv.v().as_slice());
    }

    let proof: Vec<u8> = registry_entries.kv_proof().raw_data().to_vec();

    let lib_ckb_smt = LibCKBSmt::load(&mut context);

    lib_ckb_smt
        .smt_verify(
            &registry_smt_root_hash[..],
            &keys[..],
            &values[..],
            &proof[..],
        )
        .map_err(|_| Error::SMTProofVerifyFailed)?;

    Ok(())
}

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let type_args: Bytes = script.args().unpack();
    if type_args.len() != TYPE_ARGS_LEN {
        return Err(Error::TypeArgsInvalid);
    }

    check_type_args_equal_lock_hash(&script)?;
    verify_smt_proof()?;

    Ok(())
}
