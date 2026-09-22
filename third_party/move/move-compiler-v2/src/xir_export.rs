// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Exports a module's **interface** as XIR.
//!
//! This is the inverse of the declaration half of [`crate::xir`], and
//! deliberately only that half: function bodies are omitted. A dependency
//! needs signatures, types, abilities, visibility, attributes, and friend
//! declarations in order to be compiled against; it does not need code, and
//! omitting code avoids translating stackless bytecode back into XIR
//! instructions — the expensive part of a full exporter.
//!
//! The result therefore carries `dialect: Stackless` with every function's
//! `blocks` and `loops` empty. That is not a distinct dialect: bodies are
//! simply absent, and the reader accepts absent bodies for a module supplied
//! as a *dependency* while still requiring them of a compilation target.
//!
//! A full-fidelity exporter can be built on this code later by adding body
//! translation.

use anyhow::{anyhow, bail, Context, Result};
use move_binary_format::file_format::Visibility as MoveVisibility;
use move_core_types::ability::AbilitySet;
use move_model::{
    ast::{Address, Attribute, AttributeValue, Spec, SpecBlockTarget, SpecFunDecl, Value},
    exp_rewriter::ExpRewriterFunctions,
    model::{
        FunId, FunctionEnv, Loc, ModuleEnv, ModuleId, NamedConstantEnv, QualifiedId, StructEnv,
        StructId,
    },
    sourcifier::Sourcifier,
    symbol::Symbol,
    ty::{PrimitiveType, ReferenceKind, Type},
};
use move_model_exchange::{
    Field, Type as Ty, TypeParameter as TypeParameterDecl, Value as XirValue, Variant,
    XirAttribute, XirAttributeArg, XirConstant, XirDialect, XirExternalFunction, XirExternalStruct,
    XirFunction, XirModule, XirModuleMetadata, XirModuleRef, XirSpecDeclaration, XirStruct, XirUse,
    XirUseMember, XirVisibility, XIR_SCHEMA, XIR_VERSION,
};
use std::collections::{BTreeMap, BTreeSet};

/// Accumulates the tables that give foreign declarations an index.
///
/// Local declarations occupy `0..locals`, so a reference to another module is
/// appended and addressed at `locals + position` — the convention the reader
/// resolves, and the one a bytecode handle table uses.
struct References<'env> {
    module: &'env ModuleEnv<'env>,
    local_structs: BTreeMap<StructId, usize>,
    external_structs: Vec<XirExternalStruct>,
    external_index: BTreeMap<QualifiedId<StructId>, usize>,
    local_funs: BTreeMap<FunId, usize>,
    external_functions: Vec<XirExternalFunction>,
    external_fun_index: BTreeMap<QualifiedId<FunId>, usize>,
}

impl<'env> References<'env> {
    fn new(module: &'env ModuleEnv<'env>) -> Self {
        let local_structs = exported_structs(module)
            .enumerate()
            .map(|(index, struct_env)| (struct_env.get_id(), index))
            .collect();
        let local_funs = exported_funs(module)
            .enumerate()
            .map(|(index, fun_env)| (fun_env.get_id(), index))
            .collect();
        Self {
            module,
            local_structs,
            external_structs: vec![],
            external_index: BTreeMap::new(),
            local_funs,
            external_functions: vec![],
            external_fun_index: BTreeMap::new(),
        }
    }

    /// The XIR function id for a callee, interning a foreign one on first use.
    fn function_id(&mut self, module_id: ModuleId, fun_id: FunId) -> Result<usize> {
        if module_id == self.module.get_id() {
            return self
                .local_funs
                .get(&fun_id)
                .copied()
                .context("a local callee is missing from the module's declarations");
        }
        let qid = module_id.qualified(fun_id);
        if let Some(index) = self.external_fun_index.get(&qid) {
            return Ok(self.local_funs.len() + index);
        }
        let env = self.module.env;
        let foreign = env.get_module(module_id);
        let name = foreign.get_name();
        let index = self.external_functions.len();
        self.external_functions.push(XirExternalFunction {
            address: name.addr().expect_numerical().to_hex_literal(),
            module: env.symbol_pool().string(name.name()).to_string(),
            function: env
                .symbol_pool()
                .string(foreign.get_function(fun_id).get_name())
                .to_string(),
        });
        self.external_fun_index.insert(qid, index);
        Ok(self.local_funs.len() + index)
    }

    fn local_count(&self) -> usize {
        self.local_structs.len()
    }

    /// The XIR resource id for a struct, interning a foreign one on first use.
    fn resource_id(&mut self, module_id: ModuleId, struct_id: StructId) -> Result<usize> {
        if module_id == self.module.get_id() {
            return self
                .local_structs
                .get(&struct_id)
                .copied()
                .context("a local struct is missing from the module's own declarations");
        }
        let qid = module_id.qualified(struct_id);
        if let Some(index) = self.external_index.get(&qid) {
            return Ok(self.local_count() + index);
        }
        let env = self.module.env;
        let foreign = env.get_module(module_id);
        let struct_env = foreign.get_struct(struct_id);
        let name = foreign.get_name();
        let index = self.external_structs.len();
        self.external_structs.push(XirExternalStruct {
            address: name.addr().expect_numerical().to_hex_literal(),
            module: env.symbol_pool().string(name.name()).to_string(),
            name: env.symbol_pool().string(struct_env.get_name()).to_string(),
        });
        self.external_index.insert(qid, index);
        Ok(self.local_count() + index)
    }
}

fn ability_names(abilities: AbilitySet) -> Vec<String> {
    abilities
        .iter()
        .map(|ability| ability.to_string())
        .collect()
}

fn visibility(visibility: MoveVisibility) -> XirVisibility {
    match visibility {
        MoveVisibility::Private => XirVisibility::Private,
        MoveVisibility::Public => XirVisibility::Public,
        MoveVisibility::Friend => XirVisibility::Friend,
    }
}

