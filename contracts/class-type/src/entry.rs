use alloc::vec::Vec;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, packed::*, prelude::*},
    high_level::{load_cell_data, load_script},
};
use core::result::Result;
use script_utils::{
    class::{Class, CLASS_TYPE_ARGS_LEN},
    error::Error,
    helper::{
        count_cells_by_type, count_cells_by_type_hash, load_cell_data_by_type_hash,
        load_output_type_args_ids, Action,
    },
    issuer::{Issuer, ISSUER_TYPE_ARGS_LEN},
};

fn check_issuer_id<'a>(class_args: &'a Bytes) -> impl Fn(&[u8]) -> bool + 'a {
    move |type_hash: &[u8]| {
        type_hash[0..ISSUER_TYPE_ARGS_LEN] == class_args[0..ISSUER_TYPE_ARGS_LEN]
    }
}

fn check_class_type<'a>(class_type: &'a Script) -> impl Fn(&Script) -> bool + 'a {
    let class_args: Bytes = class_type.args().unpack();
    move |type_: &Script| {
        let type_args: Bytes = type_.args().unpack();
        type_.code_hash().as_slice() == class_type.code_hash().as_slice() && 
        type_.hash_type().as_slice() == class_type.hash_type().as_slice() &&
        type_args.len() == CLASS_TYPE_ARGS_LEN && 
        type_args[0..ISSUER_TYPE_ARGS_LEN] == class_args[0..ISSUER_TYPE_ARGS_LEN]
    }
}

fn load_class_data(source: Source) -> Result<Vec<u8>, Error> {
    load_cell_data(0, source).map_err(|_| Error::ClassDataInvalid)
}

fn parse_class_action(class_type: &Script) -> Result<Action, Error> {
    let class_inputs_count = count_cells_by_type(Source::Input, &check_class_type(class_type));
    if class_inputs_count == 0 {
        return Ok(Action::Create);
    }
    let class_outputs_count = count_cells_by_type(Source::Output, &check_class_type(class_type));
    if class_inputs_count == 1 && class_outputs_count == 0 {
        return Ok(Action::Destroy);
    }
    if class_inputs_count == 1 && class_outputs_count == 1 {
        return Ok(Action::Update);
    }
    Err(Error::ClassCellsCountError)
}

fn handle_creation(class_type: &Script) -> Result<(), Error> {
    let class = Class::from_data(&load_class_data(Source::GroupOutput)?)?;
    if class.issued != 0 {
        return Err(Error::ClassIssuedInvalid);
    }

    let class_args: Bytes = class_type.args().unpack();
    let issuer_inputs_count = count_cells_by_type_hash(Source::Input, &check_issuer_id(&class_args));
    if issuer_inputs_count != 1 {
        return Err(Error::IssuerCellsCountError);
    }

    let load_issuer =
        |source| match load_cell_data_by_type_hash(source, &check_issuer_id(&class_args)) {
            Some(data) => Ok(Issuer::from_data(&data)?),
            None => Err(Error::IssuerDataInvalid),
        };
    let input_issuer = load_issuer(Source::Input)?;
    let output_issuer = load_issuer(Source::Output)?;

    if output_issuer.class_count <= input_issuer.class_count {
        return Err(Error::IssuerClassCountError);
    }

    let mut outputs_class_ids =
        load_output_type_args_ids(ISSUER_TYPE_ARGS_LEN, &check_class_type(&class_type));
    let class_outputs_increased_count =
        (output_issuer.class_count - input_issuer.class_count) as usize;
    if class_outputs_increased_count != outputs_class_ids.len() {
        return Err(Error::ClassCellsCountError);
    }
    outputs_class_ids.sort();

    let mut issuer_cell_class_ids = Vec::new();
    for class_id in input_issuer.class_count..output_issuer.class_count {
        issuer_cell_class_ids.push(class_id);
    }

    if outputs_class_ids != issuer_cell_class_ids {
        return Err(Error::ClassIdIncreaseError);
    }
    Ok(())
}

fn handle_update() -> Result<(), Error> {
    let load_class = |source| Class::from_data(&load_class_data(source)?[..]);

    let input_class = load_class(Source::GroupInput)?;
    let output_class = load_class(Source::GroupOutput)?;

    if output_class.issued < input_class.issued {
        return Err(Error::ClassIssuedInvalid);
    }

    if !input_class.immutable_equal(&output_class) {
        return Err(Error::ClassImmutableFieldsNotSame);
    }
    Ok(())
}

fn handle_destroying() -> Result<(), Error> {
    let input_class = Class::from_data(&load_class_data(Source::GroupInput)?[..])?;
    if input_class.issued > 0 {
        return Err(Error::ClassCellCannotDestroyed);
    }
    Ok(())
}

pub fn main() -> Result<(), Error> {
    let class_type = load_script()?;
    let class_args: Bytes = class_type.args().unpack();
    if class_args.len() != CLASS_TYPE_ARGS_LEN {
        return Err(Error::TypeArgsInvalid);
    }

    match parse_class_action(&class_type)? {
        Action::Create => handle_creation(&class_type),
        Action::Update => handle_update(),
        Action::Destroy => handle_destroying(),
    }
}
