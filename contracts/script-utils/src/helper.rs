use alloc::vec::Vec;

use ckb_std::{
    ckb_types::{bytes::Bytes, prelude::*},
    ckb_constants::Source,
    high_level::{load_cell_type, QueryIter},
};

pub fn count_cells_with_type_args(args: &Bytes, source: Source) -> usize {
  let type_scripts = QueryIter::new(load_cell_type, source).filter(|type_opt| {
    match type_opt {
      Some(type_) => {
        let type_args: Bytes = type_.args().unpack();
        type_args[..] == args[..]
      },
      None => false
    }
  }).collect::<Vec<_>>();
  
  type_scripts.len()
}