// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ModulePass;
use aptos_types::{
    access_path::AccessPath,
    on_chain_config::{GasSchedule, GasScheduleV2, OnChainConfig},
    state_store::{
        state_key::{StateKey, StateKeyInner},
        StateView,
    },
};
use clap::ValueEnum;
use move_binary_format::{
    access::ModuleAccess,
    control_flow_graph::{ControlFlowGraph, VMControlFlowGraph},
    file_format::{Bytecode, FunctionHandle},
    CompiledModule,
};
use move_core_types::language_storage::ModuleId;
use std::{
    collections::{BTreeMap, HashMap},
    fmt::{Display, Formatter},
    fs::File,
    io::Write,
    path::PathBuf,
};

struct InstructionCostCalculator<'s, S> {
    state_view: &'s S,
    costs: BTreeMap<String, u64>,
}

impl<'s, S: StateView> InstructionCostCalculator<'s, S> {
    fn new(state_view: &'s S) -> Self {
        // This code mimics the VM behavior to initialize gas parameters.
        let costs = match GasScheduleV2::fetch_config(state_view) {
            Some(gas_schedule) => gas_schedule.to_btree_map(),
            None => GasSchedule::fetch_config(state_view)
                .expect("At least one version of gas schedule must exist")
                .to_btree_map(),
        };
        Self { state_view, costs }
    }

    fn lookup_cost(&self, param: &str) -> u64 {
        *self.costs.get(param).unwrap_or_else(|| {
            panic!(
                "Cost should exist for every parameter, but '{}' is not found",
                param
            )
        })
    }

