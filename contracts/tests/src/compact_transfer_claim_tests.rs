use super::*;
use crate::constants::{
    BYTE22_ZEROS, BYTE3_ZEROS, CLAIMED_SMT_TYPE, OWNED_SMT_TYPE, WITHDRAWAL_SMT_TYPE,
};
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
    smt::{Blake2bHasher, H256, SMT},
    transfer::*,
};
use rand::{thread_rng, Rng};

const MAX_CYCLES: u64 = 70_000_000;

// error numbers
const WITNESS_TYPE_PARSE_ERROR: i8 = 38;
const SMT_PROOF_VERIFY_FAILED: i8 = 40;
const COMPACT_NFT_SMT_ROOT_ERROR: i8 = 45;
const COMPACT_NFT_WITHDRAW_DEP_ERROR: i8 = 50;
const CLAIMED_COMPACT_WITHDRAWAL_SMT_PROOF_VERIFY_FAILED: i8 = 51;

#[derive(PartialEq)]
enum CompactError {
    NoError,
    WitnessTypeParseError,
    SMTProofVerifyFailed,
    CompactNFTSMTRootError,
    CompactNFTWithdrawalDepError,
    ClaimedCompactWithdrawalSMTProofVerifyFailed,
}

const CLAIM_TRANSFER: u8 = 3;

fn generate_withdrawal_compact_nft_smt_data(
    input_compact_nft_out_point: OutPoint,
    to: [Byte; 20],
    class_type_args: Vec<u8>,
    withdrawal_leaves_count: usize,
) -> (
    [u8; 32],
    Vec<CompactNFTKey>,
    Vec<WithdrawCompactNFTValue>,
    Vec<u8>,
) {
    let class_type_args_bytes = class_type_args
        .iter()
        .map(|v| Byte::from(*v))
        .collect::<Vec<Byte>>();

    let leaves_count = 100;
    let mut withdrawal_smt = SMT::default();

    let mut rng = thread_rng();
    for _ in 0..leaves_count {
        let key: H256 = rng.gen::<[u8; 32]>().into();
        let value: H256 = H256::from([255u8; 32]);
        withdrawal_smt
            .update(key, value)
            .expect("SMT update leave error");
    }

    let mut owned_nft_keys: Vec<CompactNFTKey> = Vec::new();
    let mut owned_nft_values: Vec<CompactNFTInfo> = Vec::new();
    let mut withdrawal_nft_keys: Vec<CompactNFTKey> = Vec::new();
    let mut withdrawal_nft_values: Vec<WithdrawCompactNFTValue> = Vec::new();
    let mut update_leaves: Vec<(H256, H256)> = Vec::with_capacity(withdrawal_leaves_count * 2);

    let mut token_id_bytes = [Byte::from(0u8); 4];
    for index in 0..withdrawal_leaves_count {
        let mut issuer_id_bytes = [Byte::from(0); 20];
        issuer_id_bytes.copy_from_slice(&class_type_args_bytes[0..20]);
        let issuer_id = IssuerIdBuilder::default().set(issuer_id_bytes).build();

        let mut class_id_bytes = [Byte::from(0); 4];
        class_id_bytes.copy_from_slice(&class_type_args_bytes[20..24]);
        let class_id = Uint32Builder::default().set(class_id_bytes).build();

        token_id_bytes[3] = Byte::from((index + 10) as u8);
        let token_id = Uint32Builder::default().set(token_id_bytes).build();
        let nft_id = CompactNFTIdBuilder::default()
            .issuer_id(issuer_id)
            .class_id(class_id)
            .token_id(token_id)
            .build();
        let owned_nft_key = CompactNFTKeyBuilder::default()
            .nft_id(nft_id.clone())
            .smt_type(Byte::from(OWNED_SMT_TYPE))
            .build();
        let mut nft_id_vec = Vec::new();
        nft_id_vec.extend(&BYTE3_ZEROS);
        nft_id_vec.extend(&[OWNED_SMT_TYPE]);
        nft_id_vec.extend(nft_id.as_slice());
        let mut nft_id_bytes = [0u8; 32];
        nft_id_bytes.copy_from_slice(&nft_id_vec);

        let mut key = H256::from(nft_id_bytes);
        owned_nft_keys.push(owned_nft_key);

        let characteristic = CharacteristicBuilder::default()
            .set([Byte::from(0); 8])
            .build();
        let owned_nft_value = CompactNFTInfoBuilder::default()
            .characteristic(characteristic)
            .configure(Byte::from(0u8))
            .state(Byte::from(0u8))
            .build();
        let mut nft_info_vec = Vec::new();
        nft_info_vec.extend(&BYTE22_ZEROS);
        nft_info_vec.extend(owned_nft_value.as_slice());
        let mut nft_info_bytes = [0u8; 32];
        nft_info_bytes.copy_from_slice(&nft_info_vec);
        let mut value = H256::from([0u8; 32]);
        owned_nft_values.push(owned_nft_value.clone());

        withdrawal_smt
            .update(key, value)
            .expect("SMT update leave error");
        update_leaves.push((key, value));

        let withdrawal_nft_key = CompactNFTKeyBuilder::default()
            .smt_type(Byte::from(WITHDRAWAL_SMT_TYPE))
            .nft_id(nft_id)
            .build();
        nft_id_bytes[3] = WITHDRAWAL_SMT_TYPE;
        key = H256::from(nft_id_bytes);
        withdrawal_nft_keys.push(withdrawal_nft_key);

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
        let withdrawal_nft_value = WithdrawCompactNFTValueBuilder::default()
            .nft_info(owned_nft_value.clone())
            .out_point(compact_out_point)
            .to(LockHashBuilder::default().set(to).build())
            .build();
        withdrawal_nft_values.push(withdrawal_nft_value.clone());
        value = H256::from(blake2b_256(withdrawal_nft_value.as_slice()));

        withdrawal_smt
            .update(key, value)
            .expect("SMT update leave error");
        update_leaves.push((key, value));
    }

    let root_hash = withdrawal_smt.root().clone();
    let mut root_hash_bytes = [0u8; 32];
    root_hash_bytes.copy_from_slice(root_hash.as_slice());

    let withdrawal_compact_merkle_proof = withdrawal_smt
        .merkle_proof(update_leaves.iter().map(|leave| leave.0).collect())
        .unwrap();
    let withdrawal_compact_merkle_proof_compiled = withdrawal_compact_merkle_proof
        .compile(update_leaves.clone())
        .unwrap();

    let verify_result = withdrawal_compact_merkle_proof_compiled
        .verify::<Blake2bHasher>(&root_hash, update_leaves.clone())
        .expect("smt proof verify failed");
    assert!(verify_result, "withdrawal smt proof verify failed");

    let merkel_proof_vec: Vec<u8> = withdrawal_compact_merkle_proof_compiled.into();

    (
        root_hash_bytes,
        withdrawal_nft_keys,
        withdrawal_nft_values,
        merkel_proof_vec,
    )
}

