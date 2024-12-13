// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
#![allow(unused)]

use crate::publishing::publish_util::Package;
use aptos_framework::natives::code::{MoveOption, PackageMetadata};
use aptos_sdk::{
    bcs,
    move_types::{
        account_address::AccountAddress, ident_str, identifier::Identifier,
        language_storage::ModuleId,
    },
    types::{
        serde_helper::bcs_utils::bcs_size_of_byte_array,
        transaction::{EntryFunction, Script, TransactionPayload},
    },
};
use move_binary_format::{
    file_format::{FunctionHandleIndex, IdentifierIndex, SignatureToken},
    CompiledModule,
};
use rand::{distributions::Alphanumeric, prelude::StdRng, seq::SliceRandom, Rng};
use rand_core::RngCore;
use serde::{Deserialize, Serialize};

//
// Contains all the code to work on the Simple package
//

//
// Functions to load and update the original package
//

pub fn version(module: &mut CompiledModule, rng: &mut StdRng) {
    // change `const COUNTER_STEP` in Simple.move
    // That is the only u64 in the constant pool
    for constant in &mut module.constant_pool {
        if constant.type_ == SignatureToken::U64 {
            let mut v: u64 = bcs::from_bytes(&constant.data).expect("U64 must deserialize");
            v += 1;
            constant.data = bcs::to_bytes(&v).expect("U64 must serialize");
            break;
        }
    }
}

pub fn scramble(module: &mut CompiledModule, fn_count: usize, rng: &mut StdRng) {
    // change `const RANDOM` in Simple.move
    // That is the only vector<u64> in the constant pool
    let const_len = rng.gen_range(0usize, 5000usize);
    let mut v = Vec::<u64>::with_capacity(const_len);
    for i in 0..const_len {
        v.push(i as u64);
    }
    // module.constant_pool
    for constant in &mut module.constant_pool {
        if constant.type_ == SignatureToken::Vector(Box::new(SignatureToken::U64)) {
            constant.data = bcs::to_bytes(&v).expect("U64 vector must serialize");
            break;
        }
    }

    // find the copy_pasta* function in Simple.move
    let mut def = None;
    let mut handle = None;
    let mut func_name = String::new();
    for func_def in &module.function_defs {
        let func_handle = &module.function_handles[func_def.function.0 as usize];
        let name = module.identifiers[func_handle.name.0 as usize].as_str();
        if name.starts_with("copy_pasta") {
            def = Some(func_def.clone());
            handle = Some(func_handle.clone());
            func_name = String::from(name);
            break;
        }
    }
    if let Some(fd) = def {
        for suffix in 0..fn_count {
            let mut func_handle = handle.clone().expect("Handle must be defined");
            let mut func_def = fd.clone();
            let mut name = func_name.clone();
            name.push_str(suffix.to_string().as_str());
            module
                .identifiers
                .push(Identifier::new(name.as_str()).expect("Identifier name must be valid"));
            func_handle.name = IdentifierIndex((module.identifiers.len() - 1) as u16);
            module.function_handles.push(func_handle);
            func_def.function = FunctionHandleIndex((module.function_handles.len() - 1) as u16);
            module.function_defs.push(func_def);
        }
    }
}

pub enum MultiSigConfig {
    None,
    Random(usize),
    Publisher,
    FeePayerPublisher,
}

#[derive(Debug, Serialize, Deserialize)]
struct BCSStream {
    data: Vec<u8>,
    cursor: u64,
}

#[derive(Debug, Copy, Clone)]
pub enum LoopType {
    NoOp,
    Arithmetic,
    BcsToBytes { len: u64 },
}

/// Automatic arguments function expects (i.e. signer, or multiple signers, etc)
/// That execution can add before the call.
#[derive(Debug, Copy, Clone)]
pub enum AutomaticArgs {
    None,
    Signer,
    SignerAndMultiSig,
}

//
// List of entry points to expose
//
// More info in the Simple.move
#[derive(Debug, Copy, Clone)]
pub enum EntryPoints {
    /// Republish the module
    Republish,
    /// Empty (NoOp) function
    Nop,
    /// Empty (NoOp) function, signed by publisher as fee-payer
    NopFeePayer,
    /// Empty (NoOp) function, signed by 2 accounts
    Nop2Signers,
    /// Empty (NoOp) function, signed by 5 accounts
    Nop5Signers,
    /// Increment signer resource - COUNTER_STEP
    Step,
    /// Fetch signer resource - COUNTER_STEP
    GetCounter,
    /// Reset resource `Resource`
    ResetData,
    /// Double the size of `Resource`
    Double,
    /// Half the size of `Resource`
    Half,
    /// Return value from constant array (RANDOM)
    GetFromConst {
        const_idx: Option<u64>,
    },
    /// Set the `Resource.id`
    SetId,
    /// Set the `Resource.name`
    SetName,
    /// run a for loop
    Loop {
        loop_count: Option<u64>,
        loop_type: LoopType,
    },
    // next 2 functions, second arg must be existing account address with data
    // Sets `Resource` to the max from two addresses
    Maximize,
    // Sets `Resource` to the min from two addresses
    Minimize,
    // 3 args
    /// Explicitly change Resource
    MakeOrChange {
        string_length: Option<usize>,
        data_length: Option<usize>,
    },
    BytesMakeOrChange {
        data_length: Option<usize>,
    },
    EmitEvents {
        count: u64,
    },
    MakeOrChangeTable {
        offset: u64,
        count: u64,
    },
    MakeOrChangeTableRandom {
        max_offset: u64,
        max_count: u64,
    },
    /// Increment global (publisher) resource
    IncGlobal,
    /// Increment global (publisher) AggregatorV2 resource
    IncGlobalAggV2,
    /// Modify (try_add(step) or try_sub(step)) AggregatorV2 bounded counter (counter with max_value=100)
    ModifyGlobalBoundedAggV2 {
        step: u64,
    },
    /// Increment global (publisher) AggregatorV2 resource with Milestone (and conflict) every `milestone_every` increments.
    IncGlobalMilestoneAggV2 {
        milestone_every: u64,
    },
    CreateGlobalMilestoneAggV2 {
        milestone_every: u64,
    },

