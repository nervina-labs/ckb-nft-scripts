use super::*;
use crate::constants::BYTE4_ZEROS;
use ckb_testtool::ckb_types::{
    bytes::Bytes,
    core::{TransactionBuilder, TransactionView},
    packed::*,
    prelude::*,
};
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};
use nft_smt::smt::blake2b_256;
use nft_smt::{
    common::{BytesBuilder, Uint32Builder, *},
    mint::*,
    smt::{Blake2bHasher, H256, SMT},
};
use rand::{thread_rng, Rng};

const MAX_CYCLES: u64 = 70_000_000;

// error numbers
const ENCODING: i8 = 4;
const TYPE_ARGS_INVALID: i8 = 7;
const CLASS_DATA_INVALID: i8 = 12;
const CLASS_TOTAL_SMALLER_THAN_ISSUED: i8 = 13;
const CLASS_CELLS_COUNT_ERROR: i8 = 14;
const CLASS_ISSUED_INVALID: i8 = 15;
const CLASS_IMMUTABLE_FIELDS_NOT_SAME: i8 = 16;
const CLASS_CELL_CANNOT_DESTROYED: i8 = 17;
const CLASS_ID_INCREASE_ERROR: i8 = 18;
const NFT_AND_CLASS_CONFIGURE_NOT_SAME: i8 = 22;
const GROUP_INPUT_WITNESS_NONE_ERROR: i8 = 37;
const COMPACT_ISSUER_ID_OR_CLASS_ID_INVALID: i8 = 42;
const CLASS_COMPACT_MINT_SMT_ROOT_ERROR: i8 = 43;

#[derive(PartialEq, Eq, Clone, Copy)]
enum UpdateCase {
    Default,
    Batch,
    Compact,
}

#[derive(PartialEq)]
enum Action {
    Create,
    Update(UpdateCase),
    Destroy,
}

#[derive(PartialEq)]
enum ClassError {
    NoError,
    ClassDataInvalid,
    TotalSmallerThanIssued,
    ClassCellsCountError,
    ClassIssuedInvalid,
    ClassTotalNotSame,
    ClassConfigureNotSame,
    ClassNameNotSame,
    ClassDescriptionNotSame,
    ClassCellCannotDestroyed,
    ClassIdIncreaseError,
    ClassTypeArgsInvalid,
    TypeArgsClassIdNotSame,
    GroupInputWitnessNoneError,
    CompactIssuerIdOrClassIdInvalid,
    ClassCompactMintSmtRootError,
    NFTAndClassConfigureNotSame,
}

