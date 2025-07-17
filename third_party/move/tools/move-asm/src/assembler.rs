// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implements the assembler from the abstract syntax into bytecode,
//! using the `ModuleBuilder`.

use crate::{
    module_builder::{ModuleBuilder, ModuleBuilderOptions},
    syntax,
    syntax::{
        map_diag, Argument, AsmResult, Diag, Fun, Instruction, Loc, Local, Type, Unit, UnitId,
        Value,
    },
    ModuleOrScript,
};
use either::Either;
use move_binary_format::{
    file_format::{
        Bytecode, CodeOffset, FunctionDefinitionIndex, FunctionHandleIndex, LocalIndex,
        SignatureIndex, SignatureToken, TableIndex,
    },
    CompiledModule,
};
use move_core_types::{function::ClosureMask, identifier::Identifier, u256::U256};
use std::collections::BTreeMap;

struct Assembler<'a> {
    builder: ModuleBuilder<'a>,
    diags: Vec<Diag>,
    /// Context available during processing of a function.
    fun_context: Option<FunctionContext>,
}

struct FunctionContext {
    ty_param_map: BTreeMap<String, u16>,
    local_map: BTreeMap<String, (LocalIndex, SignatureToken)>,
}

pub(crate) fn compile<'a>(
    options: ModuleBuilderOptions,
    context_modules: impl IntoIterator<Item = &'a CompiledModule>,
    ast: Unit,
) -> AsmResult<ModuleOrScript> {
    let mut compiler = Assembler {
        builder: ModuleBuilder::new(options, context_modules, ast.name.module_opt()),
        diags: vec![],
        fun_context: None,
    };
    compiler.unit(&ast);
    if compiler.diags.is_empty() {
        match ast.name {
            UnitId::Script => map_diag(compiler.builder.into_script()).map(Either::Right),
            UnitId::Module(_) => map_diag(compiler.builder.into_module()).map(Either::Left),
        }
    } else {
        Err(compiler.diags)
    }
}

impl<'a> Assembler<'a> {
    fn unit(&mut self, ast: &Unit) {
        let Unit {
            name: _,
            address_aliases,
            module_aliases,
            functions,
        } = ast;
        // Register aliases
        for (n, a) in address_aliases {
            let res = self.builder.declare_address_alias(n, *a);
            self.add_diags(Loc::new(0, 0), res);
        }
        for (n, m) in module_aliases {
            let res = self.builder.declare_module_alias(n, m);
            self.add_diags(Loc::new(0, 0), res);
        }

        // Declare functions
        for fun in functions {
            self.declare_fun(fun);
        }

        // Define code for functions
        if self.diags.is_empty() {
            for (pos, fun) in functions.iter().enumerate() {
                self.define_fun(FunctionDefinitionIndex::new(pos as TableIndex), fun)
            }
        }
    }

    fn declare_fun(&mut self, fun: &Fun) {
        self.setup_fun(fun);
        let param_tys: Vec<SignatureToken> = fun
            .params
            .iter()
            .map(|local| self.build_type(fun.loc, &local.ty))
            .collect();
        let res = self.builder.signature_index(param_tys);
        let param_sign = self.add_diags(fun.loc, res).unwrap_or_default();
        let result_tys: Vec<SignatureToken> = fun
            .result
            .iter()
            .map(|ty| self.build_type(fun.loc, ty))
            .collect();
        let res = self.builder.signature_index(result_tys);
        let result_sign = self.add_diags(fun.loc, res).unwrap_or_default();
        let res = self.builder.declare_fun(
            false, // TODO(#16582): entry
            fun.name.clone(),
            fun.visibility,
            param_sign,
            result_sign,
            fun.type_params
                .iter()
                .map(|(_, abilities)| *abilities)
                .collect(),
        );
        self.add_diags(fun.loc, res);
    }

    fn setup_fun(&mut self, fun: &Fun) {
        let ty_param_map = fun
            .type_params
            .iter()
            .enumerate()
            .map(|(pos, (id, _))| (id.to_string(), pos as u16))
            .collect();
        self.fun_context = Some(FunctionContext {
            ty_param_map, // This is needed for build_type called below
            local_map: Default::default(),
        });
        self.fun_context.as_mut().unwrap().local_map = fun
            .params
            .iter()
            .chain(fun.locals.iter())
            .enumerate()
            .map(|(pos, Local { loc, name, ty })| {
                let ty = self.build_type(*loc, ty);
                (name.to_string(), (pos as LocalIndex, ty))
            })
            .collect();
    }

    fn require_fun(&self) -> &FunctionContext {
        self.fun_context.as_ref().expect("function context")
    }