    /// Modifying a single random tag in a resource group (which contains 8 tags),
    /// from a global resource (at module publishers' address)
    ResourceGroupsGlobalWriteTag {
        string_length: usize,
    },
    /// Modifying a single random tag, and reading another random tag,
    /// in a resource group (which contains 8 tags),
    /// from a global resource (at module publishers' address)
    ResourceGroupsGlobalWriteAndReadTag {
        string_length: usize,
    },
    /// Modifying a single random tag in a resource group (which contains 8 tags)
    /// from a user's resource (i.e. each user modifies their own resource)
    ResourceGroupsSenderWriteTag {
        string_length: usize,
    },
    /// Modifying 3 out of 8 random tags in a resource group
    /// from a user's resource (i.e. each user modifies their own resource)
    ResourceGroupsSenderMultiChange {
        string_length: usize,
    },
    CreateObjects {
        num_objects: u64,
        object_payload_size: u64,
    },
    CreateObjectsConflict {
        num_objects: u64,
        object_payload_size: u64,
    },
    VectorTrimAppend {
        vec_len: u64,
        element_len: u64,
        index: u64,
        repeats: u64,
    },
    VectorRemoveInsert {
        vec_len: u64,
        element_len: u64,
        index: u64,
        repeats: u64,
    },
    VectorRangeMove {
        vec_len: u64,
        element_len: u64,
        index: u64,
        move_len: u64,
        repeats: u64,
    },
    /// Initialize Token V1 NFT collection
    TokenV1InitializeCollection,
    /// Mint an NFT token. Should be called only after InitializeCollection is called
    TokenV1MintAndStoreNFTParallel,
    TokenV1MintAndStoreNFTSequential,
    TokenV1MintAndTransferNFTParallel,
    TokenV1MintAndTransferNFTSequential,
    TokenV1MintAndStoreFT,
    TokenV1MintAndTransferFT,
    // register if not registered already
    CoinInitAndMint,
    FungibleAssetMint,

    TokenV2AmbassadorMint {
        numbered: bool,
    },
    /// Burn an NFT token, only works with numbered=false tokens.
    TokenV2AmbassadorBurn,

    LiquidityPoolSwapInit {
        is_stable: bool,
    },
    LiquidityPoolSwap {
        is_stable: bool,
    },

    InitializeVectorPicture {
        length: u64,
    },
    VectorPicture {
        length: u64,
    },
    VectorPictureRead {
        length: u64,
    },
    InitializeSmartTablePicture,
    SmartTablePicture {
        length: u64,
        num_points_per_txn: usize,
    },
    DeserializeU256,
    /// No-op script with dependencies in *::simple.move. The script has unreachable code that is
    /// there to slow down deserialization & verification, effectively making it more expensive to
    /// load it into code cache.
    SimpleScript,
}

