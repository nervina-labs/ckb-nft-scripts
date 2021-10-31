use super::*;
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};
use ckb_testtool::ckb_error::assert_error_eq;
use ckb_testtool::ckb_script::ScriptError;
use ckb_testtool::ckb_types::{
    bytes::Bytes,
    core::{TransactionBuilder, TransactionView},
    packed::*,
    prelude::*,
};

const MAX_CYCLES: u64 = 70_000_000;

const TYPE: u8 = 1;
const CLASS_TYPE_CODE_HASH: [u8; 32] = [
    9, 91, 140, 11, 78, 81, 164, 95, 149, 58, 205, 31, 205, 30, 57, 72, 159, 38, 117, 180, 188,
    148, 231, 175, 39, 187, 56, 149, 135, 144, 227, 252,
];

// error numbers
const TYPE_ARGS_INVALID: i8 = 7;
const NFT_DATA_INVALID: i8 = 19;
const NFT_CHARACTERISTIC_NOT_SAME: i8 = 23;
const NFT_CONFIGURE_NOT_SAME: i8 = 24;
const NFT_CLAIMED_TO_UNCLAIMED_ERROR: i8 = 25;
const NFT_LOCKED_TO_UNLOCKED_ERROR: i8 = 26;
const NFT_DISALLOW_CLAIMED: i8 = 27;
const NFT_DISALLOW_LOCKED: i8 = 28;
const NFT_CANNOT_TRANSFER_BEFORE_CLAIM: i8 = 29;
const NFT_CANNOT_TRANSFER_AFTER_CLAIM: i8 = 30;
const NFT_CANNOT_DESTROY_BEFORE_CLAIM: i8 = 31;
const NFT_CANNOT_DESTROY_AFTER_CLAIM: i8 = 32;
const LOCKED_NFT_CANNOT_CLAIM: i8 = 33;
const LOCKED_NFT_CANNOT_TRANSFER: i8 = 34;
const LOCKED_NFT_CANNOT_DESTROY: i8 = 35;
const LOCKED_NFT_CANNOT_UPDATE_CHARACTERISTIC: i8 = 36;
const GROUP_INPUT_WITNESS_NONE_ERROR: i8 = 37;

#[derive(PartialEq, Eq, Clone, Copy)]
enum DestroyCase {
    Default,
    IssuerInput,
    ClassInput,
    Batch,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum UpdateCase {
    Claim,
    Lock,
    Transfer,
    UpdateCharacteristic,
    UpdateStateWithIssuer,
    UpdateStateWithClass,
}

#[derive(PartialEq, Eq)]
enum Action {
    // Create,
    Update(UpdateCase),
    Destroy(DestroyCase),
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum NftError {
    NoError,
    TypeArgsInvalid,
    NFTDataInvalid,
    // NFTCellsCountError,
    // NFTTokenIdIncreaseError,
    // NFTAndClassConfigureNotSame,
    NFTCharacteristicNotSame,
    NFTConfigureNotSame,
    NFTClaimedToUnclaimedError,
    NFTLockedToUnlockedError,
    NFTDisallowClaimed,
    NFTDisallowLocked,
    NFTCannotTransferBeforeClaim,
    NFTCannotTransferAfterClaim,
    NFTCannotDestroyBeforeClaim,
    NFTCannotDestroyAfterClaim,
    LockedNFTCannotClaim,
    LockedNFTCannotTransfer,
    LockedNFTCannotDestroy,
    LockedNFTCannotUpdateCharacteristic,
    UpdateStateWithoutIssuer,
    UpdateStateWithOtherIssuer,
    UpdateStateWithoutClass,
    UpdateStateWithOtherClass,
    GroupInputWitnessNoneError,
}

fn create_test_context(action: Action, nft_error: NftError) -> (Context, TransactionView) {
    // deploy contract
    let mut context = Context::default();

    let nft_bin: Bytes = Loader::default().load_binary("nft-type");
    let nft_out_point = context.deploy_cell(nft_bin);
    let nft_type_script_dep = CellDep::new_builder()
        .out_point(nft_out_point.clone())
        .build();

    let class_bin: Bytes = Loader::default().load_binary("class-type");
    let class_out_point = context.deploy_cell(class_bin);
    let class_type_script_dep = CellDep::new_builder()
        .out_point(class_out_point.clone())
        .build();

    // deploy always_success script
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());

