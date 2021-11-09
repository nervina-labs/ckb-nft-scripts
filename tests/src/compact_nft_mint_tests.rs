use super::*;
use crate::constants::{BYTE22_ZEROS, BYTE3_ZEROS, BYTE4_ZEROS, CLASS_TYPE_CODE_HASH, TYPE};
use ckb_testtool::ckb_types::{
    bytes::Bytes,
    core::{TransactionBuilder, TransactionView},
    packed::*,
    prelude::*,
};
use ckb_testtool::context::random_out_point;
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};
use nft_smt::smt::blake2b_256;
use nft_smt::{
    common::{Byte32, Byte32Builder, BytesBuilder, Uint32Builder, *},
    mint::*,
    smt::{Blake2bHasher, H256, SMT},
    transfer::*,
};
use rand::{thread_rng, Rng};

const MAX_CYCLES: u64 = 70_000_000;

// error numbers
const WITNESS_TYPE_PARSE_ERROR: i8 = 38;
const COMPACT_TYPE_ARGS_NOT_EQUAL_LOCK_HASH: i8 = 39;
const SMT_PROOF_VERIFY_FAILED: i8 = 40;
const COMPACT_CELL_POSITION_ERROR: i8 = 41;
const COMPACT_NFT_SMT_ROOT_ERROR: i8 = 45;
const COMPACT_NFT_CLASS_DEP_ERROR: i8 = 46;
const COMPACT_NFT_OUT_POINT_INVALID: i8 = 47;
const COMPACT_CLASS_MINT_SMT_PROOF_VERIFY_FAILED: i8 = 48;

#[derive(PartialEq)]
enum Action {
    Create,
    Update,
    Destroy,
}

#[derive(PartialEq)]
enum CompactError {
    NoError,
    WitnessTypeParseError,
    CompactTypeArgsNotEqualLockHash,
    SMTProofVerifyFailed,
    CompactNFTSMTRootError,
    CompactNFTClassDepError,
    CompactNFTOutPointInvalid,
    CompactClassMintSMTProofVerifyFailed,
}

const CLAIM_MINT: u8 = 1;

fn generate_class_mint_smt_data(
    class_type_args: Vec<u8>,
    receiver_lock_script: Script,
    claim_leaves_count: usize,
) -> (
    [u8; 32],
    Vec<CompactNFTId>,
    Vec<MintCompactNFTValue>,
    Vec<u8>,
) {
    let class_type_args_bytes = class_type_args
        .iter()
        .map(|v| Byte::from(*v))
        .collect::<Vec<Byte>>();
    let leaves_count = 100;
    let mut smt = SMT::default();
    let mut mint_nft_keys: Vec<CompactNFTId> = Vec::new();
    let mut mint_nft_values: Vec<MintCompactNFTValue> = Vec::new();
    let mut update_leaves: Vec<(H256, H256)> = Vec::with_capacity(claim_leaves_count);
    for index in 0..leaves_count {
        let mut issuer_id_bytes = [Byte::from(0); 20];
        issuer_id_bytes.copy_from_slice(&class_type_args_bytes[0..20]);
        let issuer_id = IssuerIdBuilder::default().set(issuer_id_bytes).build();

        let mut class_id_bytes = [Byte::from(0); 4];
        class_id_bytes.copy_from_slice(&class_type_args_bytes[20..24]);
        let class_id = Uint32Builder::default().set(class_id_bytes).build();

        let token_id_vec = ((index + 5) as u32)
            .to_be_bytes()
            .iter()
            .map(|v| Byte::from(*v))
            .collect::<Vec<Byte>>();
        let mut token_id_bytes = [Byte::from(0); 4];
        token_id_bytes.copy_from_slice(&token_id_vec);
        let token_id = Uint32Builder::default().set(token_id_bytes).build();
        let nft_id = CompactNFTIdBuilder::default()
            .issuer_id(issuer_id)
            .class_id(class_id)
            .token_id(token_id)
            .build();

        let mut nft_id_vec = Vec::new();
        nft_id_vec.extend(&BYTE4_ZEROS);
        nft_id_vec.extend(&nft_id.as_slice().to_vec());
        let mut nft_id_bytes = [0u8; 32];
        nft_id_bytes.copy_from_slice(&nft_id_vec);
        let key = H256::from(nft_id_bytes);

        let characteristic = CharacteristicBuilder::default()
            .set([Byte::from(0); 8])
            .build();
        let receiver_lock = receiver_lock_script
            .as_slice()
            .iter()
            .map(|v| Byte::from(*v))
            .collect();
        let nft_info = CompactNFTInfoBuilder::default()
            .characteristic(characteristic)
            .configure(Byte::from(0u8))
            .state(Byte::from(0u8))
            .build();
        let nft_value = MintCompactNFTValueBuilder::default()
            .nft_info(nft_info.clone())
            .receiver_lock(BytesBuilder::default().set(receiver_lock).build())
            .build();

        if index < claim_leaves_count {
            mint_nft_keys.push(nft_id.clone());
            mint_nft_values.push(nft_value.clone());
        }

        let value: H256 = H256::from(blake2b_256(nft_value.as_slice()));
        if index < claim_leaves_count {
            update_leaves.push((key, value));
        }
        smt.update(key, value).expect("SMT update leave error");
    }
    let root_hash = smt.root().clone();

    let mut root_hash_bytes = [0u8; 32];
    root_hash_bytes.copy_from_slice(root_hash.as_slice());

    let mint_merkle_proof = smt
        .merkle_proof(update_leaves.iter().map(|leave| leave.0).collect())
        .unwrap();
    let mint_merkle_proof_compiled = mint_merkle_proof.compile(update_leaves.clone()).unwrap();
    let verify_result = mint_merkle_proof_compiled
        .verify::<Blake2bHasher>(&root_hash, update_leaves.clone())
        .expect("smt proof verify failed");
    assert!(verify_result, "smt proof verify failed");

    let mint_smt_proof: Vec<u8> = mint_merkle_proof_compiled.into();

    (
        root_hash_bytes,
        mint_nft_keys,
        mint_nft_values,
        mint_smt_proof,
    )
}