impl EntryPoints {
    pub fn package_name(&self) -> &'static str {
        match self {
            EntryPoints::Republish
            | EntryPoints::Nop
            | EntryPoints::NopFeePayer
            | EntryPoints::Nop2Signers
            | EntryPoints::Nop5Signers
            | EntryPoints::Step
            | EntryPoints::GetCounter
            | EntryPoints::ResetData
            | EntryPoints::Double
            | EntryPoints::Half
            | EntryPoints::Loop { .. }
            | EntryPoints::GetFromConst { .. }
            | EntryPoints::SetId
            | EntryPoints::SetName
            | EntryPoints::Maximize
            | EntryPoints::Minimize
            | EntryPoints::MakeOrChange { .. }
            | EntryPoints::BytesMakeOrChange { .. }
            | EntryPoints::EmitEvents { .. }
            | EntryPoints::MakeOrChangeTable { .. }
            | EntryPoints::MakeOrChangeTableRandom { .. }
            | EntryPoints::SimpleScript => "simple",
            EntryPoints::IncGlobal
            | EntryPoints::IncGlobalAggV2
            | EntryPoints::ModifyGlobalBoundedAggV2 { .. }
            | EntryPoints::CreateObjects { .. }
            | EntryPoints::CreateObjectsConflict { .. }
            | EntryPoints::VectorTrimAppend { .. }
            | EntryPoints::VectorRemoveInsert { .. }
            | EntryPoints::VectorRangeMove { .. }
            | EntryPoints::TokenV1InitializeCollection
            | EntryPoints::TokenV1MintAndStoreNFTParallel
            | EntryPoints::TokenV1MintAndStoreNFTSequential
            | EntryPoints::TokenV1MintAndTransferNFTParallel
            | EntryPoints::TokenV1MintAndTransferNFTSequential
            | EntryPoints::TokenV1MintAndStoreFT
            | EntryPoints::TokenV1MintAndTransferFT
            | EntryPoints::ResourceGroupsGlobalWriteTag { .. }
            | EntryPoints::ResourceGroupsGlobalWriteAndReadTag { .. }
            | EntryPoints::ResourceGroupsSenderWriteTag { .. }
            | EntryPoints::ResourceGroupsSenderMultiChange { .. }
            | EntryPoints::CoinInitAndMint
            | EntryPoints::FungibleAssetMint => "framework_usecases",
            EntryPoints::TokenV2AmbassadorMint { .. } | EntryPoints::TokenV2AmbassadorBurn => {
                "ambassador_token"
            },
            EntryPoints::LiquidityPoolSwapInit { .. }
            | EntryPoints::LiquidityPoolSwap { .. }
            | EntryPoints::InitializeVectorPicture { .. }
            | EntryPoints::VectorPicture { .. }
            | EntryPoints::VectorPictureRead { .. }
            | EntryPoints::InitializeSmartTablePicture
            | EntryPoints::SmartTablePicture { .. } => "complex",
            EntryPoints::IncGlobalMilestoneAggV2 { .. }
            | EntryPoints::CreateGlobalMilestoneAggV2 { .. } => "aggregator_examples",
            EntryPoints::DeserializeU256 => "bcs_stream",
        }
    }

    pub fn module_name(&self) -> &'static str {
        match self {
            EntryPoints::Republish
            | EntryPoints::Nop
            | EntryPoints::NopFeePayer
            | EntryPoints::Nop2Signers
            | EntryPoints::Nop5Signers
            | EntryPoints::Step
            | EntryPoints::GetCounter
            | EntryPoints::ResetData
            | EntryPoints::Double
            | EntryPoints::Half
            | EntryPoints::Loop { .. }
            | EntryPoints::GetFromConst { .. }
            | EntryPoints::SetId
            | EntryPoints::SetName
            | EntryPoints::Maximize
            | EntryPoints::Minimize
            | EntryPoints::MakeOrChange { .. }
            | EntryPoints::BytesMakeOrChange { .. }
            | EntryPoints::EmitEvents { .. }
            | EntryPoints::MakeOrChangeTable { .. }
            | EntryPoints::MakeOrChangeTableRandom { .. }
            | EntryPoints::SimpleScript => "simple",
            EntryPoints::IncGlobal
            | EntryPoints::IncGlobalAggV2
            | EntryPoints::ModifyGlobalBoundedAggV2 { .. } => "aggregator_example",
            EntryPoints::CreateObjects { .. } | EntryPoints::CreateObjectsConflict { .. } => {
                "objects"
            },
            EntryPoints::VectorTrimAppend { .. }
            | EntryPoints::VectorRemoveInsert { .. }
            | EntryPoints::VectorRangeMove { .. } => "vector_example",
            EntryPoints::TokenV1InitializeCollection
            | EntryPoints::TokenV1MintAndStoreNFTParallel
            | EntryPoints::TokenV1MintAndStoreNFTSequential
            | EntryPoints::TokenV1MintAndTransferNFTParallel
            | EntryPoints::TokenV1MintAndTransferNFTSequential
            | EntryPoints::TokenV1MintAndStoreFT
            | EntryPoints::TokenV1MintAndTransferFT => "token_v1",
            EntryPoints::ResourceGroupsGlobalWriteTag { .. }
            | EntryPoints::ResourceGroupsGlobalWriteAndReadTag { .. }
            | EntryPoints::ResourceGroupsSenderWriteTag { .. }
            | EntryPoints::ResourceGroupsSenderMultiChange { .. } => "resource_groups_example",
            EntryPoints::CoinInitAndMint => "coin_example",
            EntryPoints::FungibleAssetMint => "fungible_asset_example",
            EntryPoints::TokenV2AmbassadorMint { .. } | EntryPoints::TokenV2AmbassadorBurn => {
                "ambassador"
            },
            EntryPoints::LiquidityPoolSwapInit { .. } | EntryPoints::LiquidityPoolSwap { .. } => {
                "liquidity_pool_wrapper"
            },
            EntryPoints::InitializeVectorPicture { .. }
            | EntryPoints::VectorPicture { .. }
            | EntryPoints::VectorPictureRead { .. } => "vector_picture",
            EntryPoints::InitializeSmartTablePicture | EntryPoints::SmartTablePicture { .. } => {
                "smart_table_picture"
            },
            EntryPoints::IncGlobalMilestoneAggV2 { .. }
            | EntryPoints::CreateGlobalMilestoneAggV2 { .. } => "counter_with_milestone",
            EntryPoints::DeserializeU256 => "bcs_stream",
        }
    }

    pub fn create_payload(
        &self,
        package: &Package,
        module_name: &str,
        rng: Option<&mut StdRng>,
        other: Option<&AccountAddress>,
    ) -> TransactionPayload {
        let module_id = package.get_module_id(module_name);
        match self {
            EntryPoints::Republish => {
                let (metadata_serialized, code) = package.get_publish_args();
                get_payload(module_id, ident_str!("publish_p").to_owned(), vec![
                    bcs::to_bytes(&metadata_serialized).unwrap(),
                    bcs::to_bytes(&code).unwrap(),
                ])
            },
            // 0 args
            EntryPoints::Nop | EntryPoints::NopFeePayer => {
                get_payload_void(module_id, ident_str!("nop").to_owned())
            },
            EntryPoints::Nop2Signers => {
                get_payload_void(module_id, ident_str!("nop_2_signers").to_owned())
            },
            EntryPoints::Nop5Signers => {
                get_payload_void(module_id, ident_str!("nop_5_signers").to_owned())
            },
            EntryPoints::Step => get_payload_void(module_id, ident_str!("step").to_owned()),
            EntryPoints::GetCounter => {
                get_payload_void(module_id, ident_str!("get_counter").to_owned())
            },
            EntryPoints::ResetData => {
                get_payload_void(module_id, ident_str!("reset_data").to_owned())
            },
            EntryPoints::Double => get_payload_void(module_id, ident_str!("double").to_owned()),
            EntryPoints::Half => get_payload_void(module_id, ident_str!("half").to_owned()),
            EntryPoints::SimpleScript => {
                Package::script(*other.expect("Must provide sender's address"))
            },
            // 1 arg
            EntryPoints::Loop {
                loop_count,
                loop_type,
            } => {
                let count = loop_count
                    .unwrap_or_else(|| rng.expect("Must provide RNG").gen_range(0u64, 1000u64));
                let mut args = vec![bcs::to_bytes(&count).unwrap()];
                let method = match loop_type {
                    LoopType::NoOp => "loop_nop",
                    LoopType::Arithmetic => "loop_arithmetic",
                    LoopType::BcsToBytes { len } => {
                        args.push(bcs::to_bytes(&len).unwrap());
                        "loop_bcs"
                    },
                };
                get_payload(module_id, ident_str!(method).to_owned(), args)
            },
            EntryPoints::GetFromConst { const_idx } => get_from_random_const(
                module_id,
                const_idx.unwrap_or_else(
                    // TODO: get a value in range for the const array in Simple.move
                    || rng.expect("Must provide RNG").gen_range(0u64, 1u64),
                ),
            ),
            EntryPoints::SetId => set_id(rng.expect("Must provide RNG"), module_id),
            EntryPoints::SetName => set_name(rng.expect("Must provide RNG"), module_id),
            // 2 args, second arg existing account address with data
            EntryPoints::Maximize => maximize(module_id, other.expect("Must provide other")),
            EntryPoints::Minimize => minimize(module_id, other.expect("Must provide other")),
            // 3 args
            EntryPoints::MakeOrChange {
                string_length,
                data_length,
            } => {
                let rng = rng.expect("Must provide RNG");
                let str_len = string_length.unwrap_or_else(|| rng.gen_range(0usize, 100usize));
                let data_len = data_length.unwrap_or_else(|| rng.gen_range(0usize, 1000usize));
                make_or_change(rng, module_id, str_len, data_len)
            },
            EntryPoints::BytesMakeOrChange { data_length } => {
                let rng = rng.expect("Must provide RNG");
                let data_len = data_length.unwrap_or_else(|| rng.gen_range(0usize, 1000usize));
                bytes_make_or_change(rng, module_id, data_len)
            },
            EntryPoints::EmitEvents { count } => {
                get_payload(module_id, ident_str!("emit_events").to_owned(), vec![
                    bcs::to_bytes(count).unwrap(),
                ])
            },
            EntryPoints::MakeOrChangeTable { offset, count } => get_payload(
                module_id,
                ident_str!("make_or_change_table").to_owned(),
                vec![
                    bcs::to_bytes(offset).unwrap(),
                    bcs::to_bytes(count).unwrap(),
                ],
            ),
            EntryPoints::MakeOrChangeTableRandom {
                max_offset,
                max_count,
            } => {
                let rng = rng.expect("Must provide RNG");
                let mut offset: u64 = rng.gen();
                offset %= max_offset;
                let mut count: u64 = rng.gen();
                count %= max_count;
                get_payload(
                    module_id,
                    ident_str!("make_or_change_table").to_owned(),
                    vec![
                        bcs::to_bytes(&offset).unwrap(),
                        bcs::to_bytes(&count).unwrap(),
                    ],
                )
            },
            EntryPoints::IncGlobal => {
                get_payload(module_id, ident_str!("increment").to_owned(), vec![])
            },
            EntryPoints::IncGlobalAggV2 => {
                get_payload(module_id, ident_str!("increment_agg_v2").to_owned(), vec![])
            },
            EntryPoints::ModifyGlobalBoundedAggV2 { step } => {
                let rng = rng.expect("Must provide RNG");
                get_payload(
                    module_id,
                    ident_str!("modify_bounded_agg_v2").to_owned(),
                    vec![
                        bcs::to_bytes(&rng.gen::<bool>()).unwrap(),
                        bcs::to_bytes(&step).unwrap(),
                    ],
                )
            },
            EntryPoints::IncGlobalMilestoneAggV2 { .. } => get_payload(
                module_id,
                ident_str!("increment_milestone").to_owned(),
                vec![],
            ),
            EntryPoints::CreateGlobalMilestoneAggV2 { milestone_every } => {
                get_payload(module_id, ident_str!("create").to_owned(), vec![
                    bcs::to_bytes(&milestone_every).unwrap(),
                ])
            },
            EntryPoints::CreateObjects {
                num_objects,
                object_payload_size,
            } => get_payload(module_id, ident_str!("create_objects").to_owned(), vec![
                bcs::to_bytes(num_objects).unwrap(),
                bcs::to_bytes(object_payload_size).unwrap(),
            ]),
            EntryPoints::CreateObjectsConflict {
                num_objects,
                object_payload_size,
            } => get_payload(
                module_id,
                ident_str!("create_objects_conflict").to_owned(),
                vec![
                    bcs::to_bytes(num_objects).unwrap(),
                    bcs::to_bytes(object_payload_size).unwrap(),
                    bcs::to_bytes(other.expect("Must provide other")).unwrap(),
                ],
            ),
            EntryPoints::VectorTrimAppend {
                vec_len,
                element_len,
                index,
                repeats,
            }
            | EntryPoints::VectorRemoveInsert {
                vec_len,
                element_len,
                index,
                repeats,
            } => get_payload(
                module_id,
                ident_str!(
                    if let EntryPoints::VectorTrimAppend { .. } = self {
                        "test_trim_append"
                    } else {
                        "test_remove_insert"
                    }
                )
                .to_owned(),
                vec![
                    bcs::to_bytes(vec_len).unwrap(),
                    bcs::to_bytes(element_len).unwrap(),
                    bcs::to_bytes(index).unwrap(),
                    bcs::to_bytes(repeats).unwrap(),
                ],
            ),
            EntryPoints::VectorRangeMove {
                vec_len,
                element_len,
                index,
                move_len,
                repeats,
            } => get_payload(
                module_id,
                ident_str!("test_middle_range_move").to_owned(),
                vec![
                    bcs::to_bytes(vec_len).unwrap(),
                    bcs::to_bytes(element_len).unwrap(),
                    bcs::to_bytes(index).unwrap(),
                    bcs::to_bytes(move_len).unwrap(),
                    bcs::to_bytes(repeats).unwrap(),
                ],
            ),
            EntryPoints::TokenV1InitializeCollection => get_payload_void(
                module_id,
                ident_str!("token_v1_initialize_collection").to_owned(),
            ),
            EntryPoints::TokenV1MintAndStoreNFTParallel => get_payload(
                module_id,
                ident_str!("token_v1_mint_and_store_nft_parallel").to_owned(),
                vec![bcs::to_bytes(other.expect("Must provide other")).unwrap()],
            ),
            EntryPoints::TokenV1MintAndStoreNFTSequential => get_payload(
                module_id,
                ident_str!("token_v1_mint_and_store_nft_sequential").to_owned(),
                vec![bcs::to_bytes(other.expect("Must provide other")).unwrap()],
            ),

            EntryPoints::TokenV1MintAndTransferNFTParallel => get_payload(
                module_id,
                ident_str!("token_v1_mint_and_transfer_nft_parallel").to_owned(),
                vec![bcs::to_bytes(other.expect("Must provide other")).unwrap()],
            ),
            EntryPoints::TokenV1MintAndTransferNFTSequential => get_payload(
                module_id,
                ident_str!("token_v1_mint_and_transfer_nft_sequential").to_owned(),
                vec![bcs::to_bytes(other.expect("Must provide other")).unwrap()],
            ),
            EntryPoints::TokenV1MintAndStoreFT => get_payload(
                module_id,
                ident_str!("token_v1_mint_and_store_ft").to_owned(),
                vec![bcs::to_bytes(other.expect("Must provide other")).unwrap()],
            ),
            EntryPoints::TokenV1MintAndTransferFT => get_payload(
                module_id,
                ident_str!("token_v1_mint_and_transfer_ft").to_owned(),
                vec![bcs::to_bytes(other.expect("Must provide other")).unwrap()],
            ),
            EntryPoints::ResourceGroupsGlobalWriteTag { string_length }
            | EntryPoints::ResourceGroupsSenderWriteTag { string_length } => {
                let rng: &mut StdRng = rng.expect("Must provide RNG");
                let index: u64 = rng.gen_range(0, 8);
                get_payload(
                    module_id,
                    ident_str!(
                        if let EntryPoints::ResourceGroupsGlobalWriteTag { .. } = self {
                            "set_p"
                        } else {
                            "set"
                        }
                    )
                    .to_owned(),
                    vec![
                        bcs::to_bytes(&index).unwrap(),
                        bcs::to_bytes(&rand_string(rng, *string_length)).unwrap(), // name
                    ],
                )
            },
            EntryPoints::ResourceGroupsGlobalWriteAndReadTag { string_length } => {
                let rng: &mut StdRng = rng.expect("Must provide RNG");
                let index1: u64 = rng.gen_range(0, 8);
                let index2: u64 = rng.gen_range(0, 8);
                get_payload(module_id, ident_str!("set_and_read_p").to_owned(), vec![
                    bcs::to_bytes(&index1).unwrap(),
                    bcs::to_bytes(&index2).unwrap(),
                    bcs::to_bytes(&rand_string(rng, *string_length)).unwrap(), // name
                ])
            },
            EntryPoints::ResourceGroupsSenderMultiChange { string_length } => {
                let rng: &mut StdRng = rng.expect("Must provide RNG");
                let index1: u64 = rng.gen_range(0, 8);
                let index2: u64 = rng.gen_range(0, 8);
                let index3: u64 = rng.gen_range(0, 8);
                get_payload(module_id, ident_str!("set_3").to_owned(), vec![
                    bcs::to_bytes(&index1).unwrap(),
                    bcs::to_bytes(&index2).unwrap(),
                    bcs::to_bytes(&index3).unwrap(),
                    bcs::to_bytes(&rand_string(rng, *string_length)).unwrap(), // name
                ])
            },
            EntryPoints::CoinInitAndMint => {
                get_payload(module_id, ident_str!("mint_p").to_owned(), vec![
                    bcs::to_bytes(&1000u64).unwrap(), // amount
                ])
            },
            EntryPoints::FungibleAssetMint => {
                get_payload(module_id, ident_str!("mint_p").to_owned(), vec![
                    bcs::to_bytes(&1000u64).unwrap(), // amount
                ])
            },
            EntryPoints::TokenV2AmbassadorMint { numbered: true } => {
                let rng: &mut StdRng = rng.expect("Must provide RNG");
                get_payload(
                    module_id,
                    ident_str!("mint_numbered_ambassador_token_by_user").to_owned(),
                    vec![
                        bcs::to_bytes(&rand_string(rng, 100)).unwrap(), // description
                        bcs::to_bytes("superstar #").unwrap(),          // name
                        bcs::to_bytes(&rand_string(rng, 50)).unwrap(),  // uri
                    ],
                )
            },
            EntryPoints::TokenV2AmbassadorMint { numbered: false } => {
                let rng: &mut StdRng = rng.expect("Must provide RNG");
                get_payload(
                    module_id,
                    ident_str!("mint_ambassador_token_by_user").to_owned(),
                    vec![
                        bcs::to_bytes(&rand_string(rng, 100)).unwrap(), // description
                        bcs::to_bytes(&rand_string(rng, 50)).unwrap(),  // uri
                    ],
                )
            },
            EntryPoints::TokenV2AmbassadorBurn => get_payload(
                module_id,
                ident_str!("burn_named_by_user").to_owned(),
                vec![],
            ),

            EntryPoints::LiquidityPoolSwapInit { is_stable } => get_payload(
                module_id,
                ident_str!("initialize_liquid_pair").to_owned(),
                vec![bcs::to_bytes(&is_stable).unwrap()],
            ),
            EntryPoints::LiquidityPoolSwap { is_stable } => {
                let rng: &mut StdRng = rng.expect("Must provide RNG");
                let from_1: bool = (rng.gen_range(0, 2) == 1);
                get_payload(module_id, ident_str!("swap").to_owned(), vec![
                    bcs::to_bytes(&rng.gen_range(1000u64, 2000u64)).unwrap(), // amount_in
                    bcs::to_bytes(&from_1).unwrap(),                          // from_1
                ])
            },
            EntryPoints::InitializeVectorPicture { length } => {
                get_payload(module_id, ident_str!("create").to_owned(), vec![
                    bcs::to_bytes(&length).unwrap(), // length
                ])
            },
            EntryPoints::VectorPicture { length } => {
                let rng: &mut StdRng = rng.expect("Must provide RNG");
                get_payload(module_id, ident_str!("update").to_owned(), vec![
                    bcs::to_bytes(&other.expect("Must provide other")).unwrap(),
                    bcs::to_bytes(&0u64).unwrap(), // palette_index
                    bcs::to_bytes(&rng.gen_range(0u64, length)).unwrap(), // index
                    bcs::to_bytes(&rng.gen_range(0u8, 255u8)).unwrap(), // color R
                    bcs::to_bytes(&rng.gen_range(0u8, 255u8)).unwrap(), // color G
                    bcs::to_bytes(&rng.gen_range(0u8, 255u8)).unwrap(), // color B
                ])
            },
            EntryPoints::VectorPictureRead { length } => {
                let rng: &mut StdRng = rng.expect("Must provide RNG");
                get_payload(module_id, ident_str!("check").to_owned(), vec![
                    bcs::to_bytes(&other.expect("Must provide other")).unwrap(),
                    bcs::to_bytes(&0u64).unwrap(), // palette_index
                    bcs::to_bytes(&rng.gen_range(0u64, length)).unwrap(), // index
                ])
            },
            EntryPoints::InitializeSmartTablePicture => {
                get_payload(module_id, ident_str!("create").to_owned(), vec![])
            },
            EntryPoints::SmartTablePicture {
                length,
                num_points_per_txn,
            } => {
                let rng: &mut StdRng = rng.expect("Must provide RNG");
                u32::try_from(*length).unwrap();
                let mut indices = (0..*num_points_per_txn)
                    .map(|_| rng.gen_range(0u64, length))
                    .collect::<Vec<_>>();
                let mut colors = (0..*num_points_per_txn)
                    .map(|_| rng.gen_range(0u8, 100u8))
                    .collect::<Vec<_>>();
                assert!(indices.len() == colors.len());
                get_payload(module_id, ident_str!("update").to_owned(), vec![
                    bcs::to_bytes(&other.expect("Must provide other")).unwrap(),
                    bcs::to_bytes(&0u64).unwrap(),    // palette_index
                    bcs::to_bytes(&indices).unwrap(), // indices
                    bcs::to_bytes(&colors).unwrap(),  // colors
                ])
            },
            EntryPoints::DeserializeU256 => {
                let rng: &mut StdRng = rng.expect("Must provide RNG");
                let mut u256_bytes = [0u8; 32];
                rng.fill_bytes(&mut u256_bytes);
                get_payload(
                    module_id,
                    ident_str!("deserialize_u256_entry").to_owned(),
                    vec![
                        bcs::to_bytes(&u256_bytes.to_vec()).unwrap(),
                        bcs::to_bytes(&0u64).unwrap(),
                    ],
                )
            },
        }
    }

    pub fn initialize_entry_point(&self) -> Option<EntryPoints> {
        match self {
            EntryPoints::TokenV1MintAndStoreNFTParallel
            | EntryPoints::TokenV1MintAndStoreNFTSequential
            | EntryPoints::TokenV1MintAndTransferNFTParallel
            | EntryPoints::TokenV1MintAndTransferNFTSequential
            | EntryPoints::TokenV1MintAndStoreFT
            | EntryPoints::TokenV1MintAndTransferFT => {
                Some(EntryPoints::TokenV1InitializeCollection)
            },
            EntryPoints::LiquidityPoolSwap { is_stable } => {
                Some(EntryPoints::LiquidityPoolSwapInit {
                    is_stable: *is_stable,
                })
            },
            EntryPoints::VectorPicture { length } | EntryPoints::VectorPictureRead { length } => {
                Some(EntryPoints::InitializeVectorPicture { length: *length })
            },
            EntryPoints::SmartTablePicture { .. } => Some(EntryPoints::InitializeSmartTablePicture),
            EntryPoints::IncGlobalMilestoneAggV2 { milestone_every } => {
                Some(EntryPoints::CreateGlobalMilestoneAggV2 {
                    milestone_every: *milestone_every,
                })
            },
            _ => None,
        }
    }

    pub fn multi_sig_additional_num(&self) -> MultiSigConfig {
        match self {
            EntryPoints::Republish => MultiSigConfig::Publisher,
            EntryPoints::NopFeePayer => MultiSigConfig::FeePayerPublisher,
            EntryPoints::Nop2Signers => MultiSigConfig::Random(1),
            EntryPoints::Nop5Signers => MultiSigConfig::Random(4),
            EntryPoints::ResourceGroupsGlobalWriteTag { .. }
            | EntryPoints::ResourceGroupsGlobalWriteAndReadTag { .. } => MultiSigConfig::Publisher,
            EntryPoints::CoinInitAndMint | EntryPoints::FungibleAssetMint => {
                MultiSigConfig::Publisher
            },
            EntryPoints::TokenV2AmbassadorMint { .. } | EntryPoints::TokenV2AmbassadorBurn => {
                MultiSigConfig::Publisher
            },
            EntryPoints::LiquidityPoolSwap { .. } => MultiSigConfig::Publisher,
            EntryPoints::CreateGlobalMilestoneAggV2 { .. } => MultiSigConfig::Publisher,
            _ => MultiSigConfig::None,
        }
    }

    pub fn automatic_args(&self) -> AutomaticArgs {
        match self {
            EntryPoints::Republish => AutomaticArgs::Signer,
            EntryPoints::Nop
            | EntryPoints::NopFeePayer
            | EntryPoints::Step
            | EntryPoints::GetCounter
            | EntryPoints::ResetData
            | EntryPoints::Double
            | EntryPoints::Half
            | EntryPoints::Loop { .. }
            | EntryPoints::GetFromConst { .. }
            | EntryPoints::SetId
            | EntryPoints::SetName
            | EntryPoints::Maximize
            | EntryPoints::Minimize
            | EntryPoints::MakeOrChange { .. }
            | EntryPoints::BytesMakeOrChange { .. }
            | EntryPoints::EmitEvents { .. }
            | EntryPoints::MakeOrChangeTable { .. }
            | EntryPoints::MakeOrChangeTableRandom { .. }
            | EntryPoints::SimpleScript => AutomaticArgs::Signer,
            EntryPoints::Nop2Signers | EntryPoints::Nop5Signers => AutomaticArgs::SignerAndMultiSig,
            EntryPoints::IncGlobal
            | EntryPoints::IncGlobalAggV2
            | EntryPoints::ModifyGlobalBoundedAggV2 { .. } => AutomaticArgs::None,
            EntryPoints::CreateObjects { .. } | EntryPoints::CreateObjectsConflict { .. } => {
                AutomaticArgs::Signer
            },
            EntryPoints::VectorTrimAppend { .. }
            | EntryPoints::VectorRemoveInsert { .. }
            | EntryPoints::VectorRangeMove { .. } => AutomaticArgs::None,
            EntryPoints::TokenV1InitializeCollection
            | EntryPoints::TokenV1MintAndStoreNFTParallel
            | EntryPoints::TokenV1MintAndStoreNFTSequential
            | EntryPoints::TokenV1MintAndTransferNFTParallel
            | EntryPoints::TokenV1MintAndTransferNFTSequential
            | EntryPoints::TokenV1MintAndStoreFT
            | EntryPoints::TokenV1MintAndTransferFT => AutomaticArgs::Signer,
            EntryPoints::ResourceGroupsGlobalWriteTag { .. }
            | EntryPoints::ResourceGroupsGlobalWriteAndReadTag { .. } => {
                AutomaticArgs::SignerAndMultiSig
            },
            EntryPoints::ResourceGroupsSenderWriteTag { .. }
            | EntryPoints::ResourceGroupsSenderMultiChange { .. } => AutomaticArgs::Signer,
            EntryPoints::CoinInitAndMint | EntryPoints::FungibleAssetMint => {
                AutomaticArgs::SignerAndMultiSig
            },
            EntryPoints::TokenV2AmbassadorMint { .. } | EntryPoints::TokenV2AmbassadorBurn => {
                AutomaticArgs::SignerAndMultiSig
            },
            EntryPoints::LiquidityPoolSwapInit { .. } => AutomaticArgs::Signer,
            EntryPoints::LiquidityPoolSwap { .. } => AutomaticArgs::SignerAndMultiSig,
            EntryPoints::InitializeVectorPicture { .. } => AutomaticArgs::Signer,
            EntryPoints::VectorPicture { .. } | EntryPoints::VectorPictureRead { .. } => {
                AutomaticArgs::None
            },
            EntryPoints::InitializeSmartTablePicture => AutomaticArgs::Signer,
            EntryPoints::SmartTablePicture { .. } => AutomaticArgs::None,
            EntryPoints::DeserializeU256 => AutomaticArgs::None,
            EntryPoints::IncGlobalMilestoneAggV2 { .. } => AutomaticArgs::None,
            EntryPoints::CreateGlobalMilestoneAggV2 { .. } => AutomaticArgs::Signer,
        }
    }
}

