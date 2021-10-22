use crate::validator::{
    validate_immutable_nft_fields, validate_nft_claim, validate_nft_lock, validate_nft_transfer
};
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
        cell_deps_and_inputs_have_issuer_or_class_lock,
        check_group_input_witness_is_none_with_type, count_cells_by_type, load_cell_data_by_type,
        load_class_type, load_output_type_args_ids, Action,
    },
    nft::{Nft, NFT_TYPE_ARGS_LEN},
};

fn check_class_type<'a>(nft_args: &'a Bytes) -> impl Fn(&Script) -> bool + 'a {
    let class_type = load_class_type(nft_args);
    move |type_: &Script| type_.as_slice() == class_type.as_slice()
}

fn check_nft_type<'a>(nft_type: &'a Script) -> impl Fn(&Script) -> bool + 'a {
    let nft_args: Bytes = nft_type.args().unpack();
    move |type_: &Script| {
        let type_args: Bytes = type_.args().unpack();
        type_.code_hash().as_slice() == nft_type.code_hash().as_slice()
            && type_.hash_type().as_slice() == nft_type.hash_type().as_slice()
            && type_args.len() == NFT_TYPE_ARGS_LEN
            && type_args[0..CLASS_TYPE_ARGS_LEN] == nft_args[0..CLASS_TYPE_ARGS_LEN]
    }
}

fn load_nft_data(source: Source) -> Result<Vec<u8>, Error> {
    load_cell_data(0, source).map_err(|_| Error::NFTDataInvalid)
}

fn issuer_or_class_lock_has_approved(nft_args: &Bytes) -> Result<bool, Error> {
    if !cell_deps_and_inputs_have_issuer_or_class_lock(&nft_args)? {
        return Ok(false);
    }
    Ok(true)
}

fn parse_nft_action(nft_type: &Script) -> Result<Action, Error> {
    let nft_inputs_count = count_cells_by_type(Source::Input, &check_nft_type(nft_type));
    if nft_inputs_count == 0 {
        return Ok(Action::Create);
    }

    let nft_outputs_count = count_cells_by_type(Source::Output, &check_nft_type(nft_type));
    if nft_outputs_count == 0 {
        return Ok(Action::Destroy);
    }

    if nft_inputs_count == nft_outputs_count {
        return Ok(Action::Update);
    }
    Err(Error::NFTCellsCountError)
}

fn handle_creation(nft_type: &Script) -> Result<(), Error> {
    let nft_args: Bytes = nft_type.args().unpack();
    let class_inputs_count = count_cells_by_type(Source::Input, &check_class_type(&nft_args));
    if class_inputs_count != 1 {
        return Err(Error::ClassCellsCountError);
    }

    let load_class = |source| match load_cell_data_by_type(source, &check_class_type(&nft_args)) {
        Some(data) => Ok(Class::from_data(&data)?),
        None => Err(Error::ClassDataInvalid),
    };
    let input_class = load_class(Source::Input)?;
    let output_class = load_class(Source::Output)?;

    if output_class.issued <= input_class.issued {
        return Err(Error::ClassIssuedInvalid);
    }

    let nft = Nft::from_data(&load_nft_data(Source::GroupOutput)?[..])?;
    if nft.configure != input_class.configure {
        return Err(Error::NFTAndClassConfigureNotSame);
    }

    let outputs_token_ids =
        load_output_type_args_ids(CLASS_TYPE_ARGS_LEN, &check_nft_type(nft_type));
    let nft_outputs_increased_count = (output_class.issued - input_class.issued) as usize;
    if nft_outputs_increased_count != outputs_token_ids.len() {
        return Err(Error::NFTCellsCountError);
    }

    let mut class_cell_token_ids = Vec::new();
    for token_id in input_class.issued..output_class.issued {
        class_cell_token_ids.push(token_id);
    }

    if outputs_token_ids != class_cell_token_ids {
        return Err(Error::NFTTokenIdIncreaseError);
    }

    Ok(())
}

fn handle_update(nft_type: &Script) -> Result<(), Error> {
    // Disable anyone-can-pay lock
    if check_group_input_witness_is_none_with_type(nft_type)? {
        return Err(Error::GroupInputWitnessNoneError);
    }
    let nft_data = (
        load_nft_data(Source::GroupInput)?,
        load_nft_data(Source::GroupOutput)?,
    );
    let nfts = (
        Nft::from_data(&nft_data.0[..])?,
        Nft::from_data(&nft_data.1[..])?,
    );
    validate_immutable_nft_fields(&nfts)?;

    let nft_args: Bytes = nft_type.args().unpack();
    if !issuer_or_class_lock_has_approved(&nft_args)? {
        validate_nft_claim(&nfts)?;
        validate_nft_lock(&nfts)?;
    }
    validate_nft_transfer(&nfts.0)?;
    Ok(())
}

fn handle_destroying(nft_type: &Script) -> Result<(), Error> {
    // Disable anyone-can-pay lock
    if check_group_input_witness_is_none_with_type(nft_type)? {
        return Err(Error::GroupInputWitnessNoneError);
    }
    let nft_args: Bytes = nft_type.args().unpack();
    if issuer_or_class_lock_has_approved(&nft_args)? {
        return Ok(());
    }
    let input_nft = Nft::from_data(&load_nft_data(Source::GroupInput)?[..])?;
    if input_nft.is_locked() {
        return Err(Error::LockedNFTCannotDestroy);
    }
    if !input_nft.is_claimed() && !input_nft.allow_destroying_before_claim() {
        return Err(Error::NFTCannotDestroyBeforeClaim);
    }
    if input_nft.is_claimed() && !input_nft.allow_destroying_after_claim() {
        return Err(Error::NFTCannotDestroyAfterClaim);
    }
    Ok(())
}

pub fn main() -> Result<(), Error> {
    let nft_type = load_script()?;
    let nft_args: Bytes = nft_type.args().unpack();
    if nft_args.len() != NFT_TYPE_ARGS_LEN {
        return Err(Error::TypeArgsInvalid);
    }

    match parse_nft_action(&nft_type)? {
        Action::Create => handle_creation(&nft_type),
        Action::Update => handle_update(&nft_type),
        Action::Destroy => handle_destroying(&nft_type),
    }
}
