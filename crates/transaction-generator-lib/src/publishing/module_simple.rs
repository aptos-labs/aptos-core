// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
#![allow(unused)]

use aptos_framework::natives::code::{MoveOption, PackageMetadata};
use aptos_sdk::{
    bcs,
    move_types::{
        account_address::AccountAddress, ident_str, identifier::Identifier,
        language_storage::ModuleId,
    },
    types::transaction::{EntryFunction, TransactionPayload},
};
use move_binary_format::{
    file_format::{FunctionHandleIndex, IdentifierIndex, SignatureToken},
    CompiledModule,
};
use rand::{distributions::Alphanumeric, prelude::StdRng, seq::SliceRandom, Rng};
use rand_core::RngCore;

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
}

#[derive(Debug, Copy, Clone)]
pub enum LoopType {
    NoOp,
    Arithmetic,
    BcsToBytes { len: u64 },
}

//
// List of entry points to expose
//
// More info in the Simple.move
#[derive(Debug, Copy, Clone)]
pub enum EntryPoints {
    /// Empty (NoOp) function
    Nop,
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
    /// Increment global (publisher) resource - COUNTER_STEP
    IncGlobal,
    /// Increment global (publisher) AggregatorV2 resource - COUNTER_STEP
    IncGlobalAggV2,
    /// Modify (try_add(step) or try_sub(step)) AggregatorV2 bounded counter (counter with max_value=100)
    ModifyGlobalBoundedAggV2 {
        step: u64,
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
    /// Initialize Token V1 NFT collection
    TokenV1InitializeCollection,
    /// Mint an NFT token. Should be called only after InitializeCollection is called
    TokenV1MintAndStoreNFTParallel,
    TokenV1MintAndStoreNFTSequential,
    TokenV1MintAndTransferNFTParallel,
    TokenV1MintAndTransferNFTSequential,
    TokenV1MintAndStoreFT,
    TokenV1MintAndTransferFT,

    TokenV2AmbassadorMint,

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
}

impl EntryPoints {
    pub fn package_name(&self) -> &'static str {
        match self {
            EntryPoints::Nop
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
            | EntryPoints::MakeOrChangeTableRandom { .. } => "simple",
            EntryPoints::IncGlobal
            | EntryPoints::IncGlobalAggV2
            | EntryPoints::ModifyGlobalBoundedAggV2 { .. }
            | EntryPoints::CreateObjects { .. }
            | EntryPoints::CreateObjectsConflict { .. }
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
            | EntryPoints::ResourceGroupsSenderMultiChange { .. } => "framework_usecases",
            EntryPoints::TokenV2AmbassadorMint => "ambassador_token",
            EntryPoints::InitializeVectorPicture { .. }
            | EntryPoints::VectorPicture { .. }
            | EntryPoints::VectorPictureRead { .. }
            | EntryPoints::InitializeSmartTablePicture
            | EntryPoints::SmartTablePicture { .. } => "complex",
        }
    }

    pub fn module_name(&self) -> &'static str {
        match self {
            EntryPoints::Nop
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
            | EntryPoints::MakeOrChangeTableRandom { .. } => "simple",
            EntryPoints::IncGlobal
            | EntryPoints::IncGlobalAggV2
            | EntryPoints::ModifyGlobalBoundedAggV2 { .. } => "aggregator_example",
            EntryPoints::CreateObjects { .. } | EntryPoints::CreateObjectsConflict { .. } => {
                "objects"
            },
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
            EntryPoints::TokenV2AmbassadorMint => "ambassador",
            EntryPoints::InitializeVectorPicture { .. }
            | EntryPoints::VectorPicture { .. }
            | EntryPoints::VectorPictureRead { .. } => "vector_picture",
            EntryPoints::InitializeSmartTablePicture | EntryPoints::SmartTablePicture { .. } => {
                "smart_table_picture"
            },
        }
    }

    pub fn create_payload(
        &self,
        module_id: ModuleId,
        rng: Option<&mut StdRng>,
        other: Option<&AccountAddress>,
    ) -> TransactionPayload {
        match self {
            // 0 args
            EntryPoints::Nop => get_payload_void(module_id, ident_str!("nop").to_owned()),
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
            EntryPoints::IncGlobal => inc_global(module_id),
            EntryPoints::IncGlobalAggV2 => inc_global_agg_v2(module_id),
            EntryPoints::ModifyGlobalBoundedAggV2 { step } => {
                modify_bounded_agg_v2(module_id, rng.expect("Must provide RNG"), *step)
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
            EntryPoints::TokenV2AmbassadorMint => {
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
            EntryPoints::VectorPicture { length } | EntryPoints::VectorPictureRead { length } => {
                Some(EntryPoints::InitializeVectorPicture { length: *length })
            },
            EntryPoints::SmartTablePicture { .. } => Some(EntryPoints::InitializeSmartTablePicture),
            _ => None,
        }
    }

    pub fn multi_sig_additional_num(&self) -> MultiSigConfig {
        match self {
            EntryPoints::Nop2Signers => MultiSigConfig::Random(1),
            EntryPoints::Nop5Signers => MultiSigConfig::Random(4),
            EntryPoints::ResourceGroupsGlobalWriteTag { .. }
            | EntryPoints::ResourceGroupsGlobalWriteAndReadTag { .. } => MultiSigConfig::Publisher,
            EntryPoints::TokenV2AmbassadorMint => MultiSigConfig::Publisher,
            _ => MultiSigConfig::None,
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

pub fn rand_simple_function(rng: &mut StdRng, module_id: ModuleId) -> TransactionPayload {
    SIMPLE_ENTRY_POINTS
        .choose(rng)
        .unwrap()
        .create_payload(module_id, Some(rng), None)
}

pub fn rand_gen_function(rng: &mut StdRng, module_id: ModuleId) -> TransactionPayload {
    GEN_ENTRY_POINTS
        .choose(rng)
        .unwrap()
        .create_payload(module_id, Some(rng), None)
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

fn inc_global(module_id: ModuleId) -> TransactionPayload {
    get_payload(module_id, ident_str!("increment").to_owned(), vec![])
}

fn inc_global_agg_v2(module_id: ModuleId) -> TransactionPayload {
    get_payload(module_id, ident_str!("increment_agg_v2").to_owned(), vec![])
}

fn modify_bounded_agg_v2(module_id: ModuleId, rng: &mut StdRng, step: u64) -> TransactionPayload {
    get_payload(
        module_id,
        ident_str!("modify_bounded_agg_v2").to_owned(),
        vec![
            bcs::to_bytes(&rng.gen::<bool>()).unwrap(),
            bcs::to_bytes(&step).unwrap(),
        ],
    )
}

fn mint_new_token(module_id: ModuleId, other: AccountAddress) -> TransactionPayload {
    get_payload(module_id, ident_str!("mint_new_token").to_owned(), vec![
        bcs::to_bytes(&other).unwrap(),
    ])
}

fn rand_string(rng: &mut StdRng, len: usize) -> String {
    rng.sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
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
