// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{format_err, Result};
use clap::Parser;
use move_binary_format::{
    access::ModuleAccess,
    file_format::{Bytecode, CompiledModule},
};
use move_core_types::account_address::AccountAddress;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{collections::HashMap, path::PathBuf, sync::Mutex};
use tokio::fs;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to the module directory.
    #[clap(long, value_parser)]
    path: String,

    /// Output directory for CSV files.
    #[clap(long, value_parser)]
    output_dir: String,
}

fn get_opcode_name(instr: &Bytecode) -> &'static str {
    match instr {
        Bytecode::Pop => "pop",
        Bytecode::Ret => "ret",
        Bytecode::BrTrue(_) => "br_true",
        Bytecode::BrFalse(_) => "br_false",
        Bytecode::Branch(_) => "branch",
        Bytecode::LdU8(_) => "ld_u8",
        Bytecode::LdU64(_) => "ld_u64",
        Bytecode::LdU128(_) => "ld_u128",
        Bytecode::CastU8 => "cast_u8",
        Bytecode::CastU64 => "cast_u64",
        Bytecode::CastU128 => "cast_u128",
        Bytecode::LdConst(_) => "ld_const",
        Bytecode::LdTrue => "ld_true",
        Bytecode::LdFalse => "ld_false",
        Bytecode::CopyLoc(_) => "copy_loc",
        Bytecode::MoveLoc(_) => "move_loc",
        Bytecode::StLoc(_) => "st_loc",
        Bytecode::Call(_) => "call",
        Bytecode::CallGeneric(_) => "call_generic",
        Bytecode::Pack(_) => "pack",
        Bytecode::PackGeneric(_) => "pack_generic",
        Bytecode::Unpack(_) => "unpack",
        Bytecode::UnpackGeneric(_) => "unpack_generic",
        Bytecode::ReadRef => "read_ref",
        Bytecode::WriteRef => "write_ref",
        Bytecode::FreezeRef => "freeze_ref",
        Bytecode::MutBorrowLoc(_) => "borrow_loc_mut",
        Bytecode::ImmBorrowLoc(_) => "borrow_loc",
        Bytecode::MutBorrowField(_) => "borrow_field_mut",
        Bytecode::MutBorrowFieldGeneric(_) => "borrow_field_generic_mut",
        Bytecode::ImmBorrowField(_) => "borrow_field",
        Bytecode::ImmBorrowFieldGeneric(_) => "borrow_field_generic",
        Bytecode::MutBorrowGlobal(_) => "borrow_global_mut",
        Bytecode::MutBorrowGlobalGeneric(_) => "borrow_global_generic_mut",
        Bytecode::ImmBorrowGlobal(_) => "borrow_global",
        Bytecode::ImmBorrowGlobalGeneric(_) => "borrow_global_generic",
        Bytecode::Add => "add",
        Bytecode::Sub => "sub",
        Bytecode::Mul => "mul",
        Bytecode::Mod => "mod",
        Bytecode::Div => "div",
        Bytecode::BitOr => "bit_or",
        Bytecode::BitAnd => "bit_and",
        Bytecode::Xor => "xor",
        Bytecode::Or => "or",
        Bytecode::And => "and",
        Bytecode::Not => "not",
        Bytecode::Eq => "eq",
        Bytecode::Neq => "neq",
        Bytecode::Lt => "lt",
        Bytecode::Gt => "gt",
        Bytecode::Le => "le",
        Bytecode::Ge => "ge",
        Bytecode::Abort => "abort",
        Bytecode::Nop => "nop",
        Bytecode::Exists(_) => "exists",
        Bytecode::ExistsGeneric(_) => "exists_generic",
        Bytecode::MoveFrom(_) => "move_from",
        Bytecode::MoveFromGeneric(_) => "move_from_generic",
        Bytecode::MoveTo(_) => "move_to",
        Bytecode::MoveToGeneric(_) => "move_to_generic",
        Bytecode::Shl => "shl",
        Bytecode::Shr => "shr",
        Bytecode::VecPack(_, _) => "vec_pack",
        Bytecode::VecLen(_) => "vec_len",
        Bytecode::VecImmBorrow(_) => "vec_borrow",
        Bytecode::VecMutBorrow(_) => "vec_borrow_mut",
        Bytecode::VecPushBack(_) => "vec_push_back",
        Bytecode::VecPopBack(_) => "vec_pop_back",
        Bytecode::VecUnpack(_, _) => "vec_unpack",
        Bytecode::VecSwap(_) => "vec_swap",
        Bytecode::LdU16(_) => "ld_u16",
        Bytecode::LdU32(_) => "ld_u32",
        Bytecode::LdU256(_) => "ld_u256",
        Bytecode::CastU16 => "cast_u16",
        Bytecode::CastU32 => "cast_u32",
        Bytecode::CastU256 => "cast_u256",
        Bytecode::PackVariant(_) => "pack_variant",
        Bytecode::PackVariantGeneric(_) => "pack_variant_generic",
        Bytecode::UnpackVariant(_) => "unpack_variant",
        Bytecode::UnpackVariantGeneric(_) => "unpack_variant_generic",
        Bytecode::TestVariant(_) => "test_variant",
        Bytecode::TestVariantGeneric(_) => "test_variant_generic",
        Bytecode::MutBorrowVariantField(_) => "borrow_variant_field_mut",
        Bytecode::MutBorrowVariantFieldGeneric(_) => "borrow_variant_field_generic_mut",
        Bytecode::ImmBorrowVariantField(_) => "borrow_variant_field",
        Bytecode::ImmBorrowVariantFieldGeneric(_) => "borrow_variant_field_generic",
        Bytecode::PackClosure(_, _) => "pack_closure",
        Bytecode::PackClosureGeneric(_, _) => "pack_closure_generic",
        Bytecode::CallClosure(_) => "call_closure",
        Bytecode::LdI8(_) => "ld_i8",
        Bytecode::LdI16(_) => "ld_i16",
        Bytecode::LdI32(_) => "ld_i32",
        Bytecode::LdI64(_) => "ld_i64",
        Bytecode::LdI128(_) => "ld_i128",
        Bytecode::LdI256(_) => "ld_i256",
        Bytecode::CastI8 => "cast_i8",
        Bytecode::CastI16 => "cast_i16",
        Bytecode::CastI32 => "cast_i32",
        Bytecode::CastI64 => "cast_i64",
        Bytecode::CastI128 => "cast_i128",
        Bytecode::CastI256 => "cast_i256",
        Bytecode::Negate => "negate",
        Bytecode::AbortMsg => "abort_msg",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
enum InstructionGroup {
    Vector,
    ALU,
    Branches,
    Calls,
    Structs,
    Enums,
    Global,
    Locals,
    Constants,
    Other,
}

impl InstructionGroup {
    fn to_string(&self) -> &'static str {
        match self {
            Self::Vector => "vector",
            Self::ALU => "ALU",
            Self::Branches => "branches",
            Self::Calls => "calls",
            Self::Structs => "structs",
            Self::Enums => "enums",
            Self::Global => "global",
            Self::Locals => "locals",
            Self::Constants => "constants",
            Self::Other => "other",
        }
    }
}

