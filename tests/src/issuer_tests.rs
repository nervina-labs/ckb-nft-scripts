use crate::{assert_script_error, Loader};
use blake2b_rs::Blake2bBuilder;
use ckb_testtool::ckb_types::{
    bytes::Bytes,
    core::{TransactionBuilder, TransactionView},
    packed::*,
    prelude::*,
};
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};

const MAX_CYCLES: u64 = 10_000_000;

// error numbers
const ISSUER_DATA_INVALID: i8 = 5;
const ISSUER_CELLS_COUNT_ERROR: i8 = 6;
const TYPE_ARGS_INVALID: i8 = 7;
const ISSUER_CLASS_COUNT_ERROR: i8 = 8;
const ISSUER_SET_COUNT_ERROR: i8 = 9;
const ISSUER_CELL_CANNOT_DESTROYED: i8 = 10;
const VERSION_INVALID: i8 = 11;
const GROUP_INPUT_WITNESS_NONE_ERROR: i8 = 40;

#[derive(PartialEq)]
enum Action {
    Create,
    Update(u8),
    Destroy,
}

#[derive(PartialEq)]
enum IssuerError {
    NoError,
    DataLenInvalid,
    DataInfoLenInvalid,
    ClassCountInvalid,
    SetCountInvalid,
    VersionInvalid,
    TypeArgsInvalid,
    IssuerCellCannotDestroyed,
    GroupInputWitnessNoneError,
    BatchDestroyError,
}

fn create_test_context(action: Action, issuer_error: IssuerError) -> (Context, TransactionView) {
    // deploy contract
    let mut context = Context::default();
    let issuer_bin: Bytes = Loader::default().load_binary("issuer-type");
    let issuer_out_point = context.deploy_cell(issuer_bin);

    // deploy always_success script
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());

    // prepare scripts
    let lock_script = context
        .build_script(&always_success_out_point, Default::default())
        .expect("script");
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

    let mut blake2b = Blake2bBuilder::new(32)
        .personal(b"ckb-default-hash")
        .build();
    blake2b.update(normal_input.as_slice());
    blake2b.update(&(0u64).to_le_bytes());
    let mut ret = [0; 32];
    blake2b.finalize(&mut ret);
    let issuer_type_args = match issuer_error {
        IssuerError::TypeArgsInvalid => Bytes::copy_from_slice(&ret[0..10]),
        _ => Bytes::copy_from_slice(&ret[0..20]),
    };

    let issuer_type_script = context
        .build_script(&issuer_out_point, issuer_type_args)
        .expect("script");
    let issuer_type_script_dep = CellDep::new_builder()
        .out_point(issuer_out_point.clone())
        .build();

    let issuer_input_data = match issuer_error {
        IssuerError::IssuerCellCannotDestroyed => {
            Bytes::from(hex::decode("0000000000000000080000").unwrap())
        }
        _ => Bytes::from(hex::decode("0000000000000000000000").unwrap()),
    };

    let issuer_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .type_(Some(issuer_type_script.clone()).pack())
            .build(),
        issuer_input_data,
    );

    let mut inputs = vec![normal_input.clone()];
    match action {
        Action::Update(count) => {
            for _ in 0..count {
                inputs.push(
                    CellInput::new_builder()
                        .previous_output(issuer_input_out_point.clone())
                        .build(),
                );
            }
        }
        Action::Destroy => {
            inputs.push(
                CellInput::new_builder()
                    .previous_output(issuer_input_out_point.clone())
                    .build(),
            );
            if issuer_error == IssuerError::BatchDestroyError {
                inputs.push(
                    CellInput::new_builder()
                        .previous_output(issuer_input_out_point.clone())
                        .build(),
                );
            }
        }
        Action::Create => (),
    }

    let mut outputs = match action {
        Action::Create => vec![CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .type_(Some(issuer_type_script.clone()).pack())
            .build()],
        Action::Update(_) => vec![CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .build()],
        Action::Destroy => vec![CellOutput::new_builder()
            .capacity(1000u64.pack())
            .lock(lock_script.clone())
            .build()],
    };

    match action {
        Action::Update(count) => {
            for _ in 0..count {
                outputs.push(
                    CellOutput::new_builder()
                        .capacity(500u64.pack())
                        .lock(lock_script.clone())
                        .type_(Some(issuer_type_script.clone()).pack())
                        .build(),
                );
            }
        }
        _ => (),
    }

    let outputs_data: Vec<_> = outputs
        .iter()
        .map(|_output| match issuer_error {
            IssuerError::DataLenInvalid => Bytes::from(hex::decode("00000000000000").unwrap()),
            IssuerError::DataInfoLenInvalid => Bytes::from(
                hex::decode("00000000000000000000207b226e616d65223a22616c696365227d").unwrap(),
            ),
            IssuerError::ClassCountInvalid => {
                Bytes::from(hex::decode("0000000006000000000000").unwrap())
            }
            IssuerError::SetCountInvalid => {
                Bytes::from(hex::decode("0000000000000000080000").unwrap())
            }
            IssuerError::VersionInvalid => {
                Bytes::from(hex::decode("0100000000000000000000").unwrap())
            }
            _ => Bytes::from(hex::decode("0000000000000000000000").unwrap()),
        })
        .collect();

    let mut witnesses = vec![];
    match issuer_error {
        IssuerError::GroupInputWitnessNoneError => {
            witnesses.push(Bytes::from("0x"))
        }
        _ => {
            witnesses.push(Bytes::from(hex::decode("5500000010000000550000005500000041000000b69c542c0ee6c4b6d8350514d876ea7d8ef563e406253e959289457204447d2c4eb4e4a993073f5e76d244d2f93f7c108652e3295a9c8d72c12477e095026b9500").unwrap()))
        }
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
        .witnesses(witnesses.pack())
        .build();
    (context, tx)
}

