use super::*;
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};
use ckb_tool::ckb_error::assert_error_eq;
use ckb_tool::ckb_error::Error;
use ckb_tool::ckb_script::ScriptError;
use ckb_tool::ckb_types::{
    bytes::Bytes,
    core::{TransactionBuilder, TransactionView},
    packed::*,
    prelude::*,
};

const MAX_CYCLES: u64 = 10_000_000;

// error numbers
const CLASS_DATA_INVALID: i8 = 12;
const CLASS_TOTAL_SMALLER_THAN_ISSUED: i8 = 13;
const CLASS_CELLS_COUNT_ERROR: i8 = 14;
const CLASS_ISSUED_INVALID: i8 = 15;
const CLASS_IMMUTABLE_FIELDS_NOT_SAME: i8 = 16;
const CLASS_CELL_CANNOT_DESTROYED: i8 = 17;
const CLASS_ID_INCREASE_ERROR: i8 = 18;

#[derive(PartialEq)]
enum Action {
    Create,
    Update,
    Destroy,
}

enum ClassError {
    NoError,
    ClassDataInvalid,
    TotalSmallerThanIssued,
    ClassCellsCountError,
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

    // deploy always_success script
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());

    // prepare scripts
    let lock_script = context
        .build_script(&always_success_out_point, Default::default())
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

    let class_input_data = match action {
        Action::Update => {
            Bytes::from(hex::decode("000000000f0000000500000155000266660003898989").unwrap())
        }
        Action::Destroy => {
            Bytes::from(hex::decode("000000000f0000000000000155000266660003898989").unwrap())
        }
        Action::Create => Bytes::new(),
    };

    let mut class_type_args = issuer_type_args.clone().to_vec();
    let mut args_class_id = 8u32.to_be_bytes().to_vec();
    class_type_args.append(&mut args_class_id);

    let class_type_script = context
        .build_script(
            &class_out_point,
            Bytes::copy_from_slice(&class_type_args[..]),
        )
        .expect("script");

    let class_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .type_(Some(class_type_script.clone()).pack())
            .build(),
        class_input_data,
    );
    let class_input = CellInput::new_builder()
        .previous_output(class_input_out_point.clone())
        .build();

    let inputs = match action {
        Action::Create => vec![issuer_input],
        _ => vec![class_input],
    };

    let mut outputs = match action {
        Action::Create => vec![CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .type_(Some(issuer_type_script.clone()).pack())
            .build()],
        Action::Update => vec![CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .type_(Some(class_type_script.clone()).pack())
            .build()],
        Action::Destroy => vec![CellOutput::new_builder()
            .capacity(2000u64.pack())
            .lock(lock_script.clone())
            .build()],
    };

    match action {
        Action::Create => {
            for class_id in [10u32, 8u32, 9u32].iter() {
                let mut class_type_args = issuer_type_args.clone().to_vec();
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
        Action::Create => vec![
            Bytes::from(hex::decode("000000000b000000000000").unwrap()),
            Bytes::from(hex::decode("000000000f0000000000000155000266660003898989").unwrap()),
            Bytes::from(hex::decode("000000000f0000000000000155000266660003898989").unwrap()),
            Bytes::from(hex::decode("000000000f0000000000000155000266660003898989").unwrap()),
        ],
        Action::Update => match class_error {
            ClassError::ClassDataInvalid => vec![Bytes::from(
                hex::decode("000000000f000000050000015500026666").unwrap(),
            )],
            ClassError::TotalSmallerThanIssued => vec![Bytes::from(
                hex::decode("000000000f000000150000015500026666000489898949").unwrap(),
            )],
            _ => vec![Bytes::from(
                hex::decode("000000000f000000050000015500026666000489898949").unwrap(),
            )],
        },
        Action::Destroy => vec![Bytes::new()],
    };

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
        .cell_dep(class_type_script_dep)
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
    let (mut context, tx) = create_test_context(Action::Update, ClassError::NoError);

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
fn test_update_class_data_error() {
    let (mut context, tx) = create_test_context(Action::Update, ClassError::ClassDataInvalid);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(CLASS_DATA_INVALID).input_type_script(script_cell_index)
    );
}

#[test]
fn test_update_class_total_smaller_than_issued_error() {
    let (mut context, tx) = create_test_context(Action::Update, ClassError::TotalSmallerThanIssued);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(CLASS_TOTAL_SMALLER_THAN_ISSUED)
            .input_type_script(script_cell_index)
    );
}

#[test]
fn test_create_class_cells_count_error() {
    let (mut context, tx) = create_test_context(Action::Create, ClassError::ClassCellsCountError);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_indexes = [1, 2, 3];

    let errors = script_cell_indexes
        .iter()
        .map(|index| {
            ScriptError::ValidationFailure(CLASS_CELLS_COUNT_ERROR).output_type_script(*index)
        })
        .collect::<Vec<_>>();

    assert_errors_contain!(err, errors);
}
