use crate::validator::{
    validate_immutable_nft_fields, validate_nft_claim, validate_nft_ext_info, validate_nft_lock,
    validate_nft_transfer,
};
use alloc::vec::Vec;
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
        count_cells_by_type_args, count_cells_by_type_hash, load_cell_data_by_type_args,
        load_output_type_args_ids, Action,
    },
    issuer::ISSUER_TYPE_ARGS_LEN,
    nft::{Nft, NFT_TYPE_ARGS_LEN},
};

fn check_issuer_id<'a>(nft_args: &'a Bytes) -> impl Fn(&[u8]) -> bool + 'a {
    move |type_hash: &[u8]| type_hash[..] == nft_args[0..ISSUER_TYPE_ARGS_LEN]
}

fn check_class_args<'a>(nft_args: &'a Bytes) -> impl Fn(&Bytes) -> bool + 'a {
    move |type_args: &Bytes| type_args[..] == nft_args[0..CLASS_TYPE_ARGS_LEN]
}

fn check_nft_args<'a>(nft_args: &'a Bytes) -> impl Fn(&Bytes) -> bool + 'a {
    move |type_args: &Bytes| {
        type_args.len() == NFT_TYPE_ARGS_LEN
            && type_args[0..CLASS_TYPE_ARGS_LEN] == nft_args[0..CLASS_TYPE_ARGS_LEN]
    }
}

fn load_nft_data(source: Source) -> Result<Vec<u8>, Error> {
    load_cell_data(0, source).map_err(|_| Error::NFTDataInvalid)
}

fn parse_nft_action(nft_args: &Bytes) -> Result<Action, Error> {
    let nft_inputs_count = count_cells_by_type_args(Source::Input, &check_nft_args(nft_args));
    if nft_inputs_count == 0 {
        return Ok(Action::Create);
    }

    let nft_outputs_count = count_cells_by_type_args(Source::Output, &check_nft_args(nft_args));
    if nft_outputs_count == 0 {
        return Ok(Action::Destroy);
    }

    if nft_inputs_count == 1 && nft_outputs_count == 1 {
        return Ok(Action::Update);
    }
    Err(Error::NFTCellsCountError)
}

fn handle_creation(nft_args: &Bytes) -> Result<(), Error> {
    let class_inputs_count = count_cells_by_type_args(Source::Input, &check_class_args(nft_args));
    if class_inputs_count != 1 {
        return Err(Error::ClassCellsCountError);
    }

    let load_class = |source| match load_cell_data_by_type_args(source, &check_class_args(nft_args))
    {
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

    let mut outputs_token_ids =
        load_output_type_args_ids(CLASS_TYPE_ARGS_LEN, &check_nft_args(nft_args));
    let nft_outputs_increased_count = (output_class.issued - input_class.issued) as usize;
    if nft_outputs_increased_count != outputs_token_ids.len() {
        return Err(Error::NFTCellsCountError);
    }
    outputs_token_ids.sort();

    let mut class_cell_token_ids = Vec::new();
    for token_id in input_class.issued..output_class.issued {
        class_cell_token_ids.push(token_id);
    }

    if outputs_token_ids != class_cell_token_ids {
        return Err(Error::NFTTokenIdIncreaseError);
    }

    Ok(())
}

fn handle_update() -> Result<(), Error> {
    let nft_data = (
        load_nft_data(Source::GroupInput)?,
        load_nft_data(Source::GroupOutput)?,
    );
    let nfts = (
        Nft::from_data(&nft_data.0[..])?,
        Nft::from_data(&nft_data.1[..])?,
    );
    validate_immutable_nft_fields(&nfts)?;
    validate_nft_claim(&nfts)?;
    validate_nft_lock(&nfts)?;
    validate_nft_transfer(&nfts.0)?;
    validate_nft_ext_info(&nfts.0, &nft_data)?;
    Ok(())
}

fn handle_destroying(nft_args: &Bytes) -> Result<(), Error> {
    let issuer_inputs_count = count_cells_by_type_hash(Source::Input, &check_issuer_id(nft_args));
    let class_inputs_count = count_cells_by_type_args(Source::Input, &check_class_args(nft_args));
    if issuer_inputs_count > 0 || class_inputs_count > 0 {
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
    let script = load_script()?;
    let nft_args: Bytes = script.args().unpack();
    if nft_args.len() != NFT_TYPE_ARGS_LEN {
        return Err(Error::TypeArgsInvalid);
    }

    match parse_nft_action(&nft_args)? {
        Action::Create => handle_creation(&nft_args),
        Action::Update => handle_update(),
        Action::Destroy => handle_destroying(&nft_args),
    }
}
