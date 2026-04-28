// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    ast::{
        AbortKind, AccessSpecifierKind, AddressSpecifier, Condition, ConditionKind, Exp, ExpData,
        FrameSpec, LambdaCaptureKind, Operation, Pattern, PropertyBag, PropertyValue, QuantKind,
        ResourceSpecifier, Spec, SpecVarDecl, TempIndex, Value,
    },
    code_writer::CodeWriter,
    emit, emitln,
    exp_builder::ExpBuilder,
    model::{
        FieldEnv, FunId, FunctionEnv, GlobalEnv, ModuleId, NodeId, Parameter, QualifiedId,
        QualifiedInstId, SpecVarId, StructEnv, StructId, TypeParameter, Visibility,
    },
    symbol::Symbol,
    ty::{PrimitiveType, ReferenceKind, Type, TypeDisplayContext},
    well_known::{BORROW_GLOBAL, BORROW_GLOBAL_MUT, UNSPECIFIED_ABORT_CODE},
};
use itertools::Itertools;
use move_core_types::ability::AbilitySet;
use num::{BigInt, Zero};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
};
//
// ========================================================================================
//

/// A type which allows to convert function, struct, and module definitions into
/// Move source.
pub struct Sourcifier<'a> {
    builder: ExpBuilder<'a>,
    writer: CodeWriter,
    // A mapping from symbols to their aliased string representation, if any.
    sym_alias_map: RefCell<BTreeMap<Symbol, String>>,
    // whether to amend the displayed results to be recompilable (e.g., remove `__` from lambda names) and more readable (e.g., local var names starting from `_v0`)
    amend: bool,
    /// When set, state labels matching this label are suppressed during rendering.
    /// Used to avoid redundant nested labels like `S1 |~ global<T>(..S1 |~ ...)`.
    enclosing_state_label: Cell<Option<crate::ast::MemoryLabel>>,
}

/// Returns true if a quantifier is "simple" — all ranges use TypeDomain with
/// empty args (renders as `x: Type`), no triggers, and no where clause.
/// Such quantifiers have no expressions in their header, so hoisting a state
/// label from the body through them is safe.
fn is_simple_quant(
    ranges: &[(Pattern, Exp)],
    triggers: &[Vec<Exp>],
    where_clause: &Option<Exp>,
) -> bool {
    triggers.is_empty() && where_clause.is_none() && ranges.iter().all(|(_, range)| {
        matches!(range.as_ref(), ExpData::Call(_, Operation::TypeDomain, args) if args.is_empty())
            || matches!(range.as_ref(), ExpData::Call(_, Operation::StateDomain, _))
    })
}

// NOTE: has_any_state_label was removed — replaced by is_state_neutral which also
// catches unlabeled Global(None)/Exists(None) references that would be corrupted by hoisting.

/// Returns true if the expression has a single state label that can be hoisted
/// out of an enclosing operation for readability. Looks through Not, binary
/// ops (where the other side has no labels), and quantifiers.
fn has_hoistable_label(exp: &Exp) -> bool {
    match exp.as_ref() {
        ExpData::Call(_, Operation::Global(Some(_)), _)
        | ExpData::Call(_, Operation::Exists(Some(_)), _) => true,
        ExpData::Call(_, Operation::Behavior(_, range), _) => {
            range.pre.is_some() || range.post.is_some()
        },
        // Look through Not
        ExpData::Call(_, Operation::Not, args) if args.len() == 1 => has_hoistable_label(&args[0]),
        // Look through hoistable binary ops — but only if the other operand is
        // state-neutral (no global/exists accesses that would be corrupted by hoisting).
        ExpData::Call(_, op, args) if args.len() == 2 && hoistable_bin_op_info(op).is_some() => {
            (has_hoistable_label(&args[0]) && is_state_neutral(&args[1]))
                || (has_hoistable_label(&args[1]) && is_state_neutral(&args[0]))
        },
        // Look through simple quantifiers (type-domain ranges only, no triggers/where)
        ExpData::Quant(_, _, ranges, triggers, where_clause, body)
            if is_simple_quant(ranges, triggers, where_clause) =>
        {
            has_hoistable_label(body)
        },
        _ => false,
    }
}

/// Returns true if the expression contains no state-sensitive operations — neither explicit
/// labels nor unlabeled global/exists accesses. An expression is state-neutral when hoisting
/// a label over it would not change its meaning after a sourcify → parse round-trip.
///
/// This subsumes and replaces `!has_any_state_label(exp)` for the hoisting check:
/// Delegates to `ExpData::is_state_neutral`.
fn is_state_neutral(exp: &Exp) -> bool {
    exp.as_ref().is_state_neutral()
}

/// Returns the priority and operator string for binary ops that support label hoisting.
fn hoistable_bin_op_info(op: &Operation) -> Option<(Priority, &'static str)> {
    match op {
        Operation::And => Some((Prio::LogicalAnd, "&&")),
        Operation::Or => Some((Prio::LogicalOr, "||")),
        Operation::Eq => Some((Prio::Relations, "==")),
        Operation::Neq => Some((Prio::Relations, "!=")),
        Operation::Lt => Some((Prio::Relations, "<")),
        Operation::Gt => Some((Prio::Relations, ">")),
        Operation::Le => Some((Prio::Relations, "<=")),
        Operation::Ge => Some((Prio::Relations, ">=")),
        Operation::Implies => Some((Prio::LogicalRelations, "==>")),
        Operation::Iff => Some((Prio::LogicalRelations, "<==>")),
        _ => None,
    }
}

// has_any_state_label removed — replaced by is_state_neutral (see above)

impl<'a> Sourcifier<'a> {
    /// Creates a new sourcifier.
    pub fn new(env: &'a GlobalEnv, amend: bool) -> Self {
        Self {
            builder: ExpBuilder::new(env),
            // Location not used, but required by the constructor
            writer: CodeWriter::new(env.unknown_loc()),
            amend,
            sym_alias_map: RefCell::new(BTreeMap::new()),
            enclosing_state_label: Cell::new(None),
        }
    }

    pub fn env(&self) -> &GlobalEnv {
        self.builder.env()
    }

    pub fn writer(&self) -> &CodeWriter {
        &self.writer
    }

    /// Destructs and returns the result
    pub fn result(self) -> String {
        self.writer.extract_result()
    }

    /// Prints a module.
    pub fn print_module(&self, module_id: ModuleId) {
        let module_env = self.env().get_module(module_id);
        if module_env.is_script_module() {
            emitln!(self.writer, "script {")
        } else {
            emitln!(self.writer, "module {} {{", module_env.get_full_name_str());
        }
        self.writer.indent();

        for use_ in module_env.get_use_decls() {
            emitln!(
                self.writer,
                "use {}{};",
                use_.module_name.display_full(self.env()).to_string(),
                use_.alias
                    .map(|alias| format!(" as {}", alias.display(self.env().symbol_pool())))
                    .unwrap_or_default()
            )
        }
        for friend_ in module_env.get_friend_decls() {
            emitln!(
                self.writer,
                "friend {};",
                friend_.module_name.display_full(self.env()).to_string()
            )
        }

        if module_env.get_struct_count() > 0 {
            for struct_env in module_env.get_structs() {
                // Skip ghost memory structs - they are internal to spec variables
                if !struct_env.is_ghost_memory() {
                    self.print_struct(struct_env.get_qualified_id());
                }
            }
        }
        if module_env.get_function_count() > 0 {
            for fun_env in module_env.get_functions() {
                // Skip auto-generated struct API wrappers (pack$S, unpack$S, borrow$S$N, …).
                // They are reconstructed by the compiler from the public struct declaration
                // and must not appear in decompiled source.
                if fun_env.is_struct_api() {
                    debug_assert!(
                        fun_env.get_def().is_none(),
                        "struct API wrapper `{}` should never have a def; \
                         lift_to_stackless_bytecode must have excluded it",
                        fun_env.get_name_string()
                    );
                    continue;
                }
                self.print_fun(fun_env.get_qualified_id(), fun_env.get_def())
            }
        }
        self.writer.unindent();
        emitln!(self.writer, "}")
    }

    /// Prints a function definition, where the defining expression is passed
    /// as a parameter.
    pub fn print_fun(&self, fun_id: QualifiedId<FunId>, def: Option<&Exp>) {
        let fun_env = self.env().get_function(fun_id);
        if !fun_env.module_env.is_script_module() {
            // Print attributes, visibility, and other modifiers
            if !fun_env.get_attributes().is_empty() {
                self.print_list("", "\n", "", fun_env.get_attributes().iter(), |attr| {
                    emit!(self.writer, "#[{}]", self.sym(attr.name()));
                });
                emitln!(self.writer);
            }
            match fun_env.visibility() {
                Visibility::Private => {},
                Visibility::Public => {
                    emit!(self.writer, "public ")
                },
                Visibility::Friend => {
                    emit!(self.writer, "friend ")
                },
            }
            if fun_env.is_entry() {
                emit!(self.writer, "entry ")
            }
            if fun_env.is_native() {
                emit!(self.writer, "native ")
            }
            if fun_env.is_inline() {
                emit!(self.writer, "inline ")
            }
        }

        emit!(
            self.writer,
            "fun {}{}",
            Self::amend_fun_name(self.env(), self.sym(fun_env.get_name()), self.amend),
            self.type_params(fun_env.get_type_parameters_ref())
        );
        let tctx = fun_env.get_type_display_ctx();
        let params = fun_env
            .get_parameters()
            .into_iter()
            .map(|Parameter(sym, ty, _)| format!("{}: {}", self.sym(sym), ty.display(&tctx)))
            .join(", ");
        emit!(self.writer, "({})", params);
        if fun_env.get_return_count() > 0 {
            emit!(
                self.writer,
                ": {}",
                fun_env.get_result_type().display(&tctx)
            )
        }
        if fun_env.get_access_specifiers().is_none() && !fun_env.is_native() {
            // Add a space so we open the code block with " {" on the same line.
            emit!(self.writer, " ");
        } else {
            self.print_access_specifiers(&tctx, &fun_env);
        }
        if let Some(def) = def {
            // Set up aliases for all temporary variables in the function body
            self.set_temp_var_alias(def);
            // A sequence or block is already automatically printed in braces with indent
            let requires_braces = !Self::is_braced(def);
            let exp_sourcifier = ExpSourcifier::for_fun(self, &fun_env, tctx, def, self.amend);
            if requires_braces {
                self.print_block(|| {
                    if !def.is_unit_exp() {
                        exp_sourcifier.print_exp(Prio::General, true, def);
                        emitln!(self.writer)
                    }
                })
            } else {
                exp_sourcifier.print_exp(Prio::General, true, def)
            }
            emitln!(self.writer);
            // Clear all temporary variable aliases after printing the function body
            self.clear_temp_var_alias();
        } else {
            emitln!(self.writer, ";");
        }

        // Print function spec if present
        self.print_fun_spec(&fun_env);
    }

    /// Amend function name to be recompilable
    fn amend_fun_name(env: &GlobalEnv, mut fun: String, amend: bool) -> String {
        // amend lambda function names
        if amend {
            if fun.starts_with("__lambda") {
                fun = fun.replacen("__", "", 1);
                fun = Self::new_global_unique_name(env, &fun);
            }
            // amend script entry function name
            if fun.starts_with("<SELF>_") {
                fun = fun.replacen("<SELF>_", "main_", 1);
            }
        }
        fun
    }

    fn is_braced(exp: &Exp) -> bool {
        match exp.as_ref() {
            ExpData::Sequence(_, exps) => exps.len() != 1 || Self::is_braced(&exps[0]),
            ExpData::Block(..) => true,
            _ => false,
        }
    }

    fn print_access_specifiers(&self, tctx: &TypeDisplayContext, fun: &FunctionEnv) {
        let Some(specs) = fun.get_access_specifiers() else {
            return;
        };
        self.writer.indent();
        let mut acc_spec_map = BTreeMap::new();

        // gather resources together under each spec kind
        for spec_kind in [
            "!reads",
            "!writes",
            "!acquires",
            "reads",
            "writes",
            "acquires",
        ] {
            acc_spec_map.insert(spec_kind.to_string(), BTreeSet::new());
        }

        for spec in specs {
            let resource = match &spec.resource.1 {
                ResourceSpecifier::Any => "*".to_string(),
                ResourceSpecifier::DeclaredAtAddress(addr) => {
                    format!("0x{}::*::*", addr.expect_numerical().short_str_lossless())
                },
                ResourceSpecifier::DeclaredInModule(mid) => {
                    format!("{}::*", self.env().get_module(*mid).get_full_name_str())
                },
                ResourceSpecifier::Resource(sid) => {
                    format!("{}", sid.to_type().display(tctx))
                },
            };

            let address = match &spec.address.1 {
                AddressSpecifier::Any => "".to_string(),
                AddressSpecifier::Address(addr) => {
                    format!("(0x{})", addr.expect_numerical().short_str_lossless())
                },
                AddressSpecifier::Parameter(sym) => {
                    format!("({})", self.sym(*sym))
                },
                AddressSpecifier::Call(fun, sym) => {
                    let func_env = self.env().get_function(fun.to_qualified_id());
                    format!(
                        "({}{}({}))",
                        self.module_qualifier(tctx, func_env.module_env.get_id()),
                        func_env.get_name_str(),
                        self.sym(*sym)
                    )
                },
            };

            let spec_kind = match spec.kind {
                AccessSpecifierKind::Reads => "reads",
                AccessSpecifierKind::Writes => "writes",
                AccessSpecifierKind::LegacyAcquires => "acquires",
            };

            let spec_key = if spec.negated {
                format!("!{}", spec_kind)
            } else {
                spec_kind.to_string()
            };

            acc_spec_map
                .get_mut(&spec_key)
                .expect("spec kind key expected")
                .insert(format!("{}{}", resource, address));
        }

        // print the spec kind and associated resources one by one
        for (spec_kind, resources) in &acc_spec_map {
            if !resources.is_empty() {
                emitln!(self.writer);
                self.print_list(
                    &format!("{} ", spec_kind),
                    ", ",
                    "",
                    resources.iter(),
                    |resource| emit!(self.writer, "{}", resource),
                );
            }
        }
        self.writer.unindent();
        emitln!(self.writer)
    }