    // prepare scripts
    let lock_script = context
        .build_script(&always_success_out_point, Default::default())
        .expect("script");
    let another_lock_script = context
        .build_script(
            &always_success_out_point,
            Bytes::from(hex::decode("12").unwrap()),
        )
        .expect("script");
    let lock_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point)
        .build();

    let issuer_bin: Bytes = Loader::default().load_binary("issuer-type");
    let issuer_out_point = context.deploy_cell(issuer_bin);

    let issuer_type_args = hex::decode("157a3633c3477d84b604a25e5fca5ca681762c10").unwrap();
    let issuer_type_script = context
        .build_script(&issuer_out_point, Bytes::from(issuer_type_args.clone()))
        .expect("script");

    // issuer type script and inputs
    let issuer_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(2000u64.pack())
            .lock(lock_script.clone())
            .build(),
        Bytes::new(),
    );
    let issuer_input = CellInput::new_builder()
        .previous_output(issuer_input_out_point.clone())
        .build();

    let issuer_cell_dep_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(2000u64.pack())
            .lock(lock_script.clone())
            .type_(Some(issuer_type_script.clone()).pack())
            .build(),
        Bytes::from(hex::decode("0100000000000000000000").unwrap()),
    );
    let issuer_cell_dep = CellDep::new_builder()
        .out_point(issuer_cell_dep_out_point.clone())
        .build();

    let another_issuer_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(2000u64.pack())
            .lock(another_lock_script.clone())
            .build(),
        Bytes::new(),
    );
    let another_issuer_input = CellInput::new_builder()
        .previous_output(another_issuer_input_out_point.clone())
        .build();

    // class type script and inputs
    let class_input_data = match action {
        // Action::Create => match nft_error {
        //     NftError::NFTCellsCountError => {
        //         Bytes::from(hex::decode("01000000640000000b00000155000266660003898989").unwrap())
        //     }
        //     NftError::NFTAndClassConfigureNotSame => {
        //         Bytes::from(hex::decode("01000000640000000907000155000266660003898989").unwrap())
        //     }
        //     _ => Bytes::from(hex::decode("01000000640000000100000155000266660003898989").
        // unwrap()), },
        Action::Destroy(case) => match case {
            DestroyCase::ClassInput => Bytes::from(hex::decode("0100000000000000000000").unwrap()),
            _ => Bytes::new(),
        },
        Action::Update(_) => Bytes::new(),
    };

    let issuer_type_hash: [u8; 32] = issuer_type_script.clone().calc_script_hash().unpack();
    let mut class_type_args = issuer_type_hash[0..20].to_vec();
    let mut args_class_id = 8u32.to_be_bytes().to_vec();
    class_type_args.append(&mut args_class_id);

    let mut another_class_type_args = issuer_type_hash[0..20].to_vec();
    let mut another_args_class_id = 9u32.to_be_bytes().to_vec();
    another_class_type_args.append(&mut another_args_class_id);

    // let class_type_script = context
    //     .build_script(
    //         &class_out_point,
    //         Bytes::copy_from_slice(&class_type_args[..]),
    //     )
    //     .expect("script");

    // let class_input_out_point = context.create_cell(
    //     CellOutput::new_builder()
    //         .capacity(100000u64.pack())
    //         .lock(lock_script.clone())
    //         .type_(Some(class_type_script.clone()).pack())
    //         .build(),
    //     class_input_data.clone(),
    // );
    // let class_input = CellInput::new_builder()
    //     .previous_output(class_input_out_point.clone())
    //     .build();

    // let class_cell_dep = CellDep::new_builder()
    //     .out_point(class_input_out_point.clone())
    //     .build();

    let class_input_out_point_without_type = context.create_cell(
        CellOutput::new_builder()
            .capacity(100000u64.pack())
            .lock(lock_script.clone())
            .build(),
        Bytes::new(),
    );
    let class_input_without_type = CellInput::new_builder()
        .previous_output(class_input_out_point_without_type.clone())
        .build();

    let another_class_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(1000u64.pack())
            .lock(another_lock_script.clone())
            .build(),
        class_input_data,
    );
    let another_class_input = CellInput::new_builder()
        .previous_output(another_class_input_out_point.clone())
        .build();

    let class_aggron_type_script = Script::new_builder()
        .code_hash(CLASS_TYPE_CODE_HASH.pack())
        .args(Bytes::copy_from_slice(&class_type_args[..]).pack())
        .hash_type(Byte::new(TYPE))
        .build();
    let class_cell_dep_aggron_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(2000u64.pack())
            .lock(lock_script.clone())
            .type_(Some(class_aggron_type_script.clone()).pack())
            .build(),
        Bytes::from("0x"),
    );
    let class_cell_aggron_dep = CellDep::new_builder()
        .out_point(class_cell_dep_aggron_out_point.clone())
        .build();

    // nft type script and inputs
    let nft_input_data = match action {
        Action::Update(case) => match case {
            UpdateCase::UpdateStateWithIssuer | UpdateCase::UpdateStateWithClass => {
                Bytes::from(hex::decode("0100000000000000000303").unwrap())
            }
            _ => match nft_error {
                NftError::NFTCharacteristicNotSame => {
                    Bytes::from(hex::decode("0100000000000000000800").unwrap())
                }
                NftError::NFTClaimedToUnclaimedError => {
                    Bytes::from(hex::decode("0100000000000000000001").unwrap())
                }
                NftError::NFTLockedToUnlockedError => {
                    Bytes::from(hex::decode("0100000000000000000002").unwrap())
                }
                NftError::NFTDisallowClaimed => {
                    Bytes::from(hex::decode("0100000000000000000100").unwrap())
                }
                NftError::NFTDisallowLocked => {
                    Bytes::from(hex::decode("0100000000000000000200").unwrap())
                }
                NftError::NFTCannotTransferBeforeClaim => {
                    Bytes::from(hex::decode("0100000000000000001000").unwrap())
                }
                NftError::NFTCannotTransferAfterClaim => {
                    Bytes::from(hex::decode("0100000000000000002001").unwrap())
                }
                NftError::LockedNFTCannotClaim => {
                    Bytes::from(hex::decode("0100000000000000000002").unwrap())
                }
                NftError::LockedNFTCannotTransfer => {
                    Bytes::from(hex::decode("0100000000000000000002").unwrap())
                }
                NftError::LockedNFTCannotUpdateCharacteristic => {
                    Bytes::from(hex::decode("0100000000000000000002").unwrap())
                }
                _ => Bytes::from(hex::decode("0100000000000000000000").unwrap()),
            },
        },
        Action::Destroy(case) => match case {
            DestroyCase::Default => match nft_error {
                NftError::NFTCannotDestroyBeforeClaim => {
                    Bytes::from(hex::decode("0100000000000000004000").unwrap())
                }
                NftError::NFTCannotDestroyAfterClaim => {
                    Bytes::from(hex::decode("0100000000000000008001").unwrap())
                }
                NftError::LockedNFTCannotDestroy => {
                    Bytes::from(hex::decode("0100000000000000000002").unwrap())
                }
                _ => Bytes::from(hex::decode("0100000000000000000000").unwrap()),
            },
            DestroyCase::Batch => Bytes::from(hex::decode("0100000000000000000000").unwrap()),
            _ => Bytes::from(hex::decode("010000000000000000c000").unwrap()),
        },
        // Action::Create => Bytes::new(),
    };

    let mut nft_type_args = class_type_args.clone().to_vec();
    let mut args_token_id = match nft_error {
        NftError::TypeArgsInvalid => 11u16.to_be_bytes().to_vec(),
        _ => 11u32.to_be_bytes().to_vec(),
    };
    nft_type_args.append(&mut args_token_id);

    let mut another_nft_type_args = another_class_type_args.clone().to_vec();
    let mut another_args_token_id = 12u32.to_be_bytes().to_vec();
    another_nft_type_args.append(&mut another_args_token_id);

    let nft_type_script = context
        .build_script(&nft_out_point, Bytes::copy_from_slice(&nft_type_args[..]))
        .expect("script");

    let another_nft_type_script = context
        .build_script(
            &nft_out_point,
            Bytes::copy_from_slice(&another_nft_type_args[..]),
        )
        .expect("script");

    let nft_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .type_(Some(nft_type_script.clone()).pack())
            .build(),
        nft_input_data.clone(),
    );
    let nft_input = CellInput::new_builder()
        .previous_output(nft_input_out_point.clone())
        .build();

    let another_nft_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .type_(Some(another_nft_type_script.clone()).pack())
            .build(),
        nft_input_data,
    );
    let another_nft_input = CellInput::new_builder()
        .previous_output(another_nft_input_out_point.clone())
        .build();

    let inputs = match action {
        // Action::Create => vec![class_input],
        Action::Update(case) => match case {
            UpdateCase::Claim => match nft_error {
                NftError::NoError => vec![nft_input, another_nft_input],
                _ => vec![nft_input],
            },
            UpdateCase::UpdateStateWithIssuer => match nft_error {
                NftError::UpdateStateWithOtherIssuer => vec![another_issuer_input, nft_input],
                _ => vec![issuer_input, nft_input],
            },
            UpdateCase::UpdateStateWithClass => match nft_error {
                NftError::UpdateStateWithOtherClass => vec![another_class_input, nft_input],
                _ => vec![class_input_without_type, nft_input],
            },
            _ => vec![nft_input],
        },
        Action::Destroy(case) => match case {
            DestroyCase::Default => vec![nft_input],
            DestroyCase::Batch => vec![nft_input.clone(), nft_input],
            DestroyCase::ClassInput => vec![class_input_without_type, nft_input],
            DestroyCase::IssuerInput => vec![issuer_input, nft_input],
        },
    };

    let mut outputs = match action {
        // Action::Create => vec![CellOutput::new_builder()
        //     .capacity(500u64.pack())
        //     .lock(lock_script.clone())
        //     .type_(Some(class_type_script.clone()).pack())
        //     .build()],
        Action::Update(case) => match case {
            UpdateCase::Transfer => vec![CellOutput::new_builder()
                .capacity(500u64.pack())
                .lock(another_lock_script.clone())
                .type_(Some(nft_type_script.clone()).pack())
                .build()],
            UpdateCase::UpdateStateWithIssuer | UpdateCase::UpdateStateWithClass => vec![
                CellOutput::new_builder()
                    .capacity(500u64.pack())
                    .lock(lock_script.clone())
                    .build(),
                CellOutput::new_builder()
                    .capacity(500u64.pack())
                    .lock(lock_script.clone())
                    .type_(Some(nft_type_script.clone()).pack())
                    .build(),
            ],
            _ => vec![CellOutput::new_builder()
                .capacity(500u64.pack())
                .lock(lock_script.clone())
                .type_(Some(nft_type_script.clone()).pack())
                .build()],
        },
        Action::Destroy(case) => match case {
            DestroyCase::Default | DestroyCase::Batch => vec![CellOutput::new_builder()
                .capacity(500u64.pack())
                .lock(lock_script.clone())
                .build()],
            DestroyCase::IssuerInput | DestroyCase::ClassInput => vec![
                CellOutput::new_builder()
                    .capacity(500u64.pack())
                    .lock(lock_script.clone())
                    .build(),
                CellOutput::new_builder()
                    .capacity(500u64.pack())
                    .lock(lock_script.clone())
                    .build(),
            ],
        },
    };

    match action {
        // Action::Create => {
        //     let token_ids = match nft_error {
        //         NftError::NFTTokenIdIncreaseError => [4u32, 3u32, 2u32, 5u32, 1u32, 6u32, 7u32,
        // 10u32, 8u32, 9u32, 13u32, 11u32, 12u32, 14u32, 15u32], _ => [1u32, 2u32, 3u32,
        // 4u32, 5u32, 6u32, 7u32, 8u32, 9u32, 10u32, 11u32, 12u32, 13u32, 14u32, 15u32],
        //     };
        //     for token_id in token_ids.iter() {
        //         let mut nft_type_args = class_type_args.clone().to_vec();
        //         let mut args_token_id = token_id.to_be_bytes().to_vec();
        //         nft_type_args.append(&mut args_token_id);

        //         let nft_type_script = context
        //             .build_script(&nft_out_point, Bytes::copy_from_slice(&nft_type_args[..]))
        //             .expect("script");

        //         outputs.push(
        //             CellOutput::new_builder()
        //                 .capacity(500u64.pack())
        //                 .lock(lock_script.clone())
        //                 .type_(Some(nft_type_script.clone()).pack())
        //                 .build(),
        //         );
        //     }
        // },
        Action::Update(case) => match case {
            UpdateCase::Claim => {
                if nft_error == NftError::NoError {
                    outputs.push(
                        CellOutput::new_builder()
                            .capacity(500u64.pack())
                            .lock(lock_script.clone())
                            .type_(Some(another_nft_type_script.clone()).pack())
                            .build(),
                    )
                }
            }
            _ => (),
        },
        _ => (),
    }

    let outputs_data: Vec<_> = match action {
        // Action::Create => match nft_error {
        //     NftError::NFTAndClassConfigureNotSame => vec![
        //         Bytes::from(hex::decode("01000000640000001007000155000266660003898989").
        // unwrap()),         Bytes::from(hex::decode("0100000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000000155").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000000155").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000000155").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000000155").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000000155").unwrap()),
        //     ],
        //     _ => vec![
        //         Bytes::from(hex::decode("01000000640000001000000155000266660003898989").
        // unwrap()),         Bytes::from(hex::decode("0100000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000000155").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000000155").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000000155").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000000155").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0100000000000000000000000155").unwrap()),
        //     ],
        // },
        Action::Update(case) => match (case, nft_error) {
            (UpdateCase::Claim, NftError::NoError) => vec![
                Bytes::from(hex::decode("0100000000000000000001").unwrap()),
                Bytes::from(hex::decode("0100000000000000000001").unwrap()),
            ],
            (UpdateCase::Lock, NftError::NoError) => {
                vec![Bytes::from(hex::decode("0100000000000000000002").unwrap())]
            }
            (UpdateCase::UpdateCharacteristic, NftError::NoError) => {
                vec![Bytes::from(hex::decode("0122334455667788990000").unwrap())]
            }
            (UpdateCase::UpdateStateWithIssuer, _) => vec![
                Bytes::new(),
                Bytes::from(hex::decode("0100000000000000000300").unwrap()),
            ],
            (UpdateCase::UpdateStateWithClass, _) => vec![
                Bytes::new(),
                Bytes::from(hex::decode("0100000000000000000300").unwrap()),
            ],
            (UpdateCase::UpdateCharacteristic, NftError::NFTCharacteristicNotSame) => {
                vec![Bytes::from(hex::decode("0122334455667788990800").unwrap())]
            }
            (UpdateCase::Claim, NftError::NFTDataInvalid) => {
                vec![Bytes::from(hex::decode("010000000000000000").unwrap())]
            }
            (UpdateCase::Claim, NftError::NFTConfigureNotSame) => {
                vec![Bytes::from(hex::decode("0100000000000000007800").unwrap())]
            }
            (UpdateCase::Claim, NftError::NFTClaimedToUnclaimedError) => {
                vec![Bytes::from(hex::decode("0100000000000000000000").unwrap())]
            }
            (UpdateCase::Lock, NftError::NFTLockedToUnlockedError) => {
                vec![Bytes::from(hex::decode("0100000000000000000000").unwrap())]
            }
            (UpdateCase::Claim, NftError::NFTDisallowClaimed) => {
                vec![Bytes::from(hex::decode("0100000000000000000101").unwrap())]
            }
            (UpdateCase::Lock, NftError::NFTDisallowLocked) => {
                vec![Bytes::from(hex::decode("0100000000000000000202").unwrap())]
            }
            (UpdateCase::Transfer, NftError::NFTCannotTransferBeforeClaim) => {
                vec![Bytes::from(hex::decode("0100000000000000001000").unwrap())]
            }
            (UpdateCase::Transfer, NftError::NFTCannotTransferAfterClaim) => {
                vec![Bytes::from(hex::decode("0100000000000000002001").unwrap())]
            }
            (UpdateCase::Claim, NftError::LockedNFTCannotClaim) => {
                vec![Bytes::from(hex::decode("0100000000000000000003").unwrap())]
            }
            (UpdateCase::Transfer, NftError::LockedNFTCannotTransfer) => {
                vec![Bytes::from(hex::decode("0100000000000000000002").unwrap())]
            }
            (UpdateCase::UpdateCharacteristic, NftError::LockedNFTCannotUpdateCharacteristic) => {
                vec![Bytes::from(hex::decode("0100000000234567890002").unwrap())]
            }
            (_, _) => vec![Bytes::from(hex::decode("0100000000000000000000").unwrap())],
        },
        Action::Destroy(case) => match case {
            DestroyCase::Default | DestroyCase::Batch => vec![Bytes::new()],
            DestroyCase::ClassInput => vec![
                Bytes::new(),
                Bytes::from(hex::decode("0100000000000000000000").unwrap()),
            ],
            DestroyCase::IssuerInput => vec![
                Bytes::new(),
                Bytes::from(hex::decode("0100000000000000000000").unwrap()),
            ],
        },
    };

    let mut witnesses = vec![];
    match nft_error {
        NftError::GroupInputWitnessNoneError => {
            witnesses.push(Bytes::from("12345678"))
        }
        _ => {
            witnesses.push(Bytes::from(hex::decode("5500000010000000550000005500000041000000b69c542c0ee6c4b6d8350514d876ea7d8ef563e406253e959289457204447d2c4eb4e4a993073f5e76d244d2f93f7c108652e3295a9c8d72c12477e095026b9500").unwrap()))
        }
    }
    match nft_error {
        NftError::UpdateStateWithOtherIssuer | NftError::UpdateStateWithOtherClass => {
            witnesses.push(Bytes::from(hex::decode("5500000010000000550000005500000041000000b69c542c0ee6c4b6d8350514d876ea7d8ef563e406253e959289457204447d2c4eb4e4a993073f5e76d244d2f93f7c108652e3295a9c8d72c12477e095026b9500").unwrap()))
        }
        _ => {
            witnesses.push(Bytes::from("0x"))
        }
    }
    if inputs.len() > 2 {
        for _ in 2..inputs.len() {
            witnesses.push(Bytes::from("0x"))
        }
    }

    let cell_deps = match action {
        Action::Destroy(case) => match case {
            DestroyCase::IssuerInput => vec![issuer_cell_dep, lock_script_dep, nft_type_script_dep],
            DestroyCase::ClassInput => {
                vec![class_cell_aggron_dep, lock_script_dep, nft_type_script_dep]
            }
            _ => vec![lock_script_dep, class_type_script_dep, nft_type_script_dep],
        },
        Action::Update(case) => match case {
            UpdateCase::UpdateStateWithIssuer => match nft_error {
                NftError::UpdateStateWithoutIssuer => vec![lock_script_dep, nft_type_script_dep],
                _ => vec![issuer_cell_dep, lock_script_dep, nft_type_script_dep],
            },
            UpdateCase::UpdateStateWithClass => match nft_error {
                NftError::UpdateStateWithoutClass => vec![lock_script_dep, nft_type_script_dep],
                _ => vec![class_cell_aggron_dep, lock_script_dep, nft_type_script_dep],
            },
            _ => vec![lock_script_dep, nft_type_script_dep],
        }, // _ => vec![lock_script_dep, class_type_script_dep, nft_type_script_dep],
    };

    // build transaction
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_deps(cell_deps)
        .witnesses(witnesses.pack())
        .build();
    (context, tx)
}

