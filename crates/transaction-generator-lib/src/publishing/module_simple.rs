// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
#![allow(unused)]

use aptos_framework::natives::code::{MoveOption, PackageMetadata};
use aptos_sdk::{
    bcs,
    move_types::{
        account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
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
            module.identifiers.push(Identifier::from(name.as_str()));
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

//
// List of entry points to expose
//
// More info in the Simple.move
#[derive(Debug, Copy, Clone)]
pub enum EntryPoints {
    // 0 args
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
    // 1 arg
    /// run a for loop
    Loopy {
        loop_count: Option<u64>,
    },
    /// Return value from constant array (RANDOM)
    GetFromConst {
        const_idx: Option<u64>,
    },
    /// Set the `Resource.id`
    SetId,
    /// Set the `Resource.name`
    SetName,
    // 2 args
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
    /// Increment destination resource - COUNTER_STEP
    StepDst,

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
            | EntryPoints::Loopy { .. }
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
            | EntryPoints::StepDst => "simple",
            EntryPoints::TokenV1InitializeCollection
            | EntryPoints::TokenV1MintAndStoreNFTParallel
            | EntryPoints::TokenV1MintAndStoreNFTSequential
            | EntryPoints::TokenV1MintAndTransferNFTParallel
            | EntryPoints::TokenV1MintAndTransferNFTSequential
            | EntryPoints::TokenV1MintAndStoreFT
            | EntryPoints::TokenV1MintAndTransferFT => "framework_usecases",
            EntryPoints::TokenV2AmbassadorMint => "ambassador_token",
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
            | EntryPoints::Loopy { .. }
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
            | EntryPoints::StepDst => "simple",
            EntryPoints::TokenV1InitializeCollection
            | EntryPoints::TokenV1MintAndStoreNFTParallel
            | EntryPoints::TokenV1MintAndStoreNFTSequential
            | EntryPoints::TokenV1MintAndTransferNFTParallel
            | EntryPoints::TokenV1MintAndTransferNFTSequential
            | EntryPoints::TokenV1MintAndStoreFT
            | EntryPoints::TokenV1MintAndTransferFT => "token_v1",
            EntryPoints::TokenV2AmbassadorMint => "ambassador",
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
            EntryPoints::Nop => get_payload_void(module_id, Identifier::from("nop")),
            EntryPoints::Nop2Signers => {
                get_payload_void(module_id, Identifier::from("nop_2_signers"))
            },
            EntryPoints::Nop5Signers => {
                get_payload_void(module_id, Identifier::from("nop_5_signers"))
            },
            EntryPoints::Step => get_payload_void(module_id, Identifier::from("step")),
            EntryPoints::GetCounter => get_payload_void(module_id, Identifier::from("get_counter")),
            EntryPoints::ResetData => get_payload_void(module_id, Identifier::from("reset_data")),
            EntryPoints::Double => get_payload_void(module_id, Identifier::from("double")),
            EntryPoints::Half => get_payload_void(module_id, Identifier::from("half")),
            // 1 arg
            EntryPoints::Loopy { loop_count } => loopy(
                module_id,
                loop_count
                    .unwrap_or_else(|| rng.expect("Must provide RNG").gen_range(0u64, 1000u64)),
            ),
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
                get_payload(module_id, Identifier::from("emit_events"), vec![
                    bcs::to_bytes(count).unwrap(),
                ])
            },
            EntryPoints::MakeOrChangeTable { offset, count } => {
                get_payload(module_id, Identifier::from("make_or_change_table"), vec![
                    bcs::to_bytes(offset).unwrap(),
                    bcs::to_bytes(count).unwrap(),
                ])
            },
            EntryPoints::MakeOrChangeTableRandom {
                max_offset,
                max_count,
            } => {
                let rng = rng.expect("Must provide RNG");
                let mut offset: u64 = rng.gen();
                offset %= max_offset;
                let mut count: u64 = rng.gen();
                count %= max_count;
                get_payload(module_id, Identifier::from("make_or_change_table"), vec![
                    bcs::to_bytes(&offset).unwrap(),
                    bcs::to_bytes(&count).unwrap(),
                ])
            },
            EntryPoints::StepDst => step_dst(module_id, other.expect("Must provide other")),
            EntryPoints::TokenV1InitializeCollection => get_payload_void(
                module_id,
                Identifier::from("token_v1_initialize_collection"),
            ),
            EntryPoints::TokenV1MintAndStoreNFTParallel => get_payload(
                module_id,
                Identifier::from("token_v1_mint_and_store_nft_parallel"),
                vec![bcs::to_bytes(other.expect("Must provide other")).unwrap()],
            ),
            EntryPoints::TokenV1MintAndStoreNFTSequential => get_payload(
                module_id,
                Identifier::from("token_v1_mint_and_store_nft_sequential"),
                vec![bcs::to_bytes(other.expect("Must provide other")).unwrap()],
            ),

            EntryPoints::TokenV1MintAndTransferNFTParallel => get_payload(
                module_id,
                Identifier::from("token_v1_mint_and_transfer_nft_parallel"),
                vec![bcs::to_bytes(other.expect("Must provide other")).unwrap()],
            ),
            EntryPoints::TokenV1MintAndTransferNFTSequential => get_payload(
                module_id,
                Identifier::from("token_v1_mint_and_transfer_nft_sequential"),
                vec![bcs::to_bytes(other.expect("Must provide other")).unwrap()],
            ),
            EntryPoints::TokenV1MintAndStoreFT => get_payload(
                module_id,
                Identifier::from("token_v1_mint_and_store_ft"),
                vec![bcs::to_bytes(other.expect("Must provide other")).unwrap()],
            ),
            EntryPoints::TokenV1MintAndTransferFT => get_payload(
                module_id,
                Identifier::from("token_v1_mint_and_transfer_ft"),
                vec![bcs::to_bytes(other.expect("Must provide other")).unwrap()],
            ),
            EntryPoints::TokenV2AmbassadorMint => {
                let rng: &mut StdRng = rng.expect("Must provide RNG");
                get_payload(
                    module_id,
                    Identifier::from("mint_ambassador_token_by_user"),
                    vec![
                        bcs::to_bytes(&rand_string(rng, 100)).unwrap(), // description
                        bcs::to_bytes(&rand_string(rng, 20)).unwrap(),  // name
                        bcs::to_bytes(&rand_string(rng, 50)).unwrap(),  // uri
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
            _ => None,
        }
    }

    pub fn multi_sig_additional_num(&self) -> MultiSigConfig {
        match self {
            EntryPoints::Nop2Signers => MultiSigConfig::Random(1),
            EntryPoints::Nop5Signers => MultiSigConfig::Random(4),
            EntryPoints::TokenV2AmbassadorMint => MultiSigConfig::Publisher,
            _ => MultiSigConfig::None,
        }
    }
}

const ZERO_ARG_ENTRY_POINTS: &[EntryPoints; 6] = &[
    EntryPoints::Nop,
    EntryPoints::Step,
    EntryPoints::GetCounter,
    EntryPoints::ResetData,
    EntryPoints::Double,
    EntryPoints::Half,
];
const ONE_ARG_ENTRY_POINTS: &[EntryPoints; 4] = &[
    EntryPoints::Loopy { loop_count: None },
    EntryPoints::GetFromConst { const_idx: None },
    EntryPoints::SetId,
    EntryPoints::SetName,
];
const SIMPLE_ENTRY_POINTS: &[EntryPoints; 9] = &[
    EntryPoints::Nop,
    EntryPoints::Step,
    EntryPoints::GetCounter,
    EntryPoints::ResetData,
    EntryPoints::Double,
    EntryPoints::Half,
    EntryPoints::Loopy { loop_count: None },
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
    EntryPoints::Loopy { loop_count: None },
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

pub fn zero_args_function(rng: &mut StdRng, module_id: ModuleId) -> TransactionPayload {
    ZERO_ARG_ENTRY_POINTS
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

fn loopy(module_id: ModuleId, count: u64) -> TransactionPayload {
    get_payload(module_id, Identifier::from("loopy"), vec![bcs::to_bytes(
        &count,
    )
    .unwrap()])
}

fn get_from_random_const(module_id: ModuleId, idx: u64) -> TransactionPayload {
    get_payload(module_id, Identifier::from("get_from_random_const"), vec![
        bcs::to_bytes(&idx).unwrap(),
    ])
}

fn set_id(rng: &mut StdRng, module_id: ModuleId) -> TransactionPayload {
    let id: u64 = rng.gen();
    get_payload(module_id, Identifier::from("set_id"), vec![bcs::to_bytes(
        &id,
    )
    .unwrap()])
}

fn set_name(rng: &mut StdRng, module_id: ModuleId) -> TransactionPayload {
    let len = rng.gen_range(0usize, 1000usize);
    let name: String = rand_string(rng, len);
    get_payload(module_id, Identifier::from("set_name"), vec![
        bcs::to_bytes(&name).unwrap(),
    ])
}

fn maximize(module_id: ModuleId, other: &AccountAddress) -> TransactionPayload {
    get_payload(module_id, Identifier::from("maximize"), vec![
        bcs::to_bytes(other).unwrap(),
    ])
}

fn minimize(module_id: ModuleId, other: &AccountAddress) -> TransactionPayload {
    get_payload(module_id, Identifier::from("minimize"), vec![
        bcs::to_bytes(other).unwrap(),
    ])
}

fn step_dst(module_id: ModuleId, dst: &AccountAddress) -> TransactionPayload {
    get_payload(module_id, Identifier::from("step_destination"), vec![
        bcs::to_bytes(dst).unwrap(),
    ])
}

fn mint_new_token(module_id: ModuleId, other: AccountAddress) -> TransactionPayload {
    get_payload(module_id, Identifier::from("mint_new_token"), vec![
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
    get_payload(module_id, Identifier::from("make_or_change"), vec![
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
    get_payload(module_id, Identifier::from("bytes_make_or_change"), vec![
        bcs::to_bytes(&bytes).unwrap(),
    ])
}

fn get_payload_void(module_id: ModuleId, func: Identifier) -> TransactionPayload {
    get_payload(module_id, func, vec![])
}

fn get_payload(module_id: ModuleId, func: Identifier, args: Vec<Vec<u8>>) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(module_id, func, vec![], args))
}