fn get_instruction_group(instr: &Bytecode) -> InstructionGroup {
    match instr {
        Bytecode::VecPack(_, _)
        | Bytecode::VecLen(_)
        | Bytecode::VecImmBorrow(_)
        | Bytecode::VecMutBorrow(_)
        | Bytecode::VecPushBack(_)
        | Bytecode::VecPopBack(_)
        | Bytecode::VecUnpack(_, _)
        | Bytecode::VecSwap(_) => InstructionGroup::Vector,

        Bytecode::Add
        | Bytecode::Sub
        | Bytecode::Mul
        | Bytecode::Mod
        | Bytecode::Div
        | Bytecode::BitOr
        | Bytecode::BitAnd
        | Bytecode::Xor
        | Bytecode::Or
        | Bytecode::And
        | Bytecode::Not
        | Bytecode::Lt
        | Bytecode::Gt
        | Bytecode::Le
        | Bytecode::Ge
        | Bytecode::Shl
        | Bytecode::Shr
        | Bytecode::CastU8
        | Bytecode::CastU64
        | Bytecode::CastU128
        | Bytecode::CastU16
        | Bytecode::CastU32
        | Bytecode::CastU256
        | Bytecode::CastI8
        | Bytecode::CastI16
        | Bytecode::CastI32
        | Bytecode::CastI64
        | Bytecode::CastI128
        | Bytecode::CastI256
        | Bytecode::Negate => InstructionGroup::ALU,

        Bytecode::Call(_) | Bytecode::CallGeneric(_) | Bytecode::CallClosure(_) => {
            InstructionGroup::Calls
        },

        Bytecode::Pack(_)
        | Bytecode::PackGeneric(_)
        | Bytecode::Unpack(_)
        | Bytecode::UnpackGeneric(_)
        | Bytecode::MutBorrowField(_)
        | Bytecode::MutBorrowFieldGeneric(_)
        | Bytecode::ImmBorrowField(_)
        | Bytecode::ImmBorrowFieldGeneric(_) => InstructionGroup::Structs,

        Bytecode::PackVariant(_)
        | Bytecode::PackVariantGeneric(_)
        | Bytecode::UnpackVariant(_)
        | Bytecode::UnpackVariantGeneric(_)
        | Bytecode::TestVariant(_)
        | Bytecode::TestVariantGeneric(_)
        | Bytecode::MutBorrowVariantField(_)
        | Bytecode::MutBorrowVariantFieldGeneric(_)
        | Bytecode::ImmBorrowVariantField(_)
        | Bytecode::ImmBorrowVariantFieldGeneric(_) => InstructionGroup::Enums,

        Bytecode::MutBorrowGlobal(_)
        | Bytecode::MutBorrowGlobalGeneric(_)
        | Bytecode::ImmBorrowGlobal(_)
        | Bytecode::ImmBorrowGlobalGeneric(_)
        | Bytecode::Exists(_)
        | Bytecode::ExistsGeneric(_)
        | Bytecode::MoveFrom(_)
        | Bytecode::MoveFromGeneric(_)
        | Bytecode::MoveTo(_)
        | Bytecode::MoveToGeneric(_) => InstructionGroup::Global,

        Bytecode::CopyLoc(_)
        | Bytecode::MoveLoc(_)
        | Bytecode::StLoc(_)
        | Bytecode::MutBorrowLoc(_)
        | Bytecode::ImmBorrowLoc(_) => InstructionGroup::Locals,

        Bytecode::LdU8(_)
        | Bytecode::LdU64(_)
        | Bytecode::LdU128(_)
        | Bytecode::LdConst(_)
        | Bytecode::LdTrue
        | Bytecode::LdFalse
        | Bytecode::LdU16(_)
        | Bytecode::LdU32(_)
        | Bytecode::LdU256(_)
        | Bytecode::LdI8(_)
        | Bytecode::LdI16(_)
        | Bytecode::LdI32(_)
        | Bytecode::LdI64(_)
        | Bytecode::LdI128(_)
        | Bytecode::LdI256(_) => InstructionGroup::Constants,

        Bytecode::BrTrue(_) | Bytecode::BrFalse(_) | Bytecode::Branch(_) => {
            InstructionGroup::Branches
        },

        Bytecode::Pop
        | Bytecode::Ret
        | Bytecode::Abort
        | Bytecode::Nop
        | Bytecode::AbortMsg
        | Bytecode::Eq
        | Bytecode::Neq
        | Bytecode::ReadRef
        | Bytecode::WriteRef
        | Bytecode::FreezeRef
        | Bytecode::PackClosure(_, _)
        | Bytecode::PackClosureGeneric(_, _) => InstructionGroup::Other,
    }
}