// #[test]
// fn test_create_nft_cells_success() {
//     let (mut context, tx) = create_test_context(Action::Create, NftError::NoError);

//     let tx = context.complete_tx(tx);
//     // run
//     let cycles = context
//         .verify_tx(&tx, MAX_CYCLES)
//         .expect("pass verification");
//     println!("consume cycles: {}", cycles);
// }

#[test]
fn test_claim_nft_cell_success() {
    let (mut context, tx) =
        create_test_context(Action::Update(UpdateCase::Claim), NftError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_lock_nft_cell_success() {
    let (mut context, tx) =
        create_test_context(Action::Update(UpdateCase::Lock), NftError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_transfer_nft_cell_success() {
    let (mut context, tx) =
        create_test_context(Action::Update(UpdateCase::Transfer), NftError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_update_characteristic_nft_cell_success() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::UpdateCharacteristic),
        NftError::NoError,
    );

    let tx = context.complete_tx(tx);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_update_nft_state_with_issuer_success() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::UpdateStateWithIssuer),
        NftError::NoError,
    );

    let tx = context.complete_tx(tx);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_update_nft_state_with_class_success() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::UpdateStateWithClass),
        NftError::NoError,
    );

    let tx = context.complete_tx(tx);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_destroy_nft_cell_with_default_success() {
    let (mut context, tx) =
        create_test_context(Action::Destroy(DestroyCase::Default), NftError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_batch_destroy_nft_cell_with_default_success() {
    let (mut context, tx) =
        create_test_context(Action::Destroy(DestroyCase::Batch), NftError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_destroy_nft_cell_with_issuer_input_success() {
    let (mut context, tx) =
        create_test_context(Action::Destroy(DestroyCase::IssuerInput), NftError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_destroy_nft_cell_with_class_input_success() {
    let (mut context, tx) =
        create_test_context(Action::Destroy(DestroyCase::ClassInput), NftError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_update_nft_cell_data_len_error() {
    let (mut context, tx) =
        create_test_context(Action::Update(UpdateCase::Claim), NftError::NFTDataInvalid);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(NFT_DATA_INVALID).input_type_script(script_cell_index)
    );
}

#[test]
fn test_update_nft_with_group_input_witness_none_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::Claim),
        NftError::GroupInputWitnessNoneError,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(GROUP_INPUT_WITNESS_NONE_ERROR)
            .input_type_script(script_cell_index)
    );
}

// #[test]
// fn test_create_nft_cells_count_error() {
//     let (mut context, tx) = create_test_context(Action::Create, NftError::NFTCellsCountError);

//     let tx = context.complete_tx(tx);
//     // run
//     let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
//     let script_cell_indexes = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

//     let errors = script_cell_indexes
//         .iter()
//         .map(|index| {
//             ScriptError::ValidationFailure(NFT_CELLS_COUNT_ERROR).output_type_script(*index)
//         })
//         .collect::<Vec<_>>();

//     assert_errors_contain!(err, errors);
// }

// #[test]
// fn test_create_nft_cells_token_id_increase_error() {
//     let (mut context, tx) = create_test_context(Action::Create,
// NftError::NFTTokenIdIncreaseError);

//     let tx = context.complete_tx(tx);
//     // run
//     let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
//     let script_cell_indexes = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

//     let errors = script_cell_indexes
//         .iter()
//         .map(|index| {
//
// ScriptError::ValidationFailure(NFT_TOKEN_ID_INCREASE_ERROR).output_type_script(*index)         })
//         .collect::<Vec<_>>();

//     assert_errors_contain!(err, errors);
// }

// #[test]
// fn test_create_nft_and_class_configure_not_same_error() {
//     let (mut context, tx) =
//         create_test_context(Action::Create, NftError::NFTAndClassConfigureNotSame);

//     let tx = context.complete_tx(tx);
//     // run
//     let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
//     let script_cell_indexes = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

//     let errors = script_cell_indexes
//         .iter()
//         .map(|index| {
//             ScriptError::ValidationFailure(NFT_AND_CLASS_CONFIGURE_NOT_SAME)
//                 .output_type_script(*index)
//         })
//         .collect::<Vec<_>>();

//     assert_errors_contain!(err, errors);
// }

#[test]
fn test_update_nft_characteristic_not_same_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::UpdateCharacteristic),
        NftError::NFTCharacteristicNotSame,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(NFT_CHARACTERISTIC_NOT_SAME)
            .input_type_script(script_cell_index)
    );
}

#[test]
fn test_update_nft_configure_not_same_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::Claim),
        NftError::NFTConfigureNotSame,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(NFT_CONFIGURE_NOT_SAME).input_type_script(script_cell_index)
    );
}