    fn define_fun(&mut self, def_idx: FunctionDefinitionIndex, fun: &Fun) {
        if !fun.instrs.is_empty() {
            self.setup_fun(fun);
            let mut open_branches = BTreeMap::new();
            let mut label_defs = BTreeMap::new();
            let mut code = vec![];
            let mut has_errors = false;
            for (offs, instr) in fun.instrs.iter().enumerate() {
                if let Some(label) = instr.label.as_ref() {
                    label_defs.insert(label.clone(), offs as CodeOffset);
                }
                if let Some(bc) = self.build_instr(
                    instr,
                    offs as CodeOffset,
                    &mut open_branches,
                    &mut label_defs,
                ) {
                    code.push(bc)
                } else {
                    // else error reported
                    has_errors = true
                }
            }
            if !has_errors {
                // Link forward pointing branch targets
                for (offs, (use_loc, label)) in open_branches {
                    if let Some(target_offs) = label_defs.get(&label) {
                        match &mut code[offs as usize] {
                            Bytecode::Branch(open)
                            | Bytecode::BrTrue(open)
                            | Bytecode::BrFalse(open) => *open = *target_offs,
                            _ => panic!("unexpected bytecode"),
                        };
                    } else {
                        self.error(use_loc, format!("unbound branch label `{}`", label))
                    }
                }
                // Define locals signature.
                let locals_start = fun.params.len();
                let mut locals: Vec<(LocalIndex, SignatureToken)> = self
                    .require_fun()
                    .local_map
                    .clone()
                    .into_iter()
                    .filter_map(|(_, r)| {
                        if r.0 as usize >= locals_start {
                            Some(r)
                        } else {
                            None
                        }
                    })
                    .collect();
                locals.sort_by(|e1, e2| e1.0.cmp(&e2.0));
                let res = self
                    .builder
                    .signature_index(locals.into_iter().map(|(_, ty)| ty).collect());
                if let Some(sign_index) = self.add_diags(fun.loc, res) {
                    self.builder.define_fun_code(def_idx, sign_index, code)
                }
            }
        }
    }

    fn build_type(&mut self, loc: Loc, ty: &Type) -> SignatureToken {
        let ck_inst = |comp: &mut Self, req: Option<usize>, inst: Option<&Vec<Type>>| -> bool {
            match req {
                None => {
                    if inst.is_some() {
                        comp.error(loc, "no type arguments expected");
                        false
                    } else {
                        true
                    }
                },
                Some(count) => {
                    if inst.map_or(0, |ty| ty.len()) != count {
                        comp.error(loc, format!("expected {} type arguments", count));
                        false
                    } else {
                        true
                    }
                },
            }
        };
        let tr_inst = |comp: &mut Self, inst: Option<&Vec<Type>>| -> Option<Vec<SignatureToken>> {
            inst.map(|tys| tys.iter().map(|t| comp.build_type(loc, t)).collect())
        };
        match ty {
            Type::Named(partial_id, opt_inst) => {
                if partial_id.address.is_none() && partial_id.id_parts.len() == 1 {
                    match partial_id.id_parts[0].as_str() {
                        s if self.require_fun().ty_param_map.contains_key(s) => {
                            SignatureToken::TypeParameter(self.require_fun().ty_param_map[s])
                        },
                        "u8" => {
                            ck_inst(self, None, opt_inst.as_ref());
                            SignatureToken::U8
                        },
                        "u16" => {
                            ck_inst(self, None, opt_inst.as_ref());
                            SignatureToken::U16
                        },
                        "u32" => {
                            ck_inst(self, None, opt_inst.as_ref());
                            SignatureToken::U32
                        },
                        "u64" => {
                            ck_inst(self, None, opt_inst.as_ref());
                            SignatureToken::U64
                        },
                        "u128" => {
                            ck_inst(self, None, opt_inst.as_ref());
                            SignatureToken::U128
                        },
                        "u256" => {
                            ck_inst(self, None, opt_inst.as_ref());
                            SignatureToken::U256
                        },
                        "bool" => {
                            ck_inst(self, None, opt_inst.as_ref());
                            SignatureToken::Bool
                        },
                        "address" => {
                            ck_inst(self, None, opt_inst.as_ref());
                            SignatureToken::Address
                        },
                        "signer" => {
                            ck_inst(self, None, opt_inst.as_ref());
                            SignatureToken::Signer
                        },
                        "vector" => {
                            if ck_inst(self, Some(1), opt_inst.as_ref()) {
                                SignatureToken::Vector(Box::new(
                                    tr_inst(self, opt_inst.as_ref()).unwrap().pop().unwrap(),
                                ))
                            } else {
                                // error reported
                                SignatureToken::Bool
                            }
                        },
                        _ => {
                            self.error(loc, "structs NYI");
                            SignatureToken::Bool
                        },
                    }
                } else {
                    self.error(loc, "structs NYI type");
                    SignatureToken::Bool
                }
            },
            Type::Ref(is_mut, ty) => {
                let ty = Box::new(self.build_type(loc, ty));
                if *is_mut {
                    SignatureToken::MutableReference(ty)
                } else {
                    SignatureToken::Reference(ty)
                }
            },
            Type::Func(args, result, abilities) => SignatureToken::Function(
                args.iter().map(|ty| self.build_type(loc, ty)).collect(),
                result.iter().map(|ty| self.build_type(loc, ty)).collect(),
                *abilities,
            ),
        }
    }

