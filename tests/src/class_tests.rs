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

const MAX_CYCLES: u64 = 10_000_000;

// error numbers
const TYPE_ARGS_INVALID: i8 = 7;
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
    ClassIssuedInvalid,
    ClassTotalNotSame,
    ClassConfigureNotSame,
    ClassNameNotSame,
    ClassDescriptionNotSame,
    ClassCellCannotDestroyed,
    ClassIdIncreaseError,
    ClassTypeArgsInvalid,
    TypeArgsClassIdNotSame,
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
        Action::Destroy => match class_error {
            ClassError::ClassCellCannotDestroyed => {
                Bytes::from(hex::decode("000000000f0000000500000155000266660003898989").unwrap())
            }
            _ => Bytes::from(hex::decode("000000000f0000000000000155000266660003898989").unwrap()),
        },
        Action::Create => Bytes::new(),
    };

    let issuer_type_hash: [u8; 32] = issuer_type_script.clone().calc_script_hash().unpack();
    let mut class_type_args = issuer_type_hash[0..20].to_vec();
    let mut args_class_id = match class_error {
        ClassError::ClassTypeArgsInvalid => 8u16.to_be_bytes().to_vec(),
        _ => 8u32.to_be_bytes().to_vec(),
    };
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

    let mut class_type_args = issuer_type_hash[0..20].to_vec();
    let mut args_class_id = match class_error {
        ClassError::TypeArgsClassIdNotSame => 6u32.to_be_bytes().to_vec(),
        ClassError::ClassTypeArgsInvalid => 8u16.to_be_bytes().to_vec(),
        _ => 8u32.to_be_bytes().to_vec(),
    };
    class_type_args.append(&mut args_class_id);

    let class_type_script = context
        .build_script(
            &class_out_point,
            Bytes::copy_from_slice(&class_type_args[..]),
        )
        .expect("script");

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
            let class_ids = match class_error {
                ClassError::ClassIdIncreaseError => [10u32, 8u32, 8u32],
                _ => [10u32, 8u32, 9u32],
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
                Bytes::from(hex::decode("000000000f0000000600000155000266660003898989").unwrap()),
                Bytes::from(hex::decode("000000000f0000000000000155000266660003898989").unwrap()),
                Bytes::from(hex::decode("000000000f0000000000000155000266660003898989").unwrap()),
            ],
            _ => vec![
                Bytes::from(hex::decode("000000000b000000000000").unwrap()),
                Bytes::from(hex::decode("000000000f0000000000000155000266660003898989").unwrap()),
                Bytes::from(hex::decode("000000000f0000000000000155000266660003898989").unwrap()),
                Bytes::from(hex::decode("000000000f0000000000000155000266660003898989").unwrap()),
            ],
        },
        Action::Update => match class_error {
            ClassError::ClassDataInvalid => vec![Bytes::from(
                hex::decode("000000000f000000050000015500026666").unwrap(),
            )],
            ClassError::TotalSmallerThanIssued => vec![Bytes::from(
                hex::decode("000000000f000000150000015500026666000489898949").unwrap(),
            )],
            ClassError::ClassIssuedInvalid => vec![Bytes::from(
                hex::decode("000000000f000000030000015500026666000489898949").unwrap(),
            )],
            ClassError::ClassTotalNotSame => vec![Bytes::from(
                hex::decode("000000002f0000000500000155000266660003898989").unwrap(),
            )],
            ClassError::ClassConfigureNotSame => vec![Bytes::from(
                hex::decode("000000000f0000000507000155000266660003898989").unwrap(),
            )],
            ClassError::ClassNameNotSame => vec![Bytes::from(
                hex::decode("000000000f00000005000001aa000266660003898989").unwrap(),
            )],
            ClassError::ClassDescriptionNotSame => vec![Bytes::from(
                hex::decode("000000000f0000000500000155000299990003898989").unwrap(),
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
fn test_update_class_data_len_error() {
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
fn test_update_class_data_error() {
    let (mut context, tx) = create_test_context(Action::Update, ClassError::TypeArgsClassIdNotSame);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    let errors = vec![
        ScriptError::ValidationFailure(CLASS_DATA_INVALID).input_type_script(script_cell_index),
        ScriptError::ValidationFailure(CLASS_DATA_INVALID).output_type_script(script_cell_index),
    ];
    assert_errors_contain!(err, errors);
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

#[test]
fn test_create_class_issued_not_zero_error() {
    let (mut context, tx) = create_test_context(Action::Create, ClassError::ClassIssuedInvalid);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 1;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(CLASS_ISSUED_INVALID).output_type_script(script_cell_index)
    );
}

#[test]
fn test_update_class_issued_invalid_error() {
    let (mut context, tx) = create_test_context(Action::Update, ClassError::ClassIssuedInvalid);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(CLASS_ISSUED_INVALID).input_type_script(script_cell_index)
    );
}

#[test]
fn test_update_class_immutable_total_not_same_error() {
    let (mut context, tx) = create_test_context(Action::Update, ClassError::ClassTotalNotSame);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(CLASS_IMMUTABLE_FIELDS_NOT_SAME)
            .input_type_script(script_cell_index)
    );
}

#[test]
fn test_update_class_immutable_configure_not_same_error() {
    let (mut context, tx) = create_test_context(Action::Update, ClassError::ClassConfigureNotSame);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(CLASS_IMMUTABLE_FIELDS_NOT_SAME)
            .input_type_script(script_cell_index)
    );
}

#[test]
fn test_update_class_immutable_name_not_same_error() {
    let (mut context, tx) = create_test_context(Action::Update, ClassError::ClassNameNotSame);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(CLASS_IMMUTABLE_FIELDS_NOT_SAME)
            .input_type_script(script_cell_index)
    );
}

#[test]
fn test_update_class_immutable_description_not_same_error() {
    let (mut context, tx) =
        create_test_context(Action::Update, ClassError::ClassDescriptionNotSame);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(CLASS_IMMUTABLE_FIELDS_NOT_SAME)
            .input_type_script(script_cell_index)
    );
}

#[test]
fn test_class_cell_cannot_destroyed_error() {
    let (mut context, tx) =
        create_test_context(Action::Destroy, ClassError::ClassCellCannotDestroyed);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(CLASS_CELL_CANNOT_DESTROYED)
            .input_type_script(script_cell_index)
    );
}

#[test]
fn test_create_class_cells_increase_error() {
    let (mut context, tx) = create_test_context(Action::Create, ClassError::ClassIdIncreaseError);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_indexes = [1, 2, 3];

    let errors = script_cell_indexes
        .iter()
        .map(|index| {
            ScriptError::ValidationFailure(CLASS_ID_INCREASE_ERROR).output_type_script(*index)
        })
        .collect::<Vec<_>>();

    assert_errors_contain!(err, errors);
}

#[test]
fn test_update_class_type_args_invalid_error() {
    let (mut context, tx) = create_test_context(Action::Update, ClassError::ClassTypeArgsInvalid);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(TYPE_ARGS_INVALID).input_type_script(script_cell_index)
    );
}