#[test]
fn test_update_nft_claimed_to_unclaimed_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::Claim),
        NftError::NFTClaimedToUnclaimedError,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(NFT_CLAIMED_TO_UNCLAIMED_ERROR)
            .input_type_script(script_cell_index)
    );
}

#[test]
fn test_update_nft_claimed_to_unclaimed_caused_by_issuer_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::UpdateStateWithIssuer),
        NftError::UpdateStateWithoutIssuer,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 1;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(NFT_CLAIMED_TO_UNCLAIMED_ERROR)
            .input_type_script(script_cell_index)
    );
}

#[test]
fn test_update_nft_claimed_to_unclaimed_caused_by_issuer_lock_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::UpdateStateWithIssuer),
        NftError::UpdateStateWithOtherIssuer,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 1;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(NFT_CLAIMED_TO_UNCLAIMED_ERROR)
            .input_type_script(script_cell_index)
    );
}

#[test]
fn test_update_nft_claimed_to_unclaimed_caused_by_class_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::UpdateStateWithClass),
        NftError::UpdateStateWithoutClass,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 1;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(NFT_CLAIMED_TO_UNCLAIMED_ERROR)
            .input_type_script(script_cell_index)
    );
}

#[test]
fn test_update_nft_claimed_to_unclaimed_caused_by_class_lock_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::UpdateStateWithClass),
        NftError::UpdateStateWithOtherClass,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 1;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(NFT_CLAIMED_TO_UNCLAIMED_ERROR)
            .input_type_script(script_cell_index)
    );
}