    fn build_instr(
        &mut self,
        instr: &Instruction,
        offs: CodeOffset,
        open_branches: &mut BTreeMap<CodeOffset, (Loc, Identifier)>,
        label_defs: &mut BTreeMap<Identifier, CodeOffset>,
    ) -> Option<Bytecode> {
        use Bytecode::*;
        let instr_name = instr.name.to_string().to_lowercase();
        let instr = match instr_name.as_str() {
            "pop" => {
                self.args0(instr)?;
                Pop
            },
            "ret" => {
                self.args0(instr)?;
                Ret
            },
            "br_true" | "br_false" | "branch" => {
                let mk_instr = match instr_name.as_str() {
                    "br_true" => BrTrue,
                    "br_false" => BrFalse,
                    "branch" => Branch,
                    _ => unreachable!(),
                };
                let arg = self.args1(instr)?;
                let label = self.simple_id(instr, arg)?;
                if let Some(label_offs) = label_defs.get(&label) {
                    mk_instr(*label_offs)
                } else {
                    // Forward branch, remember we need to link offset
                    open_branches.insert(offs, (instr.loc, label));
                    mk_instr(0)
                }
            },
            "ld_u8" => {
                let arg = self.args1(instr)?;
                let num = self.number(instr, arg, U256::from(u8::MAX))?;
                LdU8(num.unchecked_as_u8())
            },
            "ld_u16" => {
                let arg = self.args1(instr)?;
                let num = self.number(instr, arg, U256::from(u16::MAX))?;
                LdU16(num.unchecked_as_u16())
            },
            "ld_u32" => {
                let arg = self.args1(instr)?;
                let num = self.number(instr, arg, U256::from(u32::MAX))?;
                LdU32(num.unchecked_as_u32())
            },
            "ld_u64" => {
                let arg = self.args1(instr)?;
                let num = self.number(instr, arg, U256::from(u64::MAX))?;
                LdU64(num.unchecked_as_u64())
            },
            "ld_u128" => {
                let arg = self.args1(instr)?;
                let num = self.number(instr, arg, U256::from(u128::MAX))?;
                LdU128(num.unchecked_as_u128())
            },
            "ld_u256" => {
                let arg = self.args1(instr)?;
                let num = self.number(instr, arg, U256::max_value())?;
                LdU256(num)
            },

            "cast_u8" => {
                self.args0(instr)?;
                CastU8
            },
            "cast_u16" => {
                self.args0(instr)?;
                CastU16
            },
            "cast_u32" => {
                self.args0(instr)?;
                CastU32
            },
            "cast_u64" => {
                self.args0(instr)?;
                CastU64
            },
            "cast_u128" => {
                self.args0(instr)?;
                CastU128
            },
            "cast_u256" => {
                self.args0(instr)?;
                CastU256
            },
            "ld_const" => {
                let [arg1, arg2] = self.args2(instr)?;
                let ty = self.type_(instr, arg1)?;
                let val = self.value_bcs(instr, arg2, &ty)?;
                let res = self.builder.const_index(val, ty);
                let idx = self.add_diags(instr.loc, res)?;
                LdConst(idx)
            },
            "ld_false" => {
                self.args0(instr)?;
                LdFalse
            },
            "ld_true" => {
                self.args0(instr)?;
                LdTrue
            },
            "copy_loc" => {
                let arg = self.args1(instr)?;
                CopyLoc(self.local(instr, arg)?)
            },
            "move_loc" => {
                let arg = self.args1(instr)?;
                MoveLoc(self.local(instr, arg)?)
            },
            "st_loc" => {
                let arg = self.args1(instr)?;
                StLoc(self.local(instr, arg)?)
            },
            "call" => {
                let arg = self.args1(instr)?;
                let (handle_idx, targs_opt) = self.fun_ref(instr, arg)?;
                if let Some(targs) = targs_opt {
                    let res = self.builder.fun_inst_index(handle_idx, targs);
                    let inst_idx = self.add_diags(instr.loc, res)?;
                    CallGeneric(inst_idx)
                } else {
                    Call(handle_idx)
                }
            },
            // TODO: struct and enum instructions
            "read_ref" => {
                self.args0(instr)?;
                ReadRef
            },
            "write_ref" => {
                self.args0(instr)?;
                WriteRef
            },
            "freeze_ref" => {
                self.args0(instr)?;
                FreezeRef
            },
            "mut_borrow_loc" => {
                let arg = self.args1(instr)?;
                MutBorrowLoc(self.local(instr, arg)?)
            },
            "imm_borrow_loc" => {
                let arg = self.args1(instr)?;
                ImmBorrowLoc(self.local(instr, arg)?)
            },
            "add" => {
                self.args0(instr)?;
                Add
            },
            "sub" => {
                self.args0(instr)?;
                Sub
            },
            "mul" => {
                self.args0(instr)?;
                Mul
            },
            "mod" => {
                self.args0(instr)?;
                Mod
            },
            "div" => {
                self.args0(instr)?;
                Div
            },
            "bit_or" => {
                self.args0(instr)?;
                BitOr
            },
            "bit_and" => {
                self.args0(instr)?;
                BitAnd
            },
            "xor" => {
                self.args0(instr)?;
                Xor
            },
            "or" => {
                self.args0(instr)?;
                Or
            },
            "and" => {
                self.args0(instr)?;
                And
            },
            "not" => {
                self.args0(instr)?;
                Not
            },
            "eq" => {
                self.args0(instr)?;
                Eq
            },
            "neq" => {
                self.args0(instr)?;
                Neq
            },
            "lt" => {
                self.args0(instr)?;
                Lt
            },
            "gt" => {
                self.args0(instr)?;
                Gt
            },
            "le" => {
                self.args0(instr)?;
                Le
            },
            "ge" => {
                self.args0(instr)?;
                Ge
            },
            "abort" => {
                self.args0(instr)?;
                Abort
            },
            "nop" => {
                self.args0(instr)?;
                Nop
            },
            // TODO: resource operations
            "shl" => {
                self.args0(instr)?;
                Shl
            },
            "shr" => {
                self.args0(instr)?;
                Shr
            },
            "vec_pack" => {
                let [arg1, arg2] = self.args2(instr)?;
                let sign_idx = self.type_index(instr, arg1)?;
                let count = self.number(instr, arg2, U256::from(u64::MAX))?;
                VecPack(sign_idx, count.unchecked_as_u64())
            },
            "vec_len" => {
                let arg = self.args1(instr)?;
                let sign_idx = self.type_index(instr, arg)?;
                VecLen(sign_idx)
            },
            "vec_imm_borrow" => {
                let arg = self.args1(instr)?;
                let sign_idx = self.type_index(instr, arg)?;
                VecImmBorrow(sign_idx)
            },
            "vec_mut_borrow" => {
                let arg = self.args1(instr)?;
                let sign_idx = self.type_index(instr, arg)?;
                VecMutBorrow(sign_idx)
            },
            "vec_push_back" => {
                let arg = self.args1(instr)?;
                let sign_idx = self.type_index(instr, arg)?;
                VecPushBack(sign_idx)
            },
            "vec_pop_back" => {
                let arg = self.args1(instr)?;
                let sign_idx = self.type_index(instr, arg)?;
                VecPopBack(sign_idx)
            },
            "vec_unpack" => {
                let [arg1, arg2] = self.args2(instr)?;
                let sign_idx = self.type_index(instr, arg1)?;
                let count = self.number(instr, arg2, U256::from(u64::MAX))?;
                VecUnpack(sign_idx, count.unchecked_as_u64())
            },
            "vec_swap" => {
                let arg = self.args1(instr)?;
                let sign_idx = self.type_index(instr, arg)?;
                VecSwap(sign_idx)
            },
            "pack_closure" => {
                let [arg1, arg2] = self.args2(instr)?;
                let (handle_idx, targs_opt) = self.fun_ref(instr, arg1)?;
                let closure_mask = ClosureMask::new(
                    self.number(instr, arg2, U256::from(u64::MAX))?
                        .unchecked_as_u64(),
                );
                if let Some(targs) = targs_opt {
                    let res = self.builder.fun_inst_index(handle_idx, targs);
                    let inst_idx = self.add_diags(instr.loc, res)?;
                    PackClosureGeneric(inst_idx, closure_mask)
                } else {
                    PackClosure(handle_idx, closure_mask)
                }
            },
            "call_closure" => {
                let arg = self.args1(instr)?;
                let sign_idx = self.type_index(instr, arg)?;
                CallClosure(sign_idx)
            },
            _ => {
                self.error(instr.loc, format!("unknown instruction `{}`", instr.name));
                return None;
            },
        };
        Some(instr)
    }

