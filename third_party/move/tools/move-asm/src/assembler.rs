// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implements the assembler from the abstract syntax into bytecode,
//! using the `ModuleBuilder`.

use crate::{
    module_builder::{ModuleBuilder, ModuleBuilderOptions},
    syntax,
    syntax::{
        map_diag, Argument, AsmResult, Decl, Diag, Fun, Instruction, Loc, PartialIdent, Struct,
        StructLayout, Type, Unit, UnitId,
    },
};
use anyhow::{anyhow, bail};
use clap::Parser;
use codespan_reporting::{
    files::{Files, SimpleFile},
    term,
    term::{termcolor, termcolor::WriteColor},
};
use either::Either;
use move_binary_format::{
    file_format::{
        Bytecode, CodeOffset, CompiledScript, FieldDefinition, FieldHandleIndex,
        FieldInstantiationIndex, FunctionDefinitionIndex, FunctionHandleIndex, LocalIndex,
        MemberCount, SignatureIndex, SignatureToken, StructDefInstantiationIndex,
        StructDefinitionIndex, StructFieldInformation, StructVariantHandleIndex,
        StructVariantInstantiationIndex, TableIndex, TypeSignature, VariantDefinition,
        VariantFieldHandleIndex, VariantFieldInstantiationIndex, VariantIndex,
    },
    CompiledModule,
};
use move_core_types::{function::ClosureMask, identifier::Identifier, u256::U256};
use std::{collections::BTreeMap, fs, io::Write, path::PathBuf};
// ===================================================================================
// Driver

pub type ModuleOrScript = Either<CompiledModule, CompiledScript>;

#[derive(Parser, Clone, Debug, Default)]
#[clap(author, version, about)]
pub struct Options {
    /// Options for the module builder
    #[clap(flatten)]
    pub module_builder_options: ModuleBuilderOptions,

    /// List of files with binary dependencies
    #[clap(short, long)]
    pub deps: Vec<String>,

    /// Directory where to place assembled code.
    #[clap(short, long, default_value = "")]
    pub output_dir: String,

    /// Input file.
    pub inputs: Vec<String>,
}

/// Assembles source as specified by options.
pub fn run<W>(options: Options, error_writer: &mut W) -> anyhow::Result<()>
where
    W: Write + WriteColor,
{
    if options.inputs.len() != 1 {
        bail!("expected exactly one file name for the assembler source")
    }
    let input_path = options.inputs.first().unwrap();
    let input = fs::read_to_string(input_path)?;

    let context_modules = options
        .deps
        .iter()
        .map(|file| {
            let bytes = fs::read(file).map_err(|e| anyhow!(e))?;
            CompiledModule::deserialize(&bytes).map_err(|e| anyhow!(e))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let result = match assemble(&options, &input, context_modules.iter()) {
        Ok(x) => x,
        Err(diags) => {
            let diag_file = SimpleFile::new(&input_path, &input);
            report_diags(error_writer, &diag_file, diags);
            bail!("exiting with errors")
        },
    };

    let path = PathBuf::from(input_path).with_extension("mv");
    let mut out_path = PathBuf::from(options.output_dir);
    out_path.push(path.file_name().expect("file name available"));
    let mut bytes = vec![];
    match result {
        Either::Left(m) => m
            .serialize_for_version(
                Some(options.module_builder_options.bytecode_version),
                &mut bytes,
            )
            .expect("serialization succeeds"),
        Either::Right(s) => s
            .serialize_for_version(
                Some(options.module_builder_options.bytecode_version),
                &mut bytes,
            )
            .expect("serialization succeeds"),
    }
    if let Err(e) = fs::write(&out_path, &bytes) {
        bail!("failed to write result to `{}`: {}", out_path.display(), e);
    }
    Ok(())
}

pub fn assemble<'a>(
    options: &Options,
    input: &str,
    context_modules: impl Iterator<Item = &'a CompiledModule>,
) -> AsmResult<ModuleOrScript> {
    let ast = syntax::parse_asm(input)?;
    compile(options.module_builder_options.clone(), context_modules, ast)
}

pub fn diag_to_string(file_name: &str, source: &str, diags: Vec<Diag>) -> String {
    let files = SimpleFile::new(file_name, source);
    let mut error_writer = termcolor::Buffer::no_color();
    report_diags(&mut error_writer, &files, diags);
    String::from_utf8_lossy(&error_writer.into_inner()).to_string()
}

pub(crate) fn report_diags<'a, W: Write + WriteColor>(
    error_writer: &mut W,
    files: &'a impl Files<'a, FileId = ()>,
    diags: Vec<Diag>,
) {
    for diag in diags {
        term::emit(error_writer, &term::Config::default(), files, &diag)
            .unwrap_or_else(|_| eprintln!("failed to print diagnostics"))
    }
}