#[test]
fn test_update_nft_locked_to_unlocked_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::Lock),
        NftError::NFTLockedToUnlockedError,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(NFT_LOCKED_TO_UNLOCKED_ERROR)
            .input_type_script(script_cell_index)
    );
}

#[test]
fn test_update_nft_disallow_to_be_claimed_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::Claim),
        NftError::NFTDisallowClaimed,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(NFT_DISALLOW_CLAIMED).input_type_script(script_cell_index)
    );
}

#[test]
fn test_update_nft_disallow_to_be_locked_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::Lock),
        NftError::NFTDisallowLocked,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(NFT_DISALLOW_LOCKED).input_type_script(script_cell_index)
    );
}

#[test]
fn test_update_nft_cannot_transfer_before_claim_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::Transfer),
        NftError::NFTCannotTransferBeforeClaim,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(NFT_CANNOT_TRANSFER_BEFORE_CLAIM)
            .input_type_script(script_cell_index)
    );
}

#[test]
fn test_update_nft_cannot_transfer_after_claim_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::Transfer),
        NftError::NFTCannotTransferAfterClaim,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(NFT_CANNOT_TRANSFER_AFTER_CLAIM)
            .input_type_script(script_cell_index)
    );
}

#[test]
fn test_cannot_destroy_nft_before_claim_error() {
    let (mut context, tx) = create_test_context(
        Action::Destroy(DestroyCase::Default),
        NftError::NFTCannotDestroyBeforeClaim,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(NFT_CANNOT_DESTROY_BEFORE_CLAIM)
            .input_type_script(script_cell_index)
    );
}

