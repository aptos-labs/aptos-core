// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    binary_views::BinaryIndexedView,
    errors::{PartialVMError, PartialVMResult},
    file_format::{
        Bytecode, CodeUnit, CompiledModule, CompiledScript, FieldInstantiationIndex,
        FunctionInstantiationIndex, IdentifierIndex, ModuleHandleIndex, SignatureIndex,
        SignatureToken, StructDefInstantiationIndex, StructFieldInformation,
        StructVariantInstantiationIndex, TableIndex, VariantFieldInstantiationIndex,
    },
};
use move_core_types::vm_status::StatusCode;
use std::{
    cell::RefCell,
    collections::{btree_map, BTreeMap},
};

const COST_PER_TYPE_NODE: u64 = 8;
const COST_PER_IDENT_BYTE: u64 = 1;

fn safe_get_table<T>(table: &[T], idx: TableIndex) -> PartialVMResult<&T> {
    table.get(idx as usize).ok_or_else(|| {
        PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
            .with_message("Index out of bounds while checking binary complexity.".to_string())
    })
}

struct BinaryComplexityMeter<'a> {
    resolver: BinaryIndexedView<'a>,

    cached_signature_costs: RefCell<BTreeMap<SignatureIndex, u64>>,
    balance: RefCell<u64>,
}

impl<'a> BinaryComplexityMeter<'a> {
    fn charge(&self, amount: u64) -> PartialVMResult<()> {
        let mut balance = self.balance.borrow_mut();
        match balance.checked_sub(amount) {
            Some(new_balance) => {
                *balance = new_balance;
                Ok(())
            },
            None => {
                *balance = 0;
                Err(PartialVMError::new(StatusCode::PROGRAM_TOO_COMPLEX))
            },
        }
    }

    fn signature_token_cost(&self, tok: &SignatureToken) -> PartialVMResult<u64> {
        use SignatureToken::*;

        let mut cost: u64 = 0;

        for node in tok.preorder_traversal() {
            cost = cost.saturating_add(COST_PER_TYPE_NODE);

            match node {
                Struct(sh_idx) | StructInstantiation(sh_idx, _) => {
                    let sh = safe_get_table(self.resolver.struct_handles(), sh_idx.0)?;
                    let mh = safe_get_table(self.resolver.module_handles(), sh.module.0)?;
                    let struct_name = safe_get_table(self.resolver.identifiers(), sh.name.0)?;
                    let moduel_name = safe_get_table(self.resolver.identifiers(), mh.name.0)?;

                    cost = cost.saturating_add(struct_name.len() as u64 * COST_PER_IDENT_BYTE);
                    cost = cost.saturating_add(moduel_name.len() as u64 * COST_PER_IDENT_BYTE);
                },
                U8 | U16 | U32 | U64 | U128 | U256 | Signer | Address | Bool | Vector(_)
                | Function(..) | TypeParameter(_) | Reference(_) | MutableReference(_) => (),
            }
        }

        Ok(cost)
    }

    fn meter_identifier(&self, idx: IdentifierIndex) -> PartialVMResult<()> {
        let ident = safe_get_table(self.resolver.identifiers(), idx.0)?;
        self.charge(ident.len() as u64 * COST_PER_IDENT_BYTE)
    }

    fn meter_signature(&self, idx: SignatureIndex) -> PartialVMResult<()> {
        let cost = match self.cached_signature_costs.borrow_mut().entry(idx) {
            btree_map::Entry::Occupied(entry) => *entry.into_mut(),
            btree_map::Entry::Vacant(entry) => {
                let sig = safe_get_table(self.resolver.signatures(), idx.0)?;

                let mut cost: u64 = 0;
                for ty in &sig.0 {
                    cost = cost.saturating_add(self.signature_token_cost(ty)?);
                }

                *entry.insert(cost)
            },
        };

        self.charge(cost)?;

        Ok(())
    }

    fn meter_signatures(&self) -> PartialVMResult<()> {
        for sig_idx in 0..self.resolver.signatures().len() {
            self.meter_signature(SignatureIndex(sig_idx as u16))?;
        }
        Ok(())
    }

    fn meter_function_instantiation(
        &self,
        func_inst_idx: FunctionInstantiationIndex,
    ) -> PartialVMResult<()> {
        let func_inst = safe_get_table(self.resolver.function_instantiations(), func_inst_idx.0)?;
        self.meter_signature(func_inst.type_parameters)
    }

    fn meter_function_instantiations(&self) -> PartialVMResult<()> {
        for func_inst_idx in 0..self.resolver.function_instantiations().len() {
            self.meter_function_instantiation(FunctionInstantiationIndex(func_inst_idx as u16))?;
        }
        Ok(())
    }

