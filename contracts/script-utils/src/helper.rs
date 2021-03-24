use alloc::vec::Vec;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, packed::*, prelude::*},
    high_level::{load_cell_data, load_cell_type, QueryIter},
};

const ID_LEN: usize = 4;

pub enum Action {
    Create,
    Update,
    Destroy,
}

fn load_type_args(type_: &Script) -> Bytes {
    let type_args: Bytes = type_.args().unpack();
    type_args
}

fn parse_type_args_id(type_: Script, slice_start: usize) -> Option<u32> {
    let id_slice = &load_type_args(&type_)[slice_start..];
    if id_slice.len() != ID_LEN {
        return None;
    }
    let mut ids = [0u8; ID_LEN];
    ids.copy_from_slice(&id_slice[..]);
    Some(u32::from_be_bytes(ids))
}

fn parse_type_opt(type_opt: &Option<Script>, predicate: &dyn Fn(&Bytes) -> bool) -> bool {
    match type_opt {
        Some(type_) => predicate(&load_type_args(&type_)),
        None => false,
    }
}

pub fn count_cells_by_type_args(source: Source, predicate: &dyn Fn(&Bytes) -> bool) -> usize {
    QueryIter::new(load_cell_type, source)
        .filter(|type_opt| parse_type_opt(type_opt, predicate))
        .count()
}

pub fn load_output_index_by_type_args(args: &Bytes) -> Option<usize> {
    QueryIter::new(load_cell_type, Source::Output)
        .position(|type_opt| match type_opt {
            Some(type_) => load_type_args(&type_)[..] == args[..],
            None => false,
        })
    }

pub fn load_cell_data_by_type_args(
    source: Source,
    predicate: &dyn Fn(&Bytes) -> bool,
) -> Option<Vec<u8>> {
    QueryIter::new(load_cell_type, source)
        .position(|type_opt| type_opt.map_or(false, |type_| predicate(&load_type_args(&type_))))
        .map(|index| load_cell_data(index, source).map_or_else(|_| Vec::new(), |data| data))
}

pub fn load_output_type_args_ids(
    slice_start: usize,
    predicate: &dyn Fn(&Bytes) -> bool,
) -> Vec<u32> {
    QueryIter::new(load_cell_type, Source::Output)
        .filter(|type_opt| parse_type_opt(type_opt, predicate))
        .filter_map(|type_opt| match type_opt {
            Some(type_) => parse_type_args_id(type_, slice_start),
            None => None,
        })
        .collect()
}
