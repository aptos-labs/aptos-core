// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::ModulePass;
use aptos_types::{
    on_chain_config::{GasSchedule, GasScheduleV2, OnChainConfig},
    state_store::StateView,
};
use move_binary_format::{
    access::ModuleAccess,
    control_flow_graph::{ControlFlowGraph, VMControlFlowGraph},
    file_format::Bytecode,
    CompiledModule,
};
use std::collections::{BTreeMap, HashMap};

pub struct InstructionCostCalculator {
    costs: BTreeMap<String, u64>,
}

impl InstructionCostCalculator {
    #[allow(dead_code)]
    fn new(state_view: &impl StateView) -> Self {
        let costs = match GasScheduleV2::fetch_config(state_view) {
            Some(gas_schedule) => gas_schedule.to_btree_map(),
            None => GasSchedule::fetch_config(state_view)
                .expect("At least one version of gas schedule must exist")
                .to_btree_map(),
        };
        Self { costs }
    }

    fn instruction_static_cost(&self, module: &CompiledModule, instruction: &Bytecode) -> u64 {
        let cost = |param: &str| -> u64 {
            *self
                .costs
                .get(param)
                .expect("Cost should exist for every instruction")
        };

        // TODO: can we do it in a nicer way?
        match instruction {
            Bytecode::Nop => cost("nop"),
            Bytecode::Ret => cost("ret"),
            Bytecode::Abort => cost("abort"),
            Bytecode::BrTrue(_) => cost("br_true"),
            Bytecode::BrFalse(_) => cost("br_false"),
            Bytecode::Branch(_) => cost("branch"),
            Bytecode::Pop => cost("pop"),
            Bytecode::LdU8(_) => cost("ld_u8"),
            Bytecode::LdU16(_) => cost("ld_u16"),
            Bytecode::LdU32(_) => cost("ld_u32"),
            Bytecode::LdU64(_) => cost("ld_u64"),
            Bytecode::LdU128(_) => cost("ld_u128"),
            Bytecode::LdU256(_) => cost("ld_u256"),
            Bytecode::LdTrue => cost("ld_true"),
            Bytecode::LdFalse => cost("ld_false"),
            Bytecode::LdConst(_) => cost("ld_const_base"),

            Bytecode::ImmBorrowLoc(_) => cost("imm_borrow_loc"),
            Bytecode::MutBorrowLoc(_) => cost("mut_borrow_loc"),
            Bytecode::ImmBorrowField(_) => cost("imm_borrow_field"),
            Bytecode::MutBorrowField(_) => cost("mut_borrow_field"),
            Bytecode::ImmBorrowFieldGeneric(_) => cost("imm_borrow_field_generic"),
            Bytecode::MutBorrowFieldGeneric(_) => cost("mut_borrow_field_generic"),
            Bytecode::CopyLoc(_) => cost("copy_loc.base"),
            Bytecode::MoveLoc(_) => cost("move_loc.base"),
            Bytecode::StLoc(_) => cost("st_loc.base"),
            Bytecode::ReadRef => cost("read_ref.base"),
            Bytecode::WriteRef => cost("write_ref.base"),
            Bytecode::FreezeRef => cost("freeze_ref"),
            Bytecode::CastU8 => cost("cast_u8"),
            Bytecode::CastU16 => cost("cast_u16"),
            Bytecode::CastU32 => cost("cast_u32"),
            Bytecode::CastU64 => cost("cast_u64"),
            Bytecode::CastU128 => cost("cast_u128"),
            Bytecode::CastU256 => cost("cast_u256"),
            Bytecode::Add => cost("add"),
            Bytecode::Sub => cost("sub"),
            Bytecode::Mul => cost("mul"),
            Bytecode::Mod => cost("mod"),
            Bytecode::Div => cost("div"),
            Bytecode::BitOr => cost("bit_or"),
            Bytecode::BitAnd => cost("bit_and"),
            Bytecode::Xor => cost("bit_xor"),
            Bytecode::Shl => cost("bit_shl"),
            Bytecode::Shr => cost("bit_shr"),
            Bytecode::Or => cost("or"),
            Bytecode::And => cost("and"),
            Bytecode::Not => cost("not"),
            Bytecode::Lt => cost("lt"),
            Bytecode::Gt => cost("gt"),
            Bytecode::Le => cost("le"),
            Bytecode::Ge => cost("ge"),
            Bytecode::Eq => cost("eq.base"),
            Bytecode::Neq => cost("neq.base"),

            // Note: these don't take into account the cost of loading.
            Bytecode::ImmBorrowGlobal(_) => cost("imm_borrow_global.base"),
            Bytecode::ImmBorrowGlobalGeneric(_) => cost("imm_borrow_global_generic.base"),
            Bytecode::MutBorrowGlobal(_) => cost("mut_borrow_global.base"),
            Bytecode::MutBorrowGlobalGeneric(_) => cost("mut_borrow_global_generic.base"),
            Bytecode::Exists(_) => cost("exists.base"),
            Bytecode::ExistsGeneric(_) => cost("exists_generic.base"),
            Bytecode::MoveTo(_) => cost("move_to.base"),
            Bytecode::MoveToGeneric(_) => cost("move_to_generic.base"),
            Bytecode::MoveFrom(_) => cost("move_from.base"),
            Bytecode::MoveFromGeneric(_) => cost("move_from_generic.base"),

            Bytecode::VecLen(_) => cost("vec_len.base"),
            Bytecode::VecImmBorrow(_) => cost("vec_imm_borrow.base"),
            Bytecode::VecMutBorrow(_) => cost("vec_mut_borrow.base"),
            Bytecode::VecPopBack(_) => cost("vec_pop_back.base"),
            Bytecode::VecPushBack(_) => cost("vec_push_back.base"),
            Bytecode::VecSwap(_) => cost("vec_swap.base"),
            Bytecode::VecPack(_, n) => cost("vec_pack.base") + n * cost("vec_pack.per_elem"),
            Bytecode::VecUnpack(_, n) => {
                cost("vec_unpack.base") + n * cost("vec_unpack.per_expected_elem")
            },

            Bytecode::Unpack(idx) => {
                let struct_def = module.struct_def_at(*idx);
                let n = struct_def
                    .declared_field_count()
                    .expect("There are no native structs anymore") as u64;
                cost("unpack.base") + n * cost("unpack.per_field")
            },
            Bytecode::UnpackGeneric(idx) => {
                let struct_def = module.struct_def_at(module.struct_instantiation_at(*idx).def);
                let n = struct_def
                    .declared_field_count()
                    .expect("There are no native structs anymore") as u64;
                cost("unpack_generic.base") + n * cost("unpack_generic.per_field")
            },
            Bytecode::Pack(idx) => {
                let struct_def = module.struct_def_at(*idx);
                let n = struct_def
                    .declared_field_count()
                    .expect("There are no native structs anymore") as u64;
                cost("pack.base") + n * cost("pack.per_field")
            },
            Bytecode::PackGeneric(idx) => {
                let struct_def = module.struct_def_at(module.struct_instantiation_at(*idx).def);
                let n = struct_def
                    .declared_field_count()
                    .expect("There are no native structs anymore") as u64;
                cost("pack_generic.base") + n * cost("pack_generic.per_field")
            },

            // Note: for functions, we can make cross-module calls, so the API
            // mut be able to fetch modules with its dependencies. Skip for now?

            // TODO: cost("call.per_arg"), cost("call.per_local")
            Bytecode::Call(_) => cost("call.base"),
            // TODO: cost("call_generic.per_ty_arg"), cost("call_generic.per_arg"), cost("call_generic.per_local)"
            Bytecode::CallGeneric(_) => cost("call_generic.base"),
        }
    }
}