    pub fn print_value(&self, value: &Value, ty: Option<&Type>, for_spec: bool) {
        match value {
            Value::Address(address) => emit!(self.writer, "@{}", self.env().display(address)),
            Value::Number(int) => {
                // Try to fold back into named MIN/MAX constants
                if let Some(Type::Primitive(prim)) = ty {
                    if let Some(name) = self.min_max_const_name(prim, int) {
                        emit!(self.writer, "{}", name);
                        return;
                    }
                }
                // Try MAX - k or MIN + k for small offsets (spec context only)
                if for_spec {
                    if let Some(s) = self.near_bound_name(ty, int) {
                        emit!(self.writer, "{}", s);
                        return;
                    }
                }
                emit!(self.writer, "{}", int);
                // Only print type suffix when not in spec context (specs use arbitrary precision)
                if !for_spec {
                    if let Some(Type::Primitive(prim)) = ty {
                        emit!(self.writer, match prim {
                            PrimitiveType::U8 => "u8",
                            PrimitiveType::U16 => "u16",
                            PrimitiveType::U32 => "u32",
                            PrimitiveType::U64 => "",
                            PrimitiveType::U128 => "u128",
                            PrimitiveType::U256 => "u256",
                            PrimitiveType::I8 => "i8",
                            PrimitiveType::I16 => "i16",
                            PrimitiveType::I32 => "i32",
                            PrimitiveType::I64 => "i64",
                            PrimitiveType::I128 => "i128",
                            PrimitiveType::I256 => "i256",
                            _ => "",
                        })
                    }
                }
            },
            Value::Bool(b) => emit!(self.writer, "{}", b),
            Value::Vector(values) => {
                // TODO: recognize printable string
                let elem_ty = if let Some(Type::Vector(elem_ty)) = ty {
                    Some(elem_ty.as_ref())
                } else {
                    None
                };
                self.print_list("vector[", ", ", "]", values.iter(), |value| {
                    self.print_value(value, elem_ty, for_spec)
                });
            },
            Value::Tuple(values) => {
                let elem_tys = if let Some(Type::Tuple(elem_tys)) = ty {
                    Some(elem_tys)
                } else {
                    None
                };
                self.print_list("(", ", ", ")", values.iter().enumerate(), |(pos, value)| {
                    let elem_ty = elem_tys.and_then(|tys| tys.get(pos));
                    self.print_value(value, elem_ty, for_spec)
                })
            },
            Value::ByteArray(bytes) => {
                self.print_list("vector[", ", ", "]", bytes.iter(), |byte| {
                    emit!(self.writer, "{}u8", byte)
                })
            },
            Value::AddressArray(addresses) => {
                self.print_list("vector[", ", ", "]", addresses.iter(), |address| {
                    emit!(self.writer, "@{}", self.env().display(address))
                })
            },
        }
    }