fn generate_compact_nft_smt_data(
    input_compact_nft_out_point: OutPoint,
    mint_nft_keys: Vec<CompactNFTId>,
    mint_nft_values: Vec<MintCompactNFTValue>,
    class_mint_proof: Vec<u8>,
    claim_leaves_count: usize,
) -> ([u8; 32], [u8; 32], Vec<u8>) {
    let leaves_count = 100;
    let mut smt = SMT::default();
    let mut rng = thread_rng();
    for _ in 0..leaves_count {
        let key: H256 = rng.gen::<[u8; 32]>().into();
        let value: H256 = H256::from([255u8; 32]);
        smt.update(key, value).expect("SMT update leave error");
    }

    let old_smt_root = smt.root().clone();
    let mut old_root_hash_bytes = [0u8; 32];
    old_root_hash_bytes.copy_from_slice(old_smt_root.as_slice());

    let mut owned_nft_keys: Vec<CompactNFTKey> = Vec::new();
    let mut owned_nft_values: Vec<CompactNFTInfo> = Vec::new();
    let mut claimed_nft_keys: Vec<ClaimedCompactNFTKey> = Vec::new();
    let mut claimed_nft_values: Vec<Byte32> = Vec::new();
    let mut update_leaves: Vec<(H256, H256)> = Vec::with_capacity(claim_leaves_count * 2);
    for index in 0..claim_leaves_count {
        // Generate owned_nft smt kv pairs
        let mint_nft_key = mint_nft_keys.get(index).unwrap().clone();
        let mut nft_id_vec = Vec::new();
        nft_id_vec.extend(&BYTE3_ZEROS);
        nft_id_vec.extend(&[1u8]);
        nft_id_vec.extend(&mint_nft_key.as_slice().to_vec());
        let mut nft_id_bytes = [0u8; 32];
        nft_id_bytes.copy_from_slice(&nft_id_vec);
        let mut key = H256::from(nft_id_bytes);

        let owned_nft_key = CompactNFTKeyBuilder::default()
            .smt_type(Byte::from(1u8))
            .nft_id(mint_nft_key.clone())
            .build();
        owned_nft_keys.push(owned_nft_key);

        let mint_nft_value = mint_nft_values.get(index).unwrap().clone();
        let mut owed_nft_value_vec = Vec::new();
        owed_nft_value_vec.extend(&BYTE22_ZEROS);
        owed_nft_value_vec.extend(mint_nft_value.nft_info().as_slice());
        let mut owned_nft_value = [0u8; 32];
        owned_nft_value.copy_from_slice(&owed_nft_value_vec);

        owned_nft_values.push(mint_nft_value.nft_info().clone());
        let mut value: H256 = H256::from(owned_nft_value);

        update_leaves.push((key, value));
        smt.update(key, value).expect("SMT update leave error");

        // Generate claimed_nft smt kv pairs
        let compact_out_point_vec = input_compact_nft_out_point
            .as_bytes()
            .slice(12..36)
            .to_vec()
            .iter()
            .map(|v| Byte::from(*v))
            .collect::<Vec<Byte>>();
        let mut compact_out_point_bytes: [Byte; 24] = [Byte::from(0u8); 24];
        compact_out_point_bytes.copy_from_slice(&compact_out_point_vec);
        let compact_out_point = OutPointBytesBuilder::default()
            .set(compact_out_point_bytes)
            .build();
        let nft_key_ = CompactNFTKeyBuilder::default()
            .nft_id(mint_nft_key.clone())
            .smt_type(Byte::from(2u8))
            .build();
        let claimed_nft_key = ClaimedCompactNFTKeyBuilder::default()
            .nft_key(nft_key_)
            .out_point(compact_out_point)
            .build();
        claimed_nft_keys.push(claimed_nft_key.clone());
        key = H256::from(blake2b_256(claimed_nft_key.as_slice()));

        value = H256::from([255u8; 32]);
        claimed_nft_values.push(
            Byte32Builder::default()
                .set([Byte::from(255u8); 32])
                .build(),
        );

        update_leaves.push((key, value));
        smt.update(key, value).expect("SMT update leave error");
    }
    let root_hash = smt.root().clone();

    let mut root_hash_bytes = [0u8; 32];
    root_hash_bytes.copy_from_slice(root_hash.as_slice());

    let claim_mint_merkle_proof = smt
        .merkle_proof(update_leaves.iter().map(|leave| leave.0).collect())
        .unwrap();
    let claim_mint_merkle_proof_compiled = claim_mint_merkle_proof
        .compile(update_leaves.clone())
        .unwrap();
    let verify_result = claim_mint_merkle_proof_compiled
        .verify::<Blake2bHasher>(&root_hash, update_leaves.clone())
        .expect("smt proof verify failed");

    assert!(verify_result, "smt proof verify failed");

    let merkel_proof_vec: Vec<u8> = claim_mint_merkle_proof_compiled.into();

    let merkel_proof_bytes = BytesBuilder::default()
        .extend(merkel_proof_vec.iter().map(|v| Byte::from(*v)))
        .build();

    let mint_merkel_proof_bytes = BytesBuilder::default()
        .extend(class_mint_proof.iter().map(|v| Byte::from(*v)))
        .build();

    let mint_entries = ClaimMintCompactNFTEntriesBuilder::default()
        .owned_nft_keys(
            CompactNFTKeyVecBuilder::default()
                .set(owned_nft_keys)
                .build(),
        )
        .owned_nft_values(
            OwnedCompactNFTValueVecBuilder::default()
                .set(owned_nft_values)
                .build(),
        )
        .claimed_nft_keys(
            ClaimedCompactNFTKeyVecBuilder::default()
                .set(claimed_nft_keys)
                .build(),
        )
        .claimed_nft_values(
            ClaimedCommpactNFTValueVecBuilder::default()
                .set(claimed_nft_values)
                .build(),
        )
        .proof(merkel_proof_bytes)
        .mint_proof(mint_merkel_proof_bytes)
        .build();

    (
        old_root_hash_bytes,
        root_hash_bytes,
        Vec::from(mint_entries.as_slice()),
    )
}