    fn simple_id(&mut self, instr: &Instruction, arg: &Argument) -> Option<Identifier> {
        match arg {
            Argument::Id(pid, None) if pid.address.is_none() && pid.id_parts.len() == 1 => {
                Some(pid.id_parts[0].clone())
            },
            _ => {
                self.error(instr.loc, "expected simple identifier");
                None
            },
        }
    }

    fn fun_ref(
        &mut self,
        instr: &Instruction,
        arg: &Argument,
    ) -> Option<(FunctionHandleIndex, Option<Vec<SignatureToken>>)> {
        if let Argument::Id(pid, targs) = arg {
            let res = self.builder.resolve_fun(&pid.address, &pid.id_parts);
            let idx = self.add_diags(instr.loc, res)?;
            let targs = targs.as_ref().map(|tys| {
                tys.iter()
                    .map(|ty| self.build_type(instr.loc, ty))
                    .collect()
            });
            Some((idx, targs))
        } else {
            self.error(instr.loc, "expected function name");
            None
        }
    }

    fn local(&mut self, instr: &Instruction, arg: &Argument) -> Option<LocalIndex> {
        let id = self.simple_id(instr, arg)?;
        if let Some((idx, _)) = self.require_fun().local_map.get(id.as_str()) {
            Some(*idx)
        } else {
            self.error(instr.loc, format!("unknown local `{}`", id));
            None
        }
    }