#[test]
fn test_create_issuer_success() {
    let (mut context, tx) = create_test_context(Action::Create, IssuerError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_update_issuer_success() {
    let (mut context, tx) = create_test_context(Action::Update(1), IssuerError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_destroy_issuer_success() {
    let (mut context, tx) = create_test_context(Action::Destroy, IssuerError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_create_issuer_data_len_error() {
    let (mut context, tx) = create_test_context(Action::Create, IssuerError::DataLenInvalid);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ISSUER_DATA_INVALID);
}

#[test]
fn test_create_issuer_data_info_len_error() {
    let (mut context, tx) = create_test_context(Action::Create, IssuerError::DataInfoLenInvalid);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ISSUER_DATA_INVALID);
}

#[test]
fn test_update_issuer_cell_witness_none_error() {
    let (mut context, tx) =
        create_test_context(Action::Update(1), IssuerError::GroupInputWitnessNoneError);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, GROUP_INPUT_WITNESS_NONE_ERROR);
}

#[test]
fn test_update_issuer_cell_count_error() {
    let (mut context, tx) = create_test_context(Action::Update(2), IssuerError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ISSUER_CELLS_COUNT_ERROR);
}

#[test]
fn test_create_issuer_class_count_error() {
    let (mut context, tx) = create_test_context(Action::Create, IssuerError::ClassCountInvalid);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ISSUER_CLASS_COUNT_ERROR);
}

#[test]
fn test_create_issuer_set_count_error() {
    let (mut context, tx) = create_test_context(Action::Create, IssuerError::SetCountInvalid);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ISSUER_SET_COUNT_ERROR);
}

#[test]
fn test_create_issuer_version_error() {
    let (mut context, tx) = create_test_context(Action::Create, IssuerError::VersionInvalid);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, VERSION_INVALID);
}

#[test]
fn test_create_issuer_type_args_error() {
    let (mut context, tx) = create_test_context(Action::Create, IssuerError::TypeArgsInvalid);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, TYPE_ARGS_INVALID);
}

#[test]
fn test_destroy_issuer_error() {
    let (mut context, tx) =
        create_test_context(Action::Destroy, IssuerError::IssuerCellCannotDestroyed);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ISSUER_CELL_CANNOT_DESTROYED);
}

#[test]
fn test_destroy_issuer_with_witness_none_error() {
    let (mut context, tx) =
        create_test_context(Action::Destroy, IssuerError::GroupInputWitnessNoneError);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, GROUP_INPUT_WITNESS_NONE_ERROR);
}

#[test]
fn test_batch_destroy_issuer_error() {
    let (mut context, tx) = create_test_context(Action::Destroy, IssuerError::BatchDestroyError);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ISSUER_CELLS_COUNT_ERROR);
}
