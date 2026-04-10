// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This module translates the bytecode of a module to Boogie code.

use crate::{
    boogie_helpers::{
        boogie_address, boogie_address_blob, boogie_behavioral_eval_fun_name,
        boogie_behavioral_fun_result_name, boogie_behavioral_fun_spec_name,
        boogie_behavioral_result_fun_name, boogie_behavioral_spec_fun_name, boogie_byte_blob,
        boogie_closure_pack_name, boogie_constant_blob, boogie_debug_track_abort,
        boogie_debug_track_local, boogie_debug_track_return, boogie_equality_for_type,
        boogie_field_sel, boogie_field_update, boogie_fun_apply_name, boogie_fun_param_name,
        boogie_function_name, boogie_make_vec_from_strings, boogie_modifies_memory_name,
        boogie_num_literal, boogie_num_type_base, boogie_num_type_string_capital,
        boogie_reflection_type_info, boogie_reflection_type_name, boogie_resource_memory_name,
        boogie_spec_fun_name, boogie_struct_field_name, boogie_struct_field_result_fun_name,
        boogie_struct_field_spec_fun_name, boogie_struct_name, boogie_struct_variant_name,
        boogie_temp, boogie_temp_from_suffix, boogie_type, boogie_type_for_struct_field,
        boogie_type_param, boogie_type_suffix, boogie_type_suffix_for_struct,
        boogie_type_suffix_for_struct_variant, boogie_variant_field_update,
        boogie_well_formed_check, boogie_well_formed_expr, compute_evaluator_memory_union,
        field_bv_flag_global_state, TypeIdentToken,
    },
    options::BoogieOptions,
    spec_translator::{LabelInfo, SpecTranslator},
};
use codespan::LineIndex;
use codespan_reporting::diagnostic::Severity;
use itertools::Itertools;
use legacy_move_compiler::interface_generator::NATIVE_INTERFACE;
#[allow(unused_imports)]
use log::{debug, info, log, warn, Level};
use move_core_types::{ability::AbilitySet, function::ClosureMask};
use move_model::{
    ast::{
        Attribute, BehaviorKind, ConditionKind, Exp, ExpData, FrameAccessKind, FunParamAccessOf,
        MemoryLabel, Operation as AstOperation, Pattern, TempIndex, TraceKind, Value,
    },
    code_writer::CodeWriter,
    emit, emitln,
    model::{
        FieldEnv, FunId, FunctionEnv, GlobalEnv, Loc, NodeId, Parameter, QualifiedInstId,
        StructEnv, StructId,
    },
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
    mono_analysis::{ClosureInfo, FunParamInfo, StructFieldInfo},
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

macro_rules! bv_op_not_enabled_error {
    ($bytecode:expr, $fun_target:expr, $env:expr, $loc:expr) => {
        unimplemented!(
            "bit vector not supported in operation: `{}` {}",
            $bytecode
                .display($fun_target, &::std::collections::BTreeMap::default())
                .to_string()
                .replace('\n', "\n// "),
            $loc.display($env)
        )
    };
}

/// Frame access kind for the apply procedure context, using Boogie-level address strings
/// instead of Move `Exp` trees. This is used when emitting frame conditions in the
/// `$apply` procedure body where addresses are Boogie variable references.
enum ApplyFrameAccess {
    /// Resource is only read: assume post == pre
    Reads,
    /// Resource may be written at any address: no frame constraint
    WritesAll,
    /// Resource may be written at specific Boogie address expressions only
    WritesAt(Vec<String>),
}

pub struct BoogieTranslator<'env> {
    env: &'env GlobalEnv,
    options: &'env BoogieOptions,
    for_shard: Option<usize>,
    writer: &'env CodeWriter,
    spec_translator: SpecTranslator<'env>,
    targets: &'env FunctionTargetsHolder,
    /// Map from function type to the set of function parameter infos for that type.
    /// Used to emit behavioral predicate assumptions at closure construction sites.
    fun_param_infos: BTreeMap<Type, BTreeSet<FunParamInfo>>,
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
            fun_param_infos: BTreeMap::new(),
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

        // Populate fun_param_infos for use during closure construction.
        // This enables emitting behavioral predicate assumptions at the call site.
        self.fun_param_infos = mono_info.fun_param_infos.clone();

        emitln!(
            writer,
            "\n\n//==================================\n// Begin Translation\n"
        );

        // Add type reflection axioms
        if !mono_info.type_params.is_empty() {
            emitln!(writer, "function $TypeName(t: $TypeParamInfo): Vec int;");

            // type name <-> type info: primitives
            for name in [
                "Bool", "U8", "U16", "U32", "U64", "U128", "U256", "I8", "I16", "I32", "I64",
                "I128", "I256", "Address", "Signer",
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
            let suffix = boogie_type_suffix(env, &Type::TypeParameter(*idx), false);
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
                // Emit uninterpreted function for arbitrary ordering of generic type
                emitln!(
                    writer,
                    "function $Arbitrary_cmp_ordering'{}'(v1: {}, v2: {}): $1_cmp_Ordering;",
                    suffix,
                    param_type,
                    param_type
                );

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
                        emitln!(writer, "else $Arbitrary_cmp_ordering'{}'(v1, v2)", suffix);
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
        let mut translated_memory: Vec<(QualifiedInstId<StructId>, String)> = vec![];
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
                    let struct_name = boogie_struct_name(struct_env, type_inst, false);
                    if !translated_types.insert(struct_name) {
                        continue;
                    }
                    if struct_env.has_memory() {
                        let mem_qid = struct_env.get_qualified_id().instantiate(type_inst.clone());
                        let mem_name = boogie_resource_memory_name(env, &mem_qid, &None);
                        translated_memory.push((mem_qid, mem_name))
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
                if fun_env.is_native_or_intrinsic()  // no Move body to translate
                    || fun_env.is_test_only()         // not part of production verification
                    || fun_env.is_not_prover_target()
                // inline/struct-api: no independent target
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
                            let fun_name = boogie_function_name(fun_env, type_inst, &[]);
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
        let empty_param_infos = BTreeSet::new();
        let empty_struct_field_infos = BTreeSet::new();
        for (fun_type, closure_infos) in &mono_info.fun_infos {
            let fun_param_infos = mono_info
                .fun_param_infos
                .get(fun_type)
                .unwrap_or(&empty_param_infos);
            let struct_field_infos = mono_info
                .fun_struct_field_infos
                .get(fun_type)
                .unwrap_or(&empty_struct_field_infos);
            self.translate_fun_type(
                fun_type,
                closure_infos,
                fun_param_infos,
                struct_field_infos,
                &translated_memory,
            )
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
        fun_param_infos: &BTreeSet<FunParamInfo>,
        struct_field_infos: &BTreeSet<StructFieldInfo>,
        memory: &[(QualifiedInstId<StructId>, String)],
    ) {
        emitln!(
            self.writer,
            "// Function type `{}`",
            fun_type.display(&self.env.get_type_display_ctx())
        );

        // Create data type for the function type.
        let fun_ty_boogie_name = boogie_type(self.env, fun_type, false);
        emitln!(self.writer, "datatype {} {{", fun_ty_boogie_name);
        self.writer.indent();
        // Collect all variants first to handle commas properly
        let closure_count = closure_infos.len();
        let param_count = fun_param_infos.len();
        let struct_field_count = struct_field_infos.len();
        let total_count = closure_count + param_count + struct_field_count;

        for (idx, info) in closure_infos.iter().enumerate() {
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
                    boogie_type(self.env, captured_ty, false)
                )
            }
            // Only emit comma if not the last variant
            let is_last = idx == total_count - 1;
            emitln!(self.writer, "){}", if is_last { "" } else { "," });
        }
        // Add parameter variants for function-typed parameters of verification targets.
        // These enable behavioral predicate handling in the apply function.
        for (idx, info) in fun_param_infos.iter().enumerate() {
            let fun_env = self.env.get_function(info.fun.to_qualified_id());
            emitln!(
                self.writer,
                "// Parameter `{}` of function `{}`",
                info.param_sym.display(self.env.symbol_pool()),
                fun_env.get_name().display(self.env.symbol_pool())
            );
            // Parameter variants have no captured values, just a constant constructor
            // Only emit comma if not the last variant
            let is_last = closure_count + idx == total_count - 1;
            emitln!(
                self.writer,
                "{}(){}",
                boogie_fun_param_name(self.env, &info.fun, info.param_sym),
                if is_last { "" } else { "," }
            );
        }
        // Add struct field variants for storable function-typed fields.
        // These carry an `n: int` parameter to distinguish different struct instances.
        for (idx, info) in struct_field_infos.iter().enumerate() {
            let struct_env = self.env.get_struct_qid(info.struct_id.to_qualified_id());
            emitln!(
                self.writer,
                "// Struct field `{}` of `{}`",
                info.field_sym.display(self.env.symbol_pool()),
                struct_env.get_name().display(self.env.symbol_pool())
            );
            let is_last = closure_count + param_count + idx == total_count - 1;
            emitln!(
                self.writer,
                "{}(n: int){}",
                boogie_struct_field_name(self.env, &info.struct_id, info.field_sym),
                if is_last { "" } else { "," }
            );
        }
        if total_count == 0 {
            // Emit a dummy constructor to avoid an empty Boogie datatype, which
            // can happen when a function type is referenced but no closures or
            // function parameters are constructed for it.
            emitln!(self.writer, "$dummy'{}()'()", fun_ty_boogie_name);
        }
        self.writer.unindent();
        emitln!(self.writer, "}");
        emitln!(
            self.writer,
            "function $IsValid'{}'(x: {}): bool {{ true }}",
            boogie_type_suffix(self.env, fun_type, false),
            fun_ty_boogie_name
        );

        // Generate uninterpreted spec functions for behavioral predicates on function-typed parameters.
        // These are used for connecting closures to specs.
        self.generate_behavioral_spec_funs_for_params(fun_type, fun_param_infos);

        // Generate uninterpreted spec functions for behavioral predicates on struct field variants.
        self.generate_behavioral_spec_funs_for_struct_fields(fun_type, struct_field_infos);

        // Generate per-function behavioral spec functions for closure target functions.
        // These inline functions have concrete bodies derived from the function's spec.
        self.generate_behavioral_spec_funs_for_functions(fun_type, closure_infos);

        // Generate behavioral predicate evaluator functions that dispatch on closure/param/struct
        // field variants.
        self.generate_behavioral_predicate_evaluator(
            fun_type,
            closure_infos,
            fun_param_infos,
            struct_field_infos,
            BehaviorKind::RequiresOf,
        );
        self.generate_behavioral_predicate_evaluator(
            fun_type,
            closure_infos,
            fun_param_infos,
            struct_field_infos,
            BehaviorKind::AbortsOf,
        );
        self.generate_behavioral_predicate_evaluator(
            fun_type,
            closure_infos,
            fun_param_infos,
            struct_field_infos,
            BehaviorKind::EnsuresOf,
        );

        // Generate the uninterpreted result_of function and its connecting axiom.
        self.generate_result_of_function_and_axiom(
            fun_type,
            closure_infos,
            fun_param_infos,
            struct_field_infos,
        );

        // Create an apply procedure which dispatches to the appropriate closure implementation.
        emitln!(
            self.writer,
            "// Apply procedure for `{}`",
            fun_type.display(&self.env.get_type_display_ctx())
        );
        emit!(
            self.writer,
            "procedure {{:inline 1}} {}(",
            boogie_fun_apply_name(self.env, fun_type),
        );
        let Type::Fun(params, results, _abilities) = fun_type else {
            panic!("expected function type")
        };
        let params = params.clone().flatten();
        for (pos, mut ty) in params.iter().enumerate() {
            if pos > 0 {
                emit!(self.writer, ", ")
            }
            if ty.is_immutable_reference() {
                // Immutable references are eliminated in the compilation scheme, so skip
                ty = ty.skip_reference();
            }
            emit!(
                self.writer,
                "p{}: {}",
                pos,
                boogie_type(self.env, ty, false)
            )
        }
        if !params.is_empty() {
            emit!(self.writer, ", ")
        }
        emit!(self.writer, "fun: {}", fun_ty_boogie_name);
        emit!(self.writer, ") returns (");
        let mut result_locals = vec![];
        for (pos, mut ty) in results
            .clone()
            .flatten()
            .iter()
            // Mutable input parameters become output, so append them
            .chain(params.iter().filter(|p| p.is_mutable_reference()))
            .enumerate()
        {
            if pos > 0 {
                emit!(self.writer, ",")
            }
            let local_name = format!("r{}", pos);
            if ty.is_immutable_reference() {
                // Immutable references are eliminated in the compilation scheme, so skip
                ty = ty.skip_reference();
            }
            emit!(
                self.writer,
                "{}: {}",
                local_name,
                boogie_type(self.env, ty, false)
            );
            result_locals.push(local_name)
        }
        emitln!(self.writer, ") {");
        self.writer.indent();
        let result_str = result_locals.iter().cloned().join(", ");

        // Declare frame save variables for all resources that may need frame conditions.
        // This includes resources from specific variants AND all translated memory types
        // (for Reads frame conditions on memory types not referenced by a variant).
        let (frame_save_resources, old_memory_resources) =
            Self::collect_frame_and_old_memory(closure_infos, fun_param_infos, self.env);
        let mut declared_frame_vars = BTreeSet::new();
        for mem in &frame_save_resources {
            let mem_name = boogie_resource_memory_name(self.env, mem, &None);
            let struct_env = self.env.get_struct_qid(mem.to_qualified_id());
            let mem_type = format!(
                "$Memory {}",
                boogie_struct_name(&struct_env, &mem.inst, false)
            );
            emitln!(self.writer, "var {}_$frame: {};", mem_name, mem_type);
            declared_frame_vars.insert(mem_name);
        }
        // Declare frame variables for all translated memory types not already covered.
        // These are needed when a variant doesn't reference a memory type — the Reads
        // frame condition saves/restores the memory to preserve it after havoc.
        for (mem_qid, mem_name) in memory {
            if !declared_frame_vars.contains(mem_name) {
                let struct_env = self.env.get_struct_qid(mem_qid.to_qualified_id());
                let mem_type = format!(
                    "$Memory {}",
                    boogie_struct_name(&struct_env, &mem_qid.inst, false)
                );
                emitln!(self.writer, "var {}_$frame: {};", mem_name, mem_type);
            }
        }
        for mem in &old_memory_resources {
            let mem_name = boogie_resource_memory_name(self.env, mem, &None);
            let struct_env = self.env.get_struct_qid(mem.to_qualified_id());
            let mem_type = format!(
                "$Memory {}",
                boogie_struct_name(&struct_env, &mem.inst, false)
            );
            emitln!(self.writer, "var {}_$pre: {};", mem_name, mem_type);
        }
        // Also declare _$pre for any memory types needed by wildcard struct field variants
        // that aren't already covered by old_memory_resources.
        let declared_pre: BTreeSet<_> = old_memory_resources
            .iter()
            .map(|m| boogie_resource_memory_name(self.env, m, &None))
            .collect();
        for (mem_qid, mem_name) in memory {
            if !declared_pre.contains(mem_name) {
                let struct_env = self.env.get_struct_qid(mem_qid.to_qualified_id());
                let mem_type = format!(
                    "$Memory {}",
                    boogie_struct_name(&struct_env, &mem_qid.inst, false)
                );
                emitln!(self.writer, "var {}_$pre: {};", mem_name, mem_type);
            }
        }

        // Generate branches for all variants. Since the datatype is closed,
        // the last variant uses `else` without an explicit check (unless it's the only variant,
        // in which case we emit the body directly without any block).
        let closure_count = closure_infos.len();
        let param_count = fun_param_infos.len();
        let struct_field_count = struct_field_infos.len();
        let total_variants = closure_count + param_count + struct_field_count;

        for (idx, info) in closure_infos.iter().enumerate() {
            let is_last = idx == total_variants - 1;
            let is_only = total_variants == 1;
            let ctor_name = boogie_closure_pack_name(self.env, &info.fun, info.mask);
            if is_only {
                // Single variant: emit body directly without any block
            } else if is_last {
                // Last of multiple variants: use else without explicit check
                // The previous variant's closing already emitted `} else `, so just open the block
                emitln!(self.writer, "{{   // {}", ctor_name);
                self.writer.indent();
            } else {
                emitln!(self.writer, "if (fun is {}) {{", ctor_name);
                self.writer.indent();
            }
            let fun_env = &self.env.get_function(info.fun.to_qualified_id());
            let fun_name = boogie_function_name(fun_env, &info.fun.inst, &[]);
            let args = (0..info.mask.captured_count())
                .map(|pos| format!("fun->p{}", pos))
                .chain((0..params.len()).map(|pos| format!("p{}", pos)))
                .join(", ");
            let call_prefix = if result_str.is_empty() {
                String::new()
            } else {
                format!("{result_str} := ")
            };
            if fun_env.is_opaque() {
                self.emit_opaque_closure_body(
                    info,
                    fun_env,
                    &params,
                    results,
                    &result_locals,
                    memory,
                );
            } else {
                emitln!(self.writer, "call {}{}({});", call_prefix, fun_name, args);
            }
            if is_only {
                // Single variant: no closing needed
            } else {
                self.writer.unindent();
                if is_last {
                    emitln!(self.writer, "}");
                } else {
                    emitln!(self.writer, "} else ");
                }
            }
        }

        // Generate branches for function parameter variants using behavioral predicates.
        // We use a conservative approach: havoc results and abort_flag, then constrain
        // with behavioral predicates if they exist.
        for (idx, info) in fun_param_infos.iter().enumerate() {
            let is_last = closure_count + idx == total_variants - 1;
            let is_only = total_variants == 1;
            let ctor_name = boogie_fun_param_name(self.env, &info.fun, info.param_sym);
            if is_only {
                // Single variant: emit body directly without any block
            } else if is_last {
                // Last of multiple variants: use else without explicit check
                // The previous variant's closing already emitted `} else `, so just open the block
                emitln!(self.writer, "{{   // {}", ctor_name);
                self.writer.indent();
            } else {
                emitln!(self.writer, "if (fun is {}) {{", ctor_name);
                self.writer.indent();
            }
            // Build memory args for this param from access_of declarations.
            let (used_memory, old_memory) =
                Self::get_param_memory(self.env, &info.fun, info.param_sym);
            let mem_args = self.build_spec_memory_args(&used_memory, &old_memory, &[], &None, None);

            // Build the argument list for behavioral predicates.
            // Memory args come first, then data args.
            let data_args: Vec<String> = params
                .iter()
                .enumerate()
                .map(|(pos, ty)| {
                    if ty.is_mutable_reference() {
                        format!("$Dereference(p{})", pos)
                    } else {
                        format!("p{}", pos)
                    }
                })
                .collect();
            let bp_args = mem_args.iter().chain(data_args.iter()).join(", ");

            // Note: We do NOT assert requires_of here. The requires_of predicate describes
            // caller obligations in the spec (e.g., `requires requires_of<f>(x)`), not runtime
            // checks for the function body. When verifying a function, its spec preconditions
            // are assumed, and the body just uses the abstract behavioral semantics.

            // Always use aborts_of predicate for function parameters.
            let aborts_name = boogie_behavioral_spec_fun_name(
                self.env,
                &info.fun,
                info.param_sym,
                BehaviorKind::AbortsOf,
                &[],
            );
            let ensures_name = boogie_behavioral_spec_fun_name(
                self.env,
                &info.fun,
                info.param_sym,
                BehaviorKind::EnsuresOf,
                &[],
            );
            // Always use deterministic result functions for function parameters.
            let result_fun_name =
                boogie_behavioral_result_fun_name(self.env, &info.fun, info.param_sym, &[], false);
            let multi_result_fun_name =
                boogie_behavioral_result_fun_name(self.env, &info.fun, info.param_sym, &[], true);
            let explicit_results = results.clone().flatten();

            // Compute frame access for this function parameter from access_of declarations
            let frame_access =
                derive_param_frame_access(self.env, &info.fun, info.param_sym, &data_args);

            self.emit_behavioral_predicate_body(
                &aborts_name,
                &ensures_name,
                &bp_args,
                &result_fun_name,
                &multi_result_fun_name,
                &data_args,
                &used_memory,
                &old_memory,
                &result_locals,
                &explicit_results,
                &params,
                memory,
                &frame_access,
            );
            if is_only {
                // Single variant: no outer block to close
            } else {
                self.writer.unindent();
                if is_last {
                    emitln!(self.writer, "}");
                } else {
                    emitln!(self.writer, "} else ");
                }
            }
        }

        // Generate branches for struct field variants using behavioral predicates.
        // These are fully opaque with no known memory access (conservative WritesAll).
        for (idx, info) in struct_field_infos.iter().enumerate() {
            let is_last = closure_count + param_count + idx == total_variants - 1;
            let is_only = total_variants == 1;
            let ctor_name = boogie_struct_field_name(self.env, &info.struct_id, info.field_sym);
            if is_only {
                // Single variant: emit body directly without any block
            } else if is_last {
                emitln!(self.writer, "{{   // {}", ctor_name);
                self.writer.indent();
            } else {
                emitln!(self.writer, "if (fun is {}) {{", ctor_name);
                self.writer.indent();
            }
            // Build data args with `fun->n` prepended (the instance discriminator).
            let data_args: Vec<String> = std::iter::once("fun->n".to_string())
                .chain(params.iter().enumerate().map(|(pos, ty)| {
                    if ty.is_mutable_reference() {
                        format!("$Dereference(p{})", pos)
                    } else {
                        format!("p{}", pos)
                    }
                }))
                .collect();
            // Look up access declarations for this struct field. If the struct spec
            // has reads_of/modifies_of for this field, use them; otherwise assume pure.
            let struct_env_for_access = self.env.get_struct_qid(info.struct_id.to_qualified_id());
            let (used_memory, old_memory, frame_access) = derive_struct_field_frame_access(
                self.env,
                &struct_env_for_access,
                info.field_sym,
                &data_args,
            );

            // Build memory args for the pre-state (both old and current slots use same variable)
            let fun_mem_args =
                self.build_spec_memory_args(&used_memory, &old_memory, &[], &None, None);
            let bp_args = fun_mem_args.iter().chain(data_args.iter()).join(", ");

            let aborts_name = boogie_struct_field_spec_fun_name(
                self.env,
                &info.struct_id,
                info.field_sym,
                BehaviorKind::AbortsOf,
                &[],
            );
            let ensures_name = boogie_struct_field_spec_fun_name(
                self.env,
                &info.struct_id,
                info.field_sym,
                BehaviorKind::EnsuresOf,
                &[],
            );
            let result_fun_name = boogie_struct_field_result_fun_name(
                self.env,
                &info.struct_id,
                info.field_sym,
                &[],
                false,
            );
            let multi_result_fun_name = boogie_struct_field_result_fun_name(
                self.env,
                &info.struct_id,
                info.field_sym,
                &[],
                true,
            );
            let explicit_results = results.clone().flatten();

            self.emit_behavioral_predicate_body(
                &aborts_name,
                &ensures_name,
                &bp_args,
                &result_fun_name,
                &multi_result_fun_name,
                &data_args,
                &used_memory,
                &old_memory,
                &result_locals,
                &explicit_results,
                &params,
                memory,
                &frame_access,
            );
            if is_only {
                // Single variant: no outer block to close
            } else {
                self.writer.unindent();
                if is_last {
                    emitln!(self.writer, "}");
                } else {
                    emitln!(self.writer, "} else ");
                }
            }
        }
        self.writer.unindent();
        emitln!(self.writer, "}");
    }

    /// Build Boogie memory argument strings for spec functions / behavioral predicates.
    /// For each memory in `used_memory`, generates the variable name. If the memory also
    /// appears in `old_memory`, emits a pair (old, current) using the given label for old
    /// and `None` for current (or vice versa depending on context). When `inst` is non-empty,
    /// instantiates memory types. When `label` is `None`, both old and current use the
    /// same global variable (pre-call context).
    ///
    /// When `old_saves` is provided, resources in `old_memory` use the save name for the
    /// old slot and the current global variable for the current slot. This is used for
    /// post-havoc args where old state was saved before havoc.
    fn build_spec_memory_args(
        &self,
        used_memory: &BTreeSet<QualifiedInstId<StructId>>,
        old_memory: &BTreeSet<QualifiedInstId<StructId>>,
        inst: &[Type],
        label: &Option<MemoryLabel>,
        old_saves: Option<&BTreeMap<QualifiedInstId<StructId>, String>>,
    ) -> Vec<String> {
        let uses_old = !old_memory.is_empty();
        let mut mem_args = vec![];
        for mem in used_memory {
            let mem = &mem.clone().instantiate(inst);
            let name = boogie_resource_memory_name(self.env, mem, label);
            if uses_old && old_memory.contains(mem) {
                if let Some(saves) = old_saves {
                    // Post-havoc: old slot uses saved pre-state, current slot uses post-havoc
                    let old_name = saves.get(mem).cloned().unwrap_or_else(|| name.clone());
                    mem_args.push(old_name);
                    mem_args.push(name);
                } else {
                    // Pre-havoc: both slots use the same (current) variable
                    mem_args.push(name.clone());
                    mem_args.push(name);
                }
            } else {
                mem_args.push(name);
            }
        }
        mem_args
    }

    /// Emit the body for an opaque closure variant in the apply procedure.
    /// Uses behavioral predicates to model the call semantics.
    fn emit_opaque_closure_body(
        &self,
        info: &ClosureInfo,
        fun_env: &FunctionEnv,
        params: &[Type],
        results: &Type,
        result_locals: &[String],
        memory: &[(QualifiedInstId<StructId>, String)],
    ) {
        // Build args for ALL callee params by interleaving captured and non-captured
        // using ClosureMask::compose.
        let captured_args: Vec<String> = (0..info.mask.captured_count())
            .map(|pos| format!("fun->p{}", pos))
            .collect();
        let callee_param_tys = fun_env.get_parameter_types();
        let non_captured_args: Vec<String> = callee_param_tys
            .iter()
            .enumerate()
            .filter(|(i, _)| !info.mask.is_captured(*i))
            .enumerate()
            .map(|(pos, (_, ty))| {
                if ty.is_mutable_reference() {
                    format!("$Dereference(p{})", pos)
                } else {
                    format!("p{}", pos)
                }
            })
            .collect();
        let bp_arg_list = info
            .mask
            .compose(captured_args, non_captured_args)
            .expect("closure mask compose failed");

        // Build memory args (pre-state: both old and current slots use same variable)
        let fun_mem_args = self.build_spec_memory_args(
            fun_env.get_spec_used_memory(),
            fun_env.get_spec_old_memory(),
            &info.fun.inst,
            &None,
            None,
        );
        let bp_args = fun_mem_args.iter().chain(bp_arg_list.iter()).join(", ");

        let aborts_name =
            boogie_behavioral_fun_spec_name(self.env, &info.fun, BehaviorKind::AbortsOf);
        let ensures_name =
            boogie_behavioral_fun_spec_name(self.env, &info.fun, BehaviorKind::EnsuresOf);
        let result_fun_name = boogie_behavioral_fun_result_name(self.env, &info.fun, false);
        let multi_result_fun_name = boogie_behavioral_fun_result_name(self.env, &info.fun, true);
        let explicit_results = results.clone().flatten();

        // Compute frame access for this closure
        let frame_access_raw = derive_closure_frame_access(fun_env, &info.fun.inst);
        let frame_access = self.closure_frame_to_apply_frame(&frame_access_raw, &bp_arg_list);

        // Get spec memory for building post-state args
        let inst_used: BTreeSet<_> = fun_env
            .get_spec_used_memory()
            .iter()
            .map(|m| m.clone().instantiate(&info.fun.inst))
            .collect();
        let inst_old: BTreeSet<_> = fun_env
            .get_spec_old_memory()
            .iter()
            .map(|m| m.clone().instantiate(&info.fun.inst))
            .collect();

        self.emit_behavioral_predicate_body(
            &aborts_name,
            &ensures_name,
            &bp_args,
            &result_fun_name,
            &multi_result_fun_name,
            &bp_arg_list,
            &inst_used,
            &inst_old,
            result_locals,
            &explicit_results,
            params,
            memory,
            &frame_access,
        );
    }

    /// Convert `FrameAccessKind` (with Exp-level addresses) to `ApplyFrameAccess`
    /// (with Boogie-level address strings) for the apply procedure context.
    ///
    /// In the apply procedure, callee parameters are either captured (`fun->pN`) or
    /// non-captured (`pN`). The `bp_arg_list` contains the composed arguments in callee
    /// parameter order, so `Temporary(i)` in an address expression maps to `bp_arg_list[i]`.
    fn closure_frame_to_apply_frame(
        &self,
        frame_access: &BTreeMap<QualifiedInstId<StructId>, FrameAccessKind>,
        bp_arg_list: &[String],
    ) -> BTreeMap<QualifiedInstId<StructId>, ApplyFrameAccess> {
        let mut result = BTreeMap::new();
        for (mem, access) in frame_access {
            let apply_access = match access {
                FrameAccessKind::Reads => ApplyFrameAccess::Reads,
                FrameAccessKind::WritesAll => ApplyFrameAccess::WritesAll,
                FrameAccessKind::WritesAt(addrs) => {
                    // Substitute Temporary references in address expressions with
                    // the corresponding Boogie argument strings from bp_arg_list.
                    let boogie_addrs: Vec<String> = addrs
                        .iter()
                        .map(|addr| self.addr_exp_to_boogie(addr, bp_arg_list))
                        .collect();
                    ApplyFrameAccess::WritesAt(boogie_addrs)
                },
            };
            result.insert(mem.clone(), apply_access);
        }
        result
    }

    /// Convert a Move address expression to a Boogie string in the apply procedure context.
    /// `Temporary(i)` references map to `arg_list[i]`.
    fn addr_exp_to_boogie(&self, exp: &Exp, arg_list: &[String]) -> String {
        match exp.as_ref() {
            ExpData::Temporary(_, idx) => {
                if *idx < arg_list.len() {
                    arg_list[*idx].clone()
                } else {
                    format!("p{}", idx)
                }
            },
            ExpData::Value(_, Value::Address(addr)) => boogie_address(self.env, addr),
            ExpData::Call(node_id, AstOperation::SpecFunction(mid, fid, _), args) => {
                let module_env = self.env.get_module(*mid);
                let fun_name = boogie_spec_fun_name(
                    &module_env,
                    *fid,
                    &self.env.get_node_instantiation(*node_id),
                    false,
                );
                let translated_args: Vec<String> = args
                    .iter()
                    .map(|a| self.addr_exp_to_boogie(a, arg_list))
                    .collect();
                format!("{}({})", fun_name, translated_args.join(", "))
            },
            _ => {
                let loc = self.env.get_node_loc(exp.node_id());
                self.env.diag(
                    Severity::Bug,
                    &loc,
                    &format!(
                        "unsupported address expression in frame condition: {:?}",
                        exp
                    ),
                );
                "0".to_string()
            },
        }
    }

    /// Emit behavioral predicate call body: aborts_of check, memory havoc with frame
    /// conditions, result_of assignment, and ensures_of assumption. Used by both opaque
    /// closure variants and function parameter variants in the `$apply` procedure.
    ///
    /// The body is structured in two phases:
    /// - **Phase 1 (pre-havoc):** Check `aborts_of(pre_args)` where both old and current
    ///   memory slots refer to the same pre-state variable.
    /// - **Phase 2 (post-havoc):** Save pre-state for old_memory resources, havoc memory,
    ///   apply frame conditions, then assign `result_of(post_args)` and assume
    ///   `ensures_of(post_args, results)` where old slots use saved pre-state and current
    ///   slots use the post-havoc variables.
    ///
    /// `frame_access` maps resources to their access kind, which determines the frame
    /// condition emitted after havoc. Resources with `Reads` are constrained equal to
    /// their pre-havoc snapshot. Resources with `WritesAt` are constrained equal except
    /// at the specified addresses. Resources with `WritesAll` are unconstrained.
    #[allow(clippy::too_many_arguments)]
    fn emit_behavioral_predicate_body(
        &self,
        aborts_name: &str,
        ensures_name: &str,
        bp_args: &str,
        result_fun_name: &str,
        multi_result_fun_name: &str,
        data_args: &[String],
        used_memory: &BTreeSet<QualifiedInstId<StructId>>,
        old_memory: &BTreeSet<QualifiedInstId<StructId>>,
        result_locals: &[String],
        explicit_results: &[Type],
        params: &[Type],
        memory: &[(QualifiedInstId<StructId>, String)],
        frame_access: &BTreeMap<QualifiedInstId<StructId>, ApplyFrameAccess>,
    ) {
        let env = self.env;
        let explicit_result_count = explicit_results.len();
        let mut_ref_param_indices: Vec<usize> = params
            .iter()
            .enumerate()
            .filter(|(_, p)| p.is_mutable_reference())
            .map(|(idx, _)| idx)
            .collect();
        let first_mut_ref_param = mut_ref_param_indices.first().copied();

        // Compute which memory names are covered by frame_access
        let covered_by_frame: BTreeSet<String> = frame_access
            .keys()
            .map(|qid| boogie_resource_memory_name(env, qid, &None))
            .collect();

        // Phase 1: Check abort condition using pre-state args (both old and current are pre-state)
        emitln!(self.writer, "if ({}({})) {{", aborts_name, bp_args);
        self.writer.indent();
        emitln!(self.writer, "$abort_flag := true;");
        self.writer.unindent();
        emitln!(self.writer, "} else {");
        self.writer.indent();

        // Phase 2: Save pre-state, havoc, frame conditions, assign results, assume ensures

        // Save pre-state memory for old_memory resources (used for dual-state post-havoc args)
        let mut old_saves: BTreeMap<QualifiedInstId<StructId>, String> = BTreeMap::new();
        for mem in old_memory {
            let mem_name = boogie_resource_memory_name(env, mem, &None);
            let pre_name = format!("{}_$pre", mem_name);
            emitln!(self.writer, "{} := {};", pre_name, mem_name);
            old_saves.insert(mem.clone(), pre_name);
        }

        // Save memory for frame conditions before havoc.
        // For memory types NOT in frame_access, also save for Reads constraint (preserve
        // memory that this variant doesn't modify).
        let mut uncovered_saves: Vec<String> = vec![];
        for (_, mem_name) in memory {
            if !covered_by_frame.contains(mem_name) {
                let save_name = format!("{}_$frame", mem_name);
                emitln!(self.writer, "{} := {};", save_name, mem_name);
                uncovered_saves.push(mem_name.clone());
            }
        }
        let mut frame_saves: Vec<(
            String,
            String,
            &ApplyFrameAccess,
            &QualifiedInstId<StructId>,
        )> = vec![];
        for (mem, access) in frame_access {
            if matches!(access, ApplyFrameAccess::WritesAll) {
                continue;
            }
            let mem_name = boogie_resource_memory_name(env, mem, &None);
            let save_name = format!("{}_$frame", mem_name);
            emitln!(self.writer, "{} := {};", save_name, mem_name);
            frame_saves.push((save_name, mem_name, access, mem));
        }

        // Havoc memory since the function could modify anything
        for (_, mem_name) in memory {
            emitln!(self.writer, "havoc {};", mem_name)
        }

        // Emit Reads constraints for memory types not in frame_access.
        // These are memory types that this variant provably doesn't modify,
        // so they must be preserved after havoc.
        for mem_name in &uncovered_saves {
            let save_name = format!("{}_$frame", mem_name);
            emitln!(self.writer, "assume {} == {};", mem_name, save_name);
        }

        // Emit frame conditions: constrain unchanged memory after havoc
        for (save_name, mem_name, access, mem) in &frame_saves {
            match access {
                ApplyFrameAccess::Reads => {
                    emitln!(self.writer, "assume {} == {};", mem_name, save_name);
                },
                ApplyFrameAccess::WritesAt(addrs) => {
                    let struct_name = boogie_struct_name(
                        &env.get_struct_qid(mem.to_qualified_id()),
                        &mem.inst,
                        false,
                    );
                    emit!(self.writer, "assume (forall $$a: int :: ");
                    emit!(
                        self.writer,
                        "{{{}->domain[$$a]}} {{{}->contents[$$a]}} ",
                        mem_name,
                        mem_name
                    );
                    let mut first = true;
                    for addr in addrs {
                        if !first {
                            emit!(self.writer, " && ");
                        }
                        first = false;
                        emit!(self.writer, "$$a != {}", addr);
                    }
                    emitln!(
                        self.writer,
                        " ==> {}->domain[$$a] == {}->domain[$$a] && \
                         $IsEqual'{}'({}->contents[$$a], {}->contents[$$a]));",
                        save_name,
                        mem_name,
                        struct_name,
                        save_name,
                        mem_name
                    );
                },
                ApplyFrameAccess::WritesAll => unreachable!(),
            }
        }

        // Build post-state args: old slots use saved pre-state, current slots use post-havoc
        let post_mem_args = self.build_spec_memory_args(
            used_memory,
            old_memory,
            &[],
            &None,
            if old_saves.is_empty() {
                None
            } else {
                Some(&old_saves)
            },
        );
        let result_bp_args = post_mem_args.iter().chain(data_args.iter()).join(", ");

        // Assign results using result_of function (now using post-state args)
        if !result_locals.is_empty() {
            if result_locals.len() == 1 {
                let result_local = &result_locals[0];
                if explicit_result_count == 1 {
                    if explicit_results[0].is_mutable_reference() {
                        let base_param = first_mut_ref_param.unwrap_or(0);
                        emitln!(
                            self.writer,
                            "{} := $ChildMutation(p{}, -1, {}({}));",
                            result_local,
                            base_param,
                            result_fun_name,
                            result_bp_args
                        );
                    } else {
                        emitln!(
                            self.writer,
                            "{} := {}({});",
                            result_local,
                            result_fun_name,
                            result_bp_args
                        );
                    }
                } else {
                    // Mutable reference param output: wrap in $UpdateMutation
                    let param_idx = mut_ref_param_indices[0];
                    emitln!(
                        self.writer,
                        "{} := $UpdateMutation(p{}, {}({}));",
                        result_local,
                        param_idx,
                        result_fun_name,
                        result_bp_args
                    );
                }
            } else {
                // Multiple results: use tuple projection
                for (i, result_local) in result_locals.iter().enumerate() {
                    if i < explicit_result_count {
                        if explicit_results[i].is_mutable_reference() {
                            let base_param = first_mut_ref_param.unwrap_or(0);
                            emitln!(
                                self.writer,
                                "{} := $ChildMutation(p{}, -1, {}({})->${});",
                                result_local,
                                base_param,
                                multi_result_fun_name,
                                result_bp_args,
                                i
                            );
                        } else {
                            emitln!(
                                self.writer,
                                "{} := {}({})->${};",
                                result_local,
                                multi_result_fun_name,
                                result_bp_args,
                                i
                            );
                        }
                    } else {
                        let mut_ref_idx = i - explicit_result_count;
                        let param_idx = mut_ref_param_indices[mut_ref_idx];
                        emitln!(
                            self.writer,
                            "{} := $UpdateMutation(p{}, {}({})->${});",
                            result_local,
                            param_idx,
                            multi_result_fun_name,
                            result_bp_args,
                            i
                        );
                    }
                }
            }
        }

        // Build result args for ensures_of, dereferencing mutable reference results.
        // Behavioral predicates reason over plain values, not mutation types.
        let mut ensures_result_args: Vec<String> = Vec::new();
        for (i, result_local) in result_locals.iter().enumerate() {
            if i < explicit_result_count {
                if explicit_results[i].is_mutable_reference() {
                    ensures_result_args.push(format!("$Dereference({})", result_local));
                } else {
                    ensures_result_args.push(result_local.clone());
                }
            } else {
                // Mut ref param output: always a mutation, need dereference
                ensures_result_args.push(format!("$Dereference({})", result_local));
            }
        }

        // Assume ensures_of with post-state args and result values
        let ensures_args = if ensures_result_args.is_empty() {
            result_bp_args.clone()
        } else {
            format!("{}, {}", result_bp_args, ensures_result_args.join(", "))
        };
        emitln!(self.writer, "assume {}({});", ensures_name, ensures_args);

        self.writer.unindent();
        emitln!(self.writer, "}");
    }

    /// Generate a behavioral predicate evaluator function for a function type.
    /// This function dispatches on closure/param variants. Closure branches call
    /// per-function spec functions. Param branches call per-param spec functions.
    fn generate_behavioral_predicate_evaluator(
        &self,
        fun_type: &Type,
        closure_infos: &BTreeSet<ClosureInfo>,
        fun_param_infos: &BTreeSet<FunParamInfo>,
        struct_field_infos: &BTreeSet<StructFieldInfo>,
        kind: BehaviorKind,
    ) {
        let env = self.env;
        let Type::Fun(params, results, _abilities) = fun_type else {
            panic!("expected function type")
        };
        let params = params.clone().flatten();
        let results = results.clone().flatten();

        let fun_ty_boogie_name = boogie_type(env, fun_type, false);
        let eval_fun_name = boogie_behavioral_eval_fun_name(env, fun_type, kind);
        let behavioral_outputs = Self::behavioral_output_types(&params, &results);

        // Compute union of all variants' memory for the evaluator signature.
        // This includes both closure and param variants so the evaluator's inline
        // body can pass memory to per-function spec functions in closure branches.
        let (union_used_memory, union_old_memory) =
            Self::collect_union_memory(env, closure_infos, fun_param_infos, struct_field_infos);
        let (eval_mem_decls, _eval_mem_args) =
            Self::build_memory_params(env, &union_used_memory, &union_old_memory);

        // Build parameter declarations
        let mut param_decls: Vec<String> = eval_mem_decls;
        param_decls.push(format!("f: {}", fun_ty_boogie_name));
        let mut param_args = vec![];
        for (pos, ty) in params.iter().enumerate() {
            param_decls.push(format!(
                "p{}: {}",
                pos,
                boogie_type(env, ty.skip_reference(), false)
            ));
            param_args.push(format!("p{}", pos));
        }
        if kind == BehaviorKind::EnsuresOf {
            for (pos, ty) in behavioral_outputs.iter().enumerate() {
                param_decls.push(format!("r{}: {}", pos, boogie_type(env, ty, false)));
                param_args.push(format!("r{}", pos));
            }
        }

        emitln!(
            self.writer,
            "// Behavioral predicate evaluator for {} on `{}`",
            kind,
            fun_type.display(&env.get_type_display_ctx())
        );
        emit!(
            self.writer,
            "function {{:inline}} {}({}): bool {{",
            eval_fun_name,
            param_decls.join(", ")
        );
        self.writer.indent();
        emitln!(self.writer);

        let total_variants = closure_infos.len() + fun_param_infos.len() + struct_field_infos.len();
        let mut variant_idx = 0;

        // Closure branches: delegate to per-function spec functions
        for info in closure_infos {
            let ctor_name = boogie_closure_pack_name(env, &info.fun, info.mask);
            let is_last = variant_idx == total_variants - 1;
            if variant_idx == 0 && total_variants == 1 {
                // Single variant: emit directly
            } else if variant_idx == 0 {
                emit!(self.writer, "if (f is {}) then ", ctor_name);
            } else if is_last {
                emit!(self.writer, "else /* {} */ ", ctor_name);
            } else {
                emit!(self.writer, "else if (f is {}) then ", ctor_name);
            }

            // Build call to per-function spec function with proper args
            let bp_name = boogie_behavioral_fun_spec_name(env, &info.fun, kind);
            let fun_env = env.get_function(info.fun.to_qualified_id());
            let callee_param_tys = fun_env.get_parameter_types();
            let num_callee_params = callee_param_tys.len();

            // Build memory args for this function's spec memory (subset of evaluator's union)
            let fun_mem_args = Self::build_instantiated_memory_args(env, &fun_env, &info.fun.inst);

            // Build args: memory + all callee params (interleaved captured/non-captured)
            let mut call_args: Vec<String> = fun_mem_args;
            let mut captured_pos = 0;
            let mut non_captured_pos = 0;
            for i in 0..num_callee_params {
                if info.mask.is_captured(i) {
                    call_args.push(format!("f->p{}", captured_pos));
                    captured_pos += 1;
                } else {
                    call_args.push(format!("p{}", non_captured_pos));
                    non_captured_pos += 1;
                }
            }
            // For ensures_of: add result args
            if kind == BehaviorKind::EnsuresOf {
                call_args.extend(Self::behavioral_output_args(&params, &results));
            }
            emitln!(self.writer, "{}({})", bp_name, call_args.join(", "));
            variant_idx += 1;
        }

        // Param branches: delegate to per-param uninterpreted functions
        for info in fun_param_infos {
            let ctor_name = boogie_fun_param_name(env, &info.fun, info.param_sym);
            let is_last = variant_idx == total_variants - 1;
            if variant_idx == 0 && total_variants == 1 {
                // Single variant: emit directly
            } else if variant_idx == 0 {
                emit!(self.writer, "if (f is {}) then ", ctor_name);
            } else if is_last {
                emit!(self.writer, "else /* {} */ ", ctor_name);
            } else {
                emit!(self.writer, "else if (f is {}) then ", ctor_name);
            }

            let bp_name =
                boogie_behavioral_spec_fun_name(env, &info.fun, info.param_sym, kind, &[]);
            let per_param_mem_args: Vec<String> = {
                let (used, old) = Self::get_param_memory(env, &info.fun, info.param_sym);
                let (_, args) = Self::build_memory_params(env, &used, &old);
                args
            };
            let args = if kind == BehaviorKind::EnsuresOf {
                let mut args: Vec<String> = per_param_mem_args;
                for pos in 0..params.len() {
                    args.push(format!("p{}", pos));
                }
                args.extend(Self::behavioral_output_args(&params, &results));
                args.join(", ")
            } else {
                let mut args = per_param_mem_args;
                args.extend(param_args.iter().cloned());
                args.join(", ")
            };
            emitln!(self.writer, "{}({})", bp_name, args);
            variant_idx += 1;
        }

        // Struct field branches: delegate to per-struct-field uninterpreted functions.
        // These have `n: int` (instance id) prepended to args.
        for info in struct_field_infos {
            let ctor_name = boogie_struct_field_name(env, &info.struct_id, info.field_sym);
            let is_last = variant_idx == total_variants - 1;
            if variant_idx == 0 && total_variants == 1 {
                // Single variant: emit directly
            } else if variant_idx == 0 {
                emit!(self.writer, "if (f is {}) then ", ctor_name);
            } else if is_last {
                emit!(self.writer, "else /* {} */ ", ctor_name);
            } else {
                emit!(self.writer, "else if (f is {}) then ", ctor_name);
            }

            let bp_name =
                boogie_struct_field_spec_fun_name(env, &info.struct_id, info.field_sym, kind, &[]);
            // Build memory args for this struct field (if it has access declarations)
            let struct_env_for_field = env.get_struct_qid(info.struct_id.to_qualified_id());
            let field_access = struct_env_for_field.get_field_access_of();
            let access_decl = field_access.iter().find(|a| a.fun_param == info.field_sym);
            let mem_args: Vec<String> = if let Some(decl) = access_decl {
                Self::build_evaluator_memory_args_for_access(
                    env,
                    decl,
                    &union_used_memory,
                    &union_old_memory,
                )
            } else {
                vec![]
            };
            // Args: memory..., n (instance id from datatype field), then data args
            let args = if kind == BehaviorKind::EnsuresOf {
                let mut args = mem_args;
                args.push("f->n".to_string());
                for pos in 0..params.len() {
                    args.push(format!("p{}", pos));
                }
                args.extend(Self::behavioral_output_args(&params, &results));
                args.join(", ")
            } else {
                let mut args = mem_args;
                args.push("f->n".to_string());
                args.extend(param_args.iter().cloned());
                args.join(", ")
            };
            emitln!(self.writer, "{}({})", bp_name, args);
            variant_idx += 1;
        }

        self.writer.unindent();
        emitln!(self.writer, "}");
    }

    /// Generate the uninterpreted `result_of` function and its connecting axiom.
    fn generate_result_of_function_and_axiom(
        &self,
        fun_type: &Type,
        closure_infos: &BTreeSet<ClosureInfo>,
        fun_param_infos: &BTreeSet<FunParamInfo>,
        struct_field_infos: &BTreeSet<StructFieldInfo>,
    ) {
        let env = self.env;
        let Type::Fun(params, results, _abilities) = fun_type else {
            panic!("expected function type")
        };
        let params_flat = params.clone().flatten();
        let results_flat = results.clone().flatten();
        let all_outputs = Self::behavioral_output_types(&params_flat, &results_flat);

        if all_outputs.is_empty() {
            return;
        }

        let fun_ty_boogie_name = boogie_type(env, fun_type, false);

        // Compute union of all variants' memory for the result_of function.
        // Must match the evaluator's memory since the axiom references ensures_of.
        let (union_used_memory, union_old_memory) =
            Self::collect_union_memory(env, closure_infos, fun_param_infos, struct_field_infos);
        let (eval_mem_decls, eval_mem_args) =
            Self::build_memory_params(env, &union_used_memory, &union_old_memory);

        let result_of_name = boogie_behavioral_eval_fun_name(env, fun_type, BehaviorKind::ResultOf);
        let ensures_of_name =
            boogie_behavioral_eval_fun_name(env, fun_type, BehaviorKind::EnsuresOf);

        // Parameters: (memory..., fun, params...)
        let mut param_decls: Vec<String> = eval_mem_decls;
        param_decls.push(format!("f: {}", fun_ty_boogie_name));
        let mut param_args: Vec<String> = eval_mem_args;
        param_args.push("f".to_string());
        for (pos, ty) in params_flat.iter().enumerate() {
            param_decls.push(format!(
                "p{}: {}",
                pos,
                boogie_type(env, ty.skip_reference(), false)
            ));
            param_args.push(format!("p{}", pos));
        }

        let result_type = if all_outputs.len() == 1 {
            boogie_type(env, &all_outputs[0], false)
        } else {
            boogie_type(env, &Type::Tuple(all_outputs.clone()), false)
        };

        emitln!(
            self.writer,
            "function {}({}): {};",
            result_of_name,
            param_decls.join(", "),
            result_type
        );

        let result_of_call = format!("{}({})", result_of_name, param_args.join(", "));

        let axiom_body = if all_outputs.len() == 1 {
            format!(
                "{}({}, {})",
                ensures_of_name,
                param_args.join(", "),
                result_of_call
            )
        } else {
            let tuple_projections: Vec<String> = (0..all_outputs.len())
                .map(|i| format!("_r->${}", i))
                .collect();
            format!(
                "(var _r := {}; {}({}, {}))",
                result_of_call,
                ensures_of_name,
                param_args.join(", "),
                tuple_projections.join(", ")
            )
        };

        emitln!(
            self.writer,
            "axiom (forall {} :: {{{}}} {});",
            param_decls.join(", "),
            result_of_call,
            axiom_body
        );
    }

    /// Generate per-function behavioral spec functions for closure target functions.
    /// For each unique closure target function, generate inline Boogie functions
    /// with concrete bodies from the function's spec conditions.
    fn generate_behavioral_spec_funs_for_functions(
        &self,
        fun_type: &Type,
        closure_infos: &BTreeSet<ClosureInfo>,
    ) {
        let Type::Fun(_params, results, _abilities) = fun_type else {
            return;
        };
        let results_flat = results.clone().flatten();

        // Deduplicate by qualified function id (different masks may target the same function)
        let mut seen_funs: BTreeSet<QualifiedInstId<FunId>> = BTreeSet::new();
        for info in closure_infos {
            if !seen_funs.insert(info.fun.clone()) {
                continue;
            }
            let fun_env = self.env.get_function(info.fun.to_qualified_id());
            let closure_spec = fun_env.get_spec();
            let fun_param_tys: Vec<Type> = fun_env
                .get_parameter_types()
                .into_iter()
                .map(|ty| ty.instantiate(&info.fun.inst))
                .collect();

            // Get function's spec memory
            let used_memory = fun_env.get_spec_used_memory();
            let old_memory = fun_env.get_spec_old_memory();
            let inst_used: BTreeSet<_> = used_memory
                .iter()
                .map(|m| m.clone().instantiate(&info.fun.inst))
                .collect();
            let inst_old: BTreeSet<_> = old_memory
                .iter()
                .map(|m| m.clone().instantiate(&info.fun.inst))
                .collect();
            let (mem_param_decls, mem_args) =
                Self::build_memory_params(self.env, &inst_used, &inst_old);

            // Build parameter declarations for ALL of the function's parameters
            let mut input_param_decls = mem_param_decls.clone();
            let mut input_args = vec![];
            for (i, ty) in fun_param_tys.iter().enumerate() {
                input_param_decls.push(format!(
                    "p{}: {}",
                    i,
                    boogie_type(self.env, ty.skip_reference(), false)
                ));
                input_args.push(format!("p{}", i));
            }

            // Generate requires_of and aborts_of
            for kind in [BehaviorKind::RequiresOf, BehaviorKind::AbortsOf] {
                let bp_name = boogie_behavioral_fun_spec_name(self.env, &info.fun, kind);
                let body = self.translate_fun_spec_conditions(
                    &fun_env,
                    &closure_spec,
                    kind,
                    &info.fun.inst,
                    &inst_old,
                );
                emitln!(
                    self.writer,
                    "function {{:inline}} {}({}): bool {{ {} }}",
                    bp_name,
                    input_param_decls.join(", "),
                    body
                );
            }

            // Generate ensures_of with result functions.
            let all_result_type_refs =
                Self::behavioral_output_type_refs(&fun_param_tys, &results_flat);
            let all_result_types: Vec<String> =
                Self::behavioral_output_types(&fun_param_tys, &results_flat)
                    .into_iter()
                    .map(|ty| boogie_type(self.env, &ty, false))
                    .collect();

            let input_args_str = {
                let mut all_args = mem_args.clone();
                all_args.extend(input_args.iter().cloned());
                all_args.join(", ")
            };

            let precond = Self::validity_precondition(self.env, &fun_param_tys, "p");

            let ensures_fun_name =
                boogie_behavioral_fun_spec_name(self.env, &info.fun, BehaviorKind::EnsuresOf);

            // Build full parameter list including results
            let mut full_param_decls = input_param_decls.clone();
            for (i, result_type) in all_result_types.iter().enumerate() {
                full_param_decls.push(format!("r{}: {}", i, result_type));
            }

            // Memory + input args combined (for connecting axioms)
            let all_input_arg_names: Vec<String> = {
                let mut args = mem_args.clone();
                args.extend(input_args.iter().cloned());
                args
            };

            if all_result_types.len() == 1 {
                // Single result: generate result function and define ensures_of
                let result_fun_name = boogie_behavioral_fun_result_name(self.env, &info.fun, false);
                emitln!(
                    self.writer,
                    "function {}({}): {};",
                    result_fun_name,
                    input_param_decls.join(", "),
                    all_result_types[0]
                );

                // Validity axiom
                let result_fun_app = format!("{}({})", result_fun_name, input_args_str);
                self.emit_result_validity_axiom(
                    &input_param_decls,
                    &result_fun_app,
                    all_result_type_refs[0],
                    &precond,
                );

                // Connecting axiom: result satisfies ensures_of
                let ensures_body = self.translate_fun_spec_conditions(
                    &fun_env,
                    &closure_spec,
                    BehaviorKind::EnsuresOf,
                    &info.fun.inst,
                    &inst_old,
                );
                // Define ensures_of as inline function using the translated body
                emitln!(
                    self.writer,
                    "function {{:inline}} {}({}): bool {{ {} }}",
                    ensures_fun_name,
                    full_param_decls.join(", "),
                    ensures_body
                );

                // Connecting axiom: result_fun satisfies ensures_of
                let ensures_of_with_result = format!(
                    "{}({}, {}({}))",
                    ensures_fun_name,
                    all_input_arg_names.join(", "),
                    result_fun_name,
                    input_args_str
                );
                emitln!(
                    self.writer,
                    "axiom (forall {} :: {{{}}} {});",
                    input_param_decls.join(", "),
                    result_fun_app,
                    ensures_of_with_result
                );
            } else if all_result_types.len() >= 2 {
                // Multiple results: generate tuple-returning result function
                let result_fun_name = boogie_behavioral_fun_result_name(self.env, &info.fun, true);

                let tuple_element_types = Self::deref_output_types(&all_result_type_refs);
                let tuple_type =
                    boogie_type(self.env, &Type::Tuple(tuple_element_types.clone()), false);
                emitln!(
                    self.writer,
                    "function {}({}): {};",
                    result_fun_name,
                    input_param_decls.join(", "),
                    tuple_type
                );

                // Validity axiom
                let result_fun_app = format!("{}({})", result_fun_name, input_args_str);
                self.emit_tuple_result_validity_axiom(
                    &input_param_decls,
                    &result_fun_app,
                    &tuple_element_types,
                    &precond,
                );

                // Define ensures_of with translated body
                let ensures_body = self.translate_fun_spec_conditions(
                    &fun_env,
                    &closure_spec,
                    BehaviorKind::EnsuresOf,
                    &info.fun.inst,
                    &inst_old,
                );
                emitln!(
                    self.writer,
                    "function {{:inline}} {}({}): bool {{ {} }}",
                    ensures_fun_name,
                    full_param_decls.join(", "),
                    ensures_body
                );

                // Connecting axiom
                let tuple_projections: Vec<String> = (0..all_result_types.len())
                    .map(|i| format!("_r->${}", i))
                    .collect();
                let ensures_of_with_result = format!(
                    "(var _r := {}({}); {}({}, {}))",
                    result_fun_name,
                    input_args_str,
                    ensures_fun_name,
                    all_input_arg_names.join(", "),
                    tuple_projections.join(", ")
                );
                emitln!(
                    self.writer,
                    "axiom (forall {} :: {{{}}} {});",
                    input_param_decls.join(", "),
                    result_fun_app,
                    ensures_of_with_result
                );
            } else {
                // No return values: still translate ensures body for state-change conditions
                let ensures_body = self.translate_fun_spec_conditions(
                    &fun_env,
                    &closure_spec,
                    BehaviorKind::EnsuresOf,
                    &info.fun.inst,
                    &inst_old,
                );
                emitln!(
                    self.writer,
                    "function {{:inline}} {}({}): bool {{ {} }}",
                    ensures_fun_name,
                    full_param_decls.join(", "),
                    ensures_body
                );
            }
        }
    }

    /// Translate a function's spec conditions of the given kind to a Boogie expression.
    /// Uses the SpecTranslator with old-aware memory context.
    fn translate_fun_spec_conditions(
        &self,
        fun_env: &FunctionEnv<'_>,
        closure_spec: &std::cell::Ref<'_, move_model::ast::Spec>,
        kind: BehaviorKind,
        type_inst: &[Type],
        old_memory: &BTreeSet<QualifiedInstId<StructId>>,
    ) -> String {
        let conditions: Vec<_> = closure_spec
            .conditions
            .iter()
            .filter(|c| match kind {
                BehaviorKind::RequiresOf => matches!(c.kind, ConditionKind::Requires),
                BehaviorKind::AbortsOf => matches!(c.kind, ConditionKind::AbortsIf),
                BehaviorKind::EnsuresOf => matches!(c.kind, ConditionKind::Ensures),
                BehaviorKind::ResultOf => false,
            })
            .collect();

        if conditions.is_empty() {
            return match kind {
                BehaviorKind::RequiresOf | BehaviorKind::EnsuresOf => "true".to_string(),
                BehaviorKind::AbortsOf => "false".to_string(),
                BehaviorKind::ResultOf => "true".to_string(),
            };
        }

        let num_params = fun_env.get_parameter_count();
        let num_results = fun_env.get_return_count();
        let mut_ref_result_positions = fun_env
            .get_parameter_types()
            .into_iter()
            .enumerate()
            .filter(|(_, ty)| ty.is_mutable_reference())
            .enumerate()
            .map(|(mut_ref_idx, (param_idx, _))| (param_idx, num_results + mut_ref_idx))
            .collect::<Vec<_>>();

        let is_two_state = !old_memory.is_empty() || fun_env.spec_uses_old();

        let translated: Vec<_> = conditions
            .iter()
            .map(|cond| {
                let exp = self.wrap_behavioral_condition_with_lets(
                    closure_spec,
                    &cond.kind,
                    is_two_state,
                    &cond.exp,
                );
                // Translate with old-aware memory context
                let temp_writer = CodeWriter::new(self.env.internal_loc());
                let mut temp_trans = SpecTranslator::new(&temp_writer, self.env, self.options);
                temp_trans.set_current_fun_qid(
                    fun_env.get_qualified_id().instantiate(type_inst.to_vec()),
                );
                // Set up old-aware context so memory references resolve to parameter names
                if !old_memory.is_empty() || fun_env.spec_uses_old() {
                    temp_trans.set_fun_old_memory(old_memory.clone());
                    temp_trans.set_fun_mut_params(
                        fun_env
                            .get_parameters()
                            .iter()
                            .filter(|p| p.1.is_mutable_reference())
                            .map(|p| p.0)
                            .collect(),
                    );
                }
                temp_trans.translate(&exp, type_inst);
                temp_trans.clear_current_fun_qid();
                let mut result = temp_writer.extract_result();
                if result.ends_with('\n') {
                    result.pop();
                }

                for (param_idx, result_idx) in mut_ref_result_positions.iter().rev() {
                    result = result.replace(
                        &format!("old($Dereference($t{}))", param_idx),
                        &format!("p{}", param_idx),
                    );
                    let current_value = match kind {
                        BehaviorKind::EnsuresOf => format!("r{}", result_idx),
                        BehaviorKind::RequiresOf | BehaviorKind::AbortsOf => {
                            format!("p{}", param_idx)
                        },
                        BehaviorKind::ResultOf => unreachable!(),
                    };
                    result =
                        result.replace(&format!("$Dereference($t{})", param_idx), &current_value);
                }

                // Substitute parameter references: $t{i} -> p{i}
                // Iterate in reverse to avoid prefix collisions ($t1 matching inside $t10)
                for i in (0..num_params).rev() {
                    result = result.replace(&format!("$t{}", i), &format!("p{}", i));
                }

                // Substitute result references: $ret{i} -> r{i}
                for i in (0..num_results).rev() {
                    result = result.replace(&format!("$ret{}", i), &format!("r{}", i));
                }

                result
            })
            .collect();

        let joined = match kind {
            BehaviorKind::RequiresOf | BehaviorKind::EnsuresOf => {
                if translated.len() == 1 {
                    format!("({})", translated[0])
                } else {
                    format!("({})", translated.join(" && "))
                }
            },
            BehaviorKind::AbortsOf => {
                if translated.len() == 1 {
                    format!("({})", translated[0])
                } else {
                    format!("({})", translated.join(" || "))
                }
            },
            BehaviorKind::ResultOf => return "true".to_string(),
        };

        // Collect intermediate labels from conditions that need existential wrapping.
        // Behavioral predicate Boogie functions have exactly two memory parameters:
        // `old_...` (pre-state) and `...` (post-state). Any explicitly labeled memory
        // reference is an intermediate state that must be existentially bound.
        // Entry/exit labels have already been normalized away by MemoryLabelInfo,
        // so all remaining labels are genuine intermediates.
        let temp_writer = CodeWriter::new(self.env.internal_loc());
        let temp_trans = SpecTranslator::new(&temp_writer, self.env, self.options);
        let mut all_labels = BTreeSet::new();
        for cond in &conditions {
            all_labels.extend(temp_trans.collect_intermediate_labels_from_exp(&cond.exp));
        }
        if all_labels.is_empty() {
            joined
        } else {
            // When the existential has intermediate labels, include the defining
            // fragments of conditions from OTHER kinds that define those labels.
            // For example, when generating $bp_aborts_of, Ensures conditions with
            // SpecRemove/SpecPublish/SpecUpdate define the abstract states. Only the
            // label-defining conjuncts are included (not full postcondition properties).
            // This uses the same analysis as emit_state_label_assumes in
            // spec_instrumentation.rs: ExpData::all_defined_labels() identifies
            // which conditions define labels, and we filter to defining conjuncts.
            let frame_translated: Vec<String> = closure_spec
                .conditions
                .iter()
                .filter(|c| {
                    // Skip conditions already in `joined` (same kind)
                    let dominated = match kind {
                        BehaviorKind::RequiresOf => matches!(c.kind, ConditionKind::Requires),
                        BehaviorKind::AbortsOf => matches!(c.kind, ConditionKind::AbortsIf),
                        BehaviorKind::EnsuresOf => matches!(c.kind, ConditionKind::Ensures),
                        BehaviorKind::ResultOf => false,
                    };
                    if dominated {
                        return false;
                    }
                    // Include only conditions that define at least one intermediate label
                    let defined = c.exp.as_ref().all_defined_labels();
                    defined
                        .iter()
                        .any(|l| all_labels.iter().any(|(al, _)| al == l))
                })
                .flat_map(|cond| {
                    // Extract only label-defining conjuncts (same logic as
                    // Instrumenter::defining_fragment in spec_instrumentation.rs)
                    use move_model::exp_simplifier::flatten_conjunction_owned;
                    let conjuncts = flatten_conjunction_owned(&cond.exp);
                    conjuncts
                        .into_iter()
                        .filter(|c| !c.as_ref().all_defined_labels().is_empty())
                        .filter_map(|fragment| {
                            let fragment = self.wrap_behavioral_condition_with_lets(
                                closure_spec,
                                &cond.kind,
                                is_two_state,
                                &fragment,
                            );
                            let fw = CodeWriter::new(self.env.internal_loc());
                            let mut ft = SpecTranslator::new(&fw, self.env, self.options);
                            ft.set_current_fun_qid(
                                fun_env.get_qualified_id().instantiate(type_inst.to_vec()),
                            );
                            if !old_memory.is_empty() || fun_env.spec_uses_old() {
                                ft.set_fun_old_memory(old_memory.clone());
                                ft.set_fun_mut_params(
                                    fun_env
                                        .get_parameters()
                                        .iter()
                                        .filter(|p| p.1.is_mutable_reference())
                                        .map(|p| p.0)
                                        .collect(),
                                );
                            }
                            ft.translate(&fragment, type_inst);
                            ft.clear_current_fun_qid();
                            let mut result = fw.extract_result();
                            if result.ends_with('\n') {
                                result.pop();
                            }
                            if kind != BehaviorKind::EnsuresOf {
                                // For aborts_of/requires_of: skip fragments that reference
                                // result variables or &mut post-state outputs, since these
                                // functions have no result/output params. Check BEFORE
                                // substitution so we detect $Dereference (post-state) vs
                                // old($Dereference) (pre-state, which is fine).
                                let has_ret = (0..num_results)
                                    .any(|i| result.contains(&format!("$ret{}", i)));
                                let has_mut_output =
                                    mut_ref_result_positions.iter().any(|(idx, _)| {
                                        let post = format!("$Dereference($t{})", idx);
                                        let pre = format!("old($Dereference($t{}))", idx);
                                        // Has post-state ref that isn't inside old()
                                        result.contains(&post)
                                            && result.replace(&pre, "").contains(&post)
                                    });
                                if has_ret || has_mut_output {
                                    return None;
                                }
                            }
                            for (param_idx, _) in mut_ref_result_positions.iter().rev() {
                                result = result.replace(
                                    &format!("old($Dereference($t{}))", param_idx),
                                    &format!("p{}", param_idx),
                                );
                                result = result.replace(
                                    &format!("$Dereference($t{})", param_idx),
                                    &format!("p{}", param_idx),
                                );
                            }
                            for i in (0..num_params).rev() {
                                result = result.replace(&format!("$t{}", i), &format!("p{}", i));
                            }
                            if kind == BehaviorKind::EnsuresOf {
                                for i in (0..num_results).rev() {
                                    result =
                                        result.replace(&format!("$ret{}", i), &format!("r{}", i));
                                }
                            }
                            Some(result)
                        })
                        .collect::<Vec<_>>()
                })
                .collect();

            // Collect labels from frame conditions too
            for cond in &closure_spec.conditions {
                let defined = cond.exp.as_ref().all_defined_labels();
                if defined
                    .iter()
                    .any(|l| all_labels.iter().any(|(al, _)| al == l))
                {
                    all_labels.extend(temp_trans.collect_intermediate_labels_from_exp(&cond.exp));
                }
            }

            let decls: Vec<String> = all_labels
                .iter()
                .map(|(label, mem)| {
                    let name = boogie_resource_memory_name(self.env, mem, &Some(*label));
                    let ty = boogie_struct_name(
                        &self.env.get_struct_qid(mem.to_qualified_id()),
                        &mem.inst,
                        false,
                    );
                    format!("{}: $Memory {}", name, ty)
                })
                .collect();
            if frame_translated.is_empty() {
                format!("(exists {} :: {})", decls.join(", "), joined)
            } else {
                let frame_conjoined = frame_translated.join(" && ");
                format!(
                    "(exists {} :: ({}) && {})",
                    decls.join(", "),
                    frame_conjoined,
                    joined
                )
            }
        }
    }

    /// Wraps a condition expression with `(var sym := binding; body)` for each
    /// `LetPre`/`LetPost` binding in the spec. For `LetPre` in two-state contexts,
    /// Wraps a condition expression with `(var sym := binding; body)` for each
    /// `LetPre`/`LetPost` binding in the spec. For `LetPre` in two-state contexts,
    /// the binding is wrapped in `old()` to ensure pre-state evaluation.
    fn wrap_behavioral_condition_with_lets(
        &self,
        closure_spec: &move_model::ast::Spec,
        source_kind: &ConditionKind,
        is_two_state: bool,
        exp: &Exp,
    ) -> Exp {
        let include_post_lets = matches!(source_kind, ConditionKind::Ensures);
        closure_spec
            .conditions
            .iter()
            .rev()
            .fold(exp.clone(), |scope, cond| match &cond.kind {
                ConditionKind::LetPre(sym, _) => {
                    // In two-state context, wrap binding in old() to ensure pre-state
                    // evaluation. In one-state context, there's only pre-state so no
                    // old() wrapping is needed.
                    let binding = if is_two_state {
                        let ty = self.env.get_node_type(cond.exp.node_id());
                        let loc = self.env.get_node_loc(cond.exp.node_id());
                        let node_id = self.env.new_node(loc, ty);
                        ExpData::Call(node_id, AstOperation::Old, vec![cond.exp.clone()]).into_exp()
                    } else {
                        cond.exp.clone()
                    };
                    ExpData::Block(
                        cond.exp.node_id(),
                        Pattern::Var(cond.exp.node_id(), *sym),
                        Some(binding),
                        scope,
                    )
                    .into_exp()
                },
                ConditionKind::LetPost(sym, _) if include_post_lets => ExpData::Block(
                    cond.exp.node_id(),
                    Pattern::Var(cond.exp.node_id(), *sym),
                    Some(cond.exp.clone()),
                    scope,
                )
                .into_exp(),
                _ => scope,
            })
    }

    /// Collect frame save resources and old memory resources across all closure and
    /// function parameter variants. Frame save resources are those that may need frame
    /// conditions (non-WritesAll access). Old memory resources are those needing pre-state
    /// save declarations.
    fn collect_frame_and_old_memory(
        closure_infos: &BTreeSet<ClosureInfo>,
        fun_param_infos: &BTreeSet<FunParamInfo>,
        env: &GlobalEnv,
    ) -> (
        BTreeSet<QualifiedInstId<StructId>>,
        BTreeSet<QualifiedInstId<StructId>>,
    ) {
        let mut frame_save_resources: BTreeSet<QualifiedInstId<StructId>> = BTreeSet::new();
        for info in closure_infos.iter() {
            let fun_env = env.get_function(info.fun.to_qualified_id());
            let frame_access = derive_closure_frame_access(&fun_env, &info.fun.inst);
            for (mem, access) in &frame_access {
                if !matches!(access, FrameAccessKind::WritesAll) {
                    frame_save_resources.insert(mem.clone());
                }
            }
        }
        for info in fun_param_infos.iter() {
            let (used_memory, _) = Self::get_param_memory(env, &info.fun, info.param_sym);
            let fun_env = env.get_function(info.fun.to_qualified_id());
            let has_access_of = fun_env
                .get_fun_param_access_of()
                .iter()
                .any(|a| a.fun_param == info.param_sym);
            if has_access_of {
                // With access_of, some resources may have Reads or WritesAt
                for mem in &used_memory {
                    frame_save_resources.insert(mem.clone());
                }
            }
            // Without access_of, all are WritesAll, no frame saves needed
        }
        // Collect all old_memory resources across variants for pre-state save declarations.
        let mut old_memory_resources: BTreeSet<QualifiedInstId<StructId>> = BTreeSet::new();
        for info in closure_infos.iter() {
            let fun_env = env.get_function(info.fun.to_qualified_id());
            for mem in fun_env.get_spec_old_memory() {
                old_memory_resources.insert(mem.clone().instantiate(&info.fun.inst));
            }
        }
        for info in fun_param_infos.iter() {
            let (_, old) = Self::get_param_memory(env, &info.fun, info.param_sym);
            old_memory_resources.extend(old);
        }
        (frame_save_resources, old_memory_resources)
    }

    /// Collect the union of used_memory and old_memory across all closure, function
    /// parameter, and struct field variants. Used to compute the memory parameter
    /// signature for evaluator functions and result_of axioms.
    fn collect_union_memory(
        env: &GlobalEnv,
        closure_infos: &BTreeSet<ClosureInfo>,
        fun_param_infos: &BTreeSet<FunParamInfo>,
        struct_field_infos: &BTreeSet<StructFieldInfo>,
    ) -> (
        BTreeSet<QualifiedInstId<StructId>>,
        BTreeSet<QualifiedInstId<StructId>>,
    ) {
        let mut union_used_memory = BTreeSet::new();
        let mut union_old_memory = BTreeSet::new();
        for info in closure_infos {
            let fun_env = env.get_function(info.fun.to_qualified_id());
            for mem in fun_env.get_spec_used_memory() {
                union_used_memory.insert(mem.clone().instantiate(&info.fun.inst));
            }
            for mem in fun_env.get_spec_old_memory() {
                union_old_memory.insert(mem.clone().instantiate(&info.fun.inst));
            }
        }
        for info in fun_param_infos {
            let (used, old) = Self::get_param_memory(env, &info.fun, info.param_sym);
            union_used_memory.extend(used);
            union_old_memory.extend(old);
        }
        for info in struct_field_infos {
            let struct_env = env.get_struct_qid(info.struct_id.to_qualified_id());
            for access in struct_env.get_field_access_of() {
                if access.fun_param == info.field_sym {
                    if access.frame_spec.modifies_all || access.frame_spec.reads_all {
                        // Wildcard: include all translated memory types
                        let mono_info = mono_analysis::get_info(env);
                        for qid in mono_info.all_memory_qids(env) {
                            union_used_memory.insert(qid.clone());
                            if access.frame_spec.modifies_all {
                                union_old_memory.insert(qid);
                            }
                        }
                    } else {
                        union_used_memory.extend(access.used_memory.iter().cloned());
                        union_old_memory.extend(access.old_memory.iter().cloned());
                    }
                }
            }
        }
        (union_used_memory, union_old_memory)
    }

    /// Build instantiated memory args for a specific function's spec memory.
    /// Instantiates the function's used and old memory with the given type instantiation,
    /// then returns the Boogie argument strings for passing to spec functions.
    fn build_instantiated_memory_args(
        env: &GlobalEnv,
        fun_env: &FunctionEnv,
        inst: &[Type],
    ) -> Vec<String> {
        let used = fun_env.get_spec_used_memory();
        let old = fun_env.get_spec_old_memory();
        let inst_used: BTreeSet<_> = used.iter().map(|m| m.clone().instantiate(inst)).collect();
        let inst_old: BTreeSet<_> = old.iter().map(|m| m.clone().instantiate(inst)).collect();
        let (_, args) = Self::build_memory_params(env, &inst_used, &inst_old);
        args
    }

    /// Reads pre-computed `(used_memory, old_memory)` from `access_of` for a function parameter.
    /// Memory is computed during the spec_rewriter pass and stored on `FunParamAccessOf`.
    fn get_param_memory(
        env: &GlobalEnv,
        fun_qid: &QualifiedInstId<FunId>,
        param_sym: Symbol,
    ) -> (
        BTreeSet<QualifiedInstId<StructId>>,
        BTreeSet<QualifiedInstId<StructId>>,
    ) {
        let fun_env = env.get_function(fun_qid.to_qualified_id());
        for access in fun_env.get_fun_param_access_of() {
            if access.fun_param == param_sym {
                // Instantiate with the function's type parameters
                let used = access
                    .used_memory
                    .iter()
                    .map(|m| m.clone().instantiate(&fun_qid.inst))
                    .collect();
                let old = access
                    .old_memory
                    .iter()
                    .map(|m| m.clone().instantiate(&fun_qid.inst))
                    .collect();
                return (used, old);
            }
        }
        (BTreeSet::new(), BTreeSet::new())
    }

    /// Builds memory parameter declarations for Boogie functions following the dual-state pattern.
    /// Returns (param_decls, arg_exprs) where:
    /// - param_decls: vector of "name: type" strings for function declaration
    /// - arg_exprs: vector of argument name strings for calling the function
    fn build_memory_params(
        env: &GlobalEnv,
        used_memory: &BTreeSet<QualifiedInstId<StructId>>,
        old_memory: &BTreeSet<QualifiedInstId<StructId>>,
    ) -> (Vec<String>, Vec<String>) {
        let uses_old = !old_memory.is_empty();
        let mut decls = vec![];
        let mut args = vec![];
        for memory in used_memory {
            let struct_env = &env.get_struct_qid(memory.to_qualified_id());
            let mem_type = format!(
                "$Memory {}",
                boogie_struct_name(struct_env, &memory.inst, false)
            );
            let name = boogie_resource_memory_name(env, memory, &None);
            if uses_old && old_memory.contains(memory) {
                decls.push(format!("old_{}: {}", name, mem_type));
                decls.push(format!("{}: {}", name, mem_type));
                args.push(format!("old_{}", name));
                args.push(name);
            } else {
                decls.push(format!("{}: {}", name, mem_type));
                args.push(name);
            }
        }
        (decls, args)
    }

    /// Build memory argument strings for a struct field variant in the evaluator.
    /// Maps the field's declared memory to the evaluator's parameter names.
    fn build_evaluator_memory_args_for_access(
        env: &GlobalEnv,
        decl: &FunParamAccessOf,
        union_used: &BTreeSet<QualifiedInstId<StructId>>,
        union_old: &BTreeSet<QualifiedInstId<StructId>>,
    ) -> Vec<String> {
        let uses_old = !union_old.is_empty();
        let mut args = vec![];
        if decl.frame_spec.modifies_all || decl.frame_spec.reads_all {
            // Wildcard: pass all union memory args
            for mem in union_used {
                let name = boogie_resource_memory_name(env, mem, &None);
                if uses_old && (decl.frame_spec.modifies_all) && union_old.contains(mem) {
                    args.push(format!("old_{}", name));
                }
                args.push(name);
            }
        } else {
            for mem in &decl.used_memory {
                let name = boogie_resource_memory_name(env, mem, &None);
                if uses_old && decl.old_memory.contains(mem) && union_old.contains(mem) {
                    args.push(format!("old_{}", name));
                }
                if union_used.contains(mem) {
                    args.push(name);
                }
            }
        }
        args
    }

    /// Behavioral predicates reason over plain values, even when the underlying
    /// function result is a mutable reference or a `&mut` parameter post-state.
    fn behavioral_output_types(params: &[Type], results: &[Type]) -> Vec<Type> {
        let mut outputs = results
            .iter()
            .map(|ty| {
                if ty.is_mutable_reference() {
                    ty.skip_reference().clone()
                } else {
                    ty.clone()
                }
            })
            .collect::<Vec<_>>();
        outputs.extend(
            params
                .iter()
                .filter(|ty| ty.is_mutable_reference())
                .map(|ty| ty.skip_reference().clone()),
        );
        outputs
    }

    fn behavioral_output_type_refs<'a>(params: &'a [Type], results: &'a [Type]) -> Vec<&'a Type> {
        let mut outputs = results.iter().collect::<Vec<_>>();
        outputs.extend(params.iter().filter(|ty| ty.is_mutable_reference()));
        outputs
    }

    fn behavioral_output_args(params: &[Type], results: &[Type]) -> Vec<String> {
        (0..(results.len() + params.iter().filter(|ty| ty.is_mutable_reference()).count()))
            .map(|i| format!("r{}", i))
            .collect()
    }

    /// Compute Boogie validity precondition string for a list of parameter types.
    /// `param_prefix` is the name prefix used in Boogie (e.g. "p" for "p0, p1, ..." or
    /// "arg" for "arg0, arg1, ...").
    fn validity_precondition(env: &GlobalEnv, param_tys: &[Type], param_prefix: &str) -> String {
        let conds: Vec<String> = param_tys
            .iter()
            .enumerate()
            .map(|(i, ty)| {
                boogie_well_formed_expr(
                    env,
                    &format!("{}{}", param_prefix, i),
                    ty.skip_reference(),
                    false,
                )
            })
            .collect();
        if conds.is_empty() {
            "true".to_string()
        } else {
            conds.join(" && ")
        }
    }

    /// Dereference mutable reference types for behavioral output (used in result tuple types).
    fn deref_output_types(type_refs: &[&Type]) -> Vec<Type> {
        type_refs
            .iter()
            .map(|ty| {
                if ty.is_mutable_reference() {
                    ty.skip_reference().clone()
                } else {
                    (*ty).clone()
                }
            })
            .collect()
    }

    /// Emit a single-result validity axiom:
    /// `axiom (forall decls :: {trigger} precond ==> validity);`
    fn emit_result_validity_axiom(
        &self,
        input_param_decls: &[String],
        result_fun_app: &str,
        result_ty: &Type,
        precond: &str,
    ) {
        let result_ty_for_validity = if result_ty.is_mutable_reference() {
            result_ty.skip_reference()
        } else {
            result_ty
        };
        let result_validity =
            boogie_well_formed_expr(self.env, result_fun_app, result_ty_for_validity, false);
        emitln!(
            self.writer,
            "axiom (forall {} :: {{{}}} {} ==> {});",
            input_param_decls.join(", "),
            result_fun_app,
            precond,
            result_validity
        );
    }

    /// Emit a tuple-result validity axiom:
    /// `axiom (forall decls :: {trigger} precond ==> (var r := trigger; valid(r->$0) && ...));`
    fn emit_tuple_result_validity_axiom(
        &self,
        input_param_decls: &[String],
        result_fun_app: &str,
        tuple_element_types: &[Type],
        precond: &str,
    ) {
        let result_validities: Vec<String> = tuple_element_types
            .iter()
            .enumerate()
            .map(|(i, ty)| boogie_well_formed_expr(self.env, &format!("r->${}", i), ty, false))
            .collect();
        let result_validity = format!(
            "(var r := {}; {})",
            result_fun_app,
            result_validities.join(" && ")
        );
        emitln!(
            self.writer,
            "axiom (forall {} :: {{{}}} {} ==> {});",
            input_param_decls.join(", "),
            result_fun_app,
            precond,
            result_validity
        );
    }

    /// Generate uninterpreted spec functions for behavioral predicates on function-typed parameters.
    /// These functions are called by the behavioral evaluators and connected to closures via assumptions.
    ///
    /// For `ensures_of`, we make it functional by defining it in terms of uninterpreted result functions:
    /// - `ensures_of_result{i}(args...)` returns the i-th result value for given inputs
    /// - `ensures_of(args..., results...)` is defined as `ensures_of_result0(args) == result0 && ...`
    ///
    /// This ensures that `ensures_of(x, y)` is functional: for a given `x`, there's exactly one `y`.
    fn generate_behavioral_spec_funs_for_params(
        &self,
        fun_type: &Type,
        fun_param_infos: &BTreeSet<FunParamInfo>,
    ) {
        let Type::Fun(params, results, _) = fun_type else {
            return;
        };
        let params = params.clone().flatten();
        let results = results.clone().flatten();

        for info in fun_param_infos {
            // Derive memory parameters from access_of declarations
            let (used_memory, old_memory) =
                Self::get_param_memory(self.env, &info.fun, info.param_sym);
            let (mem_param_decls, mem_args) =
                Self::build_memory_params(self.env, &used_memory, &old_memory);

            // Build input parameters (used by all behavioral predicates)
            // Memory params come first, then data params
            let mut input_param_decls = mem_param_decls.clone();
            let mut input_args: Vec<String> = mem_args;
            for (i, ty) in params.iter().enumerate() {
                input_param_decls.push(format!(
                    "arg{}: {}",
                    i,
                    boogie_type(self.env, ty.skip_reference(), false)
                ));
                input_args.push(format!("arg{}", i));
            }

            // For requires_of and aborts_of: generate uninterpreted predicates
            for kind in [BehaviorKind::RequiresOf, BehaviorKind::AbortsOf] {
                let fun_name =
                    boogie_behavioral_spec_fun_name(self.env, &info.fun, info.param_sym, kind, &[]);
                emitln!(
                    self.writer,
                    "function {}({}): bool;",
                    fun_name,
                    input_param_decls.join(", ")
                );
            }

            // For ensures_of: generate result functions and define ensures_of in terms of them.
            let all_result_type_refs = Self::behavioral_output_type_refs(&params, &results);
            let all_result_types: Vec<String> = Self::behavioral_output_types(&params, &results)
                .into_iter()
                .map(|ty| boogie_type(self.env, &ty, false))
                .collect();

            // Generate uninterpreted result functions and validity axioms.
            // For single result: generates `ensures_of_result0(args) : result_type`
            // For multiple results: generates `ensures_of_results(args) : $Tuple{n} ...`
            // This is needed to establish witnesses for choice expressions like `choose y where ensures_of<f>(x, y)`.
            let input_args_str = input_args.join(", ");

            let precond = Self::validity_precondition(self.env, &params, "arg");

            // Get the ensures_of function name (used in both branches)
            let ensures_fun_name = boogie_behavioral_spec_fun_name(
                self.env,
                &info.fun,
                info.param_sym,
                BehaviorKind::EnsuresOf,
                &[],
            );

            // Build full parameter list including results
            let mut full_param_decls = input_param_decls.clone();
            for (i, result_type) in all_result_types.iter().enumerate() {
                full_param_decls.push(format!("result{}: {}", i, result_type));
            }

            if all_result_types.len() == 1 {
                // Single result: generate simple function ensures_of_result(args) : result_type
                // all_result_types already has dereferenced types.
                let result_fun_name = boogie_behavioral_result_fun_name(
                    self.env,
                    &info.fun,
                    info.param_sym,
                    &[],
                    false,
                );
                emitln!(
                    self.writer,
                    "function {}({}): {};",
                    result_fun_name,
                    input_param_decls.join(", "),
                    all_result_types[0]
                );

                // Validity axiom
                let result_fun_app = format!("{}({})", result_fun_name, input_args_str);
                self.emit_result_validity_axiom(
                    &input_param_decls,
                    &result_fun_app,
                    all_result_type_refs[0],
                    &precond,
                );

                // Define ensures_of as simple equality
                let body = format!("{}({}) == result0", result_fun_name, input_args_str);
                emitln!(
                    self.writer,
                    "function {{:inline}} {}({}): bool {{ {} }}",
                    ensures_fun_name,
                    full_param_decls.join(", "),
                    body
                );
            } else if all_result_types.len() >= 2 {
                // Multiple results: generate tuple-returning function ensures_of_results(args) : $Tuple{n} ...
                let result_fun_name = boogie_behavioral_result_fun_name(
                    self.env,
                    &info.fun,
                    info.param_sym,
                    &[],
                    true,
                );

                let tuple_element_types = Self::deref_output_types(&all_result_type_refs);
                let tuple_type =
                    boogie_type(self.env, &Type::Tuple(tuple_element_types.clone()), false);
                emitln!(
                    self.writer,
                    "function {}({}): {};",
                    result_fun_name,
                    input_param_decls.join(", "),
                    tuple_type
                );

                // Validity axiom
                let result_fun_app = format!("{}({})", result_fun_name, input_args_str);
                self.emit_tuple_result_validity_axiom(
                    &input_param_decls,
                    &result_fun_app,
                    &tuple_element_types,
                    &precond,
                );

                // Define ensures_of by unpacking tuple:
                // (var r := result_fun(args); r->$0 == result0 && r->$1 == result1 && ...)
                let equality_checks: Vec<String> = (0..all_result_types.len())
                    .map(|i| format!("r->${} == result{}", i, i))
                    .collect();
                let body = format!(
                    "(var r := {}({}); {})",
                    result_fun_name,
                    input_args_str,
                    equality_checks.join(" && ")
                );
                emitln!(
                    self.writer,
                    "function {{:inline}} {}({}): bool {{ {} }}",
                    ensures_fun_name,
                    full_param_decls.join(", "),
                    body
                );
            } else {
                // No return values: generate the real ensures body for state-change conditions.
                // This is the parameter variant evaluator — it dispatches to the concrete
                // closure's ensures body which now correctly handles void functions.
                emitln!(
                    self.writer,
                    "function {{:inline}} {}({}): bool {{ true }}",
                    ensures_fun_name,
                    full_param_decls.join(", ")
                );
            }
        }
    }

    /// Generate uninterpreted behavioral predicate spec functions for struct field variants.
    /// These are parameterized by an instance id `n: int` (first parameter) that distinguishes
    /// different struct instances. The pattern mirrors `generate_behavioral_spec_funs_for_params`.
    fn generate_behavioral_spec_funs_for_struct_fields(
        &self,
        fun_type: &Type,
        struct_field_infos: &BTreeSet<StructFieldInfo>,
    ) {
        let Type::Fun(params, results, _) = fun_type else {
            return;
        };
        let params = params.clone().flatten();
        let results = results.clone().flatten();

        for info in struct_field_infos {
            // Look up access declarations for this field's memory requirements
            let struct_env = self.env.get_struct_qid(info.struct_id.to_qualified_id());
            let field_access = struct_env.get_field_access_of();
            let access_decl = field_access.iter().find(|a| a.fun_param == info.field_sym);
            let has_memory = access_decl.is_some();

            // Build memory parameter declarations if the field has access declarations
            let mut mem_param_decls = vec![];
            let mut mem_args = vec![];
            if let Some(decl) = access_decl {
                if decl.frame_spec.modifies_all || decl.frame_spec.reads_all {
                    // Wildcard: include all translated memory types
                    let mono_info = mono_analysis::get_info(self.env);
                    for qid in mono_info.all_memory_qids(self.env) {
                        let mem_name = boogie_resource_memory_name(self.env, &qid, &None);
                        let se = self.env.get_struct_qid(qid.to_qualified_id());
                        let mem_type =
                            format!("$Memory {}", boogie_struct_name(&se, &qid.inst, false));
                        if decl.frame_spec.modifies_all {
                            let old_name = format!("old_{}", mem_name);
                            mem_param_decls.push(format!("{}: {}", old_name, mem_type));
                            mem_args.push(old_name);
                        }
                        mem_param_decls.push(format!("{}: {}", mem_name, mem_type));
                        mem_args.push(mem_name);
                    }
                } else {
                    // Specific access declarations
                    for mem in &decl.used_memory {
                        let mem_name = boogie_resource_memory_name(self.env, mem, &None);
                        let struct_env = self.env.get_struct_qid(mem.to_qualified_id());
                        let mem_type = format!(
                            "$Memory {}",
                            boogie_struct_name(&struct_env, &mem.inst, false)
                        );
                        if decl.old_memory.contains(mem) {
                            let old_name = format!("old_{}", mem_name);
                            mem_param_decls.push(format!("{}: {}", old_name, mem_type));
                            mem_args.push(old_name);
                        }
                        mem_param_decls.push(format!("{}: {}", mem_name, mem_type));
                        mem_args.push(mem_name);
                    }
                }
            }

            // Build input parameters: memory..., n (instance id), then data args.
            let mut input_param_decls = mem_param_decls.clone();
            input_param_decls.push("n: int".to_string());
            let mut input_args: Vec<String> = mem_args.clone();
            input_args.push("n".to_string());
            for (i, ty) in params.iter().enumerate() {
                input_param_decls.push(format!(
                    "arg{}: {}",
                    i,
                    boogie_type(self.env, ty.skip_reference(), false)
                ));
                input_args.push(format!("arg{}", i));
            }
            let _ = has_memory;

            // For requires_of and aborts_of: generate uninterpreted predicates
            for kind in [BehaviorKind::RequiresOf, BehaviorKind::AbortsOf] {
                let fun_name = boogie_struct_field_spec_fun_name(
                    self.env,
                    &info.struct_id,
                    info.field_sym,
                    kind,
                    &[],
                );
                emitln!(
                    self.writer,
                    "function {}({}): bool;",
                    fun_name,
                    input_param_decls.join(", ")
                );
            }

            // For ensures_of: generate result functions and define ensures_of in terms of them.
            let all_result_type_refs = Self::behavioral_output_type_refs(&params, &results);
            let all_result_types: Vec<String> = Self::behavioral_output_types(&params, &results)
                .into_iter()
                .map(|ty| boogie_type(self.env, &ty, false))
                .collect();

            let input_args_str = input_args.join(", ");
            let precond = Self::validity_precondition(self.env, &params, "arg");

            let ensures_fun_name = boogie_struct_field_spec_fun_name(
                self.env,
                &info.struct_id,
                info.field_sym,
                BehaviorKind::EnsuresOf,
                &[],
            );

            let mut full_param_decls = input_param_decls.clone();
            for (i, result_type) in all_result_types.iter().enumerate() {
                full_param_decls.push(format!("result{}: {}", i, result_type));
            }

            if all_result_types.len() == 1 {
                let result_fun_name = boogie_struct_field_result_fun_name(
                    self.env,
                    &info.struct_id,
                    info.field_sym,
                    &[],
                    false,
                );
                emitln!(
                    self.writer,
                    "function {}({}): {};",
                    result_fun_name,
                    input_param_decls.join(", "),
                    all_result_types[0]
                );
                let result_fun_app = format!("{}({})", result_fun_name, input_args_str);
                self.emit_result_validity_axiom(
                    &input_param_decls,
                    &result_fun_app,
                    all_result_type_refs[0],
                    &precond,
                );
                let body = format!("{}({}) == result0", result_fun_name, input_args_str);
                emitln!(
                    self.writer,
                    "function {{:inline}} {}({}): bool {{ {} }}",
                    ensures_fun_name,
                    full_param_decls.join(", "),
                    body
                );
            } else if all_result_types.len() >= 2 {
                let result_fun_name = boogie_struct_field_result_fun_name(
                    self.env,
                    &info.struct_id,
                    info.field_sym,
                    &[],
                    true,
                );
                let tuple_element_types = Self::deref_output_types(&all_result_type_refs);
                let tuple_type =
                    boogie_type(self.env, &Type::Tuple(tuple_element_types.clone()), false);
                emitln!(
                    self.writer,
                    "function {}({}): {};",
                    result_fun_name,
                    input_param_decls.join(", "),
                    tuple_type
                );
                let result_fun_app = format!("{}({})", result_fun_name, input_args_str);
                self.emit_tuple_result_validity_axiom(
                    &input_param_decls,
                    &result_fun_app,
                    &tuple_element_types,
                    &precond,
                );
                let equality_checks: Vec<String> = (0..all_result_types.len())
                    .map(|i| format!("r->${} == result{}", i, i))
                    .collect();
                let body = format!(
                    "(var r := {}({}); {})",
                    result_fun_name,
                    input_args_str,
                    equality_checks.join(" && ")
                );
                emitln!(
                    self.writer,
                    "function {{:inline}} {}({}): bool {{ {} }}",
                    ensures_fun_name,
                    full_param_decls.join(", "),
                    body
                );
            } else {
                emitln!(
                    self.writer,
                    "function {{:inline}} {}({}): bool {{ true }}",
                    ensures_fun_name,
                    full_param_decls.join(", ")
                );
            }
        }
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
        let field_suffix = boogie_type_suffix(self.parent.env, &self.inst(&f.get_type()), bv_flag);
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
            boogie_well_formed_expr(self.parent.env, &sel, ty, bv_flag)
        )
    }

    /// Return identity constraint for a function-valued field `f` if it has a
    /// `StructFieldInfo` entry, e.g. `s->$0_Continuation is $struct_field'State$0'`.
    /// Returns `None` if the field isn't a storable function type with a StructFieldInfo.
    pub fn boogie_field_identity_constraint(
        &self,
        field: &FieldEnv<'_>,
        sep: &str,
    ) -> Option<String> {
        let env = self.parent.env;
        let field_ty = field.get_type().instantiate(self.type_inst);
        if let Type::Fun(params, results, abilities) = &field_ty {
            if abilities.has_store() {
                let mono_info = mono_analysis::get_info(env);
                let normalized = Type::Fun(
                    Box::new(Type::tuple(params.clone().flatten())),
                    Box::new(Type::tuple(results.clone().flatten())),
                    AbilitySet::EMPTY,
                );
                let struct_qid = self
                    .struct_env
                    .get_qualified_id()
                    .instantiate(self.type_inst.to_vec());
                if let Some(field_infos) = mono_info.fun_struct_field_infos.get(&normalized) {
                    let has_entry = field_infos.iter().any(|info| {
                        info.struct_id == struct_qid && info.field_sym == field.get_name()
                    });
                    if has_entry {
                        let sel = format!("s->{}", boogie_field_sel(field));
                        let ctor_name =
                            boogie_struct_field_name(env, &struct_qid, field.get_name());
                        return Some(format!("{}{} is {}", sep, sel, ctor_name));
                    }
                }
            }
        }
        None
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
        let struct_name = boogie_struct_name(struct_env, self.type_inst, false);
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
                    // Add identity constraints for function-valued fields
                    for field in &fields {
                        if let Some(constraint) = self.boogie_field_identity_constraint(field, sep)
                        {
                            emitln!(writer, "{}", constraint);
                            sep = "  && ";
                        }
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

        // Emit uninterpreted function for arbitrary ordering fallback
        emitln!(
            writer,
            "function $Arbitrary_cmp_ordering'{}'(v1: {}, v2: {}): $1_cmp_Ordering;",
            suffix,
            struct_name,
            struct_name
        );

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
                emitln!(writer, "else $Arbitrary_cmp_ordering'{}'(v1, v2)", suffix);
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
                        let field_type_name = boogie_type_suffix(
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
        boogie_type(env, ty, bv_flag)
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
        let struct_name = boogie_struct_name(struct_env, self.type_inst, false);
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
                // Add identity constraints for function-valued fields
                for field in struct_env.get_fields() {
                    if let Some(constraint) = self.boogie_field_identity_constraint(&field, sep) {
                        emitln!(writer, "{}", constraint);
                        sep = "  && ";
                    }
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
                                    let suffix_ty = boogie_type_suffix(
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
        boogie_type(env, ty, bv_flag)
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
        // Set current function on spec_translator so behavioral predicates can
        // resolve parameter access specifiers for memory args.
        self.parent.spec_translator.set_current_fun_qid(qid.clone());
        emitln!(
            writer,
            "// fun {} [{}] {}",
            env.display(&qid),
            fun_target.data.variant,
            fun_target.get_loc().display(env)
        );

        self.generate_function_sig();
        self.generate_function_body();
        self.parent.spec_translator.clear_current_fun_qid();
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
                    VerificationFlavor::Instantiated(_) | VerificationFlavor::Split(..) => {
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
            boogie_function_name(fun_target.func_env, self.type_inst, &[]),
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
                let temp_name =
                    boogie_temp_from_suffix(env, &boogie_type_suffix(env, &ty, *bv_flag), i);
                if !dup.contains(&temp_name) {
                    emitln!(
                        writer,
                        "var {}: {};",
                        temp_name.clone(),
                        boogie_type(env, &ty, *bv_flag)
                    );
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
        let mut declared_mem_names: BTreeSet<String> = BTreeSet::new();
        for (lab, mem) in &labels {
            let mem = (*mem).clone().instantiate(self.type_inst);
            let name = boogie_resource_memory_name(env, &mem, &Some(**lab));
            if declared_mem_names.insert(name.clone()) {
                emitln!(
                    writer,
                    "var {}: $Memory {};",
                    name,
                    boogie_struct_name(
                        &env.get_struct_qid(mem.to_qualified_id()),
                        &mem.inst,
                        false
                    )
                );
            }
        }

        // Generate var declarations for behavior state labels and collect frame info.
        // These are logical variables used in behavioral predicates (ensures_of, etc.)
        // with state labels connecting sequential calls.
        let label_info = self.declare_behavior_state_labels(
            fun_target,
            code,
            env,
            writer,
            &mut declared_mem_names,
        );

        // Pass label info to the spec translator so it can resolve memory references
        // at labeled states back through the pre-state chain for non-modified types.
        self.parent.spec_translator.set_label_info(label_info);

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

    /// Collect (label, memory_type) pairs from expressions that reference state labels.
    /// Handles all operation types: Behavior (with closure resolution), Global(Some),
    /// Exists(Some), and SpecFunction with non-default ranges.
    fn collect_label_memory_pairs(
        &self,
        exps: &[Exp],
        code: &[Bytecode],
        env: &GlobalEnv,
    ) -> BTreeSet<(MemoryLabel, QualifiedInstId<StructId>)> {
        let mut result: BTreeSet<(MemoryLabel, QualifiedInstId<StructId>)> = BTreeSet::new();
        for exp in exps {
            exp.visit_pre_order(&mut |e| {
                if let ExpData::Call(_, AstOperation::Behavior(_, range), args) = e {
                    let closure_info: Option<(Vec<Type>, move_model::model::ModuleId, FunId)> =
                        match args.first().map(|a| a.as_ref()) {
                            Some(ExpData::Call(
                                closure_id,
                                AstOperation::Closure(mid, fid, _),
                                _,
                            )) => {
                                let inst = env.get_node_instantiation(*closure_id);
                                let inst = Type::instantiate_slice(&inst, self.type_inst);
                                Some((inst, *mid, *fid))
                            },
                            Some(ExpData::Temporary(_, idx)) => find_closure_for_temp(code, *idx)
                                .map(|(mid, fid, types, _)| {
                                    let inst = Type::instantiate_slice(&types, self.type_inst);
                                    (inst, mid, fid)
                                }),
                            _ => None,
                        };
                    if let Some((inst, mid, fid)) = closure_info {
                        let closure_fun_env = env.get_function(mid.qualified(fid));
                        for memory in closure_fun_env.get_spec_used_memory() {
                            let memory = memory.clone().instantiate(&inst);
                            for label in range.labels() {
                                result.insert((label, memory.clone()));
                            }
                        }
                        for memory in closure_fun_env.get_spec_old_memory() {
                            let memory = memory.clone().instantiate(&inst);
                            for label in range.labels() {
                                result.insert((label, memory.clone()));
                            }
                        }
                    } else if let Some(fun_arg) = args.first() {
                        // Runtime function value (LocalVar, field access, etc.):
                        // use the evaluator memory union from MonoInfo.
                        let fun_type = env.get_node_type(fun_arg.node_id());
                        let fun_type = fun_type.instantiate(self.type_inst);
                        let (used, old) = compute_evaluator_memory_union(env, &fun_type);
                        for memory in used.iter().chain(old.iter()) {
                            for label in range.labels() {
                                result.insert((label, memory.clone()));
                            }
                        }
                    }
                }
                true
            });
        }
        for exp in exps {
            exp.visit_pre_order(&mut |e| {
                match e {
                    ExpData::Call(node_id, AstOperation::Global(Some(label)), _)
                    | ExpData::Call(node_id, AstOperation::Exists(Some(label)), _) => {
                        let node_inst = env.get_node_instantiation(*node_id);
                        let node_inst = Type::instantiate_slice(&node_inst, self.type_inst);
                        let mem_ty = &node_inst[0];
                        let (mid, sid, inst) = mem_ty.require_struct();
                        let mem = mid.qualified_inst(sid, inst.to_owned());
                        result.insert((*label, mem));
                    },
                    _ => {},
                }
                true
            });
        }
        for exp in exps {
            exp.visit_pre_order(&mut |e| {
                if let ExpData::Call(node_id, AstOperation::SpecFunction(mid, fid, range), _) = e {
                    if !range.is_default() {
                        let inst = env.get_node_instantiation(*node_id);
                        let inst = Type::instantiate_slice(&inst, self.type_inst);
                        let module = env.get_module(*mid);
                        let fun = module.get_spec_fun(*fid);
                        for mem in fun.used_memory.iter() {
                            let mem = mem.clone().instantiate(&inst);
                            for label in range.labels() {
                                result.insert((label, mem.clone()));
                            }
                        }
                    }
                }
                true
            });
        }
        for exp in exps {
            exp.visit_pre_order(&mut |e| {
                if let ExpData::Call(
                    node_id,
                    AstOperation::SpecPublish(range)
                    | AstOperation::SpecRemove(range)
                    | AstOperation::SpecUpdate(range),
                    _,
                ) = e
                {
                    if !range.is_default() {
                        let node_inst = env.get_node_instantiation(*node_id);
                        let node_inst = Type::instantiate_slice(&node_inst, self.type_inst);
                        let mem_ty = &node_inst[0];
                        let (mid, sid, inst) = mem_ty.require_struct();
                        let mem = mid.qualified_inst(sid, inst.to_owned());
                        for label in range.labels() {
                            result.insert((label, mem.clone()));
                        }
                    }
                }
                true
            });
        }
        result
    }

    /// Compute label info (pre-state chain and modified types) for each state label
    /// found in spec expressions. This is used to resolve which resource types are
    /// actually modified at each label so that non-modified types can be resolved
    /// back through the chain to the correct memory state.
    fn compute_label_info(
        &self,
        all_exps: &[Exp],
        code: &[Bytecode],
        env: &GlobalEnv,
    ) -> BTreeMap<MemoryLabel, LabelInfo> {
        use move_model::ast::BehaviorKind;
        let mut label_info: BTreeMap<MemoryLabel, LabelInfo> = BTreeMap::new();

        for exp in all_exps {
            exp.visit_pre_order(&mut |e| {
                if let ExpData::Call(node_id, op, args) = e {
                    match op {
                        AstOperation::SpecPublish(range)
                        | AstOperation::SpecRemove(range)
                        | AstOperation::SpecUpdate(range) => {
                            if let Some(post_label) = range.post {
                                let node_inst = env.get_node_instantiation(*node_id);
                                let node_inst = Type::instantiate_slice(&node_inst, self.type_inst);
                                let (mid, sid, inst) = node_inst[0].require_struct();
                                let mem = mid.qualified_inst(sid, inst.to_owned());
                                let info = label_info.entry(post_label).or_default();
                                info.modified_types.insert(mem);
                                info.pre = info.pre.or(range.pre);
                            }
                        },
                        AstOperation::Behavior(
                            BehaviorKind::EnsuresOf | BehaviorKind::ResultOf,
                            range,
                        ) => {
                            if let Some(post_label) = range.post {
                                // Resolve closure to get callee's modifies
                                let closure_info: Option<(
                                    Vec<Type>,
                                    move_model::model::ModuleId,
                                    FunId,
                                )> = match args.first().map(|a| a.as_ref()) {
                                    Some(ExpData::Call(
                                        closure_id,
                                        AstOperation::Closure(mid, fid, _),
                                        _,
                                    )) => {
                                        let inst = env.get_node_instantiation(*closure_id);
                                        let inst = Type::instantiate_slice(&inst, self.type_inst);
                                        Some((inst, *mid, *fid))
                                    },
                                    Some(ExpData::Temporary(_, idx)) => find_closure_for_temp(
                                        code, *idx,
                                    )
                                    .map(|(mid, fid, types, _)| {
                                        let inst = Type::instantiate_slice(&types, self.type_inst);
                                        (inst, mid, fid)
                                    }),
                                    _ => None,
                                };
                                if let Some((inst, mid, fid)) = closure_info {
                                    let callee = env.get_function(mid.qualified(fid));
                                    let info = label_info.entry(post_label).or_default();
                                    // Add callee's modifies targets as modified types
                                    for mem in callee.get_modify_targets().keys() {
                                        let mem_inst =
                                            mem.module_id.qualified_inst(mem.id, inst.clone());
                                        info.modified_types.insert(mem_inst);
                                    }
                                    info.pre = info.pre.or(range.pre);
                                } else if let Some(fun_arg) = args.first() {
                                    // Runtime function value: use the evaluator
                                    // memory union from MonoInfo to determine
                                    // which memories could be modified.
                                    let fun_type = env.get_node_type(fun_arg.node_id());
                                    let fun_type = fun_type.instantiate(self.type_inst);
                                    let (_, old) = compute_evaluator_memory_union(env, &fun_type);
                                    // Only create an entry if there are actually
                                    // modified memories. An empty entry would change
                                    // label resolution from "preserve as-is" (SaveMem
                                    // snapshot) to "walk back" mode, which is incorrect
                                    // for pure functions that don't modify resources.
                                    if !old.is_empty() {
                                        let info = label_info.entry(post_label).or_default();
                                        info.modified_types.extend(old);
                                        info.pre = info.pre.or(range.pre);
                                    }
                                }
                            }
                        },
                        AstOperation::SpecFunction(mid, fid, range) => {
                            if let Some(post_label) = range.post {
                                let inst = env.get_node_instantiation(*node_id);
                                let inst = Type::instantiate_slice(&inst, self.type_inst);
                                let module = env.get_module(*mid);
                                let sfun = module.get_spec_fun(*fid);
                                let info = label_info.entry(post_label).or_default();
                                for mem in &sfun.old_memory {
                                    info.modified_types.insert(mem.clone().instantiate(&inst));
                                }
                                info.pre = info.pre.or(range.pre);
                            }
                        },
                        _ => {},
                    }
                }
                true
            });
        }
        label_info
    }

    /// Declare Boogie variables for behavior state labels found in spec conditions
    /// and Prop bytecodes. These are memory snapshots used by behavioral predicates
    /// (ensures_of, etc.) with state labels connecting sequential calls.
    /// Already-declared names (in `declared_names`) are skipped.
    fn declare_behavior_state_labels(
        &self,
        fun_target: &FunctionTarget,
        code: &[Bytecode],
        env: &GlobalEnv,
        writer: &CodeWriter,
        declared_names: &mut BTreeSet<String>,
    ) -> BTreeMap<MemoryLabel, LabelInfo> {
        let spec = fun_target.func_env.get_spec();
        let mut all_exps: Vec<_> = spec
            .conditions
            .iter()
            .flat_map(|c| std::iter::once(&c.exp).chain(c.additional_exps.iter()))
            .cloned()
            .collect();
        drop(spec);
        for bc in code.iter() {
            if let Bytecode::Prop(_, _, exp) = bc {
                all_exps.push(exp.clone());
            }
        }

        let behavior_labels = self.collect_label_memory_pairs(&all_exps, code, env);
        let label_info = self.compute_label_info(&all_exps, code, env);

        // Emit declarations for all (label, type) pairs referenced by any
        // expression. The label resolution chain (label_info) determines which
        // memory state each reference resolves to at translation time; here we
        // just ensure every potentially-referenced variable exists in Boogie.
        for (lab, mem) in &behavior_labels {
            let name = boogie_resource_memory_name(env, mem, &Some(*lab));
            if !declared_names.insert(name.clone()) {
                continue;
            }
            emitln!(
                writer,
                "var {}: $Memory {};",
                name,
                boogie_struct_name(&env.get_struct_qid(mem.to_qualified_id()), &mem.inst, false)
            );
        }
        label_info
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
        let env = fun_target.global_env();
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

        // Assume function-typed parameters equal their parameter variant.
        // This enables behavioral predicate handling in the apply function.
        let params = fun_target.func_env.get_parameters();
        let fun_id = fun_target.func_env.get_qualified_id().instantiate(vec![]);
        for (i, param) in params.iter().enumerate() {
            let ty = fun_target.get_local_type(i);
            if matches!(ty, Type::Fun(..)) {
                let variant_name = boogie_fun_param_name(env, &fun_id, param.0);
                emitln!(writer, "assume $t{} == {}();", i, variant_name);
            }
        }
    }
}

// =================================================================================================
// Bytecode Translation

impl FunctionTranslator<'_> {
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
                    // Emit struct field identity assumptions for function-valued fields.
                    // When WellFormed is assumed for a struct with function-typed fields,
                    // the apply procedure needs to know which variant the field is.
                    self.emit_struct_field_identity_assumes(exp, env, writer);
                },
                PropKind::Modifies => {
                    let ty = self.inst(&env.get_node_type(exp.node_id()));
                    let ty = ty.skip_reference();
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
                    Constant::U16(num) => boogie_num_literal(&num.to_string(), 16, bv_flag),
                    Constant::U32(num) => boogie_num_literal(&num.to_string(), 32, bv_flag),
                    Constant::U64(num) => boogie_num_literal(&num.to_string(), 64, bv_flag),
                    Constant::U128(num) => boogie_num_literal(&num.to_string(), 128, bv_flag),
                    Constant::U256(num) => boogie_num_literal(&num.to_string(), 256, bv_flag),
                    Constant::I8(num) => boogie_num_literal(&num.to_string(), 8, bv_flag),
                    Constant::I16(num) => boogie_num_literal(&num.to_string(), 16, bv_flag),
                    Constant::I32(num) => boogie_num_literal(&num.to_string(), 32, bv_flag),
                    Constant::I64(num) => boogie_num_literal(&num.to_string(), 64, bv_flag),
                    Constant::I128(num) => boogie_num_literal(&num.to_string(), 128, bv_flag),
                    Constant::I256(num) => boogie_num_literal(&num.to_string(), 256, bv_flag),
                    Constant::Address(val) => boogie_address(env, val),
                    Constant::ByteArray(val) => boogie_byte_blob(options, val, bv_flag),
                    Constant::AddressArray(val) => boogie_address_blob(env, options, val),
                    Constant::Vector(val) => boogie_constant_blob(env, options, val),
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
                    OpaqueCallBegin(..) | OpaqueCallEnd(..) => {
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
                            );
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
                            let mut fun_name = boogie_function_name(&callee_env, inst, &[]);
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
                                                &local_ty_srcs_1,
                                                false,
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
                                    fun_name = boogie_function_name(&callee_env, inst, &[bv_flag]);
                                } else if module_env.is_table() {
                                    fun_name =
                                        boogie_function_name(&callee_env, inst, &[false, bv_flag]);
                                }
                                emitln!(writer, "call {}({});", fun_name, args_str);
                            } else {
                                let dest_bv_flag = !dests.is_empty() && compute_flag(dests[0]);
                                let bv_flag = !srcs.is_empty() && compute_flag(srcs[0]);
                                if module_env.is_cmp() {
                                    fun_name = boogie_function_name(&callee_env, inst, &[
                                        bv_flag || dest_bv_flag
                                    ]);
                                }
                                // Handle the case where the return value of length is assigned to a bv int because
                                // length always returns a non-bv result
                                if module_env.is_std_vector() {
                                    fun_name = boogie_function_name(&callee_env, inst, &[
                                        bv_flag || dest_bv_flag
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
                                                &local_ty,
                                                false,
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
                                    fun_name = boogie_function_name(&callee_env, inst, &[
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
                                                &local_ty,
                                                false,
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
                            boogie_struct_name(&struct_env, inst, false),
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
                        let signer_str = str_local(srcs[0]);
                        let value_str = str_local(srcs[1]);
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
                        let (target_kind, target_base) = match oper {
                            CastU8 => ("U", "8"),
                            CastU16 => ("U", "16"),
                            CastU32 => ("U", "32"),
                            CastU64 => ("U", "64"),
                            CastU128 => ("U", "128"),
                            CastU256 => ("U", "256"),
                            _ => unreachable!(),
                        };

                        let num_oper = global_state
                            .get_temp_index_oper(mid, fid, src, baseline_flag)
                            .unwrap();

                        if self.bv_flag(num_oper) {
                            let src_type = self.get_local_type(src);
                            let src_base = boogie_num_type_base(
                                self.parent.env,
                                Some(self.fun_target.get_bytecode_loc(attr_id)),
                                &src_type,
                                false,
                            );
                            emitln!(
                                writer,
                                "call {} := $CastBv{}to{}({});",
                                str_local(dest),
                                src_base,
                                target_base,
                                str_local(src)
                            );
                        } else {
                            emitln!(
                                writer,
                                "call {} := $Cast{}{}({});",
                                str_local(dest),
                                target_kind,
                                target_base,
                                str_local(src)
                            );
                        }
                    },
                    CastI8 | CastI16 | CastI32 | CastI64 | CastI128 | CastI256 => {
                        let src = srcs[0];
                        let dest = dests[0];
                        let (target_kind, target_base) = match oper {
                            CastI8 => ("I", "8"),
                            CastI16 => ("I", "16"),
                            CastI32 => ("I", "32"),
                            CastI64 => ("I", "64"),
                            CastI128 => ("I", "128"),
                            CastI256 => ("I", "256"),
                            _ => unreachable!(),
                        };
                        let num_oper = global_state
                            .get_temp_index_oper(mid, fid, src, baseline_flag)
                            .unwrap();
                        if self.bv_flag(num_oper) {
                            // src is a bitvector: convert it to int and then do cast
                            let src_type = self.get_local_type(src);
                            let src_base = boogie_num_type_base(
                                self.parent.env,
                                Some(self.fun_target.get_bytecode_loc(attr_id)),
                                &src_type,
                                false,
                            );
                            emitln!(
                                writer,
                                "call {} := $Cast{}{}($bv2int.{}({}));",
                                str_local(dest),
                                target_kind,
                                target_base,
                                src_base,
                                str_local(src)
                            );
                        } else {
                            emitln!(
                                writer,
                                "call {} := $Cast{}{}({});",
                                str_local(dest),
                                target_kind,
                                target_base,
                                str_local(src)
                            );
                        }
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
                                boogie_num_type_string_capital("U", "8", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::U16) => format!(
                                "{}{}",
                                boogie_num_type_string_capital("U", "16", bv_flag),
                                unchecked
                            ),
                            Type::Primitive(PrimitiveType::U32) => format!(
                                "{}{}",
                                boogie_num_type_string_capital("U", "32", bv_flag),
                                unchecked
                            ),
                            Type::Primitive(PrimitiveType::U64) => format!(
                                "{}{}",
                                boogie_num_type_string_capital("U", "64", bv_flag),
                                unchecked
                            ),
                            Type::Primitive(PrimitiveType::U128) => format!(
                                "{}{}",
                                boogie_num_type_string_capital("U", "128", bv_flag),
                                unchecked
                            ),
                            Type::Primitive(PrimitiveType::U256) => format!(
                                "{}{}",
                                boogie_num_type_string_capital("U", "256", bv_flag),
                                unchecked
                            ),
                            Type::Primitive(PrimitiveType::I8) => format!(
                                "{}{}",
                                boogie_num_type_string_capital("I", "8", bv_flag),
                                unchecked
                            ),
                            Type::Primitive(PrimitiveType::I16) => format!(
                                "{}{}",
                                boogie_num_type_string_capital("I", "16", bv_flag),
                                unchecked
                            ),
                            Type::Primitive(PrimitiveType::I32) => format!(
                                "{}{}",
                                boogie_num_type_string_capital("I", "32", bv_flag),
                                unchecked
                            ),
                            Type::Primitive(PrimitiveType::I64) => format!(
                                "{}{}",
                                boogie_num_type_string_capital("I", "64", bv_flag),
                                unchecked
                            ),
                            Type::Primitive(PrimitiveType::I128) => format!(
                                "{}{}",
                                boogie_num_type_string_capital("I", "128", bv_flag),
                                unchecked
                            ),
                            Type::Primitive(PrimitiveType::I256) => format!(
                                "{}{}",
                                boogie_num_type_string_capital("I", "256", bv_flag),
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
                        let sub_type = match &self.get_local_type(dest) {
                            Type::Primitive(PrimitiveType::U8) => {
                                boogie_num_type_string_capital("U", "8", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::U16) => {
                                boogie_num_type_string_capital("U", "16", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::U32) => {
                                boogie_num_type_string_capital("U", "32", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::U64) => {
                                boogie_num_type_string_capital("U", "64", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::U128) => {
                                boogie_num_type_string_capital("U", "128", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::U256) => {
                                boogie_num_type_string_capital("U", "256", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::I8) => {
                                boogie_num_type_string_capital("I", "8", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::I16) => {
                                boogie_num_type_string_capital("I", "16", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::I32) => {
                                boogie_num_type_string_capital("I", "32", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::I64) => {
                                boogie_num_type_string_capital("I", "64", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::I128) => {
                                boogie_num_type_string_capital("I", "128", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::I256) => {
                                boogie_num_type_string_capital("I", "256", bv_flag)
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
                            "call {} := $Sub{}({}, {});",
                            str_local(dest),
                            sub_type,
                            str_local(op1),
                            str_local(op2)
                        );
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
                                boogie_num_type_string_capital("U", "8", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::U16) => {
                                boogie_num_type_string_capital("U", "16", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::U32) => {
                                boogie_num_type_string_capital("U", "32", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::U64) => {
                                boogie_num_type_string_capital("U", "64", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::U128) => {
                                boogie_num_type_string_capital("U", "128", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::U256) => {
                                boogie_num_type_string_capital("U", "256", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::I8) => {
                                boogie_num_type_string_capital("I", "8", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::I16) => {
                                boogie_num_type_string_capital("I", "16", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::I32) => {
                                boogie_num_type_string_capital("I", "32", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::I64) => {
                                boogie_num_type_string_capital("I", "64", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::I128) => {
                                boogie_num_type_string_capital("I", "128", bv_flag)
                            },
                            Type::Primitive(PrimitiveType::I256) => {
                                boogie_num_type_string_capital("I", "256", bv_flag)
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
                    Negate => {
                        let dest = dests[0];
                        let op = srcs[0];
                        let num_oper = global_state
                            .get_temp_index_oper(mid, fid, dest, baseline_flag)
                            .unwrap();
                        if self.bv_flag(num_oper) {
                            bv_op_not_enabled_error!(bytecode, fun_target, env, loc);
                        }
                        let neg_type = match &self.get_local_type(dest) {
                            Type::Primitive(PrimitiveType::I8) => "I8",
                            Type::Primitive(PrimitiveType::I16) => "I16",
                            Type::Primitive(PrimitiveType::I32) => "I32",
                            Type::Primitive(PrimitiveType::I64) => "I64",
                            Type::Primitive(PrimitiveType::I128) => "I128",
                            Type::Primitive(PrimitiveType::I256) => "I256",
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
                            "call {} := $Negate{}({});",
                            str_local(dest),
                            neg_type,
                            str_local(op),
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
                                false,
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
                                            op1_ty,
                                            false,
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
                                            op2_ty,
                                            false,
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
                        let suffix = boogie_type_suffix(env, &self.get_local_type(msg), false);
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
                                &self.get_local_type(*code),
                                false,
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
            Abort(_, src, _) => {
                let num_oper_code = global_state
                    .get_temp_index_oper(mid, fid, *src, baseline_flag)
                    .unwrap();
                let int2bv_str = if *num_oper_code == Bitwise {
                    format!(
                        "$bv2int.{}({})",
                        boogie_num_type_base(
                            self.parent.env,
                            Some(self.fun_target.get_bytecode_loc(attr_id)),
                            &self.get_local_type(*src),
                            false,
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
                if matches!(edge, BorrowEdge::Invoke) {
                    emitln!(writer, "call $t{} := $HavocMutation($t{});", idx, idx);
                } else {
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
                }
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
                BorrowEdge::Hyper(_) | BorrowEdge::Invoke => unreachable!("unexpected borrow edge"),
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
            let suffix = boogie_type_suffix(env, ty, false);
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

    /// Emit identity assumptions for function-valued struct fields.
    /// After a WellFormed assume for a struct with storable function-typed fields,
    /// this emits `assume <value>-><field> is <struct_field_ctor>;` so the apply
    /// procedure can dispatch on the correct variant.
    fn emit_struct_field_identity_assumes(&self, exp: &Exp, env: &GlobalEnv, writer: &CodeWriter) {
        let mono_info = mono_analysis::get_info(env);
        if mono_info.fun_struct_field_infos.is_empty() {
            return;
        }
        // Only extract WellFormed from top-level positive conjuncts — not from
        // inside disjunctions, negations, or other non-conjunctive positions,
        // where injecting identity assumptions would be unsound.
        use move_model::exp_simplifier::flatten_conjunction_owned;
        for conjunct in flatten_conjunction_owned(exp) {
            if let ExpData::Call(_, AstOperation::WellFormed, args) = conjunct.as_ref() {
                if let Some(arg) = args.first() {
                    let ty = self.inst(&env.get_node_type(arg.node_id()));
                    self.emit_struct_field_identity_for_type(arg, &ty, env, writer, &mono_info);
                }
            }
        }
    }

    /// Recursively emit identity assumptions for a value of a given struct type.
    fn emit_struct_field_identity_for_type(
        &self,
        value_exp: &Exp,
        ty: &Type,
        env: &GlobalEnv,
        writer: &CodeWriter,
        mono_info: &mono_analysis::MonoInfo,
    ) {
        let ty = ty.skip_reference();
        let Type::Struct(mid, sid, inst) = ty else {
            return;
        };
        let struct_env = env.get_module(*mid).into_struct(*sid);
        // Get the Boogie expression for the value. We handle Temporary nodes
        // which are the common case in WellFormed assumptions.
        let boogie_value = match value_exp.as_ref() {
            ExpData::Temporary(_, idx) => {
                let val_ty = env.get_node_type(value_exp.node_id());
                if val_ty.is_mutable_reference() {
                    format!("$Dereference($t{})", idx)
                } else {
                    format!("$t{}", idx)
                }
            },
            _ => return, // Only handle temporaries for now
        };
        let fields: Vec<_> = if struct_env.has_variants() {
            struct_env
                .get_variants()
                .flat_map(|v| struct_env.get_fields_of_variant(v).collect::<Vec<_>>())
                .collect()
        } else {
            struct_env.get_fields().collect()
        };
        for field in &fields {
            let field_ty = field.get_type().instantiate(inst);
            if let Type::Fun(params, results, abilities) = &field_ty {
                if abilities.has_store() {
                    // Normalize to check against MonoInfo (same as mono_analysis::normalize_fun_ty)
                    let normalized = Type::Fun(
                        Box::new(Type::tuple(params.clone().flatten())),
                        Box::new(Type::tuple(results.clone().flatten())),
                        AbilitySet::EMPTY,
                    );
                    let struct_qid = struct_env.get_qualified_id().instantiate(inst.clone());
                    // Check if this field has a StructFieldInfo entry
                    if let Some(field_infos) = mono_info.fun_struct_field_infos.get(&normalized) {
                        let has_entry = field_infos.iter().any(|info| {
                            info.struct_id == struct_qid && info.field_sym == field.get_name()
                        });
                        if has_entry {
                            let field_sel = boogie_field_sel(field);
                            let ctor_name =
                                boogie_struct_field_name(env, &struct_qid, field.get_name());
                            emitln!(
                                writer,
                                "assume {}->{} is {};",
                                boogie_value,
                                field_sel,
                                ctor_name
                            );
                        }
                    }
                }
            }
        }
        // Recurse into struct fields for transitive function-valued fields
        for field in &fields {
            let field_ty = field.get_type().instantiate(inst);
            if let Type::Struct(..) = field_ty.skip_reference() {
                // Only recurse if the nested struct has function-valued fields
                // (transitively). For simplicity, we skip recursion for now
                // as this can be added when needed for deeper nesting.
            }
        }
    }
}

/// Look backwards through bytecode to find the Closure operation that assigns to
/// the given temporary. Returns (mid, fid, types, mask) if found.
fn find_closure_for_temp(
    code: &[Bytecode],
    idx: TempIndex,
) -> Option<(move_model::model::ModuleId, FunId, Vec<Type>, ClosureMask)> {
    for bc in code.iter().rev() {
        if let Bytecode::Call(_, dests, Operation::Closure(mid, fid, types, mask), _srcs, _) = bc {
            if dests.first() == Some(&idx) {
                return Some((*mid, *fid, types.clone(), *mask));
            }
        }
    }
    None
}

fn derive_closure_frame_access(
    closure_fun: &FunctionEnv,
    inst: &[Type],
) -> BTreeMap<QualifiedInstId<StructId>, FrameAccessKind> {
    let mut result = BTreeMap::new();
    let all_memory: BTreeSet<_> = closure_fun
        .get_spec_used_memory()
        .iter()
        .chain(closure_fun.get_spec_old_memory().iter())
        .map(|m| m.clone().instantiate(inst))
        .collect();

    if !closure_fun.is_opaque() {
        // Non-opaque: all resources get WritesAll
        for mem in all_memory {
            result.insert(mem, FrameAccessKind::WritesAll);
        }
        return result;
    }

    // Opaque: classify based on modifies targets
    let modify_targets = closure_fun.get_modify_targets();
    for mem in &all_memory {
        let qid = mem.to_qualified_id();
        if let Some(targets) = modify_targets.get(&qid) {
            // Extract address expressions from modifies targets
            let addrs: Vec<Exp> = targets
                .iter()
                .map(|target| target.call_args()[0].clone())
                .collect();
            result.insert(mem.clone(), FrameAccessKind::WritesAt(addrs));
        } else {
            result.insert(mem.clone(), FrameAccessKind::Reads);
        }
    }
    result
}

/// Derive frame access for a function parameter variant in the apply procedure.
///
/// Uses `access_of` declarations from the function spec to classify resources.
/// If no `access_of` is declared for the parameter, all resources get `WritesAll`.
/// Resources with `reads` access get `Reads`. Resources with `writes` access get
/// `WritesAt` if address specifiers exist, otherwise `WritesAll`.
///
/// `data_args` provides the Boogie argument strings for the apply parameters,
/// used to resolve address specifiers that reference parameters.
fn derive_param_frame_access(
    env: &GlobalEnv,
    fun_qid: &QualifiedInstId<FunId>,
    param_sym: Symbol,
    data_args: &[String],
) -> BTreeMap<QualifiedInstId<StructId>, ApplyFrameAccess> {
    let fun_env = env.get_function(fun_qid.to_qualified_id());
    let mut result = BTreeMap::new();

    // Get all resources used by this parameter
    let (used_memory, _old_memory) = BoogieTranslator::get_param_memory(env, fun_qid, param_sym);

    // Find modifies_of/reads_of declaration for this parameter
    let param_access = fun_env.get_fun_param_access_of();
    let access_decl = param_access.iter().find(|a| a.fun_param == param_sym);

    match access_decl {
        None => {
            // No declaration: conservatively assume all resources can be written
            for mem in used_memory {
                result.insert(mem, ApplyFrameAccess::WritesAll);
            }
        },
        Some(decl) => {
            // Wildcard: modifies_of<f> * → WritesAll for all memory
            if decl.frame_spec.modifies_all {
                for mem in used_memory {
                    result.insert(mem, ApplyFrameAccess::WritesAll);
                }
                return result;
            }
            // Wildcard: reads_of<f> * → Reads for all memory
            if decl.frame_spec.reads_all {
                for mem in used_memory {
                    result.insert(mem, ApplyFrameAccess::Reads);
                }
                return result;
            }
            // Classify each resource based on modifies_targets and reads_types
            let mut write_resources: BTreeMap<QualifiedInstId<StructId>, Vec<String>> =
                BTreeMap::new();

            // From modifies_targets: extract resource type and address expression
            for target in &decl.frame_spec.modifies_targets {
                if let ExpData::Call(id, AstOperation::Global(_), args) = target.as_ref() {
                    let ty = env.get_node_type(*id);
                    // The node type may be &T (reference); strip it.
                    let ty = ty.skip_reference();
                    if let Type::Struct(mid, sid, inst) = ty {
                        let res = mid
                            .qualified_inst(*sid, inst.clone())
                            .instantiate(&fun_qid.inst);
                        // Extract address expression and convert to Boogie string
                        // by substituting LocalVar references with data_args
                        let addr_str = if !args.is_empty() {
                            local_var_exp_to_boogie_str(&args[0], &decl.modifies_params, data_args)
                        } else {
                            None
                        };
                        if let Some(addr) = addr_str {
                            write_resources.entry(res).or_default().push(addr);
                        } else {
                            write_resources.entry(res).or_default();
                        }
                    }
                }
            }

            // reads_targets (already QualifiedInstId<StructId>)
            let read_resources: BTreeSet<_> = decl
                .frame_spec
                .reads_targets
                .iter()
                .map(|r| r.clone().instantiate(&fun_qid.inst))
                .collect();

            // Assign access kinds to all used memory
            for mem in used_memory {
                if let Some(addrs) = write_resources.get(&mem) {
                    if addrs.is_empty() {
                        result.insert(mem, ApplyFrameAccess::WritesAll);
                    } else {
                        result.insert(mem, ApplyFrameAccess::WritesAt(addrs.clone()));
                    }
                } else if read_resources.contains(&mem) {
                    result.insert(mem, ApplyFrameAccess::Reads);
                } else {
                    // Resource is in used_memory but not explicitly declared;
                    // conservatively treat as reads
                    result.insert(mem, ApplyFrameAccess::Reads);
                }
            }
        },
    }
    result
}

/// Converts an address expression from a `modifies_of` target to a Boogie string,
/// substituting `LocalVar` references (formal parameters) with their corresponding
/// Boogie argument strings.
fn local_var_exp_to_boogie_str(
    exp: &Exp,
    params: &[Parameter],
    data_args: &[String],
) -> Option<String> {
    match exp.as_ref() {
        ExpData::LocalVar(_, sym) => {
            // Find this symbol in params and substitute with the corresponding data_arg
            params
                .iter()
                .position(|p| p.0 == *sym)
                .and_then(|pos| data_args.get(pos).cloned())
        },
        _ => None,
    }
}

/// Derive frame access for a struct field variant in the apply procedure.
///
/// Uses `reads_of`/`modifies_of` declarations from the struct spec block.
/// If no declaration exists for the field, the function is assumed pure (empty map —
/// all memory preserved). `data_args` provides the Boogie argument strings; the first
/// element is `fun->n` (instance discriminator), followed by the actual parameters.
fn derive_struct_field_frame_access(
    env: &GlobalEnv,
    struct_env: &StructEnv<'_>,
    field_sym: Symbol,
    data_args: &[String],
) -> (
    BTreeSet<QualifiedInstId<StructId>>,
    BTreeSet<QualifiedInstId<StructId>>,
    BTreeMap<QualifiedInstId<StructId>, ApplyFrameAccess>,
) {
    let field_access = struct_env.get_field_access_of();
    let access_decl = field_access.iter().find(|a| a.fun_param == field_sym);
    match access_decl {
        None => {
            // No declaration: assume pure (no memory access)
            (BTreeSet::new(), BTreeSet::new(), BTreeMap::new())
        },
        Some(decl) => {
            let mut used_memory = decl.used_memory.clone();
            let mut old_memory = decl.old_memory.clone();
            // For wildcards, populate with all translated memory types
            if decl.frame_spec.modifies_all || decl.frame_spec.reads_all {
                let mono_info = mono_analysis::get_info(env);
                for qid in mono_info.all_memory_qids(env) {
                    used_memory.insert(qid.clone());
                    if decl.frame_spec.modifies_all {
                        old_memory.insert(qid);
                    }
                }
            }
            let mut result = BTreeMap::new();

            // Wildcard: modifies_of<f> * → WritesAll for all memory
            if decl.frame_spec.modifies_all {
                for mem in &used_memory {
                    result.insert(mem.clone(), ApplyFrameAccess::WritesAll);
                }
                return (used_memory, old_memory, result);
            }
            // Wildcard: reads_of<f> * → Reads for all memory
            if decl.frame_spec.reads_all {
                for mem in &used_memory {
                    result.insert(mem.clone(), ApplyFrameAccess::Reads);
                }
                return (used_memory, old_memory, result);
            }

            // Classify resources from modifies_targets
            let mut write_resources: BTreeMap<QualifiedInstId<StructId>, Vec<String>> =
                BTreeMap::new();
            for target in &decl.frame_spec.modifies_targets {
                if let ExpData::Call(id, AstOperation::Global(_), args) = target.as_ref() {
                    let ty = env.get_node_type(*id).skip_reference().clone();
                    if let Type::Struct(mid, sid, inst) = &ty {
                        let res = mid.qualified_inst(*sid, inst.clone());
                        // Resolve address expression: data_args[1..] are the actual params
                        // (data_args[0] is `fun->n`)
                        let addr_str = if !args.is_empty() {
                            local_var_exp_to_boogie_str(
                                &args[0],
                                &decl.modifies_params,
                                &data_args[1..],
                            )
                        } else {
                            None
                        };
                        if let Some(addr) = addr_str {
                            write_resources.entry(res).or_default().push(addr);
                        } else {
                            write_resources.entry(res).or_default();
                        }
                    }
                }
            }

            // Classify each used resource
            for mem in &used_memory {
                if let Some(addrs) = write_resources.get(mem) {
                    if addrs.is_empty() {
                        result.insert(mem.clone(), ApplyFrameAccess::WritesAll);
                    } else {
                        result.insert(mem.clone(), ApplyFrameAccess::WritesAt(addrs.clone()));
                    }
                } else {
                    // Resource in reads_targets or used_memory but not in
                    // modifies_targets — treat as read-only
                    result.insert(mem.clone(), ApplyFrameAccess::Reads);
                }
            }

            (used_memory, old_memory, result)
        },
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
