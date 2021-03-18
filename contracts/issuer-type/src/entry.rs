use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    high_level::{load_cell_data, load_input_out_point, load_script},
};
use core::result::Result;
use script_utils::{
    class::CLASS_TYPE_ARGS_LEN,
    error::Error,
    hash::blake2b_160,
    helper::count_cells_with_type_args,
    issuer::{Issuer, ISSUER_TYPE_ARGS_LEN},
};

enum Action {
    Create,
    Update,
    Destroy,
}

fn parse_issuer_action(args: &Bytes) -> Result<Action, Error> {
    let check_args_equal = |type_args: &Bytes| type_args[..] == args[..];
    let inputs_count = count_cells_with_type_args(Source::Input, &check_args_equal);
    let outputs_count = count_cells_with_type_args(Source::Output, &check_args_equal);

    if inputs_count > 1 || outputs_count > 1 || (inputs_count == 0 && outputs_count == 0) {
        return Err(Error::IssuerCellsCountError);
    }

    if inputs_count == 0 && outputs_count == 1 {
        Ok(Action::Create)
    } else if inputs_count == 1 && outputs_count == 1 {
        Ok(Action::Update)
    } else {
        Ok(Action::Destroy)
    }
}

fn handle_creation(args: &Bytes) -> Result<(), Error> {
    let out_point = load_input_out_point(0, Source::Input)?;
    if args[..] != blake2b_160(out_point.as_slice()) {
        return Err(Error::TypeArgsInvalid);
    }
    let issuer_cell_data = load_cell_data(0, Source::GroupOutput)?;
    let issuer = Issuer::from_data(&issuer_cell_data[..])?;
    if issuer.class_count != 0 || issuer.set_count != 0 {
        return Err(Error::IssuerClassCountOrSetCountError);
    }
    Ok(())
}

fn count_class_cell(args: &Bytes) -> usize {
    count_cells_with_type_args(Source::Input, &|type_args: &Bytes| {
        type_args.len() == CLASS_TYPE_ARGS_LEN && type_args[0..ISSUER_TYPE_ARGS_LEN] == args[..]
    })
}

fn handle_update(args: &Bytes) -> Result<(), Error> {
    let issuer_input_data = load_cell_data(0, Source::GroupInput)?;
    let issuer_output_data = load_cell_data(0, Source::GroupOutput)?;
    let input_issuer = Issuer::from_data(&issuer_input_data[..])?;
    let output_issuer = Issuer::from_data(&issuer_output_data[..])?;
    if input_issuer.set_count != 0 || output_issuer.set_count != 0 {
        return Err(Error::IssuerClassCountOrSetCountError);
    }
    if output_issuer.class_count < input_issuer.class_count {
        return Err(Error::IssuerClassCountOrSetCountError);
    }
    let class_cells_count = (output_issuer.class_count - input_issuer.class_count) as usize;
    let class_outputs_count = count_class_cell(args);
    if class_outputs_count != class_cells_count {
        return Err(Error::IssuerClassCountOrSetCountError);
    }
    Ok(())
}

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();

    match parse_issuer_action(&args)? {
        Action::Create => handle_creation(&args),
        Action::Update => handle_update(&args),
        Action::Destroy => Ok(()),
    }
}
