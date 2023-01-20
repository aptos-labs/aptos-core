// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
#![allow(unused)]

use crate::transaction_generator::publishing::raw_module_data;
use aptos_framework::natives::code::PackageMetadata;
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
use rand::{distributions::Alphanumeric, prelude::StdRng, Rng};
use rand_core::RngCore;

//
// Contains all the code to work on the Simple package
//

//
// Functions to load and update the original package
//

pub fn load_package() -> (Vec<CompiledModule>, PackageMetadata) {
    let metadata = bcs::from_bytes::<PackageMetadata>(&raw_module_data::PACKAGE_METADATA_SIMPLE)
        .expect("PackageMetadata for GenericModule must deserialize");
    let mut modules = vec![];
    let module = CompiledModule::deserialize(&raw_module_data::MODULE_SIMPLE)
        .expect("Simple.move must deserialize");
    modules.push(module);
    (modules, metadata)
}

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

//
// List of entry points to expose
//
enum EntryPoints {
    // 0 args
    Nop = 0,
    Step = 1,
    GetCounter = 2,
    ResetData = 3,
    Double = 4,
    Half = 5,
    // 1 arg
    Loopy = 6,
    GetFromRandomConst = 7,
    SetId = 8,
    SetName = 9,
    // 2 args
    // next 2 functions, second arg must be existing account address with data
    Maximize = 10,
    Minimize = 11,
    // 3 args
    MakeOrChange = 12,
}

const ENTRY_POINTS_START: u8 = EntryPoints::Nop as u8;
const ENTRY_POINTS_END: u8 = EntryPoints::MakeOrChange as u8;
const ZERO_ARG_ENTRY_POINTS_START: u8 = EntryPoints::Nop as u8;
const ZERO_ARG_ENTRY_POINTS_END: u8 = EntryPoints::Half as u8;
const ONE_ARG_ENTRY_POINTS_START: u8 = EntryPoints::Loopy as u8;
const ONE_ARG_ENTRY_POINTS_END: u8 = EntryPoints::SetName as u8;
const TWO_ARG_ENTRY_POINTS_START: u8 = EntryPoints::Maximize as u8;
const TWO_ARG_ENTRY_POINTS_END: u8 = EntryPoints::Minimize as u8;
const THREE_ARG_ENTRY_POINTS_START: u8 = EntryPoints::MakeOrChange as u8;
const THREE_ARG_ENTRY_POINTS_END: u8 = EntryPoints::MakeOrChange as u8;

impl TryFrom<u8> for EntryPoints {
    type Error = &'static str;

    fn try_from(val: u8) -> Result<EntryPoints, &'static str> {
        match val {
            0 => Ok(EntryPoints::Nop),
            1 => Ok(EntryPoints::Step),
            2 => Ok(EntryPoints::GetCounter),
            3 => Ok(EntryPoints::ResetData),
            4 => Ok(EntryPoints::Double),
            5 => Ok(EntryPoints::Half),
            6 => Ok(EntryPoints::Loopy),
            7 => Ok(EntryPoints::GetFromRandomConst),
            8 => Ok(EntryPoints::SetId),
            9 => Ok(EntryPoints::SetName),
            10 => Ok(EntryPoints::Maximize),
            11 => Ok(EntryPoints::Minimize),
            12 => Ok(EntryPoints::MakeOrChange),
            _ => Err("Value out of range for EntryPoints"),
        }
    }
}

fn call_function(
    fun_idx: u8,
    rng: &mut StdRng,
    module_id: ModuleId,
    other: Option<AccountAddress>,
) -> TransactionPayload {
    match EntryPoints::try_from(fun_idx).expect("Must pick a function in range, bogus id generated")
    {
        // 0 args
        EntryPoints::Nop => get_payload_void(module_id, ident_str!("nop").to_owned()),
        EntryPoints::Step => get_payload_void(module_id, ident_str!("step").to_owned()),
        EntryPoints::GetCounter => {
            get_payload_void(module_id, ident_str!("get_counter").to_owned())
        },
        EntryPoints::ResetData => get_payload_void(module_id, ident_str!("reset_data").to_owned()),
        EntryPoints::Double => get_payload_void(module_id, ident_str!("double").to_owned()),
        EntryPoints::Half => get_payload_void(module_id, ident_str!("half").to_owned()),
        // 1 arg
        EntryPoints::Loopy => loopy(rng, module_id),
        EntryPoints::GetFromRandomConst => get_from_random_const(rng, module_id),
        EntryPoints::SetId => set_id(rng, module_id),
        EntryPoints::SetName => set_name(rng, module_id),
        // 2 args, second arg existing account address with data
        EntryPoints::Maximize => maximize(module_id, other.expect("Must provide other")),
        EntryPoints::Minimize => minimize(module_id, other.expect("Must provide other")),
        // 3 args
        EntryPoints::MakeOrChange => make_or_change(rng, module_id),
    }
}