// ===================================================================================
// Logic

struct Assembler<'a> {
    builder: ModuleBuilder<'a>,
    diags: Vec<Diag>,
    /// Context available during processing of a function.
    resolution_context: Option<ResolutionContext>,
}

struct ResolutionContext {
    ty_param_map: BTreeMap<String, u16>,
    local_map: BTreeMap<String, (LocalIndex, SignatureToken)>,
}

fn compile<'a>(
    options: ModuleBuilderOptions,
    context_modules: impl IntoIterator<Item = &'a CompiledModule>,
    ast: Unit,
) -> AsmResult<ModuleOrScript> {
    let mut compiler = Assembler {
        builder: ModuleBuilder::new(options, context_modules, ast.name.module_opt()),
        diags: vec![],
        resolution_context: None,
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
            friend_modules,
            structs,
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

        // Register friend modules
        for m in friend_modules {
            let res = self.builder.declare_friend_module(m);
            self.add_diags(Loc::new(0, 0), res);
        }

        // Declare structs
        for str in structs {
            self.declare_struct(str)
        }

        // Declare functions
        for fun in functions {
            self.declare_fun(fun);
        }

        if self.diags.is_empty() {
            // Define layout for structs
            for (pos, str) in structs.iter().enumerate() {
                self.define_struct(StructDefinitionIndex::new(pos as TableIndex), str)
            }

            // Define code for functions
            for (pos, fun) in functions.iter().enumerate() {
                self.define_fun(FunctionDefinitionIndex::new(pos as TableIndex), fun)
            }
        }
    }

    fn declare_struct(&mut self, str: &Struct) {
        self.setup_struct(str);
        let res = self.builder.declare_struct(
            str.name.clone(),
            str.type_params
                .iter()
                .map(|(_, constraints, is_phantom)| (*constraints, *is_phantom))
                .collect(),
            str.abilities,
        );
        self.add_diags(str.loc, res);
    }

    fn define_struct(&mut self, idx: StructDefinitionIndex, str: &Struct) {
        self.setup_struct(str);
        let layout = match &str.layout {
            StructLayout::Singleton(fields) => {
                StructFieldInformation::Declared(self.translate_fields(fields))
            },
            StructLayout::Variants(variants) => {
                let mut result = vec![];
                for (loc, name, fields) in variants {
                    let name_res = self.builder.name_index(name.clone());
                    if let Some(name) = self.add_diags(*loc, name_res) {
                        result.push(VariantDefinition {
                            name,
                            fields: self.translate_fields(fields),
                        })
                    }
                }
                StructFieldInformation::DeclaredVariants(result)
            },
        };
        self.builder.define_struct_layout(idx, layout)
    }

    fn translate_fields(&mut self, fields: &[Decl]) -> Vec<FieldDefinition> {
        let mut result = vec![];
        for field in fields {
            let name_res = self.builder.name_index(field.name.clone());
            if let Some(name) = self.add_diags(field.loc, name_res) {
                result.push(FieldDefinition {
                    name,
                    signature: TypeSignature(self.build_type(field.loc, &field.ty)),
                })
            }
        }
        result
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
        let acquires_res = self.acquires(fun.acquires.iter());
        let acquires = self.add_diags(fun.loc, acquires_res).unwrap_or_default();
        let res = self.builder.declare_fun(
            fun.is_entry,
            fun.name.clone(),
            fun.visibility,
            fun.attributes.clone(),
            param_sign,
            result_sign,
            fun.type_params
                .iter()
                .map(|(_, abilities)| *abilities)
                .collect(),
            acquires,
        );
        self.add_diags(fun.loc, res);
    }

    fn acquires<'b>(
        &mut self,
        ids: impl Iterator<Item = &'b Identifier>,
    ) -> anyhow::Result<Vec<StructDefinitionIndex>> {
        ids.map(|id| self.builder.resolve_struct_def(id.as_ident_str()))
            .collect::<anyhow::Result<Vec<_>>>()
    }

    fn setup_fun(&mut self, fun: &Fun) {
        self.setup_type_params(fun.type_params.iter().map(|(id, _)| id));
        self.resolution_context.as_mut().unwrap().local_map = fun
            .params
            .iter()
            .chain(fun.locals.iter())
            .enumerate()
            .map(|(pos, Decl { loc, name, ty })| {
                let ty = self.build_type(*loc, ty);
                (name.to_string(), (pos as LocalIndex, ty))
            })
            .collect();
    }

    fn setup_struct(&mut self, str: &Struct) {
        self.setup_type_params(str.type_params.iter().map(|(name, _, _)| name))
    }

    fn setup_type_params<'b>(&mut self, params: impl Iterator<Item = &'b Identifier>) {
        let ty_param_map = params
            .enumerate()
            .map(|(pos, id)| (id.to_string(), pos as u16))
            .collect();
        self.resolution_context = Some(ResolutionContext {
            ty_param_map,
            local_map: Default::default(),
        });
    }

    fn require_resolution_context(&self) -> &ResolutionContext {
        self.resolution_context
            .as_ref()
            .expect("resolution context")
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
                    .require_resolution_context()
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
        let tr_named =
            |comp: &mut Self, name: &PartialIdent, inst: Option<&Vec<Type>>| -> SignatureToken {
                let res = comp.builder.resolve_struct(&name.address, &name.id_parts);
                if let Some(shdl_idx) = comp.add_diags(loc, res) {
                    match tr_inst(comp, inst) {
                        Some(tys) => SignatureToken::StructInstantiation(shdl_idx, tys),
                        None => SignatureToken::Struct(shdl_idx),
                    }
                } else {
                    // error reported
                    SignatureToken::Bool
                }
            };
        match ty {
            Type::Named(partial_id, opt_inst) => {
                if partial_id.address.is_none() && partial_id.id_parts.len() == 1 {
                    match partial_id.id_parts[0].as_str() {
                        s if self
                            .require_resolution_context()
                            .ty_param_map
                            .contains_key(s) =>
                        {
                            SignatureToken::TypeParameter(
                                self.require_resolution_context().ty_param_map[s],
                            )
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
                        _ => tr_named(self, partial_id, opt_inst.as_ref()),
                    }
                } else {
                    tr_named(self, partial_id, opt_inst.as_ref())
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
                let label = self.simple_id(instr, arg, " for label")?;
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
                if let Argument::Constant(val) = arg2 {
                    let move_value = self.add_diags(instr.loc, val.to_move_value(&ty))?;
                    let bcs = move_value
                        .simple_serialize()
                        .expect("value serialization succeeds");
                    let idx = self.add_diags(instr.loc, self.builder.const_index(bcs, ty))?;
                    LdConst(idx)
                } else {
                    self.error(instr.loc, "expected a constant value");
                    return None;
                }
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
            "borrow_loc" => {
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
            "vec_borrow" => {
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
            "pack" | "unpack" => {
                let (gen_op, op): (
                    fn(StructDefInstantiationIndex) -> Bytecode,
                    fn(StructDefinitionIndex) -> Bytecode,
                ) = match instr_name.as_str() {
                    "pack" => (PackGeneric, Pack),
                    "unpack" => (UnpackGeneric, Unpack),
                    _ => unreachable!(),
                };
                let arg = self.args1(instr)?;
                let (def_idx, targs_opt) = self.struct_ref(instr, arg)?;
                if let Some(targs) = targs_opt {
                    let res = self.builder.struct_def_inst_index(def_idx, targs);
                    let inst_idx = self.add_diags(instr.loc, res)?;
                    gen_op(inst_idx)
                } else {
                    op(def_idx)
                }
            },
            "pack_variant" | "unpack_variant" | "test_variant" => {
                let (gen_op, op): (
                    fn(StructVariantInstantiationIndex) -> Bytecode,
                    fn(StructVariantHandleIndex) -> Bytecode,
                ) = match instr_name.as_str() {
                    "pack_variant" => (PackVariantGeneric, PackVariant),
                    "unpack_variant" => (UnpackVariantGeneric, UnpackVariant),
                    "test_variant" => (TestVariantGeneric, TestVariant),
                    _ => unreachable!(),
                };
                let [arg1, arg2] = self.args2(instr)?;
                let (def_idx, targs_opt) = self.struct_ref(instr, arg1)?;
                let variant_idx = self.struct_variant(instr, def_idx, arg2)?;
                if let Some(targs) = targs_opt {
                    let inst_idx = self.add_diags(
                        instr.loc,
                        self.builder.variant_inst_index(variant_idx, targs),
                    )?;
                    gen_op(inst_idx)
                } else {
                    op(variant_idx)
                }
            },
            "borrow_field" | "mut_borrow_field" => {
                let (gen_op, op): (
                    fn(FieldInstantiationIndex) -> Bytecode,
                    fn(FieldHandleIndex) -> Bytecode,
                ) = match instr_name.as_str() {
                    "borrow_field" => (ImmBorrowFieldGeneric, ImmBorrowField),
                    "mut_borrow_field" => (MutBorrowFieldGeneric, MutBorrowField),
                    _ => unreachable!(),
                };
                let [arg1, arg2] = self.args2(instr)?;
                let (def_idx, targs_opt) = self.struct_ref(instr, arg1)?;
                let field_name = self.simple_id(instr, arg2, " for field")?;
                let field_offs = self.add_diags(
                    instr.loc,
                    self.builder
                        .resolve_field(def_idx, None, field_name.as_ident_str()),
                )?;
                let hdl_idx =
                    self.add_diags(instr.loc, self.builder.field_index(def_idx, field_offs))?;
                if let Some(targs) = targs_opt {
                    let inst_idx =
                        self.add_diags(instr.loc, self.builder.field_inst_index(hdl_idx, targs))?;
                    gen_op(inst_idx)
                } else {
                    op(hdl_idx)
                }
            },
            "borrow_variant_field" | "mut_borrow_variant_field" => {
                let (gen_op, op): (
                    fn(VariantFieldInstantiationIndex) -> Bytecode,
                    fn(VariantFieldHandleIndex) -> Bytecode,
                ) = match instr_name.as_str() {
                    "borrow_variant_field" => (ImmBorrowVariantFieldGeneric, ImmBorrowVariantField),
                    "mut_borrow_variant_field" => {
                        (MutBorrowVariantFieldGeneric, MutBorrowVariantField)
                    },
                    _ => unreachable!(),
                };
                if instr.args.len() < 2 {
                    self.error(
                        instr.loc,
                        "expected at least 2 arguments for variant field borrow",
                    );
                    return None;
                }
                let (def_idx, targs_opt) = self.struct_ref(instr, &instr.args[0])?;

                let (variants, field_offs) = self.variants(instr, def_idx)?;
                let hdl_idx = self.add_diags(
                    instr.loc,
                    self.builder
                        .variant_field_index(def_idx, variants, field_offs),
                )?;
                if let Some(targs) = targs_opt {
                    let inst_idx = self.add_diags(
                        instr.loc,
                        self.builder.variant_field_inst_index(hdl_idx, targs),
                    )?;
                    gen_op(inst_idx)
                } else {
                    op(hdl_idx)
                }
            },
            "borrow_global" | "mut_borrow_global" | "exists" | "move_from" | "move_to" => {
                let (gen_op, op): (
                    fn(StructDefInstantiationIndex) -> Bytecode,
                    fn(StructDefinitionIndex) -> Bytecode,
                ) = match instr_name.as_str() {
                    "borrow_global" => (ImmBorrowGlobalGeneric, ImmBorrowGlobal),
                    "mut_borrow_global" => (MutBorrowGlobalGeneric, MutBorrowGlobal),
                    "exists" => (ExistsGeneric, Exists),
                    "move_from" => (MoveFromGeneric, MoveFrom),
                    "move_to" => (MoveToGeneric, MoveTo),
                    _ => unreachable!(),
                };
                let arg = self.args1(instr)?;
                let (def_idx, targs_opt) = self.struct_ref(instr, arg)?;
                if let Some(targs) = targs_opt {
                    let inst_idx = self.add_diags(
                        instr.loc,
                        self.builder.struct_def_inst_index(def_idx, targs),
                    )?;
                    gen_op(inst_idx)
                } else {
                    op(def_idx)
                }
            },
            _ => {
                self.error(instr.loc, format!("unknown instruction `{}`", instr.name));
                return None;
            },
        };
        Some(instr)
    }

    fn variants(
        &mut self,
        instr: &Instruction,
        def_idx: StructDefinitionIndex,
    ) -> Option<(Vec<VariantIndex>, MemberCount)> {
        let mut variants = vec![];
        let mut field_offs = None;
        for field in &instr.args[1..] {
            match field {
                Argument::Id(
                    PartialIdent {
                        address: None,
                        id_parts,
                    },
                    None,
                ) if id_parts.len() == 2 => {
                    let variant_idx = self.add_diags(
                        instr.loc,
                        self.builder
                            .resolve_variant(def_idx, id_parts[0].as_ident_str()),
                    )?;
                    variants.push(variant_idx);
                    let offs = self.add_diags(
                        instr.loc,
                        self.builder.resolve_field(
                            def_idx,
                            Some(variant_idx),
                            id_parts[1].as_ident_str(),
                        ),
                    )?;
                    if field_offs.map(|cur| cur == offs).unwrap_or(true) {
                        field_offs = Some(offs)
                    } else {
                        self.error(
                            instr.loc,
                            format!(
                                "variants of fields must be \
                        at some position, previous was {} while this is {}",
                                field_offs.unwrap(),
                                offs
                            ),
                        );
                        return None;
                    }
                },
                _ => {
                    self.error(
                        instr.loc,
                        "expected `<variant>::<field>` to describe field of variant",
                    );
                    return None;
                },
            }
        }
        Some((variants, field_offs?))
    }

    fn simple_id(&mut self, instr: &Instruction, arg: &Argument, ctx: &str) -> Option<Identifier> {
        match arg {
            Argument::Id(pid, None) if pid.address.is_none() && pid.id_parts.len() == 1 => {
                Some(pid.id_parts[0].clone())
            },
            _ => {
                self.error(instr.loc, format!("expected simple identifier{}", ctx));
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

    fn struct_ref(
        &mut self,
        instr: &Instruction,
        arg: &Argument,
    ) -> Option<(StructDefinitionIndex, Option<Vec<SignatureToken>>)> {
        match arg {
            Argument::Id(
                PartialIdent {
                    address: None,
                    id_parts,
                },
                targs,
            ) if id_parts.len() == 1 => {
                let res = self.builder.resolve_struct_def(&id_parts[0]);
                let idx = self.add_diags(instr.loc, res)?;
                let targs = targs.as_ref().map(|tys| {
                    tys.iter()
                        .map(|ty| self.build_type(instr.loc, ty))
                        .collect()
                });
                Some((idx, targs))
            },
            _ => {
                self.error(
                    instr.loc,
                    "expected simple struct name with optional type instantiation",
                );
                None
            },
        }
    }

    fn struct_variant(
        &mut self,
        instr: &Instruction,
        def_idx: StructDefinitionIndex,
        variant: &Argument,
    ) -> Option<StructVariantHandleIndex> {
        let name = self.simple_id(instr, variant, " for variant")?;
        let res = self.builder.resolve_variant(def_idx, name.as_ident_str());
        let variant_idx = self.add_diags(instr.loc, res)?;
        let res = self.builder.variant_index(def_idx, variant_idx);
        self.add_diags(instr.loc, res)
    }

    fn local(&mut self, instr: &Instruction, arg: &Argument) -> Option<LocalIndex> {
        let id = self.simple_id(instr, arg, " for local")?;
        if let Some((idx, _)) = self.require_resolution_context().local_map.get(id.as_str()) {
            Some(*idx)
        } else {
            self.error(instr.loc, format!("unknown local `{}`", id));
            None
        }
    }

    fn number(&mut self, instr: &Instruction, arg: &Argument, max: U256) -> Option<U256> {
        if let Argument::Constant(val) = arg {
            self.add_diags(instr.loc, val.check_number(max))
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
