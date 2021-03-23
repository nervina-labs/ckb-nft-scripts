use alloc::vec;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    high_level::load_script,
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
        (0, 0) => Err(Error::ClassCellsCountError),
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
    let mut class_cells_type_args_ids = vec![input_issuer.class_count];
    for id in (input_issuer.class_count + 1)..output_issuer.class_count {
        class_cells_type_args_ids.push(id);
    }

    let check_args_equal = |type_args: &Bytes| type_args[..] == args[..];
    let mut class_outputs_type_args_ids =
        load_output_type_args_ids(ISSUER_TYPE_ARGS_LEN, &check_args_equal);
    class_outputs_type_args_ids.sort();

    if &class_outputs_type_args_ids[..] == &class_cells_type_args_ids[..] {
        return Err(Error::ClassCellsCountError);
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
        Action::Update => Ok(()),
        Action::Destroy => Ok(()),
    }
}
