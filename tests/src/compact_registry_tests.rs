use super::*;
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};
use ckb_tool::ckb_error::assert_error_eq;
use ckb_tool::ckb_script::ScriptError;
use ckb_tool::ckb_types::{
    bytes::Bytes,
    core::{TransactionBuilder, TransactionView},
    packed::*,
    prelude::*,
};
use nft_smt::smt::blake2b_256;
use nft_smt::{
    registry::{
        Byte32, BytesBuilder, CompactNFTRegistryEntriesBuilder, KVPair, KVPairBuilder,
        KVPairVecBuilder,
    },
    smt::{Blake2bHasher, H256, SMT},
};
use rand::{thread_rng, Rng};

const MAX_CYCLES: u64 = 70_000_000;

// error numbers
const LENGTH_NOT_ENOUGH: i8 = 3;
const TYPE_ARGS_INVALID: i8 = 7;
const WITNESS_TYPE_PARSE_ERROR: i8 = 38;
const COMPACT_REGISTRY_TYPE_ARGS_NOT_EQUAL_LOCK_HASH: i8 = 39;
const SMT_PROOF_VERIFY_FAILED: i8 = 40;
const COMPACT_REGISTRY_CELL_POSITION_ERROR: i8 = 41;

#[derive(PartialEq)]
enum Action {
    Create,
    Update,
    Destroy,
}

#[derive(PartialEq)]
enum RegistryError {
    NoError,
    LengthNotEnough,
    TypeArgsInvalid,
    WitnessTypeParseError,
    CompactRegistryTypeArgsNotEqualLockHash,
    SMTProofVerifyFailed,
}

fn generate_smt_data() -> ([u8; 32], Vec<u8>) {
    let leaves_count = 100;
    let update_leaves_count = 100;
    let mut smt = SMT::default();
    let mut rng = thread_rng();
    for _ in 0..leaves_count {
        let key: H256 = rng.gen::<[u8; 32]>().into();
        let value: H256 = H256::from([255u8; 32]);
        smt.update(key, value).expect("SMT update leave error");
    }

    let mut update_leaves: Vec<(H256, H256)> = Vec::with_capacity(update_leaves_count);
    for _ in 0..update_leaves_count {
        let key: H256 = rng.gen::<[u8; 32]>().into();
        let value: H256 = H256::from([255u8; 32]);
        update_leaves.push((key, value));
        smt.update(key, value).expect("SMT update leave error");
    }
    let root_hash = smt.root().clone();

    let mut root_hash_bytes = [0u8; 32];
    root_hash_bytes.copy_from_slice(root_hash.as_slice());

    let registry_merkle_proof = smt
        .merkle_proof(update_leaves.iter().map(|leave| leave.0).collect())
        .unwrap();
    let registry_merkle_proof_compiled = registry_merkle_proof
        .compile(update_leaves.clone())
        .unwrap();
    let verify_result = registry_merkle_proof_compiled
        .verify::<Blake2bHasher>(&root_hash, update_leaves.clone())
        .expect("smt proof verify failed");
    assert!(verify_result, "smt proof verify failed");

    let merkel_proof_vec: Vec<u8> = registry_merkle_proof_compiled.into();

    let kv_pair_vec = update_leaves
        .iter()
        .map(|leave| {
            let key: [u8; 32] = leave.0.into();
            let value: [u8; 32] = leave.1.into();
            KVPairBuilder::default()
                .k(Byte32::from_slice(&key).unwrap())
                .v(Byte32::from_slice(&value).unwrap())
                .build()
        })
        .collect::<Vec<KVPair>>();

    let entries_builder = CompactNFTRegistryEntriesBuilder::default();
    let kv_pair_vec_builder = KVPairVecBuilder::default();
    let merkel_proof_bytes = BytesBuilder::default()
        .extend(merkel_proof_vec.iter().map(|v| Byte::from(*v)))
        .build();

    let witness_data = entries_builder
        .kv_state(kv_pair_vec_builder.set(kv_pair_vec).build())
        .kv_proof(merkel_proof_bytes)
        .build();

    (root_hash_bytes, Vec::from(witness_data.as_slice()))
}