    fn meter_struct_instantiation(
        &self,
        struct_inst_idx: StructDefInstantiationIndex,
    ) -> PartialVMResult<()> {
        let struct_insts = self.resolver.struct_instantiations().ok_or_else(|| {
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message("Can't get struct instantiations -- not a module.".to_string())
        })?;
        let struct_inst = safe_get_table(struct_insts, struct_inst_idx.0)?;

        self.meter_signature(struct_inst.type_parameters)
    }

    fn meter_struct_variant_instantiation(
        &self,
        struct_inst_idx: StructVariantInstantiationIndex,
    ) -> PartialVMResult<()> {
        let struct_variant_insts =
            self.resolver
                .struct_variant_instantiations()
                .ok_or_else(|| {
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                        "Can't get enum type instantiation -- not a module.".to_string(),
                    )
                })?;
        let struct_variant_inst = safe_get_table(struct_variant_insts, struct_inst_idx.0)?;
        self.meter_signature(struct_variant_inst.type_parameters)
    }

    fn meter_struct_def_instantiations(&self) -> PartialVMResult<()> {
        let struct_insts = self.resolver.struct_instantiations().ok_or_else(|| {
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message("Can't get struct instantiations -- not a module.".to_string())
        })?;

        for struct_inst_idx in 0..struct_insts.len() {
            self.meter_struct_instantiation(StructDefInstantiationIndex(struct_inst_idx as u16))?;
        }
        Ok(())
    }

    fn meter_field_instantiation(
        &self,
        field_inst_idx: FieldInstantiationIndex,
    ) -> PartialVMResult<()> {
        let field_insts = self.resolver.field_instantiations().ok_or_else(|| {
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message("Can't get field instantiations -- not a module.".to_string())
        })?;
        let field_inst = safe_get_table(field_insts, field_inst_idx.0)?;

        self.meter_signature(field_inst.type_parameters)
    }

    fn meter_variant_field_instantiation(
        &self,
        variant_field_inst_idx: VariantFieldInstantiationIndex,
    ) -> PartialVMResult<()> {
        let variant_field_insts =
            self.resolver
                .variant_field_instantiations()
                .ok_or_else(|| {
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                        "Can't get variant field instantiations -- not a module.".to_string(),
                    )
                })?;
        let field_inst = safe_get_table(variant_field_insts, variant_field_inst_idx.0)?;

        self.meter_signature(field_inst.type_parameters)
    }

    fn meter_field_instantiations(&self) -> PartialVMResult<()> {
        let field_insts = self.resolver.field_instantiations().ok_or_else(|| {
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message("Can't get field instantiations -- not a module.".to_string())
        })?;

        for field_inst_idx in 0..field_insts.len() {
            self.meter_field_instantiation(FieldInstantiationIndex(field_inst_idx as u16))?;
        }
        Ok(())
    }

    fn meter_module_handle(&self, mh_idx: ModuleHandleIndex) -> PartialVMResult<()> {
        let mh = safe_get_table(self.resolver.module_handles(), mh_idx.0)?;
        self.meter_identifier(mh.name)
    }

    fn meter_function_handles(&self) -> PartialVMResult<()> {
        for fh in self.resolver.function_handles() {
            self.meter_module_handle(fh.module)?;
            self.meter_identifier(fh.name)?;
            self.meter_signature(fh.parameters)?;
            self.meter_signature(fh.return_)?;
        }
        Ok(())
    }

    fn meter_struct_handles(&self) -> PartialVMResult<()> {
        for sh in self.resolver.struct_handles() {
            self.meter_module_handle(sh.module)?;
            self.meter_identifier(sh.name)?;
        }
        Ok(())
    }

    fn meter_struct_defs(&self) -> PartialVMResult<()> {
        let struct_defs = self.resolver.struct_defs().ok_or_else(|| {
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message("Can't get struct defs -- not a module.".to_string())
        })?;

        for sdef in struct_defs {
            match &sdef.field_information {
                StructFieldInformation::Native => continue,
                StructFieldInformation::Declared(fields) => {
                    for field in fields {
                        self.charge(field.signature.0.num_nodes() as u64)?;
                    }
                },
                StructFieldInformation::DeclaredVariants(variants) => {
                    for variant in variants {
                        self.meter_identifier(variant.name)?;
                        for field in &variant.fields {
                            self.charge(field.signature.0.num_nodes() as u64)?;
                        }
                    }
                },
            }
        }
        Ok(())
    }

    fn meter_code(&self, code: &CodeUnit) -> PartialVMResult<()> {
        use Bytecode::*;

        self.meter_signature(code.locals)?;

        for instr in &code.code {
            match instr {
                CallGeneric(idx) | LdFunctionGeneric(idx, ..) => {
                    self.meter_function_instantiation(*idx)?;
                },
                PackGeneric(idx) | UnpackGeneric(idx) => {
                    self.meter_struct_instantiation(*idx)?;
                },
                PackVariantGeneric(idx) | UnpackVariantGeneric(idx) | TestVariantGeneric(idx) => {
                    self.meter_struct_variant_instantiation(*idx)?;
                },
                ExistsGeneric(idx)
                | MoveFromGeneric(idx)
                | MoveToGeneric(idx)
                | ImmBorrowGlobalGeneric(idx)
                | MutBorrowGlobalGeneric(idx) => {
                    self.meter_struct_instantiation(*idx)?;
                },
                ImmBorrowFieldGeneric(idx) | MutBorrowFieldGeneric(idx) => {
                    self.meter_field_instantiation(*idx)?;
                },
                ImmBorrowVariantFieldGeneric(idx) | MutBorrowVariantFieldGeneric(idx) => {
                    self.meter_variant_field_instantiation(*idx)?;
                },
                VecPack(idx, _)
                | VecLen(idx)
                | VecImmBorrow(idx)
                | VecMutBorrow(idx)
                | VecPushBack(idx)
                | VecPopBack(idx)
                | VecUnpack(idx, _)
                | VecSwap(idx)
                | Invoke(idx)
                | EarlyBind(idx, _) => {
                    self.meter_signature(*idx)?;
                },

                // List out the other options explicitly so there's a compile error if a new
                // bytecode gets added.
                Pop
                | Ret
                | Branch(_)
                | BrTrue(_)
                | BrFalse(_)
                | LdU8(_)
                | LdU16(_)
                | LdU32(_)
                | LdU64(_)
                | LdU128(_)
                | LdU256(_)
                | LdConst(_)
                | CastU8
                | CastU16
                | CastU32
                | CastU64
                | CastU128
                | CastU256
                | LdTrue
                | LdFalse
                | Call(_)
                | Pack(_)
                | Unpack(_)
                | PackVariant(_)
                | UnpackVariant(_)
                | TestVariant(_)
                | LdFunction(_)
                | ReadRef
                | WriteRef
                | FreezeRef
                | Add
                | Sub
                | Mul
                | Mod
                | Div
                | BitOr
                | BitAnd
                | Xor
                | Shl
                | Shr
                | Or
                | And
                | Not
                | Eq
                | Neq
                | Lt
                | Gt
                | Le
                | Ge
                | CopyLoc(_)
                | MoveLoc(_)
                | StLoc(_)
                | MutBorrowLoc(_)
                | ImmBorrowLoc(_)
                | MutBorrowField(_)
                | ImmBorrowField(_)
                | MutBorrowVariantField(_)
                | ImmBorrowVariantField(_)
                | MutBorrowGlobal(_)
                | ImmBorrowGlobal(_)
                | Exists(_)
                | MoveTo(_)
                | MoveFrom(_)
                | Abort
                | Nop => (),
            }
        }
        Ok(())
    }

    fn meter_function_defs(&self) -> PartialVMResult<()> {
        let func_defs = self.resolver.function_defs().ok_or_else(|| {
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message("Can't get func defs -- not a module.".to_string())
        })?;

        for func_def in func_defs {
            if let Some(code) = &func_def.code {
                self.meter_code(code)?;
            }
        }
        Ok(())
    }
}