fn create_test_context(action: Action, compact_error: CompactError) -> (Context, TransactionView) {
    // deploy compact-nft-type script
    let mut context = Context::default();
    let compact_nft_bin: Bytes = Loader::default().load_binary("compact-nft-type");
    let compact_nft_out_point = context.deploy_cell(compact_nft_bin);
    let compact_nft_type_script_dep = CellDepBuilder::default()
        .out_point(compact_nft_out_point.clone())
        .build();

    let issuer_bin: Bytes = Loader::default().load_binary("issuer-type");
    let issuer_out_point = context.deploy_cell(issuer_bin);
    let issuer_type_script_dep = CellDepBuilder::default()
        .out_point(issuer_out_point.clone())
        .build();

    let smt_bin: Bytes = Loader::default().load_binary("ckb_smt");
    let smt_out_point = context.deploy_cell(smt_bin);
    let smt_dep = CellDepBuilder::default().out_point(smt_out_point).build();

    // deploy always_success script
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());

    // prepare scripts
    let lock_script = context
        .build_script(
            &always_success_out_point,
            Bytes::from(hex::decode("157a3633c3477d84b604a25e5fca5ca681762c10").unwrap()),
        )
        .expect("script");
    let lock_hash_160 = &blake2b_256(lock_script.as_slice())[0..20];

    let lock_script_dep = CellDepBuilder::default()
        .out_point(always_success_out_point)
        .build();

    let issuer_type_args = hex::decode("157a3633c3477d84b604a25e5fca5ca681762c10").unwrap();
    let issuer_type_script = context
        .build_script(&issuer_out_point, Bytes::from(issuer_type_args.clone()))
        .expect("script");
    let issuer_type_hash: [u8; 32] = issuer_type_script.clone().calc_script_hash().unpack();
    let mut class_type_args = issuer_type_hash[0..20].to_vec();
    let mut args_class_id = 8u32.to_be_bytes().to_vec();
    class_type_args.append(&mut args_class_id);

    let class_aggron_type_script = Script::new_builder()
        .code_hash(CLASS_TYPE_CODE_HASH.pack())
        .args(Bytes::copy_from_slice(&class_type_args[..]).pack())
        .hash_type(Byte::new(TYPE))
        .build();

    if compact_error == CompactError::CompactNFTClassDepError {
        class_type_args.sort();
    }
    let (class_mint_root, mint_nft_keys, mint_nft_values, class_mint_proof) =
        generate_class_mint_smt_data(class_type_args, lock_script.clone(), 2);

    let class_input_data = {
        let mut data = hex::decode("01000000ff0000000500000155000266660003898989").unwrap();
        data.extend(&class_mint_root[..]);
        Bytes::from(data)
    };

    let class_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .type_(Some(class_aggron_type_script.clone()).pack())
            .build(),
        class_input_data.clone(),
    );
    let class_cell_dep = CellDepBuilder::default()
        .out_point(class_input_out_point.clone())
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

    let compact_nft_type_args = match compact_error {
        CompactError::CompactTypeArgsNotEqualLockHash => {
            let error_lock_hash = [0u8; 20];
            Bytes::copy_from_slice(&error_lock_hash[..])
        }
        _ => Bytes::copy_from_slice(lock_hash_160),
    };

    let compact_nft_type_script = context
        .build_script(&compact_nft_out_point, compact_nft_type_args)
        .expect("script");

    let compact_nft_input_out_point = random_out_point();

    let class_mint_smt_proof =
        if compact_error == CompactError::CompactClassMintSMTProofVerifyFailed {
            class_mint_proof[1..].to_vec()
        } else {
            class_mint_proof.to_vec()
        };
    let out_point = if compact_error == CompactError::CompactNFTOutPointInvalid {
        random_out_point()
    } else {
        compact_nft_input_out_point.clone()
    };
    let (old_root_hash, root_hash, witness_data) = generate_compact_nft_smt_data(
        out_point,
        mint_nft_keys,
        mint_nft_values,
        class_mint_smt_proof,
        2,
    );

    let mut compact_nft_data_vec: Vec<u8> = vec![];
    let version = [0u8];
    compact_nft_data_vec.extend(&version);
    compact_nft_data_vec.extend(&old_root_hash[..]);

    context.create_cell_with_out_point(
        compact_nft_input_out_point.clone(),
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .type_(Some(compact_nft_type_script.clone()).pack())
            .build(),
        Bytes::from(compact_nft_data_vec),
    );

    let compact_nft_input = CellInput::new_builder()
        .previous_output(compact_nft_input_out_point.clone())
        .build();

    let inputs = match action {
        Action::Create => vec![normal_input.clone()],
        _ => vec![compact_nft_input.clone()],
    };

    let outputs = match action {
        Action::Destroy => vec![CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .build()],
        Action::Create => vec![CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .type_(Some(compact_nft_type_script.clone()).pack())
            .build()],
        Action::Update => vec![CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .type_(Some(compact_nft_type_script.clone()).pack())
            .build()],
    };

    let outputs_data: Vec<Bytes> = match compact_error {
        CompactError::SMTProofVerifyFailed => vec![Bytes::from(
            hex::decode("0054dfaba38275883ef9b6d5fb02053b71dbba19630ff5f2ec01d5d6965366c1f7")
                .unwrap(),
        )],
        CompactError::CompactNFTSMTRootError => {
            let mut data_vec = vec![];
            let version = [0u8];
            data_vec.extend(&version);
            data_vec.extend(&root_hash[2..]);
            vec![Bytes::from(data_vec)]
        }
        _ => {
            let mut data_vec = vec![];
            let version = [0u8];
            data_vec.extend(&version);
            data_vec.extend(&root_hash[..]);
            vec![Bytes::from(data_vec)]
        }
    };

    let error_witness_args = WitnessArgsBuilder::default()
        .input_type(
            Some(Bytes::from(
                hex::decode("0154dfaba38275883ef9b6d5fb02053b71dbba19630ff5f2ec01d5d6965366c1f7")
                    .unwrap(),
            ))
            .pack(),
        )
        .build();

    let mut witness_data_vec = vec![];
    witness_data_vec.extend(&[CLAIM_MINT]);
    witness_data_vec.extend(&witness_data);
    let witness_args = WitnessArgsBuilder::default()
        .input_type(Some(Bytes::from(witness_data_vec)).pack())
        .build();

    let witnesses = match compact_error {
        CompactError::WitnessTypeParseError => vec![error_witness_args.as_bytes()],
        _ => vec![witness_args.as_bytes()],
    };

    let cell_deps = vec![
        class_cell_dep,
        lock_script_dep,
        issuer_type_script_dep,
        compact_nft_type_script_dep,
        smt_dep,
    ];

    // build transaction
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_deps(cell_deps)
        .witnesses(witnesses.pack())
        .build();
    (context, tx)
}