const SIMPLE_ENTRY_POINTS: &[EntryPoints; 9] = &[
    EntryPoints::Nop,
    EntryPoints::Step,
    EntryPoints::GetCounter,
    EntryPoints::ResetData,
    EntryPoints::Double,
    EntryPoints::Half,
    EntryPoints::Loop {
        loop_count: None,
        loop_type: LoopType::NoOp,
    },
    EntryPoints::GetFromConst { const_idx: None },
    EntryPoints::SetId,
];
const GEN_ENTRY_POINTS: &[EntryPoints; 12] = &[
    EntryPoints::Nop,
    EntryPoints::Step,
    EntryPoints::GetCounter,
    EntryPoints::ResetData,
    EntryPoints::Double,
    EntryPoints::Half,
    EntryPoints::Loop {
        loop_count: None,
        loop_type: LoopType::NoOp,
    },
    EntryPoints::GetFromConst { const_idx: None },
    EntryPoints::SetId,
    EntryPoints::SetName,
    EntryPoints::MakeOrChange {
        string_length: None,
        data_length: None,
    },
    EntryPoints::BytesMakeOrChange { data_length: None },
];

pub fn rand_simple_function(
    package: &Package,
    module_name: &str,
    rng: &mut StdRng,
) -> TransactionPayload {
    SIMPLE_ENTRY_POINTS
        .choose(rng)
        .unwrap()
        .create_payload(package, module_name, Some(rng), None)
}