    /// Calculates a static cost of a bytecode instruction, i.e. cost which does not
    /// depend on any runtime parameters and solely on the gas schedule and the bytecode
    /// instruction itself. The cost is either complete, or only partial, in case the
    /// instruction also has a dynamic component in its cost.
    fn instruction_static_cost(&self, module: &CompiledModule, instruction: &Bytecode) -> u64 {
        match instruction {
            // Simple instructions with a fixed, statically-known gas cost.
            Bytecode::BrFalse(_) => self.lookup_cost("instr.br_false"),
            Bytecode::BrTrue(_) => self.lookup_cost("instr.br_true"),
            Bytecode::Branch(_) => self.lookup_cost("instr.branch"),
            Bytecode::Nop => self.lookup_cost("instr.nop"),
            Bytecode::Abort => self.lookup_cost("instr.abort"),
            Bytecode::Ret => self.lookup_cost("instr.ret"),
            Bytecode::LdU8(_) => self.lookup_cost("instr.ld_u8"),
            Bytecode::LdU16(_) => self.lookup_cost("instr.ld_u16"),
            Bytecode::LdU32(_) => self.lookup_cost("instr.ld_u32"),
            Bytecode::LdU64(_) => self.lookup_cost("instr.ld_u64"),
            Bytecode::LdU128(_) => self.lookup_cost("instr.ld_u128"),
            Bytecode::LdU256(_) => self.lookup_cost("instr.ld_u256"),
            Bytecode::LdTrue => self.lookup_cost("instr.ld_true"),
            Bytecode::LdFalse => self.lookup_cost("instr.ld_false"),
            Bytecode::ImmBorrowLoc(_) => self.lookup_cost("instr.imm_borrow_loc"),
            Bytecode::MutBorrowLoc(_) => self.lookup_cost("instr.mut_borrow_loc"),
            Bytecode::ImmBorrowField(_) => self.lookup_cost("instr.imm_borrow_field"),
            Bytecode::MutBorrowField(_) => self.lookup_cost("instr.mut_borrow_field"),
            Bytecode::ImmBorrowFieldGeneric(_) => {
                self.lookup_cost("instr.imm_borrow_field_generic")
            },
            Bytecode::MutBorrowFieldGeneric(_) => {
                self.lookup_cost("instr.mut_borrow_field_generic")
            },
            Bytecode::FreezeRef => self.lookup_cost("instr.freeze_ref"),
            Bytecode::CastU8 => self.lookup_cost("instr.cast_u8"),
            Bytecode::CastU16 => self.lookup_cost("instr.cast_u16"),
            Bytecode::CastU32 => self.lookup_cost("instr.cast_u32"),
            Bytecode::CastU64 => self.lookup_cost("instr.cast_u64"),
            Bytecode::CastU128 => self.lookup_cost("instr.cast_u128"),
            Bytecode::CastU256 => self.lookup_cost("instr.cast_u256"),
            Bytecode::Add => self.lookup_cost("instr.add"),
            Bytecode::Sub => self.lookup_cost("instr.sub"),
            Bytecode::Mul => self.lookup_cost("instr.mul"),
            Bytecode::Mod => self.lookup_cost("instr.mod"),
            Bytecode::Div => self.lookup_cost("instr.div"),
            Bytecode::BitOr => self.lookup_cost("instr.bit_or"),
            Bytecode::BitAnd => self.lookup_cost("instr.bit_and"),
            Bytecode::Xor => self.lookup_cost("instr.bit_xor"),
            Bytecode::Shl => self.lookup_cost("instr.bit_shl"),
            Bytecode::Shr => self.lookup_cost("instr.bit_shr"),
            Bytecode::Or => self.lookup_cost("instr.or"),
            Bytecode::And => self.lookup_cost("instr.and"),
            Bytecode::Not => self.lookup_cost("instr.not"),
            Bytecode::Lt => self.lookup_cost("instr.lt"),
            Bytecode::Gt => self.lookup_cost("instr.gt"),
            Bytecode::Le => self.lookup_cost("instr.le"),
            Bytecode::Ge => self.lookup_cost("instr.ge"),
            Bytecode::Pop => self.lookup_cost("instr.pop"),
            Bytecode::MoveLoc(_) => self.lookup_cost("instr.move_loc.base"),
            Bytecode::StLoc(_) => self.lookup_cost("instr.st_loc.base"),
            Bytecode::WriteRef => self.lookup_cost("instr.write_ref.base"),
            Bytecode::VecLen(_) => self.lookup_cost("instr.vec_len.base"),
            Bytecode::VecImmBorrow(_) => self.lookup_cost("instr.vec_imm_borrow.base"),
            Bytecode::VecMutBorrow(_) => self.lookup_cost("instr.vec_mut_borrow.base"),
            Bytecode::VecPopBack(_) => self.lookup_cost("instr.vec_pop_back.base"),
            Bytecode::VecPushBack(_) => self.lookup_cost("instr.vec_push_back.base"),
            Bytecode::VecSwap(_) => self.lookup_cost("instr.vec_swap.base"),
            Bytecode::VecPack(_, n) => {
                self.lookup_cost("instr.vec_pack.base")
                    + n * self.lookup_cost("instr.vec_pack.per_elem")
            },
            Bytecode::VecUnpack(_, n) => {
                self.lookup_cost("instr.vec_unpack.base")
                    + n * self.lookup_cost("instr.vec_unpack.per_expected_elem")
            },

            // Also statically-known costs but need to look up more information.
            Bytecode::LdConst(idx) => {
                let num_bytes = module.constant_at(*idx).data.len() as u64;
                self.lookup_cost("instr.ld_const.base")
                    + num_bytes * self.lookup_cost("instr.ld_const.per_byte")
            },
            Bytecode::Pack(idx) => {
                let struct_def = module.struct_def_at(*idx);
                let n = struct_def
                    .declared_field_count()
                    .expect("There are no native structs anymore") as u64;
                self.lookup_cost("instr.pack.base") + n * self.lookup_cost("instr.pack.per_field")
            },
            Bytecode::PackGeneric(idx) => {
                let struct_def = module.struct_def_at(module.struct_instantiation_at(*idx).def);
                let n = struct_def
                    .declared_field_count()
                    .expect("There are no native structs anymore") as u64;
                self.lookup_cost("instr.pack_generic.base")
                    + n * self.lookup_cost("instr.pack_generic.per_field")
            },
            Bytecode::Unpack(idx) => {
                let struct_def = module.struct_def_at(*idx);
                let n = struct_def
                    .declared_field_count()
                    .expect("There are no native structs anymore") as u64;
                self.lookup_cost("instr.unpack.base")
                    + n * self.lookup_cost("instr.unpack.per_field")
            },
            Bytecode::UnpackGeneric(idx) => {
                let struct_def = module.struct_def_at(module.struct_instantiation_at(*idx).def);
                let n = struct_def
                    .declared_field_count()
                    .expect("There are no native structs anymore") as u64;
                self.lookup_cost("instr.unpack_generic.base")
                    + n * self.lookup_cost("instr.unpack_generic.per_field")
            },
            Bytecode::Call(idx) => {
                let function_handle = module.function_handle_at(*idx);
                self.calculate_call_cost("call", function_handle, module)
            },
            Bytecode::CallGeneric(idx) => {
                let function_instantiation = module.function_instantiation_at(*idx);
                let num_ty_args = module
                    .signature_at(function_instantiation.type_parameters)
                    .len() as u64;
                let cost = num_ty_args * self.lookup_cost("instr.call_generic.per_ty_arg");

                let function_handle = module.function_handle_at(function_instantiation.handle);
                cost + self.calculate_call_cost("call_generic", function_handle, module)
            },

            // Partial costs: these instructions also have a dynamic component based
            // on the runtime value information.
            Bytecode::CopyLoc(_) => self.lookup_cost("instr.copy_loc.base"),
            Bytecode::ReadRef => self.lookup_cost("instr.read_ref.base"),
            Bytecode::Eq => self.lookup_cost("instr.eq.base"),
            Bytecode::Neq => self.lookup_cost("instr.neq.base"),

            // These are also "partial costs" because there is an additional cost
            // associated with loading a resource based on the number of bytes.
            Bytecode::ImmBorrowGlobal(_) => self.lookup_cost("instr.imm_borrow_global.base"),
            Bytecode::ImmBorrowGlobalGeneric(_) => {
                self.lookup_cost("instr.imm_borrow_global_generic.base")
            },
            Bytecode::MutBorrowGlobal(_) => self.lookup_cost("instr.mut_borrow_global.base"),
            Bytecode::MutBorrowGlobalGeneric(_) => {
                self.lookup_cost("instr.mut_borrow_global_generic.base")
            },
            Bytecode::Exists(_) => self.lookup_cost("instr.exists.base"),
            Bytecode::ExistsGeneric(_) => self.lookup_cost("instr.exists_generic.base"),
            Bytecode::MoveTo(_) => self.lookup_cost("instr.move_to.base"),
            Bytecode::MoveToGeneric(_) => self.lookup_cost("instr.move_to_generic.base"),
            Bytecode::MoveFrom(_) => self.lookup_cost("instr.move_from.base"),
            Bytecode::MoveFromGeneric(_) => self.lookup_cost("instr.move_from_generic.base"),
            // Not taken into account:
            //   * costs of type creation,
            //   * costs of I/O and storage,
            //   * dynamic costs for loading resources.
        }
    }

