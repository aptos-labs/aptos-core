// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! This module translates the bytecode of a module to Boogie code.

use crate::{
    boogie_helpers::{
        boogie_address, boogie_address_blob, boogie_bv_type, boogie_byte_blob,
        boogie_closure_pack_name, boogie_constant_blob, boogie_debug_track_abort,
        boogie_debug_track_local, boogie_debug_track_return, boogie_equality_for_type,
        boogie_field_sel, boogie_field_update, boogie_fun_apply_name, boogie_function_bv_name,
        boogie_function_name, boogie_make_vec_from_strings, boogie_modifies_memory_name,
        boogie_num_literal, boogie_num_type_base, boogie_num_type_string_capital,
        boogie_reflection_type_info, boogie_reflection_type_name, boogie_resource_memory_name,
        boogie_struct_name, boogie_struct_variant_name, boogie_temp, boogie_temp_from_suffix,
        boogie_type, boogie_type_for_struct_field, boogie_type_param, boogie_type_suffix,
        boogie_type_suffix_bv, boogie_type_suffix_for_struct,
        boogie_type_suffix_for_struct_variant, boogie_variant_field_update,
        boogie_well_formed_check, boogie_well_formed_expr_bv, field_bv_flag_global_state,
        TypeIdentToken,
    },
    options::BoogieOptions,
    spec_translator::SpecTranslator,
};
use codespan::LineIndex;
use codespan_reporting::diagnostic::Severity;
use itertools::Itertools;
use legacy_move_compiler::interface_generator::NATIVE_INTERFACE;
#[allow(unused_imports)]
use log::{debug, info, log, warn, Level};
use move_model::{
    ast::{Attribute, TempIndex, TraceKind},
    code_writer::CodeWriter,
    emit, emitln,
    model::{FieldEnv, FunctionEnv, GlobalEnv, Loc, NodeId, QualifiedInstId, StructEnv, StructId},
    pragmas::{
        ADDITION_OVERFLOW_UNCHECKED_PRAGMA, SEED_PRAGMA, TIMEOUT_PRAGMA,
        VERIFY_DURATION_ESTIMATE_PRAGMA,
    },
    symbol::Symbol,
    ty::{PrimitiveType, Type, TypeDisplayContext, BOOL_TYPE},
    well_known::{TYPE_INFO_MOVE, TYPE_NAME_GET_MOVE, TYPE_NAME_MOVE},
};
use move_prover_bytecode_pipeline::{
    mono_analysis,
    mono_analysis::ClosureInfo,
    number_operation::{
        FuncOperationMap, GlobalNumberOperationState, NumOperation,
        NumOperation::{Bitwise, Bottom},
    },
    options::ProverOptions,
};
use move_stackless_bytecode::{
    function_target::FunctionTarget,
    function_target_pipeline::{FunctionTargetsHolder, FunctionVariant, VerificationFlavor},
    stackless_bytecode::{
        AbortAction, AttrId, BorrowEdge, BorrowNode, Bytecode, Constant, HavocKind, IndexEdgeKind,
        Operation, PropKind,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    hash::{DefaultHasher, Hash, Hasher},
};

pub struct BoogieTranslator<'env> {
    env: &'env GlobalEnv,
    options: &'env BoogieOptions,
    for_shard: Option<usize>,
    writer: &'env CodeWriter,
    spec_translator: SpecTranslator<'env>,
    targets: &'env FunctionTargetsHolder,
}

pub struct FunctionTranslator<'env> {
    parent: &'env BoogieTranslator<'env>,
    fun_target: &'env FunctionTarget<'env>,
    type_inst: &'env [Type],
}

pub struct StructTranslator<'env> {
    parent: &'env BoogieTranslator<'env>,
    struct_env: &'env StructEnv<'env>,
    type_inst: &'env [Type],
}

impl<'env> BoogieTranslator<'env> {
    pub fn new(
        env: &'env GlobalEnv,
        options: &'env BoogieOptions,
        for_shard: Option<usize>,
        targets: &'env FunctionTargetsHolder,
        writer: &'env CodeWriter,
    ) -> Self {
        Self {
            env,
            options,
            for_shard,
            targets,
            writer,
            spec_translator: SpecTranslator::new(writer, env, options),
        }
    }

    fn get_timeout(&self, fun_target: &FunctionTarget) -> usize {
        let options = self.options;
        let estimate_timeout_opt = fun_target
            .func_env
            .get_num_pragma(VERIFY_DURATION_ESTIMATE_PRAGMA);
        let default_timeout = if options.global_timeout_overwrite {
            estimate_timeout_opt.unwrap_or(options.vc_timeout)
        } else {
            options.vc_timeout
        };
        fun_target
            .func_env
            .get_num_pragma(TIMEOUT_PRAGMA)
            .unwrap_or(default_timeout)
    }

    /// Checks whether the given function is a verification target.
    fn is_verified(&self, fun_variant: &FunctionVariant, fun_target: &FunctionTarget) -> bool {
        if !fun_variant.is_verified() {
            return false;
        }
        if let Some(shard) = self.for_shard {
            // Check whether the shard is included.
            if self.options.only_shard.is_some() && self.options.only_shard != Some(shard + 1) {
                return false;
            }
            // Check whether it is part of the shard.
            let mut hasher = DefaultHasher::new();
            fun_target.func_env.get_full_name_str().hash(&mut hasher);
            if (hasher.finish() as usize) % self.options.shards != shard {
                return false;
            }
        }
        // Check whether the estimated duration is too large for configured timeout
        let options = self.options;
        let estimate_timeout_opt = fun_target
            .func_env
            .get_num_pragma(VERIFY_DURATION_ESTIMATE_PRAGMA);
        if let Some(estimate_timeout) = estimate_timeout_opt {
            let timeout = fun_target
                .func_env
                .get_num_pragma(TIMEOUT_PRAGMA)
                .unwrap_or(options.vc_timeout);
            estimate_timeout <= timeout
        } else {
            true
        }
    }

    #[allow(clippy::literal_string_with_formatting_args)]
    fn emit_function(&self, writer: &CodeWriter, signature: &str, body_fn: impl Fn()) {
        self.emit_function_with_attr(writer, "{:inline} ", signature, body_fn)
    }

    #[allow(clippy::literal_string_with_formatting_args)]
    fn emit_procedure(&self, writer: &CodeWriter, signature: &str, body_fn: impl Fn()) {
        self.emit_procedure_with_attr(writer, "{:inline 1} ", signature, body_fn)
    }

    fn emit_with_attr(
        &self,
        writer: &CodeWriter,
        sig: &str,
        attr: &str,
        signature: &str,
        body_fn: impl Fn(),
    ) {
        emitln!(writer, "{} {}{} {{", sig, attr, signature);
        writer.indent();
        body_fn();
        writer.unindent();
        emitln!(writer, "}");
    }

    fn emit_function_with_attr(
        &self,
        writer: &CodeWriter,
        attr: &str,
        signature: &str,
        body_fn: impl Fn(),
    ) {
        self.emit_with_attr(writer, "function", attr, signature, body_fn)
    }

    fn emit_procedure_with_attr(
        &self,
        writer: &CodeWriter,
        attr: &str,
        signature: &str,
        body_fn: impl Fn(),
    ) {
        self.emit_with_attr(writer, "procedure", attr, signature, body_fn)
    }

    #[allow(clippy::literal_string_with_formatting_args)]
    pub fn translate(&mut self) {
        let writer = self.writer;
        let env = self.env;
        let spec_translator = &self.spec_translator;

        let mono_info = mono_analysis::get_info(self.env);
        let empty = &BTreeSet::new();

        emitln!(
            writer,
            "\n\n//==================================\n// Begin Translation\n"
        );

        // Add type reflection axioms
        if !mono_info.type_params.is_empty() {
            emitln!(writer, "function $TypeName(t: $TypeParamInfo): Vec int;");

            // type name <-> type info: primitives
            for name in [
                "Bool", "U8", "U16", "U32", "U64", "U128", "U256", "Address", "Signer",
            ]
            .into_iter()
            {
                emitln!(
                    writer,
                    "axiom (forall t: $TypeParamInfo :: {{$TypeName(t)}} \
                            t is $TypeParam{} ==> $IsEqual'vec'u8''($TypeName(t), {}));",
                    name,
                    TypeIdentToken::convert_to_bytes(TypeIdentToken::make(&name.to_lowercase()))
                );
                emitln!(
                    writer,
                    "axiom (forall t: $TypeParamInfo :: {{$TypeName(t)}} \
                            $IsEqual'vec'u8''($TypeName(t), {}) ==> t is $TypeParam{});",
                    TypeIdentToken::convert_to_bytes(TypeIdentToken::make(&name.to_lowercase())),
                    name,
                );
            }

            // type name <-> type info: vector
            let mut tokens = TypeIdentToken::make("vector<");
            tokens.push(TypeIdentToken::Variable("$TypeName(t->e)".to_string()));
            tokens.extend(TypeIdentToken::make(">"));
            emitln!(
                writer,
                "axiom (forall t: $TypeParamInfo :: {{$TypeName(t)}} \
                            t is $TypeParamVector ==> $IsEqual'vec'u8''($TypeName(t), {}));",
                TypeIdentToken::convert_to_bytes(tokens)
            );
            // TODO(mengxu): this will parse it to an uninterpreted vector element type
            emitln!(
                writer,
                "axiom (forall t: $TypeParamInfo :: {{$TypeName(t)}} \
                            ($IsPrefix'vec'u8''($TypeName(t), {}) && $IsSuffix'vec'u8''($TypeName(t), {})) ==> t is $TypeParamVector);",
                TypeIdentToken::convert_to_bytes(TypeIdentToken::make("vector<")),
                TypeIdentToken::convert_to_bytes(TypeIdentToken::make(">")),
            );

            // type name <-> type info: struct
            let mut tokens = TypeIdentToken::make("0x");
            // TODO(mengxu): this is not a correct radix16 encoding of an integer
            tokens.push(TypeIdentToken::Variable("MakeVec1(t->a)".to_string()));
            tokens.extend(TypeIdentToken::make("::"));
            tokens.push(TypeIdentToken::Variable("t->m".to_string()));
            tokens.extend(TypeIdentToken::make("::"));
            tokens.push(TypeIdentToken::Variable("t->s".to_string()));
            emitln!(
                writer,
                "axiom (forall t: $TypeParamInfo :: {{$TypeName(t)}} \
                            t is $TypeParamStruct ==> $IsEqual'vec'u8''($TypeName(t), {}));",
                TypeIdentToken::convert_to_bytes(tokens)
            );
            // TODO(mengxu): this will parse it to an uninterpreted struct
            emitln!(
                writer,
                "axiom (forall t: $TypeParamInfo :: {{$TypeName(t)}} \
                            $IsPrefix'vec'u8''($TypeName(t), {}) ==> t is $TypeParamVector);",
                TypeIdentToken::convert_to_bytes(TypeIdentToken::make("0x")),
            );
        }

        // Add given type declarations for type parameters.
        emitln!(writer, "\n\n// Given Types for Type Parameters\n");
        for idx in &mono_info.type_params {
            let param_type = boogie_type_param(env, *idx);
            let suffix = boogie_type_suffix(env, &Type::TypeParameter(*idx));
            emitln!(writer, "type {};", param_type);
            emitln!(
                writer,
                "function {{:inline}} $IsEqual'{}'(x1: {}, x2: {}): bool {{ x1 == x2 }}",
                suffix,
                param_type,
                param_type
            );
            emitln!(
                writer,
                "function {{:inline}} $IsValid'{}'(x: {}): bool {{ true }}",
                suffix,
                param_type,
            );

            // declare free variables to represent the type info for this type
            emitln!(writer, "var {}_info: $TypeParamInfo;", param_type);

            // declare the memory variable for this type
            emitln!(writer, "var {}_$memory: $Memory {};", suffix, param_type);

            // If cmp module is included, emit cmp functions for generic types
            if env
                .get_modules()
                .any(|m| m.get_full_name_str() == "0x1::cmp")
            {
                self.emit_function(
                    writer,
                    &format!(
                        "$1_cmp_$compare'{}'(v1: {}, v2: {}): $1_cmp_Ordering",
                        suffix, param_type, param_type
                    ),
                    || {
                        emitln!(
                            writer,
                            "if $IsEqual'{}'(v1, v2) then $1_cmp_Ordering_Equal()",
                            suffix
                        );
                        emitln!(writer, "else $Arbitrary_value_of'$1_cmp_Ordering'()");
                    },
                );

                self.emit_procedure(
                    writer,
                    &format!(
                        "$1_cmp_compare'{}'(v1: {}, v2: {}) returns ($ret0: $1_cmp_Ordering)",
                        suffix, param_type, param_type
                    ),
                    || {
                        emitln!(writer, "$ret0 := $1_cmp_$compare'{}'(v1, v2);", suffix);
                    },
                );
            }
        }
        emitln!(writer);

        self.spec_translator
            .translate_axioms(env, mono_info.as_ref());

        let mut translated_types = BTreeSet::new();
        let mut translated_memory = vec![];
        let mut translated_funs = BTreeSet::new();
        let mut verified_functions_count = 0;
        info!("generating verification conditions");
        for module_env in self.env.get_modules() {
            self.writer.set_location(&module_env.env.internal_loc());

            spec_translator.translate_spec_vars(&module_env, mono_info.as_ref());
            spec_translator.translate_spec_funs(&module_env, mono_info.as_ref());

            for ref struct_env in module_env.get_structs() {
                if struct_env.is_intrinsic() {
                    continue;
                }
                for type_inst in mono_info
                    .structs
                    .get(&struct_env.get_qualified_id())
                    .unwrap_or(empty)
                {
                    let struct_name = boogie_struct_name(struct_env, type_inst);
                    if !translated_types.insert(struct_name) {
                        continue;
                    }
                    if struct_env.has_memory() {
                        translated_memory.push(boogie_resource_memory_name(
                            env,
                            &struct_env.get_qualified_id().instantiate(type_inst.clone()),
                            &None,
                        ))
                    }
                    StructTranslator {
                        parent: self,
                        struct_env,
                        type_inst: type_inst.as_slice(),
                    }
                    .translate();
                }
            }

            for ref fun_env in module_env.get_functions() {
                if fun_env.is_native_or_intrinsic() || fun_env.is_inline() || fun_env.is_test_only()
                {
                    continue;
                }
                for (variant, ref fun_target) in self.targets.get_targets(fun_env) {
                    if self.is_verified(&variant, fun_target) {
                        verified_functions_count += 1;
                        debug!(
                            "will verify primary function `{}`",
                            env.display(&module_env.get_id().qualified(fun_target.get_id()))
                        );
                        // Always produce a verified functions with an empty instantiation such that
                        // there is at least one top-level entry points for a VC.
                        FunctionTranslator {
                            parent: self,
                            fun_target,
                            type_inst: &[],
                        }
                        .translate();

                        // There maybe more verification targets that needs to be produced as we
                        // defer the instantiation of verified functions to this stage
                        if !self.options.skip_instance_check {
                            for type_inst in mono_info
                                .funs
                                .get(&(fun_target.func_env.get_qualified_id(), variant))
                                .unwrap_or(empty)
                            {
                                // Skip redundant instantiations. Those are any permutation of
                                // type parameters. We can simple test this by the same number
                                // of disjoint type parameters equals the count of type parameters.
                                // Verifying a disjoint set of type parameters is the same
                                // as the `&[]` verification already done above.
                                let type_params_in_inst = type_inst
                                    .iter()
                                    .filter(|t| matches!(t, Type::TypeParameter(_)))
                                    .cloned()
                                    .collect::<BTreeSet<_>>();
                                if type_params_in_inst.len() == type_inst.len() {
                                    continue;
                                }

                                verified_functions_count += 1;
                                let tctx = &fun_env.get_type_display_ctx();
                                debug!(
                                    "will verify function instantiation `{}`<{}>",
                                    env.display(
                                        &module_env.get_id().qualified(fun_target.get_id())
                                    ),
                                    type_inst
                                        .iter()
                                        .map(|t| t.display(tctx).to_string())
                                        .join(", ")
                                );
                                FunctionTranslator {
                                    parent: self,
                                    fun_target,
                                    type_inst,
                                }
                                .translate();
                            }
                        }
                    } else {
                        // This variant is inlined, so translate for all type instantiations.
                        for type_inst in mono_info
                            .funs
                            .get(&(
                                fun_target.func_env.get_qualified_id(),
                                FunctionVariant::Baseline,
                            ))
                            .unwrap_or(empty)
                        {
                            let fun_name = boogie_function_name(fun_env, type_inst);
                            if !translated_funs.insert(fun_name) {
                                continue;
                            }
                            FunctionTranslator {
                                parent: self,
                                fun_target,
                                type_inst,
                            }
                            .translate();
                        }
                    }
                }
            }
        }

        // Emit function types and closures
        for (fun_type, closure_infos) in &mono_info.fun_infos {
            self.translate_fun_type(fun_type, closure_infos, &translated_memory)
        }

        // Emit any finalization items required by spec translation.
        self.spec_translator.finalize();
        let shard_info = if let Some(shard) = self.for_shard {
            format!(" (for shard #{} of {})", shard + 1, self.options.shards)
        } else {
            "".to_string()
        };
        info!(
            "{} verification conditions{}",
            verified_functions_count, shard_info
        );
    }

