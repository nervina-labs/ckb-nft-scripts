use ckb_std::{ckb_constants::Source, ckb_types::prelude::*, high_level::load_cell_lock};
use core::result::Result;
use script_utils::{
    error::Error,
    nft::{Nft},
};

type Nfts = (Nft, Nft);

pub fn validate_immutable_nft_fields((input_nft, output_nft): &Nfts) -> Result<(), Error> {
    if input_nft.characteristic != output_nft.characteristic {
        if !input_nft.allow_update_characteristic() {
            return Err(Error::NFTCharacteristicNotSame);
        }
        if input_nft.is_locked() {
            return Err(Error::LockedNFTCannotUpdateCharacteristic);
        }
    }
    if input_nft.configure != output_nft.configure {
        return Err(Error::NFTConfigureNotSame);
    }
    Ok(())
}

pub fn validate_nft_claim((input_nft, output_nft): &Nfts) -> Result<(), Error> {
    match (input_nft.is_claimed(), output_nft.is_claimed()) {
        (false, true) => {
            if input_nft.is_locked() {
                return Err(Error::LockedNFTCannotClaim);
            }
            if !input_nft.allow_claim() {
                return Err(Error::NFTDisallowClaimed);
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
                return Err(Error::NFTDisallowLocked);
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
        if input_nft.is_locked() {
            return Err(Error::LockedNFTCannotTransfer);
        }
        if !input_nft.is_claimed() && !input_nft.allow_transfer_before_claim() {
            return Err(Error::NFTCannotTransferBeforeClaim);
        }
        if input_nft.is_claimed() && !input_nft.allow_transfer_after_claim() {
            return Err(Error::NFTCannotTransferAfterClaim);
        }
    }
    Ok(())
}