    fn calculate_call_cost(
        &self,
        param: &str,
        function_handle: &FunctionHandle,
        module: &CompiledModule,
    ) -> u64 {
        let function_name = module.identifier_at(function_handle.name);
        let module_handle = module.module_handle_at(function_handle.module);

        let address = module.address_identifier_at(module_handle.address);
        let name = module.identifier_at(module_handle.name);
        let id = ModuleId::new(*address, name.to_owned());

        let additional_call_cost = |defining_module: &CompiledModule| -> u64 {
            for function_def in defining_module.function_defs() {
                let handle = defining_module.function_handle_at(function_def.function);
                let name = defining_module.identifier_at(handle.name);
                if name == function_name {
                    let num_arguments =
                        defining_module.signature_at(handle.parameters).len() as u64;
                    let mut cost =
                        num_arguments * self.lookup_cost(&format!("instr.{param}.per_arg"));

                    if let Some(code_unit) = &function_def.code {
                        let num_locals =
                            defining_module.signature_at(code_unit.locals).len() as u64;
                        cost += num_locals * self.lookup_cost(&format!("instr.{param}.per_local"));
                    }
                    return cost;
                }
            }
            unreachable!("Function must exist in defining module")
        };

        let mut cost = self.lookup_cost(&format!("instr.{param}.base"));
        if module.self_id() == id {
            cost += additional_call_cost(module)
        } else {
            // This is a cross-module call.
            let state_key = StateKey::new(StateKeyInner::AccessPath(AccessPath::code_access_path(
                id.clone(),
            )));
            let bytes = self
                .state_view
                .get_state_value_bytes(&state_key)
                .unwrap_or_else(|_| panic!("Error when fetching bytes for module {id}"))
                .unwrap_or_else(|| panic!("Module {id} does not exist"));
            let callee_module = CompiledModule::deserialize(&bytes)
                .unwrap_or_else(|_| panic!("Deserialization of module {id} should not fail"));
            cost += additional_call_cost(&callee_module)
        };
        cost
    }
}

#[derive(ValueEnum, Clone)]
pub enum Extension {
    Txt,
    Dot,
}

