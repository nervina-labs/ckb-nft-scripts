use super::*;
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};
use ckb_tool::ckb_error::assert_error_eq;
use ckb_tool::ckb_script::ScriptError;
use ckb_tool::ckb_types::{
    bytes::Bytes,
    core::{TransactionBuilder, TransactionView},
    packed::*,
    prelude::*,
};

const MAX_CYCLES: u64 = 70_000_000;

// error numbers
const TYPE_ARGS_INVALID: i8 = 7;
const NFT_DATA_INVALID: i8 = 19;
// const NFT_CELLS_COUNT_ERROR: i8 = 20;
// const NFT_TOKEN_ID_INCREASE_ERROR: i8 = 21;
// const NFT_AND_CLASS_CONFIGURE_NOT_SAME: i8 = 22;
const NFT_CHARACTERISTIC_NOT_SAME: i8 = 23;
const NFT_CONFIGURE_NOT_SAME: i8 = 24;
const NFT_CLAIMED_TO_UNCLAIMED_ERROR: i8 = 25;
const NFT_LOCKED_TO_UNLOCKED_ERROR: i8 = 26;
const NFT_DISALLOW_CLAIMED: i8 = 27;
const NFT_DISALLOW_LOCKED: i8 = 28;
const NFT_CANNOT_TRANSFER_BEFORE_CLAIM: i8 = 29;
const NFT_CANNOT_TRANSFER_AFTER_CLAIM: i8 = 30;
const NFT_EXT_INFO_LEN_ERROR: i8 = 31;
const NFT_EXT_INFO_CANNOT_MODIFY: i8 = 32;
const NFT_CANNOT_DESTROY_BEFORE_CLAIM: i8 = 33;
const NFT_CANNOT_DESTROY_AFTER_CLAIM: i8 = 34;
const LOCKED_NFT_CANNOT_CLAIM: i8 = 35;
const LOCKED_NFT_CANNOT_TRANSFER: i8 = 36;
const LOCKED_NFT_CANNOT_ADD_EXT_INFO: i8 = 37;
const LOCKED_NFT_CANNOT_DESTROY: i8 = 38;
const LOCKED_NFT_CANNOT_UPDATE_CHARACTERISTIC: i8 = 39;

#[derive(PartialEq, Eq, Clone, Copy)]
enum DestroyCase {
    Default,
    IssuerInput,
    // ClassInput,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum UpdateCase {
    Claim,
    Lock,
    Transfer,
    AddExtInfo,
    UpdateCharacteristic,
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
    NFTAllowAddExtInfoShortError,
    NFTAllowAddExtInfoNotSameError,
    NFTDisallowAddExtInfoLenError,
    NFTCannotDestroyBeforeClaim,
    NFTCannotDestroyAfterClaim,
    LockedNFTCannotClaim,
    LockedNFTCannotTransfer,
    LockedNFTCannotAddExtInfo,
    LockedNFTCannotDestroy,
    LockedNFTCannotUpdateCharacteristic,
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

    let issuer_bin: Bytes = Loader::default().load_binary("issuer-type");
    let issuer_out_point = context.deploy_cell(issuer_bin);
    let issuer_type_script_dep = CellDep::new_builder()
        .out_point(issuer_out_point.clone())
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

    let issuer_type_args = hex::decode("157a3633c3477d84b604a25e5fca5ca681762c10").unwrap();
    let issuer_type_script = context
        .build_script(&issuer_out_point, Bytes::from(issuer_type_args.clone()))
        .expect("script");

