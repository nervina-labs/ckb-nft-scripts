use ckb_std::error::SysError;

/// Error
#[repr(i8)]
pub enum Error {
    IndexOutOfBound = 1,
    ItemMissing,
    LengthNotEnough,
    Encoding,
    IssuerDataInvalid = 5,
    IssuerCellsCountError,
    TypeArgsInvalid,
    IssuerClassCountError,
    IssuerSetCountError,
    IssuerCellCannotDestroyed = 10,
    VersionInvalid,
    ClassDataInvalid,
    ClassTotalSmallerThanIssued,
    ClassCellsCountError,
    ClassIssuedInvalid = 15,
    ClassImmutableFieldsNotSame,
    ClassCellCannotDestroyed,
    ClassIdIncreaseError,
    NFTDataInvalid,
    NFTCellsCountError = 20,
    TokenIdIncreaseError,
    NFTAndClassConfigureNotSame,
    NFTCharacteristicNotSame,
    NFTConfigureNotSame,
}

impl From<SysError> for Error {
    fn from(err: SysError) -> Self {
        use SysError::*;
        match err {
            IndexOutOfBound => Self::IndexOutOfBound,
            ItemMissing => Self::ItemMissing,
            LengthNotEnough(_) => Self::LengthNotEnough,
            Encoding => Self::Encoding,
            Unknown(err_code) => panic!("unexpected sys error {}", err_code),
        }
    }
}