pub fn rand_gen_function(
    package: &Package,
    module_name: &str,
    rng: &mut StdRng,
) -> TransactionPayload {
    GEN_ENTRY_POINTS
        .choose(rng)
        .unwrap()
        .create_payload(package, module_name, Some(rng), None)
}

//
// Entry points payload
//

fn get_from_random_const(module_id: ModuleId, idx: u64) -> TransactionPayload {
    get_payload(
        module_id,
        ident_str!("get_from_random_const").to_owned(),
        vec![bcs::to_bytes(&idx).unwrap()],
    )
}

fn set_id(rng: &mut StdRng, module_id: ModuleId) -> TransactionPayload {
    let id: u64 = rng.gen();
    get_payload(module_id, ident_str!("set_id").to_owned(), vec![
        bcs::to_bytes(&id).unwrap(),
    ])
}

fn set_name(rng: &mut StdRng, module_id: ModuleId) -> TransactionPayload {
    let len = rng.gen_range(0usize, 1000usize);
    let name: String = rand_string(rng, len);
    get_payload(module_id, ident_str!("set_name").to_owned(), vec![
        bcs::to_bytes(&name).unwrap(),
    ])
}

fn maximize(module_id: ModuleId, other: &AccountAddress) -> TransactionPayload {
    get_payload(module_id, ident_str!("maximize").to_owned(), vec![
        bcs::to_bytes(other).unwrap(),
    ])
}