fn generate_smt_data(
    class_error: &ClassError,
    class_type_args: Vec<u8>,
    receiver_lock_script: Script,
) -> ([u8; 32], [u8; 32], Vec<u8>) {
    if class_error == &ClassError::ClassTypeArgsInvalid {
        return ([0u8; 32], [0u8; 32], vec![0, 0, 0, 0]);
    }
    let class_type_args_bytes = if class_error == &ClassError::CompactIssuerIdOrClassIdInvalid {
        Vec::from([Byte::from(0u8); 24])
    } else {
        class_type_args
            .iter()
            .map(|v| Byte::from(*v))
            .collect::<Vec<Byte>>()
    };
    let leaves_count = 100;
    let update_leaves_count = 100;
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

    let mut nft_keys: Vec<CompactNFTId> = Vec::new();
    let mut nft_values: Vec<MintCompactNFTValue> = Vec::new();
    let mut update_leaves: Vec<(H256, H256)> = Vec::with_capacity(update_leaves_count);
    for index in 0..update_leaves_count {
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
        nft_keys.push(nft_id.clone());
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
        let configure = if class_error == &ClassError::NFTAndClassConfigureNotSame {
            1u8
        } else {
            0u8
        };
        let nft_info = CompactNFTInfoBuilder::default()
            .characteristic(characteristic)
            .configure(Byte::from(configure))
            .state(Byte::from(0u8))
            .build();
        let nft_value = MintCompactNFTValueBuilder::default()
            .nft_info(nft_info.clone())
            .receiver_lock(BytesBuilder::default().set(receiver_lock).build())
            .build();

        nft_values.push(nft_value.clone());

        let value: H256 = H256::from(blake2b_256(nft_value.as_slice()));
        update_leaves.push((key, value));
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

    let merkel_proof_vec: Vec<u8> = mint_merkle_proof_compiled.into();

    let merkel_proof_bytes = BytesBuilder::default()
        .extend(merkel_proof_vec.iter().map(|v| Byte::from(*v)))
        .build();

    let mint_entries = MintCompactNFTEntriesBuilder::default()
        .nft_keys(MintCompactNFTKeyVecBuilder::default().set(nft_keys).build())
        .nft_values(
            MintCompactNFTValueVecBuilder::default()
                .set(nft_values)
                .build(),
        )
        .proof(merkel_proof_bytes)
        .build();

    (
        old_root_hash_bytes,
        root_hash_bytes,
        Vec::from(mint_entries.as_slice()),
    )
}

fn create_test_context(action: Action, class_error: ClassError) -> (Context, TransactionView) {
    // deploy contract
    let mut context = Context::default();

    let class_bin: Bytes = Loader::default().load_binary("class-type");
    let class_out_point = context.deploy_cell(class_bin);
    let class_type_script_dep = CellDep::new_builder()
        .out_point(class_out_point.clone())
        .build();

    let issuer_bin: Bytes = Loader::default().load_binary("issuer-type");
    let issuer_out_point = context.deploy_cell(issuer_bin);
    let issuer_type_script_dep = CellDep::new_builder()
        .out_point(issuer_out_point.clone())
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
    let receiver_lock_script = context
        .build_script(
            &always_success_out_point,
            Bytes::from(hex::decode("157a3633c3477d84b604a25e5fca5ca681762c10").unwrap()),
        )
        .expect("script");
    let lock_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point)
        .build();

    let issuer_type_args = hex::decode("157a3633c3477d84b604a25e5fca5ca681762c10").unwrap();
    let issuer_type_script = context
        .build_script(&issuer_out_point, Bytes::from(issuer_type_args.clone()))
        .expect("script");

    // prepare cells
    let issuer_input_data = match class_error {
        ClassError::ClassCellsCountError => {
            Bytes::from(hex::decode("0000000005000000000000").unwrap())
        }
        _ => Bytes::from(hex::decode("0000000008000000000000").unwrap()),
    };
    let issuer_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(2000u64.pack())
            .lock(lock_script.clone())
            .type_(Some(issuer_type_script.clone()).pack())
            .build(),
        issuer_input_data,
    );
    let issuer_input = CellInput::new_builder()
        .previous_output(issuer_input_out_point.clone())
        .build();

    let issuer_type_hash: [u8; 32] = issuer_type_script.clone().calc_script_hash().unpack();
    let mut class_type_args = issuer_type_hash[0..20].to_vec();
    let mut args_class_id = match class_error {
        ClassError::ClassTypeArgsInvalid => 8u16.to_be_bytes().to_vec(),
        _ => 8u32.to_be_bytes().to_vec(),
    };
    class_type_args.append(&mut args_class_id);

    let mut another_class_type_args = issuer_type_hash[0..20].to_vec();
    let mut args_class_id = 9u32.to_be_bytes().to_vec();
    another_class_type_args.append(&mut args_class_id);

    let class_type_script = context
        .build_script(
            &class_out_point,
            Bytes::copy_from_slice(&class_type_args[..]),
        )
        .expect("script");

    let (old_smt_root, smt_root, witness_data) =
        generate_smt_data(&class_error, class_type_args, receiver_lock_script);

    let class_input_data = match action {
        Action::Update(_) => {
            let mut data = hex::decode("01000000ff0000000500000155000266660003898989").unwrap();
            data.extend(&old_smt_root[..]);
            Bytes::from(data)
        }
        Action::Destroy => match class_error {
            ClassError::ClassCellCannotDestroyed => {
                Bytes::from(hex::decode("010000000f0000000500000155000266660003898989").unwrap())
            }
            _ => Bytes::from(hex::decode("010000000f0000000000000155000266660003898989").unwrap()),
        },
        Action::Create => Bytes::new(),
    };

    let class_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .type_(Some(class_type_script.clone()).pack())
            .build(),
        class_input_data.clone(),
    );
    let class_input = CellInput::new_builder()
        .previous_output(class_input_out_point.clone())
        .build();

    let another_class_type_script = context
        .build_script(
            &class_out_point,
            Bytes::copy_from_slice(&another_class_type_args[..]),
        )
        .expect("script");

    let another_class_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .type_(Some(another_class_type_script.clone()).pack())
            .build(),
        class_input_data,
    );
    let another_class_input = CellInput::new_builder()
        .previous_output(another_class_input_out_point.clone())
        .build();

    let inputs = match action {
        Action::Create => vec![issuer_input],
        Action::Destroy => vec![class_input.clone(), class_input],
        Action::Update(case) => match case {
            UpdateCase::Default => vec![class_input],
            UpdateCase::Batch => vec![class_input, another_class_input],
            UpdateCase::Compact => vec![class_input],
        },
    };

    let mut class_type_args = issuer_type_hash[0..20].to_vec();
    let mut args_class_id = match class_error {
        ClassError::TypeArgsClassIdNotSame => 6u32.to_be_bytes().to_vec(),
        ClassError::ClassTypeArgsInvalid => 8u16.to_be_bytes().to_vec(),
        _ => 8u32.to_be_bytes().to_vec(),
    };
    class_type_args.append(&mut args_class_id);

    let mut another_class_type_args = issuer_type_hash[0..20].to_vec();
    let mut args_class_id = 9u32.to_be_bytes().to_vec();
    another_class_type_args.append(&mut args_class_id);

    let class_type_script = context
        .build_script(
            &class_out_point,
            Bytes::copy_from_slice(&class_type_args[..]),
        )
        .expect("script");

    let another_class_type_script = context
        .build_script(
            &class_out_point,
            Bytes::copy_from_slice(&another_class_type_args[..]),
        )
        .expect("script");

    let mut outputs = match action {
        Action::Create => vec![CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .type_(Some(issuer_type_script.clone()).pack())
            .build()],
        Action::Update(case) => match case {
            UpdateCase::Default => vec![CellOutput::new_builder()
                .capacity(500u64.pack())
                .lock(lock_script.clone())
                .type_(Some(class_type_script.clone()).pack())
                .build()],
            UpdateCase::Batch => vec![
                CellOutput::new_builder()
                    .capacity(500u64.pack())
                    .lock(lock_script.clone())
                    .type_(Some(class_type_script.clone()).pack())
                    .build(),
                CellOutput::new_builder()
                    .capacity(500u64.pack())
                    .lock(lock_script.clone())
                    .type_(Some(another_class_type_script.clone()).pack())
                    .build(),
            ],
            UpdateCase::Compact => vec![CellOutput::new_builder()
                .capacity(500u64.pack())
                .lock(lock_script.clone())
                .type_(Some(class_type_script.clone()).pack())
                .build()],
        },
        Action::Destroy => vec![CellOutput::new_builder()
            .capacity(2000u64.pack())
            .lock(lock_script.clone())
            .build()],
    };

    match action {
        Action::Create => {
            let class_ids = match class_error {
                ClassError::ClassIdIncreaseError => [10u32, 8u32, 9u32],
                _ => [8u32, 9u32, 10u32],
            };
            for class_id in class_ids.iter() {
                let mut class_type_args = issuer_type_hash[0..20].to_vec();
                let mut args_class_id = class_id.to_be_bytes().to_vec();
                class_type_args.append(&mut args_class_id);

                let class_type_script = context
                    .build_script(
                        &class_out_point,
                        Bytes::copy_from_slice(&class_type_args[..]),
                    )
                    .expect("script");

                outputs.push(
                    CellOutput::new_builder()
                        .capacity(500u64.pack())
                        .lock(lock_script.clone())
                        .type_(Some(class_type_script.clone()).pack())
                        .build(),
                );
            }
        }
        _ => (),
    }

    let outputs_data: Vec<_> = match action {
        Action::Create => match class_error {
            ClassError::ClassIssuedInvalid => vec![
                Bytes::from(hex::decode("000000000b000000000000").unwrap()),
                Bytes::from(hex::decode("010000000f0000000600000155000266660003898989").unwrap()),
                Bytes::from(hex::decode("010000000f0000000000000155000266660003898989").unwrap()),
                Bytes::from(hex::decode("010000000f0000000000000155000266660003898989").unwrap()),
            ],
            _ => vec![
                Bytes::from(hex::decode("000000000b000000000000").unwrap()),
                Bytes::from(hex::decode("010000000f0000000000000155000266660003898989").unwrap()),
                Bytes::from(hex::decode("010000000f0000000000000155000266660003898989").unwrap()),
                Bytes::from(hex::decode("010000000f0000000000000155000266660003898989").unwrap()),
            ],
        },
        Action::Update(case) => match class_error {
            ClassError::ClassDataInvalid => vec![Bytes::from(
                hex::decode("01000000ff000000050000015500026666").unwrap(),
            )],
            ClassError::TotalSmallerThanIssued => vec![Bytes::from(
                hex::decode("01000000ff000001ff0000015500026666000489898949").unwrap(),
            )],
            ClassError::ClassIssuedInvalid => vec![Bytes::from(
                hex::decode("01000000ff000000030000015500026666000489898949").unwrap(),
            )],
            ClassError::ClassTotalNotSame => vec![Bytes::from(
                hex::decode("010000002f0000000500000155000266660003898989").unwrap(),
            )],
            ClassError::ClassConfigureNotSame => vec![Bytes::from(
                hex::decode("01000000ff0000000507000155000266660003898989").unwrap(),
            )],
            ClassError::ClassNameNotSame => vec![Bytes::from(
                hex::decode("01000000ff00000005000001aa000266660003898989").unwrap(),
            )],
            ClassError::ClassDescriptionNotSame => vec![Bytes::from(
                hex::decode("01000000ff0000000500000155000299990003898989").unwrap(),
            )],
            _ => match case {
                UpdateCase::Default => vec![Bytes::from(
                    hex::decode("01000000ff000000050000015500026666000489898949").unwrap(),
                )],
                UpdateCase::Batch => vec![
                    Bytes::from(
                        hex::decode("01000000ff000000050000015500026666000489898949").unwrap(),
                    ),
                    Bytes::from(
                        hex::decode("01000000ff000000050000015500026666000489898949").unwrap(),
                    ),
                ],
                UpdateCase::Compact => {
                    let mut data = if class_error == ClassError::ClassIssuedInvalid {
                        hex::decode("01000000ff000000500000015500026666000489898949").unwrap()
                    } else {
                        hex::decode("01000000ff000000690000015500026666000489898949").unwrap()
                    };
                    if class_error == ClassError::ClassCompactMintSmtRootError {
                        data.extend(&smt_root[..30]);
                    } else {
                        data.extend(&smt_root[..]);
                    }
                    vec![Bytes::from(data)]
                }
            },
        },
        Action::Destroy => vec![Bytes::new()],
    };

    let mut witnesses = vec![];
    match action {
        Action::Update(case ) => {
            if case == UpdateCase::Compact {
                let lock = Some(Bytes::from(hex::decode("12345678").unwrap())).pack();
                let witness_args = WitnessArgsBuilder::default()
                    .lock(lock)
                    .input_type(Some(Bytes::from(witness_data)).pack())
                    .build();
                println!("witness_args length: {:?}", witness_args.as_slice().len());
                witnesses.push(Bytes::from(Vec::from(witness_args.clone().as_slice())));
            } else {
                witnesses.push(Bytes::from(hex::decode("550000001000000055000000550000004100000010f86974898b2f3685facb78741801bf2b932c7c548afe5bbc5d06ee135aeb792d700a02b62c492f1fd6e88afd655ffe305489fe9a76670a8999c641c8e2b16701").unwrap()))
            }
        }
        _ => witnesses.push(Bytes::from(hex::decode("550000001000000055000000550000004100000010f86974898b2f3685facb78741801bf2b932c7c548afe5bbc5d06ee135aeb792d700a02b62c492f1fd6e88afd655ffe305489fe9a76670a8999c641c8e2b16701").unwrap()))
    }
    if class_error == ClassError::GroupInputWitnessNoneError {
        witnesses[0] = Bytes::from("0x");
    }
    for _ in 1..inputs.len() {
        witnesses.push(Bytes::from("0x"))
    }

    // build transaction
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_dep(lock_script_dep)
        .cell_dep(issuer_type_script_dep)
        .cell_dep(class_type_script_dep)
        .cell_dep(smt_dep)
        .witnesses(witnesses.pack())
        .build();
    (context, tx)
}

