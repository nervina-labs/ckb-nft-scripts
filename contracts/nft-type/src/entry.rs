use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    high_level::load_script,
};
use core::result::Result;
use script_utils::{
    error::Error,
    helper::{count_cells_by_type_args, Action},
    nft::{NFT_TYPE_ARGS_LEN},
    class::{CLASS_TYPE_ARGS_LEN},
};

fn check_nft_args<'a>(nft_args: &'a Bytes) -> impl Fn(&Bytes) -> bool + 'a {
    move |type_args: &Bytes| {
        type_args.len() == NFT_TYPE_ARGS_LEN
            && type_args[0..CLASS_TYPE_ARGS_LEN] == nft_args[0..CLASS_TYPE_ARGS_LEN]
    }
}

fn parse_nft_action(nft_args: &Bytes) -> Result<Action, Error> {
    let inputs_count = count_cells_by_type_args(Source::Input, &check_nft_args(nft_args));
    let outputs_count = count_cells_by_type_args(Source::Output, &check_nft_args(nft_args));

    match (inputs_count, outputs_count) {
        (0, _) => Ok(Action::Create),
        (1, 1) => Ok(Action::Update),
        (1, 0) => Ok(Action::Destroy),
        _ => Err(Error::NFTCellsCountError),
    }
}

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let nft_args: Bytes = script.args().unpack();
    if nft_args.len() != NFT_TYPE_ARGS_LEN {
        return Err(Error::TypeArgsInvalid);
    }

    match parse_nft_action(&nft_args)? {
        Action::Create => Ok(()),
        Action::Update => Ok(()),
        Action::Destroy => Ok(()),
    }
}