    fn translate_fun_type(
        &self,
        fun_type: &Type,
        closure_infos: &BTreeSet<ClosureInfo>,
        memory: &[String],
    ) {
        emitln!(
            self.writer,
            "// Function type `{}`",
            fun_type.display(&self.env.get_type_display_ctx())
        );

        // Create data type for the function type.
        let fun_ty_boogie_name = boogie_type(self.env, fun_type);
        emitln!(self.writer, "datatype {} {{", fun_ty_boogie_name);
        self.writer.indent();
        for info in closure_infos {
            emitln!(
                self.writer,
                "// Closure from function `{}`, capture mask 0b{}",
                self.env.display(&info.fun),
                info.mask
            );
            emit!(
                self.writer,
                "{}(",
                boogie_closure_pack_name(self.env, &info.fun, info.mask)
            );
            let fun_env = self.env.get_function(info.fun.to_qualified_id());
            let param_tys = Type::instantiate_vec(fun_env.get_parameter_types(), &info.fun.inst);
            for (pos, captured_ty) in info.mask.extract(&param_tys, true).iter().enumerate() {
                if pos > 0 {
                    emit!(self.writer, ", ")
                }
                emit!(
                    self.writer,
                    "p{}: {}",
                    pos,
                    boogie_type(self.env, captured_ty)
                )
            }
            emitln!(self.writer, "),")
        }
        // Last variant for unknown value.
        emitln!(
            self.writer,
            "// Value to represent some arbitrary unknown function"
        );
        emitln!(
            self.writer,
            "$unknown_function'{}'(id: int)",
            boogie_type_suffix(self.env, fun_type),
        );
        self.writer.unindent();
        emitln!(self.writer, "}");
        emitln!(
            self.writer,
            "function $IsValid'{}'(x: {}): bool {{ true }}",
            boogie_type_suffix(self.env, fun_type),
            fun_ty_boogie_name
        );

        // Create an apply procedure.
        emit!(
            self.writer,
            "procedure {{:inline 1}} {}(",
            boogie_fun_apply_name(self.env, fun_type),
        );
        let Type::Fun(params, results, _abilities) = fun_type else {
            panic!("expected function type")
        };
        let params = params.clone().flatten();
        for (pos, ty) in params.iter().enumerate() {
            if pos > 0 {
                emit!(self.writer, ", ")
            }
            emit!(self.writer, "p{}: {}", pos, boogie_type(self.env, ty))
        }
        if !params.is_empty() {
            emit!(self.writer, ", ")
        }
        emit!(self.writer, "fun: {}", fun_ty_boogie_name);
        emit!(self.writer, ") returns (");
        let mut result_locals = vec![];
        for (pos, ty) in results.clone().flatten().iter().enumerate() {
            if pos > 0 {
                emit!(self.writer, ",")
            }
            let local_name = format!("r{}", pos);
            emit!(self.writer, "{}: {}", local_name, boogie_type(self.env, ty));
            result_locals.push(local_name)
        }
        emitln!(self.writer, ") {");
        self.writer.indent();
        let result_str = result_locals.iter().cloned().join(", ");
        for info in closure_infos {
            let ctor_name = boogie_closure_pack_name(self.env, &info.fun, info.mask);
            emitln!(self.writer, "if (fun is {}) {{", ctor_name);
            self.writer.indent();
            let fun_env = &self.env.get_function(info.fun.to_qualified_id());
            let fun_name = boogie_function_name(fun_env, &info.fun.inst);
            let args = (0..info.mask.captured_count())
                .map(|pos| format!("fun->p{}", pos))
                .chain((0..params.len()).map(|pos| format!("p{}", pos)))
                .join(", ");
            let call_prefix = if result_str.is_empty() {
                String::new()
            } else {
                format!("{result_str} := ")
            };
            emitln!(self.writer, "call {}{}({});", call_prefix, fun_name, args);
            self.writer.unindent();
            emitln!(self.writer, "} else ");
        }
        if !closure_infos.is_empty() {
            emitln!(self.writer, "{");
            self.writer.indent();
        }
        emitln!(
            self.writer,
            "// Havoc memory and abort_flag since the unknown function could modify anything."
        );
        emitln!(self.writer, "havoc $abort_flag;");
        for mem in memory {
            emitln!(self.writer, "havoc {};", mem)
        }
        if !closure_infos.is_empty() {
            self.writer.unindent();
            emitln!(self.writer, "}");
        }
        self.writer.unindent();
        emitln!(self.writer, "}");
    }
}

// =================================================================================================
// Struct Translation