#[test]
fn test_create_class_cells_success() {
    let (mut context, tx) = create_test_context(Action::Create, ClassError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_update_class_cell_success() {
    let (mut context, tx) =
        create_test_context(Action::Update(UpdateCase::Default), ClassError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_batch_update_class_cell_success() {
    let (mut context, tx) =
        create_test_context(Action::Update(UpdateCase::Batch), ClassError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_update_compact_class_cell_success() {
    let (mut context, tx) =
        create_test_context(Action::Update(UpdateCase::Compact), ClassError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_destroy_class_cell_success() {
    let (mut context, tx) = create_test_context(Action::Destroy, ClassError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_update_class_data_len_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::Default),
        ClassError::ClassDataInvalid,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, CLASS_DATA_INVALID);
}

#[test]
fn test_update_class_data_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::Default),
        ClassError::TypeArgsClassIdNotSame,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_errors(err, &[ENCODING, CLASS_DATA_INVALID]);
}

#[test]
fn test_update_class_with_witness_none_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::Default),
        ClassError::GroupInputWitnessNoneError,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, GROUP_INPUT_WITNESS_NONE_ERROR);
}

#[test]
fn test_update_class_total_smaller_than_issued_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::Default),
        ClassError::TotalSmallerThanIssued,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, CLASS_TOTAL_SMALLER_THAN_ISSUED);
}