pub fn check_module_complexity(module: &CompiledModule, budget: u64) -> PartialVMResult<u64> {
    let meter = BinaryComplexityMeter {
        resolver: BinaryIndexedView::Module(module),
        cached_signature_costs: RefCell::new(BTreeMap::new()),
        balance: RefCell::new(budget),
    };

    meter.meter_signatures()?;
    meter.meter_function_instantiations()?;
    meter.meter_struct_def_instantiations()?;
    meter.meter_field_instantiations()?;

    meter.meter_function_handles()?;
    meter.meter_struct_handles()?;
    meter.meter_function_defs()?;
    meter.meter_struct_defs()?;

    let used = budget - *meter.balance.borrow();
    Ok(used)
}

pub fn check_script_complexity(script: &CompiledScript, budget: u64) -> PartialVMResult<u64> {
    let meter = BinaryComplexityMeter {
        resolver: BinaryIndexedView::Script(script),
        cached_signature_costs: RefCell::new(BTreeMap::new()),
        balance: RefCell::new(budget),
    };

    meter.meter_signatures()?;
    meter.meter_function_instantiations()?;

    meter.meter_function_handles()?;
    meter.meter_struct_handles()?;
    meter.meter_code(&script.code)?;

    let used = budget - *meter.balance.borrow();
    Ok(used)
}
