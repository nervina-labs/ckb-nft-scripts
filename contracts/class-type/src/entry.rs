use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    high_level::{load_cell_data, load_input, load_script},
};
use core::result::Result;
use script_utils::{
    class::{Class, CLASS_TYPE_ARGS_LEN},
    error::Error,
    hash::blake2b_160,
    helper::{count_cells_with_type_args, Action},
};

fn parse_class_action(args: &Bytes) -> Result<Action, Error> {
    let check_args_equal = |type_args: &Bytes| type_args[..] == args[..];
    let inputs_count = count_cells_with_type_args(Source::Input, &check_args_equal);
    let outputs_count = count_cells_with_type_args(Source::Output, &check_args_equal);

    match (inputs_count, outputs_count) {
        (0, 0) => Err(Error::ClassCellsCountError),
        (0, outputs_count) => Ok(Action::Create),
        (1, 1) => Ok(Action::Update),
        (1, 0) => Ok(Action::Destroy),
        _ => Err(Error::ClassCellsCountError),
    }
}

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();
    if args.len() != CLASS_TYPE_ARGS_LEN {
        return Err(Error::TypeArgsInvalid);
    }

    match parse_class_action(&args)? {
        Action::Create => Ok(()),
        Action::Update => Ok(()),
        Action::Destroy => Ok(()),
    }
}