    /// Returns the name of a MIN/MAX constant if the given value matches the
    /// boundary of the given primitive type (e.g., `u8::MAX` -> `"MAX_U8"`).
    #[allow(clippy::only_used_in_recursion)]
    fn min_max_const_name(&self, prim: &PrimitiveType, value: &BigInt) -> Option<&'static str> {
        use PrimitiveType::*;
        // For `Num` (the spec-only abstract numeric type), try all concrete int types
        if *prim == Num {
            return PrimitiveType::all_int_types()
                .iter()
                .find_map(|t| self.min_max_const_name(t, value));
        }
        let max = prim.get_max_value()?;
        if *value == max {
            return Some(match prim {
                U8 => "MAX_U8",
                U16 => "MAX_U16",
                U32 => "MAX_U32",
                U64 => "MAX_U64",
                U128 => "MAX_U128",
                U256 => "MAX_U256",
                I8 => "MAX_I8",
                I16 => "MAX_I16",
                I32 => "MAX_I32",
                I64 => "MAX_I64",
                I128 => "MAX_I128",
                I256 => "MAX_I256",
                _ => return None,
            });
        }
        let min = prim.get_min_value()?;
        if *value == min && min != BigInt::from(0u64) {
            return Some(match prim {
                I8 => "MIN_I8",
                I16 => "MIN_I16",
                I32 => "MIN_I32",
                I64 => "MIN_I64",
                I128 => "MIN_I128",
                I256 => "MIN_I256",
                _ => return None,
            });
        }
        None
    }

    /// Returns a display string for a value near a type bound, e.g. `MAX_U64 - 1`.
    /// Only used for small offsets (≤ 32) and when the value's decimal representation
    /// is long enough that the bound expression is more readable (≥ 5 digits).
    fn near_bound_name(&self, ty: Option<&Type>, value: &BigInt) -> Option<String> {
        // Only use offset notation for values large enough to be unreadable as decimal
        let decimal_str = value.to_string();
        let decimal_len = if decimal_str.starts_with('-') {
            decimal_str.len() - 1
        } else {
            decimal_str.len()
        };
        if decimal_len < 5 {
            return None;
        }
        let try_prim = |prim: &PrimitiveType, value: &BigInt| -> Option<String> {
            if let Some(max) = prim.get_max_value() {
                let diff = &max - value;
                if diff > BigInt::zero() && diff <= BigInt::from(32) {
                    if let Some(name) = self.min_max_const_name(prim, &max) {
                        return Some(format!("{} - {}", name, diff));
                    }
                }
            }
            if let Some(min) = prim.get_min_value() {
                let diff = value - &min;
                if diff > BigInt::zero() && diff <= BigInt::from(32) {
                    if let Some(name) = self.min_max_const_name(prim, &min) {
                        return Some(format!("{} + {}", name, diff));
                    }
                }
            }
            None
        };
        let try_all = || {
            PrimitiveType::all_unsigned_int_types()
                .iter()
                .chain(PrimitiveType::all_signed_int_types().iter())
                .find_map(|t| try_prim(t, value))
        };
        match ty {
            Some(Type::Primitive(prim)) => {
                if *prim == PrimitiveType::Num {
                    try_all()
                } else {
                    try_prim(prim, value).or_else(try_all)
                }
            },
            // For non-primitive types (e.g., Num from reference erasure), try all integer types
            _ => try_all(),
        }
    }

    /// Prints a struct (or enum) declaration.
    pub fn print_struct(&self, struct_id: QualifiedId<StructId>) {
        let struct_env = self.env().get_struct(struct_id);
        let type_display_ctx = struct_env.get_type_display_ctx();
        let ability_str = if !struct_env.get_abilities().is_empty() {
            format!(" has {}", self.abilities(", ", struct_env.get_abilities()))
        } else {
            "".to_string()
        };
        let type_param_str = self.type_params(struct_env.get_type_parameters());
        let vis_prefix = match struct_env.get_visibility() {
            Visibility::Public => "public ",
            Visibility::Friend => "friend ",
            Visibility::Private => "",
        };
        if struct_env.has_variants() {
            emitln!(
                self.writer,
                "{}enum {}{}{} {{",
                vis_prefix,
                self.sym(struct_env.get_name()),
                type_param_str,
                ability_str
            );
            self.writer.indent();
            for variant in struct_env.get_variants() {
                emit!(self.writer, "{}", self.sym(variant));
                let fields = struct_env.get_fields_of_variant(variant).collect_vec();
                if !fields.is_empty() {
                    emitln!(self.writer, " {");
                    self.writer.indent();
                    for fld in fields {
                        emitln!(self.writer, "{},", self.field(&type_display_ctx, &fld))
                    }
                    self.writer.unindent();
                    emitln!(self.writer, "}")
                } else {
                    emitln!(self.writer, ",")
                }
            }
        } else {
            emitln!(
                self.writer,
                "{}struct {}{}{} {{",
                vis_prefix,
                self.sym(struct_env.get_name()),
                type_param_str,
                ability_str
            );
            self.writer.indent();
            for fld in struct_env.get_fields() {
                // TODO: currently we just filter the dummy field out, but the user might have
                //   actually declared one. We could try heuristics on the code whether the
                //   user intended to use the dummy field. (Its in a borrow field or in a
                //   Unpack to an unused destination.)
                if !self.is_dummy_field(&fld) {
                    emitln!(self.writer, "{},", self.field(&type_display_ctx, &fld))
                }
            }
        }
        self.writer.unindent();
        emitln!(self.writer, "}");

        // Print struct spec if present
        self.print_struct_spec(&struct_env);
    }

    /// Check for dummy field the compiler introduces for empty structs
    fn is_dummy_field(&self, fld: &FieldEnv) -> bool {
        fld.get_type().is_bool() && self.sym(fld.get_name()) == "dummy_field"
    }

    fn field(&self, tctx: &TypeDisplayContext, field: &FieldEnv) -> String {
        let mut tctx = tctx.clone();
        // If the field is a struct defined in a different module with name identical to any struct in the current module, e.g.,
        //    ```Move
        //    use 0x2::X;
        //    struct X {
        //        f: X::S,
        //    }
        //    ```
        //
        //    and
        //
        //    ```Move
        //    use 0x2::X;
        //    struct Y {
        //        f: X::S,
        //    }
        //    enum X {
        //        S,
        //    }
        //    ```
        // we will force to print out the module address (i.e., `f: X::S` -> `f: 0x2::X::S`).
        if let Type::Struct(field_mid, _, _) = field.get_type() {
            let sym_pool = self.env().symbol_pool();
            // Get the module containing the parent struct of the field
            let parent_module = &field.struct_env.module_env;
            let parent_mid = parent_module.get_id();
            // Field type is defined in a different module
            if parent_mid != field_mid {
                // Get the module name defining the field struct
                let field_mname = self
                    .env()
                    .get_module(field_mid)
                    .get_name()
                    .display(self.env())
                    .to_string();

                // If the parent module has any structure with the same name as `field_mname`
                if parent_module
                    .get_structs()
                    .any(|s| s.get_name().display(sym_pool).to_string() == field_mname)
                {
                    tctx.display_module_addr = true;
                }
            }
        }
        format!(
            "{}: {}",
            self.sym(field.get_name()),
            field.get_type().display(&tctx)
        )
    }

    fn type_params(&self, params: &[TypeParameter]) -> String {
        let type_params = params
            .iter()
            .map(|TypeParameter(sym, kind, _)| {
                let abilities = if kind.abilities.is_empty() {
                    "".to_string()
                } else {
                    format!(": {}", self.abilities(" + ", kind.abilities))
                };
                format!(
                    "{}{}{}",
                    if kind.is_phantom { "phantom " } else { "" },
                    self.sym(*sym),
                    abilities
                )
            })
            .join(", ");
        if !type_params.is_empty() {
            format!("<{}>", type_params)
        } else {
            "".to_string()
        }
    }

    fn abilities(&self, sep: &str, abilities: AbilitySet) -> String {
        abilities.iter().map(|a| a.to_string()).join(sep)
    }

    fn sym(&self, sym: Symbol) -> String {
        let sym_str = if let Some(alias) = self.sym_alias_map.borrow().get(&sym) {
            alias.clone()
        } else {
            sym.display(self.env().symbol_pool()).to_string()
        };

        // Replace the `invalid` characters so that the generated code is recompilable.
        if self.amend {
            sym_str.replace('$', "_")
        } else {
            sym_str
        }
    }

    fn print_block(&self, content_printer: impl Fn()) {
        emitln!(self.writer, "{");
        self.writer.indent();
        content_printer();
        self.writer.unindent();
        emit!(self.writer, "}")
    }

    fn print_list<T>(
        &self,
        start: &str,
        sep: &str,
        end: &str,
        items: impl Iterator<Item = T>,
        item_printer: impl Fn(T),
    ) {
        emit!(self.writer, start);
        let mut first = true;
        for item in items {
            if first {
                first = false
            } else {
                emit!(self.writer, "{}", sep)
            }
            item_printer(item)
        }
        emit!(self.writer, end)
    }

    fn module_qualifier(&self, tctx: &TypeDisplayContext, mid: ModuleId) -> String {
        let module_env = self.env().get_module(mid);
        let module_name = module_env.get_name();
        if tctx.is_current_module(module_name) {
            // Current module, no qualification needed
            "".to_string()
        } else {
            format!(
                "{}::",
                tctx.get_alias(module_name)
                    .map(|sym| self.sym(sym))
                    .unwrap_or_else(|| {
                        if tctx.used_modules.contains(&mid) {
                            self.sym(module_env.get_name().name())
                        } else {
                            module_env.get_full_name_str()
                        }
                    })
            )
        }
    }

    /// Creates a new name which has no clashes with symbols in the global environment.
    fn new_global_unique_name(env: &GlobalEnv, base_name: &str) -> String {
        let spool = env.symbol_pool();
        // Name is available
        if !spool.name_already_taken(base_name) {
            return base_name.to_string();
        }
        // Name is taken, try to append a postfix
        for i in 0..256 {
            let new_name = &format!("{}_{}", base_name, i);
            if !spool.name_already_taken(new_name) {
                return new_name.to_string();
            }
        }
        // Give up after too many attempts
        panic!("too many fruitless attempts to generate unique name")
    }

    fn is_temp_var(s: &str) -> bool {
        if let Some(rest) = s.strip_prefix("_t") {
            rest.chars().all(|c| c.is_ascii_digit())
        } else {
            false
        }
    }

    /// our assignment optimization at the AST level removes many dummy temp vars. As such, their names no longer start from `_t0`
    /// this function amends names of temp vars in the function body to start from `_v0`
    fn set_temp_var_alias(&self, def: &ExpData) {
        if self.amend {
            // Collect all used local variables in the function body
            // Why use vec instead of set: we need to keep the order of the symbols as they appear in the function body
            let mut used_vars = Vec::new();
            let mut visitor = |exp: &ExpData| {
                match exp {
                    ExpData::Block(_, pat, _, _) => {
                        for (_, sym) in pat.vars().iter() {
                            if !used_vars.contains(sym) {
                                used_vars.push(*sym);
                            }
                        }
                    },
                    ExpData::LocalVar(_, sym) => {
                        if !used_vars.contains(sym) {
                            used_vars.push(*sym);
                        }
                    },
                    ExpData::Assign(_, pat, _) => {
                        for (_, sym) in pat.vars().iter() {
                            if !used_vars.contains(sym) {
                                used_vars.push(*sym);
                            }
                        }
                    },
                    _ => {},
                }
                true
            };
            def.visit_pre_order(&mut visitor);

            // set up alias for each temp var
            for used_var in used_vars {
                let var_name = used_var.display(self.env().symbol_pool()).to_string();
                if Self::is_temp_var(&var_name) {
                    let new_name = Self::new_global_unique_name(
                        self.env(),
                        format!("_v{}", self.sym_alias_map.borrow().len()).as_str(),
                    );
                    self.sym_alias_map
                        .borrow_mut()
                        .insert(used_var, new_name.clone());
                }
            }
        }
    }

    fn clear_temp_var_alias(&self) {
        self.sym_alias_map.borrow_mut().clear();
    }

    // ========================================================================================
    // Spec printing

    /// Prints condition properties (e.g., `[concrete]`, `[global]`).
    fn print_properties(&self, props: &PropertyBag) {
        if !props.is_empty() {
            self.print_list("[", ", ", "] ", props.iter(), |(key, value)| {
                emit!(self.writer, "{}", key.display(self.env().symbol_pool()));
                match value {
                    PropertyValue::Value(Value::Bool(true)) => {
                        // Omit "= true", it's implied for boolean properties
                    },
                    PropertyValue::Value(v) => {
                        emit!(self.writer, " = ");
                        self.print_value(v, None, false);
                    },
                    PropertyValue::Symbol(s) => {
                        emit!(self.writer, " = {}", s.display(self.env().symbol_pool()));
                    },
                    PropertyValue::QualifiedSymbol(qs) => {
                        emit!(
                            self.writer,
                            " = {}::{}",
                            qs.module_name.display(self.env()),
                            qs.symbol.display(self.env().symbol_pool())
                        );
                    },
                }
            });
        }
    }

    /// Prints a single spec condition using the given expression sourcifier.
    fn print_condition(&self, cond: &Condition, exp_sourcifier: &ExpSourcifier) {
        // Each condition is a separate scope — reset the let-binding name counter
        // so identical extractions across conditions get the same names (enabling
        // structural equality comparison in the instrumentation pass).
        exp_sourcifier.let_counter.set(0);
        use ConditionKind::*;
        match &cond.kind {
            LetPre(name, _) => {
                emit!(
                    self.writer,
                    "let {} = ",
                    name.display(self.env().symbol_pool())
                );
                exp_sourcifier.print_exp(Prio::General, false, &cond.exp);
                emitln!(self.writer, ";");
            },
            LetPost(name, _) => {
                emit!(
                    self.writer,
                    "let post {} = ",
                    name.display(self.env().symbol_pool())
                );
                exp_sourcifier.print_exp(Prio::General, false, &cond.exp);
                emitln!(self.writer, ";");
            },
            Requires => {
                emit!(self.writer, "requires ");
                self.print_properties(&cond.properties);
                exp_sourcifier.print_exp(Prio::General, false, &cond.exp);
                emitln!(self.writer, ";");
            },
            Ensures => {
                emit!(self.writer, "ensures ");
                self.print_properties(&cond.properties);
                exp_sourcifier.print_exp_hoisting_label(&cond.exp);
                emitln!(self.writer, ";");
            },
            AbortsIf => {
                emit!(self.writer, "aborts_if ");
                self.print_properties(&cond.properties);
                exp_sourcifier.print_exp_hoisting_label(&cond.exp);
                // Check for abort code in additional_exps
                if !cond.additional_exps.is_empty() {
                    emit!(self.writer, " with ");
                    exp_sourcifier.print_exp(Prio::General, false, &cond.additional_exps[0]);
                }
                emitln!(self.writer, ";");
            },
            AbortsWith => {
                emit!(self.writer, "aborts_with ");
                self.print_properties(&cond.properties);
                let all_exps: Vec<_> = std::iter::once(&cond.exp)
                    .chain(&cond.additional_exps)
                    .collect();
                for (i, exp) in all_exps.iter().enumerate() {
                    if i > 0 {
                        emit!(self.writer, ", ");
                    }
                    exp_sourcifier.print_exp(Prio::General, false, exp);
                }
                emitln!(self.writer, ";");
            },
            SucceedsIf => {
                emit!(self.writer, "succeeds_if ");
                self.print_properties(&cond.properties);
                exp_sourcifier.print_exp(Prio::General, false, &cond.exp);
                emitln!(self.writer, ";");
            },
            Emits => {
                emit!(self.writer, "emits ");
                self.print_properties(&cond.properties);
                let exps: Vec<_> = std::iter::once(&cond.exp)
                    .chain(&cond.additional_exps)
                    .collect();
                // emits <msg> to <handle> [if <cond>]
                exp_sourcifier.print_exp(Prio::General, false, exps[0]);
                emit!(self.writer, " to ");
                exp_sourcifier.print_exp(Prio::General, false, exps[1]);
                if exps.len() > 2 {
                    emit!(self.writer, " if ");
                    exp_sourcifier.print_exp(Prio::General, false, exps[2]);
                }
                emitln!(self.writer, ";");
            },
            Assert => {
                emit!(self.writer, "assert ");
                self.print_properties(&cond.properties);
                exp_sourcifier.print_exp(Prio::General, false, &cond.exp);
                emitln!(self.writer, ";");
            },
            Assume => {
                emit!(self.writer, "assume ");
                self.print_properties(&cond.properties);
                exp_sourcifier.print_exp(Prio::General, false, &cond.exp);
                emitln!(self.writer, ";");
            },
            Decreases => {
                emit!(self.writer, "decreases ");
                self.print_properties(&cond.properties);
                exp_sourcifier.print_exp(Prio::General, false, &cond.exp);
                emitln!(self.writer, ";");
            },
            StructInvariant | FunctionInvariant | LoopInvariant => {
                emit!(self.writer, "invariant ");
                self.print_properties(&cond.properties);
                exp_sourcifier.print_exp(Prio::General, false, &cond.exp);
                emitln!(self.writer, ";");
            },
            GlobalInvariant(type_params) | GlobalInvariantUpdate(type_params) => {
                emit!(self.writer, "invariant");
                if !type_params.is_empty() {
                    self.print_list("<", ", ", ">", type_params.iter(), |(sym, _)| {
                        emit!(self.writer, "{}", sym.display(self.env().symbol_pool()));
                    });
                }
                if matches!(cond.kind, GlobalInvariantUpdate(_)) {
                    emit!(self.writer, " update ");
                } else {
                    emit!(self.writer, " ");
                }
                self.print_properties(&cond.properties);
                exp_sourcifier.print_exp(Prio::General, false, &cond.exp);
                emitln!(self.writer, ";");
            },
            SchemaInvariant => {
                emit!(self.writer, "invariant ");
                self.print_properties(&cond.properties);
                exp_sourcifier.print_exp(Prio::General, false, &cond.exp);
                emitln!(self.writer, ";");
            },
            Axiom(type_params) => {
                emit!(self.writer, "axiom");
                if !type_params.is_empty() {
                    self.print_list("<", ", ", ">", type_params.iter(), |(sym, _)| {
                        emit!(self.writer, "{}", sym.display(self.env().symbol_pool()));
                    });
                }
                emit!(self.writer, " ");
                self.print_properties(&cond.properties);
                exp_sourcifier.print_exp(Prio::General, false, &cond.exp);
                emitln!(self.writer, ";");
            },
            Update => {
                emit!(self.writer, "update ");
                self.print_properties(&cond.properties);
                // Update has the target in additional_exps[0] and value in exp
                if !cond.additional_exps.is_empty() {
                    exp_sourcifier.print_exp(Prio::General, false, &cond.additional_exps[0]);
                    emit!(self.writer, " = ");
                }
                exp_sourcifier.print_exp(Prio::General, false, &cond.exp);
                emitln!(self.writer, ";");
            },
        }
    }

    /// Collects all spec variables referenced in a spec.
    fn collect_spec_var_refs(&self, spec: &Spec) -> BTreeSet<QualifiedId<SpecVarId>> {
        let mut spec_vars = BTreeSet::new();
        for cond in &spec.conditions {
            self.collect_spec_var_refs_in_exp(&cond.exp, &mut spec_vars);
            for exp in &cond.additional_exps {
                self.collect_spec_var_refs_in_exp(exp, &mut spec_vars);
            }
        }
        spec_vars
    }

    /// Helper to collect spec variable references from an expression.
    fn collect_spec_var_refs_in_exp(
        &self,
        exp: &Exp,
        spec_vars: &mut BTreeSet<QualifiedId<SpecVarId>>,
    ) {
        exp.visit_pre_order(&mut |e| {
            if let ExpData::Call(_, Operation::Select(mid, sid, _), _) = e {
                let struct_env = self.env().get_module(*mid).into_struct(*sid);
                if let Some(sv_id) = struct_env.get_ghost_memory_spec_var() {
                    spec_vars.insert(sv_id);
                }
            }
            true
        });
    }

    /// Prints a spec variable declaration.
    fn print_spec_var_decl(&self, spec_var: &SpecVarDecl, tctx: &TypeDisplayContext) {
        emit!(self.writer, "global {}", self.sym(spec_var.name));
        if !spec_var.type_params.is_empty() {
            emit!(self.writer, "{}", self.type_params(&spec_var.type_params));
        }
        emit!(self.writer, ": {}", spec_var.type_.display(tctx));
        // Note: init expressions are not printed here as they are typically
        // not used in function spec contexts
        emitln!(self.writer, ";");
    }

    /// Prints pragma declarations from a spec's PropertyBag.
    /// Prints frame spec (modifies/reads) if present.
    fn print_frame_spec(&self, frame_spec: &Option<FrameSpec>, exp_sourcifier: &ExpSourcifier) {
        if let Some(frame) = frame_spec {
            for target in &frame.modifies_targets {
                emit!(self.writer, "modifies ");
                exp_sourcifier.print_exp(Prio::General, false, target);
                emitln!(self.writer, ";");
            }
            if !frame.reads_targets.is_empty() {
                emit!(self.writer, "reads ");
                let names: Vec<_> = frame
                    .reads_targets
                    .iter()
                    .map(|qid| self.env().display(qid).to_string())
                    .collect();
                emitln!(self.writer, "{};", names.join(", "));
            }
        }
    }

    fn print_pragmas(&self, properties: &PropertyBag) {
        if properties.is_empty() {
            return;
        }
        emit!(self.writer, "pragma ");
        let mut first = true;
        for (key, value) in properties {
            if !first {
                emit!(self.writer, ", ");
            }
            first = false;
            let key_str = key.display(self.env().symbol_pool());
            emit!(self.writer, "{} = {}", key_str, self.env().display(value));
        }
        emitln!(self.writer, ";");
    }

    /// Print `use` declarations inside a spec block for external modules
    /// referenced by SpecFunction calls that don't already have a module-level
    /// import in the enclosing module.
    fn print_spec_use_decls(
        &self,
        fun_env: &FunctionEnv,
        spec: &Spec,
        tctx: &mut TypeDisplayContext,
    ) {
        use crate::ast::Operation;
        let current_mid = fun_env.module_env.get_id();
        // Modules with module-level imports: `use m;`, `use m as n;`, or
        // `use m::{Self, ...}` — all make the module qualifier available.
        let pool = fun_env.module_env.env.symbol_pool();
        let self_sym = pool.make("Self");
        let env = self.env();
        let imported_names: BTreeSet<Symbol> = fun_env
            .module_env
            .get_use_decls()
            .iter()
            .filter(|u| {
                u.members.is_empty()
                    || u.alias.is_some()
                    || u.members.iter().any(|(_, sym, _)| *sym == self_sym)
            })
            .map(|u| u.module_name.name())
            .collect();
        let mut needed: BTreeSet<ModuleId> = BTreeSet::new();
        let mut check_mid = |mid: &ModuleId| {
            if *mid != current_mid
                && !imported_names.contains(&env.get_module(*mid).get_name().name())
            {
                needed.insert(*mid);
            }
        };
        for cond in &spec.conditions {
            cond.exp.as_ref().visit_pre_order(&mut |e: &ExpData| {
                if let ExpData::Call(_, Operation::SpecFunction(mid, _, _), _) = e {
                    check_mid(mid);
                }
                true
            });
        }
        if let Some(frame) = &spec.frame_spec {
            for target in &frame.modifies_targets {
                target.as_ref().visit_pre_order(&mut |e: &ExpData| {
                    if let ExpData::Call(_, Operation::SpecFunction(mid, _, _), _) = e {
                        check_mid(mid);
                    }
                    true
                });
            }
        }
        for mid in &needed {
            let module_env = self.env().get_module(*mid);
            emitln!(self.writer, "use {};", module_env.get_full_name_str());
        }
        // Add these modules to tctx so module_qualifier uses short names
        tctx.used_modules.extend(needed);
    }

    /// Prints a spec block for a function, including the repeated signature.
    pub fn print_fun_spec(&self, fun_env: &FunctionEnv) {
        let spec = fun_env.get_spec();
        let has_frame = spec
            .frame_spec
            .as_ref()
            .is_some_and(|f| !f.modifies_targets.is_empty() || !f.reads_targets.is_empty());
        if spec.conditions.is_empty()
            && spec.properties.is_empty()
            && !has_frame
            && spec.proof.is_none()
        {
            return;
        }

        emitln!(self.writer);
        emit!(self.writer, "spec ");
        self.print_fun_signature(fun_env);
        emitln!(self.writer, " {");

        self.writer.indent();
        let mut tctx = fun_env.get_type_display_ctx();

        // Print use declarations for external modules referenced by SpecFunction
        // calls in the spec conditions (e.g., `use 0x1::signer;`).
        // Also add these modules to tctx so module_qualifier uses short names.
        self.print_spec_use_decls(fun_env, &spec, &mut tctx);

        self.print_pragmas(&spec.properties);

        // Print frame spec (modifies/reads)
        {
            let exp_sourcifier =
                ExpSourcifier::for_fun_spec(self, fun_env, tctx.clone(), self.amend);
            self.print_frame_spec(&spec.frame_spec, &exp_sourcifier);
        }

        // Print spec variable declarations that are referenced in this spec
        let spec_var_ids = self.collect_spec_var_refs(&spec);
        for sv_id in spec_var_ids {
            let module_env = self.env().get_module(sv_id.module_id);
            let spec_var = module_env.get_spec_var(sv_id.id);
            self.print_spec_var_decl(spec_var, &tctx);
        }

        let exp_sourcifier = ExpSourcifier::for_fun_spec(self, fun_env, tctx, self.amend);
        for cond in &spec.conditions {
            self.print_condition(cond, &exp_sourcifier);
        }
        self.writer.unindent();

        // The `proof { ... }` clause is a sibling of the `spec { ... }` block in
        // source syntax, not a member, so emit it after the closing brace.
        if let Some(proof) = &spec.proof {
            emit!(self.writer, "} ");
            self.print_proof(proof, &exp_sourcifier);
        } else {
            emitln!(self.writer, "}");
        }
        emitln!(self.writer);
    }

    /// Prints just the function signature (name, type params, params, return type).
    fn print_fun_signature(&self, fun_env: &FunctionEnv) {
        let name = Sourcifier::amend_fun_name(self.env(), self.sym(fun_env.get_name()), self.amend);
        emit!(
            self.writer,
            "{}{}",
            name,
            self.type_params(fun_env.get_type_parameters_ref())
        );
        let tctx = fun_env.get_type_display_ctx();
        let params = fun_env
            .get_parameters()
            .into_iter()
            .map(|Parameter(sym, ty, _)| format!("{}: {}", self.sym(sym), ty.display(&tctx)))
            .join(", ");
        emit!(self.writer, "({})", params);
        if fun_env.get_return_count() > 0 {
            emit!(
                self.writer,
                ": {}",
                fun_env.get_result_type().display(&tctx)
            )
        }
    }

    /// Prints a spec block for a struct.
    pub fn print_struct_spec(&self, struct_env: &StructEnv) {
        let spec = struct_env.get_spec();
        if spec.conditions.is_empty() && spec.properties.is_empty() {
            return;
        }

        emitln!(self.writer);
        emitln!(self.writer, "spec {} {{", self.sym(struct_env.get_name()));

        self.writer.indent();
        self.print_pragmas(&spec.properties);
        let tctx = struct_env.get_type_display_ctx();
        let exp_sourcifier = ExpSourcifier::for_spec(self, tctx, self.amend);
        for cond in &spec.conditions {
            self.print_condition(cond, &exp_sourcifier);
        }
        self.writer.unindent();

        emitln!(self.writer, "}");
        emitln!(self.writer);
    }

    /// Prints a spec block directly from a Spec object with the given header.
    pub fn print_spec(&self, spec: &Spec, header: &str, tctx: TypeDisplayContext) {
        let has_frame = spec
            .frame_spec
            .as_ref()
            .is_some_and(|f| !f.modifies_targets.is_empty() || !f.reads_targets.is_empty());
        if spec.conditions.is_empty()
            && spec.properties.is_empty()
            && !has_frame
            && spec.proof.is_none()
        {
            return;
        }

        emitln!(self.writer);
        emitln!(self.writer, "spec {} {{", header);

        self.writer.indent();
        self.print_pragmas(&spec.properties);
        // Print frame spec (modifies/reads)
        {
            let exp_sourcifier = ExpSourcifier::for_spec(self, tctx.clone(), self.amend);
            self.print_frame_spec(&spec.frame_spec, &exp_sourcifier);
        }
        let exp_sourcifier = ExpSourcifier::for_spec(self, tctx, self.amend);
        for cond in &spec.conditions {
            self.print_condition(cond, &exp_sourcifier);
        }
        self.writer.unindent();

        // The `proof { ... }` clause is a sibling of the `spec { ... }` block in
        // source syntax, not a member, so emit it after the closing brace.
        if let Some(proof) = &spec.proof {
            emit!(self.writer, "} ");
            self.print_proof(proof, &exp_sourcifier);
        } else {
            emitln!(self.writer, "}");
        }
        emitln!(self.writer);
    }

    fn print_proof(&self, proof: &crate::ast::Proof, exp_sourcifier: &ExpSourcifier) {
        emitln!(self.writer, "proof {");
        self.writer.indent();
        self.print_proof_body(proof, exp_sourcifier);
        self.writer.unindent();
        emitln!(self.writer, "}");
    }

    fn print_proof_body(&self, proof: &crate::ast::Proof, exp_sourcifier: &ExpSourcifier) {
        use crate::ast::Proof;
        match proof {
            Proof::Let(_, sym, exp) => {
                let env = exp_sourcifier.env();
                emit!(self.writer, "let {} = ", sym.display(env.symbol_pool()));
                exp_sourcifier.print_exp(Prio::General, false, exp);
                emitln!(self.writer, ";");
            },
            Proof::IfElse(_, cond, then_branch, else_branch) => {
                emit!(self.writer, "if (");
                exp_sourcifier.print_exp(Prio::General, false, cond);
                emitln!(self.writer, ") {");
                self.writer.indent();
                self.print_proof_body(then_branch, exp_sourcifier);
                self.writer.unindent();
                if let Some(eb) = else_branch {
                    emitln!(self.writer, "} else {");
                    self.writer.indent();
                    self.print_proof_body(eb, exp_sourcifier);
                    self.writer.unindent();
                }
                emitln!(self.writer, "}");
            },
            Proof::Block(_, stmts) => {
                for stmt in stmts {
                    self.print_proof_body(stmt, exp_sourcifier);
                }
            },
            Proof::Assert(_, exp) => {
                emit!(self.writer, "assert ");
                exp_sourcifier.print_exp(Prio::General, false, exp);
                emitln!(self.writer, ";");
            },
            Proof::Assume(_, exp) => {
                emit!(self.writer, "assume [trusted] ");
                exp_sourcifier.print_exp(Prio::General, false, exp);
                emitln!(self.writer, ";");
            },
            Proof::Apply(_, lemma_qid, args) => {
                let env = exp_sourcifier.env();
                let module_env = env.get_module(lemma_qid.module_id);
                let lemma = module_env.get_lemma(lemma_qid.id);
                let lemma_name = lemma.name.display(env.symbol_pool());
                emit!(self.writer, "apply {}(", lemma_name);
                let mut comma = "";
                for arg in args {
                    emit!(self.writer, "{}", comma);
                    exp_sourcifier.print_exp(Prio::General, false, arg);
                    comma = ", ";
                }
                emitln!(self.writer, ");");
            },
            Proof::ForallApply(_, binds, patterns, lemma_qid, args) => {
                let env = exp_sourcifier.env();
                let module_env = env.get_module(lemma_qid.module_id);
                let lemma = module_env.get_lemma(lemma_qid.id);
                let lemma_name = lemma.name.display(env.symbol_pool());
                emit!(self.writer, "forall ");
                let mut comma = "";
                for (sym, ty) in binds {
                    let tctx = crate::ty::TypeDisplayContext::new(env);
                    emit!(
                        self.writer,
                        "{}{}: {}",
                        comma,
                        sym.display(env.symbol_pool()),
                        ty.display(&tctx),
                    );
                    comma = ", ";
                }
                if !patterns.is_empty() {
                    emit!(self.writer, " [");
                    let mut sep = "";
                    for group in patterns {
                        emit!(self.writer, "{}", sep);
                        let mut inner_sep = "";
                        for p in group {
                            emit!(self.writer, "{}", inner_sep);
                            exp_sourcifier.print_exp(Prio::General, false, p);
                            inner_sep = ", ";
                        }
                        sep = "; ";
                    }
                    emit!(self.writer, "]");
                }
                emit!(self.writer, " apply {}(", lemma_name);
                comma = "";
                for arg in args {
                    emit!(self.writer, "{}", comma);
                    exp_sourcifier.print_exp(Prio::General, false, arg);
                    comma = ", ";
                }
                emitln!(self.writer, ");");
            },
            Proof::Calc(_, steps) => {
                emit!(self.writer, "calc(");
                for (lhs, op, _rhs) in steps {
                    exp_sourcifier.print_exp(Prio::General, false, lhs);
                    let op_str = match op {
                        crate::ast::Operation::Eq => "==",
                        crate::ast::Operation::Neq => "!=",
                        crate::ast::Operation::Lt => "<",
                        crate::ast::Operation::Gt => ">",
                        crate::ast::Operation::Le => "<=",
                        crate::ast::Operation::Ge => ">=",
                        _ => "==",
                    };
                    emit!(self.writer, " {} ", op_str);
                }
                // Print the last rhs
                if let Some((_, _, rhs)) = steps.last() {
                    exp_sourcifier.print_exp(Prio::General, false, rhs);
                }
                emitln!(self.writer, ");");
            },
            Proof::Post(_, inner) => {
                emit!(self.writer, "post ");
                self.print_proof_body(inner, exp_sourcifier);
            },
            Proof::Split(_, exp) => {
                emit!(self.writer, "split ");
                exp_sourcifier.print_exp(Prio::General, false, exp);
                emitln!(self.writer, ";");
            },
        }
    }
}