/// Translates a model type, interning any foreign declaration it names.
fn export_type(refs: &mut References, ty: &Type) -> Result<Ty> {
    Ok(match ty {
        Type::Primitive(primitive) => match primitive {
            PrimitiveType::Bool => Ty::Bool,
            PrimitiveType::U8 => Ty::U8,
            PrimitiveType::U16 => Ty::U16,
            PrimitiveType::U32 => Ty::U32,
            PrimitiveType::U64 => Ty::U64,
            PrimitiveType::U128 => Ty::U128,
            PrimitiveType::U256 => Ty::U256,
            PrimitiveType::I8 => Ty::I8,
            PrimitiveType::I16 => Ty::I16,
            PrimitiveType::I32 => Ty::I32,
            PrimitiveType::I64 => Ty::I64,
            PrimitiveType::I128 => Ty::I128,
            PrimitiveType::I256 => Ty::I256,
            PrimitiveType::Address => Ty::Address,
            PrimitiveType::Signer => Ty::Signer,
            other => bail!("`{other:?}` is a specification-only type and has no XIR form"),
        },
        Type::TypeParameter(index) => Ty::TypeParameter(*index as usize),
        Type::Struct(module_id, struct_id, args) => {
            let id = refs.resource_id(*module_id, *struct_id)?;
            // An enum is a struct with variants; the reader treats the two
            // resource forms alike, so the struct form is always correct here.
            if args.is_empty() {
                Ty::Struct(id)
            } else {
                Ty::StructInst(
                    id,
                    args.iter()
                        .map(|arg| export_type(refs, arg))
                        .collect::<Result<Vec<_>>>()?,
                )
            }
        },
        Type::Vector(element) => Ty::Vector(Box::new(export_type(refs, element)?)),
        Type::Reference(ReferenceKind::Immutable, referent) => {
            Ty::Ref(Box::new(export_type(refs, referent)?))
        },
        Type::Reference(ReferenceKind::Mutable, referent) => {
            Ty::MutRef(Box::new(export_type(refs, referent)?))
        },
        Type::Fun(args, result, abilities) => Ty::Fun(
            export_type_list(refs, args)?,
            export_type_list(refs, result)?,
            ability_names(*abilities),
        ),
        other => bail!("type `{other:?}` has no XIR form"),
    })
}

/// Exports the argument or result position of a function type, which XIR
/// spells as a list.
///
/// `move_model` writes such a position as a bare type at arity one and as a
/// `Tuple` otherwise — `|T|` is `Fun(T, Tuple([]))`, `|&T, &V|` is
/// `Fun(Tuple([&T, &V]), Tuple([]))`. This is the only place a `Tuple` is
/// legitimate, which is why `export_type` rejects it everywhere else.
fn export_type_list(refs: &mut References, ty: &Type) -> Result<Vec<Ty>> {
    match ty {
        Type::Tuple(types) => types
            .iter()
            .map(|ty| export_type(refs, ty))
            .collect::<Result<Vec<_>>>(),
        single => Ok(vec![export_type(refs, single)?]),
    }
}

/// Translates model attributes, the inverse of `xir::model_attribute`.
///
/// A top-level attribute is a `{name, args}` object, so an assignment can only
/// be spelled there by the reader's rule that a lone `num`/`bool` argument
/// means assignment:
///
/// - `Apply(name, args)`                → `{name, args}`
/// - `Assign(name, Value::Number|Bool)` → `{name, args: [num|bool]}`
///
/// In *argument* position both forms are available, and assignment always uses
/// the explicit one — `{assign, value}` — including for literals. That keeps a
/// single rule for reading a nested argument and is what lets an assigned
/// *name* be represented at all, as in
/// `#[resource_group_member(group = 0x1::object::ObjectGroup)]`, which the
/// framework's extended checker requires to stay an `Assign`.
fn export_attributes(module: &ModuleEnv, attributes: &[Attribute]) -> Result<Vec<XirAttribute>> {
    attributes
        .iter()
        .map(|attribute| {
            let (name, args) = export_attribute_parts(module, attribute)?;
            Ok(XirAttribute { name, args })
        })
        .collect()
}

fn export_attribute_parts(
    module: &ModuleEnv,
    attribute: &Attribute,
) -> Result<(String, Vec<XirAttributeArg>)> {
    let pool = module.env.symbol_pool();
    match attribute {
        Attribute::Apply(_, name, args) => Ok((
            pool.string(*name).to_string(),
            args.iter()
                .map(|arg| export_attribute_arg(module, arg))
                .collect::<Result<Vec<_>>>()?,
        )),
        Attribute::Assign(_, name, value @ AttributeValue::Value(..)) => {
            Ok((pool.string(*name).to_string(), vec![
                export_attribute_value(module, *name, value)?,
            ]))
        },
        // A top-level `{name, args}` object has no slot for an assigned name;
        // only argument position does. No framework attribute takes this
        // shape, so it is reported rather than reshaped into an application,
        // which the extended checker would not recognize.
        Attribute::Assign(_, name, AttributeValue::Name(..)) => bail!(
            "attribute `{}` is assigned a name at top level, which XIR represents \
             only in argument position",
            pool.string(*name)
        ),
    }
}

fn export_attribute_arg(module: &ModuleEnv, attribute: &Attribute) -> Result<XirAttributeArg> {
    let pool = module.env.symbol_pool();
    match attribute {
        Attribute::Apply(_, name, args) => Ok(XirAttributeArg::Name {
            name: pool.string(*name).to_string(),
            args: args
                .iter()
                .map(|arg| export_attribute_arg(module, arg))
                .collect::<Result<Vec<_>>>()?,
        }),
        Attribute::Assign(_, name, value) => Ok(XirAttributeArg::Assign {
            assign: pool.string(*name).to_string(),
            value: Box::new(export_attribute_value(module, *name, value)?),
        }),
    }
}

