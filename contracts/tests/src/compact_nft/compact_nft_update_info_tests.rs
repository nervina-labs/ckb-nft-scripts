use crate::constants::{BYTE22_ZEROS, BYTE3_ZEROS, OWNED_SMT_TYPE};
use crate::{assert_script_error, Loader};
use ckb_testtool::ckb_types::{
    bytes::Bytes,
    core::{TransactionBuilder, TransactionView},
    packed::*,
    prelude::*,
};
use ckb_testtool::context::random_out_point;
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};
use nft_smt::{
    common::{BytesBuilder, Uint32Builder, *},
    smt::{Blake2bHasher, H256, SMT},
    update::*,
};
use rand::{thread_rng, Rng};

const MAX_CYCLES: u64 = 70_000_000;

// error numbers
const NFT_CONFIGURE_NOT_SAME: i8 = 24;
const NFT_CLAIMED_TO_UNCLAIMED_ERROR: i8 = 25;
const NFT_DISALLOW_LOCKED: i8 = 28;
const LOCKED_NFT_CANNOT_UPDATE_CHARACTERISTIC: i8 = 36;
const WITNESS_TYPE_PARSE_ERROR: i8 = 38;
const SMT_PROOF_VERIFY_FAILED: i8 = 40;
const COMPACT_NFT_SMT_ROOT_ERROR: i8 = 45;
const COMPACT_NFT_SMT_TYPE: i8 = 58;

#[derive(PartialEq)]
enum UpdateError {
    NoError,
    WitnessTypeParseError,
    SMTProofVerifyFailed,
    CompactNFTSMTRootError,
    CompactNFTSmtTypeError,
    LockedNFTCannotUpdateCharacteristic,
    NFTConfigureNotSame,
    NFTClaimedToUnclaimedError,
    NFTDisallowLocked,
}

const UPDATE_INFO: u8 = 4;

fn generator_owned_nft_value(is_new: bool, update_error: &UpdateError) -> (H256, CompactNFTInfo) {
    let mut char = 0u8;
    let mut configure = 0u8;
    let mut state = 0u8;

    match update_error {
        &UpdateError::NFTConfigureNotSame => {
            if is_new {
                configure = 0;
            } else {
                configure = 1;
            }
        }
        &UpdateError::LockedNFTCannotUpdateCharacteristic => {
            state = 2;
            if is_new {
                char = 10;
            } else {
                char = 5;
            }
        }
        &UpdateError::NFTDisallowLocked => {
            configure = 2;
            if is_new {
                state = 2;
            } else {
                state = 0;
            }
        }
        &UpdateError::NFTClaimedToUnclaimedError => {
            if is_new {
                state = 0;
            } else {
                state = 1;
            }
        }
        _ => {
            if is_new {
                char = 10u8;
            } else {
                char = 5u8;
            }
        }
    }
    let characteristic = CharacteristicBuilder::default()
        .set([Byte::from(char); 8])
        .build();
    let owned_nft_value = CompactNFTInfoBuilder::default()
        .characteristic(characteristic)
        .configure(Byte::from(configure))
        .state(Byte::from(state))
        .build();
    let mut nft_info_vec = Vec::new();
    nft_info_vec.extend(&BYTE22_ZEROS);
    nft_info_vec.extend(owned_nft_value.as_slice());
    let mut nft_info_bytes = [0u8; 32];
    nft_info_bytes.copy_from_slice(&nft_info_vec);
    (H256::from(nft_info_bytes), owned_nft_value)
}