//
// ========================================================================================
//

/// A type which allows to convert expressions into Move source.
pub struct ExpSourcifier<'a> {
    parent: &'a Sourcifier<'a>,
    type_display_context: TypeDisplayContext<'a>,
    temp_names: BTreeMap<TempIndex, Symbol>,
    // A mapping from node-id if `loop` to associated label, if one is needed.
    loop_labels: BTreeMap<NodeId, Symbol>,
    // A mapping from node-id if `break` or `continue` to associated label, if one is needed.
    cont_labels: BTreeMap<NodeId, Symbol>,
    // Number of return values for the function (used for result printing)
    result_count: usize,
    // Whether we are printing spec expressions (affects number suffix printing)
    for_spec: bool,
    amend: bool,
    // Shared counter for let-binding names, ensuring nested extractions don't clash.
    let_counter: Cell<usize>,
}

/// Helper type to split blocks and sequences into vector of items
enum LetOrStm {
    Let(Pattern, Option<Exp>),
    Stm(Exp),
}

/// A priority we assign to expressions for parenthesizing. This
/// is aligned with `syntax::get_precedence` in the parser and
/// need to stay like this.
///
/// Notice no `enum` type used here since there is the need
/// to do `priority + 1`.
type Priority = usize;
#[allow(non_snake_case, non_upper_case_globals)]
mod Prio {
    pub const General: usize = 1;
    #[allow(unused)] // TODO: specs
    pub const LogicalRelations: usize = 2;
    pub const LogicalOr: usize = 3;
    pub const LogicalAnd: usize = 4;
    pub const Relations: usize = 5;
    #[allow(unused)] // TODO: specs
    pub const Range: usize = 6;
    pub const BitOr: usize = 7;
    pub const BitXor: usize = 8;
    pub const BitAnd: usize = 9;
    pub const BitShift: usize = 10;
    pub const Additions: usize = 11;
    pub const Multiplications: usize = 12;
    pub const Prefix: usize = 13;
    pub const Postfix: usize = 14;
}