#[test]
fn test_cannot_destroy_nft_after_claim_error() {
    let (mut context, tx) = create_test_context(
        Action::Destroy(DestroyCase::Default),
        NftError::NFTCannotDestroyAfterClaim,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(NFT_CANNOT_DESTROY_AFTER_CLAIM)
            .input_type_script(script_cell_index)
    );
}

#[test]
fn test_locked_nft_cannot_claim_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::Claim),
        NftError::LockedNFTCannotClaim,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(LOCKED_NFT_CANNOT_CLAIM)
            .input_type_script(script_cell_index)
    );
}

#[test]
fn test_locked_nft_cannot_transfer_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::Transfer),
        NftError::LockedNFTCannotTransfer,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(LOCKED_NFT_CANNOT_TRANSFER)
            .input_type_script(script_cell_index)
    );
}

#[test]
fn test_locked_nft_cannot_update_characteristic_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::UpdateCharacteristic),
        NftError::LockedNFTCannotUpdateCharacteristic,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(LOCKED_NFT_CANNOT_UPDATE_CHARACTERISTIC)
            .input_type_script(script_cell_index)
    );
}

#[test]
fn test_locked_nft_cannot_destroy_error() {
    let (mut context, tx) = create_test_context(
        Action::Destroy(DestroyCase::Default),
        NftError::LockedNFTCannotDestroy,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(LOCKED_NFT_CANNOT_DESTROY)
            .input_type_script(script_cell_index)
    );
}

#[test]
fn test_destroy_nft_with_group_input_witness_none_error() {
    let (mut context, tx) = create_test_context(
        Action::Destroy(DestroyCase::Default),
        NftError::GroupInputWitnessNoneError,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(GROUP_INPUT_WITNESS_NONE_ERROR)
            .input_type_script(script_cell_index)
    );
}

#[test]
fn test_nft_type_args_invalid_error() {
    let (mut context, tx) =
        create_test_context(Action::Update(UpdateCase::Claim), NftError::TypeArgsInvalid);

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(TYPE_ARGS_INVALID).input_type_script(script_cell_index)
    );
}