#[test]
fn test_create_compact_nft_cell_success() {
    let (mut context, tx) = create_test_context(Action::Create, CompactError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_update_compact_nft_cell_success() {
    let (mut context, tx) = create_test_context(Action::Update, CompactError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_destroy_compact_nft_cell_error() {
    let (mut context, tx) = create_test_context(Action::Destroy, CompactError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, COMPACT_CELL_POSITION_ERROR);
}

#[test]
fn test_compact_type_args_not_equal_lock_hash_error() {
    let (mut context, tx) = create_test_context(
        Action::Update,
        CompactError::CompactTypeArgsNotEqualLockHash,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, COMPACT_TYPE_ARGS_NOT_EQUAL_LOCK_HASH);
}

#[test]
fn test_compact_smt_proof_verify_error() {
    let (mut context, tx) = create_test_context(Action::Update, CompactError::SMTProofVerifyFailed);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, SMT_PROOF_VERIFY_FAILED);
}

#[test]
fn test_compact_class_mint_smt_proof_verify_error() {
    let (mut context, tx) = create_test_context(
        Action::Update,
        CompactError::CompactClassMintSMTProofVerifyFailed,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, COMPACT_CLASS_MINT_SMT_PROOF_VERIFY_FAILED);
}

#[test]
fn test_compact_nft_witness_type_parse_error() {
    let (mut context, tx) =
        create_test_context(Action::Update, CompactError::WitnessTypeParseError);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, WITNESS_TYPE_PARSE_ERROR);
}

#[test]
fn test_compact_nft_smt_root_error() {
    let (mut context, tx) =
        create_test_context(Action::Update, CompactError::CompactNFTSMTRootError);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, COMPACT_NFT_SMT_ROOT_ERROR);
}

#[test]
fn test_compact_nft_class_dep_error() {
    let (mut context, tx) =
        create_test_context(Action::Update, CompactError::CompactNFTClassDepError);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, COMPACT_NFT_CLASS_DEP_ERROR);
}

#[test]
fn test_compact_nft_out_point_invalid_error() {
    let (mut context, tx) =
        create_test_context(Action::Update, CompactError::CompactNFTOutPointInvalid);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, COMPACT_NFT_OUT_POINT_INVALID);
}