impl<'a> ExpSourcifier<'a> {
    /// Base names for let-bound variables.
    const LET_BIND_NAMES: &'static [&'static str] = &["a", "b", "c", "d", "e", "f"];
    /// Base names for lambda/closure parameters.
    const PARAM_NAMES: &'static [&'static str] = &["x", "y", "z", "w", "v", "u"];

    /// Creates a sourcifier for the given expression in the context of the function
    pub fn for_fun(
        parent: &'a Sourcifier<'a>,
        fun_env: &'a FunctionEnv,
        tctx: TypeDisplayContext<'a>,
        exp: &Exp,
        amend: bool,
    ) -> Self {
        let temp_names = (0..fun_env.get_parameter_count())
            .map(|i| (i, fun_env.get_local_name(i)))
            .collect();
        Self::new(
            parent,
            tctx,
            temp_names,
            exp,
            fun_env.get_return_count(),
            false, // not in spec context
            amend,
        )
    }

    /// Creates a sourcifier for spec expressions (no function body context needed)
    pub fn for_spec(parent: &'a Sourcifier<'a>, tctx: TypeDisplayContext<'a>, amend: bool) -> Self {
        Self {
            parent,
            type_display_context: tctx,
            temp_names: BTreeMap::new(),
            loop_labels: BTreeMap::new(),
            cont_labels: BTreeMap::new(),
            result_count: 0,
            for_spec: true,
            amend,
            let_counter: Cell::new(0),
        }
    }

    /// Creates a sourcifier for function-level spec expressions with parameter names
    pub fn for_fun_spec(
        parent: &'a Sourcifier<'a>,
        fun_env: &FunctionEnv,
        tctx: TypeDisplayContext<'a>,
        amend: bool,
    ) -> Self {
        let temp_names = (0..fun_env.get_parameter_count())
            .map(|i| (i, fun_env.get_local_name(i)))
            .collect();
        Self {
            parent,
            type_display_context: tctx,
            temp_names,
            loop_labels: BTreeMap::new(),
            cont_labels: BTreeMap::new(),
            result_count: fun_env.get_return_count(),
            for_spec: true,
            amend,
            let_counter: Cell::new(0),
        }
    }

    /// Creates a sourcifier for inline spec blocks, preserving temp_names from parent
    fn for_inline_spec(&self) -> Self {
        Self {
            parent: self.parent,
            type_display_context: self.type_display_context.clone(),
            temp_names: self.temp_names.clone(),
            loop_labels: BTreeMap::new(),
            cont_labels: BTreeMap::new(),
            result_count: self.result_count,
            for_spec: true,
            amend: self.amend,
            let_counter: Cell::new(0),
        }
    }

    fn new(
        parent: &'a Sourcifier<'a>,
        type_display_context: TypeDisplayContext<'a>,
        temp_names: BTreeMap<TempIndex, Symbol>,
        for_exp: &Exp,
        result_count: usize,
        for_spec: bool,
        amend: bool,
    ) -> Self {
        let (_, cont_to_loop) = for_exp.compute_loop_bindings();
        let mut loop_labels = BTreeMap::new();
        let mut cont_labels = BTreeMap::new();
        for_exp.visit_pre_order(&mut |e| {
            if let ExpData::LoopCont(id, nest, _) = e {
                if *nest > 0 {
                    // A label is required since this refers to an outer block
                    let loop_id = cont_to_loop[id];
                    let loop_label_count = loop_labels.len();
                    let label = *loop_labels.entry(loop_id).or_insert_with(|| {
                        parent
                            .env()
                            .symbol_pool()
                            .make(&format!("l{}", loop_label_count))
                    });
                    assert!(
                        cont_labels.insert(*id, label).is_none(),
                        "unexpected duplicate node id in expression"
                    );
                }
            };
            true
        });
        Self {
            parent,
            type_display_context,
            temp_names,
            loop_labels,
            cont_labels,
            result_count,
            for_spec,
            amend,
            let_counter: Cell::new(0),
        }
    }

    fn print_exp(&self, context_prio: Priority, is_result: bool, exp: &Exp) {
        use ExpData::*;
        match exp.as_ref() {
            // Following forms are all atomic and do not require parenthesis
            Invalid(_) => emit!(self.wr(), "*invalid*"),
            Value(_, v) => {
                let ty = self.env().get_node_type(exp.node_id());
                self.parent.print_value(v, Some(&ty), self.for_spec);
            },
            LocalVar(_, name) => {
                emit!(self.wr(), "{}", self.sym(*name))
            },
            Temporary(_, idx) => {
                if let Some(name) = self.temp_names.get(idx) {
                    emit!(self.wr(), "{}", self.sym(*name))
                } else {
                    emit!(self.wr(), "_t{}", idx)
                }
            },
            // Following forms may require parenthesis
            Lambda(_, pat, body, capture_kind, spec_opt) => {
                self.parenthesize(context_prio, Prio::General, || {
                    if *capture_kind != LambdaCaptureKind::Default {
                        emit!(self.wr(), "{} ", capture_kind);
                    };
                    emit!(self.wr(), "|");
                    self.print_pat(pat, true, false);
                    emit!(self.wr(), "| ");
                    self.print_exp(Prio::General, true, body);
                    if let Some(spec) = spec_opt {
                        self.print_exp(Prio::General, true, spec);
                    }
                });
            },
            Block(..) | Sequence(..) => {
                let mut items = vec![];
                self.extract_items(&mut items, exp);
                match items.len() {
                    0 => {
                        emit!(self.wr(), "()")
                    },
                    1 if matches!(items.last().unwrap(), LetOrStm::Stm(..)) => {
                        let LetOrStm::Stm(stm) = items.pop().unwrap() else {
                            unreachable!()
                        };
                        self.print_exp(context_prio, is_result, &stm)
                    },
                    _ => {
                        // Block has atomic priority, no parenthesis needed
                        self.parent.print_block(|| {
                            for (pos, item) in items.iter().enumerate() {
                                let last = pos == items.len() - 1;
                                if pos > 0 {
                                    // Close previous statement
                                    emitln!(self.wr(), ";")
                                }
                                match item {
                                    LetOrStm::Stm(stm) => {
                                        if last &&
                                            (stm.is_unit_exp() ||
                                                is_result && matches!(stm.as_ref(), Return(_, e) if e.is_unit_exp())) {
                                            // We can omit it
                                        } else {
                                            self.print_exp(Prio::General, last && is_result, stm);
                                            if last {
                                                emitln!(self.wr())
                                            }
                                        }
                                    }
                                    LetOrStm::Let(pat, binding) => {
                                        emit!(self.wr(), "let ");
                                        self.print_pat(pat, false, !binding.as_ref().is_some_and(|exp| matches!(exp.as_ref(), ExpData::Call(_, Operation::Closure(..), _))));
                                        if let Some(exp) = binding {
                                            emit!(self.wr(), " = ");
                                            self.print_exp(Prio::General, matches!(exp.as_ref(), Block(..) | Sequence(..)), exp);
                                        }
                                        if last {
                                            if !is_result {
                                                emit!(self.wr(), ";")
                                            }
                                            emitln!(self.wr())
                                        }
                                    }
                                }
                            }
                        })
                    },
                }
            },
            IfElse(_, cond, if_exp, else_exp) => {
                // Special case: the if-else can be printed as an `assert!`
                if self.print_assert(context_prio, cond, if_exp, else_exp) {
                    return;
                }
                self.parenthesize(context_prio, Prio::General, || {
                    emit!(self.wr(), "if (");
                    self.print_exp(Prio::General, false, cond);
                    emit!(self.wr(), ") ");
                    if else_exp.is_unit_exp() {
                        // We can omit the `else`, however, if the
                        // inner expression is an `if` again, we
                        // wrap it in a block to avoid the dangling-else problem.
                        if matches!(if_exp.as_ref(), IfElse(..)) {
                            self.parent
                                .print_block(|| self.print_exp(Prio::General, is_result, if_exp))
                        } else {
                            self.print_exp(Prio::General, is_result, if_exp)
                        }
                    } else {
                        self.print_exp(Prio::General, is_result, if_exp);
                        emit!(self.wr(), " else ");
                        self.print_exp(Prio::General, is_result, else_exp);
                    }
                })
            },
            Match(_, discriminator, arms) => self.parenthesize(context_prio, Prio::General, || {
                emit!(self.wr(), "match (");
                self.print_exp(Prio::General, false, discriminator);
                emitln!(self.wr(), ") {");
                self.wr().indent();
                for arm in arms {
                    self.print_pat(&arm.pattern, false, true);
                    if let Some(exp) = &arm.condition {
                        emit!(self.wr(), " if ");
                        self.print_exp(Prio::General, false, exp);
                    }
                    emit!(self.wr(), " => ");
                    self.print_exp(Prio::General, is_result, &arm.body);
                    emitln!(self.wr(), ",");
                }
                self.wr().unindent();
                emit!(self.wr(), "}");
            }),
            Loop(id, body) => self.parenthesize(context_prio, Prio::General, || {
                if let Some(label) = self.loop_labels.get(id) {
                    emit!(self.wr(), "'{}: ", self.sym(*label));
                }
                let print_loop_body = |loop_body: &Exp| {
                    // Remove obsolete continue
                    match loop_body.as_ref() {
                        Sequence(id, stms)
                            if matches!(
                                stms.last().map(|e| e.as_ref()),
                                Some(LoopCont(_, 0, true))
                            ) =>
                        {
                            self.print_exp(
                                Prio::General,
                                false,
                                &Sequence(*id, stms.iter().take(stms.len() - 1).cloned().collect())
                                    .into_exp(),
                            )
                        },
                        _ => self.print_exp(Prio::General, false, loop_body),
                    }
                };
                // Extract spec blocks from a while condition. Returns (actual_cond, spec_blocks).
                // The compiler puts loop invariants into the condition as:
                //   Sequence([SpecBlock(...), ..., actual_cond])
                let extract_spec_blocks = |cond: &Exp| -> (Exp, Vec<Exp>) {
                    match cond.as_ref() {
                        Sequence(_, stms) if stms.len() >= 2 => {
                            let specs: Vec<Exp> = stms
                                .iter()
                                .filter(|e| matches!(e.as_ref(), SpecBlock(..)))
                                .cloned()
                                .collect();
                            if !specs.is_empty() {
                                let actual: Vec<Exp> = stms
                                    .iter()
                                    .filter(|e| !matches!(e.as_ref(), SpecBlock(..)))
                                    .cloned()
                                    .collect();
                                let actual_cond = if actual.len() == 1 {
                                    actual.into_iter().next().unwrap()
                                } else {
                                    Sequence(stms[0].as_ref().node_id(), actual).into_exp()
                                };
                                return (actual_cond, specs);
                            }
                            (cond.clone(), vec![])
                        },
                        _ => (cond.clone(), vec![]),
                    }
                };
                // Print spec blocks after the loop body
                let print_trailing_specs = |specs: &[Exp]| {
                    for spec_exp in specs {
                        emit!(self.wr(), " ");
                        self.print_exp(Prio::General, false, spec_exp);
                    }
                };
                let general_loop = || {
                    emit!(self.wr(), "loop ");
                    print_loop_body(body)
                };

                // Detect while loop
                match body.as_ref() {
                    // Pattern 1: loop { if (c) e else break }
                    IfElse(_, cond, if_true, if_false) if if_false.is_loop_cont(Some(0), false) => {
                        let (actual_cond, specs) = extract_spec_blocks(cond);
                        emit!(self.wr(), "while (");
                        self.print_exp(Prio::General, false, &actual_cond);
                        emit!(self.wr(), ") ");
                        print_loop_body(if_true);
                        print_trailing_specs(&specs);
                    },
                    // Pattern 2: loop { if (c) break else e }
                    IfElse(_, cond, if_true, if_false) if if_true.is_loop_cont(Some(0), false) => {
                        let (actual_cond, specs) = extract_spec_blocks(cond);
                        emit!(self.wr(), "while (");
                        self.print_exp(Prio::General, false, &self.parent.builder.not(actual_cond));
                        emit!(self.wr(), ") ");
                        print_loop_body(if_false);
                        print_trailing_specs(&specs);
                    },
                    // Pattern 3: loop { if (c) e; break }
                    Sequence(_, stms)
                        if stms.len() == 2 && stms[1].is_loop_cont(Some(0), false) =>
                    {
                        match stms[0].as_ref() {
                            IfElse(_, cond, if_true, if_false) if if_false.is_unit_exp() => {
                                let (actual_cond, specs) = extract_spec_blocks(cond);
                                emit!(self.wr(), "while (");
                                self.print_exp(Prio::General, false, &actual_cond);
                                emit!(self.wr(), ") ");
                                print_loop_body(if_true);
                                print_trailing_specs(&specs);
                            },
                            _ => general_loop(),
                        }
                    },
                    // Pattern 4: loop { ( if (c) break )+; e }
                    Sequence(_, stms) if !stms.is_empty() => {
                        if let Some((cond, rest)) =
                            self.parent.builder.match_if_break_list(body.clone())
                        {
                            let not_cond = self.parent.builder.not(cond);
                            let (actual_cond, specs) = extract_spec_blocks(&not_cond);
                            emit!(self.wr(), "while (");
                            self.print_exp(Prio::General, false, &actual_cond);
                            emit!(self.wr(), ") ");
                            print_loop_body(&rest);
                            print_trailing_specs(&specs);
                        } else {
                            general_loop()
                        }
                    },
                    _ => general_loop(),
                }
            }),
            LoopCont(id, _, is_cont) => self.parenthesize(context_prio, Prio::General, || {
                if *is_cont {
                    emit!(self.wr(), "continue")
                } else {
                    emit!(self.wr(), "break")
                }
                if let Some(label) = self.cont_labels.get(id) {
                    emit!(self.wr(), " '{}", self.sym(*label))
                }
            }),
            Invoke(_, fun, args) => self.parenthesize(context_prio, Prio::Postfix, || {
                self.print_exp(Prio::Postfix, false, fun);
                self.print_exp_list("(", ")", args);
            }),
            Call(id, oper, args) => self.print_call(context_prio, *id, oper, args),
            Return(_, val) => self.parenthesize(context_prio, Prio::General, || {
                if !is_result {
                    emit!(self.wr(), "return ");
                }
                self.print_exp(Prio::General, false, val)
            }),
            Assign(_, pat, val) => self.parenthesize(context_prio, Prio::General, || {
                self.print_pat(pat, false, true);
                emit!(self.wr(), " = ");
                self.print_exp(
                    Prio::General,
                    matches!(exp.as_ref(), Block(..) | Sequence(..)),
                    val,
                );
            }),
            Mutate(_, lhs, rhs) => self.parenthesize(context_prio, Prio::General, || {
                if let Call(inner_id, Operation::Select(..) | Operation::SelectVariants(..), _) =
                    lhs.as_ref()
                {
                    let result_ty = self.env().get_node_type(*inner_id);
                    // While this looks weird, it is a must due to how `WriteRef` is translated into AST
                    if result_ty.is_mutable_reference() {
                        emit!(self.wr(), "*");
                    }
                    self.print_exp(Prio::General, false, lhs);
                    emit!(self.wr(), " = ");
                    self.print_exp(Prio::General, false, rhs)
                } else {
                    emit!(self.wr(), "*");
                    self.print_exp(Prio::Prefix, false, lhs);
                    emit!(self.wr(), " = ");
                    self.print_exp(Prio::General, false, rhs)
                }
            }),
            SpecBlock(_, spec) => {
                // Print inline spec block, preserving temp_names for parameter resolution
                emitln!(self.wr(), "spec {");
                self.wr().indent();
                let spec_sourcifier = self.for_inline_spec();
                for cond in &spec.conditions {
                    self.parent.print_condition(cond, &spec_sourcifier);
                }
                self.wr().unindent();
                emit!(self.wr(), "}")
            },
            Quant(_, kind, ranges, triggers, where_clause, body) => {
                self.parenthesize(context_prio, Prio::General, || {
                    self.print_quant_header(kind, ranges, triggers, where_clause);
                    self.print_exp(Prio::General, false, body);
                })
            },
        }
    }

    /// Print quantifier header (keyword, ranges, triggers, where clause, and trailing `: `).
    fn print_quant_header(
        &self,
        kind: &QuantKind,
        ranges: &[(Pattern, Exp)],
        triggers: &[Vec<Exp>],
        where_clause: &Option<Exp>,
    ) {
        let keyword = match kind {
            QuantKind::Forall => "forall",
            QuantKind::Exists => "exists",
            QuantKind::Choose => "choose",
            QuantKind::ChooseMin => "choose min",
        };
        emit!(self.wr(), "{} ", keyword);
        for (i, (pat, range)) in ranges.iter().enumerate() {
            if i > 0 {
                emit!(self.wr(), ", ");
            }
            self.print_pat(pat, false, true);
            if let ExpData::Call(range_id, Operation::TypeDomain, args) = range.as_ref() {
                if args.is_empty() {
                    let inst = self.env().get_node_instantiation(*range_id);
                    if let Some(ty) = inst.first() {
                        emit!(self.wr(), ": {}", self.ty(ty));
                    } else {
                        emit!(self.wr(), " in ");
                        self.print_exp(Prio::General, false, range);
                    }
                } else {
                    emit!(self.wr(), " in ");
                    self.print_exp(Prio::General, false, range);
                }
            } else if matches!(range.as_ref(), ExpData::Call(_, Operation::StateDomain, _)) {
                emit!(self.wr(), " in *");
            } else {
                emit!(self.wr(), " in ");
                self.print_exp(Prio::General, false, range);
            }
        }
        for trigger in triggers {
            emit!(self.wr(), " {");
            for (i, t) in trigger.iter().enumerate() {
                if i > 0 {
                    emit!(self.wr(), ", ");
                }
                self.print_exp(Prio::General, false, t);
            }
            emit!(self.wr(), "}");
        }
        if let Some(where_exp) = where_clause {
            emit!(self.wr(), " where ");
            self.print_exp(Prio::General, false, where_exp);
        }
        emit!(self.wr(), ": ");
    }

    /// Print a memory label with its name (from GlobalEnv) or fallback to numeric ID.
    fn print_memory_label(&self, label: &crate::ast::MemoryLabel) {
        if let Some(name) = self.env().get_memory_label_name(*label) {
            emit!(self.wr(), "{}", name.display(self.env().symbol_pool()));
        } else {
            emit!(self.wr(), "state_{}", label.as_usize());
        }
    }

    fn print_exp_list(&self, open: &str, close: &str, exps: &[Exp]) {
        self.parent.print_list(open, ", ", close, exps.iter(), |e| {
            self.print_exp(Prio::General, false, e)
        })
    }

    fn print_call(&self, context_prio: Priority, id: NodeId, oper: &Operation, args: &[Exp]) {
        match oper {
            Operation::MoveFunction(mid, fid) => {
                self.parenthesize(context_prio, Prio::Postfix, || {
                    let fun_env = self.env().get_module(*mid).into_function(*fid);
                    emit!(
                        self.wr(),
                        "{}{}",
                        self.parent
                            .module_qualifier(&self.type_display_context, *mid),
                        Sourcifier::amend_fun_name(
                            self.env(),
                            self.sym(fun_env.get_name()),
                            self.amend
                        )
                    );
                    self.print_node_inst(id);
                    self.print_exp_list("(", ")", args);
                })
            },
            Operation::Closure(mid, fid, mask) => {
                self.parenthesize(context_prio, Prio::Postfix, || {
                    // We need to generate `|t| f(captured, t)` to represent the closure.
                    // For that, we first create the closure expression, then call
                    // sourcifier again.
                    // Captured arguments are evaluated eagerly when the closure is
                    // created, so complex captured expressions (e.g. containing
                    // `old()`) must be hoisted into `let` bindings outside the lambda
                    // to avoid unsupported constructs inside the lambda body.
                    let loc = self.env().get_node_loc(id);
                    let ty = self.env().get_node_type(id);
                    let Type::Fun(arg_ty, res_ty, _) = ty else {
                        emit!(self.wr(), "<<wrongly typed closure expression>>");
                        return;
                    };
                    // Hoist non-trivial captured args into let bindings.
                    let mut let_bindings: Vec<(Pattern, Exp)> = vec![];
                    let captured_args: Vec<Exp> = args
                        .iter()
                        .map(|arg| {
                            if matches!(
                                arg.as_ref(),
                                ExpData::LocalVar(..) | ExpData::Value(..) | ExpData::Temporary(..)
                            ) {
                                arg.clone()
                            } else {
                                let (pat, var_exp) = self.let_bind_exp(arg, args);
                                let_bindings.push((pat, arg.clone()));
                                var_exp
                            }
                        })
                        .collect();
                    let mut lambda_params = vec![];
                    let mut lambda_param_tys = vec![];
                    let mut lambda_param_exps = vec![];
                    for (i, ty) in arg_ty.flatten().into_iter().enumerate() {
                        let base = Self::PARAM_NAMES[i % Self::PARAM_NAMES.len()];
                        // Include already-chosen lambda params in the scope to avoid
                        // duplicate names when there are more params than PARAM_NAMES.
                        let scope: Vec<Exp> = args
                            .iter()
                            .cloned()
                            .chain(lambda_param_exps.iter().cloned())
                            .collect();
                        let name = self.new_unique_name(&scope, base);
                        let local_id = self.env().new_node(loc.clone(), ty.clone());
                        lambda_params.push(Pattern::Var(local_id, name));
                        lambda_param_exps.push(ExpData::LocalVar(local_id, name).into_exp());
                        lambda_param_tys.push(ty);
                    }
                    let Some(all_args) = mask.compose(captured_args, lambda_param_exps) else {
                        emit!(self.wr(), "<<inconsistent closure mask>>");
                        return;
                    };
                    let new_node_id = self.env().new_node(loc.clone(), res_ty.as_ref().clone());
                    let call_exp =
                        ExpData::Call(new_node_id, Operation::MoveFunction(*mid, *fid), all_args)
                            .into_exp();
                    // Need to migrate the type instantiations to move function call.
                    if let Some(inst) = self.env().get_node_instantiation_opt(id) {
                        self.env().set_node_instantiation(new_node_id, inst);
                    }
                    let lambda_pat = if lambda_params.len() != 1 {
                        Pattern::Tuple(
                            self.env()
                                .new_node(loc.clone(), Type::tuple(lambda_param_tys)),
                            lambda_params,
                        )
                    } else {
                        lambda_params.pop().unwrap()
                    };
                    let lambda_exp =
                        ExpData::Lambda(id, lambda_pat, call_exp, LambdaCaptureKind::Default, None)
                            .into_exp();
                    // Wrap in let bindings for hoisted captured args.
                    let result_exp =
                        let_bindings
                            .into_iter()
                            .rev()
                            .fold(lambda_exp, |body, (pat, binding)| {
                                let block_id = self
                                    .env()
                                    .new_node(loc.clone(), self.env().get_node_type(id));
                                ExpData::Block(block_id, pat, Some(binding), body).into_exp()
                            });
                    self.print_exp(context_prio, false, &result_exp)
                })
            },
            Operation::Pack(mid, sid, variant) => {
                self.parenthesize(context_prio, Prio::Postfix, || {
                    let qid = mid.qualified_inst(*sid, self.env().get_node_instantiation(id));
                    self.print_constructor(&qid, variant, args.iter(), |e| {
                        self.print_exp(Prio::General, false, e)
                    })
                })
            },
            Operation::Tuple => {
                if args.len() == 1 {
                    // Single-element tuple - no parens needed
                    self.print_exp(Prio::General, false, &args[0])
                } else {
                    self.print_exp_list("(", ")", args)
                }
            },
            Operation::Select(mid, sid, fid) => {
                let struct_env = self.env().get_module(*mid).into_struct(*sid);
                // Check if this is ghost memory access: global<Ghost$X>(@addr).v -> X
                if let Some(spec_var_qid) = struct_env.get_ghost_memory_spec_var() {
                    let module_env = self.env().get_module(spec_var_qid.module_id);
                    let spec_var = module_env.get_spec_var(spec_var_qid.id);
                    emit!(self.wr(), "{}", self.sym(spec_var.name));
                    return;
                }
                let field_env = struct_env.get_field(*fid);
                let result_ty = self.env().get_node_type(id);
                // In spec contexts (e.g., struct invariants), Select may have no receiver
                if args.is_empty() {
                    // Just print the field name
                    emit!(self.wr(), "{}", self.sym(field_env.get_name()))
                } else {
                    // This is to accommodate nested `&` and `&mut`
                    let given_prio = if result_ty.is_reference() {
                        Prio::Prefix
                    } else {
                        Prio::Postfix
                    };
                    self.parenthesize(context_prio, given_prio, || {
                        if result_ty.is_immutable_reference() {
                            emit!(self.wr(), "&");
                        } else if result_ty.is_mutable_reference() {
                            emit!(self.wr(), "&mut ");
                        }
                        self.print_exp(Prio::Postfix, false, &args[0]);
                        emit!(self.wr(), ".{}", self.sym(field_env.get_name()))
                    })
                }
            },
            Operation::SelectVariants(mid, sid, fids) => {
                let struct_env = self.env().get_module(*mid).into_struct(*sid);
                let fid = fids[0];
                let field_env = struct_env.get_field(fid);
                let field_name = field_env.get_name();
                let result_ty = self.env().get_node_type(id);
                // Check if the field name is ambiguous (different types across variants),
                // requiring variant qualification in the output.
                let needs_variant_qualification = struct_env.has_variants() && {
                    let types: Vec<_> = struct_env
                        .get_fields()
                        .filter(|f| f.get_name() == field_name)
                        .map(|f| f.get_type())
                        .collect();
                    types.len() > 1 && types.windows(2).any(|w| w[0] != w[1])
                };
                let variant_prefix = if needs_variant_qualification {
                    field_env
                        .get_variant()
                        .map(|v| format!("{}.", self.sym(v)))
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                // In spec contexts, SelectVariants may have no receiver
                if args.is_empty() {
                    emit!(self.wr(), "{}{}", variant_prefix, self.sym(field_name))
                } else {
                    let given_prio = if result_ty.is_reference() {
                        Prio::Prefix
                    } else {
                        Prio::Postfix
                    };
                    self.parenthesize(context_prio, given_prio, || {
                        if result_ty.is_immutable_reference() {
                            emit!(self.wr(), "&");
                        } else if result_ty.is_mutable_reference() {
                            emit!(self.wr(), "&mut ");
                        }
                        self.print_exp(Prio::Postfix, false, &args[0]);
                        emit!(self.wr(), ".{}{}", variant_prefix, self.sym(field_name))
                    })
                }
            },
            Operation::TestVariants(_, _, variants) => {
                self.parenthesize(context_prio, Prio::General, || {
                    self.print_exp(Prio::General, false, &args[0]);
                    emit!(self.wr(), " is ");
                    self.parent.print_list("", " | ", "", variants.iter(), |v| {
                        emit!(self.wr(), "{}", self.sym(*v))
                    })
                })
            },
            Operation::Add => self.print_bin(context_prio, Prio::Additions, "+", args),
            Operation::Sub => self.print_bin(context_prio, Prio::Additions, "-", args),
            Operation::Mul => self.print_bin(context_prio, Prio::Multiplications, "*", args),
            Operation::Mod => self.print_bin(context_prio, Prio::Multiplications, "%", args),
            Operation::Div => self.print_bin(context_prio, Prio::Multiplications, "/", args),
            Operation::BitOr => self.print_bin(context_prio, Prio::BitOr, "|", args),
            Operation::BitAnd => self.print_bin(context_prio, Prio::BitAnd, "&", args),
            Operation::Xor => self.print_bin(context_prio, Prio::BitXor, "^", args),
            Operation::Shl => self.print_bin(context_prio, Prio::BitShift, "<<", args),
            Operation::Shr => self.print_bin(context_prio, Prio::BitShift, ">>", args),
            Operation::And => {
                self.print_bin_with_label_hoisting(context_prio, Prio::LogicalAnd, "&&", args)
            },
            Operation::Or => {
                self.print_bin_with_label_hoisting(context_prio, Prio::LogicalOr, "||", args)
            },
            Operation::Eq => {
                self.print_bin_with_label_hoisting(context_prio, Prio::Relations, "==", args)
            },
            Operation::Neq => {
                self.print_bin_with_label_hoisting(context_prio, Prio::Relations, "!=", args)
            },
            Operation::Lt => {
                self.print_bin_with_label_hoisting(context_prio, Prio::Relations, "<", args)
            },
            Operation::Gt => {
                self.print_bin_with_label_hoisting(context_prio, Prio::Relations, ">", args)
            },
            Operation::Le => {
                self.print_bin_with_label_hoisting(context_prio, Prio::Relations, "<=", args)
            },
            Operation::Ge => {
                self.print_bin_with_label_hoisting(context_prio, Prio::Relations, ">=", args)
            },
            Operation::Copy => self.parenthesize(context_prio, Prio::General, || {
                emit!(self.wr(), "copy ");
                self.print_exp(Prio::General, false, &args[0])
            }),
            Operation::Move => self.parenthesize(context_prio, Prio::General, || {
                emit!(self.wr(), "move ");
                self.print_exp(Prio::General, false, &args[0])
            }),
            Operation::Abort(kind) => {
                self.parenthesize(context_prio, Prio::General, || match kind {
                    AbortKind::Code => {
                        emit!(self.wr(), "abort ");
                        self.print_exp(Prio::General, false, &args[0])
                    },
                    AbortKind::Message => {
                        if Self::is_unspecified_abort_code(&args[0]) {
                            emit!(self.wr(), "abort ");
                            self.print_exp(Prio::General, false, &args[1]);
                        } else {
                            emit!(
                                self.wr(),
                                "/* unsupported abort with explicit code and message */"
                            );
                        }
                    },
                })
            },
            Operation::Freeze(explicit) => {
                if *explicit {
                    self.print_exp_list("freeze(", ")", &args[0..1]);
                } else {
                    // Implicit freeze - just print the inner expression without comment
                    self.print_exp(context_prio, false, &args[0]);
                }
            },
            Operation::Not => {
                if has_hoistable_label(&args[0]) {
                    self.parenthesize(context_prio, Prio::General, || {
                        self.print_hoisted_label_prefix(&args[0]);
                        emit!(self.wr(), "!");
                        self.print_exp_without_label(Prio::Prefix, &args[0])
                    })
                } else {
                    self.parenthesize(context_prio, Prio::Prefix, || {
                        emit!(self.wr(), "!");
                        self.print_exp(Prio::Prefix, false, &args[0])
                    })
                }
            },
            Operation::Negate => self.parenthesize(context_prio, Prio::Prefix, || {
                emit!(self.wr(), "-");
                self.print_exp(Prio::Prefix, false, &args[0])
            }),
            Operation::Cast => self.parenthesize(context_prio, Prio::General, || {
                // We take `as` as a postfix operator, to enforce a parenthesis around LHS as needed
                self.print_exp(Prio::Postfix, false, &args[0]);
                let ty = self.env().get_node_type(id);
                emit!(self.wr(), " as {}", self.ty(&ty))
            }),
            Operation::Exists(memory_label) => {
                if let Some(label) = memory_label {
                    if self
                        .parent
                        .enclosing_state_label
                        .get()
                        .is_some_and(|ctx| ctx == *label)
                    {
                        // Suppress — matches enclosing context
                        self.parenthesize(context_prio, Prio::Postfix, || {
                            emit!(self.wr(), "exists");
                            self.print_node_inst(id);
                            self.print_exp_list("(", ")", args)
                        })
                    } else {
                        self.parenthesize(context_prio, Prio::General, || {
                            self.print_memory_label(label);
                            emit!(self.wr(), " |~ exists");
                            self.print_node_inst(id);
                            let prev = self.parent.enclosing_state_label.replace(Some(*label));
                            self.print_exp_list("(", ")", args);
                            self.parent.enclosing_state_label.set(prev);
                        })
                    }
                } else {
                    self.parenthesize(context_prio, Prio::Postfix, || {
                        emit!(self.wr(), "exists");
                        self.print_node_inst(id);
                        self.print_exp_list("(", ")", args)
                    })
                }
            },
            Operation::SpecPublish(range)
            | Operation::SpecRemove(range)
            | Operation::SpecUpdate(range) => {
                let has_range = range.pre.is_some() || range.post.is_some();
                // When the operation has a state label range, check if any argument
                // is state-dependent. If so, extract it into a let-binding so the
                // label doesn't incorrectly scope over it after round-trip.
                let needs_let = has_range && args.iter().any(|a| !is_state_neutral(a));
                if needs_let {
                    let mut let_bindings = vec![];
                    let new_args: Vec<Exp> = args
                        .iter()
                        .map(|arg| {
                            if is_state_neutral(arg) {
                                arg.clone()
                            } else {
                                let (pat, var_exp) = self.let_bind_exp(arg, args);
                                let_bindings.push((pat, arg.clone()));
                                var_exp
                            }
                        })
                        .collect();
                    // Rebuild the operation with substituted args, wrap in lets
                    let new_call = ExpData::Call(id, oper.clone(), new_args).into_exp();
                    let wrapped = self.wrap_in_lets(let_bindings, new_call);
                    self.print_exp(context_prio, true, &wrapped);
                } else {
                    // No let-extraction needed — print normally
                    let prio = if has_range {
                        Prio::General
                    } else {
                        Prio::Postfix
                    };
                    self.parenthesize(context_prio, prio, || {
                        match (range.pre, range.post) {
                            (None, Some(post)) => {
                                emit!(self.wr(), "..");
                                self.print_memory_label(&post);
                                emit!(self.wr(), " |~ ");
                            },
                            (Some(pre), None) => {
                                self.print_memory_label(&pre);
                                emit!(self.wr(), ".. |~ ");
                            },
                            (Some(pre), Some(post)) => {
                                self.print_memory_label(&pre);
                                emit!(self.wr(), "..");
                                self.print_memory_label(&post);
                                emit!(self.wr(), " |~ ");
                            },
                            (None, None) => {},
                        }
                        let name = match oper {
                            Operation::SpecPublish(_) => "publish",
                            Operation::SpecRemove(_) => "remove",
                            Operation::SpecUpdate(_) => "update",
                            _ => unreachable!(),
                        };
                        emit!(self.wr(), "{}", name);
                        self.print_node_inst(id);
                        self.print_exp_list("(", ")", args);
                    })
                }
            },
            Operation::BorrowGlobal(kind) => self.parenthesize(context_prio, Prio::Postfix, || {
                if *kind == ReferenceKind::Mutable {
                    emit!(self.wr(), BORROW_GLOBAL_MUT)
                } else {
                    emit!(self.wr(), BORROW_GLOBAL)
                }
                self.print_node_inst(id);
                self.print_exp_list("(", ")", args)
            }),
            Operation::MoveTo => self.parenthesize(context_prio, Prio::Postfix, || {
                emit!(self.wr(), "move_to");
                self.print_node_inst(id);
                self.print_exp_list("(", ")", args)
            }),
            Operation::MoveFrom => self.parenthesize(context_prio, Prio::Postfix, || {
                emit!(self.wr(), "move_from");
                self.print_node_inst(id);
                self.print_exp_list("(", ")", args)
            }),
            Operation::Borrow(kind) => self.parenthesize(context_prio, Prio::Prefix, || {
                if *kind == ReferenceKind::Mutable {
                    emit!(self.wr(), "&mut ")
                } else {
                    emit!(self.wr(), "&")
                }
                let inner_ty = self.env().get_node_type(args[0].node_id());
                let context_prio = if inner_ty.is_reference() {
                    Prio::Postfix
                } else {
                    Prio::Prefix
                };
                self.print_exp(context_prio, false, &args[0])
            }),
            Operation::Deref => self.parenthesize(context_prio, Prio::Prefix, || {
                emit!(self.wr(), "*");
                self.print_exp(Prio::Prefix, false, &args[0])
            }),
            Operation::Vector => self.parenthesize(context_prio, Prio::Postfix, || {
                emit!(self.wr(), "vector");
                self.print_exp_list("[", "]", args)
            }),

            // Spec-only operations
            Operation::Old => self.parenthesize(context_prio, Prio::Postfix, || {
                emit!(self.wr(), "old(");
                self.print_exp(Prio::General, false, &args[0]);
                emit!(self.wr(), ")")
            }),
            Operation::Global(memory_label) => {
                if let Some(label) = memory_label {
                    // Check if this label matches the enclosing context — if so,
                    // suppress it and render as unlabeled (Move-2 index syntax).
                    if self
                        .parent
                        .enclosing_state_label
                        .get()
                        .is_some_and(|ctx| ctx == *label)
                    {
                        self.parenthesize(context_prio, Prio::Postfix, || {
                            if let Some(inst) = self.env().get_node_instantiation_opt(id) {
                                if let Some(ty) = inst.first() {
                                    emit!(self.wr(), "{}", self.ty(ty));
                                }
                            }
                            emit!(self.wr(), "[");
                            // Set this label as context for children
                            let prev = self.parent.enclosing_state_label.replace(Some(*label));
                            self.print_exp(Prio::General, false, &args[0]);
                            self.parent.enclosing_state_label.set(prev);
                            emit!(self.wr(), "]")
                        })
                    } else {
                        self.parenthesize(context_prio, Prio::General, || {
                            self.print_memory_label(label);
                            emit!(self.wr(), " |~ global");
                            self.print_node_inst(id);
                            emit!(self.wr(), "(");
                            // Set this label as context for children
                            let prev = self.parent.enclosing_state_label.replace(Some(*label));
                            self.print_exp(Prio::General, false, &args[0]);
                            self.parent.enclosing_state_label.set(prev);
                            emit!(self.wr(), ")")
                        })
                    }
                } else {
                    // Print as Move-2 index syntax: Type[addr]
                    self.parenthesize(context_prio, Prio::Postfix, || {
                        if let Some(inst) = self.env().get_node_instantiation_opt(id) {
                            if let Some(ty) = inst.first() {
                                emit!(self.wr(), "{}", self.ty(ty));
                            }
                        }
                        emit!(self.wr(), "[");
                        self.print_exp(Prio::General, false, &args[0]);
                        emit!(self.wr(), "]")
                    })
                }
            },
            Operation::Implies => self.print_bin_with_label_hoisting(
                context_prio,
                Prio::LogicalRelations,
                "==>",
                args,
            ),
            Operation::Iff => self.print_bin_with_label_hoisting(
                context_prio,
                Prio::LogicalRelations,
                "<==>",
                args,
            ),
            Operation::Identical => self.print_bin(context_prio, Prio::Relations, "===", args),
            Operation::Range => self.parenthesize(context_prio, Prio::Range, || {
                self.print_exp(Prio::Range, false, &args[0]);
                emit!(self.wr(), "..");
                self.print_exp(Prio::Range + 1, false, &args[1])
            }),
            Operation::Index => self.parenthesize(context_prio, Prio::Postfix, || {
                let base_ty = self.env().get_node_type(args[0].node_id());
                if let Type::Tuple(_) = &base_ty {
                    // Tuple element selection: print `.N` instead of `[N]`
                    self.print_exp(Prio::Postfix, false, &args[0]);
                    if let ExpData::Value(_, Value::Number(n)) = args[1].as_ref() {
                        emit!(self.wr(), ".{}", n)
                    } else {
                        emit!(self.wr(), ".");
                        self.print_exp(Prio::Postfix, false, &args[1])
                    }
                } else {
                    self.print_exp(Prio::Postfix, false, &args[0]);
                    emit!(self.wr(), "[");
                    self.print_exp(Prio::General, false, &args[1]);
                    emit!(self.wr(), "]")
                }
            }),
            Operation::Slice => self.parenthesize(context_prio, Prio::Postfix, || {
                self.print_exp(Prio::Postfix, false, &args[0]);
                emit!(self.wr(), "[");
                self.print_exp(Prio::General, false, &args[1]);
                emit!(self.wr(), "..");
                self.print_exp(Prio::General, false, &args[2]);
                emit!(self.wr(), "]")
            }),
            Operation::Len => self.parenthesize(context_prio, Prio::Postfix, || {
                emit!(self.wr(), "len(");
                self.print_exp(Prio::General, false, &args[0]);
                emit!(self.wr(), ")")
            }),
            Operation::Result(idx) => {
                // If there's exactly one result, use "result". Otherwise use "result_1", "result_2", etc.
                if self.result_count == 1 && *idx == 0 {
                    emit!(self.wr(), "result")
                } else {
                    emit!(self.wr(), "result_{}", idx + 1)
                }
            },
            Operation::Trace(kind) => self.parenthesize(context_prio, Prio::Postfix, || {
                emit!(self.wr(), "TRACE");
                if let crate::ast::TraceKind::SubAuto = kind {
                    emit!(self.wr(), "_SUB")
                } else if let crate::ast::TraceKind::Auto = kind {
                    emit!(self.wr(), "_AUTO")
                }
                emit!(self.wr(), "(");
                self.print_exp(Prio::General, false, &args[0]);
                emit!(self.wr(), ")")
            }),
            Operation::SpecFunction(mid, sid, _) => {
                self.parenthesize(context_prio, Prio::Postfix, || {
                    let module_env = self.env().get_module(*mid);
                    let spec_fun = module_env.get_spec_fun(*sid);
                    // Strip the leading '$' from spec function names as it's not valid Move syntax
                    let name = spec_fun.name.display(self.env().symbol_pool()).to_string();
                    let name = name.strip_prefix('$').unwrap_or(&name);
                    emit!(
                        self.wr(),
                        "{}{}",
                        self.parent
                            .module_qualifier(&self.type_display_context, *mid),
                        name
                    );
                    self.print_node_inst(id);
                    self.print_exp_list("(", ")", args)
                })
            },
            Operation::UpdateVec => self.parenthesize(context_prio, Prio::Postfix, || {
                emit!(self.wr(), "update(");
                self.print_exp(Prio::General, false, &args[0]);
                emit!(self.wr(), ", ");
                self.print_exp(Prio::General, false, &args[1]);
                emit!(self.wr(), ", ");
                self.print_exp(Prio::General, false, &args[2]);
                emit!(self.wr(), ")")
            }),
            Operation::ConcatVec => self.parenthesize(context_prio, Prio::Postfix, || {
                emit!(self.wr(), "concat(");
                self.print_exp(Prio::General, false, &args[0]);
                emit!(self.wr(), ", ");
                self.print_exp(Prio::General, false, &args[1]);
                emit!(self.wr(), ")")
            }),
            Operation::IndexOfVec => self.parenthesize(context_prio, Prio::Postfix, || {
                emit!(self.wr(), "index_of(");
                self.print_exp(Prio::General, false, &args[0]);
                emit!(self.wr(), ", ");
                self.print_exp(Prio::General, false, &args[1]);
                emit!(self.wr(), ")")
            }),
            Operation::ContainsVec => self.parenthesize(context_prio, Prio::Postfix, || {
                emit!(self.wr(), "contains(");
                self.print_exp(Prio::General, false, &args[0]);
                emit!(self.wr(), ", ");
                self.print_exp(Prio::General, false, &args[1]);
                emit!(self.wr(), ")")
            }),
            Operation::InRangeVec => self.parenthesize(context_prio, Prio::Postfix, || {
                emit!(self.wr(), "in_range(");
                self.print_exp(Prio::General, false, &args[0]);
                emit!(self.wr(), ", ");
                self.print_exp(Prio::General, false, &args[1]);
                emit!(self.wr(), ")")
            }),
            Operation::InRangeRange => self.parenthesize(context_prio, Prio::Postfix, || {
                emit!(self.wr(), "in_range(");
                self.print_exp(Prio::General, false, &args[0]);
                emit!(self.wr(), ", ");
                self.print_exp(Prio::General, false, &args[1]);
                emit!(self.wr(), ")")
            }),
            Operation::RangeVec => self.parenthesize(context_prio, Prio::Postfix, || {
                self.print_exp(Prio::Postfix, false, &args[0]);
                emit!(self.wr(), ".range()")
            }),
            Operation::SingleVec => self.parenthesize(context_prio, Prio::Postfix, || {
                emit!(self.wr(), "vec(");
                self.print_exp(Prio::General, false, &args[0]);
                emit!(self.wr(), ")")
            }),
            Operation::EmptyVec => self.parenthesize(context_prio, Prio::Postfix, || {
                emit!(self.wr(), "vec");
                self.print_node_inst(id);
                emit!(self.wr(), "()")
            }),
            Operation::TypeValue => self.parenthesize(context_prio, Prio::Postfix, || {
                emit!(self.wr(), "type_info::type_of");
                self.print_node_inst(id);
                emit!(self.wr(), "()")
            }),
            Operation::TypeDomain => self.parenthesize(context_prio, Prio::Postfix, || {
                emit!(self.wr(), "domain");
                self.print_node_inst(id);
                emit!(self.wr(), "()")
            }),
            Operation::ResourceDomain => self.parenthesize(context_prio, Prio::Postfix, || {
                emit!(self.wr(), "resources");
                self.print_node_inst(id);
                emit!(self.wr(), "()")
            }),
            Operation::StateDomain => emit!(self.wr(), "*"),
            Operation::MaxU8 => emit!(self.wr(), "MAX_U8"),
            Operation::MaxU16 => emit!(self.wr(), "MAX_U16"),
            Operation::MaxU32 => emit!(self.wr(), "MAX_U32"),
            Operation::MaxU64 => emit!(self.wr(), "MAX_U64"),
            Operation::MaxU128 => emit!(self.wr(), "MAX_U128"),
            Operation::MaxU256 => emit!(self.wr(), "MAX_U256"),
            Operation::CanModify => self.parenthesize(context_prio, Prio::Postfix, || {
                emit!(self.wr(), "can_modify");
                self.print_node_inst(id);
                emit!(self.wr(), "(");
                self.print_exp(Prio::General, false, &args[0]);
                emit!(self.wr(), ")")
            }),
            Operation::UpdateField(mid, sid, fid) => {
                self.parenthesize(context_prio, Prio::Postfix, || {
                    let struct_env = self.env().get_module(*mid).into_struct(*sid);
                    let field_env = struct_env.get_field(*fid);
                    emit!(self.wr(), "update_field(");
                    self.print_exp(Prio::General, false, &args[0]);
                    emit!(self.wr(), ", {}, ", self.sym(field_env.get_name()));
                    self.print_exp(Prio::General, false, &args[1]);
                    emit!(self.wr(), ")")
                })
            },
            Operation::Bv2Int => self.parenthesize(context_prio, Prio::Postfix, || {
                emit!(self.wr(), "bv2int(");
                self.print_exp(Prio::General, false, &args[0]);
                emit!(self.wr(), ")")
            }),
            Operation::Int2Bv => self.parenthesize(context_prio, Prio::Postfix, || {
                emit!(self.wr(), "int2bv(");
                self.print_exp(Prio::General, false, &args[0]);
                emit!(self.wr(), ")")
            }),
            Operation::AbortFlag => emit!(self.wr(), "ABORTED"),
            Operation::AbortCode => emit!(self.wr(), "ABORT_CODE"),
            Operation::WellFormed => self.parenthesize(context_prio, Prio::Postfix, || {
                emit!(self.wr(), "WellFormed(");
                self.print_exp(Prio::General, false, &args[0]);
                emit!(self.wr(), ")")
            }),
            Operation::Behavior(kind, range) => {
                let kind_str = match kind {
                    crate::ast::BehaviorKind::AbortsOf => "aborts_of",
                    crate::ast::BehaviorKind::EnsuresOf => "ensures_of",
                    crate::ast::BehaviorKind::RequiresOf => "requires_of",
                    crate::ast::BehaviorKind::ResultOf => "result_of",
                };
                let has_labels = range.pre.is_some() || range.post.is_some();
                let print_behavior = |this: &Self| {
                    this.parenthesize(context_prio, Prio::Postfix, || {
                        emit!(this.wr(), "{}", kind_str);
                        emit!(this.wr(), "<");
                        this.print_behavior_target(&args[0]);
                        emit!(this.wr(), ">");
                        this.print_exp_list("(", ")", &args[1..]);
                    })
                };
                if has_labels {
                    // Check if all labels match enclosing state context — suppress if so
                    if let Some(ctx) = self.parent.enclosing_state_label.get() {
                        let pre_ok = range.pre.is_none() || range.pre == Some(ctx);
                        let post_ok = range.post.is_none() || range.post == Some(ctx);
                        if pre_ok && post_ok {
                            print_behavior(self);
                            return;
                        }
                    }
                    // aborts_of/requires_of only take a pre-state (single state form),
                    // ensures_of/result_of can have pre..post ranges.
                    let is_single_state = matches!(
                        kind,
                        crate::ast::BehaviorKind::AbortsOf | crate::ast::BehaviorKind::RequiresOf
                    ) && range.post.is_none();
                    self.parenthesize(context_prio, Prio::General, || {
                        if is_single_state {
                            // Single state: `label |~ behavior<f>(args)`
                            if let Some(pre) = &range.pre {
                                self.print_memory_label(pre);
                            }
                        } else {
                            // Range: `[pre]..[post] |~ behavior<f>(args)`
                            if let Some(pre) = &range.pre {
                                self.print_memory_label(pre);
                            }
                            emit!(self.wr(), "..");
                            if let Some(post) = &range.post {
                                self.print_memory_label(post);
                            }
                        }
                        emit!(self.wr(), " |~ ");
                        // Set enclosing label context so inner behavioral
                        // predicates with matching labels are suppressed.
                        let repr_label = match (range.pre, range.post) {
                            (Some(p), Some(q)) if p == q => Some(p),
                            (Some(p), None) => Some(p),
                            (None, Some(q)) => Some(q),
                            _ => None,
                        };
                        if let Some(label) = repr_label {
                            let prev = self.parent.enclosing_state_label.replace(Some(label));
                            print_behavior(self);
                            self.parent.enclosing_state_label.set(prev);
                        } else {
                            print_behavior(self);
                        }
                    })
                } else {
                    print_behavior(self);
                }
            },

            // Operations that are less commonly used in user-written specs
            Operation::BoxValue
            | Operation::UnboxValue
            | Operation::EmptyEventStore
            | Operation::ExtendEventStore
            | Operation::EventStoreIncludes
            | Operation::EventStoreIncludedIn
            | Operation::NoOp => {
                emit!(self.wr(), "/* internal spec operation {:?} */", oper)
            },
        }
    }

    fn print_assert(&self, context_prio: Priority, cond_: &Exp, then_: &Exp, else_: &Exp) -> bool {
        // Match the pattern `if (!cond) abort(code) else ()`
        let inner_cond = match cond_.as_ref() {
            ExpData::Call(_, Operation::Not, args) if args.len() == 1 => args[0].clone(),
            _ => return false,
        };
        let abort_code = match then_.as_ref() {
            ExpData::Call(_, Operation::Abort(_), args) if args.len() == 1 => args[0].clone(),
            _ => return false,
        };
        if !else_.is_unit_exp() {
            return false;
        }

        // All matched, print as `assert!(cond, code)`
        self.parenthesize(context_prio, Prio::General, || {
            emit!(self.wr(), "assert!(");
            self.print_exp(Prio::General, false, &inner_cond);
            emit!(self.wr(), ", ");
            self.print_exp(Prio::General, false, &abort_code);
            emit!(self.wr(), ")");
        });
        true
    }

    fn print_constructor<I>(
        &self,
        qid: &QualifiedInstId<StructId>,
        variant: &Option<Symbol>,
        items: impl Iterator<Item = I>,
        printer: impl Fn(I),
    ) {
        let struct_env = self.env().get_struct(qid.to_qualified_id());
        emit!(
            self.wr(),
            "{}{}",
            self.parent
                .module_qualifier(&self.type_display_context, struct_env.module_env.get_id()),
            self.sym(struct_env.get_name())
        );
        if let Some(v) = variant {
            emit!(self.wr(), "::{}", self.sym(*v))
        }
        self.print_inst(&qid.inst);
        let (open, close) = if struct_env
            .get_fields_optional_variant(*variant)
            .any(|f| f.is_positional())
        {
            ("(", ")")
        } else {
            ("{", "}")
        };
        self.parent.print_list(
            open,
            ", ",
            close,
            struct_env
                .get_fields_optional_variant(*variant)
                .zip(items)
                .filter(|(f, _)| !self.parent.is_dummy_field(f)),
            |(f, i)| {
                if !f.is_positional() {
                    emit!(self.wr(), "{}: ", self.sym(f.get_name()));
                }
                printer(i)
            },
        );
    }

    fn print_bin(&self, context_prio: Priority, prio: Priority, repr: &str, args: &[Exp]) {
        self.parenthesize(context_prio, prio, || {
            self.print_exp(prio, false, &args[0]);
            emit!(self.wr(), " {} ", repr);
            if args.len() > 1 {
                self.print_exp(prio + 1, false, &args[1])
            } else {
                emit!(self.wr(), "ERROR")
            }
        })
    }

    /// Print an expression at the top level of a spec condition, hoisting any
    /// state label prefix out if the entire expression has a single hoistable label.
    ///
    /// This is a readability normalization: in the AST, labels are tightly scoped
    /// around individual memory operations (e.g., `(S |~ global<R>(a)) == x`).
    /// Hoisting produces `S |~ global<R>(a) == x`, where the label provides context
    /// for the entire relation — a more natural reading for humans.
    fn print_exp_hoisting_label(&self, exp: &Exp) {
        if has_hoistable_label(exp) {
            self.print_hoisted_label_prefix(exp);
            self.print_exp_without_label(Prio::General, exp);
        } else {
            self.print_exp(Prio::General, false, exp);
        }
    }

    /// Print a binary operator, hoisting a state label out if possible.
    fn print_bin_with_label_hoisting(
        &self,
        context_prio: Priority,
        prio: Priority,
        repr: &str,
        args: &[Exp],
    ) {
        if !self.try_print_label_hoisted_bin(context_prio, prio, repr, args) {
            self.print_bin(context_prio, prio, repr, args)
        }
    }

    /// Try to hoist a state label out of a binary operation for readability.
    /// If exactly one operand has a hoistable label and the other has no state labels,
    /// renders `label |~ lhs op rhs` (with the label stripped from the labeled operand).
    /// Returns true if hoisting was performed.
    fn try_print_label_hoisted_bin(
        &self,
        context_prio: Priority,
        prio: Priority,
        repr: &str,
        args: &[Exp],
    ) -> bool {
        if args.len() != 2 {
            return false;
        }
        let lhs_labeled = has_hoistable_label(&args[0]);
        let rhs_labeled = has_hoistable_label(&args[1]);
        if lhs_labeled && is_state_neutral(&args[1]) {
            self.parenthesize(context_prio, Prio::General, || {
                self.print_hoisted_label_prefix(&args[0]);
                self.parenthesize(Prio::General, prio, || {
                    self.print_exp_without_label(prio, &args[0]);
                    emit!(self.wr(), " {} ", repr);
                    self.print_exp(prio + 1, false, &args[1]);
                });
            });
            true
        } else if rhs_labeled && is_state_neutral(&args[0]) {
            self.parenthesize(context_prio, Prio::General, || {
                self.print_hoisted_label_prefix(&args[1]);
                self.parenthesize(Prio::General, prio, || {
                    self.print_exp(prio, false, &args[0]);
                    emit!(self.wr(), " {} ", repr);
                    self.print_exp_without_label(prio + 1, &args[1]);
                });
            });
            true
        } else {
            false
        }
    }

    /// Print the label/range prefix (e.g., `S |~ `) extracted from a labeled expression.
    fn print_hoisted_label_prefix(&self, exp: &Exp) {
        match exp.as_ref() {
            ExpData::Call(_, Operation::Global(Some(label)), _)
            | ExpData::Call(_, Operation::Exists(Some(label)), _) => {
                self.print_memory_label(label);
                emit!(self.wr(), " |~ ");
            },
            ExpData::Call(_, Operation::Behavior(kind, range), _) => {
                let is_single_state = matches!(
                    kind,
                    crate::ast::BehaviorKind::AbortsOf | crate::ast::BehaviorKind::RequiresOf
                ) && range.post.is_none();
                if is_single_state {
                    if let Some(pre) = &range.pre {
                        self.print_memory_label(pre);
                    }
                } else {
                    if let Some(pre) = &range.pre {
                        self.print_memory_label(pre);
                    }
                    emit!(self.wr(), "..");
                    if let Some(post) = &range.post {
                        self.print_memory_label(post);
                    }
                }
                emit!(self.wr(), " |~ ");
            },
            // Look through Not
            ExpData::Call(_, Operation::Not, args) if args.len() == 1 => {
                self.print_hoisted_label_prefix(&args[0])
            },
            // Look through binary ops
            ExpData::Call(_, op, args)
                if args.len() == 2 && hoistable_bin_op_info(op).is_some() =>
            {
                if has_hoistable_label(&args[0]) {
                    self.print_hoisted_label_prefix(&args[0])
                } else {
                    self.print_hoisted_label_prefix(&args[1])
                }
            },
            // Look through simple quantifiers
            ExpData::Quant(_, _, _, _, _, body) => self.print_hoisted_label_prefix(body),
            _ => {},
        }
    }

    /// Print a labeled expression as if it had no label.
    fn print_exp_without_label(&self, prio: Priority, exp: &Exp) {
        match exp.as_ref() {
            ExpData::Call(id, Operation::Global(Some(label)), args) => {
                self.parenthesize(prio, Prio::Postfix, || {
                    emit!(self.wr(), "global");
                    self.print_node_inst(*id);
                    emit!(self.wr(), "(");
                    let prev = self.parent.enclosing_state_label.replace(Some(*label));
                    self.print_exp(Prio::General, false, &args[0]);
                    self.parent.enclosing_state_label.set(prev);
                    emit!(self.wr(), ")")
                })
            },
            ExpData::Call(id, Operation::Exists(Some(label)), args) => {
                self.parenthesize(prio, Prio::Postfix, || {
                    emit!(self.wr(), "exists");
                    self.print_node_inst(*id);
                    let prev = self.parent.enclosing_state_label.replace(Some(*label));
                    self.print_exp_list("(", ")", args);
                    self.parent.enclosing_state_label.set(prev);
                })
            },
            ExpData::Call(_id, Operation::Behavior(kind, _range), args) => {
                let kind_str = match kind {
                    crate::ast::BehaviorKind::AbortsOf => "aborts_of",
                    crate::ast::BehaviorKind::EnsuresOf => "ensures_of",
                    crate::ast::BehaviorKind::RequiresOf => "requires_of",
                    crate::ast::BehaviorKind::ResultOf => "result_of",
                };
                self.parenthesize(prio, Prio::Postfix, || {
                    emit!(self.wr(), "{}", kind_str);
                    emit!(self.wr(), "<");
                    self.print_behavior_target(&args[0]);
                    emit!(self.wr(), ">");
                    self.print_exp_list("(", ")", &args[1..]);
                })
            },
            // Look through Not: print `!` then the inner without label
            ExpData::Call(_, Operation::Not, args) if args.len() == 1 => {
                self.parenthesize(prio, Prio::Prefix, || {
                    emit!(self.wr(), "!");
                    self.print_exp_without_label(Prio::Prefix, &args[0])
                })
            },
            // Look through binary ops: strip label from the labeled side
            ExpData::Call(_, op, args)
                if args.len() == 2 && hoistable_bin_op_info(op).is_some() =>
            {
                let (op_prio, op_str) = hoistable_bin_op_info(op).unwrap();
                if has_hoistable_label(&args[0]) && is_state_neutral(&args[1]) {
                    self.parenthesize(prio, op_prio, || {
                        self.print_exp_without_label(op_prio, &args[0]);
                        emit!(self.wr(), " {} ", op_str);
                        self.print_exp(op_prio + 1, false, &args[1]);
                    })
                } else if has_hoistable_label(&args[1]) && is_state_neutral(&args[0]) {
                    self.parenthesize(prio, op_prio, || {
                        self.print_exp(op_prio, false, &args[0]);
                        emit!(self.wr(), " {} ", op_str);
                        self.print_exp_without_label(op_prio + 1, &args[1]);
                    })
                } else {
                    self.print_exp(prio, false, exp)
                }
            },
            // Look through simple quantifiers
            ExpData::Quant(_, kind, ranges, triggers, where_clause, body)
                if is_simple_quant(ranges, triggers, where_clause) =>
            {
                self.parenthesize(prio, Prio::General, || {
                    self.print_quant_header(kind, ranges, triggers, where_clause);
                    self.print_exp_without_label(Prio::General, body);
                })
            },
            _ => self.print_exp(prio, false, exp),
        }
    }

    fn print_node_inst(&self, id: NodeId) {
        if let Some(inst) = self.env().get_node_instantiation_opt(id) {
            self.print_inst(&inst)
        }
    }

    fn print_inst(&self, inst: &[Type]) {
        if !inst.is_empty() {
            self.parent.print_list("<", ", ", ">", inst.iter(), |ty| {
                emit!(self.wr(), "{}", self.ty(ty))
            });
        }
    }

    /// Optionally print parenthesis if printing expression of given_prio requires it in
    /// the context.
    fn parenthesize(&self, context_prio: Priority, given_prio: Priority, emit_exp: impl FnOnce()) {
        let need = context_prio > given_prio;
        if need {
            emit!(self.wr(), "(")
        }
        emit_exp();
        if need {
            emit!(self.wr(), ")")
        }
    }

    /// Print the target of a behavior predicate. If it's a simple function reference
    /// (a closure with no captures, or a lambda that just forwards to a function),
    /// print the function name directly. Otherwise, print the expression as-is.
    /// Returns true if the expression can be printed directly inside `<...>` of
    /// a behavior predicate (i.e. it produces a simple name).
    fn print_behavior_target(&self, exp: &Exp) {
        use ExpData::*;

        // Check for Closure with no captured arguments - just a function reference
        if let Call(node_id, Operation::Closure(mid, fid, mask), args) = exp.as_ref() {
            // If no captured arguments (all bits in mask are for lambda params),
            // we can print just the function name.
            //
            // Notice that if we did not special case this here, the generic closure
            // sourcifier would generate `|arg| f(arg)`.
            if args.is_empty() && mask.captured_count() == 0 {
                let fun_env = self.env().get_module(*mid).into_function(*fid);
                emit!(
                    self.wr(),
                    "{}{}",
                    self.parent
                        .module_qualifier(&self.type_display_context, *mid),
                    self.sym(fun_env.get_name())
                );
                // Print type arguments if any
                if let Some(inst) = self.env().get_node_instantiation_opt(*node_id) {
                    if !inst.is_empty() {
                        self.parent.print_list("<", ", ", ">", inst.iter(), |ty| {
                            emit!(self.wr(), "{}", self.ty(ty))
                        });
                    }
                }
                return;
            }
        }

        // Fall back to printing the expression normally
        self.print_exp(Prio::General, false, exp);
    }

    fn print_pat(&self, pat: &Pattern, no_parenthesize: bool, no_type: bool) {
        match pat {
            Pattern::Var(node_id, name) => {
                emit!(self.wr(), "{}", self.sym(*name));
                let ty = self.env().get_node_type(*node_id);
                if let Type::Fun(_, _, ability) = ty {
                    if !ability.is_empty() && !no_type {
                        emit!(self.wr(), ": {}", self.ty(&ty));
                    }
                }
            },
            Pattern::Wildcard(_) => emit!(self.wr(), "_"),
            Pattern::Tuple(_, elems) => {
                if elems.len() == 1 && !no_parenthesize {
                    // Single-element tuple - just print the inner pattern
                    self.print_pat(&elems[0], no_parenthesize, no_type)
                } else {
                    let (start, end) = if !no_parenthesize {
                        ("(", ")")
                    } else {
                        // When printing the args of a function value, we cannot parenthesize the tuple
                        ("", "")
                    };
                    self.parent.print_list(start, ",", end, elems.iter(), |p| {
                        self.print_pat(p, no_parenthesize, no_type)
                    })
                }
            },
            Pattern::Struct(_, qid, variant, elems) => {
                self.print_constructor(qid, variant, elems.iter(), |p| {
                    self.print_pat(p, no_parenthesize, no_type)
                })
            },
            Pattern::LiteralValue(node_id, val) => {
                let ty = self.env().get_node_type(*node_id);
                let ty = ty.skip_reference();
                self.parent.print_value(val, Some(ty), self.for_spec);
            },
            Pattern::Range(node_id, lo, hi, inclusive) => {
                let ty = self.env().get_node_type(*node_id);
                let ty = ty.skip_reference();
                if let Some(l) = lo {
                    self.parent.print_value(l, Some(ty), self.for_spec);
                }
                if *inclusive {
                    emit!(self.wr(), "..=");
                } else {
                    emit!(self.wr(), "..");
                }
                if let Some(h) = hi {
                    self.parent.print_value(h, Some(ty), self.for_spec);
                }
            },
            Pattern::Error(_) => emit!(self.wr(), "*error*"),
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn extract_items(&self, items: &mut Vec<LetOrStm>, exp: &Exp) {
        match exp.as_ref() {
            ExpData::Block(_, pat, binding, body) => {
                // It is cheap to clone expressions, so do not bother with
                // taking references here.
                items.push(LetOrStm::Let(pat.clone(), binding.clone()));
                self.extract_items(items, body)
            },
            ExpData::Sequence(_, stms) => {
                if !stms.is_empty() {
                    items.extend(stms.iter().take(stms.len() - 1).cloned().map(LetOrStm::Stm));
                    self.extract_items(items, stms.last().unwrap())
                }
            },
            _ => items.push(LetOrStm::Stm(exp.clone())),
        }
    }

    fn ty(&self, ty: &Type) -> String {
        ty.display(&self.type_display_context).to_string()
    }

    fn sym(&self, sym: Symbol) -> String {
        self.parent.sym(sym)
    }

    fn wr(&self) -> &CodeWriter {
        &self.parent.writer
    }

    fn env(&self) -> &GlobalEnv {
        self.parent.env()
    }

    /// Creates a new name which has no clashes with free names in given expressions.
    /// Note: if this turns to be too inefficient for sourcifier, we can use
    /// a ref-call counter instead.
    fn new_unique_name(&self, for_scope: &[Exp], base_name: &str) -> Symbol {
        let mut free_vars = BTreeSet::new();
        let spool = self.env().symbol_pool();
        for e in for_scope {
            free_vars.append(&mut e.free_vars());
            // Collect bound variables and Temporary display names so we don't
            // shadow them when choosing fresh names.
            e.as_ref().visit_pre_order(&mut |ed| {
                match ed {
                    ExpData::Block(_, pat, _, _) | ExpData::Lambda(_, pat, _, _, _) => {
                        pat.vars().iter().for_each(|(_, sym)| {
                            free_vars.insert(*sym);
                        });
                    },
                    ExpData::Temporary(_, idx) => {
                        if let Some(name) = self.temp_names.get(idx) {
                            free_vars.insert(*name);
                        }
                    },
                    _ => {},
                }
                true
            });
        }
        for i in 0..256 {
            let name = if i > 0 {
                spool.make(&format!("{}_{}", base_name, i))
            } else {
                spool.make(base_name)
            };
            if !free_vars.contains(&name) {
                return name;
            }
        }
        panic!("too many fruitless attempts to generate unique name")
    }

    /// Extract `exp` into a let-binding with a fresh variable name.
    /// Returns `(pattern, var_exp)` — the caller accumulates let-bindings and wraps
    /// in a flat Block for printing. Uses the shared `let_counter` to ensure nested
    /// extractions produce distinct names.
    fn let_bind_exp(&self, exp: &Exp, scope: &[Exp]) -> (Pattern, Exp) {
        let count = self.let_counter.get();
        let base = Self::LET_BIND_NAMES[count % Self::LET_BIND_NAMES.len()];
        self.let_counter.set(count + 1);
        let name = self.new_unique_name(scope, base);
        let ty = self.env().get_node_type(exp.node_id());
        let loc = self.env().get_node_loc(exp.node_id());
        let local_id = self.env().new_node(loc, ty);
        let pat = Pattern::Var(local_id, name);
        let var_exp = ExpData::LocalVar(local_id, name).into_exp();
        (pat, var_exp)
    }

    /// Wrap `body` in a flat block with the given let-bindings.
    /// Produces `{ let x = e1; let y = e2; body }` as a single Block chain.
    fn wrap_in_lets(&self, let_bindings: Vec<(Pattern, Exp)>, body: Exp) -> Exp {
        let_bindings
            .into_iter()
            .rev()
            .fold(body, |acc, (pat, binding)| {
                let block_id = self.env().new_node(
                    self.env().get_node_loc(acc.node_id()),
                    self.env().get_node_type(acc.node_id()),
                );
                ExpData::Block(block_id, pat, Some(binding), acc).into_exp()
            })
    }

    fn is_unspecified_abort_code(exp: &Exp) -> bool {
        matches!(exp.as_ref(), ExpData::Value(_, Value::Number(n)) if *n == UNSPECIFIED_ABORT_CODE.into())
    }
}