impl StructTranslator<'_> {
    fn inst(&self, ty: &Type) -> Type {
        ty.instantiate(self.type_inst)
    }

    /// Return equality check for the field `f`
    pub fn boogie_field_is_equal(&self, f: &FieldEnv<'_>, sep: &str) -> String {
        let field_sel = boogie_field_sel(f);
        let bv_flag = self.field_bv_flag(f);
        let field_suffix =
            boogie_type_suffix_bv(self.parent.env, &self.inst(&f.get_type()), bv_flag);
        format!(
            "{}$IsEqual'{}'(s1->{}, s2->{})",
            sep, field_suffix, field_sel, field_sel,
        )
    }

    /// Return validity check of the field `f`
    pub fn boogie_field_is_valid(&self, field: &FieldEnv<'_>, sep: &str) -> String {
        let sel = format!("s->{}", boogie_field_sel(field));
        let ty = &field.get_type().instantiate(self.type_inst);
        let bv_flag = self.field_bv_flag(field);
        format!(
            "{}{}",
            sep,
            boogie_well_formed_expr_bv(self.parent.env, &sel, ty, bv_flag)
        )
    }

    /// Return argument of constructor for the field `f`
    pub fn boogie_field_arg_for_fun_sig(&self, f: &FieldEnv<'_>) -> String {
        format!(
            "{}: {}",
            boogie_field_sel(f),
            self.boogie_type_for_struct_field(f, self.parent.env, &self.inst(&f.get_type()))
        )
    }

    /// Emit is_valid and update for fields of `variant`
    pub fn emit_struct_variant_is_valid_and_update(
        &self,
        struct_env: &StructEnv,
        variant: Symbol,
        field_variant_map: &mut BTreeMap<(String, (String, String)), Vec<String>>,
    ) {
        let writer = self.parent.writer;
        let env = self.parent.env;
        let suffix_variant =
            boogie_type_suffix_for_struct_variant(struct_env, self.type_inst, &variant);
        let struct_variant_name = boogie_struct_variant_name(struct_env, self.type_inst, variant);
        let struct_name = boogie_struct_name(struct_env, self.type_inst);
        let fields = struct_env.get_fields_of_variant(variant).collect_vec();
        // Emit $Update function for each field in `variant`.
        for (pos, field_env) in struct_env.get_fields_of_variant(variant).enumerate() {
            let field_name = field_env.get_name().display(env.symbol_pool()).to_string();
            let field_type_name = self.boogie_type_for_struct_field(
                &field_env,
                env,
                &self.inst(&field_env.get_type()),
            );
            // The type name of the field is required when generating the corresponding update function
            // cannot use inst here because the caller side does not know the type instantiation info
            let field_type_name_uninst =
                self.boogie_type_for_struct_field(&field_env, env, &field_env.get_type());
            if field_variant_map.contains_key(&(
                field_name.clone(),
                (field_type_name.clone(), field_type_name_uninst.clone()),
            )) {
                let variants_vec = field_variant_map
                    .get_mut(&(
                        field_name.clone(),
                        (field_type_name.clone(), field_type_name_uninst.clone()),
                    ))
                    .unwrap();
                variants_vec.push(struct_variant_name.clone());
            } else {
                let variants_vec = vec![struct_variant_name.clone()];
                field_variant_map.insert(
                    (
                        field_name.to_string(),
                        (
                            field_type_name.to_string(),
                            field_type_name_uninst.to_string(),
                        ),
                    ),
                    variants_vec,
                );
            }
            self.emit_function(
                &format!(
                    "$Update'{}'_{}(s: {}, x: {}): {}",
                    suffix_variant, field_name, struct_name, field_type_name, struct_name
                ),
                || {
                    let args = fields
                        .iter()
                        .enumerate()
                        .map(|(p, f)| {
                            if p == pos {
                                "x".to_string()
                            } else {
                                format!("s->{}", boogie_field_sel(f))
                            }
                        })
                        .join(", ");
                    emitln!(writer, "{}({})", struct_variant_name, args);
                },
            );
        }

        // Emit $IsValid function for `variant`.
        self.parent.emit_function_with_attr(
            writer,
            "", // not inlined!
            &format!("$IsValid'{}'(s: {}): bool", suffix_variant, struct_name),
            || {
                if struct_env.is_intrinsic()
                    || struct_env
                        .get_fields_of_variant(variant)
                        .collect_vec()
                        .is_empty()
                {
                    emitln!(writer, "true")
                } else {
                    let mut sep = "";
                    emitln!(writer, "if s is {} then", suffix_variant);
                    for field in &fields {
                        emitln!(writer, "{}", self.boogie_field_is_valid(field, sep));
                        sep = "  && ";
                    }
                    emitln!(writer, "else false");
                }
            },
        );
    }

    // Emit $IsValid function for struct.
    fn emit_is_valid_struct(&self, struct_env: &StructEnv, struct_name: &str, emit_fn: impl Fn()) {
        let writer = self.parent.writer;
        self.parent.emit_function_with_attr(
            writer,
            "", // not inlined!
            &format!("$IsValid'{}'(s: {}): bool", struct_name, struct_name),
            || {
                if struct_env.is_intrinsic() || struct_env.get_field_count() == 0 {
                    emitln!(writer, "true")
                } else {
                    emit_fn()
                }
            },
        );
    }

    /// Emit $Update and $IsValid for enum
    fn emit_update_is_valid_for_enum(&self, struct_env: &StructEnv, struct_name: &str) {
        let writer = self.parent.writer;
        let mut field_variant_map: BTreeMap<(String, (String, String)), Vec<String>> =
            BTreeMap::new();
        for variant in struct_env.get_variants() {
            self.emit_struct_variant_is_valid_and_update(
                struct_env,
                variant,
                &mut field_variant_map,
            );
        }
        for ((field, (field_type, field_type_uninst)), variant_name) in field_variant_map {
            self.emit_function(
                &format!(
                    "$Update'{}'_{}_{}(s: {}, x: {}): {}",
                    struct_name,
                    // type name is needed in the update function name
                    // to distinguish fields with the same name but different types in different variants
                    // remove parentheses and spaces from field type name
                    field_type_uninst.replace(['(', ')'], "").replace(' ', "_"),
                    field,
                    struct_name,
                    field_type,
                    struct_name
                ),
                || {
                    let mut else_symbol = "";
                    for struct_variant_name in &variant_name {
                        let match_condition = format!("s is {}", struct_variant_name);
                        let update_str =
                            format!("$Update'{}'_{}(s, x)", struct_variant_name, field);
                        emitln!(writer, "{} if {} then", else_symbol, match_condition);
                        emitln!(writer, "{}", update_str);
                        if else_symbol.is_empty() {
                            else_symbol = "else";
                        }
                    }
                    emitln!(writer, "else s");
                },
            );
        }

        self.emit_is_valid_struct(struct_env, struct_name, || {
            let mut else_symbol = "";
            for variant in struct_env.get_variants() {
                let struct_variant_name =
                    boogie_struct_variant_name(struct_env, self.type_inst, variant);
                let match_condition = format!("s is {}", struct_variant_name);
                let str = format!("$IsValid'{}'(s)", struct_variant_name,);
                emitln!(writer, "{} if {} then", else_symbol, match_condition);
                emitln!(writer, "{}", str);
                if else_symbol.is_empty() {
                    else_symbol = "else";
                }
            }
            emitln!(writer, "else false");
        });
    }

    /// Emit the function body of $IsEqual for enum
    fn emit_is_equal_fn_body_for_enum(&self, struct_env: &StructEnv) {
        let writer = self.parent.writer;
        let mut sep = "";
        let mut else_symbol = "";
        for variant in struct_env.get_variants() {
            let mut equal_statement = "".to_string();
            for f in struct_env.get_fields_of_variant(variant) {
                let field_equal_str = self.boogie_field_is_equal(&f, sep);
                equal_statement = format!("{}{}", equal_statement, field_equal_str);
                sep = "&& ";
            }
            let struct_variant_name =
                boogie_struct_variant_name(struct_env, self.type_inst, variant);
            let match_condition = format!(
                "s1 is {} && s2 is {}",
                struct_variant_name, struct_variant_name
            );
            emitln!(writer, "{} if {} then", else_symbol, match_condition);
            if equal_statement.is_empty() {
                equal_statement = "true".to_string();
            }
            emitln!(writer, "{}", equal_statement);
            if else_symbol.is_empty() {
                else_symbol = "else";
            }
            sep = "";
        }
        emitln!(writer, "else false");
    }

    /// Emit the function cmp::compare for enum
    fn emit_cmp_for_enum(&self, struct_env: &StructEnv, struct_name: &str) {
        let writer = self.parent.writer;
        let suffix: String = boogie_type_suffix_for_struct(struct_env, self.type_inst, false);
        self.emit_function(
            &format!(
                "$1_cmp_$compare'{}'(v1: {}, v2: {}): $1_cmp_Ordering",
                suffix, struct_name, struct_name
            ),
            || {
                let mut else_symbol = "";
                for (pos_1, v1) in struct_env.get_variants().collect_vec().iter().enumerate() {
                    for (pos_2, v2) in struct_env.get_variants().collect_vec().iter().enumerate() {
                        if pos_2 <= pos_1 {
                            continue;
                        }
                        let struct_variant_name_1 =
                            boogie_struct_variant_name(struct_env, self.type_inst, *v1);
                        let struct_variant_name_2 =
                            boogie_struct_variant_name(struct_env, self.type_inst, *v2);
                        let cmp_order_less = format!(
                            "{} if v1 is {} && v2 is {} then $1_cmp_Ordering_Less()",
                            else_symbol, struct_variant_name_1, struct_variant_name_2
                        );
                        let cmp_order_greater = format!(
                            "else if v1 is {} && v2 is {} then $1_cmp_Ordering_Greater()",
                            struct_variant_name_2, struct_variant_name_1
                        );
                        if else_symbol.is_empty() {
                            else_symbol = "else";
                        }
                        emitln!(writer, "{}", cmp_order_less);
                        emitln!(writer, "{}", cmp_order_greater);
                    }
                }
                for variant in struct_env.get_variants().collect_vec().iter() {
                    let struct_variant_name_1 =
                        boogie_struct_variant_name(struct_env, self.type_inst, *variant);
                    let suffix_variant =
                        boogie_type_suffix_for_struct_variant(struct_env, self.type_inst, variant);
                    let cmp_order = format!(
                        "{} if v1 is {} && v2 is {} then $1_cmp_$compare'{}'(v1, v2)",
                        else_symbol, struct_variant_name_1, struct_variant_name_1, suffix_variant
                    );
                    emitln!(writer, "{}", cmp_order);
                }
                emitln!(writer, "else $Arbitrary_value_of'$1_cmp_Ordering'()");
            },
        );
        for variant in struct_env.get_variants().collect_vec().iter() {
            self.emit_cmp_for_enum_variant(struct_env, *variant, struct_name);
        }
    }

    /// Emit the function cmp::compare for each enum variant
    fn emit_cmp_for_enum_variant(
        &self,
        struct_env: &StructEnv,
        variant: Symbol,
        struct_name: &str,
    ) {
        let writer = self.parent.writer;
        let suffix_variant =
            boogie_type_suffix_for_struct_variant(struct_env, self.type_inst, &variant);
        self.emit_function(
            &format!(
                "$1_cmp_$compare'{}'(v1: {}, v2: {}): $1_cmp_Ordering",
                suffix_variant, struct_name, struct_name
            ),
            || {
                if struct_env
                    .get_fields_of_variant(variant)
                    .collect_vec()
                    .is_empty()
                {
                    emitln!(writer, "$1_cmp_Ordering_Equal()");
                } else {
                    for (pos, field) in struct_env.get_fields_of_variant(variant).enumerate() {
                        let bv_flag = self.field_bv_flag(&field);
                        let field_type_name = boogie_type_suffix_bv(
                            self.parent.env,
                            &self.inst(&field.get_type()),
                            bv_flag,
                        );
                        let cmp_field_call = format!(
                            "$1_cmp_$compare'{}'(v1->{}, v2->{})",
                            field_type_name,
                            boogie_field_sel(&field),
                            boogie_field_sel(&field)
                        );
                        let cmp_field_call_less =
                            format!("{} == $1_cmp_Ordering_Less()", cmp_field_call);
                        let cmp_field_call_greater =
                            format!("{} == $1_cmp_Ordering_Greater()", cmp_field_call);
                        emitln!(writer, "if {}", cmp_field_call_less);
                        emitln!(writer, "then $1_cmp_Ordering_Less()");
                        emitln!(writer, "else if {}", cmp_field_call_greater);
                        emitln!(writer, "then $1_cmp_Ordering_Greater()");
                        if pos
                            < struct_env
                                .get_fields_of_variant(variant)
                                .collect_vec()
                                .len()
                                - 1
                        {
                            emitln!(writer, "else");
                        } else {
                            emitln!(writer, "else $1_cmp_Ordering_Equal()");
                        }
                    }
                }
            },
        );
    }

    /// Return whether a field involves bitwise operations
    pub fn field_bv_flag(&self, field_env: &FieldEnv) -> bool {
        let global_state = &self
            .parent
            .env
            .get_extension::<GlobalNumberOperationState>()
            .expect("global number operation state");
        field_bv_flag_global_state(global_state, field_env)
    }

    /// Return boogie type for a struct
    pub fn boogie_type_for_struct_field(
        &self,
        field: &FieldEnv,
        env: &GlobalEnv,
        ty: &Type,
    ) -> String {
        let bv_flag = self.field_bv_flag(field);
        if bv_flag {
            boogie_bv_type(env, ty)
        } else {
            boogie_type(env, ty)
        }
    }

    /// Translates the given struct.
    fn translate(&self) {
        let writer = self.parent.writer;
        let struct_env = self.struct_env;
        let env = struct_env.module_env.env;

        let qid = struct_env
            .get_qualified_id()
            .instantiate(self.type_inst.to_owned());
        let struct_or_enum = if struct_env.has_variants() {
            "enum"
        } else {
            "struct"
        };
        emitln!(
            writer,
            "// {} {} {}",
            struct_or_enum,
            env.display(&qid),
            struct_env.get_loc().display(env)
        );

        // Set the location to internal as default.
        writer.set_location(&env.internal_loc());

        // Emit data type
        let struct_name = boogie_struct_name(struct_env, self.type_inst);
        emitln!(writer, "datatype {} {{", struct_name);

        // Emit constructor
        if struct_env.has_variants() {
            let constructor = struct_env
                .get_variants()
                .map(|variant| {
                    let struct_variant_name =
                        boogie_struct_variant_name(struct_env, self.type_inst, variant);
                    let variant_fields = struct_env
                        .get_fields_of_variant(variant)
                        .map(|f| self.boogie_field_arg_for_fun_sig(&f))
                        .join(", ");
                    format!("    {}({})", struct_variant_name, variant_fields,)
                })
                .join(", \n");
            emitln!(writer, "{}", constructor);
            emitln!(writer, "}");
        } else {
            let fields = struct_env
                .get_fields()
                .map(|f| self.boogie_field_arg_for_fun_sig(&f))
                .join(", ");
            emitln!(writer, "    {}({})", struct_name, fields,);
            emitln!(writer, "}");
        }

        // Emit $UpdateField functions.
        if struct_env.has_variants() {
            self.emit_update_is_valid_for_enum(struct_env, &struct_name);
        } else {
            let suffix = boogie_type_suffix_for_struct(struct_env, self.type_inst, false);
            let fields = struct_env.get_fields().collect_vec();
            for (pos, field_env) in fields.iter().enumerate() {
                let field_name = field_env.get_name().display(env.symbol_pool()).to_string();
                self.emit_function(
                    &format!(
                        "$Update'{}'_{}(s: {}, x: {}): {}",
                        suffix,
                        field_name,
                        struct_name,
                        self.boogie_type_for_struct_field(
                            field_env,
                            env,
                            &self.inst(&field_env.get_type())
                        ),
                        struct_name
                    ),
                    || {
                        let args = fields
                            .iter()
                            .enumerate()
                            .map(|(p, f)| {
                                if p == pos {
                                    "x".to_string()
                                } else {
                                    format!("s->{}", boogie_field_sel(f))
                                }
                            })
                            .join(", ");
                        emitln!(writer, "{}({})", struct_name, args);
                    },
                );
            }

            // Emit $IsValid function.
            self.emit_is_valid_struct(struct_env, &struct_name, || {
                let mut sep = "";
                for field in struct_env.get_fields() {
                    emitln!(writer, "{}", self.boogie_field_is_valid(&field, sep));
                    sep = "  && ";
                }
            });
        }

        // Emit equality
        self.emit_function(
            &format!(
                "$IsEqual'{}'(s1: {}, s2: {}): bool",
                boogie_type_suffix_for_struct(struct_env, self.type_inst, false),
                struct_name,
                struct_name
            ),
            || {
                if struct_has_native_equality(struct_env, self.type_inst, self.parent.options) {
                    emitln!(writer, "s1 == s2")
                } else if struct_env.has_variants() {
                    self.emit_is_equal_fn_body_for_enum(struct_env);
                } else {
                    let mut sep = "";
                    for field in &struct_env.get_fields().collect_vec() {
                        let field_equal_str = self.boogie_field_is_equal(field, sep);
                        emit!(writer, "{}", field_equal_str,);
                        sep = "\n&& ";
                    }
                }
            },
        );

        if struct_env.has_memory() {
            // Emit memory variable.
            let memory_name = boogie_resource_memory_name(
                env,
                &struct_env
                    .get_qualified_id()
                    .instantiate(self.type_inst.to_owned()),
                &None,
            );
            emitln!(writer, "var {}: $Memory {};", memory_name, struct_name);
        }

        // Emit compare function and procedure
        let cmp_struct_types = self.parent.env.cmp_types.borrow();
        for cmp_struct_type in cmp_struct_types.iter() {
            if let Some((cur_struct, inst)) = cmp_struct_type.get_struct(env) {
                if cur_struct.get_id() == struct_env.get_id() && inst == self.type_inst {
                    if !struct_env.has_variants() {
                        let suffix =
                            boogie_type_suffix_for_struct(struct_env, self.type_inst, false);
                        self.emit_function(
                            &format!(
                                "$1_cmp_$compare'{}'(v1: {}, v2: {}): $1_cmp_Ordering",
                                suffix, struct_name, struct_name
                            ),
                            || {
                                for (pos, field) in struct_env.get_fields().enumerate() {
                                    let bv_flag = self.field_bv_flag(&field);
                                    let suffix_ty = boogie_type_suffix_bv(
                                        self.parent.env,
                                        &self.inst(&field.get_type()),
                                        bv_flag,
                                    );
                                    let cmp_field_call = format!(
                                        "$1_cmp_$compare'{}'(v1->{}, v2->{})",
                                        suffix_ty,
                                        boogie_field_sel(&field),
                                        boogie_field_sel(&field)
                                    );
                                    let cmp_field_call_less =
                                        format!("{} == $1_cmp_Ordering_Less()", cmp_field_call);
                                    let cmp_field_call_greater =
                                        format!("{} == $1_cmp_Ordering_Greater()", cmp_field_call);
                                    emitln!(writer, "if {}", cmp_field_call_less);
                                    emitln!(writer, "then $1_cmp_Ordering_Less()");
                                    emitln!(writer, "else if {}", cmp_field_call_greater);
                                    emitln!(writer, "then $1_cmp_Ordering_Greater()");
                                    if pos < struct_env.get_field_count() - 1 {
                                        emitln!(writer, "else");
                                    } else {
                                        emitln!(writer, "else $1_cmp_Ordering_Equal()");
                                    }
                                }
                            },
                        );
                        self.emit_procedure(
                            &format!(
                            "$1_cmp_compare'{}'(v1: {}, v2: {}) returns ($ret0: $1_cmp_Ordering)",
                            suffix, struct_name, struct_name
                        ),
                            || {
                                emitln!(writer, "$ret0 := $1_cmp_$compare'{}'(v1, v2);", suffix);
                            },
                        );
                    } else {
                        self.emit_cmp_for_enum(struct_env, &struct_name);
                        let suffix: String =
                            boogie_type_suffix_for_struct(struct_env, self.type_inst, false);
                        self.emit_procedure(
                            &format!(
                            "$1_cmp_compare'{}'(v1: {}, v2: {}) returns ($ret0: $1_cmp_Ordering)",
                            suffix, struct_name, struct_name
                        ),
                            || {
                                emitln!(writer, "$ret0 := $1_cmp_$compare'{}'(v1, v2);", suffix);
                            },
                        );
                    }
                    break;
                }
            }
        }

        emitln!(writer);
    }

    #[allow(clippy::literal_string_with_formatting_args)]
    fn emit_function(&self, signature: &str, body_fn: impl Fn()) {
        self.parent
            .emit_function(self.parent.writer, signature, body_fn);
    }

    #[allow(clippy::literal_string_with_formatting_args)]
    fn emit_procedure(&self, signature: &str, body_fn: impl Fn()) {
        self.parent
            .emit_procedure(self.parent.writer, signature, body_fn);
    }
}

// =================================================================================================
// Function Translation

