use alloc::vec::Vec;
use ckb_std::{ckb_constants::Source, ckb_types::prelude::*, high_level::load_cell_lock};
use core::result::Result;
use script_utils::{
    error::Error,
    nft::{Nft, NFT_DATA_MIN_LEN},
};

type Nfts = (Nft, Nft);
type NftDataTuple = (Vec<u8>, Vec<u8>);

pub fn validate_immutable_nft_fields((input_nft, output_nft): &Nfts) -> Result<(), Error> {
    if input_nft.characteristic != output_nft.characteristic {
        return Err(Error::NFTCharacteristicNotSame);
    }
    if input_nft.configure != output_nft.configure {
        return Err(Error::NFTConfigureNotSame);
    }
    Ok(())
}

pub fn validate_nft_claim((input_nft, output_nft): &Nfts) -> Result<(), Error> {
    match (input_nft.is_claimed(), output_nft.is_claimed()) {
        (false, true) => {
            if !input_nft.allow_claim() {
                return Err(Error::NFTCannotClaimed);
            }
            Ok(())
        }
        (true, false) => Err(Error::NFTClaimedToUnclaimedError),
        _ => Ok(()),
    }
}

pub fn validate_nft_lock((input_nft, output_nft): &Nfts) -> Result<(), Error> {
    match (input_nft.is_locked(), output_nft.is_locked()) {
        (false, true) => {
            if !input_nft.allow_lock() {
                return Err(Error::NFTCannotLocked);
            }
            Ok(())
        }
        (true, false) => Err(Error::NFTLockedToUnlockedError),
        _ => Ok(()),
    }
}

pub fn validate_nft_transfer(input_nft: &Nft) -> Result<(), Error> {
    let input_lock = load_cell_lock(0, Source::GroupInput)?;
    let output_lock = load_cell_lock(0, Source::GroupOutput)?;
    if input_lock.as_slice() != output_lock.as_slice() {
        if !input_nft.is_claimed() && !input_nft.allow_transfer_before_claim() {
            return Err(Error::NFTCannotTransferBeforeClaim);
        }
        if input_nft.is_claimed() && !input_nft.allow_transfer_after_claim() {
            return Err(Error::NFTCannotTransferAfterClaim);
        }
        return Ok(());
    }
    Ok(())
}

pub fn validate_nft_ext_info(
    input_nft: &Nft,
    (input_nft_data, output_nft_data): &NftDataTuple,
) -> Result<(), Error> {
    if input_nft.allow_ext_info() {
        let input_len = input_nft_data.len();
        if input_nft_data[NFT_DATA_MIN_LEN..input_len]
            != output_nft_data[NFT_DATA_MIN_LEN..input_len]
        {
            return Err(Error::NFTExtInfoLenError);
        }
        Ok(())
    } else {
        if input_nft_data.len() != output_nft_data.len() {
            return Err(Error::NFTExtInfoLenError);
        }
        Ok(())
    }
}
