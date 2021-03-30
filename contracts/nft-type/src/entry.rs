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
        count_cells_by_type_args, load_cell_data_by_type_args, load_output_type_args_ids, Action,
    },
    nft::{Nft, NFT_TYPE_ARGS_LEN},
};

fn check_class_args<'a>(nft_args: &'a Bytes) -> impl Fn(&Bytes) -> bool + 'a {
    move |type_args: &Bytes| type_args[..] == nft_args[0..CLASS_TYPE_ARGS_LEN]
}

fn check_nft_args<'a>(nft_args: &'a Bytes) -> impl Fn(&Bytes) -> bool + 'a {
    move |type_args: &Bytes| {
        type_args.len() == NFT_TYPE_ARGS_LEN
            && type_args[0..CLASS_TYPE_ARGS_LEN] == nft_args[0..CLASS_TYPE_ARGS_LEN]
    }
}

fn parse_nft_action(nft_args: &Bytes) -> Result<Action, Error> {
    let count_cells = |source| count_cells_by_type_args(source, &check_nft_args(nft_args));
    let nft_cells_count = (count_cells(Source::Input), count_cells(Source::Output));
    match nft_cells_count {
        (0, _) => Ok(Action::Create),
        (1, 1) => Ok(Action::Update),
        (1, 0) => Ok(Action::Destroy),
        _ => Err(Error::NFTCellsCountError),
    }
}

fn handle_creation(nft_args: &Bytes) -> Result<(), Error> {
    let count_cells = |source| count_cells_by_type_args(source, &check_class_args(nft_args));
    let class_cells_count = (count_cells(Source::Input), count_cells(Source::Output));
    if class_cells_count != (1, 1) {
        return Err(Error::NFTCellsCountError);
    }

    let load_class =
        |source| match load_cell_data_by_type_args(source, &check_class_args(nft_args)) {
            Some(data) => Ok(Class::from_data(&data)?),
            None => Err(Error::IssuerDataInvalid),
        };
    let input_class = load_class(Source::Input)?;
    let output_class = load_class(Source::Output)?;

    if output_class.issued <= input_class.issued {
        return Err(Error::ClassIssuedInvalid);
    }

    let nft = Nft::from_data(&load_cell_data(0, Source::GroupOutput)?)?;
    if nft.configure != input_class.configure {
        return Err(Error::NFTAndClassConfigureNotSame);
    }

    let mut outputs_token_ids =
        load_output_type_args_ids(CLASS_TYPE_ARGS_LEN, &check_class_args(nft_args));
    let nft_outputs_increased_count =
        (output_class.issued - input_class.issued) as usize;
    if nft_outputs_increased_count != outputs_token_ids.len() {
        return Err(Error::NFTCellsCountError);
    }
    outputs_token_ids.sort();

    let mut class_cell_token_ids = Vec::new();
    for token_id in input_class.issued..output_class.issued {
        class_cell_token_ids.push(token_id);
    }

    if &outputs_token_ids != &class_cell_token_ids {
        return Err(Error::TokenIdIncreaseError);
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
        Action::Update => Ok(()),
        Action::Destroy => Ok(()),
    }
}