    fn number(&mut self, instr: &Instruction, arg: &Argument, max: U256) -> Option<U256> {
        if let Argument::Constant(Value::Number(n)) = arg {
            if *n <= max {
                Some(*n)
            } else {
                self.error(
                    instr.loc,
                    format!("number {} out of range (max {})", n, max),
                );
                None
            }
        } else {
            self.error(instr.loc, "expected number argument");
            None
        }
    }

    fn type_(&mut self, instr: &Instruction, arg: &Argument) -> Option<SignatureToken> {
        if let Argument::Type(ty) = arg {
            Some(self.build_type(instr.loc, ty))
        } else {
            self.error(instr.loc, "expected type argument");
            None
        }
    }

    fn type_index(&mut self, instr: &Instruction, arg: &Argument) -> Option<SignatureIndex> {
        let ty = self.type_(instr, arg)?;
        let res = self.builder.signature_index(vec![ty]);
        self.add_diags(instr.loc, res)
    }

    fn value_bcs(
        &mut self,
        instr: &Instruction,
        arg: &Argument,
        _ty: &SignatureToken,
    ) -> Option<Vec<u8>> {
        if let Argument::Constant(Value::Bytes(bytes)) = arg {
            Some(bytes.clone())
        } else {
            self.error(instr.loc, "expected byte blob");
            None
        }
    }

    fn args0(&mut self, instr: &Instruction) -> Option<()> {
        if instr.args.is_empty() {
            Some(())
        } else {
            self.error(
                instr.loc,
                format!(
                    "expected zero but found {} arguments for instruction `{}`",
                    instr.args.len(),
                    instr.name
                ),
            );
            None
        }
    }

    fn args1<'i>(&mut self, instr: &'i Instruction) -> Option<&'i Argument> {
        if instr.args.len() == 1 {
            Some(&instr.args[0])
        } else {
            self.error(
                instr.loc,
                format!(
                    "expected 1 but found {} arguments for instruction `{}`",
                    instr.args.len(),
                    instr.name
                ),
            );
            None
        }
    }

    fn args2<'i>(&mut self, instr: &'i Instruction) -> Option<[&'i Argument; 2]> {
        if instr.args.len() == 2 {
            Some([&instr.args[0], &instr.args[1]])
        } else {
            self.error(
                instr.loc,
                format!(
                    "expected 1 but found {} arguments for instruction `{}`",
                    instr.args.len(),
                    instr.name
                ),
            );
            None
        }
    }

    /// Report error
    fn error(&mut self, loc: Loc, msg: impl ToString) {
        self.diags.append(&mut syntax::error(loc, msg))
    }

    /// Convert anyhow error to diagnostics in the compiler instance.
    fn add_diags<T>(&mut self, loc: Loc, res: anyhow::Result<T>) -> Option<T> {
        match res {
            Err(e) => {
                self.error(loc, e.to_string());
                None
            },
            Ok(x) => Some(x),
        }
    }
}
