use alloc::vec::Vec;
use blake2b_rs::Blake2bBuilder;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, packed::*, prelude::*},
    high_level::{load_cell_data, load_input, load_script},
};
use core::result::Result;
use script_utils::{
    error::Error,
    helper::{
        check_group_input_witness_is_none_with_type, count_cells_by_type,
        load_output_index_by_type, Action,
    },
    issuer::{Issuer, ISSUER_TYPE_ARGS_LEN},
};

fn load_issuer_data(source: Source) -> Result<Vec<u8>, Error> {
    load_cell_data(0, source).map_err(|_| Error::IssuerDataInvalid)
}

fn parse_issuer_action(issuer_type: &Script) -> Result<Action, Error> {
    let count_cells = |source| {
        count_cells_by_type(source, &|type_: &Script| {
            type_.as_slice() == issuer_type.as_slice()
        })
    };
    let issuer_cells_count = (count_cells(Source::Input), count_cells(Source::Output));
    match issuer_cells_count {
        (0, 1) => Ok(Action::Create),
        (1, 1) => Ok(Action::Update),
        (1, 0) => Ok(Action::Destroy),
        _ => Err(Error::IssuerCellsCountError),
    }
}

fn handle_creation(issuer_type: &Script) -> Result<(), Error> {
    let first_input = load_input(0, Source::Input)?;
    let first_output_index = load_output_index_by_type(issuer_type).ok_or(Error::Encoding)?;
    let mut blake2b = Blake2bBuilder::new(32)
        .personal(b"ckb-default-hash")
        .build();
    blake2b.update(first_input.as_slice());
    blake2b.update(&(first_output_index as u64).to_le_bytes());
    let mut ret = [0; 32];
    blake2b.finalize(&mut ret);

    let issuer_args: Bytes = issuer_type.args().unpack();
    if issuer_args[..] != ret[0..ISSUER_TYPE_ARGS_LEN] {
        return Err(Error::TypeArgsInvalid);
    }
    let issuer = Issuer::from_data(&load_issuer_data(Source::GroupOutput)?[..])?;
    if issuer.class_count != 0 {
        return Err(Error::IssuerClassCountError);
    }
    if issuer.set_count != 0 {
        return Err(Error::IssuerSetCountError);
    }
    Ok(())
}

fn handle_update(issuer_type: &Script) -> Result<(), Error> {
    // Disable anyone-can-pay lock
    if check_group_input_witness_is_none_with_type(issuer_type)? {
        return Err(Error::GroupInputWitnessNoneError);
    }
    let load_issuer = |source| Issuer::from_data(&load_issuer_data(source)?[..]);
    let input_issuer = load_issuer(Source::GroupInput)?;
    let output_issuer = load_issuer(Source::GroupOutput)?;
    if output_issuer.set_count < input_issuer.set_count {
        return Err(Error::IssuerSetCountError);
    }
    if output_issuer.class_count < input_issuer.class_count {
        return Err(Error::IssuerClassCountError);
    }
    Ok(())
}

fn handle_destroying(issuer_type: &Script) -> Result<(), Error> {
    // Disable anyone-can-pay lock
    if check_group_input_witness_is_none_with_type(issuer_type)? {
        return Err(Error::GroupInputWitnessNoneError);
    }
    let input_issuer = Issuer::from_data(&load_issuer_data(Source::GroupInput)?[..])?;
    if input_issuer.class_count != 0 || input_issuer.set_count != 0 {
        return Err(Error::IssuerCellCannotDestroyed);
    }
    Ok(())
}

pub fn main() -> Result<(), Error> {
    let issuer_type = load_script()?;
    let issuer_args: Bytes = issuer_type.args().unpack();
    if issuer_args.len() != ISSUER_TYPE_ARGS_LEN {
        return Err(Error::TypeArgsInvalid);
    }

    match parse_issuer_action(&issuer_type)? {
        Action::Create => handle_creation(&issuer_type),
        Action::Update => handle_update(&issuer_type),
        Action::Destroy => handle_destroying(&issuer_type),
    }
}