fn create_test_context(
    action: Action,
    registry_error: RegistryError,
) -> (Context, TransactionView) {
    // deploy compact-registry-type script
    let mut context = Context::default();
    let registry_bin: Bytes = Loader::default().load_binary("compact-registry-type");
    let registry_out_point = context.deploy_cell(registry_bin);
    let registry_type_script_dep = CellDep::new_builder()
        .out_point(registry_out_point.clone())
        .build();

    let smt_bin: Bytes = Loader::default().load_binary("ckb_smt");
    let smt_out_point = context.deploy_cell(smt_bin);
    let smt_dep = CellDep::new_builder().out_point(smt_out_point).build();

    // deploy always_success script
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());

    // prepare scripts
    let lock_script = context
        .build_script(&always_success_out_point, Default::default())
        .expect("script");
    let lock_hash_160 = &blake2b_256(lock_script.as_slice())[0..20];

    let lock_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point)
        .build();

    // prepare cells
    let normal_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .build(),
        Bytes::new(),
    );
    let normal_input = CellInput::new_builder()
        .previous_output(normal_input_out_point.clone())
        .build();

    let registry_type_args = match registry_error {
        RegistryError::TypeArgsInvalid => Bytes::copy_from_slice(&lock_hash_160[0..10]),
        RegistryError::CompactRegistryTypeArgsNotEqualLockHash => {
            let error_lock_hash = [0u8; 20];
            Bytes::copy_from_slice(&error_lock_hash[..])
        }
        _ => Bytes::copy_from_slice(lock_hash_160),
    };

    let registry_type_script = context
        .build_script(&registry_out_point, registry_type_args)
        .expect("script");
    let registry_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .type_(Some(registry_type_script.clone()).pack())
            .build(),
        Bytes::new(),
    );
    let registry_input = CellInput::new_builder()
        .previous_output(registry_input_out_point.clone())
        .build();

    let inputs = match action {
        Action::Create => vec![normal_input.clone()],
        _ => vec![registry_input.clone()],
    };

    let outputs = match action {
        Action::Destroy => vec![CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .build()],
        Action::Create => vec![CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .type_(Some(registry_type_script.clone()).pack())
            .build()],
        Action::Update => vec![CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .type_(Some(registry_type_script.clone()).pack())
            .build()],
    };

    let (root_hash, witness_data) = generate_smt_data();

    let outputs_data: Vec<Bytes> = match registry_error {
        RegistryError::LengthNotEnough => vec![Bytes::from(hex::decode("00000000000000").unwrap())],
        RegistryError::SMTProofVerifyFailed => vec![Bytes::from(
            hex::decode("54dfaba38275883ef9b6d5fb02053b71dbba19630ff5f2ec01d5d6965366c1f7")
                .unwrap(),
        )],
        _ => vec![Bytes::from(Vec::from(&root_hash[..]))],
    };

    let error_witness_args = WitnessArgsBuilder::default()
        .input_type(
            Some(Bytes::from(
                hex::decode("54dfaba38275883ef9b6d5fb02053b71dbba19630ff5f2ec01d5d6965366c1f7")
                    .unwrap(),
            ))
            .pack(),
        )
        .build();

    let witness_args = WitnessArgsBuilder::default()
        .input_type(Some(Bytes::from(witness_data.clone())).pack())
        .build();

    let witnesses = match registry_error {
        RegistryError::WitnessTypeParseError => vec![error_witness_args.as_bytes()],
        _ => vec![witness_args.as_bytes()],
    };

    // build transaction
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_dep(lock_script_dep)
        .cell_dep(registry_type_script_dep)
        .cell_dep(smt_dep)
        .witnesses(witnesses.pack())
        .build();
    (context, tx)
}

#[test]
fn test_create_registry_cell_success() {
    let (mut context, tx) = create_test_context(Action::Create, RegistryError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_update_registry_cell_success() {
    let (mut context, tx) = create_test_context(Action::Update, RegistryError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_destroy_registry_cell_error() {
    let (mut context, tx) = create_test_context(Action::Destroy, RegistryError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    let errors = vec![
        ScriptError::ValidationFailure(COMPACT_REGISTRY_CELL_POSITION_ERROR)
            .input_type_script(script_cell_index),
        ScriptError::ValidationFailure(COMPACT_REGISTRY_CELL_POSITION_ERROR)
            .output_type_script(script_cell_index),
    ];
    assert_errors_contain!(err, errors);
}

#[test]
fn test_registry_type_args_not_equal_to_lock_hash_error() {
    let (mut context, tx) = create_test_context(
        Action::Create,
        RegistryError::CompactRegistryTypeArgsNotEqualLockHash,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(COMPACT_REGISTRY_TYPE_ARGS_NOT_EQUAL_LOCK_HASH)
            .output_type_script(script_cell_index)
    );
}

#[test]
fn test_registry_type_smt_verify_error() {
    let (mut context, tx) =
        create_test_context(Action::Create, RegistryError::SMTProofVerifyFailed);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(SMT_PROOF_VERIFY_FAILED)
            .output_type_script(script_cell_index)
    );
}

#[test]
fn test_registry_cell_data_length_error() {
    let (mut context, tx) = create_test_context(Action::Create, RegistryError::LengthNotEnough);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(LENGTH_NOT_ENOUGH).output_type_script(script_cell_index)
    );
}

#[test]
fn test_registry_type_args_error() {
    let (mut context, tx) = create_test_context(Action::Create, RegistryError::TypeArgsInvalid);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(TYPE_ARGS_INVALID).output_type_script(script_cell_index)
    );
}

#[test]
fn test_registry_type_parse_witness_error() {
    let (mut context, tx) =
        create_test_context(Action::Create, RegistryError::WitnessTypeParseError);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(WITNESS_TYPE_PARSE_ERROR)
            .output_type_script(script_cell_index)
    );
}