#[test]
fn test_create_class_cells_count_error() {
    let (mut context, tx) = create_test_context(Action::Create, ClassError::ClassCellsCountError);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, CLASS_CELLS_COUNT_ERROR);
}

#[test]
fn test_create_class_issued_not_zero_error() {
    let (mut context, tx) = create_test_context(Action::Create, ClassError::ClassIssuedInvalid);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, CLASS_ISSUED_INVALID);
}

#[test]
fn test_update_class_issued_invalid_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::Default),
        ClassError::ClassIssuedInvalid,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, CLASS_ISSUED_INVALID);
}

#[test]
fn test_update_class_immutable_total_not_same_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::Default),
        ClassError::ClassTotalNotSame,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, CLASS_IMMUTABLE_FIELDS_NOT_SAME);
}

#[test]
fn test_update_class_immutable_configure_not_same_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::Default),
        ClassError::ClassConfigureNotSame,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, CLASS_IMMUTABLE_FIELDS_NOT_SAME);
}

#[test]
fn test_update_class_immutable_name_not_same_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::Default),
        ClassError::ClassNameNotSame,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, CLASS_IMMUTABLE_FIELDS_NOT_SAME);
}

#[test]
fn test_update_class_immutable_description_not_same_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::Default),
        ClassError::ClassDescriptionNotSame,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, CLASS_IMMUTABLE_FIELDS_NOT_SAME);
}

