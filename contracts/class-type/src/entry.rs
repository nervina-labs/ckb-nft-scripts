use alloc::vec;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    high_level::{load_cell_data, load_script},
};
use core::result::Result;
use script_utils::{
    class::{Class, CLASS_TYPE_ARGS_LEN},
    error::Error,
    helper::{
        count_cells_by_type_args, load_cell_data_by_type_args, load_output_type_args_ids, Action,
    },
    issuer::{Issuer, ISSUER_TYPE_ARGS_LEN},
};

fn parse_class_action(args: &Bytes) -> Result<Action, Error> {
    let check_args_equal = |type_args: &Bytes| type_args[..] == args[..];
    let inputs_count = count_cells_by_type_args(Source::Input, &check_args_equal);
    let outputs_count = count_cells_by_type_args(Source::Output, &check_args_equal);

    match (inputs_count, outputs_count) {
        (0, _outputs_count) => Ok(Action::Create),
        (1, 1) => Ok(Action::Update),
        (1, 0) => Ok(Action::Destroy),
        _ => Err(Error::ClassCellsCountError),
    }
}

fn handle_creation(args: &Bytes) -> Result<(), Error> {
    let check_args_subset = |type_args: &Bytes| type_args[..] == args[0..ISSUER_TYPE_ARGS_LEN];
    let issuer_inputs_count = count_cells_by_type_args(Source::Input, &check_args_subset);
    let issuer_outputs_count = count_cells_by_type_args(Source::Output, &check_args_subset);
    if issuer_inputs_count != 1 || issuer_outputs_count != 1 {
        return Err(Error::IssuerCellsCountError);
    }
    let class = Class::from_data(args)?;
    if class.issued != 0 {
        return Err(Error::ClassIssuedInvalid);
    }
    let input_issuer = match load_cell_data_by_type_args(Source::Input, &check_args_subset) {
        Some(issuer_input_data) => Ok(Issuer::from_data(&issuer_input_data[..])?),
        None => Err(Error::IssuerDataInvalid),
    }?;
    let output_issuer = match load_cell_data_by_type_args(Source::Output, &check_args_subset) {
        Some(issuer_output_data) => Ok(Issuer::from_data(&issuer_output_data[..])?),
        None => Err(Error::IssuerDataInvalid),
    }?;

    if output_issuer.class_count < input_issuer.class_count {
        return Err(Error::IssuerClassCountError);
    }
    let mut issuer_cell_class_ids = vec![input_issuer.class_count];
    for class_id in (input_issuer.class_count + 1)..output_issuer.class_count {
        issuer_cell_class_ids.push(class_id);
    }

    let check_args_equal = |type_args: &Bytes| type_args[..] == args[..];
    let mut outputs_class_ids = load_output_type_args_ids(ISSUER_TYPE_ARGS_LEN, &check_args_equal);
    outputs_class_ids.sort();

    if &outputs_class_ids[..] == &issuer_cell_class_ids[..] {
        return Err(Error::ClassCellsCountError);
    }
    Ok(())
}

fn handle_update() -> Result<(), Error> {
    let class_input_data = load_cell_data(0, Source::GroupInput)?;
    let class_output_data = load_cell_data(0, Source::GroupOutput)?;
    let input_class = Class::from_data(&class_input_data[..])?;
    let output_class = Class::from_data(&class_output_data[..])?;

    if !input_class.immutable_equal(&output_class) {
        return Err(Error::ClassImmutableFieldsNotSame);
    }
    Ok(())
}

fn handle_destroying() -> Result<(), Error> {
    let class_input_data = load_cell_data(0, Source::GroupInput)?;
    let input_class = Class::from_data(&class_input_data[..])?;

    if input_class.issued > 0 {
        return Err(Error::ClassCellCannotDestroyed);
    }
    Ok(())
}

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();
    if args.len() != CLASS_TYPE_ARGS_LEN {
        return Err(Error::TypeArgsInvalid);
    }

    match parse_class_action(&args)? {
        Action::Create => handle_creation(&args),
        Action::Update => handle_update(),
        Action::Destroy => handle_destroying(),
    }
}