impl FunctionTranslator<'_> {
    /// Return whether a specific TempIndex involves in bitwise operations
    pub fn bv_flag_from_map(&self, i: &usize, operation_map: &FuncOperationMap) -> bool {
        let mid = self.fun_target.module_env().get_id();
        let sid = self.fun_target.func_env.get_id();
        let param_oper = operation_map.get(&(mid, sid)).unwrap().get(i);
        matches!(param_oper, Some(&Bitwise))
    }

    /// Return whether a specific TempIndex involves in bitwise operations
    pub fn bv_flag(&self, num_oper: &NumOperation) -> bool {
        *num_oper == Bitwise
    }

    /// Return whether a return value at position i involves in bitwise operation
    pub fn ret_bv_flag(&self, i: &usize) -> bool {
        let global_state = &self
            .fun_target
            .global_env()
            .get_extension::<GlobalNumberOperationState>()
            .expect("global number operation state");
        let operation_map = &global_state.get_ret_map();
        self.bv_flag_from_map(i, operation_map)
    }

    /// Return boogie type for a local with given signature token.
    pub fn boogie_type_for_fun(
        &self,
        env: &GlobalEnv,
        ty: &Type,
        num_oper: &NumOperation,
    ) -> String {
        let bv_flag = self.bv_flag(num_oper);
        if bv_flag {
            boogie_bv_type(env, ty)
        } else {
            boogie_type(env, ty)
        }
    }

    fn inst(&self, ty: &Type) -> Type {
        ty.instantiate(self.type_inst)
    }

    fn inst_slice(&self, tys: &[Type]) -> Vec<Type> {
        tys.iter().map(|ty| self.inst(ty)).collect()
    }

    fn get_local_type(&self, idx: TempIndex) -> Type {
        self.fun_target
            .get_local_type(idx)
            .instantiate(self.type_inst)
    }

    /// Translates the given function.
    fn translate(self) {
        let writer = self.parent.writer;
        let fun_target = self.fun_target;
        let env = fun_target.global_env();
        let qid = fun_target
            .func_env
            .get_qualified_id()
            .instantiate(self.type_inst.to_owned());
        emitln!(
            writer,
            "// fun {} [{}] {}",
            env.display(&qid),
            fun_target.data.variant,
            fun_target.get_loc().display(env)
        );
        self.generate_function_sig();
        self.generate_function_body();
        emitln!(self.parent.writer);
    }

    /// Return a string for a boogie procedure header. Use inline attribute and name
    /// suffix as indicated by `entry_point`.
    fn generate_function_sig(&self) {
        let writer = self.parent.writer;
        let options = self.parent.options;
        let fun_target = self.fun_target;
        let (args, rets) = self.generate_function_args_and_returns();

        let (suffix, attribs) = match &fun_target.data.variant {
            FunctionVariant::Baseline => ("".to_string(), "{:inline 1} ".to_string()),
            FunctionVariant::Verification(flavor) => {
                let mut attribs = vec![format!(
                    "{{:timeLimit {}}} ",
                    self.parent.get_timeout(fun_target)
                )];

                if let Some(seed) = fun_target.func_env.get_num_pragma(SEED_PRAGMA) {
                    attribs.push(format!("{{:random_seed {}}} ", seed));
                } else if fun_target.func_env.is_pragma_true(SEED_PRAGMA, || false) {
                    attribs.push(format!("{{:random_seed {}}} ", options.random_seed));
                }

                let suffix = match flavor {
                    VerificationFlavor::Regular => "$verify".to_string(),
                    VerificationFlavor::Instantiated(_) => {
                        format!("$verify_{}", flavor)
                    },
                    VerificationFlavor::Inconsistency(_) => {
                        attribs.push(format!(
                            "{{:msg_if_verifies \"inconsistency_detected{}\"}} ",
                            self.loc_str(&fun_target.get_loc())
                        ));
                        format!("$verify_{}", flavor)
                    },
                };
                (suffix, attribs.join(""))
            },
        };
        writer.set_location(&fun_target.get_loc());
        emitln!(
            writer,
            "procedure {}{}{}({}) returns ({})",
            attribs,
            boogie_function_name(fun_target.func_env, self.type_inst),
            suffix,
            args,
            rets,
        )
    }

    /// Generate boogie representation of function args and return args.
    fn generate_function_args_and_returns(&self) -> (String, String) {
        let fun_target = self.fun_target;
        let env = fun_target.global_env();
        let baseline_flag = self.fun_target.data.variant == FunctionVariant::Baseline;
        let global_state = &self
            .fun_target
            .global_env()
            .get_extension::<GlobalNumberOperationState>()
            .expect("global number operation state");
        let mid = fun_target.func_env.module_env.get_id();
        let fid = fun_target.func_env.get_id();
        let args = (0..fun_target.get_parameter_count())
            .map(|i| {
                let ty = self.get_local_type(i);
                // Boogie does not allow to assign to parameters, so we need to proxy them.
                let prefix = if self.parameter_needs_to_be_mutable(fun_target, i) {
                    "_$"
                } else {
                    "$"
                };
                let num_oper = global_state
                    .get_temp_index_oper(mid, fid, i, baseline_flag)
                    .unwrap_or(&Bottom);
                format!(
                    "{}t{}: {}",
                    prefix,
                    i,
                    self.boogie_type_for_fun(env, &ty, num_oper)
                )
            })
            .join(", ");
        let mut_ref_inputs = (0..fun_target.get_parameter_count())
            .enumerate()
            .filter_map(|(i, idx)| {
                let ty = self.get_local_type(idx);
                if ty.is_mutable_reference() {
                    Some((i, ty))
                } else {
                    None
                }
            })
            .collect_vec();
        let rets = fun_target
            .get_return_types()
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let s = self.inst(s);
                let operation_map = global_state.get_ret_map();
                let num_oper = operation_map.get(&(mid, fid)).unwrap().get(&i).unwrap();
                format!("$ret{}: {}", i, self.boogie_type_for_fun(env, &s, num_oper))
            })
            // Add implicit return parameters for &mut
            .chain(mut_ref_inputs.into_iter().enumerate().map(|(i, (_, ty))| {
                let num_oper = &global_state
                    .get_temp_index_oper(mid, fid, i, baseline_flag)
                    .unwrap();
                format!(
                    "$ret{}: {}",
                    usize::saturating_add(fun_target.get_return_count(), i),
                    self.boogie_type_for_fun(env, &ty, num_oper)
                )
            }))
            .join(", ");
        (args, rets)
    }

    /// Generates boogie implementation body.
    fn generate_function_body(&self) {
        let writer = self.parent.writer;
        let fun_target = self.fun_target;
        let variant = &fun_target.data.variant;
        let instantiation = &fun_target.data.type_args;
        let env = fun_target.global_env();
        let baseline_flag = self.fun_target.data.variant == FunctionVariant::Baseline;
        let global_state = &self
            .fun_target
            .global_env()
            .get_extension::<GlobalNumberOperationState>()
            .expect("global number operation state");

        // Be sure to set back location to the whole function definition as a default.
        writer.set_location(&fun_target.get_loc().at_start());

        emitln!(writer, "{");
        writer.indent();

        // Print instantiation information
        if !instantiation.is_empty() {
            let display_ctxt = TypeDisplayContext::new(env);
            emitln!(
                writer,
                "// function instantiation <{}>",
                instantiation
                    .iter()
                    .map(|ty| ty.display(&display_ctxt))
                    .join(", ")
            );
            emitln!(writer, "");
        }

        // Generate local variable declarations. They need to appear first in boogie.
        emitln!(writer, "// declare local variables");
        let num_args = fun_target.get_parameter_count();
        let mid = fun_target.func_env.module_env.get_id();
        let fid = fun_target.func_env.get_id();
        for i in num_args..fun_target.get_local_count() {
            let num_oper = global_state
                .get_temp_index_oper(mid, fid, i, baseline_flag)
                .unwrap();
            let local_type = &self.get_local_type(i);
            emitln!(
                writer,
                "var $t{}: {};",
                i,
                self.boogie_type_for_fun(env, local_type, num_oper)
            );
        }
        // Generate declarations for renamed parameters.
        let proxied_parameters = self.get_mutable_parameters();
        for (idx, ty) in &proxied_parameters {
            let num_oper = &global_state
                .get_temp_index_oper(mid, fid, *idx, baseline_flag)
                .unwrap();
            emitln!(
                writer,
                "var $t{}: {};",
                idx,
                self.boogie_type_for_fun(env, &ty.instantiate(self.type_inst), num_oper)
            );
        }
        // Generate declarations for modifies condition.
        let mut mem_inst_seen = BTreeSet::new();
        for qid in fun_target.get_modify_ids() {
            let memory = qid.instantiate(self.type_inst);
            if !mem_inst_seen.contains(&memory) {
                emitln!(
                    writer,
                    "var {}: {}",
                    boogie_modifies_memory_name(fun_target.global_env(), &memory),
                    "[int]bool;"
                );
                mem_inst_seen.insert(memory);
            }
        }
        let mut dup: Vec<String> = vec![];
        // Declare temporaries for debug tracing and other purposes.
        for (_, (ty, ref bv_flag, cnt)) in self.compute_needed_temps() {
            for i in 0..cnt {
                let bv_type = if *bv_flag {
                    boogie_bv_type
                } else {
                    boogie_type
                };
                let temp_name =
                    boogie_temp_from_suffix(env, &boogie_type_suffix_bv(env, &ty, *bv_flag), i);
                if !dup.contains(&temp_name) {
                    emitln!(writer, "var {}: {};", temp_name.clone(), bv_type(env, &ty));
                    dup.push(temp_name);
                }
            }
        }

        // Generate memory snapshot variable declarations.
        let code = fun_target.get_bytecode();
        let labels = code
            .iter()
            .filter_map(|bc| {
                use Bytecode::*;
                match bc {
                    SaveMem(_, lab, mem) => Some((lab, mem)),
                    SaveSpecVar(..) => panic!("spec var memory snapshots NYI"),
                    _ => None,
                }
            })
            .collect::<BTreeSet<_>>();
        for (lab, mem) in labels {
            let mem = &mem.to_owned().instantiate(self.type_inst);
            let name = boogie_resource_memory_name(env, mem, &Some(*lab));
            emitln!(
                writer,
                "var {}: $Memory {};",
                name,
                boogie_struct_name(&env.get_struct_qid(mem.to_qualified_id()), &mem.inst)
            );
        }

        // Initialize renamed parameters.
        for (idx, _) in proxied_parameters {
            emitln!(writer, "$t{} := _$t{};", idx, idx);
        }

        // Initial assumptions
        if self.parent.is_verified(variant, fun_target) {
            self.translate_verify_entry_assumptions(fun_target);
        }

        // Generate bytecode
        emitln!(writer, "\n// bytecode translation starts here");
        let mut last_tracked_loc = None;
        for bytecode in code.iter() {
            self.translate_bytecode(&mut last_tracked_loc, bytecode);
        }

        writer.unindent();
        emitln!(writer, "}");
    }

    fn get_mutable_parameters(&self) -> Vec<(TempIndex, Type)> {
        let fun_target = self.fun_target;
        (0..fun_target.get_parameter_count())
            .filter_map(|i| {
                if self.parameter_needs_to_be_mutable(fun_target, i) {
                    Some((i, fun_target.get_local_type(i).clone()))
                } else {
                    None
                }
            })
            .collect_vec()
    }

    /// Determines whether the parameter of a function needs to be mutable.
    /// Boogie does not allow to assign to procedure parameters. In some cases
    /// (e.g. for memory instrumentation, but also as a result of copy propagation),
    /// we may need to assign to parameters.
    fn parameter_needs_to_be_mutable(
        &self,
        _fun_target: &FunctionTarget<'_>,
        _idx: TempIndex,
    ) -> bool {
        // For now, we just always say true. This could be optimized because the actual (known
        // so far) sources for mutability are parameters which are used in WriteBack(LocalRoot(p))
        // position.
        true
    }

    fn translate_verify_entry_assumptions(&self, fun_target: &FunctionTarget<'_>) {
        let writer = self.parent.writer;
        emitln!(writer, "\n// verification entrypoint assumptions");

        // Prelude initialization
        emitln!(writer, "call $InitVerification();");

        // Assume reference parameters to be based on the Param(i) Location, ensuring
        // they are disjoint from all other references. This prevents aliasing and is justified as
        // follows:
        // - for mutual references, by their exclusive access in Move.
        // - for immutable references because we have eliminated them
        for i in 0..fun_target.get_parameter_count() {
            let ty = fun_target.get_local_type(i);
            if ty.is_reference() {
                emitln!(writer, "assume $t{}->l == $Param({});", i, i);
            }
        }
    }
}

// =================================================================================================
// Bytecode Translation

