use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    high_level::{load_script, load_cell_data, load_witness_args, load_cell_lock_hash, load_cell_type},
};
use alloc::vec::Vec;
use core::result::Result;
use sparse_merkle_tree::{CompiledMerkleProof, H256};
use script_utils::{
    error::Error,
    hash::Blake2bHasher,
};
use nft_smt::{CompactNFTRegistryEntries, molecule::prelude::Entity};

const TYPE_ARGS_LEN: usize = 20;
const REGISTRY_SMT_ROOT_HASH: usize = 32;

fn check_type_args_equal_lock_hash() -> Result<(), Error> {
    match load_cell_type(0, Source::Output)? {
        Some(type_) => {
            let lock_hash = load_cell_lock_hash(0, Source::Output)?;
            let type_args: Bytes = type_.args().unpack();
            if type_args[..] != lock_hash[0..20] {
                return Err(Error::CompactRegistryTypeArgsNotEqualLockHash);
            }
        },
        None => {
            return Err(Error::CompactRegistryTypeArgsNotEqualLockHash)
        }
    }

    if let Some(type_) = load_cell_type(0, Source::Input)? {
        let lock_hash = load_cell_lock_hash(0, Source::Input)?;
        let type_args: Bytes = type_.args().unpack();
        if type_args[..] != lock_hash[0..20] {
            return Err(Error::CompactRegistryTypeArgsNotEqualLockHash);
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

    // Parse witness_args.type to get smt leaves and proof
    let registry_witness_type = load_witness_args(0, Source::Input)?.input_type();
    let registry_entries = registry_witness_type
        .to_opt()
        .ok_or(Error::ItemMissing)
        .map(|witness_type| {
            let witness_type_: Bytes = witness_type.unpack();
            CompactNFTRegistryEntries::from_slice(&witness_type_[..])
        })?
        .map_err(|_| Error::WitnessTypeParseError)?;
    let registry_merkel_proof = CompiledMerkleProof(Vec::from(registry_entries.kv_proof().raw_data()));
    let mut leaves: Vec<(H256, H256)> = Vec::new();
    for kv in registry_entries.kv_state() {
        let mut key = [0u8; 32];
        key.copy_from_slice(kv.v().as_slice());
        let mut value = [0u8; 32];
        value.copy_from_slice(kv.v().as_slice());
        leaves.push((H256::from(key), H256::from(value)));
    }

    let verify_result = registry_merkel_proof
            .verify::<Blake2bHasher>(&H256::from(registry_smt_root_hash), leaves)
            .map_err(|_e| Error::SMTProofVerifyFailed)?;

    if !verify_result {
        return Err(Error::SMTProofVerifyFailed);
    }

    Ok(())
}

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let type_args: Bytes = script.args().unpack();
    if type_args.len() != TYPE_ARGS_LEN {
        return Err(Error::TypeArgsInvalid);
    }

    check_type_args_equal_lock_hash()?;
    verify_smt_proof()?;

    Ok(())
}