fn minimize(module_id: ModuleId, other: &AccountAddress) -> TransactionPayload {
    get_payload(module_id, ident_str!("minimize").to_owned(), vec![
        bcs::to_bytes(other).unwrap(),
    ])
}

fn rand_string(rng: &mut StdRng, len: usize) -> String {
    let res = rng
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect();
    assert_eq!(
        bcs::serialized_size(&res).unwrap(),
        bcs_size_of_byte_array(len)
    );
    res
}

fn make_or_change(
    rng: &mut StdRng,
    module_id: ModuleId,
    str_len: usize,
    data_len: usize,
) -> TransactionPayload {
    let id: u64 = rng.gen();
    let name: String = rand_string(rng, str_len);
    let mut bytes = Vec::<u8>::with_capacity(data_len);
    rng.fill_bytes(&mut bytes);
    get_payload(module_id, ident_str!("make_or_change").to_owned(), vec![
        bcs::to_bytes(&id).unwrap(),
        bcs::to_bytes(&name).unwrap(),
        bcs::to_bytes(&bytes).unwrap(),
    ])
}

fn bytes_make_or_change(
    rng: &mut StdRng,
    module_id: ModuleId,
    data_len: usize,
) -> TransactionPayload {
    let mut bytes = Vec::<u8>::with_capacity(data_len);
    rng.fill_bytes(&mut bytes);
    get_payload(
        module_id,
        ident_str!("bytes_make_or_change").to_owned(),
        vec![bcs::to_bytes(&bytes).unwrap()],
    )
}

fn get_payload_void(module_id: ModuleId, func: Identifier) -> TransactionPayload {
    get_payload(module_id, func, vec![])
}

fn get_payload(module_id: ModuleId, func: Identifier, args: Vec<Vec<u8>>) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(module_id, func, vec![], args))
}
