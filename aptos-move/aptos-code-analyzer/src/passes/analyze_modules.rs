// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ModulePass;
use move_binary_format::{
    access::ModuleAccess,
    file_format::{Bytecode, Visibility},
    CompiledModule,
};
use move_core_types::account_address::AccountAddress;
use std::{
    collections::{
        hash_map::Entry::{Occupied, Vacant},
        HashMap,
    },
    fmt::{Display, Formatter},
    fs::File,
    io::Write,
    path::PathBuf,
};

// Bytecode only implements Debug and dumps extra information, but here we only
// need opcodes.
fn bytecode_to_text(instruction: &Bytecode) -> &str {
    match instruction {
        Bytecode::Pop => "Pop",
        Bytecode::Ret => "Ret",
        Bytecode::BrTrue(_) => "BrTrue",
        Bytecode::BrFalse(_) => "BrFalse",
        Bytecode::Branch(_) => "Branch",
        Bytecode::LdU8(_) => "LdU8",
        Bytecode::LdU16(_) => "LdU16",
        Bytecode::LdU32(_) => "LdU32",
        Bytecode::LdU64(_) => "LdU64",
        Bytecode::LdU128(_) => "LdU128",
        Bytecode::LdU256(_) => "LdU256",
        Bytecode::CastU8 => "CastU8",
        Bytecode::CastU16 => "CastU16",
        Bytecode::CastU32 => "CastU32",
        Bytecode::CastU64 => "CastU64",
        Bytecode::CastU128 => "CastU128",
        Bytecode::CastU256 => "CastU256",
        Bytecode::LdConst(_) => "LdConst",
        Bytecode::LdTrue => "LdTrue",
        Bytecode::LdFalse => "LdFalse",
        Bytecode::CopyLoc(_) => "CopyLoc",
        Bytecode::MoveLoc(_) => "MoveLoc",
        Bytecode::StLoc(_) => "StLoc",
        Bytecode::Call(_) => "Call",
        Bytecode::CallGeneric(_) => "CallGeneric",
        Bytecode::Pack(_) => "Pack",
        Bytecode::PackGeneric(_) => "PackGeneric",
        Bytecode::Unpack(_) => "Unpack",
        Bytecode::UnpackGeneric(_) => "UnpackGeneric",
        Bytecode::ReadRef => "ReadRef",
        Bytecode::WriteRef => "WriteRef",
        Bytecode::FreezeRef => "FreezeRef",
        Bytecode::MutBorrowLoc(_) => "MutBorrowLoc",
        Bytecode::ImmBorrowLoc(_) => "ImmBorrowLoc",
        Bytecode::MutBorrowField(_) => "MutBorrowField",
        Bytecode::MutBorrowFieldGeneric(_) => "MutBorrowFieldGeneric",
        Bytecode::ImmBorrowField(_) => "ImmBorrowField",
        Bytecode::ImmBorrowFieldGeneric(_) => "ImmBorrowFieldGeneric",
        Bytecode::MutBorrowGlobal(_) => "MutBorrowGlobal",
        Bytecode::MutBorrowGlobalGeneric(_) => "MutBorrowGlobalGeneric",
        Bytecode::ImmBorrowGlobal(_) => "ImmBorrowGlobal",
        Bytecode::ImmBorrowGlobalGeneric(_) => "ImmBorrowGlobalGeneric",
        Bytecode::Add => "Add",
        Bytecode::Sub => "Sub",
        Bytecode::Mul => "Mul",
        Bytecode::Mod => "Mod",
        Bytecode::Div => "Div",
        Bytecode::BitOr => "BitOr",
        Bytecode::BitAnd => "BitAnd",
        Bytecode::Xor => "Xor",
        Bytecode::Shl => "Shl",
        Bytecode::Shr => "Shr",
        Bytecode::Or => "Or",
        Bytecode::And => "And",
        Bytecode::Not => "Not",
        Bytecode::Eq => "Eq",
        Bytecode::Neq => "Neq",
        Bytecode::Lt => "Lt",
        Bytecode::Gt => "Gt",
        Bytecode::Le => "Le",
        Bytecode::Ge => "Ge",
        Bytecode::Abort => "Abort",
        Bytecode::Nop => "Nop",
        Bytecode::Exists(_) => "Exists",
        Bytecode::ExistsGeneric(_) => "ExistsGeneric",
        Bytecode::MoveFrom(_) => "MoveFrom",
        Bytecode::MoveFromGeneric(_) => "MoveFromGeneric",
        Bytecode::MoveTo(_) => "MoveTo",
        Bytecode::MoveToGeneric(_) => "MoveToGeneric",
        Bytecode::VecPack(_, _) => "VecPack",
        Bytecode::VecLen(_) => "VecLen",
        Bytecode::VecImmBorrow(_) => "VecImmBorrow",
        Bytecode::VecMutBorrow(_) => "VecMutBorrow",
        Bytecode::VecPushBack(_) => "VecPushBack",
        Bytecode::VecPopBack(_) => "VecPopBack",
        Bytecode::VecUnpack(_, _) => "VecUnpack",
        Bytecode::VecSwap(_) => "VecSwap",
    }
}

pub struct InstructionCount {
    address: AccountAddress,
    module_name: String,
    function_name: String,
    count: usize,
}

pub struct StructAnalysis {
    address: AccountAddress,
    module_name: String,
    name: String,
    num_fields: usize,
    num_ty_args: usize,
    has_key: bool,
    has_store: bool,
    has_drop: bool,
    has_copy: bool,
}

impl Display for StructAnalysis {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}, {}, {}, {}, {}, {}, {}, {}, {}",
            self.address,
            self.module_name,
            self.name,
            self.num_fields,
            self.num_ty_args,
            self.has_key,
            self.has_store,
            self.has_drop,
            self.has_copy
        )
    }
}

