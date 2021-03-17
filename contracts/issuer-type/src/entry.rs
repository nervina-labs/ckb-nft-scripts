use core::result::Result;

use ckb_std::{
    ckb_types::{bytes::Bytes, prelude::*},
    ckb_constants::Source,
    high_level::{load_script, load_input_out_point, load_cell_data},
};

use script_utils::{issuer::Issuer, error::Error, helper::count_cells_with_type_args, hash::blake2b_160};

enum Action {
    Create,
    Update,
    Destroy,
}

fn parse_issuer_action(args: &Bytes) -> Result<Action, Error> {
    let inputs_count = count_cells_with_type_args(args, Source::Input);
    let outputs_count = count_cells_with_type_args(args, Source::Output);

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

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();

    let action = parse_issuer_action(&args)?;

    match action {
        Action::Create => {
            let out_point = load_input_out_point(0, Source::Input)?;
            if args[..] != blake2b_160(out_point.as_slice()) {
                return Err(Error::TypeArgsInvalid);
            }
            let issuer_cell_data = load_cell_data(0, Source::GroupOutput)?;
            let issuer = Issuer::from_data(&issuer_cell_data[..])?;
            if issuer.class_count != 0 || issuer.set_count != 0 {
                return Err(Error::IssuerClassCountOrSetCountError);
            }
        },
        Action::Update => {
            return Ok(());
        },
        Action::Destroy => {
            return Ok(());
        }
    }

    Ok(())
}