fn generate_claimed_compact_nft_smt_data(
    withdrawal_compact_nft_out_point: OutPoint,
    withdrawal_nft_keys: Vec<CompactNFTKey>,
    withdrawal_nft_values: Vec<WithdrawCompactNFTValue>,
    withdrawal_nft_proof: Vec<u8>,
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
        let withdrawal_nft_key = withdrawal_nft_keys.get(index).unwrap().clone();
        let mut nft_id_vec = Vec::new();
        nft_id_vec.extend(&BYTE3_ZEROS);
        nft_id_vec.extend(&[OWNED_SMT_TYPE]);
        nft_id_vec.extend(withdrawal_nft_key.nft_id().as_slice());
        let mut nft_id_bytes = [0u8; 32];
        nft_id_bytes.copy_from_slice(&nft_id_vec);
        let mut key = H256::from(nft_id_bytes);

        let owned_nft_key = CompactNFTKeyBuilder::default()
            .smt_type(Byte::from(OWNED_SMT_TYPE))
            .nft_id(withdrawal_nft_key.nft_id().clone())
            .build();
        owned_nft_keys.push(owned_nft_key);

        let withdrawal_nft_value = withdrawal_nft_values.get(index).unwrap().clone();
        let mut owned_nft_value_vec = Vec::new();
        owned_nft_value_vec.extend(&BYTE22_ZEROS);
        owned_nft_value_vec.extend(withdrawal_nft_value.nft_info().as_slice());
        let mut owned_nft_value_bytes = [0u8; 32];
        owned_nft_value_bytes.copy_from_slice(&owned_nft_value_vec);

        owned_nft_values.push(withdrawal_nft_value.nft_info().clone());
        let mut value: H256 = H256::from(owned_nft_value_bytes);

        update_leaves.push((key, value));
        smt.update(key, value).expect("SMT update leave error");

        // Generate claimed_nft smt kv pairs
        let compact_out_point_vec = withdrawal_compact_nft_out_point
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
            .nft_id(withdrawal_nft_key.nft_id())
            .smt_type(Byte::from(CLAIMED_SMT_TYPE))
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

    let withdraw_merkel_proof_bytes = BytesBuilder::default()
        .extend(withdrawal_nft_proof.iter().map(|v| Byte::from(*v)))
        .build();

    let claim_entries = ClaimTransferCompactNFTEntriesBuilder::default()
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
        .withdrawal_proof(withdraw_merkel_proof_bytes)
        .build();

    (
        old_root_hash_bytes,
        root_hash_bytes,
        Vec::from(claim_entries.as_slice()),
    )
}