fn generate_update_compact_nft_info_smt_data(
    update_error: &UpdateError,
    class_type_args: Vec<u8>,
    update_leaves_count: usize,
) -> ([u8; 32], [u8; 32], Vec<u8>) {
    let class_type_args_bytes = class_type_args
        .iter()
        .map(|v| Byte::from(*v))
        .collect::<Vec<Byte>>();

    let leaves_count = 100;
    let mut before_update_smt = SMT::default();
    let mut after_update_smt = SMT::default();

    let mut rng = thread_rng();
    for _ in 0..leaves_count {
        let key: H256 = rng.gen::<[u8; 32]>().into();
        let value: H256 = H256::from([255u8; 32]);
        before_update_smt
            .update(key, value)
            .expect("SMT update leave error");
        after_update_smt
            .update(key, value)
            .expect("SMT update leave error");
    }

    let mut owned_nft_keys: Vec<CompactNFTKey> = Vec::new();
    let mut old_nft_values: Vec<CompactNFTInfo> = Vec::new();
    let mut new_nft_values: Vec<CompactNFTInfo> = Vec::new();
    let mut old_update_leaves: Vec<(H256, H256)> = Vec::with_capacity(update_leaves_count);
    let mut new_update_leaves: Vec<(H256, H256)> = Vec::with_capacity(update_leaves_count);

    let mut token_id_bytes = [Byte::from(0u8); 4];
    for index in 0..update_leaves_count {
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
        let smt_type = if update_error == &UpdateError::CompactNFTSmtTypeError {
            0u8
        } else {
            OWNED_SMT_TYPE
        };
        let owned_nft_key = CompactNFTKeyBuilder::default()
            .nft_id(nft_id.clone())
            .smt_type(Byte::from(smt_type))
            .build();
        let mut nft_id_vec = Vec::new();
        nft_id_vec.extend(&BYTE3_ZEROS);
        nft_id_vec.extend(&[smt_type]);
        nft_id_vec.extend(nft_id.as_slice());
        let mut nft_id_bytes = [0u8; 32];
        nft_id_bytes.copy_from_slice(&nft_id_vec);

        let key = H256::from(nft_id_bytes);
        owned_nft_keys.push(owned_nft_key);

        let (value, owned_nft_value) = generator_owned_nft_value(false, update_error);
        old_nft_values.push(owned_nft_value.clone());
        old_update_leaves.push((key, value));
        before_update_smt
            .update(key, value)
            .expect("SMT update leave error");

        let (value, owned_nft_value) = generator_owned_nft_value(true, update_error);
        new_nft_values.push(owned_nft_value.clone());
        new_update_leaves.push((key, value));
        after_update_smt
            .update(key, value)
            .expect("SMT update leave error");
    }

    let old_smt_root = before_update_smt.root().clone();
    let mut old_root_hash_bytes = [0u8; 32];
    old_root_hash_bytes.copy_from_slice(old_smt_root.as_slice());

    let root_hash = after_update_smt.root().clone();
    let mut root_hash_bytes = [0u8; 32];
    root_hash_bytes.copy_from_slice(root_hash.as_slice());

    let update_info_merkle_proof = after_update_smt
        .merkle_proof(old_update_leaves.iter().map(|leave| leave.0).collect())
        .unwrap();
    let update_info_merkle_proof_compiled = update_info_merkle_proof
        .compile(old_update_leaves.clone())
        .unwrap();
    let verify_result = update_info_merkle_proof_compiled
        .verify::<Blake2bHasher>(&old_smt_root, old_update_leaves.clone())
        .expect("smt proof verify failed");
    assert!(verify_result, "before updating smt proof verify failed");

    let verify_result = update_info_merkle_proof_compiled
        .verify::<Blake2bHasher>(&root_hash, new_update_leaves.clone())
        .expect("smt proof verify failed");
    assert!(verify_result, "after updating smt proof verify failed");

    let merkel_proof_vec: Vec<u8> = update_info_merkle_proof_compiled.into();
    let merkel_proof_bytes = BytesBuilder::default()
        .extend(merkel_proof_vec.iter().map(|v| Byte::from(*v)))
        .build();

    let update_entries = UpdateCompactNFTEntriesBuilder::default()
        .owned_nft_keys(
            CompactNFTKeyVecBuilder::default()
                .extend(owned_nft_keys)
                .build(),
        )
        .old_nft_values(
            OwnedCompactNFTValueVecBuilder::default()
                .extend(old_nft_values)
                .build(),
        )
        .new_nft_values(
            OwnedCompactNFTValueVecBuilder::default()
                .extend(new_nft_values)
                .build(),
        )
        .proof(merkel_proof_bytes)
        .build();

    (
        old_root_hash_bytes,
        root_hash_bytes,
        Vec::from(update_entries.as_slice()),
    )
}

