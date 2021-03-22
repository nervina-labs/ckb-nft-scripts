use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    high_level::{load_cell_data, load_input, load_script},
};
use blake2b_rs::Blake2bBuilder;
use core::result::Result;
use script_utils::{
    class::CLASS_TYPE_ARGS_LEN,
    error::Error,
    helper::{count_cells_with_type_args, load_output_index_by_type_args, Action},
    issuer::{Issuer, ISSUER_TYPE_ARGS_LEN},
};

fn parse_issuer_action(args: &Bytes) -> Result<Action, Error> {
    let check_args_equal = |type_args: &Bytes| type_args[..] == args[..];
    let inputs_count = count_cells_with_type_args(Source::Input, &check_args_equal);
    let outputs_count = count_cells_with_type_args(Source::Output, &check_args_equal);

    match (inputs_count, outputs_count) {
        (0, 1) => Ok(Action::Create),
        (1, 1) => Ok(Action::Update),
        (1, 0) => Ok(Action::Destroy),
        _ => Err(Error::IssuerCellsCountError),
    }
}

fn count_class_cell(args: &Bytes, source: Source) -> usize {
    count_cells_with_type_args(source, &|type_args: &Bytes| {
        type_args.len() == CLASS_TYPE_ARGS_LEN && type_args[0..ISSUER_TYPE_ARGS_LEN] == args[..]
    })
}

fn handle_creation(args: &Bytes) -> Result<(), Error> {
    let first_input = load_input(0, Source::Input)?;
    let fist_output_index = match load_output_index_by_type_args(args) {
        Some(index) => Ok(index),
        None => Err(Error::Encoding),
    }?;
    let mut blake2b = Blake2bBuilder::new(32)
                .personal(b"ckb-default-hash")
                .build();
    blake2b.update(first_input.as_slice());
    blake2b.update(&(fist_output_index as u64).to_le_bytes());
    let mut ret = [0; 32];
    blake2b.finalize(&mut ret);

    if args[..] != ret[0..20] {
        return Err(Error::TypeArgsInvalid);
    }
    let issuer_cell_data = load_cell_data(0, Source::GroupOutput)?;
    let issuer = Issuer::from_data(&issuer_cell_data[..])?;
    if issuer.class_count != 0 {
        return Err(Error::IssuerClassCountError);
    }
    if issuer.set_count != 0 {
        return Err(Error::IssuerSetCountError);
    }
    Ok(())
}

fn handle_update(args: &Bytes) -> Result<(), Error> {
    let class_inputs_count = count_class_cell(args, Source::Input);
    if class_inputs_count > 0 {
        return Err(Error::IssuerClassCountError);
    }
    let issuer_input_data = load_cell_data(0, Source::GroupInput)?;
    let issuer_output_data = load_cell_data(0, Source::GroupOutput)?;
    let input_issuer = Issuer::from_data(&issuer_input_data[..])?;
    let output_issuer = Issuer::from_data(&issuer_output_data[..])?;
    if output_issuer.set_count < input_issuer.set_count {
        return Err(Error::IssuerSetCountError);
    }
    if output_issuer.class_count < input_issuer.class_count {
        return Err(Error::IssuerClassCountError);
    }
    let class_cells_count = (output_issuer.class_count - input_issuer.class_count) as usize;
    let class_cells_increased_count = count_class_cell(args, Source::Output);
    if class_outputs_count != class_cells_increased_count {
        return Err(Error::IssuerClassCountError);
    }
    Ok(())
}

fn handle_destroying() -> Result<(), Error> {
    let issuer_input_data = load_cell_data(0, Source::GroupInput)?;
    let input_issuer = Issuer::from_data(&issuer_input_data[..])?;
    if input_issuer.class_count != 0 || input_issuer.set_count != 0 {
        return Err(Error::IssuerCellCannotDestroyed);
    }
    Ok(())
}

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();
    if args.len() != ISSUER_TYPE_ARGS_LEN {
        return Err(Error::TypeArgsInvalid);
    }

    match parse_issuer_action(&args)? {
        Action::Create => handle_creation(&args),
        Action::Update => handle_update(&args),
        Action::Destroy => handle_destroying(),
    }
}