pub fn any_function(
    rng: &mut StdRng,
    module_id: ModuleId,
    other: Option<AccountAddress>,
) -> TransactionPayload {
    let fun_idx = rng.gen_range(ENTRY_POINTS_START, ENTRY_POINTS_END + 1);
    call_function(fun_idx, rng, module_id, other)
}

pub fn rand_simple_function(rng: &mut StdRng, module_id: ModuleId) -> TransactionPayload {
    let fun_idx = rng.gen_range(ZERO_ARG_ENTRY_POINTS_START, EntryPoints::SetName as u8);
    call_function(fun_idx, rng, module_id, None)
}

pub fn zero_args_function(rng: &mut StdRng, module_id: ModuleId) -> TransactionPayload {
    let fun_idx = rng.gen_range(ZERO_ARG_ENTRY_POINTS_START, ZERO_ARG_ENTRY_POINTS_END + 1);
    call_function(fun_idx, rng, module_id, None)
}

pub fn rand_gen_function(rng: &mut StdRng, module_id: ModuleId) -> TransactionPayload {
    let fun_idx = rng.gen_range(ZERO_ARG_ENTRY_POINTS_START, ONE_ARG_ENTRY_POINTS_END + 1);
    call_function(fun_idx, rng, module_id, None)
}

//
// Entry points payload
//

fn loopy(rng: &mut StdRng, module_id: ModuleId) -> TransactionPayload {
    let count = rng.gen_range(0u64, 1000u64);
    get_payload(module_id, ident_str!("loopy").to_owned(), vec![
        bcs::to_bytes(&count).unwrap(),
    ])
}

fn get_from_random_const(rng: &mut StdRng, module_id: ModuleId) -> TransactionPayload {
    // TODO: get a value in range for the const array in Simple.move
    let idx = rng.gen_range(0u64, 1u64);
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
    let name: String = rng
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect();
    get_payload(module_id, ident_str!("set_name").to_owned(), vec![
        bcs::to_bytes(&name).unwrap(),
    ])
}

fn maximize(module_id: ModuleId, other: AccountAddress) -> TransactionPayload {
    get_payload(module_id, ident_str!("maximize").to_owned(), vec![
        bcs::to_bytes(&other).unwrap(),
    ])
}

fn minimize(module_id: ModuleId, other: AccountAddress) -> TransactionPayload {
    get_payload(module_id, ident_str!("minimize").to_owned(), vec![
        bcs::to_bytes(&other).unwrap(),
    ])
}

fn make_or_change(rng: &mut StdRng, module_id: ModuleId) -> TransactionPayload {
    let id: u64 = rng.gen();
    let len = rng.gen_range(0usize, 100usize);
    let name: String = rng
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect();
    let len = rng.gen_range(0usize, 1000usize);
    let mut bytes = Vec::<u8>::with_capacity(len);
    rng.fill_bytes(&mut bytes);
    get_payload(module_id, ident_str!("make_or_change").to_owned(), vec![
        bcs::to_bytes(&id).unwrap(),
        bcs::to_bytes(&name).unwrap(),
        bcs::to_bytes(&bytes).unwrap(),
    ])
}

fn get_payload_void(module_id: ModuleId, func: Identifier) -> TransactionPayload {
    get_payload(module_id, func, vec![])
}

fn get_payload(module_id: ModuleId, func: Identifier, args: Vec<Vec<u8>>) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(module_id, func, vec![], args))
}