#[test]
fn test_class_cell_cannot_destroyed_error() {
    let (mut context, tx) =
        create_test_context(Action::Destroy, ClassError::ClassCellCannotDestroyed);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, CLASS_CELL_CANNOT_DESTROYED);
}

#[test]
fn test_create_class_cells_increase_error() {
    let (mut context, tx) = create_test_context(Action::Create, ClassError::ClassIdIncreaseError);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, CLASS_ID_INCREASE_ERROR);
}

#[test]
fn test_update_class_type_args_invalid_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::Default),
        ClassError::ClassTypeArgsInvalid,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, TYPE_ARGS_INVALID);
}

#[test]
fn test_destroy_class_with_witness_none_error() {
    let (mut context, tx) =
        create_test_context(Action::Destroy, ClassError::GroupInputWitnessNoneError);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, GROUP_INPUT_WITNESS_NONE_ERROR);
}

#[test]
fn test_update_class_compact_smt_root_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::Compact),
        ClassError::ClassCompactMintSmtRootError,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, CLASS_COMPACT_MINT_SMT_ROOT_ERROR);
}

#[test]
fn test_update_class_compact_issued_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::Compact),
        ClassError::ClassIssuedInvalid,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, CLASS_ISSUED_INVALID);
}

#[test]
fn test_update_class_compact_issuer_id_or_class_id_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::Compact),
        ClassError::CompactIssuerIdOrClassIdInvalid,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, COMPACT_ISSUER_ID_OR_CLASS_ID_INVALID);
}

#[test]
fn test_update_compact_nft_and_class_configure_not_same_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::Compact),
        ClassError::NFTAndClassConfigureNotSame,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, NFT_AND_CLASS_CONFIGURE_NOT_SAME);
}
