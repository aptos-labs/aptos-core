// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ast::{
        AccessSpecifierKind, AddressSpecifier, Exp, ExpData, LambdaCaptureKind, Operation, Pattern,
        ResourceSpecifier, TempIndex, Value,
    },
    code_writer::CodeWriter,
    emit, emitln,
    exp_builder::ExpBuilder,
    model::{
        FieldEnv, FunId, FunctionEnv, GlobalEnv, ModuleId, NodeId, Parameter, QualifiedId,
        QualifiedInstId, StructId, TypeParameter, Visibility,
    },
    symbol::Symbol,
    ty::{PrimitiveType, ReferenceKind, Type, TypeDisplayContext},
};
use itertools::Itertools;
use move_core_types::ability::AbilitySet;
use std::collections::{BTreeMap, BTreeSet};
//
// ========================================================================================
//

/// A type which allows to convert function, struct, and module definitions into
/// Move source.
pub struct Sourcifier<'a> {
    builder: ExpBuilder<'a>,
    writer: CodeWriter,
}

impl<'a> Sourcifier<'a> {
    /// Creates a new sourcifier.
    pub fn new(env: &'a GlobalEnv) -> Self {
        Self {
            builder: ExpBuilder::new(env),
            // Location not used, but required by the constructor
            writer: CodeWriter::new(env.unknown_loc()),
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
        for use_ in module_env.get_used_modules(false) {
            emitln!(
                self.writer,
                "use {};",
                self.env().get_module(use_).get_full_name_str()
            )
        }
        for friend in module_env.get_friend_modules() {
            emitln!(
                self.writer,
                "friend {};",
                self.env().get_module(friend).get_full_name_str()
            )
        }
        if module_env.get_struct_count() > 0 {
            for struct_env in module_env.get_structs() {
                self.print_struct(struct_env.get_qualified_id())
            }
        }
        if module_env.get_function_count() > 0 {
            for fun_env in module_env.get_functions() {
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
        emit!(
            self.writer,
            "fun {}{}",
            self.sym(fun_env.get_name()),
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
            // A sequence or block is already automatically printed in braces with indent
            let requires_braces = !Self::is_braced(def);
            let exp_sourcifier = ExpSourcifier::for_fun(self, &fun_env, def);
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
        } else {
            emitln!(self.writer, ";");
        }
    }

    fn is_braced(exp: &Exp) -> bool {
        match exp.as_ref() {
            ExpData::Sequence(_, exps) => exps.len() != 1 || Self::is_braced(&exps[0]),
            ExpData::Block(..) => true,
            _ => false,
        }
    }

    fn print_access_specifiers(&self, tctx: &TypeDisplayContext, fun: &FunctionEnv) {
        if let Some(specs) = fun.get_access_specifiers() {
            self.writer.indent();
            for spec in specs {
                emitln!(self.writer);
                if spec.negated {
                    emit!(self.writer, "!")
                }
                match &spec.kind {
                    AccessSpecifierKind::Reads => emit!(self.writer, "reads "),
                    AccessSpecifierKind::Writes => emit!(self.writer, "writes "),
                    AccessSpecifierKind::LegacyAcquires => emit!(self.writer, "acquires "),
                }
                match &spec.resource.1 {
                    ResourceSpecifier::Any => emit!(self.writer, "*"),
                    ResourceSpecifier::DeclaredAtAddress(addr) => {
                        emit!(
                            self.writer,
                            "0x{}::*",
                            addr.expect_numerical().short_str_lossless()
                        )
                    },
                    ResourceSpecifier::DeclaredInModule(mid) => {
                        emit!(
                            self.writer,
                            "{}::*",
                            self.env().get_module(*mid).get_full_name_str()
                        )
                    },
                    ResourceSpecifier::Resource(sid) => {
                        emit!(self.writer, "{}", sid.to_type().display(tctx))
                    },
                }
                if !matches!(spec.address.1, AddressSpecifier::Any) {
                    emit!(self.writer, "(");
                    match &spec.address.1 {
                        AddressSpecifier::Any => {
                            // should be also valid, but not reached:
                            //   emit!(self.writer, "*")
                            unreachable!()
                        },
                        AddressSpecifier::Address(addr) => {
                            emit!(
                                self.writer,
                                "0x{}",
                                addr.expect_numerical().short_str_lossless()
                            )
                        },
                        AddressSpecifier::Parameter(sym) => {
                            emit!(self.writer, "{}", self.sym(*sym))
                        },
                        AddressSpecifier::Call(fun, sym) => emit!(
                            self.writer,
                            "{}({})",
                            self.env()
                                .get_function(fun.to_qualified_id())
                                .get_full_name_str(),
                            self.sym(*sym)
                        ),
                    }
                    emit!(self.writer, ")")
                }
            }
            self.writer.unindent();
            emitln!(self.writer)
        }
    }

    pub fn print_value(&self, value: &Value, ty: Option<&Type>) {
        match value {
            Value::Address(address) => emit!(self.writer, "{}", self.env().display(address)),
            Value::Number(int) => {
                emit!(self.writer, "{}", int);
                if let Some(Type::Primitive(prim)) = ty {
                    emit!(self.writer, match prim {
                        PrimitiveType::U8 => "u8",
                        PrimitiveType::U16 => "u16",
                        PrimitiveType::U32 => "u32",
                        PrimitiveType::U64 => "",
                        PrimitiveType::U128 => "u128",
                        PrimitiveType::U256 => "u256",
                        _ => "",
                    })
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
                    self.print_value(value, elem_ty)
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
                    self.print_value(value, elem_ty)
                })
            },
            Value::ByteArray(bytes) => {
                self.print_list("vector[", ", ", "]", bytes.iter(), |byte| {
                    emit!(self.writer, "{}u8", byte)
                })
            },
            Value::AddressArray(addresses) => {
                self.print_list("vector[", ", ", "]", addresses.iter(), |address| {
                    emit!(self.writer, "{}", self.env().display(address))
                })
            },
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
        if struct_env.has_variants() {
            emitln!(
                self.writer,
                "enum {}{}{} {{",
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
                "struct {}{}{} {{",
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
        let spec = struct_env.get_spec();
        if !spec.is_empty() {
            // TODO: support specs, the output below is debug output
            emitln!(self.writer, "/*\n {}\n*/", self.env().display(&*spec))
        }
    }

    /// Check for dummy field the compiler introduces for empty structs
    fn is_dummy_field(&self, fld: &FieldEnv) -> bool {
        fld.get_type().is_bool() && self.sym(fld.get_name()) == "dummy_field"
    }

    fn field(&self, tctx: &TypeDisplayContext, field: &FieldEnv) -> String {
        format!(
            "{}: {}",
            self.sym(field.get_name()),
            field.get_type().display(tctx)
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
        sym.display(self.env().symbol_pool()).to_string()
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
    /// Creates a sourcifier for the given expression in the context of the function
    pub fn for_fun(parent: &'a Sourcifier<'a>, fun_env: &'a FunctionEnv, exp: &Exp) -> Self {
        print!("[Debug] Processing function {:?}\n", fun_env.get_name_str());
        let type_display_context = fun_env.get_type_display_ctx();
        let temp_names = (0..fun_env.get_parameter_count())
            .map(|i| (i, fun_env.get_local_name(i)))
            .collect();
        Self::new(parent, type_display_context, temp_names, exp)
    }

    fn new(
        parent: &'a Sourcifier<'a>,
        type_display_context: TypeDisplayContext<'a>,
        temp_names: BTreeMap<TempIndex, Symbol>,
        for_exp: &Exp,
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
        }
    }

    fn print_exp(&self, context_prio: Priority, is_result: bool, exp: &Exp) {
        use ExpData::*;
        match exp.as_ref() {
            // Following forms are all atomic and do not require parenthesis
            Invalid(_) => emit!(self.wr(), "*invalid*"),
            Value(_, v) => {
                let ty = self.env().get_node_type(exp.node_id());
                self.parent.print_value(v, Some(&ty));
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
                    self.print_pat(pat);
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
                                        self.print_pat(pat);
                                        if let Some(exp) = binding {
                                            emit!(self.wr(), " = ");
                                            self.print_exp(Prio::General, is_result, exp);
                                        }
                                        if last {
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
                    self.print_pat(&arm.pattern);
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
                let general_loop = || {
                    emit!(self.wr(), "loop ");
                    print_loop_body(body)
                };

                // Detect while loop
                match body.as_ref() {
                    // Pattern 1: loop { if (c) e else break }
                    IfElse(_, cond, if_true, if_false) if if_false.is_loop_cont(Some(0), false) => {
                        emit!(self.wr(), "while (");
                        self.print_exp(Prio::General, false, cond);
                        emit!(self.wr(), ") ");
                        print_loop_body(if_true)
                    },
                    // Pattern 2: loop { if (c) break else e }
                    IfElse(_, cond, if_true, if_false) if if_true.is_loop_cont(Some(0), false) => {
                        emit!(self.wr(), "while (");
                        self.print_exp(
                            Prio::General,
                            false,
                            &self.parent.builder.not(cond.clone()),
                        );
                        emit!(self.wr(), ") ");
                        print_loop_body(if_false)
                    },
                    // Pattern 3: loop { if (c) e; break }
                    Sequence(_, stms)
                        if stms.len() == 2 && stms[1].is_loop_cont(Some(0), false) =>
                    {
                        match stms[0].as_ref() {
                            IfElse(_, cond, if_true, if_false) if if_false.is_unit_exp() => {
                                emit!(self.wr(), "while (");
                                self.print_exp(Prio::General, false, cond);
                                emit!(self.wr(), ") ");
                                print_loop_body(if_true)
                            },
                            _ => general_loop(),
                        }
                    },
                    // Pattern 4: loop { ( if (c) break )+; e }
                    Sequence(_, stms) if !stms.is_empty() => {
                        if let Some((cond, rest)) =
                            self.parent.builder.match_if_break_list(body.clone())
                        {
                            emit!(self.wr(), "while (");
                            self.print_exp(Prio::General, false, &self.parent.builder.not(cond));
                            emit!(self.wr(), ") ");
                            print_loop_body(&rest)
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
                self.print_pat(pat);
                emit!(self.wr(), " = ");
                self.print_exp(Prio::General, false, val)
            }),
            Mutate(_, lhs, rhs) => self.parenthesize(context_prio, Prio::General, || {
                if matches!(
                    lhs.as_ref(),
                    Call(_, Operation::Select(..) | Operation::SelectVariants(..), _)
                ) {
                    // No deref required
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
                // TODO: make specs true source ('SpecSourcifier'), env.display(s)
                // is debug print.
                emitln!(self.wr());
                emitln!(self.wr(), "/* {} */", self.env().display(spec))
            },
            Quant(..) => {
                emitln!(self.wr(), "/* unsupported spec quantifier */")
            },
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
                        self.module_qualifier(*mid),
                        self.sym(fun_env.get_name())
                    );
                    self.print_node_inst(id);
                    self.print_exp_list("(", ")", args)
                })
            },
            Operation::Closure(mid, fid, mask) => {
                self.parenthesize(context_prio, Prio::Postfix, || {
                    // We need to generate `|t| f(captured, t)` to represent the closure.
                    // For that, we first create the closure expression, then call
                    // sourcifier again.
                    let loc = self.env().get_node_loc(id);
                    let ty = self.env().get_node_type(id);
                    let Type::Fun(arg_ty, res_ty, _) = ty else {
                        emit!(self.wr(), "<<wrongly typed closure expression>>");
                        return;
                    };
                    let mut lambda_params = vec![];
                    let mut lambda_param_tys = vec![];
                    let mut lambda_param_exps = vec![];
                    for (i, ty) in arg_ty.flatten().into_iter().enumerate() {
                        let name = self.new_unique_name(args, &format!("arg{}", i));
                        let local_id = self.env().new_node(loc.clone(), ty.clone());
                        lambda_params.push(Pattern::Var(local_id, name));
                        lambda_param_exps.push(ExpData::LocalVar(local_id, name).into_exp());
                        lambda_param_tys.push(ty);
                    }
                    let Some(all_args) = mask.compose(args.iter().cloned(), lambda_param_exps)
                    else {
                        emit!(self.wr(), "<<inconsistent closure mask>>");
                        return;
                    };
                    let call_exp = ExpData::Call(
                        self.env().new_node(loc.clone(), res_ty.as_ref().clone()),
                        Operation::MoveFunction(*mid, *fid),
                        all_args,
                    )
                    .into_exp();
                    let lambda_pat = if lambda_params.len() != 1 {
                        Pattern::Tuple(
                            self.env()
                                .new_node(loc.clone(), Type::tuple(lambda_param_tys)),
                            lambda_params,
                        )
                    } else {
                        lambda_params.pop().unwrap()
                    };
                    self.print_exp(
                        context_prio,
                        false,
                        &ExpData::Lambda(
                            id,
                            lambda_pat,
                            call_exp,
                            LambdaCaptureKind::Default,
                            None,
                        )
                        .into_exp(),
                    )
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
            Operation::Tuple => self.print_exp_list("(", ")", args),
            Operation::Select(mid, sid, fid) => {
                self.parenthesize(context_prio, Prio::Postfix, || {
                    let struct_env = self.env().get_module(*mid).into_struct(*sid);
                    let field_env = struct_env.get_field(*fid);
                    let result_ty = self.env().get_node_type(id);
                    if result_ty.is_immutable_reference() {
                        emit!(self.wr(), "&")
                    } else if result_ty.is_mutable_reference() {
                        emit!(self.wr(), "&mut ")
                    }
                    self.print_exp(Prio::Postfix, false, &args[0]);
                    emit!(self.wr(), ".{}", self.sym(field_env.get_name()))
                })
            },
            Operation::SelectVariants(mid, sid, fids) => {
                self.parenthesize(context_prio, Prio::Postfix, || {
                    let struct_env = self.env().get_module(*mid).into_struct(*sid);
                    // All field names are the same, so we can choose one representative
                    // on source level.
                    let fid = fids[0];
                    let field_env = struct_env.get_field(fid);
                    let result_ty = self.env().get_node_type(id);
                    if result_ty.is_immutable_reference() {
                        emit!(self.wr(), "&")
                    } else if result_ty.is_mutable_reference() {
                        emit!(self.wr(), "&mut ")
                    }
                    self.print_exp(Prio::Postfix, false, &args[0]);
                    emit!(self.wr(), ".{}", self.sym(field_env.get_name()))
                })
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
            Operation::And => self.print_bin(context_prio, Prio::LogicalAnd, "&&", args),
            Operation::Or => self.print_bin(context_prio, Prio::LogicalOr, "||", args),
            Operation::Eq => self.print_bin(context_prio, Prio::Relations, "==", args),
            Operation::Neq => self.print_bin(context_prio, Prio::Relations, "!=", args),
            Operation::Lt => self.print_bin(context_prio, Prio::Relations, "<", args),
            Operation::Gt => self.print_bin(context_prio, Prio::Relations, ">", args),
            Operation::Le => self.print_bin(context_prio, Prio::Relations, "<=", args),
            Operation::Ge => self.print_bin(context_prio, Prio::Relations, ">=", args),
            Operation::Copy => self.parenthesize(context_prio, Prio::General, || {
                emit!(self.wr(), "copy ");
                self.print_exp(Prio::General, false, &args[0])
            }),
            Operation::Move => self.parenthesize(context_prio, Prio::General, || {
                emit!(self.wr(), "move ");
                self.print_exp(Prio::General, false, &args[0])
            }),
            Operation::Abort => self.parenthesize(context_prio, Prio::General, || {
                emit!(self.wr(), "abort ");
                self.print_exp(Prio::General, false, &args[0])
            }),
            Operation::Freeze(_) => {
                emit!(self.wr(), "/*freeze*/");
                self.print_exp(context_prio, false, &args[0])
            },
            Operation::Not => self.parenthesize(context_prio, Prio::Prefix, || {
                emit!(self.wr(), "!");
                self.print_exp(Prio::Prefix, false, &args[0])
            }),
            Operation::Cast => self.parenthesize(context_prio, Prio::General, || {
                self.print_exp(Prio::General, false, &args[0]);
                let ty = self.env().get_node_type(id);
                emit!(self.wr(), " as {}", self.ty(&ty))
            }),
            Operation::Exists(_) => self.parenthesize(context_prio, Prio::Postfix, || {
                emit!(self.wr(), "exists");
                self.print_node_inst(id);
                self.print_exp_list("(", ")", args)
            }),
            Operation::BorrowGlobal(kind) => self.parenthesize(context_prio, Prio::Postfix, || {
                if *kind == ReferenceKind::Mutable {
                    emit!(self.wr(), "borrow_global_mut")
                } else {
                    emit!(self.wr(), "borrow_global")
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
                self.print_exp(Prio::Prefix, false, &args[0])
            }),
            Operation::Deref => self.parenthesize(context_prio, Prio::Prefix, || {
                emit!(self.wr(), "*");
                self.print_exp(Prio::Prefix, false, &args[0])
            }),
            Operation::Vector => self.parenthesize(context_prio, Prio::Postfix, || {
                emit!(self.wr(), "vector");
                self.print_exp_list("[", "]", args)
            }),

            // Following belong to specs and currently are not supported
            Operation::Len
            | Operation::TypeValue
            | Operation::TypeDomain
            | Operation::ResourceDomain
            | Operation::Global(_)
            | Operation::CanModify
            | Operation::Old
            | Operation::Trace(_)
            | Operation::EmptyVec
            | Operation::SingleVec
            | Operation::UpdateVec
            | Operation::ConcatVec
            | Operation::IndexOfVec
            | Operation::ContainsVec
            | Operation::InRangeRange
            | Operation::InRangeVec
            | Operation::RangeVec
            | Operation::MaxU8
            | Operation::MaxU16
            | Operation::MaxU32
            | Operation::MaxU64
            | Operation::MaxU128
            | Operation::MaxU256
            | Operation::Bv2Int
            | Operation::Int2Bv
            | Operation::AbortFlag
            | Operation::AbortCode
            | Operation::WellFormed
            | Operation::BoxValue
            | Operation::UnboxValue
            | Operation::EmptyEventStore
            | Operation::ExtendEventStore
            | Operation::EventStoreIncludes
            | Operation::EventStoreIncludedIn
            | Operation::SpecFunction(_, _, _)
            | Operation::UpdateField(_, _, _)
            | Operation::Result(_)
            | Operation::Index
            | Operation::Slice
            | Operation::Range
            | Operation::Implies
            | Operation::Iff
            | Operation::Identical
            | Operation::NoOp => {
                emitln!(self.wr(), "/* unsupported spec operation {:?} */", oper)
            },
        }
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
            self.module_qualifier(struct_env.module_env.get_id()),
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
            ",",
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

    fn print_node_inst(&self, id: NodeId) {
        if let Some(inst) = self.env().get_node_instantiation_opt(id) {
            self.print_inst(&inst)
        }
    }

    fn print_inst(&self, inst: &[Type]) {
        if !inst.is_empty() {
            self.parent.print_list("<", ",", ">", inst.iter(), |ty| {
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

    fn print_pat(&self, pat: &Pattern) {
        match pat {
            Pattern::Var(_, name) => emit!(self.wr(), "{}", self.sym(*name)),
            Pattern::Wildcard(_) => emit!(self.wr(), "_"),
            Pattern::Tuple(_, elems) => self
                .parent
                .print_list("(", ",", ")", elems.iter(), |p| self.print_pat(p)),
            Pattern::Struct(_, qid, variant, elems) => {
                self.print_constructor(qid, variant, elems.iter(), |p| self.print_pat(p))
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

    fn module_qualifier(&self, mid: ModuleId) -> String {
        let module_env = self.env().get_module(mid);
        if self.type_display_context.module_name.as_ref() == Some(module_env.get_name()) {
            // Current module, no qualification needed
            "".to_string()
        } else {
            format!(
                "{}::",
                if self.type_display_context.used_modules.contains(&mid) {
                    self.sym(module_env.get_name().name())
                } else {
                    module_env.get_full_name_str()
                }
            )
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
}