/// Translates the right-hand side of an attribute assignment. A name is
/// rendered as a path so that its module qualifier survives; the reader splits
/// it back at the last `::`.
fn export_attribute_value(
    module: &ModuleEnv,
    name: Symbol,
    value: &AttributeValue,
) -> Result<XirAttributeArg> {
    let pool = module.env.symbol_pool();
    match value {
        AttributeValue::Value(_, Value::Number(number)) => Ok(XirAttributeArg::Num {
            value: number.to_string(),
        }),
        AttributeValue::Value(_, Value::Bool(flag)) => Ok(XirAttributeArg::Bool { value: *flag }),
        AttributeValue::Value(_, other) => bail!(
            "attribute `{}` has a value of an unsupported kind: {other:?}",
            pool.string(name)
        ),
        AttributeValue::Name(_, module_name, symbol) => {
            let symbol = pool.string(*symbol).to_string();
            Ok(XirAttributeArg::Name {
                name: match module_name {
                    Some(module_name) => {
                        format!("{}::{}", module_name.display_full(module.env), symbol)
                    },
                    None => symbol,
                },
                args: vec![],
            })
        },
    }
}

fn export_type_parameters(
    module: &ModuleEnv,
    parameters: &[move_model::model::TypeParameter],
) -> Vec<TypeParameterDecl> {
    parameters
        .iter()
        .map(|parameter| TypeParameterDecl {
            name: module.env.symbol_pool().string(parameter.0).to_string(),
            abilities: ability_names(parameter.1.abilities),
            phantom: parameter.1.is_phantom,
        })
        .collect()
}