impl FunctionTranslator<'_> {
    /// Translates one bytecode instruction.
    fn translate_bytecode(
        &self,
        last_tracked_loc: &mut Option<(Loc, LineIndex)>,
        bytecode: &Bytecode,
    ) {
        use Bytecode::*;

        let writer = self.parent.writer;
        let spec_translator = &self.parent.spec_translator;
        let options = self.parent.options;
        let fun_target = self.fun_target;
        let env = fun_target.global_env();

        // Set location of this code in the CodeWriter.
        let attr_id = bytecode.get_attr_id();
        let loc = fun_target.get_bytecode_loc(attr_id);
        writer.set_location(&loc);

        // Print location.
        emitln!(
            writer,
            "// {} {}",
            bytecode
                .display(fun_target, &BTreeMap::default())
                .to_string()
                .replace('\n', "\n// "),
            loc.display(env)
        );

        // Print debug comments.
        if let Some(comment) = fun_target.get_debug_comment(attr_id) {
            if comment.starts_with("info: ") {
                // if the comment is annotated with "info: ", it should be displayed to the user
                emitln!(
                    writer,
                    "assume {{:print \"${}(){}\"}} true;",
                    &comment[..4],
                    &comment[4..]
                );
            } else {
                emitln!(writer, "// {}", comment);
            }
        }

        // Track location for execution traces.
        if matches!(bytecode, Call(_, _, Operation::TraceAbort, ..)) {
            // Ensure that aborts always has the precise location instead of the
            // line-approximated one
            *last_tracked_loc = None;
        }
        self.track_loc(last_tracked_loc, &loc);
        if matches!(bytecode, Label(_, _)) {
            // For labels, retrack the location after the label itself, so
            // the information will not be missing if we jump to this label
            *last_tracked_loc = None;
        }

        // Helper function to get a a string for a local
        let str_local = |idx: usize| format!("$t{}", idx);
        let baseline_flag = self.fun_target.data.variant == FunctionVariant::Baseline;
        let global_state = &self
            .fun_target
            .global_env()
            .get_extension::<GlobalNumberOperationState>()
            .expect("global number operation state");
        let mid = self.fun_target.func_env.module_env.get_id();
        let fid = self.fun_target.func_env.get_id();

        // Translate the bytecode instruction.
        match bytecode {
            SaveMem(_, label, mem) => {
                let mem = &mem.to_owned().instantiate(self.type_inst);
                let snapshot = boogie_resource_memory_name(env, mem, &Some(*label));
                let current = boogie_resource_memory_name(env, mem, &None);
                emitln!(writer, "{} := {};", snapshot, current);
            },
            SaveSpecVar(_, _label, _var) => {
                panic!("spec var snapshot NYI")
            },
            Prop(id, kind, exp) => match kind {
                PropKind::Assert => {
                    emit!(writer, "assert ");
                    let info = fun_target
                        .get_vc_info(*id)
                        .map(|s| s.as_str())
                        .unwrap_or("unknown assertion failed");
                    emit!(
                        writer,
                        "{{:msg \"assert_failed{}: {}\"}}\n  ",
                        self.loc_str(&loc),
                        info
                    );
                    spec_translator.translate(exp, self.type_inst);
                    emitln!(writer, ";");
                },
                PropKind::Assume => {
                    emit!(writer, "assume ");
                    spec_translator.translate(exp, self.type_inst);
                    emitln!(writer, ";");
                },
                PropKind::Modifies => {
                    let ty = &self.inst(&env.get_node_type(exp.node_id()));
                    let bv_flag = global_state.get_node_num_oper(exp.node_id()) == Bitwise;
                    let (mid, sid, inst) = ty.require_struct();
                    let memory = boogie_resource_memory_name(
                        env,
                        &mid.qualified_inst(sid, inst.to_owned()),
                        &None,
                    );
                    let exists_str = boogie_temp(env, &BOOL_TYPE, 0, false);
                    emitln!(writer, "havoc {};", exists_str);
                    emitln!(writer, "if ({}) {{", exists_str);
                    writer.with_indent(|| {
                        let val_str = boogie_temp(env, ty, 0, bv_flag);
                        emitln!(writer, "havoc {};", val_str);
                        emit!(writer, "{} := $ResourceUpdate({}, ", memory, memory);
                        spec_translator.translate(&exp.call_args()[0], self.type_inst);
                        emitln!(writer, ", {});", val_str);
                    });
                    emitln!(writer, "} else {");
                    writer.with_indent(|| {
                        emit!(writer, "{} := $ResourceRemove({}, ", memory, memory);
                        spec_translator.translate(&exp.call_args()[0], self.type_inst);
                        emitln!(writer, ");");
                    });
                    emitln!(writer, "}");
                },
            },
            Label(_, label) => {
                writer.unindent();
                emitln!(writer, "L{}:", label.as_usize());
                writer.indent();
            },
            Jump(_, target) => emitln!(writer, "goto L{};", target.as_usize()),
            Branch(_, then_target, else_target, idx) => emitln!(
                writer,
                "if ({}) {{ goto L{}; }} else {{ goto L{}; }}",
                str_local(*idx),
                then_target.as_usize(),
                else_target.as_usize(),
            ),
            Assign(_, dest, src, _) => {
                emitln!(writer, "{} := {};", str_local(*dest), str_local(*src));
            },
            Ret(_, rets) => {
                for (i, r) in rets.iter().enumerate() {
                    emitln!(writer, "$ret{} := {};", i, str_local(*r));
                }
                // Also assign input to output $mut parameters
                let mut ret_idx = rets.len();
                for i in 0..fun_target.get_parameter_count() {
                    if self.get_local_type(i).is_mutable_reference() {
                        emitln!(writer, "$ret{} := {};", ret_idx, str_local(i));
                        ret_idx = usize::saturating_add(ret_idx, 1);
                    }
                }
                emitln!(writer, "return;");
            },
            Load(_, dest, c) => {
                let num_oper = global_state
                    .get_temp_index_oper(mid, fid, *dest, baseline_flag)
                    .unwrap();
                let bv_flag = self.bv_flag(num_oper);
                let value = match c {
                    Constant::Bool(true) => "true".to_string(),
                    Constant::Bool(false) => "false".to_string(),
                    Constant::U8(num) => boogie_num_literal(&num.to_string(), 8, bv_flag),
                    Constant::U64(num) => boogie_num_literal(&num.to_string(), 64, bv_flag),
                    Constant::U128(num) => boogie_num_literal(&num.to_string(), 128, bv_flag),
                    Constant::U256(num) => boogie_num_literal(&num.to_string(), 256, bv_flag),
                    Constant::Address(val) => boogie_address(env, val),
                    Constant::ByteArray(val) => boogie_byte_blob(options, val, bv_flag),
                    Constant::AddressArray(val) => boogie_address_blob(env, options, val),
                    Constant::Vector(val) => boogie_constant_blob(env, options, val),
                    Constant::U16(num) => boogie_num_literal(&num.to_string(), 16, bv_flag),
                    Constant::U32(num) => boogie_num_literal(&num.to_string(), 32, bv_flag),
                };
                let dest_str = str_local(*dest);
                emitln!(writer, "{} := {};", dest_str, value);
                // Insert a WellFormed assumption so the new value gets tagged as u8, ...
                let ty = &self.get_local_type(*dest);
                let check = boogie_well_formed_check(env, &dest_str, ty, bv_flag);
                if !check.is_empty() {
                    emitln!(writer, &check);
                }
            },
            Call(_, dests, oper, srcs, aa) => {
                use Operation::*;
                match oper {
                    TestVariant(mid, sid, variant, inst) => {
                        let inst = &self.inst_slice(inst);
                        let src_type = self.get_local_type(srcs[0]);
                        let src_str = if src_type.is_reference() {
                            format!("$Dereference({})", str_local(srcs[0]))
                        } else {
                            str_local(srcs[0])
                        };
                        let dst_str = str_local(dests[0]);
                        let struct_env = env.get_module(*mid).into_struct(*sid);
                        let struct_variant_name =
                            boogie_struct_variant_name(&struct_env, inst, *variant);
                        emitln!(
                            writer,
                            "if ({} is {}) {{ {} := true; }}",
                            src_str,
                            struct_variant_name,
                            dst_str
                        );
                        emitln!(writer, "else {{ {} := false; }}", dst_str);
                    },
                    FreezeRef(_) => unreachable!(),
                    UnpackRef | UnpackRefDeep | PackRef | PackRefDeep => {
                        // No effect
                    },
                    OpaqueCallBegin(_, _, _) | OpaqueCallEnd(_, _, _) => {
                        // These are just markers.  There is no generated code.
                    },
                    WriteBack(node, edge) => {
                        self.translate_write_back(node, edge, srcs[0]);
                    },
                    IsParent(node, edge) => {
                        if let BorrowNode::Reference(parent) = node {
                            let src_str = str_local(srcs[0]);
                            let edge_pattern = edge
                                .flatten()
                                .into_iter()
                                .filter_map(|e| match e {
                                    BorrowEdge::Field(_, _, offset) => Some(format!("{}", offset)),
                                    BorrowEdge::Index(_) => Some("-1".to_owned()),
                                    BorrowEdge::Direct => None,
                                    _ => unreachable!(),
                                })
                                .collect_vec();
                            if edge_pattern.is_empty() {
                                emitln!(
                                    writer,
                                    "{} := $IsSameMutation({}, {});",
                                    str_local(dests[0]),
                                    str_local(*parent),
                                    src_str
                                );
                            } else if edge_pattern.len() == 1 {
                                emitln!(
                                    writer,
                                    "{} := $IsParentMutation({}, {}, {});",
                                    str_local(dests[0]),
                                    str_local(*parent),
                                    edge_pattern[0],
                                    src_str
                                );
                            } else {
                                emitln!(
                                    writer,
                                    "{} := $IsParentMutationHyper({}, {}, {});",
                                    str_local(dests[0]),
                                    str_local(*parent),
                                    boogie_make_vec_from_strings(&edge_pattern),
                                    src_str
                                );
                            }
                        } else {
                            panic!("inconsistent IsParent instruction: expected a reference node")
                        }
                    },
                    BorrowLoc => {
                        let src = srcs[0];
                        let dest = dests[0];
                        emitln!(
                            writer,
                            "{} := $Mutation($Local({}), EmptyVec(), {});",
                            str_local(dest),
                            src,
                            str_local(src)
                        );
                    },
                    ReadRef => {
                        let src = srcs[0];
                        let dest = dests[0];
                        emitln!(
                            writer,
                            "{} := $Dereference({});",
                            str_local(dest),
                            str_local(src)
                        );
                    },
                    WriteRef => {
                        let reference = srcs[0];
                        let value = srcs[1];
                        emitln!(
                            writer,
                            "{} := $UpdateMutation({}, {});",
                            str_local(reference),
                            str_local(reference),
                            str_local(value),
                        );
                    },
                    Function(mid, fid, inst) | Closure(mid, fid, inst, _) => {
                        let inst = &self.inst_slice(inst);
                        let module_env = env.get_module(*mid);
                        let callee_env = module_env.get_function(*fid);

                        let mut args_str = srcs.iter().cloned().map(str_local).join(", ");
                        let dest_str = dests
                            .iter()
                            .cloned()
                            .map(str_local)
                            // Add implict dest returns for &mut srcs:
                            //  f(x) --> x := f(x)  if type(x) = &mut_
                            .chain(
                                srcs.iter()
                                    .filter(|idx| self.get_local_type(**idx).is_mutable_reference())
                                    .cloned()
                                    .map(str_local),
                            )
                            .join(",");

                        if self.try_reflection_call(writer, env, inst, &callee_env, &dest_str) {
                            // Special case of reflection call, code is generated
                        } else if let Closure(_, _, _, mask) = oper {
                            // Special case of closure construction
                            let closure_ctor_name = boogie_closure_pack_name(
                                env,
                                &mid.qualified_inst(*fid, inst.clone()),
                                *mask,
                            );
                            emitln!(
                                writer,
                                "{} := {}({});",
                                dest_str,
                                closure_ctor_name,
                                args_str
                            )
                        } else {
                            // regular path
                            let targeted = self.fun_target.module_env().is_target();
                            // If the callee has been generated from a native interface, return
                            // an error
                            if callee_env.is_native() && targeted {
                                for attr in callee_env.get_attributes() {
                                    if let Attribute::Apply(_, name, _) = attr {
                                        if self
                                            .fun_target
                                            .module_env()
                                            .symbol_pool()
                                            .string(*name)
                                            .as_str()
                                            == NATIVE_INTERFACE
                                        {
                                            let loc = self.fun_target.get_bytecode_loc(attr_id);
                                            self.parent
                                                .env
                                                .error(&loc, "Unknown native function is called");
                                        }
                                    }
                                }
                            }
                            let caller_mid = self.fun_target.module_env().get_id();
                            let caller_fid = self.fun_target.get_id();
                            let fun_verified =
                                !self.fun_target.func_env.is_explicitly_not_verified(
                                    &ProverOptions::get(self.fun_target.global_env()).verify_scope,
                                );
                            let mut fun_name = boogie_function_name(&callee_env, inst);
                            // Helper function to check whether the idx corresponds to a bitwise operation
                            let compute_flag = |idx: TempIndex| {
                                targeted
                                    && fun_verified
                                    && *global_state
                                        .get_temp_index_oper(
                                            caller_mid,
                                            caller_fid,
                                            idx,
                                            baseline_flag,
                                        )
                                        .unwrap()
                                        == Bitwise
                            };
                            let instrument_bv2int =
                                |idx: TempIndex, args_str_vec: &mut Vec<String>| {
                                    let local_ty_srcs_1 = self.get_local_type(idx);
                                    let srcs_1_bv_flag = compute_flag(idx);
                                    let mut args_src_1_str = str_local(idx);
                                    if srcs_1_bv_flag {
                                        args_src_1_str = format!(
                                            "$bv2int.{}({})",
                                            boogie_num_type_base(
                                                self.parent.env,
                                                Some(self.fun_target.get_bytecode_loc(attr_id)),
                                                &local_ty_srcs_1
                                            ),
                                            args_src_1_str
                                        );
                                    }
                                    args_str_vec.push(args_src_1_str);
                                };
                            let callee_name = callee_env.get_name_str();
                            if dest_str.is_empty() {
                                let bv_flag = !srcs.is_empty() && compute_flag(srcs[0]);
                                if module_env.is_std_vector() {
                                    // Check the target vector contains bv values
                                    if callee_name.contains("insert") {
                                        let mut args_str_vec =
                                            vec![str_local(srcs[0]), str_local(srcs[1])];
                                        assert!(srcs.len() > 2);
                                        instrument_bv2int(srcs[2], &mut args_str_vec);
                                        args_str = args_str_vec.iter().cloned().join(", ");
                                    }
                                    fun_name =
                                        boogie_function_bv_name(&callee_env, inst, &[bv_flag]);
                                } else if module_env.is_table() {
                                    fun_name = boogie_function_bv_name(&callee_env, inst, &[
                                        false, bv_flag,
                                    ]);
                                }
                                emitln!(writer, "call {}({});", fun_name, args_str);
                            } else {
                                let dest_bv_flag = !dests.is_empty() && compute_flag(dests[0]);
                                let bv_flag = !srcs.is_empty() && compute_flag(srcs[0]);
                                if module_env.is_cmp() {
                                    fun_name = boogie_function_bv_name(&callee_env, inst, &[
                                        bv_flag || dest_bv_flag,
                                    ]);
                                }
                                // Handle the case where the return value of length is assigned to a bv int because
                                // length always returns a non-bv result
                                if module_env.is_std_vector() {
                                    fun_name = boogie_function_bv_name(&callee_env, inst, &[
                                        bv_flag || dest_bv_flag,
                                    ]);
                                    // Handle the case where the return value of length is assigned to a bv int because
                                    // length always returns a non-bv result
                                    if callee_name.contains("length") && dest_bv_flag {
                                        let local_ty = self.get_local_type(dests[0]);
                                        // Insert '$' for calling function instead of procedure
                                        // TODO(tengzhang): a hacky way to convert int to bv for return value
                                        fun_name.insert(10, '$');
                                        // first call len fun then convert its return value to a bv type
                                        emitln!(
                                            writer,
                                            "call {} := $int2bv{}({}({}));",
                                            dest_str,
                                            boogie_num_type_base(
                                                self.parent.env,
                                                Some(self.fun_target.get_bytecode_loc(attr_id)),
                                                &local_ty
                                            ),
                                            fun_name,
                                            args_str
                                        );
                                    } else if callee_name.contains("borrow")
                                        || callee_name.contains("remove")
                                        || callee_name.contains("swap")
                                    {
                                        let mut args_str_vec = vec![str_local(srcs[0])];
                                        instrument_bv2int(srcs[1], &mut args_str_vec);
                                        // Handle swap with three parameters
                                        if srcs.len() > 2 {
                                            instrument_bv2int(srcs[2], &mut args_str_vec);
                                        }
                                        args_str = args_str_vec.iter().cloned().join(", ");
                                    }
                                } else if module_env.is_table() {
                                    fun_name = boogie_function_bv_name(&callee_env, inst, &[
                                        false,
                                        bv_flag || dest_bv_flag,
                                    ]);
                                    if dest_bv_flag && callee_name.contains("length") {
                                        // Handle the case where the return value of length is assigned to a bv int because
                                        // length always returns a non-bv result
                                        let local_ty = self.get_local_type(dests[0]);
                                        // Replace with "spec_len"
                                        let length_idx_start = callee_name.find("length").unwrap();
                                        let length_idx_end = length_idx_start + "length".len();
                                        fun_name = [
                                            callee_name[0..length_idx_start].to_string(),
                                            "spec_len".to_string(),
                                            callee_name[length_idx_end..].to_string(),
                                        ]
                                        .join("");
                                        // first call len fun then convert its return value to a bv type
                                        emitln!(
                                            writer,
                                            "call {} := $int2bv{}({}({}));",
                                            dest_str,
                                            boogie_num_type_base(
                                                self.parent.env,
                                                Some(self.fun_target.get_bytecode_loc(attr_id)),
                                                &local_ty
                                            ),
                                            fun_name,
                                            args_str
                                        );
                                    }
                                }
                                emitln!(writer, "call {} := {}({});", dest_str, fun_name, args_str);
                            }
                        }

                        // Clear the last track location after function call, as the call inserted
                        // location tracks before it returns.
                        *last_tracked_loc = None;
                    },
                    Invoke => {
                        // Function is last argument
                        let fun_type = self.inst(fun_target.get_local_type(*srcs.last().unwrap()));
                        let args_str = srcs.iter().cloned().map(str_local).join(", ");
                        let dest_str = dests
                            .iter()
                            .cloned()
                            .map(str_local)
                            // Add implict dest returns for &mut srcs:
                            //  f(x) --> x := f(x)  if type(x) = &mut_
                            .chain(
                                srcs.iter()
                                    .filter(|idx| self.get_local_type(**idx).is_mutable_reference())
                                    .cloned()
                                    .map(str_local),
                            )
                            .join(",");
                        let apply_fun = boogie_fun_apply_name(env, &fun_type);
                        let call_prefix = if dest_str.is_empty() {
                            String::new()
                        } else {
                            format!("{dest_str} := ")
                        };
                        emitln!(writer, "call {}{}({});", call_prefix, apply_fun, args_str);
                    },
                    Pack(mid, sid, inst) => {
                        let inst = &self.inst_slice(inst);
                        let struct_env = env.get_module(*mid).into_struct(*sid);
                        let args = srcs.iter().cloned().map(str_local).join(", ");
                        let dest_str = str_local(dests[0]);
                        emitln!(
                            writer,
                            "{} := {}({});",
                            dest_str,
                            boogie_struct_name(&struct_env, inst),
                            args
                        );
                    },
                    PackVariant(mid, sid, variant, inst) => {
                        let inst = &self.inst_slice(inst);
                        let struct_env = env.get_module(*mid).into_struct(*sid);
                        let args = srcs.iter().cloned().map(str_local).join(", ");
                        let dest_str = str_local(dests[0]);
                        emitln!(
                            writer,
                            "{} := {}({});",
                            dest_str,
                            boogie_struct_variant_name(&struct_env, inst, *variant),
                            args
                        );
                    },
                    Unpack(mid, sid, _) => {
                        let struct_env = env.get_module(*mid).into_struct(*sid);
                        for (i, ref field_env) in struct_env.get_fields().enumerate() {
                            let field_sel =
                                format!("{}->{}", str_local(srcs[0]), boogie_field_sel(field_env),);
                            emitln!(writer, "{} := {};", str_local(dests[i]), field_sel);
                        }
                    },
                    UnpackVariant(mid, sid, variant, inst) => {
                        let inst = &self.inst_slice(inst);
                        let src_str = str_local(srcs[0]);
                        let struct_env = env.get_module(*mid).into_struct(*sid);
                        let struct_variant_name =
                            boogie_struct_variant_name(&struct_env, inst, *variant);
                        emitln!(writer, "if ({} is {}) {{", src_str, struct_variant_name);
                        for (i, ref field_env) in
                            struct_env.get_fields_of_variant(*variant).enumerate()
                        {
                            let field_sel =
                                format!("{}->{}", str_local(srcs[0]), boogie_field_sel(field_env),);
                            emitln!(writer, "{} := {};", str_local(dests[i]), field_sel);
                        }
                        emitln!(writer, "} else { call $ExecFailureAbort(); }");
                    },
                    BorrowField(mid, sid, _, field_offset) => {
                        let src_str = str_local(srcs[0]);
                        let dest_str = str_local(dests[0]);
                        let struct_env = env.get_module(*mid).into_struct(*sid);
                        self.check_intrinsic_select(attr_id, &struct_env);
                        let field_env = &struct_env.get_field_by_offset(*field_offset);
                        let field_sel = boogie_field_sel(field_env);
                        emitln!(
                            writer,
                            "{} := $ChildMutation({}, {}, $Dereference({})->{});",
                            dest_str,
                            src_str,
                            field_offset,
                            src_str,
                            field_sel,
                        );
                    },
                    BorrowVariantField(mid, sid, variants, inst, field_offset) => {
                        let inst = &self.inst_slice(inst);
                        let src_str = str_local(srcs[0]);
                        let deref_src_str = format!("$Dereference({})", src_str);
                        let dest_str = str_local(dests[0]);
                        let struct_env = env.get_module(*mid).into_struct(*sid);
                        self.check_intrinsic_select(attr_id, &struct_env);
                        let mut else_symbol = "";
                        // Need to go through all variants to find the correct field
                        for variant in variants {
                            emit!(writer, "{} if (", else_symbol);
                            let struct_variant_name =
                                boogie_struct_variant_name(&struct_env, inst, *variant);
                            let field_env = struct_env.get_field_by_offset_optional_variant(
                                Some(*variant),
                                *field_offset,
                            );
                            let field_sel = boogie_field_sel(&field_env);
                            emitln!(writer, "{} is {}) {{", deref_src_str, struct_variant_name);
                            emitln!(
                                writer,
                                "{} := $ChildMutation({}, {}, {}->{});",
                                dest_str,
                                src_str,
                                field_offset,
                                deref_src_str,
                                field_sel,
                            );
                            emitln!(writer, "}");
                            if else_symbol.is_empty() {
                                else_symbol = " else ";
                            }
                        }
                        emitln!(writer, "else { call $ExecFailureAbort(); }");
                    },
                    GetField(mid, sid, _, field_offset) => {
                        let src = srcs[0];
                        let mut src_str = str_local(src);
                        let dest_str = str_local(dests[0]);
                        let struct_env = env.get_module(*mid).into_struct(*sid);
                        self.check_intrinsic_select(attr_id, &struct_env);
                        let field_env = &struct_env.get_field_by_offset(*field_offset);
                        let field_sel = boogie_field_sel(field_env);
                        if self.get_local_type(src).is_reference() {
                            src_str = format!("$Dereference({})", src_str);
                        };
                        emitln!(writer, "{} := {}->{};", dest_str, src_str, field_sel);
                    },
                    GetVariantField(mid, sid, variants, inst, field_offset) => {
                        let inst = &self.inst_slice(inst);
                        let src = srcs[0];
                        let mut src_str = str_local(src);
                        let dest_str = str_local(dests[0]);
                        let struct_env = env.get_module(*mid).into_struct(*sid);
                        self.check_intrinsic_select(attr_id, &struct_env);
                        let mut else_symbol = "";
                        // Need to go through all variants to find the correct field
                        for variant in variants {
                            emitln!(writer, "{} if (", else_symbol);
                            if self.get_local_type(src).is_reference() {
                                src_str = format!("$Dereference({})", src_str);
                            };
                            let struct_variant_name =
                                boogie_struct_variant_name(&struct_env, inst, *variant);
                            let field_env = struct_env.get_field_by_offset_optional_variant(
                                Some(*variant),
                                *field_offset,
                            );
                            let field_sel = boogie_field_sel(&field_env);
                            emit!(writer, "{} is {}) {{", src_str, struct_variant_name);
                            emitln!(writer, "{} := {}->{};", dest_str, src_str, field_sel);
                            emitln!(writer, "}");
                            if else_symbol.is_empty() {
                                else_symbol = " else ";
                            }
                        }
                        emitln!(writer, "else { call $ExecFailureAbort(); }");
                    },
                    Exists(mid, sid, inst) => {
                        let inst = self.inst_slice(inst);
                        let addr_str = str_local(srcs[0]);
                        let dest_str = str_local(dests[0]);
                        let memory = boogie_resource_memory_name(
                            env,
                            &mid.qualified_inst(*sid, inst),
                            &None,
                        );
                        emitln!(
                            writer,
                            "{} := $ResourceExists({}, {});",
                            dest_str,
                            memory,
                            addr_str
                        );
                    },
                    BorrowGlobal(mid, sid, inst) => {
                        let inst = self.inst_slice(inst);
                        let addr_str = str_local(srcs[0]);
                        let dest_str = str_local(dests[0]);
                        let memory = boogie_resource_memory_name(
                            env,
                            &mid.qualified_inst(*sid, inst),
                            &None,
                        );
                        emitln!(writer, "if (!$ResourceExists({}, {})) {{", memory, addr_str);
                        writer.with_indent(|| emitln!(writer, "call $ExecFailureAbort();"));
                        emitln!(writer, "} else {");
                        writer.with_indent(|| {
                            emitln!(
                                writer,
                                "{} := $Mutation($Global({}), EmptyVec(), $ResourceValue({}, {}));",
                                dest_str,
                                addr_str,
                                memory,
                                addr_str
                            );
                        });
                        emitln!(writer, "}");
                    },
                    GetGlobal(mid, sid, inst) => {
                        let inst = self.inst_slice(inst);
                        let memory = boogie_resource_memory_name(
                            env,
                            &mid.qualified_inst(*sid, inst),
                            &None,
                        );
                        let addr_str = str_local(srcs[0]);
                        let dest_str = str_local(dests[0]);
                        emitln!(writer, "if (!$ResourceExists({}, {})) {{", memory, addr_str);
                        writer.with_indent(|| emitln!(writer, "call $ExecFailureAbort();"));
                        emitln!(writer, "} else {");
                        writer.with_indent(|| {
                            emitln!(
                                writer,
                                "{} := $ResourceValue({}, {});",
                                dest_str,
                                memory,
                                addr_str
                            );
                        });
                        emitln!(writer, "}");
                    },
                    MoveTo(mid, sid, inst) => {
                        let inst = self.inst_slice(inst);
                        let memory = boogie_resource_memory_name(
                            env,
                            &mid.qualified_inst(*sid, inst),
                            &None,
                        );
                        let value_str = str_local(srcs[0]);
                        let signer_str = str_local(srcs[1]);
                        emitln!(
                            writer,
                            "if ($ResourceExists({}, {}->$addr)) {{",
                            memory,
                            signer_str
                        );
                        writer.with_indent(|| emitln!(writer, "call $ExecFailureAbort();"));
                        emitln!(writer, "} else {");
                        writer.with_indent(|| {
                            emitln!(
                                writer,
                                "{} := $ResourceUpdate({}, {}->$addr, {});",
                                memory,
                                memory,
                                signer_str,
                                value_str
                            );
                        });
                        emitln!(writer, "}");
                    },
                    MoveFrom(mid, sid, inst) => {
                        let inst = &self.inst_slice(inst);
                        let memory = boogie_resource_memory_name(
                            env,
                            &mid.qualified_inst(*sid, inst.to_owned()),
                            &None,
                        );
                        let addr_str = str_local(srcs[0]);
                        let dest_str = str_local(dests[0]);
                        emitln!(writer, "if (!$ResourceExists({}, {})) {{", memory, addr_str);
                        writer.with_indent(|| emitln!(writer, "call $ExecFailureAbort();"));
                        emitln!(writer, "} else {");
                        writer.with_indent(|| {
                            emitln!(
                                writer,
                                "{} := $ResourceValue({}, {});",
                                dest_str,
                                memory,
                                addr_str
                            );
                            emitln!(
                                writer,
                                "{} := $ResourceRemove({}, {});",
                                memory,
                                memory,
                                addr_str
                            );
                        });
                        emitln!(writer, "}");
                    },
                    Havoc(HavocKind::Value) | Havoc(HavocKind::MutationAll) => {
                        let var_str = str_local(dests[0]);
                        emitln!(writer, "havoc {};", var_str);
                    },
                    Havoc(HavocKind::MutationValue) => {
                        let ty = &self.get_local_type(dests[0]);
                        let num_oper = global_state
                            .get_temp_index_oper(mid, fid, dests[0], baseline_flag)
                            .unwrap();
                        let bv_flag = self.bv_flag(num_oper);
                        let var_str = str_local(dests[0]);
                        let temp_str = boogie_temp(env, ty.skip_reference(), 0, bv_flag);
                        emitln!(writer, "havoc {};", temp_str);
                        emitln!(
                            writer,
                            "{} := $UpdateMutation({}, {});",
                            var_str,
                            var_str,
                            temp_str
                        );
                    },
                    Stop => {
                        // the two statements combined terminate any execution trace that reaches it
                        emitln!(writer, "assume false;");
                        emitln!(writer, "return;");
                    },
                    CastU8 | CastU16 | CastU32 | CastU64 | CastU128 | CastU256 => {
                        let src = srcs[0];
                        let dest = dests[0];
                        let make_cast = |target_base: &str, src: TempIndex, dest: TempIndex| {
                            let num_oper = global_state
                                .get_temp_index_oper(mid, fid, src, baseline_flag)
                                .unwrap();
                            let bv_flag = self.bv_flag(num_oper);
                            if bv_flag {
                                let src_type = self.get_local_type(src);
                                let base = boogie_num_type_base(
                                    self.parent.env,
                                    Some(self.fun_target.get_bytecode_loc(attr_id)),
                                    &src_type,
                                );
                                emitln!(
                                    writer,
                                    "call {} := $CastBv{}to{}({});",
                                    str_local(dest),
                                    base,
                                    target_base,
                                    str_local(src)
                                );
                            } else {
                                emitln!(
                                    writer,
                                    "call {} := $CastU{}({});",
                                    str_local(dest),
                                    target_base,
                                    str_local(src)
                                );
                            }
                        };
                        let target_base = match oper {
                            CastU8 => "8",
                            CastU16 => "16",
                            CastU32 => "32",
                            CastU64 => "64",
                            CastU128 => "128",
                            CastU256 => "256",
                            _ => unreachable!(),
                        };
                        make_cast(target_base, src, dest);
                    },
                    Not => {
                        let src = srcs[0];
                        let dest = dests[0];
                        emitln!(
                            writer,
                            "call {} := $Not({});",
                            str_local(dest),
                            str_local(src)
                        );
                    },
                    Add => {
                        let dest = dests[0];
                        let op1 = srcs[0];
                        let op2 = srcs[1];
                        let unchecked = if fun_target
                            .is_pragma_true(ADDITION_OVERFLOW_UNCHECKED_PRAGMA, || false)
                        {
                            "_unchecked"
                        } else {
                            ""
                        };
                        let num_oper = global_state
                            .get_temp_index_oper(mid, fid, dest, baseline_flag)
                            .unwrap();
                        let bv_flag = self.bv_flag(num_oper);

                        let add_type = match &self.get_local_type(dest) {
                            Type::Primitive(PrimitiveType::U8) => {
                                boogie_num_type_string_capital("8", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::U16) => format!(
                                "{}{}",
                                boogie_num_type_string_capital("16", bv_flag),
                                unchecked
                            ),
                            Type::Primitive(PrimitiveType::U32) => format!(
                                "{}{}",
                                boogie_num_type_string_capital("32", bv_flag),
                                unchecked
                            ),
                            Type::Primitive(PrimitiveType::U64) => format!(
                                "{}{}",
                                boogie_num_type_string_capital("64", bv_flag),
                                unchecked
                            ),
                            Type::Primitive(PrimitiveType::U128) => format!(
                                "{}{}",
                                boogie_num_type_string_capital("128", bv_flag),
                                unchecked
                            ),
                            Type::Primitive(PrimitiveType::U256) => format!(
                                "{}{}",
                                boogie_num_type_string_capital("256", bv_flag),
                                unchecked
                            ),
                            Type::Primitive(_)
                            | Type::Tuple(_)
                            | Type::Vector(_)
                            | Type::Struct(_, _, _)
                            | Type::TypeParameter(_)
                            | Type::Reference(_, _)
                            | Type::Fun(..)
                            | Type::TypeDomain(_)
                            | Type::ResourceDomain(_, _, _)
                            | Type::Error
                            | Type::Var(_) => unreachable!(),
                        };
                        emitln!(
                            writer,
                            "call {} := $Add{}({}, {});",
                            str_local(dest),
                            add_type,
                            str_local(op1),
                            str_local(op2)
                        );
                    },
                    Sub => {
                        let dest = dests[0];
                        let op1 = srcs[0];
                        let op2 = srcs[1];
                        let num_oper = global_state
                            .get_temp_index_oper(mid, fid, dest, baseline_flag)
                            .unwrap();
                        let bv_flag = self.bv_flag(num_oper);
                        if bv_flag {
                            let sub_type = match &self.get_local_type(dest) {
                                Type::Primitive(PrimitiveType::U8) => "Bv8".to_string(),
                                Type::Primitive(PrimitiveType::U16) => "Bv16".to_string(),
                                Type::Primitive(PrimitiveType::U32) => "Bv32".to_string(),
                                Type::Primitive(PrimitiveType::U64) => "Bv64".to_string(),
                                Type::Primitive(PrimitiveType::U128) => "Bv128".to_string(),
                                Type::Primitive(PrimitiveType::U256) => "Bv256".to_string(),
                                Type::Primitive(_)
                                | Type::Tuple(_)
                                | Type::Vector(_)
                                | Type::Struct(_, _, _)
                                | Type::TypeParameter(_)
                                | Type::Reference(_, _)
                                | Type::Fun(..)
                                | Type::TypeDomain(_)
                                | Type::ResourceDomain(_, _, _)
                                | Type::Error
                                | Type::Var(_) => unreachable!(),
                            };
                            emitln!(
                                writer,
                                "call {} := $Sub{}({}, {});",
                                str_local(dest),
                                sub_type,
                                str_local(op1),
                                str_local(op2)
                            );
                        } else {
                            emitln!(
                                writer,
                                "call {} := $Sub({}, {});",
                                str_local(dest),
                                str_local(op1),
                                str_local(op2)
                            );
                        }
                    },
                    Mul => {
                        let dest = dests[0];
                        let op1 = srcs[0];
                        let op2 = srcs[1];
                        let num_oper = global_state
                            .get_temp_index_oper(mid, fid, dest, baseline_flag)
                            .unwrap();
                        let bv_flag = self.bv_flag(num_oper);
                        let mul_type = match &self.get_local_type(dest) {
                            Type::Primitive(PrimitiveType::U8) => {
                                boogie_num_type_string_capital("8", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::U16) => {
                                boogie_num_type_string_capital("16", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::U32) => {
                                boogie_num_type_string_capital("32", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::U64) => {
                                boogie_num_type_string_capital("64", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::U128) => {
                                boogie_num_type_string_capital("128", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::U256) => {
                                boogie_num_type_string_capital("256", bv_flag)
                            },
                            Type::Primitive(_)
                            | Type::Tuple(_)
                            | Type::Vector(_)
                            | Type::Struct(_, _, _)
                            | Type::TypeParameter(_)
                            | Type::Reference(_, _)
                            | Type::Fun(..)
                            | Type::TypeDomain(_)
                            | Type::ResourceDomain(_, _, _)
                            | Type::Error
                            | Type::Var(_) => unreachable!(),
                        };
                        emitln!(
                            writer,
                            "call {} := $Mul{}({}, {});",
                            str_local(dest),
                            mul_type,
                            str_local(op1),
                            str_local(op2)
                        );
                    },
                    Div => {
                        let dest = dests[0];
                        let op1 = srcs[0];
                        let op2 = srcs[1];
                        let num_oper = global_state
                            .get_temp_index_oper(mid, fid, dest, baseline_flag)
                            .unwrap();
                        let bv_flag = self.bv_flag(num_oper);
                        let div_type = if bv_flag {
                            match &self.get_local_type(dest) {
                                Type::Primitive(PrimitiveType::U8) => "Bv8".to_string(),
                                Type::Primitive(PrimitiveType::U16) => "Bv16".to_string(),
                                Type::Primitive(PrimitiveType::U32) => "Bv32".to_string(),
                                Type::Primitive(PrimitiveType::U64) => "Bv64".to_string(),
                                Type::Primitive(PrimitiveType::U128) => "Bv128".to_string(),
                                Type::Primitive(PrimitiveType::U256) => "Bv256".to_string(),
                                Type::Primitive(_)
                                | Type::Tuple(_)
                                | Type::Vector(_)
                                | Type::Struct(_, _, _)
                                | Type::TypeParameter(_)
                                | Type::Reference(_, _)
                                | Type::Fun(..)
                                | Type::TypeDomain(_)
                                | Type::ResourceDomain(_, _, _)
                                | Type::Error
                                | Type::Var(_) => unreachable!(),
                            }
                        } else {
                            "".to_string()
                        };
                        emitln!(
                            writer,
                            "call {} := $Div{}({}, {});",
                            str_local(dest),
                            div_type,
                            str_local(op1),
                            str_local(op2)
                        );
                    },
                    Mod => {
                        let dest = dests[0];
                        let op1 = srcs[0];
                        let op2 = srcs[1];
                        let num_oper = global_state
                            .get_temp_index_oper(mid, fid, dest, baseline_flag)
                            .unwrap();
                        let bv_flag = self.bv_flag(num_oper);
                        let mod_type = if bv_flag {
                            match &self.get_local_type(dest) {
                                Type::Primitive(PrimitiveType::U8) => "Bv8".to_string(),
                                Type::Primitive(PrimitiveType::U16) => "Bv16".to_string(),
                                Type::Primitive(PrimitiveType::U32) => "Bv32".to_string(),
                                Type::Primitive(PrimitiveType::U64) => "Bv64".to_string(),
                                Type::Primitive(PrimitiveType::U128) => "Bv128".to_string(),
                                Type::Primitive(PrimitiveType::U256) => "Bv256".to_string(),
                                Type::Primitive(_)
                                | Type::Tuple(_)
                                | Type::Vector(_)
                                | Type::Struct(_, _, _)
                                | Type::TypeParameter(_)
                                | Type::Reference(_, _)
                                | Type::Fun(..)
                                | Type::TypeDomain(_)
                                | Type::ResourceDomain(_, _, _)
                                | Type::Error
                                | Type::Var(_) => unreachable!(),
                            }
                        } else {
                            "".to_string()
                        };
                        emitln!(
                            writer,
                            "call {} := $Mod{}({}, {});",
                            str_local(dest),
                            mod_type,
                            str_local(op1),
                            str_local(op2)
                        );
                    },
                    Shl | Shr => {
                        let dest = dests[0];
                        let op1 = srcs[0];
                        let op2 = srcs[1];
                        let sh_oper_str = if oper == &Shl { "Shl" } else { "Shr" };
                        let num_oper = global_state
                            .get_temp_index_oper(mid, fid, dest, baseline_flag)
                            .unwrap();
                        let bv_flag = self.bv_flag(num_oper);
                        if bv_flag {
                            let target_type = match &self.get_local_type(dest) {
                                Type::Primitive(PrimitiveType::U8) => "Bv8",
                                Type::Primitive(PrimitiveType::U16) => "Bv16",
                                Type::Primitive(PrimitiveType::U32) => "Bv32",
                                Type::Primitive(PrimitiveType::U64) => "Bv64",
                                Type::Primitive(PrimitiveType::U128) => "Bv128",
                                Type::Primitive(PrimitiveType::U256) => "Bv256",
                                Type::Primitive(_)
                                | Type::Tuple(_)
                                | Type::Vector(_)
                                | Type::Struct(_, _, _)
                                | Type::TypeParameter(_)
                                | Type::Reference(_, _)
                                | Type::Fun(..)
                                | Type::TypeDomain(_)
                                | Type::ResourceDomain(_, _, _)
                                | Type::Error
                                | Type::Var(_) => unreachable!(),
                            };
                            let src_type = boogie_num_type_base(
                                self.parent.env,
                                Some(self.fun_target.get_bytecode_loc(attr_id)),
                                &self.get_local_type(op2),
                            );
                            emitln!(
                                writer,
                                "call {} := ${}{}From{}({}, {});",
                                str_local(dest),
                                sh_oper_str,
                                target_type,
                                src_type,
                                str_local(op1),
                                str_local(op2)
                            );
                        } else {
                            let sh_type = match &self.get_local_type(dest) {
                                Type::Primitive(PrimitiveType::U8) => "U8",
                                Type::Primitive(PrimitiveType::U16) => "U16",
                                Type::Primitive(PrimitiveType::U32) => "U32",
                                Type::Primitive(PrimitiveType::U64) => "U64",
                                Type::Primitive(PrimitiveType::U128) => "U128",
                                Type::Primitive(PrimitiveType::U256) => "U256",
                                Type::Primitive(_)
                                | Type::Tuple(_)
                                | Type::Vector(_)
                                | Type::Struct(_, _, _)
                                | Type::TypeParameter(_)
                                | Type::Reference(_, _)
                                | Type::Fun(..)
                                | Type::TypeDomain(_)
                                | Type::ResourceDomain(_, _, _)
                                | Type::Error
                                | Type::Var(_) => unreachable!(),
                            };
                            emitln!(
                                writer,
                                "call {} := ${}{}({}, {});",
                                str_local(dest),
                                sh_oper_str,
                                sh_type,
                                str_local(op1),
                                str_local(op2)
                            );
                        }
                    },
                    Lt | Le | Gt | Ge => {
                        let dest = dests[0];
                        let op1 = srcs[0];
                        let op2 = srcs[1];
                        let make_comparison = |comp_oper: &str, op1, op2, dest| {
                            let num_oper = global_state
                                .get_temp_index_oper(mid, fid, op1, baseline_flag)
                                .unwrap();
                            let bv_flag = self.bv_flag(num_oper);
                            let lt_type = if bv_flag {
                                match &self.get_local_type(op1) {
                                    Type::Primitive(PrimitiveType::U8) => "Bv8".to_string(),
                                    Type::Primitive(PrimitiveType::U16) => "Bv16".to_string(),
                                    Type::Primitive(PrimitiveType::U32) => "Bv32".to_string(),
                                    Type::Primitive(PrimitiveType::U64) => "Bv64".to_string(),
                                    Type::Primitive(PrimitiveType::U128) => "Bv128".to_string(),
                                    Type::Primitive(PrimitiveType::U256) => "Bv256".to_string(),
                                    Type::Primitive(_)
                                    | Type::Tuple(_)
                                    | Type::Vector(_)
                                    | Type::Struct(_, _, _)
                                    | Type::TypeParameter(_)
                                    | Type::Reference(_, _)
                                    | Type::Fun(..)
                                    | Type::TypeDomain(_)
                                    | Type::ResourceDomain(_, _, _)
                                    | Type::Error
                                    | Type::Var(_) => unreachable!(),
                                }
                            } else {
                                "".to_string()
                            };
                            emitln!(
                                writer,
                                "call {} := {}{}({}, {});",
                                str_local(dest),
                                comp_oper,
                                lt_type,
                                str_local(op1),
                                str_local(op2)
                            );
                        };
                        let comp_oper = match oper {
                            Lt => "$Lt",
                            Le => "$Le",
                            Gt => "$Gt",
                            Ge => "$Ge",
                            _ => unreachable!(),
                        };
                        make_comparison(comp_oper, op1, op2, dest);
                    },
                    Or => {
                        let dest = dests[0];
                        let op1 = srcs[0];
                        let op2 = srcs[1];
                        emitln!(
                            writer,
                            "call {} := $Or({}, {});",
                            str_local(dest),
                            str_local(op1),
                            str_local(op2)
                        );
                    },
                    And => {
                        let dest = dests[0];
                        let op1 = srcs[0];
                        let op2 = srcs[1];
                        emitln!(
                            writer,
                            "call {} := $And({}, {});",
                            str_local(dest),
                            str_local(op1),
                            str_local(op2)
                        );
                    },
                    Eq | Neq => {
                        let dest = dests[0];
                        let op1 = srcs[0];
                        let op2 = srcs[1];
                        let num_oper = global_state
                            .get_temp_index_oper(mid, fid, op1, baseline_flag)
                            .unwrap();
                        let bv_flag = self.bv_flag(num_oper);
                        let oper = boogie_equality_for_type(
                            env,
                            oper == &Eq,
                            &self.get_local_type(op1),
                            bv_flag,
                        );
                        emitln!(
                            writer,
                            "{} := {}({}, {});",
                            str_local(dest),
                            oper,
                            str_local(op1),
                            str_local(op2)
                        );
                    },
                    Xor | BitOr | BitAnd => {
                        let dest = dests[0];
                        let op1 = srcs[0];
                        let op2 = srcs[1];
                        let make_bitwise =
                            |bv_oper: &str, op1: TempIndex, op2: TempIndex, dest: TempIndex| {
                                let base = match &self.get_local_type(dest) {
                                    Type::Primitive(PrimitiveType::U8) => "Bv8".to_string(),
                                    Type::Primitive(PrimitiveType::U16) => "Bv16".to_string(),
                                    Type::Primitive(PrimitiveType::U32) => "Bv32".to_string(),
                                    Type::Primitive(PrimitiveType::U64) => "Bv64".to_string(),
                                    Type::Primitive(PrimitiveType::U128) => "Bv128".to_string(),
                                    Type::Primitive(PrimitiveType::U256) => "Bv256".to_string(),
                                    Type::Primitive(_)
                                    | Type::Tuple(_)
                                    | Type::Vector(_)
                                    | Type::Struct(_, _, _)
                                    | Type::TypeParameter(_)
                                    | Type::Reference(_, _)
                                    | Type::Fun(..)
                                    | Type::TypeDomain(_)
                                    | Type::ResourceDomain(_, _, _)
                                    | Type::Error
                                    | Type::Var(_) => unreachable!(),
                                };
                                let op1_ty = &self.get_local_type(op1);
                                let op2_ty = &self.get_local_type(op2);
                                let num_oper_1 = global_state
                                    .get_temp_index_oper(mid, fid, op1, baseline_flag)
                                    .unwrap();
                                let op1_bv_flag = self.bv_flag(num_oper_1);
                                let num_oper_2 = global_state
                                    .get_temp_index_oper(mid, fid, op2, baseline_flag)
                                    .unwrap();
                                let op2_bv_flag = self.bv_flag(num_oper_2);
                                let op1_str = if !op1_bv_flag {
                                    format!(
                                        "$int2bv.{}({})",
                                        boogie_num_type_base(
                                            self.parent.env,
                                            Some(self.fun_target.get_bytecode_loc(attr_id)),
                                            op1_ty
                                        ),
                                        str_local(op1)
                                    )
                                } else {
                                    str_local(op1)
                                };
                                let op2_str = if !op2_bv_flag {
                                    format!(
                                        "$int2bv.{}({})",
                                        boogie_num_type_base(
                                            self.parent.env,
                                            Some(self.fun_target.get_bytecode_loc(attr_id)),
                                            op2_ty
                                        ),
                                        str_local(op2)
                                    )
                                } else {
                                    str_local(op2)
                                };
                                emitln!(
                                    writer,
                                    "call {} := {}{}({}, {});",
                                    str_local(dest),
                                    bv_oper,
                                    base,
                                    op1_str,
                                    op2_str
                                );
                            };
                        let bv_oper_str = match oper {
                            Xor => "$Xor",
                            BitOr => "$Or",
                            BitAnd => "$And",
                            _ => unreachable!(),
                        };
                        make_bitwise(bv_oper_str, op1, op2, dest);
                    },
                    Uninit => {
                        emitln!(writer, "assume $t{}->l == $Uninitialized();", srcs[0]);
                    },
                    Drop | Release => {},
                    TraceLocal(idx) => {
                        let num_oper = global_state
                            .get_temp_index_oper(mid, fid, srcs[0], baseline_flag)
                            .unwrap();
                        let bv_flag = self.bv_flag(num_oper);
                        self.track_local(*idx, srcs[0], bv_flag);
                    },
                    TraceReturn(i) => {
                        let oper_map = global_state.get_ret_map();
                        let bv_flag = self.bv_flag_from_map(&srcs[0], oper_map);
                        self.track_return(*i, srcs[0], bv_flag);
                    },
                    TraceAbort => self.track_abort(&str_local(srcs[0])),
                    TraceExp(kind, node_id) => {
                        let bv_flag = *global_state
                            .get_temp_index_oper(mid, fid, srcs[0], baseline_flag)
                            .unwrap()
                            == Bitwise;
                        self.track_exp(*kind, *node_id, srcs[0], bv_flag)
                    },
                    EmitEvent => {
                        let msg = srcs[0];
                        let handle = srcs[1];
                        let suffix = boogie_type_suffix(env, &self.get_local_type(msg));
                        emit!(
                            writer,
                            "$es := ${}ExtendEventStore'{}'($es, ",
                            if srcs.len() > 2 { "Cond" } else { "" },
                            suffix
                        );
                        emit!(writer, "{}, {}", str_local(handle), str_local(msg));
                        if srcs.len() > 2 {
                            emit!(writer, ", {}", str_local(srcs[2]));
                        }
                        emitln!(writer, ");");
                    },
                    EventStoreDiverge => {
                        emitln!(writer, "call $es := $EventStore__diverge($es);");
                    },
                    TraceGlobalMem(mem) => {
                        let mem = &mem.to_owned().instantiate(self.type_inst);
                        let node_id = env.new_node(env.unknown_loc(), mem.to_type());
                        self.track_global_mem(mem, node_id);
                    },
                    Vector => unimplemented!("vector"),
                }
                if let Some(AbortAction(target, code)) = aa {
                    emitln!(writer, "if ($abort_flag) {");
                    writer.indent();
                    *last_tracked_loc = None;
                    self.track_loc(last_tracked_loc, &loc);
                    let num_oper_code = global_state
                        .get_temp_index_oper(mid, fid, *code, baseline_flag)
                        .unwrap();
                    let bv2int_str = if *num_oper_code == Bitwise {
                        format!(
                            "$int2bv.{}($abort_code)",
                            boogie_num_type_base(
                                self.parent.env,
                                Some(self.fun_target.get_bytecode_loc(attr_id)),
                                &self.get_local_type(*code)
                            )
                        )
                    } else {
                        "$abort_code".to_string()
                    };
                    let code_str = str_local(*code);
                    emitln!(writer, "{} := {};", code_str, bv2int_str);
                    self.track_abort(&code_str);
                    emitln!(writer, "goto L{};", target.as_usize());
                    writer.unindent();
                    emitln!(writer, "}");
                }
            },
            Abort(_, src) => {
                let num_oper_code = global_state
                    .get_temp_index_oper(mid, fid, *src, baseline_flag)
                    .unwrap();
                let int2bv_str = if *num_oper_code == Bitwise {
                    format!(
                        "$bv2int.{}({})",
                        boogie_num_type_base(
                            self.parent.env,
                            Some(self.fun_target.get_bytecode_loc(attr_id)),
                            &self.get_local_type(*src)
                        ),
                        str_local(*src)
                    )
                } else {
                    str_local(*src)
                };
                emitln!(writer, "$abort_code := {};", int2bv_str);
                emitln!(writer, "$abort_flag := true;");
                emitln!(writer, "return;")
            },
            Nop(..) => {},
            SpecBlock(..) => {
                // spec blocks should only appear in bytecode during compilation
                // to Move bytecode, so bail out.
                panic!("unexpected spec block")
            },
        }
        emitln!(writer);
    }

    fn try_reflection_call(
        &self,
        writer: &CodeWriter,
        env: &GlobalEnv,
        inst: &[Type],
        callee_env: &FunctionEnv,
        dest_str: &String,
    ) -> bool {
        let module_env = &callee_env.module_env;
        let mk_qualified_name = || {
            format!(
                "{}::{}",
                module_env.get_name().name().display(env.symbol_pool()),
                callee_env.get_name().display(env.symbol_pool()),
            )
        };
        if env.get_extlib_address() == *module_env.get_name().addr() {
            let qualified_name = mk_qualified_name();
            if qualified_name == TYPE_NAME_MOVE {
                assert_eq!(inst.len(), 1);
                if dest_str.is_empty() {
                    emitln!(
                        writer,
                        "{}",
                        boogie_reflection_type_name(env, &inst[0], false)
                    );
                } else {
                    emitln!(
                        writer,
                        "{} := {};",
                        dest_str,
                        boogie_reflection_type_name(env, &inst[0], false)
                    );
                }
                return true;
            } else if qualified_name == TYPE_INFO_MOVE {
                assert_eq!(inst.len(), 1);
                let (flag, info) = boogie_reflection_type_info(env, &inst[0]);
                emitln!(writer, "if (!{}) {{", flag);
                writer.with_indent(|| emitln!(writer, "call $ExecFailureAbort();"));
                emitln!(writer, "}");
                if !dest_str.is_empty() {
                    emitln!(writer, "else {");
                    writer.with_indent(|| emitln!(writer, "{} := {};", dest_str, info));
                    emitln!(writer, "}");
                }
                return true;
            }
        }

        if env.get_stdlib_address() == *module_env.get_name().addr() {
            let qualified_name = mk_qualified_name();
            if qualified_name == TYPE_NAME_GET_MOVE {
                assert_eq!(inst.len(), 1);
                if dest_str.is_empty() {
                    emitln!(
                        writer,
                        "{}",
                        boogie_reflection_type_name(env, &inst[0], true)
                    );
                } else {
                    emitln!(
                        writer,
                        "{} := {};",
                        dest_str,
                        boogie_reflection_type_name(env, &inst[0], true)
                    );
                }
                return true;
            }
        }

        false
    }

    fn translate_write_back(&self, dest: &BorrowNode, edge: &BorrowEdge, src: TempIndex) {
        use BorrowNode::*;
        let writer = self.parent.writer;
        let env = self.parent.env;
        let src_str = format!("$t{}", src);
        match dest {
            ReturnPlaceholder(_) => {
                unreachable!("unexpected transient borrow node")
            },
            GlobalRoot(memory) => {
                assert!(matches!(edge, BorrowEdge::Direct));
                let memory = &memory.to_owned().instantiate(self.type_inst);
                let memory_name = boogie_resource_memory_name(env, memory, &None);
                emitln!(
                    writer,
                    "{} := $ResourceUpdate({}, $GlobalLocationAddress({}),\n    \
                                     $Dereference({}));",
                    memory_name,
                    memory_name,
                    src_str,
                    src_str
                );
            },
            LocalRoot(idx) => {
                assert!(matches!(edge, BorrowEdge::Direct));
                emitln!(writer, "$t{} := $Dereference({});", idx, src_str);
            },
            Reference(idx) => {
                let dst_value = format!("$Dereference($t{})", idx);
                let src_value = format!("$Dereference({})", src_str);
                let get_path_index = |offset: usize| {
                    if offset == 0 {
                        format!("ReadVec({}->p, LenVec($t{}->p))", src_str, idx)
                    } else {
                        format!("ReadVec({}->p, LenVec($t{}->p) + {})", src_str, idx, offset)
                    }
                };
                let update = if let BorrowEdge::Hyper(edges) = edge {
                    self.translate_write_back_update(
                        &mut || dst_value.clone(),
                        &get_path_index,
                        src_value,
                        edges,
                        0,
                    )
                } else {
                    self.translate_write_back_update(
                        &mut || dst_value.clone(),
                        &get_path_index,
                        src_value,
                        &[edge.to_owned()],
                        0,
                    )
                };
                emitln!(
                    writer,
                    "$t{} := $UpdateMutation($t{}, {});",
                    idx,
                    idx,
                    update
                );
            },
        }
    }

    fn check_intrinsic_select(&self, attr_id: AttrId, struct_env: &StructEnv) {
        if struct_env.is_intrinsic() && self.fun_target.global_env().generated_by_v2() {
            // There is code in the framework which produces this warning.
            // Only report if we are running v2.
            self.parent.env.diag(
                Severity::Warning,
                &self.fun_target.get_bytecode_loc(attr_id),
                "cannot select field of intrinsic struct",
            )
        }
    }

    /// Returns read aggregate and write aggregate if fun_env matches one of the native functions
    /// implementing custom mutable borrow.
    fn get_borrow_native_aggregate_names(&self, fn_name: &String) -> Option<(String, String)> {
        for f in &self.parent.options.borrow_aggregates {
            if &f.name == fn_name {
                return Some((f.read_aggregate.clone(), f.write_aggregate.clone()));
            }
        }
        None
    }

    fn translate_write_back_update(
        &self,
        mk_dest: &mut dyn FnMut() -> String,
        get_path_index: &dyn Fn(usize) -> String,
        src: String,
        edges: &[BorrowEdge],
        at: usize,
    ) -> String {
        if at >= edges.len() {
            src
        } else {
            match &edges[at] {
                BorrowEdge::Direct => {
                    self.translate_write_back_update(mk_dest, get_path_index, src, edges, at + 1)
                },
                BorrowEdge::Field(memory, variant, offset) => {
                    let memory = memory.to_owned().instantiate(self.type_inst);
                    let struct_env = &self.parent.env.get_struct_qid(memory.to_qualified_id());
                    let field_env = if variant.is_none() {
                        struct_env.get_field_by_offset_optional_variant(None, *offset)
                    } else {
                        struct_env.get_field_by_offset_optional_variant(
                            Some(variant.clone().unwrap()[0]),
                            *offset,
                        )
                    };
                    let field_sel = boogie_field_sel(&field_env);
                    let new_dest = format!("{}->{}", (*mk_dest)(), field_sel);
                    let mut new_dest_needed = false;
                    let new_src = self.translate_write_back_update(
                        &mut || {
                            new_dest_needed = true;
                            format!("$$sel{}", at)
                        },
                        get_path_index,
                        src,
                        edges,
                        at + 1,
                    );
                    let update_fun = if variant.is_none() {
                        boogie_field_update(&field_env, &memory.inst)
                    } else {
                        // When updating a field of an enum variant,
                        // the type of the field is required to call the correct update function
                        let global_state = &self
                            .parent
                            .env
                            .get_extension::<GlobalNumberOperationState>()
                            .expect("global number operation state");
                        let type_name = boogie_type_for_struct_field(
                            global_state,
                            &field_env,
                            self.parent.env,
                            &field_env.get_type(),
                        );
                        boogie_variant_field_update(&field_env, type_name, &memory.inst)
                    };

                    if new_dest_needed {
                        format!(
                            "(var $$sel{} := {}; {}({}, {}))",
                            at,
                            new_dest,
                            update_fun,
                            (*mk_dest)(),
                            new_src
                        )
                    } else {
                        format!("{}({}, {})", update_fun, (*mk_dest)(), new_src)
                    }
                },
                BorrowEdge::Index(index_edge_kind) => {
                    // Index edge is used for both vectors, tables, and custom native methods
                    // implementing similar functionality (mutable borrow). Determine which
                    // operations to use to read and update.
                    let (read_aggregate, update_aggregate) = match index_edge_kind {
                        IndexEdgeKind::Vector => ("ReadVec".to_string(), "UpdateVec".to_string()),
                        IndexEdgeKind::Table => ("GetTable".to_string(), "UpdateTable".to_string()),
                        IndexEdgeKind::Custom(name) => {
                            // panic here means that custom borrow natives options were not specified properly
                            self.get_borrow_native_aggregate_names(name).unwrap()
                        },
                    };

                    // Compute the offset into the path where to retrieve the index.
                    let offset = edges[0..at]
                        .iter()
                        .filter(|e| !matches!(e, BorrowEdge::Direct))
                        .count();
                    let index = (*get_path_index)(offset);
                    let new_dest = format!("{}({}, {})", read_aggregate, (*mk_dest)(), index);
                    let mut new_dest_needed = false;
                    // Recursively perform write backs for next edges
                    let new_src = self.translate_write_back_update(
                        &mut || {
                            new_dest_needed = true;
                            format!("$$sel{}", at)
                        },
                        get_path_index,
                        src,
                        edges,
                        at + 1,
                    );
                    if new_dest_needed {
                        format!(
                            "(var $$sel{} := {}; {}({}, {}, {}))",
                            at,
                            new_dest,
                            update_aggregate,
                            (*mk_dest)(),
                            index,
                            new_src
                        )
                    } else {
                        format!(
                            "{}({}, {}, {})",
                            update_aggregate,
                            (*mk_dest)(),
                            index,
                            new_src
                        )
                    }
                },
                BorrowEdge::Hyper(_) => unreachable!("unexpected borrow edge"),
            }
        }
    }

    /// Track location for execution trace, avoiding to track the same line multiple times.
    fn track_loc(&self, last_tracked_loc: &mut Option<(Loc, LineIndex)>, loc: &Loc) {
        let env = self.fun_target.global_env();
        if let Some(l) = env.get_location(loc) {
            if let Some((last_loc, last_line)) = last_tracked_loc {
                if *last_line == l.line {
                    // This line already tracked.
                    return;
                }
                *last_loc = loc.clone();
                *last_line = l.line;
            } else {
                *last_tracked_loc = Some((loc.clone(), l.line));
            }
            emitln!(
                self.parent.writer,
                "assume {{:print \"$at{}\"}} true;",
                self.loc_str(loc)
            );
        }
    }

    fn track_abort(&self, code_var: &str) {
        emitln!(
            self.parent.writer,
            &boogie_debug_track_abort(self.fun_target, code_var)
        );
    }

    /// Generates an update of the debug information about temporary.
    fn track_local(&self, origin_idx: TempIndex, idx: TempIndex, bv_flag: bool) {
        emitln!(
            self.parent.writer,
            &boogie_debug_track_local(
                self.fun_target,
                origin_idx,
                idx,
                &self.get_local_type(idx),
                bv_flag
            )
        );
    }

    /// Generates an update of the debug information about the return value at given location.
    fn track_return(&self, return_idx: usize, idx: TempIndex, bv_flag: bool) {
        emitln!(
            self.parent.writer,
            &boogie_debug_track_return(
                self.fun_target,
                return_idx,
                idx,
                &self.get_local_type(idx),
                bv_flag
            )
        );
    }

    /// Generates the bytecode to print out the value of mem.
    fn track_global_mem(&self, mem: &QualifiedInstId<StructId>, node_id: NodeId) {
        let env = self.parent.env;
        let temp_str = boogie_resource_memory_name(env, mem, &None);
        emitln!(
            self.parent.writer,
            "assume {{:print \"$track_global_mem({}):\", {}}} true;",
            node_id.as_usize(),
            temp_str,
        );
    }

    fn track_exp(&self, kind: TraceKind, node_id: NodeId, temp: TempIndex, bv_flag: bool) {
        let env = self.parent.env;
        let writer = self.parent.writer;
        let ty = self.get_local_type(temp);
        let temp_str = if ty.is_reference() {
            let new_temp = boogie_temp(env, ty.skip_reference(), 0, bv_flag);
            emitln!(writer, "{} := $Dereference($t{});", new_temp, temp);
            new_temp
        } else {
            format!("$t{}", temp)
        };
        let suffix = if kind == TraceKind::SubAuto {
            "_sub"
        } else {
            ""
        };
        emitln!(
            self.parent.writer,
            "assume {{:print \"$track_exp{}({}):\", {}}} true;",
            suffix,
            node_id.as_usize(),
            temp_str,
        );
    }

    fn loc_str(&self, loc: &Loc) -> String {
        let file_idx = self.fun_target.global_env().file_id_to_idx(loc.file_id());
        format!("({},{},{})", file_idx, loc.span().start(), loc.span().end())
    }

    fn compute_needed_temps(&self) -> BTreeMap<(String, bool), (Type, bool, usize)> {
        use Bytecode::*;
        use Operation::*;

        let fun_target = self.fun_target;
        let env = fun_target.global_env();

        let mut res: BTreeMap<(String, bool), (Type, bool, usize)> = BTreeMap::new();
        let mut need = |ty: &Type, bv_flag: bool, n: usize| {
            // Index by type suffix, which is more coarse grained then type.
            let ty = ty.skip_reference();
            let suffix = boogie_type_suffix(env, ty);
            let cnt = res
                .entry((suffix, bv_flag))
                .or_insert_with(|| (ty.to_owned(), bv_flag, 0));
            cnt.2 = cnt.2.max(n);
        };
        let baseline_flag = self.fun_target.data.variant == FunctionVariant::Baseline;
        let global_state = &self
            .fun_target
            .global_env()
            .get_extension::<GlobalNumberOperationState>()
            .expect("global number operation state");
        let ret_oper_map = &global_state.get_ret_map();
        let mid = fun_target.func_env.module_env.get_id();
        let fid = fun_target.func_env.get_id();

        for bc in &fun_target.data.code {
            match bc {
                Call(_, dests, oper, srcs, ..) => match oper {
                    TraceExp(_, id) => {
                        let ty = &self.inst(&env.get_node_type(*id));
                        let bv_flag = global_state.get_node_num_oper(*id) == Bitwise;
                        need(ty, bv_flag, 1)
                    },
                    TraceReturn(idx) => {
                        let ty = &self.inst(&fun_target.get_return_type(*idx));
                        let bv_flag = self.bv_flag_from_map(idx, ret_oper_map);
                        need(ty, bv_flag, 1)
                    },
                    TraceLocal(_) => {
                        let ty = &self.get_local_type(srcs[0]);
                        let num_oper = &global_state
                            .get_temp_index_oper(mid, fid, srcs[0], baseline_flag)
                            .unwrap();
                        let bv_flag = self.bv_flag(num_oper);
                        need(ty, bv_flag, 1)
                    },
                    Havoc(HavocKind::MutationValue) => {
                        let ty = &self.get_local_type(dests[0]);
                        let num_oper = &global_state
                            .get_temp_index_oper(mid, fid, dests[0], baseline_flag)
                            .unwrap();
                        let bv_flag = self.bv_flag(num_oper);
                        need(ty, bv_flag, 1)
                    },
                    _ => {},
                },
                Prop(_, PropKind::Modifies, exp) => {
                    let bv_flag = global_state.get_node_num_oper(exp.node_id()) == Bitwise;
                    need(&BOOL_TYPE, false, 1);
                    need(&self.inst(&env.get_node_type(exp.node_id())), bv_flag, 1)
                },
                _ => {},
            }
        }
        res
    }
}

fn struct_has_native_equality(
    struct_env: &StructEnv<'_>,
    inst: &[Type],
    options: &BoogieOptions,
) -> bool {
    if options.native_equality {
        // Everything has native equality
        return true;
    }
    if struct_env.has_variants() {
        for variant in struct_env.get_variants() {
            for field in struct_env.get_fields_of_variant(variant) {
                if !has_native_equality(
                    struct_env.module_env.env,
                    options,
                    &field.get_type().instantiate(inst),
                ) {
                    return false;
                }
            }
        }
    } else {
        for field in struct_env.get_fields() {
            if !has_native_equality(
                struct_env.module_env.env,
                options,
                &field.get_type().instantiate(inst),
            ) {
                return false;
            }
        }
    }
    true
}

pub fn has_native_equality(env: &GlobalEnv, options: &BoogieOptions, ty: &Type) -> bool {
    if options.native_equality {
        // Everything has native equality
        return true;
    }
    match ty {
        Type::Vector(..) => false,
        Type::Struct(mid, sid, sinst) => {
            struct_has_native_equality(&env.get_struct_qid(mid.qualified(*sid)), sinst, options)
        },
        Type::Primitive(_)
        | Type::Tuple(_)
        | Type::TypeParameter(_)
        | Type::Reference(_, _)
        | Type::Fun(..)
        | Type::TypeDomain(_)
        | Type::ResourceDomain(_, _, _)
        | Type::Error
        | Type::Var(_) => true,
    }
}