    // issuer type script and inputs
    let issuer_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(2000u64.pack())
            .lock(lock_script.clone())
            .type_(Some(issuer_type_script.clone()).pack())
            .build(),
        Bytes::from(hex::decode("0000000000000000000000").unwrap()),
    );
    let issuer_input = CellInput::new_builder()
        .previous_output(issuer_input_out_point.clone())
        .build();

    // class type script and inputs
    let class_input_data = match action {
        // Action::Create => match nft_error {
        //     NftError::NFTCellsCountError => {
        //         Bytes::from(hex::decode("00000000640000000b00000155000266660003898989").unwrap())
        //     }
        //     NftError::NFTAndClassConfigureNotSame => {
        //         Bytes::from(hex::decode("00000000640000000907000155000266660003898989").unwrap())
        //     }
        //     _ => Bytes::from(hex::decode("00000000640000000100000155000266660003898989").unwrap()),
        // },
        Action::Destroy(case) => match case {
            // DestroyCase::ClassInput => {
            //     Bytes::from(hex::decode("000000000f0000000500000155000266660003898989").unwrap())
            // }
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

    let class_type_script = context
        .build_script(
            &class_out_point,
            Bytes::copy_from_slice(&class_type_args[..]),
        )
        .expect("script");

    let class_input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(100000u64.pack())
            .lock(lock_script.clone())
            .type_(Some(class_type_script.clone()).pack())
            .build(),
        class_input_data,
    );
    let class_input = CellInput::new_builder()
        .previous_output(class_input_out_point.clone())
        .build();

    // nft type script and inputs
    let nft_input_data = match action {
        Action::Update(_) => match nft_error {
            NftError::NFTCharacteristicNotSame => {
                Bytes::from(hex::decode("0000000000000000000800").unwrap())
            }
            NftError::NFTClaimedToUnclaimedError => {
                Bytes::from(hex::decode("0000000000000000000001").unwrap())
            }
            NftError::NFTLockedToUnlockedError => {
                Bytes::from(hex::decode("0000000000000000000002").unwrap())
            }
            NftError::NFTDisallowClaimed => {
                Bytes::from(hex::decode("0000000000000000000100").unwrap())
            }
            NftError::NFTDisallowLocked => {
                Bytes::from(hex::decode("0000000000000000000200").unwrap())
            }
            NftError::NFTCannotTransferBeforeClaim => {
                Bytes::from(hex::decode("0000000000000000001000").unwrap())
            }
            NftError::NFTCannotTransferAfterClaim => {
                Bytes::from(hex::decode("0000000000000000002001").unwrap())
            }
            NftError::NFTAllowAddExtInfoShortError => {
                Bytes::from(hex::decode("000000000000000000000000028899").unwrap())
            }
            NftError::NFTAllowAddExtInfoNotSameError => {
                Bytes::from(hex::decode("000000000000000000000000028899").unwrap())
            }
            NftError::NFTDisallowAddExtInfoLenError => {
                Bytes::from(hex::decode("0000000000000000000400").unwrap())
            }
            NftError::LockedNFTCannotClaim => {
                Bytes::from(hex::decode("0000000000000000000002").unwrap())
            }
            NftError::LockedNFTCannotTransfer => {
                Bytes::from(hex::decode("0000000000000000000002").unwrap())
            }
            NftError::LockedNFTCannotAddExtInfo => {
                Bytes::from(hex::decode("0000000000000000000002").unwrap())
            }
            NftError::LockedNFTCannotUpdateCharacteristic => {
                Bytes::from(hex::decode("0000000000000000000002").unwrap())
            }
            _ => Bytes::from(hex::decode("0000000000000000000000").unwrap()),
        },
        Action::Destroy(case) => match case {
            DestroyCase::Default => match nft_error {
                NftError::NFTCannotDestroyBeforeClaim => {
                    Bytes::from(hex::decode("0000000000000000004000").unwrap())
                }
                NftError::NFTCannotDestroyAfterClaim => {
                    Bytes::from(hex::decode("0000000000000000008001").unwrap())
                }
                NftError::LockedNFTCannotDestroy => {
                    Bytes::from(hex::decode("0000000000000000000002").unwrap())
                }
                _ => Bytes::from(hex::decode("0000000000000000000000").unwrap()),
            },
            _ => Bytes::from(hex::decode("000000000000000000c000").unwrap()),
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
        .build_script(&nft_out_point, Bytes::copy_from_slice(&another_nft_type_args[..]))
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
                _ => vec![nft_input]
            },
            _ => vec![nft_input]
        },
        Action::Destroy(case) => match case {
            DestroyCase::Default => vec![nft_input],
            // DestroyCase::ClassInput => vec![nft_input, class_input],
            DestroyCase::IssuerInput => vec![nft_input, issuer_input],
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
            _ => vec![CellOutput::new_builder()
                .capacity(500u64.pack())
                .lock(lock_script.clone())
                .type_(Some(nft_type_script.clone()).pack())
                .build()],
        },
        Action::Destroy(case) => match case {
            DestroyCase::Default => vec![CellOutput::new_builder()
                .capacity(500u64.pack())
                .lock(lock_script.clone())
                .build()],
            // DestroyCase::ClassInput => vec![
            //     CellOutput::new_builder()
            //         .capacity(500u64.pack())
            //         .lock(lock_script.clone())
            //         .build(),
            //     CellOutput::new_builder()
            //         .capacity(500u64.pack())
            //         .lock(lock_script.clone())
            //         .type_(Some(class_type_script.clone()).pack())
            //         .build(),
            // ],
            DestroyCase::IssuerInput => vec![
                CellOutput::new_builder()
                    .capacity(500u64.pack())
                    .lock(lock_script.clone())
                    .build(),
                CellOutput::new_builder()
                    .capacity(500u64.pack())
                    .lock(lock_script.clone())
                    .type_(Some(issuer_type_script.clone()).pack())
                    .build(),
            ],
        },
    };

    match action {
        // Action::Create => {
        //     let token_ids = match nft_error {
        //         NftError::NFTTokenIdIncreaseError => [4u32, 4u32, 4u32, 4u32, 1u32, 6u32, 7u32, 10u32, 8u32, 9u32, 13u32, 11u32, 12u32, 14u32, 15u32],
        //         _ => [5u32, 4u32, 3u32, 2u32, 1u32, 6u32, 7u32, 10u32, 8u32, 9u32, 13u32, 11u32, 12u32, 14u32, 15u32],
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
                        .build())
                }
            },
            _ => ()
        }
        _ => (),
    }

    let outputs_data: Vec<_> = match action {
        // Action::Create => match nft_error {
        //     NftError::NFTAndClassConfigureNotSame => vec![
        //         Bytes::from(hex::decode("00000000640000001007000155000266660003898989").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000000155").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000000155").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000000155").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000000155").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000000155").unwrap()),
        //     ],
        //     _ => vec![
        //         Bytes::from(hex::decode("00000000640000001000000155000266660003898989").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000000155").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000000155").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000000155").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000000155").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000").unwrap()),
        //         Bytes::from(hex::decode("0000000000000000000000000155").unwrap()),
        //     ],
        // },
        Action::Update(case) => match (case, nft_error) {
            (UpdateCase::Claim, NftError::NoError) => {
                vec![Bytes::from(hex::decode("0000000000000000000001").unwrap()), 
                Bytes::from(hex::decode("0000000000000000000001").unwrap())]
            }
            (UpdateCase::Lock, NftError::NoError) => {
                vec![Bytes::from(hex::decode("0000000000000000000002").unwrap())]
            }
            (UpdateCase::AddExtInfo, NftError::NoError) => vec![Bytes::from(
                hex::decode("0000000000000000000002000155").unwrap(),
            )],
            (UpdateCase::UpdateCharacteristic, NftError::NoError) => vec![Bytes::from(
                hex::decode("0022334455667788990000").unwrap(),
            )],
            (UpdateCase::UpdateCharacteristic, NftError::NFTCharacteristicNotSame) => {
                vec![Bytes::from(hex::decode("0022334455667788990800").unwrap())]
            }
            (UpdateCase::Claim, NftError::NFTDataInvalid) => {
                vec![Bytes::from(hex::decode("000000000000000000").unwrap())]
            }
            (UpdateCase::Claim, NftError::NFTConfigureNotSame) => {
                vec![Bytes::from(hex::decode("0000000000000000007800").unwrap())]
            }
            (UpdateCase::Claim, NftError::NFTClaimedToUnclaimedError) => {
                vec![Bytes::from(hex::decode("0000000000000000000000").unwrap())]
            }
            (UpdateCase::Lock, NftError::NFTLockedToUnlockedError) => {
                vec![Bytes::from(hex::decode("0000000000000000000000").unwrap())]
            }
            (UpdateCase::Claim, NftError::NFTDisallowClaimed) => {
                vec![Bytes::from(hex::decode("0000000000000000000101").unwrap())]
            }
            (UpdateCase::Lock, NftError::NFTDisallowLocked) => {
                vec![Bytes::from(hex::decode("0000000000000000000202").unwrap())]
            }
            (UpdateCase::Transfer, NftError::NFTCannotTransferBeforeClaim) => {
                vec![Bytes::from(hex::decode("0000000000000000001000").unwrap())]
            }
            (UpdateCase::Transfer, NftError::NFTCannotTransferAfterClaim) => {
                vec![Bytes::from(hex::decode("0000000000000000002001").unwrap())]
            }
            (UpdateCase::AddExtInfo, NftError::NFTAllowAddExtInfoShortError) => vec![Bytes::from(
                hex::decode("0000000000000000000000000288").unwrap(),
            )],
            (UpdateCase::AddExtInfo, NftError::NFTAllowAddExtInfoNotSameError) => {
                vec![Bytes::from(
                    hex::decode("000000000000000000000000026677").unwrap(),
                )]
            }
            (UpdateCase::AddExtInfo, NftError::NFTDisallowAddExtInfoLenError) => vec![Bytes::from(
                hex::decode("0000000000000000000400023344").unwrap(),
            )],
            (UpdateCase::Claim, NftError::LockedNFTCannotClaim) => {
                vec![Bytes::from(hex::decode("0000000000000000000003").unwrap())]
            }
            (UpdateCase::Transfer, NftError::LockedNFTCannotTransfer) => {
                vec![Bytes::from(hex::decode("0000000000000000000002").unwrap())]
            }
            (UpdateCase::AddExtInfo, NftError::LockedNFTCannotAddExtInfo) => vec![Bytes::from(
                hex::decode("0000000000000000000002000199").unwrap(),
            )],
            (UpdateCase::UpdateCharacteristic, NftError::LockedNFTCannotUpdateCharacteristic) => {
                vec![Bytes::from(hex::decode("0000000000234567890002").unwrap())]
            }
            (_, _) => vec![Bytes::from(hex::decode("0000000000000000000000").unwrap())],
        },
        Action::Destroy(case) => match case {
            DestroyCase::Default => vec![Bytes::new()],
            // DestroyCase::ClassInput => vec![
            //     Bytes::new(),
            //     Bytes::from(hex::decode("000000000f0000000500000155000266660003898989").unwrap()),
            // ],
            DestroyCase::IssuerInput => vec![
                Bytes::new(),
                Bytes::from(hex::decode("0000000000000000000000").unwrap()),
            ],
        },
    };

    let witnesses = inputs
        .iter()
        .map(|_input| Bytes::from("0x"))
        .collect::<Vec<Bytes>>();

    // build transaction
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_dep(lock_script_dep)
        .cell_dep(issuer_type_script_dep)
        .cell_dep(class_type_script_dep)
        .cell_dep(nft_type_script_dep)
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
fn test_add_ext_info_nft_cell_success() {
    let (mut context, tx) =
        create_test_context(Action::Update(UpdateCase::AddExtInfo), NftError::NoError);

    let tx = context.complete_tx(tx);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_update_characteristic_nft_cell_success() {
    let (mut context, tx) =
        create_test_context(Action::Update(UpdateCase::UpdateCharacteristic), NftError::NoError);

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

// #[test]
// fn test_destroy_nft_cell_with_class_input_success() {
//     let (mut context, tx) =
//         create_test_context(Action::Destroy(DestroyCase::ClassInput), NftError::NoError);

//     let tx = context.complete_tx(tx);
//     // run
//     let cycles = context
//         .verify_tx(&tx, MAX_CYCLES)
//         .expect("pass verification");
//     println!("consume cycles: {}", cycles);
// }

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
//     let (mut context, tx) = create_test_context(Action::Create, NftError::NFTTokenIdIncreaseError);

//     let tx = context.complete_tx(tx);
//     // run
//     let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
//     let script_cell_indexes = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

//     let errors = script_cell_indexes
//         .iter()
//         .map(|index| {
//             ScriptError::ValidationFailure(NFT_TOKEN_ID_INCREASE_ERROR).output_type_script(*index)
//         })
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
fn test_update_nft_ext_info_len_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::AddExtInfo),
        NftError::NFTAllowAddExtInfoShortError,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(NFT_EXT_INFO_LEN_ERROR).input_type_script(script_cell_index)
    );
}

#[test]
fn test_update_nft_ext_info_not_same_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::AddExtInfo),
        NftError::NFTAllowAddExtInfoNotSameError,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(NFT_EXT_INFO_CANNOT_MODIFY)
            .input_type_script(script_cell_index)
    );
}

#[test]
fn test_update_nft_cannot_add_ext_info_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::AddExtInfo),
        NftError::NFTDisallowAddExtInfoLenError,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(NFT_EXT_INFO_LEN_ERROR).input_type_script(script_cell_index)
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
fn test_locked_nft_cannot_add_ext_info_error() {
    let (mut context, tx) = create_test_context(
        Action::Update(UpdateCase::AddExtInfo),
        NftError::LockedNFTCannotAddExtInfo,
    );

    let tx = context.complete_tx(tx);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(LOCKED_NFT_CANNOT_ADD_EXT_INFO)
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