fn create_test_context(compact_error: CompactError) -> (Context, TransactionView) {
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
    let lock_hash = blake2b_256(lock_script.as_slice());
    let mut lock_hash_160_bytes = [Byte::from(0u8); 20];
    lock_hash_160_bytes.copy_from_slice(
        &lock_hash.clone()[0..20]
            .iter()
            .map(|v| Byte::from(*v))
            .collect::<Vec<Byte>>(),
    );
    let lock_hash_160 = lock_hash[0..20].to_vec();

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

    let compact_nft_type_args = Bytes::from(lock_hash_160);
    let compact_nft_type_script = context
        .build_script(&compact_nft_out_point, compact_nft_type_args)
        .expect("script");

    let mut withdraw_cell_dep_script = compact_nft_type_script.clone();

    if compact_error == CompactError::CompactNFTWithdrawalDepError {
        withdraw_cell_dep_script = compact_nft_type_script
            .clone()
            .as_builder()
            .hash_type(Byte::from(1u8))
            .build()
    };

    let compact_nft_input_out_point = random_out_point();

    let (withdraw_smt_root, withdraw_nft_keys, withdraw_nft_values, withdraw_smt_proof) =
        generate_withdrawal_compact_nft_smt_data(
            compact_nft_input_out_point.clone(),
            lock_hash_160_bytes,
            class_type_args,
            2,
        );

    let withdraw_cell_data = {
        let mut data = vec![0u8];
        data.extend(&withdraw_smt_root[..]);
        Bytes::from(data)
    };

    let withdraw_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .type_(Some(withdraw_cell_dep_script.clone()).pack())
            .build(),
        withdraw_cell_data.clone(),
    );

    let withdraw_cell_dep = CellDepBuilder::default()
        .out_point(withdraw_out_point)
        .build();

    let withdraw_nft_smt_proof =
        if compact_error == CompactError::ClaimedCompactWithdrawalSMTProofVerifyFailed {
            withdraw_smt_proof[1..].to_vec()
        } else {
            withdraw_smt_proof.to_vec()
        };
    let (old_root_hash, root_hash, witness_data) = generate_claimed_compact_nft_smt_data(
        compact_nft_input_out_point.clone(),
        withdraw_nft_keys,
        withdraw_nft_values,
        withdraw_nft_smt_proof,
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

    let inputs = vec![compact_nft_input.clone()];

    let outputs = vec![CellOutput::new_builder()
        .capacity(500u64.pack())
        .lock(lock_script.clone())
        .type_(Some(compact_nft_type_script.clone()).pack())
        .build()];

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
    witness_data_vec.extend(&[CLAIM_TRANSFER]);
    witness_data_vec.extend(&witness_data);
    let witness_args = WitnessArgsBuilder::default()
        .input_type(Some(Bytes::from(witness_data_vec)).pack())
        .build();

    let witnesses = match compact_error {
        CompactError::WitnessTypeParseError => vec![error_witness_args.as_bytes()],
        _ => vec![witness_args.as_bytes()],
    };

    let cell_deps = vec![
        withdraw_cell_dep,
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
fn test_claim_compact_nft_cell_success() {
    let (mut context, tx) = create_test_context(CompactError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_claim_compact_smt_proof_verify_error() {
    let (mut context, tx) = create_test_context(CompactError::SMTProofVerifyFailed);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, SMT_PROOF_VERIFY_FAILED);
}

#[test]
fn test_compact_withdrawal_smt_proof_verify_error() {
    let (mut context, tx) =
        create_test_context(CompactError::ClaimedCompactWithdrawalSMTProofVerifyFailed);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, CLAIMED_COMPACT_WITHDRAWAL_SMT_PROOF_VERIFY_FAILED);
}

#[test]
fn test_compact_nft_witness_type_parse_error() {
    let (mut context, tx) = create_test_context(CompactError::WitnessTypeParseError);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, WITNESS_TYPE_PARSE_ERROR);
}

#[test]
fn test_compact_nft_smt_root_error() {
    let (mut context, tx) = create_test_context(CompactError::CompactNFTSMTRootError);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, COMPACT_NFT_SMT_ROOT_ERROR);
}

#[test]
fn test_compact_nft_withdraw_dep_error() {
    let (mut context, tx) = create_test_context(CompactError::CompactNFTWithdrawalDepError);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, COMPACT_NFT_WITHDRAW_DEP_ERROR);
}