fn export_struct(refs: &mut References, struct_env: &StructEnv) -> Result<XirStruct> {
    let module = &struct_env.module_env;
    let pool = module.env.symbol_pool();
    let variants = if struct_env.has_variants() {
        Some(
            struct_env
                .get_variants()
                .map(|variant| {
                    Ok(Variant {
                        name: pool.string(variant).to_string(),
                        fields: struct_env
                            .get_fields_of_variant(variant)
                            .map(|field| {
                                Ok(Field {
                                    name: pool.string(field.get_name()).to_string(),
                                    ty: export_type(refs, &field.get_type())?,
                                })
                            })
                            .collect::<Result<Vec<_>>>()?,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        )
    } else {
        None
    };
    // Enum fields live on the variants; the top-level list stays empty.
    let fields = if variants.is_some() {
        vec![]
    } else {
        struct_env
            .get_fields()
            .map(|field| {
                Ok(Field {
                    name: pool.string(field.get_name()).to_string(),
                    ty: export_type(refs, &field.get_type())?,
                })
            })
            .collect::<Result<Vec<_>>>()?
    };
    Ok(XirStruct {
        name: pool.string(struct_env.get_name()).to_string(),
        visibility: visibility(struct_env.get_visibility()),
        abilities: ability_names(struct_env.get_abilities()),
        type_parameters: export_type_parameters(module, struct_env.get_type_parameters()),
        fields,
        variants,
        attributes: export_attributes(module, struct_env.get_attributes())?,
    })
}

/// Exports one function's declaration surface. `locals` holds exactly the
/// parameter types, since with no body there are no other locals.
fn export_function(refs: &mut References, fun_env: &FunctionEnv) -> Result<XirFunction> {
    let module = &fun_env.module_env;
    let pool = module.env.symbol_pool();
    let locals = fun_env
        .get_parameters_ref()
        .iter()
        .map(|parameter| export_type(refs, &parameter.1))
        .collect::<Result<Vec<_>>>()?;
    let returns = export_type_list(refs, &fun_env.get_result_type())?;
    let mut calls = interface_callees(fun_env)?
        .into_iter()
        .map(|callee| refs.function_id(callee.module_id, callee.id))
        .collect::<Result<Vec<_>>>()?;
    calls.sort_unstable();
    let acquires = fun_env
        .get_acquires_global_resources()
        .unwrap_or_default()
        .into_iter()
        .map(|struct_id| refs.resource_id(module.get_id(), struct_id))
        .collect::<Result<Vec<_>>>()?;
    Ok(XirFunction {
        name: pool.string(fun_env.get_name()).to_string(),
        type_parameters: export_type_parameters(module, fun_env.get_type_parameters_ref()),
        visibility: visibility(fun_env.visibility()),
        is_entry: fun_env.is_entry(),
        is_native: fun_env.is_native(),
        acquires,
        params: locals.len(),
        locals,
        // Parameter names are not decoration here. Move's receiver syntax
        // (`v.length()`) resolves only against a function whose first
        // parameter is named `self`, so dropping names turns every receiver
        // call on a dependency into "undeclared receiver function".
        local_names: fun_env
            .get_parameters_ref()
            .iter()
            .map(|parameter| Some(pool.string(parameter.0).to_string()))
            .collect(),
        returns,
        calls,
        blocks: vec![],
        entry: 0,
        loops: vec![],
        spec: move_model_exchange::Contract {
            requires: vec![],
            aborts_if: vec![],
            ensures: vec![],
            modifies: vec![],
        },
        attributes: export_attributes(module, fun_env.get_attributes())?,
        source_map: None,
        source: inline_source(fun_env)?,
    })
}

/// Exports the interface of `module`.
///
/// Private functions are omitted: a dependent cannot call them, so they are
/// not part of the surface it compiles against. Every struct is included,
/// because a type may be named even where its constructor is not accessible.
/// The structs that belong in an interface, in the declaration order that
/// fixes their resource ids.
///
/// Ghost memory is excluded: a specification variable — `spec module { global
/// supply<CoinType>: num; }` in `coin` — is represented in the model as a
/// struct, but it has no runtime existence and its type is spec-only, so no
/// dependent ever compiles against it.
///
/// [`References::new`] indexes the same sequence, so the filter must not be
/// applied in one place and not the other: dropping a struct from the exported
/// list alone would shift every id after it.
fn exported_structs<'env>(
    module: &'env ModuleEnv<'env>,
) -> impl Iterator<Item = StructEnv<'env>> + 'env {
    module
        .get_structs()
        .filter(|struct_env| !struct_env.is_ghost_memory())
}

/// Whether a function belongs in an interface: something a dependent can
/// actually *link against*.
///
/// Private functions are excluded because a dependent cannot name them.
///
/// Inline functions **are** included, but they are the one case a declaration
/// cannot describe. A dependent does not link an inline function, it expands
/// it, and an inline function has no entry in the deployed module at all — it
/// never reaches file-format generation. Declaring one `native`, as this
/// exporter does for every other function, would therefore be an active lie:
/// the dependent would emit a call to a function that does not exist and fail
/// at *runtime* with `FUNCTION_RESOLUTION_FAILURE`. So an inline function
/// carries its rendered body in [`XirFunction::source`] instead — see
/// [`inline_source`] — and the interface generator emits that rather than a
/// stub.
///
/// Omitting them, which this function used to do, was the safe half-measure:
/// it moved the failure to compile time, at the cost of making every framework
/// package unusable as an interface, since all three export non-private inline
/// functions.
///
/// Compiler-generated wrappers are excluded too. `pack$S`, `borrow_mut$S$N`
/// and `const$NAME` are synthesized during file-format generation to realize
/// struct visibility and cross-module constant access; a dependent never names
/// one, the compiler generates its own calls to them, and `$` is not even a
/// legal Move identifier — so emitting them produces an interface that cannot
/// be parsed back.
///
/// These only exist in a model that has been through the *full* compiler.
/// A model from `run_checker` alone has none, which is why an
/// export-and-typecheck sweep cannot discover this and the first modular build
/// did immediately.
fn exported_in_interface(fun_env: &FunctionEnv) -> bool {
    fun_env.visibility() != MoveVisibility::Private
        && !fun_env.is_struct_api()
        && !fun_env.is_const_accessor()
}

/// Empties every `spec { }` block in an expression, leaving its structure.
///
/// An interface carries declarations; it does not carry the module's spec file.
/// A rendered body that mentions a spec function therefore names something the
/// dependent cannot resolve — `vector::map_ref`'s loop invariants call
/// `spec_map_ref`, which lives in `vector.spec.move`. Spec blocks contribute no
/// bytecode, so emptying them removes exactly what a dependent cannot compile
/// and nothing it needs. Verification is unaffected: the prover runs against
/// sources, never against a generated interface.
struct StripSpecs;

impl ExpRewriterFunctions for StripSpecs {
    fn rewrite_spec(&mut self, _target: &SpecBlockTarget, spec: &Spec) -> Option<Spec> {
        let is_empty = spec.conditions.is_empty()
            && spec.update_map.is_empty()
            && spec.on_impl.is_empty()
            && spec.proof.is_none();
        (!is_empty).then(|| Spec {
            loc: spec.loc.clone(),
            ..Default::default()
        })
    }
}

/// The Move source of an `inline` function, for [`XirFunction::source`].
///
/// A declaration is enough for anything a dependent *links* against; an inline
/// function is expanded instead, so the dependent needs the body. `Sourcifier`
/// renders it — including lambdas, which is what makes this viable where a
/// typed-AST route is not.
///
/// Safe to hand to a dependent because a non-private inline function may only
/// call what its callers can reach: `check_inline_callee_visibility` warns
/// otherwise, and the framework produces no such warning.
fn inline_source(fun_env: &FunctionEnv) -> Result<Option<String>> {
    if !fun_env.is_inline() {
        return Ok(None);
    }

    // `Sourcifier` renders an attribute as its bare name, dropping arguments —
    // `#[expected_failure(abort_code = 1)]` becomes `#[expected_failure]`. For
    // a declaration that is harmless, because the generator emits `attributes`
    // from XIR, which keeps them; for a rendered body it is the whole output,
    // so the arguments would be lost with nothing to notice.
    //
    // No non-private inline function in the framework has an attribute at all,
    // so this rejects nothing today. Reported rather than dropped, on the same
    // grounds as a non-private constant below: an interface that quietly says
    // less than the module is worse than one that refuses to exist.
    if let Some(attribute) = fun_env
        .get_attributes()
        .iter()
        .find(|attribute| match attribute {
            // A bare `#[name]` survives the rendering intact.
            Attribute::Apply(_, _, args) => !args.is_empty(),
            Attribute::Assign(..) => true,
        })
    {
        let pool = fun_env.module_env.env.symbol_pool();
        bail!(
            "inline function `{}` carries attribute `{}` with arguments, which its rendered \
             source cannot represent; this package must be compiled monolithically",
            fun_env.get_full_name_str(),
            pool.string(attribute.name())
        )
    }

    // `amend` is what makes the rendering *recompilable*. The AST reaching here
    // has been desugared, so it holds synthesized locals named `$t`, `$lb` and
    // the like; `$` is not a Move identifier character, and rendering them
    // verbatim produces source no dependent can parse. Amending rewrites them.
    // The rendering lands in a generated file with no `use` declarations, so
    // every external name must carry its address.
    let sourcifier = Sourcifier::new(fun_env.module_env.env, /*amend*/ true)
        .with_fully_qualified_external_names();
    let def = fun_env
        .get_def()
        .map(|def| StripSpecs.rewrite_exp(def.clone()));
    sourcifier.print_fun(fun_env.get_qualified_id(), def.as_ref());
    Ok(Some(sourcifier.result()))
}

/// Whether a specification function is one this module *declares*.
///
/// Two kinds of entry are not:
///
/// - `is_move_fun` marks a Move function lifted into specification space rather
///   than a `spec fun` declaration. The real function is already exported, and
///   re-emitting it would declare the name twice.
/// - **Clones left by inlining.** Expanding a `public inline` function copies
///   the specification functions its body mentions into the *calling* module,
///   once per expansion. `vector::map_ref`'s loop invariant calls
///   `spec_map_ref`, and `confidential_balance` calls `map_ref` twelve times,
///   so that module's table holds a dozen `spec_map_ref` entries that its
///   source never wrote. Emitting them produces duplicate declarations.
///
/// A clone keeps the location it was cloned from, so it is recognised by the
/// file it points into: one that belongs to a different module. A module's own
/// `.spec.move` file is not any module's declaration site, so this leaves those
/// alone.
fn exported_spec_fun(decl: &SpecFunDecl, foreign_locs: &BTreeSet<Loc>) -> bool {
    !decl.is_move_fun && !foreign_locs.contains(&decl.loc)
}

/// Where every *other* module's specification functions are declared — see
/// [`exported_spec_fun`].
///
/// Identity is the declaration's location, not its file. A clone keeps the
/// exact `Loc` it was copied from, so a decl matching one of these was
/// certainly copied rather than written here.
///
/// Matching on the file alone gets this wrong in both directions. Two modules
/// declared in one source file would see their own declarations discarded,
/// since that file is another module's file too; and a clone copied from a
/// foreign `*.spec.move` companion would be kept, since that companion is no
/// module's *declaration* file — leaving one duplicate per inline expansion for
/// the dependent to trip over.
fn foreign_spec_fun_locs(module: &ModuleEnv) -> BTreeSet<Loc> {
    module
        .env
        .get_modules()
        .filter(|other| other.get_id() != module.get_id())
        .flat_map(|other| {
            other
                .get_spec_funs()
                .map(|(_, decl)| decl.loc.clone())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The declaration's original source text, for [`XirSpecDeclaration::source`].
///
/// Copied verbatim rather than rendered from the AST, which is the opposite of
/// what [`inline_source`] does, and deliberately. `Sourcifier` targets the Move
/// *implementation* language and loses fidelity on specification constructs: it
/// expands `self.borrow()` to `borrow(&self)`, which specifications reject, and
/// prints a named constant in `abort EOPTION_NOT_SET` as a bare literal that
/// then types as `u256`. Both were caught by the framework sweep on the first
/// attempt. Copying cannot have such gaps — the text already compiled once.
///
/// The cost is that the text keeps the module's `use` aliases, which is why the
/// generated interface re-emits those declarations.
///
/// Two spellings reach here: `spec fun f(..)` when written at module level, and
/// `fun f(..)` when written inside a `spec <module> { .. }` block. Normalized to
/// the latter, since the generator wraps the result in `spec module { .. }`.
fn spec_declaration_source(module: &ModuleEnv, loc: &move_model::model::Loc) -> Result<String> {
    let text = module
        .env
        .get_source(loc)
        .map_err(|err| anyhow!("cannot read the declaration's source text: {err}"))?
        .trim();
    Ok(match text.strip_prefix("spec ") {
        Some(rest) => rest.trim_start().to_string(),
        None => text.to_string(),
    })
}

/// Converts a constant's value to the format's representation.
fn export_value(value: &Value) -> Result<XirValue> {
    Ok(match value {
        Value::Bool(value) => XirValue::Bool(*value),
        Value::Number(value) => XirValue::Num(value.to_string()),
        Value::Address(address) => XirValue::Address(address.expect_numerical().to_hex_literal()),
        // A byte array is a `vector<u8>`; the format has no separate form, and
        // the declared type already says the width.
        Value::ByteArray(bytes) => XirValue::Vector(
            bytes
                .iter()
                .map(|byte| XirValue::Num(byte.to_string()))
                .collect(),
        ),
        Value::AddressArray(addresses) => XirValue::Vector(
            addresses
                .iter()
                .map(|address| XirValue::Address(address.expect_numerical().to_hex_literal()))
                .collect(),
        ),
        Value::Vector(values) => {
            XirValue::Vector(values.iter().map(export_value).collect::<Result<_>>()?)
        },
        // Not expressible as a Move constant, so reaching this means the model
        // holds something a constant declaration could not have produced.
        Value::Tuple(_) => bail!("a constant cannot hold a tuple"),
    })
}

fn export_constant(refs: &mut References, constant: &NamedConstantEnv) -> Result<XirConstant> {
    let pool = constant.module_env.env.symbol_pool();
    Ok(XirConstant {
        name: pool.string(constant.get_name()).to_string(),
        ty: export_type(refs, &constant.get_type())?,
        value: export_value(&constant.get_value())?,
        visibility: match constant.get_visibility() {
            MoveVisibility::Private => None,
            MoveVisibility::Public => Some(XirVisibility::Public),
            MoveVisibility::Friend => Some(XirVisibility::Friend),
        },
    })
}

fn exported_funs<'env>(
    module: &'env ModuleEnv<'env>,
) -> impl Iterator<Item = FunctionEnv<'env>> + 'env {
    module.get_functions().filter(exported_in_interface)
}

/// Callees of `fun_env` that some interface will declare.
///
/// Expands through anything an interface omits — private and inline functions,
/// in this module or another — so every id recorded resolves for a consumer.
/// An inline callee is still a callee here because the checker has not inlined
/// it yet.
///
/// A dependent's analyses walk the call graph, so an interface reporting no
/// callees lets a transitive property go unnoticed.
fn interface_callees(fun_env: &FunctionEnv) -> Result<BTreeSet<QualifiedId<FunId>>> {
    let env = fun_env.module_env.env;
    let mut reached = BTreeSet::new();
    let mut seen = BTreeSet::new();
    let mut todo = vec![fun_env.get_qualified_id()];
    while let Some(current) = todo.pop() {
        let callees = env
            .get_function(current)
            .get_called_functions()
            .with_context(|| {
                format!(
                    "`{}` has no call graph, so its interface would silently drop one",
                    env.get_function(current).get_full_name_str()
                )
            })?
            .clone();
        for callee in callees {
            if exported_in_interface(&env.get_function(callee)) {
                reached.insert(callee);
            } else if seen.insert(callee) {
                todo.push(callee);
            }
        }
    }
    Ok(reached)
}

pub fn export_interface(module: &ModuleEnv) -> Result<XirModule> {
    let pool = module.env.symbol_pool();
    let name = module.get_name();
    let mut refs = References::new(module);

    let structs = exported_structs(module)
        .map(|struct_env| {
            export_struct(&mut refs, &struct_env)
                .with_context(|| format!("on struct `{}`", struct_env.get_full_name_str()))
        })
        .collect::<Result<Vec<_>>>()?;

    let functions = module
        .get_functions()
        .filter(|fun_env| exported_in_interface(fun_env))
        .map(|fun_env| {
            export_function(&mut refs, &fun_env)
                .with_context(|| format!("on function `{}`", fun_env.get_full_name_str()))
        })
        .collect::<Result<Vec<_>>>()?;

    // Every constant, including private ones — see `XirModule::constants` for
    // why visibility is the wrong filter here.
    let constants = module
        .get_named_constants()
        .map(|constant| {
            export_constant(&mut refs, &constant)
                .with_context(|| format!("on constant `{}`", pool.string(constant.get_name())))
        })
        .collect::<Result<Vec<_>>>()?;

    let foreign_locs = foreign_spec_fun_locs(module);
    let mut spec_declarations = module
        .get_spec_funs()
        .filter(|(_, decl)| exported_spec_fun(decl, &foreign_locs))
        .map(|(_, decl)| {
            Ok(XirSpecDeclaration {
                name: pool.string(decl.name).to_string(),
                source: spec_declaration_source(module, &decl.loc).with_context(|| {
                    format!("on specification function `{}`", pool.string(decl.name))
                })?,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    // Schemas are expanded away during model building, so unlike specification
    // functions they have no table of their own; the record that survives is
    // the spec block's location, which is all that copying needs.
    for info in module.get_spec_block_infos() {
        let SpecBlockTarget::Schema(module_id, schema_id, _) = &info.target else {
            continue;
        };
        if *module_id != module.get_id() || foreign_locs.contains(&info.loc) {
            continue;
        }
        let name = pool.string(schema_id.symbol()).to_string();
        spec_declarations.push(XirSpecDeclaration {
            source: spec_declaration_source(module, &info.loc)
                .with_context(|| format!("on specification schema `{name}`"))?,
            name,
        });
    }

    // Only the copied specification text needs these; see `XirModule::uses`.
    let uses = if spec_declarations.is_empty() {
        vec![]
    } else {
        module
            .get_use_decls()
            .iter()
            .map(|decl| {
                // A `use` records the address as written, which is often a
                // named one — `use std::option` keeps `std` symbolically. The
                // generated interface has to give the numerical address: it is
                // compiled on its own, where no package assigns `std` a value.
                //
                // Resolution goes through the environment, by id when the
                // declaration was resolved and by simple name otherwise, since
                // a symbolic `std::option` never compares equal to the
                // environment's `0x1::option`.
                let name = decl
                    .module_id
                    .map(|id| module.env.get_module(id))
                    .or_else(|| module.env.find_module_by_name(decl.module_name.name()))
                    .map(|resolved| resolved.get_name().clone())
                    .unwrap_or_else(|| decl.module_name.clone());
                XirUse {
                    // Not `expect_numerical`: a model built by the checker alone
                    // keeps named addresses symbolic, and `std::vector` resolves
                    // there exactly as written.
                    address: match name.addr() {
                        Address::Numerical(address) => address.to_hex_literal(),
                        Address::Symbolic(symbol) => pool.string(*symbol).to_string(),
                    },
                    module: pool.string(name.name()).to_string(),
                    alias: decl.alias.map(|alias| pool.string(alias).to_string()),
                    members: decl
                        .members
                        .iter()
                        .map(|(_, name, alias)| XirUseMember {
                            name: pool.string(*name).to_string(),
                            alias: alias.map(|alias| pool.string(alias).to_string()),
                        })
                        .collect(),
                }
            })
            .collect::<Vec<_>>()
    };

    let friends = module
        .get_friend_decls()
        .iter()
        .map(|decl| XirModuleRef {
            address: decl.module_name.addr().expect_numerical().to_hex_literal(),
            module: pool.string(decl.module_name.name()).to_string(),
        })
        .collect();

    Ok(XirModule {
        schema: XIR_SCHEMA.to_string(),
        version: XIR_VERSION,
        module: XirModuleMetadata {
            address: name.addr().expect_numerical().to_hex_literal(),
            name: pool.string(name.name()).to_string(),
            dialect: XirDialect::Stackless,
        },
        structs,
        functions,
        friends,
        constants,
        spec_declarations,
        uses,
        external_functions: refs.external_functions,
        external_structs: refs.external_structs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xir::{import_sources, parse_source_with_target};
    use move_model::model::GlobalEnv;
    use move_stackless_bytecode::function_target_pipeline::FunctionTargetsHolder;
    use std::path::PathBuf;

    /// Imports XIR documents as targets and returns the populated model.
    fn model_of(documents: &[serde_json::Value]) -> GlobalEnv {
        let sources = documents
            .iter()
            .map(|document| {
                parse_source_with_target(
                    PathBuf::from("test.xir.json"),
                    String::new(),
                    &document.to_string(),
                    true,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        import_sources(&mut env, &sources, &mut targets).unwrap();
        env
    }

    fn document(
        name: &str,
        structs: serde_json::Value,
        functions: serde_json::Value,
    ) -> serde_json::Value {
        serde_json::json!({
            "schema": XIR_SCHEMA,
            "version": XIR_VERSION,
            "module": {"address": "0x42", "name": name, "dialect": "stackless"},
            "friends": [{"address": "0x42", "module": "buddy"}],
            "structs": structs,
            "functions": functions,
        })
    }

    fn empty_body() -> serde_json::Value {
        serde_json::json!([{"instrs": [], "term": {"ret": []}}])
    }

    /// Exporting an interface and importing it again yields the same
    /// declarations. This is the exporter's real test: it exercises the
    /// producer and the reader against each other rather than against a
    /// hand-written expectation.
    #[test]
    fn interface_round_trips_through_the_reader() {
        let source = document(
            "m",
            serde_json::json!([{
                "name": "Holder", "visibility": "public",
                "abilities": ["copy", "drop", "store"],
                "type_parameters": [{"name": "T", "abilities": ["drop"], "phantom": false}],
                "fields": [{"name": "value", "ty": {"type_parameter": 0}},
                           {"name": "count", "ty": "i64"}],
            }]),
            serde_json::json!([{
                "name": "visible", "visibility": "public", "is_entry": true,
                "is_native": false, "acquires": [], "params": 2,
                "locals": [{"struct_inst": [0, ["u64"]]}, "i32", "bool"],
                "returns": ["bool"],
                "blocks": [{"instrs": [{"load": [2, {"bool": true}]}],
                            "term": {"ret": [2]}}],
                "entry": 0, "loops": [],
                "spec": {"requires": [], "modifies": [], "ensures": [], "aborts_if": []},
            }, {
                "name": "hidden", "visibility": "private", "is_entry": false,
                "is_native": false, "acquires": [], "params": 0,
                "locals": [], "returns": [], "blocks": empty_body(), "entry": 0,
                "loops": [],
                "spec": {"requires": [], "modifies": [], "ensures": [], "aborts_if": []},
            }]),
        );

        let env = model_of(&[source]);
        let module = env.get_modules().next().unwrap();
        let exported = export_interface(&module).unwrap();

        // Private functions are not part of the interface.
        assert_eq!(exported.functions.len(), 1);
        assert_eq!(exported.functions[0].name, "visible");
        assert!(exported.functions[0].is_entry);
        assert_eq!(exported.functions[0].params, 2);
        assert!(
            exported.functions[0].blocks.is_empty(),
            "an interface export carries no bodies"
        );
        assert_eq!(exported.functions[0].returns, vec![Ty::Bool]);
        // The signed width and the generic instantiation survive.
        assert_eq!(exported.functions[0].locals[1], Ty::I32);
        assert_eq!(
            exported.functions[0].locals[0],
            Ty::StructInst(0, vec![Ty::U64])
        );

        let exported_struct = &exported.structs[0];
        assert_eq!(exported_struct.name, "Holder");
        assert_eq!(exported_struct.visibility, XirVisibility::Public);
        assert_eq!(exported_struct.abilities, vec!["copy", "drop", "store"]);
        assert_eq!(exported_struct.type_parameters[0].name, "T");
        assert_eq!(exported_struct.fields[1].ty, Ty::I64);
        assert_eq!(exported.friends.len(), 1);

        // The export re-imports as a dependency, bodies absent and all.
        let round_tripped = parse_source_with_target(
            PathBuf::from("exported.xir.json"),
            String::new(),
            &serde_json::to_string(&exported).unwrap(),
            false,
        )
        .unwrap();
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        import_sources(&mut env, &[round_tripped], &mut targets).unwrap();
        assert_eq!(env.get_module_count(), 1);
        assert_eq!(
            targets.get_funs().count(),
            0,
            "an interface is a dependency, not a compilation target"
        );
    }

    /// Attributes survive an interface export. This is the reason an
    /// XIR-described dependency beats a `.mv` one: the binary loader never
    /// reads `CompiledModule::metadata`, so annotations are lost there.
    #[test]
    fn attributes_survive_the_interface_export() {
        let source = serde_json::json!({
            "schema": XIR_SCHEMA,
            "version": XIR_VERSION,
            "module": {"address": "0x42", "name": "m", "dialect": "stackless"},
            "structs": [{
                "name": "Registry", "visibility": "public", "abilities": ["key"],
                "fields": [{"name": "v", "ty": "u64"}],
                "attributes": [{"name": "resource_group", "args": [
                    {"name": "scope", "args": [{"name": "global", "args": []}]}
                ]}],
            }],
            "functions": [{
                "name": "act", "visibility": "public", "is_entry": true,
                "is_native": false, "acquires": [], "params": 0,
                "locals": [], "returns": [], "blocks": empty_body(), "entry": 0,
                "loops": [],
                "spec": {"requires": [], "modifies": [], "ensures": [], "aborts_if": []},
                "attributes": [{"name": "randomness", "args": [{"num": "7"}]},
                               {"name": "lint.skip", "args": []}],
            }],
        });

        let env = model_of(&[source]);
        let module = env.get_modules().next().unwrap();
        let exported = export_interface(&module).unwrap();

        assert_eq!(exported.structs[0].attributes.len(), 1);
        assert_eq!(exported.structs[0].attributes[0].name, "resource_group");
        assert_eq!(exported.structs[0].attributes[0].args, vec![
            XirAttributeArg::Name {
                name: "scope".to_owned(),
                args: vec![XirAttributeArg::Name {
                    name: "global".to_owned(),
                    args: vec![],
                }],
            }
        ]);

        let function_attributes = &exported.functions[0].attributes;
        assert_eq!(function_attributes.len(), 2);
        // A lone numeric argument decodes as an assignment and must come back
        // out in that same shape.
        assert_eq!(function_attributes[0].name, "randomness");
        assert_eq!(function_attributes[0].args, vec![XirAttributeArg::Num {
            value: "7".to_owned()
        }]);
        assert_eq!(function_attributes[1].name, "lint.skip");
        assert!(function_attributes[1].args.is_empty());
    }

    /// `#[resource_group_member(group = 0x1::object::ObjectGroup)]` and
    /// `#[resource_group(scope = global)]` — the two shapes the framework
    /// actually uses. Both must stay `Attribute::Assign` in the model, because
    /// the extended checker matches on that; reshaping either into an
    /// application would make the group membership invisible to it.
    #[test]
    fn assigned_names_round_trip_with_their_module_qualifier() {
        let attributes = serde_json::json!([
            {"name": "resource_group_member", "args": [
                {"assign": "group", "value": {"name": "0x1::object::ObjectGroup"}}
            ]},
            {"name": "resource_group", "args": [
                {"assign": "scope", "value": {"name": "global"}}
            ]},
        ]);
        let source = serde_json::json!({
            "schema": XIR_SCHEMA,
            "version": XIR_VERSION,
            "module": {"address": "0x42", "name": "m", "dialect": "stackless"},
            "structs": [{
                "name": "Member", "visibility": "public", "abilities": ["key"],
                "fields": [{"name": "v", "ty": "u64"}],
                "attributes": attributes.clone(),
            }],
            "functions": [],
        });

        let env = model_of(&[source]);
        let module = env.get_modules().next().unwrap();

        // The reader produced assignments, not applications.
        let struct_env = module.get_structs().next().unwrap();
        let pool = env.symbol_pool();
        let group = match &struct_env.get_attributes()[0] {
            Attribute::Apply(_, name, args) => {
                assert_eq!(pool.string(*name).as_str(), "resource_group_member");
                args[0].clone()
            },
            other => panic!("expected an application, got {other:?}"),
        };
        match &group {
            Attribute::Assign(_, name, AttributeValue::Name(_, Some(module_name), value)) => {
                assert_eq!(pool.string(*name).as_str(), "group");
                assert_eq!(module_name.display_full(&env).to_string(), "0x1::object");
                assert_eq!(pool.string(*value).as_str(), "ObjectGroup");
            },
            other => panic!("expected a module-qualified assignment, got {other:?}"),
        }

        // And exporting gives back exactly the shape we started from.
        let exported = export_interface(&module).unwrap();
        assert_eq!(
            serde_json::to_value(&exported.structs[0].attributes).unwrap(),
            attributes
        );
    }

    /// A positional struct whose one field is a function type — the shape of
    /// `sigma_protocol_homomorphism::Homomorphism`. Both parts were gaps: the
    /// field is named `0`, which is not an identifier, and a function type's
    /// argument list has arity two with an empty result.
    #[test]
    fn positional_fields_and_function_types_round_trip() {
        let structs = serde_json::json!([{
            "name": "Homomorphism", "visibility": "public", "abilities": ["copy", "drop"],
            "type_parameters": [{"name": "P", "phantom": true}],
            "fields": [{"name": "0", "ty": {"fun": [
                [{"ref": {"struct_inst": [1, [{"type_parameter": 0}]]}}, {"ref": {"struct": 2}}],
                [{"struct": 3}],
                []
            ]}}],
        }, {
            "name": "Statement", "visibility": "public", "abilities": ["drop"],
            "type_parameters": [{"name": "P", "phantom": true}],
            "fields": [{"name": "v", "ty": "u64"}],
        }, {
            "name": "Witness", "visibility": "public", "abilities": ["drop"],
            "fields": [{"name": "v", "ty": "u64"}],
        }, {
            "name": "RepresentationVec", "visibility": "public", "abilities": ["drop"],
            "fields": [{"name": "v", "ty": "u64"}],
        }]);
        let source = serde_json::json!({
            "schema": XIR_SCHEMA,
            "version": XIR_VERSION,
            "module": {"address": "0x42", "name": "m", "dialect": "stackless"},
            "structs": structs.clone(),
            "functions": [],
        });

        let env = model_of(&[source]);
        let module = env.get_modules().next().unwrap();

        // The model has a genuine function type, with the two-argument list
        // spelled as a tuple and the single result spelled bare.
        let struct_env = module.get_structs().next().unwrap();
        let field = struct_env.get_fields().next().unwrap();
        match field.get_type() {
            Type::Fun(args, result, _) => {
                assert!(matches!(*args, Type::Tuple(ref types) if types.len() == 2));
                assert!(matches!(*result, Type::Struct(..)));
            },
            other => panic!("expected a function type, got {other:?}"),
        }

        // Compared as values, not as JSON text: the two differ only in fields
        // serde always writes but the input may omit (`variants: null`).
        let exported = export_interface(&module).unwrap();
        let expected: Vec<XirStruct> = serde_json::from_value(structs).unwrap();
        assert_eq!(exported.structs, expected);
    }

    /// A type from another module is interned into `external_structs` and
    /// addressed past the local structs, so the export is self-contained.
    #[test]
    fn foreign_types_are_interned_as_external_references() {
        let provider = document(
            "provider",
            serde_json::json!([{
                "name": "Token", "visibility": "public", "abilities": ["drop"],
                "fields": [{"name": "v", "ty": "u64"}],
            }]),
            serde_json::json!([]),
        );
        let consumer = serde_json::json!({
            "schema": XIR_SCHEMA,
            "version": XIR_VERSION,
            "module": {"address": "0x42", "name": "consumer", "dialect": "stackless"},
            "structs": [{
                "name": "Local", "visibility": "public", "abilities": ["drop"],
                "fields": [{"name": "v", "ty": "u64"}],
            }],
            "functions": [{
                "name": "takes", "visibility": "public", "is_entry": false,
                "is_native": false, "acquires": [], "params": 1,
                // Resource id 1 is the first external type.
                "locals": [{"struct": 1}], "returns": [],
                "blocks": empty_body(), "entry": 0, "loops": [],
                "spec": {"requires": [], "modifies": [], "ensures": [], "aborts_if": []},
            }],
            "external_structs": [{"address": "0x42", "module": "provider", "name": "Token"}],
        });

        let env = model_of(&[provider, consumer]);
        let consumer_env = env
            .get_modules()
            .find(|module| {
                env.symbol_pool().string(module.get_name().name()).as_str() == "consumer"
            })
            .unwrap();
        let exported = export_interface(&consumer_env).unwrap();

        assert_eq!(exported.external_structs.len(), 1);
        assert_eq!(exported.external_structs[0].module, "provider");
        assert_eq!(exported.external_structs[0].name, "Token");
        // One local struct, so the external type is addressed at index 1.
        assert_eq!(exported.structs.len(), 1);
        assert_eq!(exported.functions[0].locals[0], Ty::Struct(1));
    }
}