#[derive(ValueEnum, Clone)]
pub enum Counter {
    Instructions,
    StaticGas,
}

impl Display for Extension {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Extension::Txt => write!(f, "txt"),
            Extension::Dot => write!(f, "dot"),
        }
    }
}

pub struct CollectCostCFGs<'s, S> {
    output_dir: &'s PathBuf,
    output_extension: &'s Extension,
    calculator: Option<InstructionCostCalculator<'s, S>>,
}

impl<'s, S: StateView> CollectCostCFGs<'s, S> {
    pub fn new(
        output_dir: &'s PathBuf,
        output_extension: &'s Extension,
        state_view: &'s S,
        counter: &Counter,
    ) -> Self {
        let calculator = match counter {
            Counter::Instructions => None,
            Counter::StaticGas => Some(InstructionCostCalculator::new(state_view)),
        };
        Self {
            output_dir,
            output_extension,
            calculator,
        }
    }

    fn count(&self, module: &CompiledModule, instruction: &Bytecode) -> u64 {
        self.calculator
            .as_ref()
            .map(|c| c.instruction_static_cost(module, instruction))
            .unwrap_or(1)
    }
}

impl<'s, S: StateView> ModulePass for CollectCostCFGs<'s, S> {
    fn run_on_module(&mut self, module: &CompiledModule) {
        for function in module.function_defs() {
            // Skip native functions because they don't have any code.
            if function.is_native() {
                continue;
            }

            let code_unit = &function.code;
            let code = &code_unit.as_ref().unwrap().code;
            let cfg = VMControlFlowGraph::new(code);

            // Renumber the blocks because CFG representation here uses offsets.
            let mut num_edges = 0;
            let mut idx_map = HashMap::new();
            for (idx, bb) in cfg.blocks().into_iter().enumerate() {
                idx_map.insert(bb, idx);
                num_edges += cfg.successors(bb).len();
            }
            let num_vertices = cfg.num_blocks();

            // File names also have number of vertices and edges for easier lookup or
            // manual sorting.
            let handle = module.function_handle_at(function.function);
            let address = module.self_id().address.to_standard_string();
            let module_name = module.self_id().name.to_string();
            let function_name = module.identifier_at(handle.name).to_owned();
            let filename = format!(
                "{}-{}-{}-{}-{}.{}",
                num_vertices, num_edges, address, module_name, function_name, self.output_extension
            );

            let path = self.output_dir.as_path().join(filename);
            let mut output = File::create(path.as_path())
                .expect("Should be able to create a file for CFG output");

            match self.output_extension {
                Extension::Txt => {
                    // Output follows classical order:
                    //   - number of vertices and edges
                    //   - vertex list with their costs
                    //   - edge list
                    writeln!(output, "{} {}", num_vertices, num_edges).unwrap();
                    for bb in cfg.blocks() {
                        let mut cost = 0;
                        for i in cfg.instr_indexes(bb) {
                            cost += self.count(module, &code[i as usize]);
                        }
                        let u = idx_map.get(&bb).unwrap();
                        writeln!(output, "{} {}", u, cost).unwrap();
                    }
                    for bb1 in cfg.blocks() {
                        for bb2 in cfg.successors(bb1) {
                            let u = idx_map.get(&bb1).unwrap();
                            let v = idx_map.get(bb2).unwrap();
                            writeln!(output, "{} {}", u, v).unwrap();
                        }
                    }
                },
                Extension::Dot => {
                    // DOT format for pretty printing.
                    writeln!(output, "digraph {{")
                    .unwrap();
                    for bb1 in cfg.blocks() {
                        for bb2 in cfg.successors(bb1) {
                            let u = idx_map.get(&bb1).unwrap();
                            let v = idx_map.get(bb2).unwrap();
                            writeln!(output, "  {} -> {}", u, v).unwrap();
                        }
                    }
                    writeln!(output).unwrap();
                    for bb in cfg.blocks() {
                        let mut cost = 0;
                        for i in cfg.instr_indexes(bb) {
                            cost += self.count(module, &code[i as usize]);
                        }
                        let shape = if bb == 0 || cfg.successors(bb).is_empty() {
                            "box"
                        } else {
                            "circle"
                        };
                        let u = idx_map.get(&bb).unwrap();
                        writeln!(output, "  {} [shape={}, label=\"{}\"]", u, shape, cost).unwrap();
                    }
                    writeln!(output, "}}").unwrap();
                },
            }
        }
    }
}