pub struct FunctionAnalysis {
    address: AccountAddress,
    module_name: String,
    name: String,
    visibility: Visibility,
    is_entry: bool,
    is_native: bool,
    num_instructions: usize,
    num_locals: usize,
    num_args: usize,
    num_ty_args: usize,
}

impl Display for FunctionAnalysis {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}, {}, {}, {}, {}, {}, {}, {}, {}, {}",
            self.address,
            self.module_name,
            self.name,
            self.visibility as u8,
            self.is_entry,
            self.is_native,
            self.num_instructions,
            self.num_locals,
            self.num_args,
            self.num_ty_args
        )
    }
}

pub struct ModuleAnalysis {
    output_csv_dir: PathBuf,
    function_analysis: HashMap<String, FunctionAnalysis>,
    struct_analysis: HashMap<String, StructAnalysis>,
    instruction_analysis: HashMap<String, HashMap<Bytecode, InstructionCount>>,
}

impl ModuleAnalysis {
    pub fn new(output_csv_dir: PathBuf) -> Self {
        Self {
            output_csv_dir,
            function_analysis: HashMap::new(),
            struct_analysis: HashMap::new(),
            instruction_analysis: HashMap::new(),
        }
    }

    pub fn finish(self) {
        let Self {
            output_csv_dir,
            function_analysis,
            struct_analysis,
            instruction_analysis,
        } = self;

        let path = output_csv_dir.as_path().join("struct-analysis.csv");
        let mut output = File::create(path.as_path())
            .expect("Should be able to create a file for CSV struct analysis output");
        writeln!(output, "address, module, name, num_fields, num_ty_args, has_key, has_store, has_drop, has_copy").unwrap();
        for s in struct_analysis.values() {
            writeln!(output, "{}", s).unwrap();
        }

        let path = output_csv_dir.as_path().join("function-analysis.csv");
        let mut output = File::create(path.as_path())
            .expect("Should be able to create a file for CSV function analysis output");
        writeln!(output, "address, module_name, name, visibility, is_entry, is_native, num_instructions, num_locals, num_args, num_ty_args").unwrap();
        for f in function_analysis.values() {
            writeln!(output, "{}", f).unwrap();
        }

        let path = output_csv_dir.as_path().join("instruction-analysis.csv");
        let mut output = File::create(path.as_path())
            .expect("Should be able to create a file for CSV instruction analysis output");
        writeln!(
            output,
            "address, module_name, function_name, instruction, count"
        )
        .unwrap();
        for counts in instruction_analysis.values() {
            for (instruction, count) in counts {
                writeln!(
                    output,
                    "{}, {}, {}, {}, {}",
                    count.address,
                    count.module_name,
                    count.function_name,
                    bytecode_to_text(instruction),
                    count.count
                )
                .unwrap();
            }
        }
    }
}

impl ModulePass for ModuleAnalysis {
    fn run_on_module(&mut self, module: &CompiledModule) {
        let address = module.self_id().address;
        let module_name = module.self_id().name.to_string();

        for struct_def in module.struct_defs() {
            let num_fields = struct_def.declared_field_count().unwrap() as usize;

            let struct_handle = module.struct_handle_at(struct_def.struct_handle);
            let name = module
                .identifier_at(struct_handle.name)
                .as_str()
                .to_string();
            let num_ty_args = struct_handle.type_parameters.len();
            let abilities = struct_handle.abilities;

            let key = format!("{}-{}-{}", address, module_name, name);
            assert!(self
                .struct_analysis
                .insert(key, StructAnalysis {
                    address,
                    module_name: module_name.clone(),
                    name,
                    num_fields,
                    num_ty_args,
                    has_key: abilities.has_key(),
                    has_store: abilities.has_store(),
                    has_drop: abilities.has_drop(),
                    has_copy: abilities.has_copy(),
                })
                .is_none());
        }

        for function_def in module.function_defs() {
            let handle = module.function_handle_at(function_def.function);
            let name = module.identifier_at(handle.name).to_owned().to_string();

            let is_entry = function_def.is_entry;
            let is_native = function_def.is_native();

            let num_instructions = function_def
                .code
                .as_ref()
                .map(|cu| cu.code.len())
                .unwrap_or(0);
            let num_locals = function_def
                .code
                .as_ref()
                .map(|cu| module.signature_at(cu.locals).len())
                .unwrap_or(0);
            let num_args = module.signature_at(handle.parameters).len();
            let num_ty_args = handle.type_parameters.len();

            let key = format!("{}-{}-{}", address, module_name, name);
            assert!(self
                .function_analysis
                .insert(key.clone(), FunctionAnalysis {
                    address,
                    module_name: module_name.clone(),
                    name: name.clone(),
                    visibility: function_def.visibility,
                    is_entry,
                    is_native,
                    num_instructions,
                    num_locals,
                    num_args,
                    num_ty_args,
                })
                .is_none());

            if let Some(cu) = &function_def.code {
                let num_instructions = cu.code.len();
                let mut instruction_count = HashMap::new();
                for i in 0..num_instructions {
                    let instruction = cu.code[i].clone();
                    match instruction_count.entry(instruction) {
                        Vacant(e) => {
                            e.insert(InstructionCount {
                                address,
                                module_name: module_name.clone(),
                                function_name: name.clone(),
                                count: 1,
                            });
                        },
                        Occupied(mut e) => {
                            e.get_mut().count += 1;
                        },
                    }
                }
                self.instruction_analysis
                    .insert(key.clone(), instruction_count);
            }
        }
    }
}