fn create_test_context(update_error: UpdateError) -> (Context, TransactionView) {
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
    let lock_hash_160_vec = &lock_script.calc_script_hash().as_bytes()[0..20];

    let to_lock_script = context
        .build_script(
            &always_success_out_point,
            Bytes::from(hex::decode("7164f48d7a5bf2298166f8d81b81ea4e908e16ad").unwrap()),
        )
        .expect("script");
    let to_lock_hash_160_vec = &to_lock_script.calc_script_hash().as_bytes()[0..20];
    let mut to_lock_hash_160 = [Byte::from(0u8); 20];
    let to_lock_hash_bytes: Vec<Byte> = to_lock_hash_160_vec
        .to_vec()
        .iter()
        .map(|v| Byte::from(*v))
        .collect();
    to_lock_hash_160.copy_from_slice(&to_lock_hash_bytes);

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

    // prepare cells
    let compact_nft_type_script = context
        .build_script(
            &compact_nft_out_point,
            Bytes::copy_from_slice(lock_hash_160_vec),
        )
        .expect("script");

    let (old_root_hash, root_hash, witness_data) =
        generate_update_compact_nft_info_smt_data(&update_error, class_type_args, 5);

    let mut compact_nft_data_vec: Vec<u8> = vec![];
    let version = [0u8];
    compact_nft_data_vec.extend(&version);
    compact_nft_data_vec.extend(&old_root_hash[..]);

    let compact_nft_input_out_point = random_out_point();
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

    let outputs_data: Vec<Bytes> = match update_error {
        UpdateError::SMTProofVerifyFailed => vec![Bytes::from(
            hex::decode("0054dfaba38275883ef9b6d5fb02053b71dbba19630ff5f2ec01d5d6965366c1f7")
                .unwrap(),
        )],
        UpdateError::CompactNFTSMTRootError => {
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
    witness_data_vec.extend(&[UPDATE_INFO]);
    witness_data_vec.extend(&witness_data);
    let witness_args = WitnessArgsBuilder::default()
        .input_type(Some(Bytes::from(witness_data_vec)).pack())
        .build();

    let witnesses = match update_error {
        UpdateError::WitnessTypeParseError => vec![error_witness_args.as_bytes()],
        _ => vec![witness_args.as_bytes()],
    };

    let cell_deps = vec![
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
fn test_update_compact_nft_info_success() {
    let (mut context, tx) = create_test_context(UpdateError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_update_compact_smt_proof_verify_error() {
    let (mut context, tx) = create_test_context(UpdateError::SMTProofVerifyFailed);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, SMT_PROOF_VERIFY_FAILED);
}

#[test]
fn test_update_compact_nft_witness_type_parse_error() {
    let (mut context, tx) = create_test_context(UpdateError::WitnessTypeParseError);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, WITNESS_TYPE_PARSE_ERROR);
}

#[test]
fn test_update_compact_nft_smt_root_error() {
    let (mut context, tx) = create_test_context(UpdateError::CompactNFTSMTRootError);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, COMPACT_NFT_SMT_ROOT_ERROR);
}

#[test]
fn test_update_compact_nft_smt_type_error() {
    let (mut context, tx) = create_test_context(UpdateError::CompactNFTSmtTypeError);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, COMPACT_NFT_SMT_TYPE);
}

#[test]
fn test_update_locked_compact_nft_cannot_update_characteristic_error() {
    let (mut context, tx) = create_test_context(UpdateError::LockedNFTCannotUpdateCharacteristic);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, LOCKED_NFT_CANNOT_UPDATE_CHARACTERISTIC);
}

#[test]
fn test_update_compact_nft_disallow_locked_error() {
    let (mut context, tx) = create_test_context(UpdateError::NFTDisallowLocked);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, NFT_DISALLOW_LOCKED);
}

#[test]
fn test_update_compact_nft_claimed_to_unclaimed_error() {
    let (mut context, tx) = create_test_context(UpdateError::NFTClaimedToUnclaimedError);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, NFT_CLAIMED_TO_UNCLAIMED_ERROR);
}

#[test]
fn test_update_compact_nft_configure_not_same_error() {
    let (mut context, tx) = create_test_context(UpdateError::NFTConfigureNotSame);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, NFT_CONFIGURE_NOT_SAME);
}
