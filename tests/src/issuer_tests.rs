use super::*;
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};
use ckb_tool::ckb_error::assert_error_eq;
use ckb_tool::ckb_hash::blake2b_256;
use ckb_tool::ckb_script::ScriptError;
use ckb_tool::ckb_types::{
    bytes::Bytes,
    core::{TransactionBuilder, TransactionView},
    packed::*,
    prelude::*,
};

const MAX_CYCLES: u64 = 10_000_000;

// error numbers
const ISSUER_DATA_INVALID: i8 = 5;
const ISSUER_CELLS_COUNT_ERROR: i8 = 6;
const TYPE_ARGS_INVALID: i8 = 7;
const ISSUER_CLASS_COUNT_ERROR: i8 = 8;
const ISSUER_SET_COUNT_ERROR: i8 = 9;
const ISSUER_CELL_CANNOT_DESTROYED: i8 = 10;
const VERSION_INVALID: i8 = 11;

#[derive(PartialEq)]
enum Action {
    Create,
    Update(u8),
    Destroy,
}

enum IssuerError {
    NoError,
    DataLenInvalid,
    ClassCountInvalid,
    SetCountInvalid,
    VersionInvalid,
    TypeArgsInvalid,
    IssuerCellCannotDestroyed,
}

pub fn blake160(data: &[u8]) -> [u8; 20] {
    let mut buf = [0u8; 20];
    let hash = blake2b_256(data);
    buf.clone_from_slice(&hash[..20]);
    buf
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

    let out_point_hash = blake160(&normal_input_out_point.as_slice());
    let issuer_type_args = match issuer_error {
        IssuerError::TypeArgsInvalid => Bytes::copy_from_slice(&out_point_hash[0..10]),
        _ => Bytes::copy_from_slice(&out_point_hash),
    };

    let issuer_type_script = context
        .build_script(&issuer_out_point, issuer_type_args)
        .expect("script");
    let issuer_type_script_dep = CellDep::new_builder().out_point(issuer_out_point).build();

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

    let mut inputs = vec![CellInput::new_builder()
        .previous_output(normal_input_out_point)
        .build()];
    match action {
        Action::Update(count) => {
            for _ in 0..count {
                inputs.push(
                    CellInput::new_builder()
                        .previous_output(issuer_input_out_point.clone())
                        .build(),
                );
            }
        },
        Action::Destroy => {
            inputs.push(
                CellInput::new_builder()
                    .previous_output(issuer_input_out_point.clone())
                    .build(),
            );
        },
        Action::Create => (),
    }

    let mut outputs = match action {
        Action::Create => vec![CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .type_(Some(issuer_type_script.clone()).pack())
            .build()],
        Action::Destroy => vec![CellOutput::new_builder()
            .capacity(1000u64.pack())
            .lock(lock_script.clone())
            .build()],
        _ => vec![CellOutput::new_builder()
            .capacity(500u64.pack())
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

    let witnesses = inputs
        .iter()
        .map(|_input| Bytes::from("0x"))
        .collect::<Vec<Bytes>>();

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
fn test_create_issuer_data_error() {
    let (mut context, tx) = create_test_context(Action::Create, IssuerError::DataLenInvalid);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(ISSUER_DATA_INVALID).output_type_script(script_cell_index)
    );
}

#[test]
fn test_update_issuer_cell_count_error() {
    let (mut context, tx) = create_test_context(Action::Update(2), IssuerError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 1;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(ISSUER_CELLS_COUNT_ERROR)
            .input_type_script(script_cell_index)
    );
}

#[test]
fn test_create_issuer_class_count_error() {
    let (mut context, tx) = create_test_context(Action::Create, IssuerError::ClassCountInvalid);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(ISSUER_CLASS_COUNT_ERROR)
            .output_type_script(script_cell_index)
    );
}

#[test]
fn test_create_issuer_set_count_error() {
    let (mut context, tx) = create_test_context(Action::Create, IssuerError::SetCountInvalid);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(ISSUER_SET_COUNT_ERROR)
            .output_type_script(script_cell_index)
    );
}

#[test]
fn test_create_issuer_version_error() {
    let (mut context, tx) = create_test_context(Action::Create, IssuerError::VersionInvalid);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(VERSION_INVALID).output_type_script(script_cell_index)
    );
}

#[test]
fn test_create_issuer_type_args_error() {
    let (mut context, tx) = create_test_context(Action::Create, IssuerError::TypeArgsInvalid);

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
fn test_destroy_issuer_error() {
    let (mut context, tx) =
        create_test_context(Action::Destroy, IssuerError::IssuerCellCannotDestroyed);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 1;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(ISSUER_CELL_CANNOT_DESTROYED)
            .input_type_script(script_cell_index)
    );
}