async fn list_files_with_extension(
    dir: &str,
    extension: &str,
) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut paths = vec![];
    let mut stack = vec![PathBuf::from(dir)];

    while let Some(curr_dir) = stack.pop() {
        let mut entries = fs::read_dir(curr_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == extension) {
                paths.push(path);
            } else if path.is_dir() {
                stack.push(path);
            }
        }
    }

    Ok(paths)
}

async fn read_modules(dir: &str) -> Result<Vec<Vec<u8>>> {
    let paths = list_files_with_extension(dir, "mv").await?;

    let reads = paths
        .into_iter()
        .map(|path| async move { fs::read(path).await });

    futures::future::join_all(reads)
        .await
        .into_iter()
        .map(|res| res.map_err(|err| format_err!("Failed to read file: {}", err)))
        .collect()
}

#[derive(Debug, Default, Clone)]
struct InstructionDistribution {
    counts: HashMap<&'static str, u64>,
}

impl InstructionDistribution {
    fn merge(&mut self, other: &InstructionDistribution) {
        for (k, v) in &other.counts {
            *self.counts.entry(k).or_insert(0) += v;
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct FunctionInfo {
    module_id: String,
    func_name: String,
    num_instructions: usize,
    instruction_distribution: InstructionDistribution,
}

#[derive(Debug)]
struct GlobalStats {
    // Counters across all modules.
    total_functions: usize,
    total_instructions: usize,
    total_instruction_distribution: InstructionDistribution,

    // Instruction ditribution per module.
    module_instruction_distribution: HashMap<String, InstructionDistribution>,

    // Distribution for all functions.
    function_distribution: HashMap<String, InstructionDistribution>,

    // Function-related statistics.
    num_generic_funcs: u64,
    num_non_generic_funcs: u64,

    num_non_generic_calls: u64,
    num_generic_calls: u64,
    num_local_calls: u64,
    num_framework_calls: u64,
    num_cross_module_calls: u64,

    // Call target counts (for Call bytecode).
    call_targets: HashMap<String, u64>,
    // Call target counts (for CallGeneric bytecode).
    call_generic_targets: HashMap<String, u64>,

    // Instruction group distribution.
    group_distribution: HashMap<InstructionGroup, u64>,

    // Distribution for non-special (user) modules.
    user_instruction_distribution: InstructionDistribution,
}

impl GlobalStats {
    fn new() -> Self {
        Self {
            total_functions: 0,
            total_instructions: 0,
            total_instruction_distribution: InstructionDistribution::default(),
            module_instruction_distribution: HashMap::new(),
            function_distribution: HashMap::new(),
            // Distribution for non-special (user) modules.
            user_instruction_distribution: InstructionDistribution::default(),
            num_generic_funcs: 0,
            num_non_generic_funcs: 0,
            num_non_generic_calls: 0,
            num_generic_calls: 0,
            num_local_calls: 0,
            num_framework_calls: 0,
            num_cross_module_calls: 0,
            call_targets: HashMap::new(),
            call_generic_targets: HashMap::new(),
            group_distribution: HashMap::new(),
        }
    }

    fn merge(&mut self, other: GlobalStats) {
        self.total_functions += other.total_functions;
        self.total_instructions += other.total_instructions;
        self.total_instruction_distribution
            .merge(&other.total_instruction_distribution);

        self.user_instruction_distribution
            .merge(&other.user_instruction_distribution);

        for (group, count) in other.group_distribution {
            *self.group_distribution.entry(group).or_insert(0) += count;
        }

        for (module_name, stats) in other.module_instruction_distribution {
            self.module_instruction_distribution
                .entry(module_name)
                .or_default()
                .merge(&stats);
        }

        for (func_name, stats) in other.function_distribution {
            self.function_distribution
                .entry(func_name)
                .or_default()
                .merge(&stats);
        }

        self.num_generic_funcs += other.num_generic_funcs;
        self.num_non_generic_funcs += other.num_non_generic_funcs;
        self.num_non_generic_calls += other.num_non_generic_calls;
        self.num_generic_calls += other.num_generic_calls;
        self.num_local_calls += other.num_local_calls;
        self.num_framework_calls += other.num_framework_calls;
        self.num_cross_module_calls += other.num_cross_module_calls;

        for (target, count) in other.call_targets {
            *self.call_targets.entry(target).or_insert(0) += count;
        }
        for (target, count) in other.call_generic_targets {
            *self.call_generic_targets.entry(target).or_insert(0) += count;
        }
    }
}

fn process_modules(modules: &Vec<Vec<u8>>) -> Result<GlobalStats> {
    let stats = Mutex::new(GlobalStats::new());

    modules.par_iter().for_each(|bytes| {
        if let Ok(module) = CompiledModule::deserialize(bytes) {
            let module_id = module.self_id().short_str_lossless();
            let is_user_module = !module.self_id().address().is_special();
            let mut global_stats = GlobalStats::new();

            for func_def in module.function_defs() {
                if let Some(code_unit) = &func_def.code {
                    let func_handle = module.function_handle_at(func_def.function);
                    let full_func_name = format!(
                        "{}::{}",
                        module_id,
                        module.identifier_at(func_handle.name).to_string()
                    );

                    if func_handle.type_parameters.is_empty() {
                        global_stats.num_non_generic_funcs += 1;
                    } else {
                        global_stats.num_generic_funcs += 1;
                    }

                    let mut instruction_distribution = InstructionDistribution::default();
                    for instr in &code_unit.code {
                        let name = get_opcode_name(instr);
                        *instruction_distribution.counts.entry(name).or_insert(0) += 1;

                        let group = get_instruction_group(instr);
                        *global_stats.group_distribution.entry(group).or_insert(0) += 1;

                        let (idx, is_generic) = match instr {
                            Bytecode::Call(idx) => {
                                global_stats.num_non_generic_calls += 1;
                                (idx, false)
                            },
                            Bytecode::CallGeneric(idx) => {
                                global_stats.num_generic_calls += 1;
                                let func_inst = module.function_instantiation_at(*idx);
                                (&func_inst.handle, true)
                            },
                            _ => {
                                continue;
                            },
                        };

                        let target_func = module.function_handle_at(*idx);
                        let target_module_handle = module.module_handle_at(target_func.module);
                        let target_module_addr =
                            module.address_identifier_at(target_module_handle.address);
                        let target_module_name =
                            module.identifier_at(target_module_handle.name).to_string();
                        let target_func_name = module.identifier_at(target_func.name).to_string();

                        // Build full target function name.
                        let full_target_name = format!(
                            "{}::{}::{}",
                            target_module_addr.to_hex_literal(),
                            target_module_name,
                            target_func_name
                        );

                        // Track call target.
                        if is_generic {
                            *global_stats
                                .call_generic_targets
                                .entry(full_target_name.clone())
                                .or_insert(0) += 1;
                        } else {
                            *global_stats
                                .call_targets
                                .entry(full_target_name.clone())
                                .or_insert(0) += 1;
                        }

                        if target_func.module == module.self_handle_idx() {
                            global_stats.num_local_calls += 1;
                        } else {
                            global_stats.num_cross_module_calls += 1;
                        }

                        if target_module_addr.is_special() {
                            global_stats.num_framework_calls += 1;
                        }
                    }

                    global_stats.total_functions += 1;
                    global_stats.total_instructions += code_unit.code.len();
                    global_stats
                        .total_instruction_distribution
                        .merge(&instruction_distribution);

                    if is_user_module {
                        global_stats
                            .user_instruction_distribution
                            .merge(&instruction_distribution);
                    }

                    global_stats
                        .module_instruction_distribution
                        .entry(module_id.clone())
                        .or_default()
                        .merge(&instruction_distribution);

                    global_stats
                        .function_distribution
                        .entry(full_func_name)
                        .or_default()
                        .merge(&instruction_distribution);
                }
            }

            stats.lock().unwrap().merge(global_stats);
        }
    });

    Ok(stats.into_inner().unwrap())
}

fn write_csv<T: serde::Serialize>(path: PathBuf, data: &[T]) -> Result<()> {
    let mut wtr = csv::Writer::from_path(path)?;
    for record in data {
        wtr.serialize(record)?;
    }
    wtr.flush()?;
    Ok(())
}

#[derive(serde::Serialize)]
struct InstructionCountRecord {
    name: String,
    count: u64,
    percentage: f64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let modules = read_modules(&args.path).await?;
    println!("Read {} modules.", modules.len());

    println!("Processing statistics...");
    let stats = process_modules(&modules)?;

    fs::create_dir_all(&args.output_dir).await?;
    let out_dir = PathBuf::from(&args.output_dir);

    // Distribution for all instructions.
    let total_instructions = stats.total_instructions as f64;
    let mut total_instruction_records: Vec<InstructionCountRecord> = stats
        .total_instruction_distribution
        .counts
        .iter()
        .map(|(k, v)| InstructionCountRecord {
            name: k.to_string(),
            count: *v,
            percentage: (*v as f64 / total_instructions) * 100.0,
        })
        .collect();
    total_instruction_records.sort_by(|a, b| b.count.cmp(&a.count));
    write_csv(
        out_dir.join("total_instructions.csv"),
        &total_instruction_records,
    )?;

    println!("\nInstruction distribution for all modules:");
    for (i, op) in total_instruction_records.iter().enumerate() {
        println!(
            "{:>4}) {:<35} {:>6} ({:.2}%)",
            i + 1,
            op.name,
            op.count,
            op.percentage,
        );
    }

    println!("\nInstruction group distribution:");
    let mut group_records: Vec<_> = stats
        .group_distribution
        .iter()
        .map(|(group, count)| {
            (
                group.to_string(),
                *count,
                (*count as f64 / total_instructions) * 100.0,
            )
        })
        .collect();

    group_records.sort_by(|a, b| b.1.cmp(&a.1));

    for (name, count, percentage) in group_records {
        println!("  {:<20} {:>8} ({:.2}%)", name, count, percentage);
    }

    // Function type distribution.
    let total_funcs = (stats.num_generic_funcs + stats.num_non_generic_funcs) as f64;
    let generic_funcs_percentage = (stats.num_generic_funcs as f64 / total_funcs) * 100.0;
    let non_generic_funcs_percentage = (stats.num_non_generic_funcs as f64 / total_funcs) * 100.0;

    println!("\nFunction definition distribution:");
    println!(
        "  {:<20} {:>8} ({:.2}%)",
        "non-generic", stats.num_non_generic_funcs, non_generic_funcs_percentage
    );
    println!(
        "  {:<20} {:>8} ({:.2}%)",
        "generic", stats.num_generic_funcs, generic_funcs_percentage
    );

    // Function call distribution.
    let total_calls = (stats.num_non_generic_calls + stats.num_generic_calls) as f64;
    let non_generic_calls_percentage = (stats.num_non_generic_calls as f64 / total_calls) * 100.0;
    let generic_calls_percentage = (stats.num_generic_calls as f64 / total_calls) * 100.0;
    let local_calls_percentage = (stats.num_local_calls as f64 / total_calls) * 100.0;
    let framework_calls_percentage = (stats.num_framework_calls as f64 / total_calls) * 100.0;
    let cross_module_calls_percentage = (stats.num_cross_module_calls as f64 / total_calls) * 100.0;

    println!("\nFunction call distribution (by type):");
    println!("  {:<20} {:>8} ({:.2}%)", "total", total_calls, 100.0);
    println!(
        "  {:<20} {:>8} ({:.2}%)",
        "non-generic", stats.num_non_generic_calls, non_generic_calls_percentage
    );
    println!(
        "  {:<20} {:>8} ({:.2}%)",
        "generic", stats.num_generic_calls, generic_calls_percentage
    );

    println!("\nFunction call distribution (by target):");
    println!(
        "  {:<20} {:>8} ({:.2}%)",
        "local", stats.num_local_calls, local_calls_percentage
    );
    println!(
        "  {:<20} {:>8} ({:.2}%)",
        "cross-module", stats.num_cross_module_calls, cross_module_calls_percentage
    );
    println!(
        "  {:<20} {:>8} ({:.2}%)",
        "framework", stats.num_framework_calls, framework_calls_percentage
    );

    let mut call_targets: Vec<_> = stats.call_targets.iter().collect();
    call_targets.sort_by(|a, b| b.1.cmp(a.1));

    println!("\nTop-10 call targets (non-generic):");
    for (i, (target, count)) in call_targets.iter().take(10).enumerate() {
        let percentage = (**count as f64 / stats.num_non_generic_calls as f64) * 100.0;
        println!(
            "{:>4}) {:<60} {:>8} ({:.2}%)",
            i + 1,
            target,
            count,
            percentage
        );
    }

    let mut call_generic_targets: Vec<_> = stats.call_generic_targets.iter().collect();
    call_generic_targets.sort_by(|a, b| b.1.cmp(a.1));

    println!("\nTop-10 call targets (generic):");
    for (i, (target, count)) in call_generic_targets.iter().take(10).enumerate() {
        let percentage = (**count as f64 / stats.num_generic_calls as f64) * 100.0;
        println!(
            "{:>4}) {:<60} {:>8} ({:.2}%)",
            i + 1,
            target,
            count,
            percentage
        );
    }

    let is_user_target = |target: &str| -> bool {
        if let Some(addr_str) = target.split("::").next() {
            if let Ok(addr) = AccountAddress::from_hex_literal(addr_str) {
                return !addr.is_special();
            }
        }
        false
    };

    println!("\nTop-20 call user targets (non-generic):");
    let user_call_targets: Vec<_> = call_targets
        .iter()
        .filter(|(target, _)| is_user_target(target))
        .take(20)
        .collect();

    for (i, (target, count)) in user_call_targets.iter().enumerate() {
        let percentage = (**count as f64 / stats.num_non_generic_calls as f64) * 100.0;
        println!(
            "{:>4}) {:<110} {:>8} ({:.2}%)",
            i + 1,
            target,
            count,
            percentage
        );
    }

    println!("\nTop-20 call user targets (generic):");
    let user_call_generic_targets: Vec<_> = call_generic_targets
        .iter()
        .filter(|(target, _)| is_user_target(target))
        .take(20)
        .collect();

    for (i, (target, count)) in user_call_generic_targets.iter().enumerate() {
        let percentage = (**count as f64 / stats.num_generic_calls as f64) * 100.0;
        println!(
            "{:>4}) {:<110} {:>8} ({:.2}%)",
            i + 1,
            target,
            count,
            percentage
        );
    }

    Ok(())
}