impl ModulePass for InstructionCostCalculator {
    fn run_on_module(&mut self, module: &CompiledModule) {
        for function in module.function_defs() {
            let handle = module.function_handle_at(function.function);
            let function_name = module.identifier_at(handle.name).to_owned();

            if function.code.is_none() {
                return;
            }

            println!("{}", function_name);
            let code = &function.code;
            let code = &code.as_ref().unwrap().code;
            let cfg = VMControlFlowGraph::new(code);

            let mut idx_map = HashMap::new();
            let mut num_edges = 0;
            for (idx, bb) in cfg.blocks().into_iter().enumerate() {
                idx_map.insert(bb, idx);
                num_edges += cfg.successors(bb).len();
            }
            println!("{} {}", cfg.num_blocks(), num_edges);

            for bb in cfg.blocks() {
                let mut bb_cost = 0;
                for i in cfg.instr_indexes(bb) {
                    bb_cost += self.instruction_static_cost(module, &code[i as usize]);
                }
                println!("{} {}", idx_map.get(&bb).unwrap(), bb_cost)
            }

            for u in cfg.blocks() {
                for v in cfg.successors(u) {
                    println!("{} {}", idx_map.get(&u).unwrap(), idx_map.get(v).unwrap())
                }
            }
        }
    }
}
