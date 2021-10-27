use alloc::vec::Vec;
use ckb_std::{
    ckb_types::{bytes::Bytes, packed::*, prelude::*},
    high_level::load_script,
};
use core::result::Result;
use script_utils::error::Error;

const TYPE_ARGS_LEN: usize = 20;

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let type_args: Bytes = script.args().unpack();
    if type_args.len() != TYPE_ARGS_LEN {
        return Err(Error::TypeArgsInvalid);
    }
    Ok(())
}
