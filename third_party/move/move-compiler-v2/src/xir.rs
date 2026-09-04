// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! XIR support for compiler-v2.
//!
//! This reader registers a versioned deployable XIR module in the Move model
//! and translates its function bodies directly to baseline stackless bytecode.
//! Source frontends are responsible for producing the JSON; the ordinary
//! compiler-v2 stackless checks, optimizations, file-format generator, and
//! verifier own all later compilation stages.

mod typing;

use crate::env_pipeline::function_checker::call_access_error;
use anyhow::{bail, ensure, Context, Result};
use codespan::Span;
use move_binary_format::file_format::Visibility as MoveVisibility;
use move_command_line_common::files::FileHash;
use move_core_types::{
    ability::{Ability, AbilitySet},
    account_address::AccountAddress,
    function::ClosureMask,
    identifier::Identifier,
};
use move_model::{
    ast::{Address, Attribute, AttributeValue, FriendDecl, ModuleName, Value},
    metadata::lang_feature_versions::LANGUAGE_VERSION_FOR_PUBLIC_STRUCT,
    model::{
        FieldData, FunId, FunctionKind, GlobalEnv, Loc, ModuleId, Parameter, QualifiedId, StructId,
        TypeParameter, TypeParameterKind,
    },
    ty::{PrimitiveType, ReferenceKind, Type},
    well_known,
    xir_loader::{
        XirFunctionData as ModelXirFunctionData, XirModuleData as ModelXirModuleData,
        XirStructData as ModelXirStructData, XirVariantData as ModelXirVariantData,
    },
};
use move_model_exchange::{
    Block, Instr, IntType, Oper, Term, Type as Ty, TypeParameter as TypeParameterDecl,
    Value as Constant, XirAttribute, XirAttributeArg, XirDialect, XirFunction as FunctionDecl,
    XirModule, XirSourceSpan, XirStruct as StructDecl, XirVisibility,
};
use move_stackless_bytecode::{
    function_target::FunctionData as TargetFunctionData,
    function_target_pipeline::{FunctionTargetsHolder, FunctionVariant},
    stackless_bytecode::{
        AssignKind, AttrId, Bytecode, Constant as StacklessConstant, Label,
        Operation as StacklessOperation,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    rc::Rc,
};

const VECTOR_INDEX_OUT_OF_BOUNDS: u64 = 0x20000;

pub struct XirSource {
    path: PathBuf,
    text: String,
    module: XirModule,
    is_target: bool,
}

impl XirSource {
    /// The validated module this source parsed to.
    pub fn module(&self) -> &XirModule {
        &self.module
    }
}

/// Parses XIR to be *compiled*: every non-native function must have a body.
pub fn parse_source(path: PathBuf, text: String, json: &str) -> Result<XirSource> {
    parse_source_with_target(path, text, json, true)
}

/// Parses XIR that describes a *dependency* rather than a compilation target,
/// which is what [`crate::xir_export::export_interface`] produces: signatures,
/// types and attributes, with function bodies omitted.
///
/// Takes no source text, and so does not check source-map spans. An interface
/// is consumed for its declarations; any spans in it index a text this caller
/// does not hold, which makes them unusable rather than invalid. Checking them
/// against the empty string would reject every non-zero span — that is, every
/// source map a producer such as Lean emits for a non-native function.
pub fn parse_interface(path: PathBuf, json: &str) -> Result<XirSource> {
    parse_xir(path, String::new(), json, false, /*check_spans*/ false)
}

pub(crate) fn parse_source_with_target(
    path: PathBuf,
    text: String,
    json: &str,
    is_target: bool,
) -> Result<XirSource> {
    parse_xir(path, text, json, is_target, /*check_spans*/ true)
}

fn parse_xir(
    path: PathBuf,
    text: String,
    json: &str,
    is_target: bool,
    check_spans: bool,
) -> Result<XirSource> {
    let module: XirModule = serde_json::from_str(json)
        .with_context(|| format!("invalid XIR from `{}`", path.display()))?;
    validate(&module, is_target)?;
    if check_spans {
        validate_source_maps(&module, &text)?;
    }
    Ok(XirSource {
        path,
        text,
        module,
        is_target,
    })
}

fn validate_source_maps(module: &XirModule, source: &str) -> Result<()> {
    let validate_span = |span: XirSourceSpan, owner: &str| -> Result<()> {
        ensure!(
            span.start <= span.end
                && span.end as usize <= source.len()
                && source.is_char_boundary(span.start as usize)
                && source.is_char_boundary(span.end as usize),
            "source span {}..{} is outside the source text in {owner}",
            span.start,
            span.end
        );
        Ok(())
    };
    for function in &module.functions {
        let Some(source_map) = &function.source_map else {
            continue;
        };
        if let Some(span) = source_map.span {
            validate_span(span, &format!("function `{}`", function.name))?;
        }
        ensure!(
            source_map.blocks.len() == function.blocks.len(),
            "source map for function `{}` has {} blocks; expected {}",
            function.name,
            source_map.blocks.len(),
            function.blocks.len()
        );
        for (block_id, (block_map, block)) in
            source_map.blocks.iter().zip(&function.blocks).enumerate()
        {
            ensure!(
                block_map.instrs.len() == block.instrs.len(),
                "source map for function `{}`, block {block_id}, has {} instruction spans; expected {}",
                function.name,
                block_map.instrs.len(),
                block.instrs.len()
            );
            for span in block_map.instrs.iter().flatten() {
                validate_span(
                    *span,
                    &format!("function `{}`, block {block_id}", function.name),
                )?;
            }
            if let Some(span) = block_map.term {
                validate_span(
                    span,
                    &format!("function `{}`, block {block_id} terminator", function.name),
                )?;
            }
        }
    }
    Ok(())
}

fn loc_for_span(fallback: &Loc, span: Option<XirSourceSpan>) -> Loc {
    span.map(|span| Loc::new(fallback.file_id(), Span::new(span.start, span.end)))
        .unwrap_or_else(|| fallback.clone())
}

/// Validates a module. `is_target` distinguishes a module being *compiled*
/// from one supplied as a *dependency*: a dependency contributes only its
/// declaration surface, so its functions may legitimately carry no body.
fn validate(module: &XirModule, is_target: bool) -> Result<()> {
    module.check_version().map_err(anyhow::Error::msg)?;
    ensure!(
        module.module.dialect == XirDialect::Stackless,
        "only stackless XIR is deployable"
    );
    AccountAddress::from_hex_literal(&module.module.address)
        .with_context(|| format!("invalid module address `{}`", module.module.address))?;
    valid_identifier("module", &module.module.name)?;
    for friend in &module.friends {
        AccountAddress::from_hex_literal(&friend.address)
            .with_context(|| format!("invalid friend address `{}`", friend.address))?;
        valid_identifier("friend module", &friend.module)?;
    }
    let mut struct_names = BTreeSet::new();
    for decl in &module.structs {
        valid_identifier("struct", &decl.name)?;
        decl.abilities.iter().try_for_each(|a| valid_ability(a))?;
        decl.attributes.iter().try_for_each(valid_attribute)?;
        for param in &decl.type_parameters {
            valid_identifier("type parameter", &param.name)?;
            param.abilities.iter().try_for_each(|a| valid_ability(a))?;
        }
        ensure!(
            struct_names.insert(&decl.name),
            "duplicate struct `{}`",
            decl.name
        );
        let mut field_names = BTreeSet::new();
        for field in &decl.fields {
            valid_field_name(&field.name)?;
            ensure!(
                field_names.insert(&field.name),
                "duplicate field `{}` in struct `{}`",
                field.name,
                decl.name
            );
            validate_type_parameters(
                &field.ty,
                decl.type_parameters.len(),
                &format!("struct `{}`", decl.name),
            )?;
        }
        if let Some(variants) = &decl.variants {
            for variant in variants {
                valid_identifier("variant", &variant.name)?;
                for field in &variant.fields {
                    valid_field_name(&field.name)?;
                    validate_type_parameters(
                        &field.ty,
                        decl.type_parameters.len(),
                        &format!("enum `{}` variant `{}`", decl.name, variant.name),
                    )?;
                }
            }
        }
    }
    let mut function_names = BTreeSet::new();
    for decl in &module.functions {
        valid_identifier("function", &decl.name)?;
        decl.attributes.iter().try_for_each(valid_attribute)?;
        for param in &decl.type_parameters {
            valid_identifier("type parameter", &param.name)?;
            param.abilities.iter().try_for_each(|a| valid_ability(a))?;
        }
        ensure!(
            function_names.insert(&decl.name),
            "duplicate function `{}`",
            decl.name
        );
        ensure!(
            decl.params <= decl.locals.len(),
            "function `{}` has more parameters than locals",
            decl.name
        );
        // A compilation target must have code to compile; a dependency need
        // only declare its interface. Natives are bodyless in either role.
        if decl.blocks.is_empty() {
            ensure!(
                decl.is_native || !is_target,
                "function `{}` has no blocks",
                decl.name
            );
        } else {
            // Code that is present is translated, whatever the module's role.
            ensure!(
                decl.entry < decl.blocks.len(),
                "entry block is out of range in `{}`",
                decl.name
            );
        }
        ensure!(
            decl.blocks.len() <= u16::MAX as usize,
            "too many blocks in `{}`",
            decl.name
        );
        ensure!(
            decl.locals.len() <= u16::MAX as usize,
            "too many locals in `{}`",
            decl.name
        );
        ensure!(
            decl.local_names.is_empty() || decl.local_names.len() == decl.locals.len(),
            "function `{}` has {} local names; expected {}",
            decl.name,
            decl.local_names.len(),
            decl.locals.len()
        );
        // Any non-empty name: Lean producers write names such as `x'`. The
        // interface generator renders only names that are plain identifiers.
        for name in decl.local_names.iter().flatten() {
            ensure!(
                !name.is_empty(),
                "function `{}` has an empty local name",
                decl.name
            );
        }
        for ty in decl.locals.iter().chain(&decl.returns) {
            validate_type_parameters(
                ty,
                decl.type_parameters.len(),
                &format!("function `{}`", decl.name),
            )?;
        }
        for block in &decl.blocks {
            for instruction in &block.instrs {
                if let Instr::Call(_, operation, _) = instruction {
                    validate_operation_type_parameters(
                        operation,
                        decl.type_parameters.len(),
                        &decl.name,
                    )?;
                }
            }
        }
        ensure!(
            decl.loops.is_empty(),
            "loop metadata is not supported by the XIR reader yet in `{}`",
            decl.name
        );
        let _ = &decl.spec;
    }
    for reference in &module.external_functions {
        AccountAddress::from_hex_literal(&reference.address)
            .with_context(|| format!("invalid external module address `{}`", reference.address))?;
        valid_identifier("external module", &reference.module)?;
        valid_identifier("external function", &reference.function)?;
    }
    for reference in &module.external_structs {
        AccountAddress::from_hex_literal(&reference.address)
            .with_context(|| format!("invalid external module address `{}`", reference.address))?;
        valid_identifier("external module", &reference.module)?;
        valid_identifier("external struct", &reference.name)?;
    }
    Ok(())
}

fn validate_type_parameters(ty: &Ty, count: usize, owner: &str) -> Result<()> {
    match ty {
        Ty::TypeParameter(index) => ensure!(
            *index < count,
            "type parameter index {index} is out of range in {owner}"
        ),
        Ty::StructInst(_, args) | Ty::EnumInst(_, args) => {
            for arg in args {
                validate_type_parameters(arg, count, owner)?;
            }
        },
        Ty::Vector(element) | Ty::Ref(element) | Ty::MutRef(element) => {
            validate_type_parameters(element, count, owner)?;
        },
        Ty::Function(params, results, abilities) => {
            // The interface generator writes these into Move source.
            abilities
                .iter()
                .try_for_each(|ability| valid_ability(ability))?;
            ensure!(
                parse_ability_set(abilities)?.is_valid_for_function_type(),
                "a function type cannot have the abilities `{}`",
                abilities.join(", ")
            );
            for ty in params.iter().chain(results) {
                validate_type_parameters(ty, count, owner)?;
            }
        },
        Ty::Bool
        | Ty::U8
        | Ty::U16
        | Ty::U32
        | Ty::U64
        | Ty::U128
        | Ty::U256
        | Ty::I8
        | Ty::I16
        | Ty::I32
        | Ty::I64
        | Ty::I128
        | Ty::I256
        | Ty::Address
        | Ty::Signer
        | Ty::Struct(_)
        | Ty::Enum(_) => {},
    }
    Ok(())
}

fn validate_operation_type_parameters(
    operation: &Oper,
    count: usize,
    function: &str,
) -> Result<()> {
    for arg in operation.type_arguments() {
        validate_type_parameters(arg, count, &format!("function `{function}` operation"))?;
    }
    Ok(())
}

/// A field name is an identifier, or a decimal index for a field of a
/// positional struct — `struct Homomorphism<phantom P>(|&Statement<P>| Rep)`,
/// whose one field `move-model` names `0`. Consumers address fields by their
/// offset, so the name is documentation, but keeping the model's spelling
/// makes the round trip exact.
fn valid_field_name(name: &str) -> Result<()> {
    if !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_digit()) {
        return Ok(());
    }
    valid_identifier("field", name)
}

/// Every name below is rendered into generated Move source by
/// [`crate::xir_interface_generator`], so each must be a *single token* in the
/// position it lands in.
///
/// This is not hygiene. A type parameter named `T, U` is one string, so the
/// document is consistent by every other rule here, but it renders as two
/// type parameters. Local names are not checked here: the generator renders
/// a parameter's name only when it is a plain identifier.
fn valid_identifier(kind: &str, name: &str) -> Result<()> {
    Identifier::new(name)
        .map(|_| ())
        .with_context(|| format!("invalid {kind} identifier `{name}`"))
}

/// An ability is written into a `has` clause and a type parameter constraint,
/// so it is checked against the closed set rather than as an identifier.
fn valid_ability(name: &str) -> Result<()> {
    ensure!(
        matches!(name, "copy" | "drop" | "store" | "key"),
        "invalid ability `{name}`"
    );
    Ok(())
}

/// A dotted or `::`-qualified path, as attributes use — `lint.skip`,
/// `aptos_framework::object::ObjectGroup`. Every segment must be an
/// identifier, or an address where one is allowed.
fn valid_name_path(kind: &str, path: &str) -> Result<()> {
    for segment in path.split("::").flat_map(|part| part.split('.')) {
        if AccountAddress::from_hex_literal(segment).is_ok() {
            continue;
        }
        valid_identifier(kind, segment)?;
    }
    Ok(())
}

fn valid_attribute(attribute: &XirAttribute) -> Result<()> {
    valid_name_path("attribute", &attribute.name)?;
    attribute.args.iter().try_for_each(valid_attribute_arg)
}

fn valid_attribute_arg(arg: &XirAttributeArg) -> Result<()> {
    match arg {
        XirAttributeArg::Name { name, args } => {
            valid_name_path("attribute argument", name)?;
            args.iter().try_for_each(valid_attribute_arg)?;
        },
        XirAttributeArg::Assign { assign, value } => {
            valid_identifier("attribute argument", assign)?;
            valid_attribute_arg(value)?;
        },
        // Rendered verbatim, so a value carrying punctuation would escape the
        // argument list the same way a name would.
        XirAttributeArg::Num { value } => ensure!(
            !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()),
            "invalid numeric attribute argument `{value}`"
        ),
        XirAttributeArg::Bool { .. } => {},
    }
    Ok(())
}

pub fn import_sources(
    env: &mut GlobalEnv,
    sources: &[XirSource],
    targets: &mut FunctionTargetsHolder,
) -> Result<()> {
    import_source_refs(env, &sources.iter().collect::<Vec<_>>(), targets)
}

fn import_source_refs(
    env: &mut GlobalEnv,
    sources: &[&XirSource],
    targets: &mut FunctionTargetsHolder,
) -> Result<()> {
    if sources.is_empty() {
        return Ok(());
    }
    let mut imported = vec![false; sources.len()];
    let mut imported_count = 0;
    while imported_count < sources.len() {
        let ready = sources.iter().enumerate().find_map(|(index, source)| {
            (!imported[index] && external_modules_available(env, &source.module)).then_some(index)
        });
        let Some(index) = ready else {
            let blocked = sources
                .iter()
                .enumerate()
                .filter(|(index, _)| !imported[*index])
                .map(|(_, source)| source.module.module.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            bail!("unresolved or cyclic XIR module dependencies: {blocked}")
        };
        let source = sources[index];
        import_source(env, source, targets)
            .with_context(|| format!("loading XIR from `{}`", source.path.display()))?;
        imported[index] = true;
        imported_count += 1;
    }
    env.set_function_size_estimates(targets.compute_function_size_estimates());
    Ok(())
}

/// Whether every module this one references — for calls *and* for types — is
/// already loaded, so import ordering can pick a module that is ready.
fn external_modules_available(env: &GlobalEnv, xir: &XirModule) -> bool {
    let available = |address: &str, module: &str| {
        let Ok(address) = AccountAddress::from_hex_literal(address) else {
            return false;
        };
        let name = ModuleName::new(Address::Numerical(address), env.symbol_pool().make(module));
        env.find_module(&name).is_some()
    };
    xir.external_functions
        .iter()
        .all(|reference| available(&reference.address, &reference.module))
        && xir
            .external_structs
            .iter()
            .all(|reference| available(&reference.address, &reference.module))
}

/// Resolve the external struct table against the loaded modules, in table
/// order, so struct ids beyond the local table index it.
fn external_structs(env: &GlobalEnv, xir: &XirModule) -> Result<Vec<QualifiedId<StructId>>> {
    xir.external_structs
        .iter()
        .map(|reference| {
            let address = AccountAddress::from_hex_literal(&reference.address)?;
            let module_name = ModuleName::new(
                Address::Numerical(address),
                env.symbol_pool().make(&reference.module),
            );
            let module = env.find_module(&module_name).with_context(|| {
                format!(
                    "external module `{}::{}` is not loaded",
                    reference.address, reference.module
                )
            })?;
            let struct_env = module
                .find_struct(env.symbol_pool().make(&reference.name))
                .with_context(|| {
                    format!(
                        "external module `{}::{}` has no struct `{}`",
                        reference.address, reference.module, reference.name
                    )
                })?;
            Ok(module.get_id().qualified(struct_env.get_id()))
        })
        .collect()
}

/// The number of type parameters of each struct in a module's scope, in
/// [`StructScope`] order.
fn struct_arities(
    env: &GlobalEnv,
    xir: &XirModule,
    external: &[QualifiedId<StructId>],
) -> Vec<usize> {
    xir.structs
        .iter()
        .map(|decl| decl.type_parameters.len())
        .chain(
            external
                .iter()
                .map(|qid| env.get_struct(*qid).get_type_parameters().len()),
        )
        .collect()
}

/// The structs a module's types may mention: its own, then the external
/// ones, in table order.
struct StructScope<'a> {
    module_id: ModuleId,
    local: &'a [StructId],
    external: &'a [QualifiedId<StructId>],
    arities: &'a [usize],
}

impl StructScope<'_> {
    /// Resolves a struct type with `args` type arguments, which must match
    /// the struct's type parameters. Whether the document says struct or
    /// enum is not checked: the model has one type for both.
    fn resolve(&self, id: usize, kind: &str, args: usize) -> Result<(ModuleId, StructId)> {
        let resolved = if let Some(struct_id) = self.local.get(id) {
            (self.module_id, *struct_id)
        } else {
            let external_id = id
                .checked_sub(self.local.len())
                .with_context(|| format!("{kind} id underflow"))?;
            self.external
                .get(external_id)
                .map(|qid| (qid.module_id, qid.id))
                .with_context(|| {
                    format!("{kind} id {id} is outside the local and external struct tables")
                })?
        };
        let expected = self.arities[id];
        ensure!(
            args == expected,
            "{kind} id {id} takes {expected} type arguments, but {args} are given"
        );
        Ok(resolved)
    }
}

fn model_attribute(env: &mut GlobalEnv, loc: &Loc, attribute: &XirAttribute) -> Result<Attribute> {
    model_attribute_apply(env, loc, &attribute.name, &attribute.args)
}

fn model_attribute_apply(
    env: &mut GlobalEnv,
    loc: &Loc,
    name: &str,
    args: &[XirAttributeArg],
) -> Result<Attribute> {
    let node_id = env.new_node(loc.clone(), Type::Tuple(vec![]));
    let symbol = env.symbol_pool().make(name);
    match args {
        [XirAttributeArg::Num { value }] => Ok(Attribute::Assign(
            node_id,
            symbol,
            AttributeValue::Value(node_id, Value::Number(value.parse()?)),
        )),
        [XirAttributeArg::Bool { value }] => Ok(Attribute::Assign(
            node_id,
            symbol,
            AttributeValue::Value(node_id, Value::Bool(*value)),
        )),
        _ => Ok(Attribute::Apply(
            node_id,
            symbol,
            args.iter()
                .map(|arg| match arg {
                    XirAttributeArg::Name { name, args } => {
                        model_attribute_apply(env, loc, name, args)
                    },
                    XirAttributeArg::Assign { assign, value } => {
                        let symbol = env.symbol_pool().make(assign);
                        let value = model_attribute_value(env, loc, assign, value)?;
                        Ok(Attribute::Assign(
                            env.new_node(loc.clone(), Type::Tuple(vec![])),
                            symbol,
                            value,
                        ))
                    },
                    XirAttributeArg::Num { .. } | XirAttributeArg::Bool { .. } => {
                        bail!("attribute `{name}` has an unnamed literal argument")
                    },
                })
                .collect::<Result<Vec<_>>>()?,
        )),
    }
}

/// Translates the right-hand side of an attribute assignment. Inverse of
/// `xir_export::export_attribute_value`: a name path is split at its last
/// `::` back into an optional module qualifier and a symbol.
fn model_attribute_value(
    env: &mut GlobalEnv,
    loc: &Loc,
    name: &str,
    value: &XirAttributeArg,
) -> Result<AttributeValue> {
    let node_id = env.new_node(loc.clone(), Type::Tuple(vec![]));
    match value {
        XirAttributeArg::Num { value } => Ok(AttributeValue::Value(
            node_id,
            Value::Number(value.parse()?),
        )),
        XirAttributeArg::Bool { value } => Ok(AttributeValue::Value(node_id, Value::Bool(*value))),
        XirAttributeArg::Name { name: path, args } => {
            ensure!(
                args.is_empty(),
                "attribute `{name}` is assigned `{path}`, which cannot take arguments"
            );
            let (module_name, symbol) = match path.rfind("::") {
                Some(split) => {
                    let (module_path, symbol) = (&path[..split], &path[split + 2..]);
                    let split = module_path.rfind("::").with_context(|| {
                        format!(
                            "attribute `{name}` is assigned `{path}`, which is not a \
                                 module-qualified name"
                        )
                    })?;
                    let address = AccountAddress::from_hex_literal(&module_path[..split])
                        .with_context(|| format!("invalid address in attribute value `{path}`"))?;
                    let module_name = ModuleName::new(
                        Address::Numerical(address),
                        env.symbol_pool().make(&module_path[split + 2..]),
                    );
                    (Some(module_name), symbol)
                },
                None => (None, path.as_str()),
            };
            let symbol = env.symbol_pool().make(symbol);
            Ok(AttributeValue::Name(node_id, module_name, symbol))
        },
        XirAttributeArg::Assign { assign, .. } => {
            bail!("attribute `{name}` is assigned the assignment `{assign}`, which has no meaning")
        },
    }
}

fn import_source(
    env: &mut GlobalEnv,
    source: &XirSource,
    targets: &mut FunctionTargetsHolder,
) -> Result<ModuleId> {
    let xir = &source.module;
    let address = AccountAddress::from_hex_literal(&xir.module.address)?;
    let module_symbol = env.symbol_pool().make(&xir.module.name);
    let module_name = ModuleName::new(Address::Numerical(address), module_symbol);
    ensure!(
        env.find_module(&module_name).is_none(),
        "duplicate module `{}::{}`",
        xir.module.address,
        xir.module.name
    );
    // Move source drops test-only items from a build without test code, and
    // verify-only items from one without verification code; the file-format
    // generator asserts the first and silently publishes the second. XIR cannot
    // drop them. Without compiler options there is no code generation.
    if let Some(options) = env.get_extension::<crate::Options>() {
        let modes: [(&str, fn(&str) -> bool, bool, &str); 2] = [
            (
                "test-only",
                well_known::is_test_only_attribute_name,
                options.compile_test_code,
                "test code",
            ),
            (
                "verify-only",
                well_known::is_verify_only_attribute_name,
                options.compile_verify_code,
                "verification code",
            ),
        ];
        for (marker, is_marked, included, code) in modes {
            if included {
                continue;
            }
            let items = xir
                .structs
                .iter()
                .map(|decl| ("struct", &decl.name, &decl.attributes))
                .chain(
                    xir.functions
                        .iter()
                        .map(|decl| ("function", &decl.name, &decl.attributes)),
                );
            for (kind, name, attributes) in items {
                ensure!(
                    !attributes
                        .iter()
                        .any(|attribute| is_marked(&attribute.name)),
                    "{kind} `{name}` is {marker}, which a build without {code} cannot include"
                );
            }
        }
    }
    let file_id = env.add_source(
        FileHash::new(&source.text),
        Rc::new(BTreeMap::new()),
        &source.path.to_string_lossy(),
        &source.text,
        source.is_target,
        source.is_target,
    );
    let end = u32::try_from(source.text.len()).unwrap_or(u32::MAX);
    let loc = Loc::new(file_id, Span::new(0, end));
    let module_id = ModuleId::new(env.get_module_count());
    let struct_ids = xir
        .structs
        .iter()
        .map(|decl| StructId::new(env.symbol_pool().make(&decl.name)))
        .collect::<Vec<_>>();
    let function_ids = xir
        .functions
        .iter()
        .map(|decl| FunId::new(env.symbol_pool().make(&decl.name)))
        .collect::<Vec<_>>();
    let external_struct_ids = external_structs(env, xir)?;
    let arities = struct_arities(env, xir, &external_struct_ids);
    let scope = StructScope {
        module_id,
        local: &struct_ids,
        external: &external_struct_ids,
        arities: &arities,
    };

    let mut structs = vec![];
    for (decl, struct_id) in xir.structs.iter().zip(&struct_ids) {
        let mut fields = vec![];
        for (offset, field) in decl.fields.iter().enumerate() {
            let field_symbol = env.symbol_pool().make(&field.name);
            fields.push(FieldData {
                name: field_symbol,
                loc: loc.clone(),
                offset,
                variant: None,
                ty: model_value_type(&field.ty, &scope)
                    .with_context(|| format!("field `{}` of `{}`", field.name, decl.name))?,
                is_ghost: false,
                init: None,
            });
        }
        let variants = decl.variants.as_ref().map(|variants| {
            variants
                .iter()
                .map(|variant| ModelXirVariantData {
                    name: env.symbol_pool().make(&variant.name),
                    loc: loc.clone(),
                })
                .collect::<Vec<_>>()
        });
        if let Some(variants) = &decl.variants {
            for variant in variants {
                let variant_symbol = env.symbol_pool().make(&variant.name);
                for (offset, field) in variant.fields.iter().enumerate() {
                    fields.push(FieldData {
                        name: env.symbol_pool().make(&field.name),
                        loc: loc.clone(),
                        offset,
                        variant: Some(variant_symbol),
                        ty: model_value_type(&field.ty, &scope).with_context(|| {
                            format!("field `{}` of `{}`", field.name, decl.name)
                        })?,
                        is_ghost: false,
                        init: None,
                    });
                }
            }
        }
        // The model's `Attribute` cannot hold a literal inside an argument
        // list, which Lean's positional grammar allows; warn and skip it.
        let mut attributes = vec![];
        for attribute in &decl.attributes {
            match model_attribute(env, &loc, attribute) {
                Ok(attribute) => attributes.push(attribute),
                Err(error) => env.warning(
                    &loc,
                    &format!(
                        "attribute `{}` on struct `{}` is not carried: {error:#}",
                        attribute.name, decl.name
                    ),
                ),
            }
        }
        // The rules the source compiler applies to a struct's visibility: below
        // the version that has it, a warning, and a resource cannot have it.
        let abilities = ability_set(decl)?;
        let visibility = move_visibility(&decl.visibility);
        if visibility != MoveVisibility::Private {
            if !env.language_version().language_version_for_public_struct() {
                env.warning(
                    &loc,
                    &format!(
                        "structs/enums with visibility modifier are only supported at version {} or later",
                        LANGUAGE_VERSION_FOR_PUBLIC_STRUCT
                    ),
                );
            } else {
                ensure!(
                    !abilities.has_ability(Ability::Key),
                    "struct `{}`: structs/enums with key ability cannot have public, package or friend visibility",
                    decl.name
                );
            }
        }
        structs.push(ModelXirStructData {
            name: struct_id.symbol(),
            loc: loc.clone(),
            abilities,
            type_parameters: model_type_parameters(env, &loc, &decl.type_parameters)?,
            fields,
            variants,
            visibility,
            attributes,
        });
    }

    let mut functions = vec![];
    for (decl, fun_id) in xir.functions.iter().zip(&function_ids) {
        let function_loc = loc_for_span(&loc, decl.source_map.as_ref().and_then(|map| map.span));
        let local_types = decl
            .locals
            .iter()
            .enumerate()
            .map(|(id, ty)| {
                model_type(ty, &scope).with_context(|| format!("local l{id} of `{}`", decl.name))
            })
            .collect::<Result<Vec<_>>>()?;
        let params = local_types
            .iter()
            .take(decl.params)
            .enumerate()
            .map(|(index, ty)| {
                let name = decl
                    .local_names
                    .get(index)
                    .and_then(Option::as_deref)
                    .map(String::from)
                    .unwrap_or_else(|| format!("p{index}"));
                Parameter(
                    env.symbol_pool().make(&name),
                    ty.clone(),
                    function_loc.clone(),
                )
            })
            .collect();
        let returns = model_tuple(&decl.returns, &scope)
            .with_context(|| format!("return type of `{}`", decl.name))?;
        let acquired = decl
            .acquires
            .iter()
            .map(|id| struct_at(&struct_ids, *id, &decl.name))
            .collect::<Result<BTreeSet<_>>>()?;
        let (used, called) = used_functions(env, xir, decl, module_id, &function_ids)?;
        functions.push(ModelXirFunctionData {
            name: fun_id.symbol(),
            loc: function_loc.clone(),
            visibility: move_visibility(&decl.visibility),
            is_native: decl.is_native,
            kind: if decl.is_entry {
                FunctionKind::Entry
            } else {
                FunctionKind::Regular
            },
            attributes: decl
                .attributes
                .iter()
                .map(|attribute| model_attribute(env, &function_loc, attribute))
                .collect::<Result<Vec<_>>>()?,
            type_parameters: model_type_parameters(env, &function_loc, &decl.type_parameters)?,
            params,
            result_type: returns,
            acquired_structs: acquired,
            used_funs: used,
            called_funs: called,
        });
    }

    // Friend targets need not be loaded: a module may grant access to one that
    // is absent from this compilation, so resolve the id where possible and
    // keep the name either way.
    let friends = xir
        .friends
        .iter()
        .map(|reference| {
            let address =
                AccountAddress::from_hex_literal(&reference.address).with_context(|| {
                    format!("invalid friend module address `{}`", reference.address)
                })?;
            valid_identifier("friend module", &reference.module)?;
            let name = ModuleName::new(
                Address::Numerical(address),
                env.symbol_pool().make(&reference.module),
            );
            let module_id = env.find_module(&name).map(|module| module.get_id());
            Ok(FriendDecl {
                loc: loc.clone(),
                module_name: name,
                module_id,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let added_id = env.load_xir_module(ModelXirModuleData {
        loc,
        name: module_name,
        structs,
        functions,
        friends,
    })?;
    ensure!(
        added_id == module_id,
        "model assigned an unexpected module id"
    );
    env.add_package_friends(module_id);
    // A friend usually calls the module that grants it access, so it loads
    // after it. Resolve those grants now, before translation checks its calls.
    let loaded = env
        .get_modules()
        .map(|module| module.get_id())
        .collect::<Vec<_>>();
    env.resolve_xir_friend_declarations(&loaded);
    for (decl, fun_id) in xir.functions.iter().zip(&function_ids) {
        // An interface-only declaration carries no body, so there is nothing
        // to translate and no target to create. Gate on the body rather than
        // on target status: a `.lean` dependency is a real module whose bodies
        // downstream whole-program analyses still follow.
        //
        // Natives are bodyless too but keep their (empty) target, as they did
        // before interfaces existed.
        if decl.blocks.is_empty() && !decl.is_native {
            continue;
        }
        let qid = module_id.qualified(*fun_id);
        let data = translate_function(
            env,
            xir,
            module_id,
            &struct_ids,
            &external_struct_ids,
            &function_ids,
            decl,
            qid,
        )?;
        // The call graph is what the translated code uses and calls, including
        // the calls it lowers operations to; a closure's target is used
        // without being called.
        let mut used = BTreeSet::new();
        let mut called = BTreeSet::new();
        for bytecode in &data.code {
            match bytecode {
                Bytecode::Call(_, _, StacklessOperation::Function(module, fun, _), _, _) => {
                    called.insert(module.qualified(*fun));
                },
                Bytecode::Call(_, _, StacklessOperation::Closure(module, fun, _, _), _, _) => {
                    used.insert(module.qualified(*fun));
                },
                _ => {},
            }
        }
        used.extend(called.iter().copied());
        env.set_xir_used_functions(qid, used, called);
        targets.insert_target_data(&qid, FunctionVariant::Baseline, data);
    }
    add_transitive_callee_targets(env, module_id, targets);
    Ok(module_id)
}

/// XIR is imported after the ordinary Move stackless-bytecode generation pass.
/// Add library callees discovered in XIR to the target holder so subsequent
/// whole-program analyses see the same transitive call graph as for Move source.
fn add_transitive_callee_targets(
    env: &GlobalEnv,
    module_id: ModuleId,
    targets: &mut FunctionTargetsHolder,
) {
    let mut todo = env
        .get_module(module_id)
        .get_functions()
        .flat_map(|function| {
            function
                .get_used_functions()
                .expect("XIR call information is available")
                .clone()
                .into_iter()
        })
        .collect::<BTreeSet<_>>();
    let mut done = targets.get_funs().collect::<BTreeSet<_>>();

    while let Some(id) = todo.pop_first() {
        if !done.insert(id) {
            continue;
        }
        let function = env.get_function(id);
        if function.is_excluded_from_bytecode_gen() {
            continue;
        }
        // A callee reached here is not already a target. If it also has no AST
        // definition and is not native, it came from a declaration-only
        // dependency: there is no body to generate, and synthesizing an empty
        // one would produce a target whose CFG cannot be built.
        if function.get_def().is_none() && !function.is_native() {
            continue;
        }
        let data = crate::bytecode_generator::generate_bytecode(env, id);
        targets.insert_target_data(&id, FunctionVariant::Baseline, data);
        todo.extend(
            function
                .get_used_functions()
                .expect("called function information is available")
                .iter()
                .filter(|callee| !done.contains(callee))
                .copied(),
        );
    }
}

fn ability_set(decl: &StructDecl) -> Result<AbilitySet> {
    parse_ability_set(&decl.abilities).with_context(|| format!("on struct `{}`", decl.name))
}

fn parse_ability_set(abilities: &[String]) -> Result<AbilitySet> {
    let mut result = AbilitySet::EMPTY;
    for ability in abilities {
        result = result.add(match ability.as_str() {
            "copy" => Ability::Copy,
            "drop" => Ability::Drop,
            "store" => Ability::Store,
            "key" => Ability::Key,
            _ => bail!("unknown ability `{ability}`"),
        });
    }
    Ok(result)
}

fn model_type_parameters(
    env: &GlobalEnv,
    loc: &Loc,
    params: &[TypeParameterDecl],
) -> Result<Vec<TypeParameter>> {
    params
        .iter()
        .map(|param| {
            let abilities = parse_ability_set(&param.abilities)
                .with_context(|| format!("on type parameter `{}`", param.name))?;
            let kind = if param.phantom {
                TypeParameterKind::new_phantom(abilities)
            } else {
                TypeParameterKind::new(abilities)
            };
            Ok(TypeParameter(
                env.symbol_pool().make(&param.name),
                kind,
                loc.clone(),
            ))
        })
        .collect()
}

fn move_visibility(visibility: &XirVisibility) -> MoveVisibility {
    match visibility {
        XirVisibility::Private => MoveVisibility::Private,
        XirVisibility::Public => MoveVisibility::Public,
        XirVisibility::Friend => MoveVisibility::Friend,
    }
}

fn model_type(ty: &Ty, scope: &StructScope) -> Result<Type> {
    Ok(match ty {
        Ty::Bool => Type::Primitive(PrimitiveType::Bool),
        Ty::U8 => Type::Primitive(PrimitiveType::U8),
        Ty::U16 => Type::Primitive(PrimitiveType::U16),
        Ty::U32 => Type::Primitive(PrimitiveType::U32),
        Ty::U64 => Type::Primitive(PrimitiveType::U64),
        Ty::U128 => Type::Primitive(PrimitiveType::U128),
        Ty::U256 => Type::Primitive(PrimitiveType::U256),
        Ty::I8 => Type::Primitive(PrimitiveType::I8),
        Ty::I16 => Type::Primitive(PrimitiveType::I16),
        Ty::I32 => Type::Primitive(PrimitiveType::I32),
        Ty::I64 => Type::Primitive(PrimitiveType::I64),
        Ty::I128 => Type::Primitive(PrimitiveType::I128),
        Ty::I256 => Type::Primitive(PrimitiveType::I256),
        Ty::Address => Type::Primitive(PrimitiveType::Address),
        Ty::Signer => Type::Primitive(PrimitiveType::Signer),
        Ty::TypeParameter(index) => {
            ensure!(
                *index <= u16::MAX as usize,
                "type parameter index is too large"
            );
            Type::TypeParameter(*index as u16)
        },
        Ty::Struct(id) => {
            let (module_id, struct_id) = scope.resolve(*id, "struct", 0)?;
            Type::Struct(module_id, struct_id, vec![])
        },
        Ty::StructInst(id, args) => {
            let (module_id, struct_id) = scope.resolve(*id, "struct", args.len())?;
            Type::Struct(
                module_id,
                struct_id,
                args.iter()
                    .map(|arg| model_value_type(arg, scope))
                    .collect::<Result<Vec<_>>>()?,
            )
        },
        Ty::Enum(id) => {
            let (module_id, struct_id) = scope.resolve(*id, "enum", 0)?;
            Type::Struct(module_id, struct_id, vec![])
        },
        Ty::EnumInst(id, args) => {
            let (module_id, struct_id) = scope.resolve(*id, "enum", args.len())?;
            Type::Struct(
                module_id,
                struct_id,
                args.iter()
                    .map(|arg| model_value_type(arg, scope))
                    .collect::<Result<Vec<_>>>()?,
            )
        },
        Ty::Vector(element) => Type::Vector(Box::new(model_value_type(element, scope)?)),
        Ty::Ref(referent) => Type::Reference(
            ReferenceKind::Immutable,
            Box::new(model_value_type(referent, scope)?),
        ),
        Ty::MutRef(referent) => Type::Reference(
            ReferenceKind::Mutable,
            Box::new(model_value_type(referent, scope)?),
        ),
        Ty::Function(params, results, abilities) => Type::function(
            model_tuple(params, scope)?,
            model_tuple(results, scope)?,
            parse_ability_set(abilities).context("on a function type")?,
        ),
    })
}

/// A type that cannot be a reference: a field, or a type nested in another.
/// Move has references only as the type of a local, a return value, or a
/// function type's parameter or result.
fn model_value_type(ty: &Ty, scope: &StructScope) -> Result<Type> {
    let model = model_type(ty, scope)?;
    ensure!(
        !model.is_reference(),
        "`{ty:?}` is a reference, which cannot be a field or nested in a type"
    );
    Ok(model)
}

/// The types of a function's or function type's parameters or results, as a
/// tuple; each may be a reference.
fn model_tuple(types: &[Ty], scope: &StructScope) -> Result<Type> {
    Ok(Type::tuple(
        types
            .iter()
            .map(|ty| model_type(ty, scope))
            .collect::<Result<Vec<_>>>()?,
    ))
}

fn function_at(
    env: &GlobalEnv,
    xir: &XirModule,
    module_id: ModuleId,
    functions: &[FunId],
    id: usize,
) -> Result<QualifiedId<FunId>> {
    if let Some(fun_id) = functions.get(id) {
        return Ok(module_id.qualified(*fun_id));
    }
    let external_id = id
        .checked_sub(functions.len())
        .context("external function id underflow")?;
    let reference = xir.external_functions.get(external_id).with_context(|| {
        format!("function id {id} is outside the local and external function tables")
    })?;
    let address = AccountAddress::from_hex_literal(&reference.address)?;
    let module_name = ModuleName::new(
        Address::Numerical(address),
        env.symbol_pool().make(&reference.module),
    );
    let module = env.find_module(&module_name).with_context(|| {
        format!(
            "external module `{}::{}` is not loaded",
            reference.address, reference.module
        )
    })?;
    let function = module
        .find_function(env.symbol_pool().make(&reference.function))
        .with_context(|| {
            format!(
                "external module `{}::{}` has no function `{}`",
                reference.address, reference.module, reference.function
            )
        })?;
    Ok(module.get_id().qualified(function.get_id()))
}

/// Records interface functions' callees in the model.
///
/// An interface reaches the front end as generated source declaring every
/// function `native`, so no calls are derived from it. Analyses that walk the
/// call graph across a dependency — Aptos rejects a public function that can
/// reach `0x1::randomness` that way — would otherwise stop at the boundary.
pub fn apply_interface_call_graphs(env: &mut GlobalEnv, modules: &[XirModule]) -> Result<()> {
    for xir in modules {
        let Some(module_id) = interface_module_id(env, &xir.module.address, &xir.module.name)
        else {
            continue;
        };
        for decl in &xir.functions {
            let called = decl.called_ids();
            if called.is_empty() {
                continue;
            }
            let fun_id = interface_fun_id(env, module_id, &decl.name).with_context(|| {
                format!("`{}` is missing from the generated interface", decl.name)
            })?;
            let callees = called
                .iter()
                .map(|id| interface_callee(env, xir, module_id, *id))
                .collect::<Result<BTreeSet<_>>>()
                .with_context(|| format!("resolving the callees of `{}`", decl.name))?;
            // An interface records calls only, so it uses what it calls.
            env.set_xir_used_functions(module_id.qualified(fun_id), callees.clone(), callees);
        }
    }
    Ok(())
}

fn interface_module_id(env: &GlobalEnv, address: &str, module: &str) -> Option<ModuleId> {
    let address = AccountAddress::from_hex_literal(address).ok()?;
    let name = ModuleName::new(Address::Numerical(address), env.symbol_pool().make(module));
    Some(env.find_module(&name)?.get_id())
}

fn interface_fun_id(env: &GlobalEnv, module_id: ModuleId, name: &str) -> Option<FunId> {
    env.get_module(module_id)
        .find_function(env.symbol_pool().make(name))
        .map(|fun| fun.get_id())
}

/// Resolves a call id under the local-then-`external_functions` convention.
///
/// Every recorded edge must resolve. Dropping one is not neutral: analyses
/// follow this graph to decide things like whether a public function can reach
/// `0x1::randomness`, and the *deployed* dependency still makes the call
/// whatever this build happens to have loaded. A missing edge therefore yields
/// a weaker answer than the source build gives, silently.
///
/// An absent callee module means the build does not contain the closure this
/// interface names — the package system supplies it via transitive
/// dependencies — so it is reported rather than skipped.
fn interface_callee(
    env: &GlobalEnv,
    xir: &XirModule,
    module_id: ModuleId,
    id: usize,
) -> Result<QualifiedId<FunId>> {
    let (owner, name) = match xir.functions.get(id) {
        Some(decl) => (module_id, decl.name.clone()),
        None => {
            let reference = xir
                .external_functions
                .get(id - xir.functions.len())
                .with_context(|| format!("call id {id} is outside the function tables"))?;
            let owner = interface_module_id(env, &reference.address, &reference.module)
                .with_context(|| {
                    format!(
                        "`{}::{}` is called by this interface but is not in the build; \
                         supply its interface too (loaded: {:?})",
                        reference.address,
                        reference.module,
                        env.get_modules()
                            .map(|module| module.get_full_name_str())
                            .take(8)
                            .collect::<Vec<_>>()
                    )
                })?;
            (owner, reference.function.clone())
        },
    };
    let fun_id = interface_fun_id(env, owner, &name).with_context(|| {
        format!(
            "`{name}` is missing from `{}`, which is loaded: the interface and the module disagree",
            env.get_module(owner).get_full_name_str()
        )
    })?;
    Ok(owner.qualified(fun_id))
}

/// The functions a declaration uses and, among them, those it calls; a
/// closure's target is used without being called.
fn used_functions(
    env: &GlobalEnv,
    xir: &XirModule,
    decl: &FunctionDecl,
    module_id: ModuleId,
    functions: &[FunId],
) -> Result<(BTreeSet<QualifiedId<FunId>>, BTreeSet<QualifiedId<FunId>>)> {
    // The explicit uses; the translated code adds the calls it lowers to.
    let mut used = BTreeSet::new();
    let mut called = BTreeSet::new();
    // An interface has no blocks and records its call graph explicitly.
    for id in &decl.calls {
        called.insert(function_at(env, xir, module_id, functions, *id)?);
    }
    for block in &decl.blocks {
        for instr in &block.instrs {
            match instr {
                Instr::Call(_, Oper::Function(id) | Oper::FunctionInst(id, _), _) => {
                    called.insert(function_at(env, xir, module_id, functions, *id)?);
                },
                Instr::Call(_, Oper::Closure(id, _) | Oper::ClosureInst(id, _, _), _) => {
                    used.insert(function_at(env, xir, module_id, functions, *id)?);
                },
                _ => {},
            }
        }
    }
    used.extend(called.iter().copied());
    Ok((used, called))
}

/// The XIR width of a model type, if it is a Move integer.
fn int_type_of(ty: &Type) -> Option<IntType> {
    let Type::Primitive(primitive) = ty else {
        return None;
    };
    match primitive {
        PrimitiveType::U8 => Some(IntType::U8),
        PrimitiveType::U16 => Some(IntType::U16),
        PrimitiveType::U32 => Some(IntType::U32),
        PrimitiveType::U64 => Some(IntType::U64),
        PrimitiveType::U128 => Some(IntType::U128),
        PrimitiveType::U256 => Some(IntType::U256),
        PrimitiveType::I8 => Some(IntType::I8),
        PrimitiveType::I16 => Some(IntType::I16),
        PrimitiveType::I32 => Some(IntType::I32),
        PrimitiveType::I64 => Some(IntType::I64),
        PrimitiveType::I128 => Some(IntType::I128),
        PrimitiveType::I256 => Some(IntType::I256),
        PrimitiveType::Bool
        | PrimitiveType::Address
        | PrimitiveType::Signer
        | PrimitiveType::Num
        | PrimitiveType::Range
        | PrimitiveType::EventStore => None,
    }
}

/// Resolves a resource id used by a *global storage* operation. Move requires
/// the type of `move_to`/`move_from`/`borrow_global`/`exists` to be declared in
/// the acting module, so only local ids are valid here — an id reaching into
/// [`XirModule::external_structs`] is a genuine error, not a lookup miss.
fn struct_at(structs: &[StructId], id: usize, function: &str) -> Result<StructId> {
    structs.get(id).copied().with_context(|| {
        format!(
            "struct id {id} is out of range in `{function}`; global storage operations \
             require a type declared in this module"
        )
    })
}

fn translate_function(
    env: &GlobalEnv,
    xir: &XirModule,
    module_id: ModuleId,
    struct_ids: &[StructId],
    external_struct_ids: &[QualifiedId<StructId>],
    function_ids: &[FunId],
    decl: &FunctionDecl,
    qid: QualifiedId<FunId>,
) -> Result<TargetFunctionData> {
    let func_env = env.get_function(qid);
    let struct_arities = struct_arities(env, xir, external_struct_ids);
    let scope = StructScope {
        module_id,
        local: struct_ids,
        external: external_struct_ids,
        arities: &struct_arities,
    };
    let mut translator = FunctionTranslator {
        env,
        xir,
        module_id,
        struct_ids,
        external_struct_ids,
        struct_arities: &struct_arities,
        function_ids,
        decl,
        qid,
        loc: func_env.get_loc(),
        function_loc: func_env.get_loc(),
        code: vec![],
        locations: BTreeMap::new(),
        local_types: decl
            .locals
            .iter()
            .map(|ty| model_type(ty, &scope))
            .collect::<Result<Vec<_>>>()?,
        next_attr: 0,
        next_label: decl.blocks.len(),
    };
    translator.check_types()?;
    translator.emit(|attr| Bytecode::Jump(attr, Label::new(decl.entry)))?;
    for (block_id, block) in decl.blocks.iter().enumerate() {
        let block_source_map = decl
            .source_map
            .as_ref()
            .and_then(|source_map| source_map.blocks.get(block_id));
        let block_span = block_source_map
            .and_then(|source_map| source_map.instrs.iter().flatten().next().copied())
            .or_else(|| block_source_map.and_then(|source_map| source_map.term));
        translator.set_source_span(block_span);
        translator.emit(|attr| Bytecode::Label(attr, Label::new(block_id)))?;
        let mut instruction_id = 0;
        while instruction_id < block.instrs.len() {
            translator.set_source_span(
                block_source_map
                    .and_then(|source_map| source_map.instrs.get(instruction_id))
                    .copied()
                    .flatten(),
            );
            if let Some(consumed) = translator
                .translate_reference_vector_update(
                    block_id,
                    &block.instrs[instruction_id..],
                    &block.term,
                )
                .with_context(|| {
                    format!(
                        "function `{}`, block {block_id}, instruction {instruction_id}",
                        decl.name
                    )
                })?
            {
                instruction_id += consumed;
                continue;
            }
            let instruction = &block.instrs[instruction_id];
            translator
                .translate_instruction(instruction)
                .with_context(|| {
                    format!(
                        "function `{}`, block {block_id}, instruction {instruction_id}",
                        decl.name
                    )
                })?;
            instruction_id += 1;
        }
        translator.set_source_span(block_source_map.and_then(|source_map| source_map.term));
        translator
            .translate_term(&block.term)
            .with_context(|| format!("function `{}`, block {block_id}", decl.name))?;
    }
    let result_type = Type::tuple(
        decl.returns
            .iter()
            .map(|ty| model_type(ty, &scope))
            .collect::<Result<Vec<_>>>()?,
    );
    let acquires = decl
        .acquires
        .iter()
        .map(|id| struct_at(struct_ids, *id, &decl.name))
        .collect::<Result<Vec<_>>>()?;
    let mut used_local_names = BTreeSet::new();
    // Every local needs a stable unique name for internal lookup, but only
    // source-provided names may be shown in diagnostics. In particular, do not
    // put generated `_lN` names in `FunctionData::local_names`: compiler-v2
    // deliberately treats absence from that map as an anonymous value.
    let all_local_names = (0..translator.local_types.len())
        .map(|index| {
            let preferred = decl
                .local_names
                .get(index)
                .and_then(Option::as_deref)
                .map(String::from)
                // Unnamed XIR locals are compiler-generated temporaries. Mark
                // them as intentionally unused so diagnostics do not ask users
                // to edit values which do not exist in their Lean source.
                .unwrap_or_else(|| format!("_l{index}"));
            let mut unique = preferred.clone();
            if !used_local_names.insert(unique.clone()) {
                unique = format!("{preferred}${index}");
                let mut discriminator = 0;
                while !used_local_names.insert(unique.clone()) {
                    discriminator += 1;
                    unique = format!("{preferred}${index}${discriminator}");
                }
            }
            (index, env.symbol_pool().make(&unique))
        })
        .collect::<BTreeMap<_, _>>();
    let name_to_index = all_local_names
        .iter()
        .map(|(index, name)| (*name, *index))
        .collect();
    let local_names = decl
        .local_names
        .iter()
        .enumerate()
        .filter_map(|(index, name)| {
            name.as_deref()
                .map(|name| (index, env.symbol_pool().make(name)))
        })
        .collect();
    Ok(TargetFunctionData::new(
        &func_env,
        translator.code,
        translator.local_types,
        result_type,
        translator.locations,
        name_to_index,
        acquires,
        BTreeMap::new(),
        BTreeSet::new(),
        local_names,
    ))
}

struct FunctionTranslator<'a> {
    env: &'a GlobalEnv,
    xir: &'a XirModule,
    module_id: ModuleId,
    /// Locally declared structs, for global-storage operations, which Move
    /// requires to target a type of the acting module.
    struct_ids: &'a [StructId],
    external_struct_ids: &'a [QualifiedId<StructId>],
    struct_arities: &'a [usize],
    function_ids: &'a [FunId],
    decl: &'a FunctionDecl,
    /// The function being translated.
    qid: QualifiedId<FunId>,
    loc: Loc,
    function_loc: Loc,
    code: Vec<Bytecode>,
    locations: BTreeMap<AttrId, Loc>,
    local_types: Vec<Type>,
    next_attr: usize,
    next_label: usize,
}

impl FunctionTranslator<'_> {
    fn set_source_span(&mut self, span: Option<XirSourceSpan>) {
        self.loc = loc_for_span(&self.function_loc, span);
    }

    fn emit(&mut self, make: impl FnOnce(AttrId) -> Bytecode) -> Result<()> {
        ensure!(self.next_attr <= u16::MAX as usize, "function is too large");
        let attr = AttrId::new(self.next_attr);
        self.next_attr += 1;
        self.locations.insert(attr, self.loc.clone());
        self.code.push(make(attr));
        Ok(())
    }

    fn local(&self, id: usize) -> Result<&Type> {
        self.local_types
            .get(id)
            .with_context(|| format!("local l{id} is out of range in `{}`", self.decl.name))
    }

    /// Checks that local `id` has the annotated width. Stackless arithmetic
    /// carries none, so a mismatch would otherwise be lost here.
    fn check_width(&self, oper: &Oper, width: IntType, id: usize) -> Result<()> {
        let ty = self.local(id)?;
        ensure!(
            int_type_of(ty) == Some(width),
            "{oper:?} is annotated {width:?}, but l{id} is `{}`",
            self.show(ty)
        );
        Ok(())
    }

    fn show(&self, ty: &Type) -> String {
        ty.display(&self.env.get_type_display_ctx()).to_string()
    }

    fn block(&self, id: usize) -> Result<&Block> {
        self.decl
            .blocks
            .get(id)
            .with_context(|| format!("block {id} is out of range in `{}`", self.decl.name))
    }

    fn fresh_local(&mut self, ty: Type) -> usize {
        let id = self.local_types.len();
        self.local_types.push(ty);
        id
    }

    fn fresh_label(&mut self) -> Result<Label> {
        ensure!(
            self.next_label <= u16::MAX as usize,
            "function `{}` requires more than {} labels",
            self.decl.name,
            u16::MAX
        );
        let label = Label::new(self.next_label);
        self.next_label += 1;
        Ok(label)
    }

    fn translate_instruction(&mut self, instruction: &Instr) -> Result<()> {
        match instruction {
            Instr::Load(dst, constant) => {
                let ty = self.local(*dst)?.clone();
                let constant = stackless_constant(constant, &ty)?;
                self.emit(|attr| Bytecode::Load(attr, *dst, constant))
            },
            Instr::Assign(dst, src) => {
                self.local(*dst)?;
                self.local(*src)?;
                self.emit(|attr| Bytecode::Assign(attr, *dst, *src, AssignKind::Inferred))
            },
            Instr::Call(dsts, oper, srcs) => self.translate_call(dsts, oper, srcs),
            Instr::Nop => self.emit(Bytecode::Nop),
        }
    }

    /// Lean's small IR represents reference mutation using the already-proved
    /// `read_ref; functional update; write_ref` vocabulary. Recognize that
    /// sequence before stackless lowering and keep the vector borrowed. This
    /// avoids copying a potentially large vector merely to call Move's native
    /// reference-based vector operations.
    fn translate_reference_vector_update(
        &mut self,
        block_id: usize,
        instrs: &[Instr],
        term: &Term,
    ) -> Result<Option<usize>> {
        let [Instr::Call(read_dsts, Oper::ReadRef, read_srcs), Instr::Call(update_dsts, oper @ (Oper::VecInsert | Oper::VecRemove), update_srcs), Instr::Call(write_dsts, Oper::WriteRef, write_srcs), ..] =
            instrs
        else {
            return Ok(None);
        };
        let ([old], [reference]) = (read_dsts.as_slice(), read_srcs.as_slice()) else {
            return Ok(None);
        };
        if !write_dsts.is_empty() {
            return Ok(None);
        }
        let Some(updated) = update_dsts.first() else {
            return Ok(None);
        };
        if update_srcs.first() != Some(old) || write_srcs.as_slice() != [*reference, *updated] {
            return Ok(None);
        }
        let Type::Reference(ReferenceKind::Mutable, referent) = self.local(*reference)? else {
            return Ok(None);
        };
        let Type::Vector(element) = referent.as_ref() else {
            return Ok(None);
        };
        if self.local(*old)? != referent.as_ref() || self.local(*updated)? != referent.as_ref() {
            return Ok(None);
        }
        let value_dst = match oper {
            Oper::VecInsert if update_dsts.len() == 1 && update_srcs.len() == 3 => None,
            Oper::VecRemove if update_dsts.len() == 2 && update_srcs.len() == 2 => {
                Some(update_dsts[1])
            },
            _ => return Ok(None),
        };
        let u64_type = Type::Primitive(PrimitiveType::U64);
        if self.local(update_srcs[1])? != &u64_type
            || (oper == &Oper::VecInsert && self.local(update_srcs[2])? != element.as_ref())
            || value_dst.is_some_and(|dst| self.local(dst).ok() != Some(element.as_ref()))
            || old == updated
            || value_dst.is_some_and(|dst| dst == *old || dst == *updated)
            || self.local_is_read_outside_vector_update(*old, block_id, &instrs[3..], term)
            || self.local_is_read_outside_vector_update(*updated, block_id, &instrs[3..], term)
        {
            return Ok(None);
        }
        self.translate_vector_update_on_reference(
            value_dst,
            oper,
            update_srcs,
            *reference,
            element.as_ref().clone(),
        )?;
        Ok(Some(3))
    }

    fn local_is_read_outside_vector_update(
        &self,
        local: usize,
        block_id: usize,
        current_tail: &[Instr],
        current_term: &Term,
    ) -> bool {
        local_is_read_after(local, current_tail, current_term)
            || self.decl.blocks.iter().enumerate().any(|(index, block)| {
                index != block_id && local_is_read_after(local, &block.instrs, &block.term)
            })
    }

    fn translate_call(&mut self, dsts: &[usize], oper: &Oper, srcs: &[usize]) -> Result<()> {
        for id in dsts.iter().chain(srcs) {
            self.local(*id)?;
        }
        match oper {
            Oper::VecLen => {
                arity(dsts, srcs, 1, 1, oper)?;
                let source_type = self.local(srcs[0])?.clone();
                let element = self.vector_element_type(&source_type)?;
                let reference = match source_type {
                    Type::Reference(ReferenceKind::Immutable, _) => srcs[0],
                    Type::Reference(ReferenceKind::Mutable, referent) => {
                        let reference =
                            self.fresh_local(Type::Reference(ReferenceKind::Immutable, referent));
                        self.emit(|attr| {
                            Bytecode::Call(
                                attr,
                                vec![reference],
                                StacklessOperation::FreezeRef(true),
                                vec![srcs[0]],
                                None,
                            )
                        })?;
                        reference
                    },
                    vector_type => {
                        let reference = self.fresh_local(Type::Reference(
                            ReferenceKind::Immutable,
                            Box::new(vector_type),
                        ));
                        self.emit(|attr| {
                            Bytecode::Call(
                                attr,
                                vec![reference],
                                StacklessOperation::BorrowLoc,
                                vec![srcs[0]],
                                None,
                            )
                        })?;
                        reference
                    },
                };
                let operation = self.vector_function("length", element)?;
                self.emit(|attr| {
                    Bytecode::Call(attr, dsts.to_vec(), operation, vec![reference], None)
                })
            },
            Oper::VecGet => {
                arity(dsts, srcs, 1, 2, oper)?;
                let source_type = self.local(srcs[0])?.clone();
                let element = self.vector_element_type(&source_type)?;
                let vector_ref = match source_type {
                    Type::Reference(ReferenceKind::Immutable, _) => srcs[0],
                    Type::Reference(ReferenceKind::Mutable, referent) => {
                        let reference =
                            self.fresh_local(Type::Reference(ReferenceKind::Immutable, referent));
                        self.emit(|attr| {
                            Bytecode::Call(
                                attr,
                                vec![reference],
                                StacklessOperation::FreezeRef(true),
                                vec![srcs[0]],
                                None,
                            )
                        })?;
                        reference
                    },
                    vector_type => {
                        let reference = self.fresh_local(Type::Reference(
                            ReferenceKind::Immutable,
                            Box::new(vector_type),
                        ));
                        self.emit(|attr| {
                            Bytecode::Call(
                                attr,
                                vec![reference],
                                StacklessOperation::BorrowLoc,
                                vec![srcs[0]],
                                None,
                            )
                        })?;
                        reference
                    },
                };
                let element_ref = self.fresh_local(Type::Reference(
                    ReferenceKind::Immutable,
                    Box::new(element.clone()),
                ));
                let operation = self.vector_function("borrow", element)?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![element_ref],
                        operation,
                        vec![vector_ref, srcs[1]],
                        None,
                    )
                })?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        dsts.to_vec(),
                        StacklessOperation::ReadRef,
                        vec![element_ref],
                        None,
                    )
                })
            },
            Oper::VecSet
            | Oper::VecPush
            | Oper::VecPop
            | Oper::VecInsert
            | Oper::VecRemove
            | Oper::VecSwap => self.translate_functional_vector_update(dsts, oper, srcs),
            Oper::BorrowVecElem => {
                arity(dsts, srcs, 1, 2, oper)?;
                let element = self.vector_element_type(self.local(srcs[0])?)?;
                let mutable = matches!(
                    self.local(dsts[0])?,
                    Type::Reference(ReferenceKind::Mutable, _)
                );
                let operation =
                    self.vector_function(if mutable { "borrow_mut" } else { "borrow" }, element)?;
                self.emit(|attr| {
                    Bytecode::Call(attr, dsts.to_vec(), operation, srcs.to_vec(), None)
                })
            },
            Oper::TestVariant(variant) | Oper::TestVariantInst(variant, _) => {
                arity(dsts, srcs, 1, 1, oper)?;
                let enum_type = self.local(srcs[0])?.clone();
                let sid = self.struct_from_type(&enum_type)?;
                let enum_ref = self.fresh_local(Type::Reference(
                    ReferenceKind::Immutable,
                    Box::new(enum_type),
                ));
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![enum_ref],
                        StacklessOperation::BorrowLoc,
                        vec![srcs[0]],
                        None,
                    )
                })?;
                let args = match oper {
                    Oper::TestVariantInst(_, args) => self.type_args(args)?,
                    _ => vec![],
                };
                let operation = StacklessOperation::TestVariant(
                    self.module_id,
                    sid,
                    self.variant(sid, *variant)?,
                    args,
                );
                self.emit(|attr| {
                    Bytecode::Call(attr, dsts.to_vec(), operation, vec![enum_ref], None)
                })
            },
            Oper::GetField(field) | Oper::GetFieldInst(field, _) => {
                arity(dsts, srcs, 1, 1, oper)?;
                let struct_type = self.local(srcs[0])?.clone();
                let field_type = self.local(dsts[0])?.clone();
                let sid = self.struct_from_type(&struct_type)?;
                self.field(sid, *field)?;
                let args = match oper {
                    Oper::GetFieldInst(_, args) => self.type_args(args)?,
                    _ => vec![],
                };
                let struct_ref = self.fresh_local(Type::Reference(
                    ReferenceKind::Immutable,
                    Box::new(struct_type),
                ));
                let field_ref = self.fresh_local(Type::Reference(
                    ReferenceKind::Immutable,
                    Box::new(field_type),
                ));
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![struct_ref],
                        StacklessOperation::BorrowLoc,
                        vec![srcs[0]],
                        None,
                    )
                })?;
                let module_id = self.module_id;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![field_ref],
                        StacklessOperation::BorrowField(module_id, sid, args, *field),
                        vec![struct_ref],
                        None,
                    )
                })?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        dsts.to_vec(),
                        StacklessOperation::ReadRef,
                        vec![field_ref],
                        None,
                    )
                })
            },
            Oper::GetGlobal(id) | Oper::GetGlobalInst(id, _) => {
                arity(dsts, srcs, 1, 1, oper)?;
                let sid = struct_at(self.struct_ids, *id, &self.decl.name)?;
                let args = match oper {
                    Oper::GetGlobalInst(_, args) => self.type_args(args)?,
                    _ => vec![],
                };
                let reference = self.fresh_local(Type::Reference(
                    ReferenceKind::Immutable,
                    Box::new(Type::Struct(self.module_id, sid, args.clone())),
                ));
                let module_id = self.module_id;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![reference],
                        StacklessOperation::BorrowGlobal(module_id, sid, args),
                        srcs.to_vec(),
                        None,
                    )
                })?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        dsts.to_vec(),
                        StacklessOperation::ReadRef,
                        vec![reference],
                        None,
                    )
                })
            },
            Oper::MoveTo(id) | Oper::MoveToInst(id, _) => {
                arity(dsts, srcs, 0, 2, oper)?;
                let sid = struct_at(self.struct_ids, *id, &self.decl.name)?;
                let args = match oper {
                    Oper::MoveToInst(_, args) => self.type_args(args)?,
                    _ => vec![],
                };
                let signer_ref = match self.local(srcs[0])?.clone() {
                    Type::Reference(ReferenceKind::Immutable, _) => srcs[0],
                    Type::Reference(ReferenceKind::Mutable, referent) => {
                        let reference =
                            self.fresh_local(Type::Reference(ReferenceKind::Immutable, referent));
                        self.emit(|attr| {
                            Bytecode::Call(
                                attr,
                                vec![reference],
                                StacklessOperation::FreezeRef(true),
                                vec![srcs[0]],
                                None,
                            )
                        })?;
                        reference
                    },
                    signer_type => {
                        let reference = self.fresh_local(Type::Reference(
                            ReferenceKind::Immutable,
                            Box::new(signer_type),
                        ));
                        self.emit(|attr| {
                            Bytecode::Call(
                                attr,
                                vec![reference],
                                StacklessOperation::BorrowLoc,
                                vec![srcs[0]],
                                None,
                            )
                        })?;
                        reference
                    },
                };
                let module_id = self.module_id;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![],
                        StacklessOperation::MoveTo(module_id, sid, args),
                        vec![signer_ref, srcs[1]],
                        None,
                    )
                })
            },
            Oper::WriteGlobal(id) => {
                arity(dsts, srcs, 0, 2, oper)?;
                let sid = struct_at(self.struct_ids, *id, &self.decl.name)?;
                let reference = self.fresh_local(Type::Reference(
                    ReferenceKind::Mutable,
                    Box::new(Type::Struct(self.module_id, sid, vec![])),
                ));
                let module_id = self.module_id;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![reference],
                        StacklessOperation::BorrowGlobal(module_id, sid, vec![]),
                        vec![srcs[0]],
                        None,
                    )
                })?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![],
                        StacklessOperation::WriteRef,
                        vec![reference, srcs[1]],
                        None,
                    )
                })
            },
            // A malformed `<` falls through to the arity check below.
            Oper::Lt if srcs.len() == 2 && !self.local(srcs[0])?.is_number() => {
                self.translate_generic_less(dsts, srcs)
            },
            _ => {
                let operation = self.operation(dsts, oper, srcs)?;
                self.emit(|attr| {
                    Bytecode::Call(attr, dsts.to_vec(), operation, srcs.to_vec(), None)
                })
            },
        }
    }

    fn operation(&self, dsts: &[usize], oper: &Oper, srcs: &[usize]) -> Result<StacklessOperation> {
        Ok(match oper {
            Oper::Add(width)
            | Oper::Sub(width)
            | Oper::Mul(width)
            | Oper::Div(width)
            | Oper::Mod(width)
            | Oper::BitAnd(width)
            | Oper::BitOr(width)
            | Oper::BitXor(width)
            | Oper::Shl(width)
            | Oper::Shr(width) => {
                arity(dsts, srcs, 1, 2, oper)?;
                // Operands and result share the annotated type, except a
                // shift's amount, which is a `u8` the annotation does not cover.
                let is_shift = matches!(oper, Oper::Shl(_) | Oper::Shr(_));
                self.check_width(oper, *width, srcs[0])?;
                self.check_width(oper, *width, dsts[0])?;
                if !is_shift {
                    self.check_width(oper, *width, srcs[1])?;
                }
                match oper {
                    Oper::Add(_) => StacklessOperation::Add,
                    Oper::Sub(_) => StacklessOperation::Sub,
                    Oper::Mul(_) => StacklessOperation::Mul,
                    Oper::Div(_) => StacklessOperation::Div,
                    Oper::Mod(_) => StacklessOperation::Mod,
                    Oper::BitAnd(_) => StacklessOperation::BitAnd,
                    Oper::BitOr(_) => StacklessOperation::BitOr,
                    Oper::BitXor(_) => StacklessOperation::Xor,
                    Oper::Shl(_) => StacklessOperation::Shl,
                    Oper::Shr(_) => StacklessOperation::Shr,
                    _ => unreachable!(),
                }
            },
            Oper::Lt | Oper::Le | Oper::Eq | Oper::And | Oper::Or => {
                arity(dsts, srcs, 1, 2, oper)?;
                match oper {
                    Oper::Lt => StacklessOperation::Lt,
                    Oper::Le => StacklessOperation::Le,
                    Oper::Eq => StacklessOperation::Eq,
                    Oper::And => StacklessOperation::And,
                    Oper::Or => StacklessOperation::Or,
                    _ => unreachable!(),
                }
            },
            Oper::Cast(target) => {
                arity(dsts, srcs, 1, 1, oper)?;
                // A cast's width names its result, not its operand.
                self.check_width(oper, *target, dsts[0])?;
                match target {
                    IntType::U8 => StacklessOperation::CastU8,
                    IntType::U16 => StacklessOperation::CastU16,
                    IntType::U32 => StacklessOperation::CastU32,
                    IntType::U64 => StacklessOperation::CastU64,
                    IntType::U128 => StacklessOperation::CastU128,
                    IntType::U256 => StacklessOperation::CastU256,
                    IntType::I8 => StacklessOperation::CastI8,
                    IntType::I16 => StacklessOperation::CastI16,
                    IntType::I32 => StacklessOperation::CastI32,
                    IntType::I64 => StacklessOperation::CastI64,
                    IntType::I128 => StacklessOperation::CastI128,
                    IntType::I256 => StacklessOperation::CastI256,
                }
            },
            Oper::Not => {
                arity(dsts, srcs, 1, 1, oper)?;
                StacklessOperation::Not
            },
            Oper::VecPack => {
                ensure!(dsts.len() == 1, "vec_pack expects one destination");
                StacklessOperation::Vector
            },
            Oper::Pack => {
                ensure!(dsts.len() == 1, "pack expects one destination");
                let sid = self.struct_from_type(self.local(dsts[0])?)?;
                StacklessOperation::Pack(self.module_id, sid, vec![])
            },
            Oper::PackInst(args) => {
                ensure!(dsts.len() == 1, "pack expects one destination");
                let sid = self.struct_from_type(self.local(dsts[0])?)?;
                StacklessOperation::Pack(self.module_id, sid, self.type_args(args)?)
            },
            Oper::Unpack => {
                ensure!(srcs.len() == 1, "unpack expects one source");
                let sid = self.struct_from_type(self.local(srcs[0])?)?;
                StacklessOperation::Unpack(self.module_id, sid, vec![])
            },
            Oper::UnpackInst(args) => {
                ensure!(srcs.len() == 1, "unpack expects one source");
                let sid = self.struct_from_type(self.local(srcs[0])?)?;
                StacklessOperation::Unpack(self.module_id, sid, self.type_args(args)?)
            },
            Oper::PackVariant(variant) => {
                ensure!(dsts.len() == 1, "pack_variant expects one destination");
                let sid = self.struct_from_type(self.local(dsts[0])?)?;
                StacklessOperation::PackVariant(
                    self.module_id,
                    sid,
                    self.variant(sid, *variant)?,
                    vec![],
                )
            },
            Oper::PackVariantInst(variant, args) => {
                ensure!(dsts.len() == 1, "pack_variant expects one destination");
                let sid = self.struct_from_type(self.local(dsts[0])?)?;
                StacklessOperation::PackVariant(
                    self.module_id,
                    sid,
                    self.variant(sid, *variant)?,
                    self.type_args(args)?,
                )
            },
            Oper::UnpackVariant(variant) => {
                ensure!(srcs.len() == 1, "unpack_variant expects one source");
                let sid = self.struct_from_type(self.local(srcs[0])?)?;
                StacklessOperation::UnpackVariant(
                    self.module_id,
                    sid,
                    self.variant(sid, *variant)?,
                    vec![],
                )
            },
            Oper::UnpackVariantInst(variant, args) => {
                ensure!(srcs.len() == 1, "unpack_variant expects one source");
                let sid = self.struct_from_type(self.local(srcs[0])?)?;
                StacklessOperation::UnpackVariant(
                    self.module_id,
                    sid,
                    self.variant(sid, *variant)?,
                    self.type_args(args)?,
                )
            },
            Oper::GetField(field) => {
                arity(dsts, srcs, 1, 1, oper)?;
                let sid = self.struct_from_type(self.local(srcs[0])?)?;
                self.field(sid, *field)?;
                StacklessOperation::GetField(self.module_id, sid, vec![], *field)
            },
            Oper::GetFieldInst(field, args) => {
                arity(dsts, srcs, 1, 1, oper)?;
                let sid = self.struct_from_type(self.local(srcs[0])?)?;
                self.field(sid, *field)?;
                StacklessOperation::GetField(self.module_id, sid, self.type_args(args)?, *field)
            },
            Oper::MoveTo(id) => {
                arity(dsts, srcs, 0, 2, oper)?;
                StacklessOperation::MoveTo(
                    self.module_id,
                    struct_at(self.struct_ids, *id, &self.decl.name)?,
                    vec![],
                )
            },
            Oper::MoveToInst(id, args) => {
                arity(dsts, srcs, 0, 2, oper)?;
                StacklessOperation::MoveTo(
                    self.module_id,
                    struct_at(self.struct_ids, *id, &self.decl.name)?,
                    self.type_args(args)?,
                )
            },
            Oper::MoveFrom(id) => {
                arity(dsts, srcs, 1, 1, oper)?;
                StacklessOperation::MoveFrom(
                    self.module_id,
                    struct_at(self.struct_ids, *id, &self.decl.name)?,
                    vec![],
                )
            },
            Oper::MoveFromInst(id, args) => {
                arity(dsts, srcs, 1, 1, oper)?;
                StacklessOperation::MoveFrom(
                    self.module_id,
                    struct_at(self.struct_ids, *id, &self.decl.name)?,
                    self.type_args(args)?,
                )
            },
            Oper::Exists(id) => {
                arity(dsts, srcs, 1, 1, oper)?;
                StacklessOperation::Exists(
                    self.module_id,
                    struct_at(self.struct_ids, *id, &self.decl.name)?,
                    vec![],
                )
            },
            Oper::ExistsInst(id, args) => {
                arity(dsts, srcs, 1, 1, oper)?;
                StacklessOperation::Exists(
                    self.module_id,
                    struct_at(self.struct_ids, *id, &self.decl.name)?,
                    self.type_args(args)?,
                )
            },
            Oper::Function(id) | Oper::FunctionInst(id, _) => {
                let target =
                    function_at(self.env, self.xir, self.module_id, self.function_ids, *id)?;
                let type_args = match oper {
                    Oper::FunctionInst(_, args) => self.type_args(args)?,
                    _ => vec![],
                };
                let callee = self.env.get_function(target);
                // Inline functions and lemmas have no bytecode; source calls to
                // them are expanded before translation, which XIR cannot do.
                ensure!(
                    !callee.is_excluded_from_bytecode_gen(),
                    "{oper:?}: `{}` has no bytecode (an inline function or lemma), so it cannot be called",
                    callee.get_full_name_with_address()
                );
                // The same visibility rule the source compiler applies to calls,
                // for the same modules: those being compiled. A dependency was
                // checked in its own build.
                if self.env.get_module(self.module_id).is_primary_target() {
                    if let Some((message, _)) =
                        call_access_error(&self.env.get_function(self.qid), &callee)
                    {
                        bail!("{oper:?}: {message}");
                    }
                }
                ensure!(
                    callee.get_type_parameter_count() == type_args.len(),
                    "function `{}` takes {} type arguments, but the call supplies {}",
                    callee.get_full_name_str(),
                    callee.get_type_parameter_count(),
                    type_args.len()
                );
                StacklessOperation::Function(target.module_id, target.id, type_args)
            },
            Oper::Closure(id, mask) | Oper::ClosureInst(id, mask, _) => {
                let target =
                    function_at(self.env, self.xir, self.module_id, self.function_ids, *id)?;
                let type_args = match oper {
                    Oper::ClosureInst(_, _, args) => self.type_args(args)?,
                    _ => vec![],
                };
                let callee = self.env.get_function(target);
                ensure!(
                    callee.get_type_parameter_count() == type_args.len(),
                    "function `{}` takes {} type arguments, but the closure supplies {}",
                    callee.get_full_name_str(),
                    callee.get_type_parameter_count(),
                    type_args.len()
                );
                let mask = ClosureMask::new(*mask);
                ensure!(
                    mask.max_captured()
                        .is_none_or(|index| index < callee.get_parameter_count()),
                    "closure mask {mask} captures beyond the parameters of `{}`",
                    callee.get_full_name_str()
                );
                arity(dsts, srcs, 1, mask.captured_count() as usize, oper)?;
                StacklessOperation::Closure(target.module_id, target.id, type_args, mask)
            },
            Oper::Invoke => {
                ensure!(
                    !srcs.is_empty(),
                    "invoke expects the function value as its last source"
                );
                StacklessOperation::Invoke
            },
            Oper::BorrowLoc => {
                arity(dsts, srcs, 1, 1, oper)?;
                StacklessOperation::BorrowLoc
            },
            Oper::BorrowField(field) => {
                arity(dsts, srcs, 1, 1, oper)?;
                let sid = self.struct_from_type(self.local(srcs[0])?)?;
                self.field(sid, *field)?;
                StacklessOperation::BorrowField(self.module_id, sid, vec![], *field)
            },
            Oper::BorrowFieldInst(field, args) => {
                arity(dsts, srcs, 1, 1, oper)?;
                let sid = self.struct_from_type(self.local(srcs[0])?)?;
                self.field(sid, *field)?;
                StacklessOperation::BorrowField(self.module_id, sid, self.type_args(args)?, *field)
            },
            Oper::BorrowVariantField(variants, field) => {
                arity(dsts, srcs, 1, 1, oper)?;
                let sid = self.struct_from_type(self.local(srcs[0])?)?;
                let variants = variants
                    .iter()
                    .map(|variant| self.variant(sid, *variant))
                    .collect::<Result<Vec<_>>>()?;
                StacklessOperation::BorrowVariantField(
                    self.module_id,
                    sid,
                    variants,
                    vec![],
                    *field,
                )
            },
            Oper::BorrowVariantFieldInst(variants, field, args) => {
                arity(dsts, srcs, 1, 1, oper)?;
                let sid = self.struct_from_type(self.local(srcs[0])?)?;
                let variants = variants
                    .iter()
                    .map(|variant| self.variant(sid, *variant))
                    .collect::<Result<Vec<_>>>()?;
                StacklessOperation::BorrowVariantField(
                    self.module_id,
                    sid,
                    variants,
                    self.type_args(args)?,
                    *field,
                )
            },
            Oper::TestVariantRef(variant) => {
                arity(dsts, srcs, 1, 1, oper)?;
                let sid = self.struct_from_type(self.local(srcs[0])?)?;
                StacklessOperation::TestVariant(
                    self.module_id,
                    sid,
                    self.variant(sid, *variant)?,
                    vec![],
                )
            },
            Oper::TestVariantRefInst(variant, args) => {
                arity(dsts, srcs, 1, 1, oper)?;
                let sid = self.struct_from_type(self.local(srcs[0])?)?;
                StacklessOperation::TestVariant(
                    self.module_id,
                    sid,
                    self.variant(sid, *variant)?,
                    self.type_args(args)?,
                )
            },
            Oper::BorrowGlobal(id) => {
                arity(dsts, srcs, 1, 1, oper)?;
                StacklessOperation::BorrowGlobal(
                    self.module_id,
                    struct_at(self.struct_ids, *id, &self.decl.name)?,
                    vec![],
                )
            },
            Oper::BorrowGlobalInst(id, args) => {
                arity(dsts, srcs, 1, 1, oper)?;
                StacklessOperation::BorrowGlobal(
                    self.module_id,
                    struct_at(self.struct_ids, *id, &self.decl.name)?,
                    self.type_args(args)?,
                )
            },
            Oper::ReadRef => {
                arity(dsts, srcs, 1, 1, oper)?;
                StacklessOperation::ReadRef
            },
            Oper::WriteRef => {
                arity(dsts, srcs, 0, 2, oper)?;
                StacklessOperation::WriteRef
            },
            Oper::FreezeRef => {
                arity(dsts, srcs, 1, 1, oper)?;
                StacklessOperation::FreezeRef(true)
            },
            unsupported => bail!("unsupported XIR operation {unsupported:?}"),
        })
    }

    fn type_args(&self, args: &[Ty]) -> Result<Vec<Type>> {
        let scope = self.scope();
        args.iter().map(|arg| model_type(arg, &scope)).collect()
    }

    fn vector_element_type(&self, ty: &Type) -> Result<Type> {
        match ty {
            Type::Vector(element) => Ok(element.as_ref().clone()),
            Type::Reference(_, referent) => self.vector_element_type(referent),
            other => bail!("expected a vector type, got {other:?}"),
        }
    }

    fn vector_function(&self, name: &str, element: Type) -> Result<StacklessOperation> {
        let module = self
            .env
            .get_modules()
            .find(|module| module.is_std_vector())
            .context("the Move model has no standard vector module")?;
        let function = module
            .find_function(self.env.symbol_pool().make(name))
            .with_context(|| format!("the standard vector module has no `{name}` function"))?;
        Ok(StacklessOperation::Function(
            module.get_id(),
            function.get_id(),
            vec![element],
        ))
    }

    /// XIR enters the model after compiler-v2's AST comparison rewriter has
    /// run. Reproduce that rewrite here so generic `<` follows exactly the
    /// production Move path: `std::cmp::compare<T>(&left, &right).is_lt()`.
    fn translate_generic_less(&mut self, dsts: &[usize], srcs: &[usize]) -> Result<()> {
        arity(dsts, srcs, 1, 2, &Oper::Lt)?;
        let operand_type = self.local(srcs[0])?.clone();
        ensure!(
            self.local(srcs[1])? == &operand_type,
            "generic comparison operands have different types"
        );

        let module = self
            .env
            .get_modules()
            .find(|module| module.is_cmp())
            .context("generic comparison requires the standard `cmp` module")?;
        let compare = module
            .find_function(self.env.symbol_pool().make("compare"))
            .context("the standard `cmp` module has no `compare` function")?;
        let is_lt = module
            .find_function(self.env.symbol_pool().make("is_lt"))
            .context("the standard `cmp` module has no `is_lt` function")?;
        let module_id = module.get_id();

        let (compare_type, left_ref, right_ref) = match operand_type {
            Type::Reference(ReferenceKind::Immutable, referent) => {
                (referent.as_ref().clone(), srcs[0], srcs[1])
            },
            Type::Reference(ReferenceKind::Mutable, referent) => {
                let immutable_type = Type::Reference(ReferenceKind::Immutable, referent.clone());
                let left_ref = self.fresh_local(immutable_type.clone());
                let right_ref = self.fresh_local(immutable_type);
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![left_ref],
                        StacklessOperation::FreezeRef(true),
                        vec![srcs[0]],
                        None,
                    )
                })?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![right_ref],
                        StacklessOperation::FreezeRef(true),
                        vec![srcs[1]],
                        None,
                    )
                })?;
                (referent.as_ref().clone(), left_ref, right_ref)
            },
            value_type => {
                let reference_type =
                    Type::Reference(ReferenceKind::Immutable, Box::new(value_type.clone()));
                let left_ref = self.fresh_local(reference_type.clone());
                let right_ref = self.fresh_local(reference_type);
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![left_ref],
                        StacklessOperation::BorrowLoc,
                        vec![srcs[0]],
                        None,
                    )
                })?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![right_ref],
                        StacklessOperation::BorrowLoc,
                        vec![srcs[1]],
                        None,
                    )
                })?;
                (value_type, left_ref, right_ref)
            },
        };

        let ordering_type = compare.get_result_type();
        let ordering = self.fresh_local(ordering_type.clone());
        self.emit(|attr| {
            Bytecode::Call(
                attr,
                vec![ordering],
                StacklessOperation::Function(module_id, compare.get_id(), vec![compare_type]),
                vec![left_ref, right_ref],
                None,
            )
        })?;
        let ordering_ref = self.fresh_local(Type::Reference(
            ReferenceKind::Immutable,
            Box::new(ordering_type),
        ));
        self.emit(|attr| {
            Bytecode::Call(
                attr,
                vec![ordering_ref],
                StacklessOperation::BorrowLoc,
                vec![ordering],
                None,
            )
        })?;
        self.emit(|attr| {
            Bytecode::Call(
                attr,
                dsts.to_vec(),
                StacklessOperation::Function(module_id, is_lt.get_id(), vec![]),
                vec![ordering_ref],
                None,
            )
        })
    }

    fn translate_functional_vector_update(
        &mut self,
        dsts: &[usize],
        oper: &Oper,
        srcs: &[usize],
    ) -> Result<()> {
        let (vector_dst, value_dst) = match oper {
            Oper::VecSet => {
                arity(dsts, srcs, 1, 3, oper)?;
                (dsts[0], None)
            },
            Oper::VecPush => {
                arity(dsts, srcs, 1, 2, oper)?;
                (dsts[0], None)
            },
            Oper::VecPop => {
                arity(dsts, srcs, 2, 1, oper)?;
                (dsts[0], Some(dsts[1]))
            },
            Oper::VecInsert => {
                arity(dsts, srcs, 1, 3, oper)?;
                (dsts[0], None)
            },
            Oper::VecRemove => {
                arity(dsts, srcs, 2, 2, oper)?;
                (dsts[0], Some(dsts[1]))
            },
            Oper::VecSwap => {
                arity(dsts, srcs, 1, 3, oper)?;
                (dsts[0], None)
            },
            _ => unreachable!(),
        };
        let element = self.vector_element_type(self.local(srcs[0])?)?;
        self.emit(|attr| Bytecode::Assign(attr, vector_dst, srcs[0], AssignKind::Inferred))?;
        let vector_ref = self.fresh_local(Type::Reference(
            ReferenceKind::Mutable,
            Box::new(Type::Vector(Box::new(element.clone()))),
        ));
        self.emit(|attr| {
            Bytecode::Call(
                attr,
                vec![vector_ref],
                StacklessOperation::BorrowLoc,
                vec![vector_dst],
                None,
            )
        })?;
        self.translate_vector_update_on_reference(value_dst, oper, srcs, vector_ref, element)
    }

    fn translate_vector_update_on_reference(
        &mut self,
        value_dst: Option<usize>,
        oper: &Oper,
        srcs: &[usize],
        vector_ref: usize,
        element: Type,
    ) -> Result<()> {
        match oper {
            Oper::VecSet => {
                let element_ref = self.fresh_local(Type::Reference(
                    ReferenceKind::Mutable,
                    Box::new(element.clone()),
                ));
                let borrow = self.vector_function("borrow_mut", element)?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![element_ref],
                        borrow,
                        vec![vector_ref, srcs[1]],
                        None,
                    )
                })?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![],
                        StacklessOperation::WriteRef,
                        vec![element_ref, srcs[2]],
                        None,
                    )
                })
            },
            Oper::VecPush => {
                let push = self.vector_function("push_back", element)?;
                self.emit(|attr| {
                    Bytecode::Call(attr, vec![], push, vec![vector_ref, srcs[1]], None)
                })
            },
            Oper::VecPop => {
                let pop = self.vector_function("pop_back", element)?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![value_dst.expect("checked vec_pop destination")],
                        pop,
                        vec![vector_ref],
                        None,
                    )
                })
            },
            Oper::VecInsert => {
                // This Aptos stdlib does not expose `vector::insert` in the
                // bytecode model. Reconstruct its stable-shift algorithm
                // from actual vector opcodes so the generated control flow
                // still passes through compiler-v2's complete pipeline.
                let u64_type = Type::Primitive(PrimitiveType::U64);
                let bool_type = Type::Primitive(PrimitiveType::Bool);
                let len = self.fresh_local(u64_type.clone());
                let cursor = self.fresh_local(u64_type.clone());
                let one = self.fresh_local(u64_type.clone());
                let condition = self.fresh_local(bool_type);
                let abort_code = self.fresh_local(u64_type);
                let valid_label = self.fresh_label()?;
                let abort_label = self.fresh_label()?;
                let loop_label = self.fresh_label()?;
                let body_label = self.fresh_label()?;
                let done_label = self.fresh_label()?;
                let length = self.vector_function("length", element.clone())?;
                let push = self.vector_function("push_back", element.clone())?;
                let swap = self.vector_function("swap", element)?;
                self.emit(|attr| Bytecode::Call(attr, vec![len], length, vec![vector_ref], None))?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![condition],
                        StacklessOperation::Le,
                        vec![srcs[1], len],
                        None,
                    )
                })?;
                self.emit(|attr| Bytecode::Branch(attr, valid_label, abort_label, condition))?;
                self.emit(|attr| Bytecode::Label(attr, abort_label))?;
                self.emit(|attr| {
                    Bytecode::Load(
                        attr,
                        abort_code,
                        StacklessConstant::U64(VECTOR_INDEX_OUT_OF_BOUNDS),
                    )
                })?;
                self.emit(|attr| Bytecode::Abort(attr, abort_code, None))?;
                self.emit(|attr| Bytecode::Label(attr, valid_label))?;
                self.emit(|attr| {
                    Bytecode::Call(attr, vec![], push, vec![vector_ref, srcs[2]], None)
                })?;
                self.emit(|attr| Bytecode::Assign(attr, cursor, srcs[1], AssignKind::Inferred))?;
                self.emit(|attr| Bytecode::Load(attr, one, StacklessConstant::U64(1)))?;
                self.emit(|attr| Bytecode::Jump(attr, loop_label))?;
                self.emit(|attr| Bytecode::Label(attr, loop_label))?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![condition],
                        StacklessOperation::Lt,
                        vec![cursor, len],
                        None,
                    )
                })?;
                self.emit(|attr| Bytecode::Branch(attr, body_label, done_label, condition))?;
                self.emit(|attr| Bytecode::Label(attr, body_label))?;
                self.emit(|attr| {
                    Bytecode::Call(attr, vec![], swap, vec![vector_ref, cursor, len], None)
                })?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![cursor],
                        StacklessOperation::Add,
                        vec![cursor, one],
                        None,
                    )
                })?;
                self.emit(|attr| Bytecode::Jump(attr, loop_label))?;
                self.emit(|attr| Bytecode::Label(attr, done_label))
            },
            Oper::VecRemove => {
                // Stable removal is the dual shift: swap each successor one
                // position left, then pop the last element.
                let u64_type = Type::Primitive(PrimitiveType::U64);
                let bool_type = Type::Primitive(PrimitiveType::Bool);
                let len = self.fresh_local(u64_type.clone());
                let last = self.fresh_local(u64_type.clone());
                let cursor = self.fresh_local(u64_type.clone());
                let next = self.fresh_local(u64_type.clone());
                let one = self.fresh_local(u64_type.clone());
                let condition = self.fresh_local(bool_type);
                let abort_code = self.fresh_local(u64_type);
                let valid_label = self.fresh_label()?;
                let abort_label = self.fresh_label()?;
                let loop_label = self.fresh_label()?;
                let body_label = self.fresh_label()?;
                let done_label = self.fresh_label()?;
                let length = self.vector_function("length", element.clone())?;
                let swap = self.vector_function("swap", element.clone())?;
                let pop = self.vector_function("pop_back", element)?;
                self.emit(|attr| Bytecode::Call(attr, vec![len], length, vec![vector_ref], None))?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![condition],
                        StacklessOperation::Lt,
                        vec![srcs[1], len],
                        None,
                    )
                })?;
                self.emit(|attr| Bytecode::Branch(attr, valid_label, abort_label, condition))?;
                self.emit(|attr| Bytecode::Label(attr, abort_label))?;
                self.emit(|attr| {
                    Bytecode::Load(
                        attr,
                        abort_code,
                        StacklessConstant::U64(VECTOR_INDEX_OUT_OF_BOUNDS),
                    )
                })?;
                self.emit(|attr| Bytecode::Abort(attr, abort_code, None))?;
                self.emit(|attr| Bytecode::Label(attr, valid_label))?;
                self.emit(|attr| Bytecode::Load(attr, one, StacklessConstant::U64(1)))?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![last],
                        StacklessOperation::Sub,
                        vec![len, one],
                        None,
                    )
                })?;
                self.emit(|attr| Bytecode::Assign(attr, cursor, srcs[1], AssignKind::Inferred))?;
                self.emit(|attr| Bytecode::Jump(attr, loop_label))?;
                self.emit(|attr| Bytecode::Label(attr, loop_label))?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![condition],
                        StacklessOperation::Lt,
                        vec![cursor, last],
                        None,
                    )
                })?;
                self.emit(|attr| Bytecode::Branch(attr, body_label, done_label, condition))?;
                self.emit(|attr| Bytecode::Label(attr, body_label))?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![next],
                        StacklessOperation::Add,
                        vec![cursor, one],
                        None,
                    )
                })?;
                self.emit(|attr| {
                    Bytecode::Call(attr, vec![], swap, vec![vector_ref, cursor, next], None)
                })?;
                self.emit(|attr| Bytecode::Assign(attr, cursor, next, AssignKind::Inferred))?;
                self.emit(|attr| Bytecode::Jump(attr, loop_label))?;
                self.emit(|attr| Bytecode::Label(attr, done_label))?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![value_dst.expect("checked vec_remove destination")],
                        pop,
                        vec![vector_ref],
                        None,
                    )
                })
            },
            Oper::VecSwap => {
                let swap = self.vector_function("swap", element)?;
                self.emit(|attr| {
                    Bytecode::Call(attr, vec![], swap, vec![vector_ref, srcs[1], srcs[2]], None)
                })
            },
            _ => unreachable!(),
        }
    }

    fn struct_from_type(&self, ty: &Type) -> Result<StructId> {
        match ty {
            Type::Struct(mid, sid, _) if *mid == self.module_id => Ok(*sid),
            Type::Reference(_, referent) => self.struct_from_type(referent),
            other => bail!("expected a local struct type, got {other:?}"),
        }
    }

    fn field(&self, sid: StructId, field: usize) -> Result<()> {
        let index = self
            .struct_ids
            .iter()
            .position(|candidate| *candidate == sid)
            .context("unknown local struct")?;
        self.xir.structs[index]
            .fields
            .get(field)
            .with_context(|| format!("field id {field} is out of range"))?;
        Ok(())
    }

    fn variant(&self, sid: StructId, variant: usize) -> Result<move_model::symbol::Symbol> {
        let index = self
            .struct_ids
            .iter()
            .position(|candidate| *candidate == sid)
            .context("unknown local enum")?;
        let variants = self.xir.structs[index]
            .variants
            .as_ref()
            .context("variant operation used with a non-enum type")?;
        let variant = variants
            .get(variant)
            .with_context(|| format!("variant id {variant} is out of range"))?;
        Ok(self.env.symbol_pool().make(&variant.name))
    }

    fn translate_term(&mut self, term: &Term) -> Result<()> {
        match term {
            Term::Jump(target) => {
                self.block(*target)?;
                self.emit(|attr| Bytecode::Jump(attr, Label::new(*target)))
            },
            Term::Branch(cond, then_block, else_block) => {
                self.local(*cond)?;
                self.block(*then_block)?;
                self.block(*else_block)?;
                self.emit(|attr| {
                    Bytecode::Branch(
                        attr,
                        Label::new(*then_block),
                        Label::new(*else_block),
                        *cond,
                    )
                })
            },
            Term::Ret(values) => {
                ensure!(
                    values.len() == self.decl.returns.len(),
                    "return arity mismatch"
                );
                for value in values {
                    self.local(*value)?;
                }
                self.emit(|attr| Bytecode::Ret(attr, values.clone()))
            },
            Term::Abort(code) => {
                self.local(*code)?;
                self.emit(|attr| Bytecode::Abort(attr, *code, None))
            },
        }
    }
}

/// Whether a local whose defining instructions are omitted by a peephole is
/// read before being redefined. Destinations are deliberately ignored: a
/// later definition makes the skipped definition dead.
fn local_is_read_after(local: usize, instrs: &[Instr], term: &Term) -> bool {
    for instruction in instrs {
        match instruction {
            Instr::Load(destination, _) if *destination == local => return false,
            Instr::Assign(destination, source) => {
                if *source == local {
                    return true;
                }
                if *destination == local {
                    return false;
                }
            },
            Instr::Call(destinations, _, sources) => {
                if sources.contains(&local) {
                    return true;
                }
                if destinations.contains(&local) {
                    return false;
                }
            },
            Instr::Load(_, _) | Instr::Nop => {},
        }
    }
    match term {
        Term::Jump(_) => false,
        Term::Branch(condition, _, _) | Term::Abort(condition) => *condition == local,
        Term::Ret(values) => values.contains(&local),
    }
}

fn stackless_constant(constant: &Constant, ty: &Type) -> Result<StacklessConstant> {
    Ok(match constant {
        Constant::Num(value) => {
            let parse_err = || format!("invalid integer constant `{value}`");
            match ty {
                Type::Primitive(PrimitiveType::U8) => {
                    StacklessConstant::U8(value.parse().with_context(parse_err)?)
                },
                Type::Primitive(PrimitiveType::U16) => {
                    StacklessConstant::U16(value.parse().with_context(parse_err)?)
                },
                Type::Primitive(PrimitiveType::U32) => {
                    StacklessConstant::U32(value.parse().with_context(parse_err)?)
                },
                Type::Primitive(PrimitiveType::U64) => {
                    StacklessConstant::U64(value.parse().with_context(parse_err)?)
                },
                Type::Primitive(PrimitiveType::U128) => {
                    StacklessConstant::U128(value.parse().with_context(parse_err)?)
                },
                Type::Primitive(PrimitiveType::U256) => {
                    StacklessConstant::U256(value.parse::<ethnum::U256>().with_context(parse_err)?)
                },
                Type::Primitive(PrimitiveType::I8) => {
                    StacklessConstant::I8(value.parse().with_context(parse_err)?)
                },
                Type::Primitive(PrimitiveType::I16) => {
                    StacklessConstant::I16(value.parse().with_context(parse_err)?)
                },
                Type::Primitive(PrimitiveType::I32) => {
                    StacklessConstant::I32(value.parse().with_context(parse_err)?)
                },
                Type::Primitive(PrimitiveType::I64) => {
                    StacklessConstant::I64(value.parse().with_context(parse_err)?)
                },
                Type::Primitive(PrimitiveType::I128) => {
                    StacklessConstant::I128(value.parse().with_context(parse_err)?)
                },
                Type::Primitive(PrimitiveType::I256) => {
                    StacklessConstant::I256(value.parse::<ethnum::I256>().with_context(parse_err)?)
                },
                other => bail!("integer constant loaded into non-integer type {other:?}"),
            }
        },
        Constant::Bool(value) => StacklessConstant::Bool(*value),
        Constant::Address(value) => StacklessConstant::Address(Address::Numerical(
            AccountAddress::from_hex_literal(value)
                .with_context(|| format!("invalid address constant `{value}`"))?,
        )),
        Constant::Vector(_) => bail!("vector values are not valid XIR load constants"),
    })
}

fn arity(
    dsts: &[usize],
    srcs: &[usize],
    expected_dsts: usize,
    expected_srcs: usize,
    oper: &Oper,
) -> Result<()> {
    ensure!(
        dsts.len() == expected_dsts && srcs.len() == expected_srcs,
        "{oper:?} expects {expected_dsts} destinations and {expected_srcs} sources"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Options;
    use move_model_exchange::XirModuleRef;
    use std::fs;

    fn account_golden() -> String {
        fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/xir/account.xir.json"),
        )
        .unwrap()
    }

    fn account_module() -> XirModule {
        serde_json::from_str(&account_golden()).unwrap()
    }

    fn import_and_verify(module: XirModule) {
        let source = parse_source(
            PathBuf::from("test.xir.json"),
            String::new(),
            &serde_json::to_string(&module).unwrap(),
        )
        .unwrap();
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        import_sources(&mut env, &[source], &mut targets).unwrap();
        let options = Options::default();
        env.set_extension(options.clone());
        crate::run_stackless_bytecode_pipeline(
            &env,
            crate::stackless_bytecode_check_pipeline(&options),
            &mut targets,
        );
        assert!(!env.has_errors());
        crate::run_stackless_bytecode_pipeline(
            &env,
            crate::stackless_bytecode_optimization_pipeline(&options),
            &mut targets,
        );
        assert!(!env.has_errors());
        let units = crate::run_file_format_gen(&mut env, &targets);
        assert!(!env.has_errors());
        let legacy_move_compiler::compiled_unit::CompiledUnit::Module(module) = &units[0] else {
            panic!("expected module")
        };
        move_bytecode_verifier::verify_module(&module.module).unwrap();
    }

    /// A module declaring one struct and, optionally, one bodyless function
    /// taking a type from `external`.
    fn v6_module(name: &str, external: Option<(&str, &str)>, with_body: bool) -> XirModule {
        let external_structs = match external {
            Some((module, ty)) => serde_json::json!([
                {"address": "0x42", "module": module, "name": ty}
            ]),
            None => serde_json::json!([]),
        };
        // Local struct is id 0; the external type, when present, is id 1.
        let (param_ty, blocks) = if external.is_some() {
            (serde_json::json!({"struct": 1}), with_body)
        } else {
            (serde_json::json!({"struct": 0}), with_body)
        };
        let body = if blocks {
            serde_json::json!([{"instrs": [], "term": {"ret": []}}])
        } else {
            serde_json::json!([])
        };
        serde_json::from_value(serde_json::json!({
            "schema": move_model_exchange::XIR_SCHEMA,
            "version": move_model_exchange::XIR_VERSION,
            "module": {"address": "0x42", "name": name, "dialect": "stackless"},
            "friends": [{"address": "0x42", "module": "buddy"}],
            "structs": [{
                "name": "Local", "visibility": "public",
                "abilities": ["drop"],
                "fields": [{"name": "v", "ty": "u64"}],
            }],
            "functions": [{
                "name": "takes", "visibility": "public", "is_entry": false,
                "is_native": false, "acquires": [], "params": 1,
                "locals": [param_ty], "returns": [],
                "blocks": body, "entry": 0, "loops": [],
                "spec": {"requires": [], "modifies": [], "ensures": [], "aborts_if": []},
            }],
            "external_structs": external_structs,
        }))
        .unwrap()
    }

    fn source_of(module: &XirModule, is_target: bool) -> XirSource {
        parse_source_with_target(
            PathBuf::from("test.xir.json"),
            String::new(),
            &serde_json::to_string(module).unwrap(),
            is_target,
        )
        .unwrap()
    }

    /// A type from another module resolves through `external_structs` to that
    /// module's struct, not to a local one.
    #[test]
    fn external_structs_resolve_to_the_declaring_module() {
        let provider = source_of(&v6_module("provider", None, true), true);
        let consumer = source_of(
            &v6_module("consumer", Some(("provider", "Local")), true),
            true,
        );
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        import_sources(&mut env, &[provider, consumer], &mut targets).unwrap();

        let pool = env.symbol_pool();
        let find = |name: &str| {
            env.find_module(&ModuleName::new(
                Address::Numerical(AccountAddress::from_hex_literal("0x42").unwrap()),
                pool.make(name),
            ))
            .unwrap()
        };
        let provider_id = find("provider").get_id();
        let consumer_env = find("consumer");
        let function = consumer_env
            .find_function(pool.make("takes"))
            .expect("consumer has `takes`");
        let Type::Struct(module_id, _, _) = function.get_parameter_types()[0].clone() else {
            panic!("expected a struct parameter")
        };
        assert_eq!(
            module_id, provider_id,
            "the parameter type must resolve to the declaring module"
        );
    }

    /// Import ordering accounts for type dependencies, not just calls: the
    /// consumer is listed first but must be imported after its provider.
    #[test]
    fn import_order_respects_external_type_dependencies() {
        let consumer = source_of(
            &v6_module("consumer", Some(("provider", "Local")), true),
            true,
        );
        let provider = source_of(&v6_module("provider", None, true), true);
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        import_sources(&mut env, &[consumer, provider], &mut targets).unwrap();
        assert_eq!(env.get_module_count(), 2);
    }

    #[test]
    fn rejects_unresolvable_external_type() {
        let consumer = source_of(
            &v6_module("consumer", Some(("missing", "Local")), true),
            true,
        );
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        let error = import_sources(&mut env, &[consumer], &mut targets).unwrap_err();
        assert!(format!("{error:#}").contains("unresolved or cyclic"));
    }

    /// A dependency contributes declarations only: no bodies are translated,
    /// so a bodyless interface module is accepted and produces no targets.
    #[test]
    fn bodyless_module_imports_as_a_dependency_only() {
        let dependency = source_of(&v6_module("iface", None, false), false);
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        import_sources(&mut env, &[dependency], &mut targets).unwrap();
        assert_eq!(env.get_module_count(), 1);
        assert_eq!(
            targets.get_funs().count(),
            0,
            "a dependency must not enter the target holder"
        );
    }

    /// The same module supplied as a compilation target is rejected: a target
    /// must have code to compile.
    #[test]
    fn rejects_bodyless_module_supplied_as_a_target() {
        let error = parse_source_with_target(
            PathBuf::from("iface.xir.json"),
            String::new(),
            &serde_json::to_string(&v6_module("iface", None, false)).unwrap(),
            true,
        )
        .err()
        .expect("a bodyless target must be rejected");
        assert!(format!("{error:#}").contains("has no blocks"));
    }

    #[test]
    fn friends_and_struct_visibility_reach_the_model() {
        let source = source_of(&v6_module("m", None, true), true);
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        import_sources(&mut env, &[source], &mut targets).unwrap();
        let module = env.get_modules().next().unwrap();
        let friends = module.get_friend_decls();
        assert_eq!(friends.len(), 1, "the friend declaration must be recorded");
        assert_eq!(
            env.symbol_pool()
                .string(friends[0].module_name.name())
                .as_str(),
            "buddy"
        );
        let struct_env = module.get_structs().next().unwrap();
        assert_eq!(struct_env.get_visibility(), MoveVisibility::Public);
    }

    /// A friend declaration must reach `friend_modules`, not just
    /// `friend_decls`.
    ///
    /// These are two different things and only the former is serialized:
    /// `module_generator.rs` builds the bytecode's friend list from
    /// `ModuleEnv::get_friend_modules()`. Source modules get that set filled by
    /// `check_and_update_friend_info` at the end of the model builder, which an
    /// XIR module is added too late to participate in — so without resolving
    /// them in the loader, an XIR target compiles to bytecode with no friends
    /// at all, silently revoking access from its declared friend modules.
    #[test]
    fn friend_declarations_resolve_to_module_ids() {
        // Here `buddy` happens to load first, so the grant resolves during the
        // load itself. The reverse order is covered by
        // `friend_grants_survive_when_the_friend_loads_later`.
        let buddy = source_of(&v6_module("buddy", None, true), true);
        let m = source_of(&v6_module("m", None, true), true);
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        import_sources(&mut env, &[buddy, m], &mut targets).unwrap();

        let buddy_id = env
            .get_modules()
            .find(|module| env.symbol_pool().string(module.get_name().name()).as_str() == "buddy")
            .expect("buddy was loaded")
            .get_id();
        let m = env
            .get_modules()
            .find(|module| env.symbol_pool().string(module.get_name().name()).as_str() == "m")
            .expect("m was loaded");

        assert!(
            m.get_friend_modules().contains(&buddy_id),
            "the friend must reach the set that file-format generation reads"
        );
        assert_eq!(
            m.get_friend_decls()[0].module_id,
            Some(buddy_id),
            "the declaration must also carry the resolved id"
        );
    }

    /// A grant survives when the friend module loads *after* the grantor.
    ///
    /// This is the only order the dependency sort can produce whenever the
    /// friend actually uses what it was granted: the friend refers to the
    /// granting module, so the grantor loads first, and the friend is absent
    /// at the moment the grant would be resolved. Resolving grants only at
    /// load time therefore drops them in exactly the configuration friend
    /// declarations exist for — and drops them silently, since an absent
    /// friend is deliberately not an error.
    #[test]
    fn friend_grants_survive_when_the_friend_loads_later() {
        let mut grantor = v6_module("provider", None, true);
        grantor.friends = vec![XirModuleRef {
            address: "0x42".to_owned(),
            module: "consumer".to_owned(),
        }];
        // Naming a type from `provider` forces `consumer` to load second.
        let consumer = v6_module("consumer", Some(("provider", "Local")), true);

        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        import_sources(
            &mut env,
            &[source_of(&grantor, true), source_of(&consumer, true)],
            &mut targets,
        )
        .unwrap();

        let id_of = |name: &str| {
            env.get_modules()
                .find(|module| env.symbol_pool().string(module.get_name().name()).as_str() == name)
                .unwrap_or_else(|| panic!("`{name}` was loaded"))
                .get_id()
        };
        let consumer_id = id_of("consumer");
        let provider = env.get_module(id_of("provider"));
        assert!(
            provider.get_friend_modules().contains(&consumer_id),
            "the grant must reach the set file-format generation reads, even though \
             the friend loaded after the grantor"
        );
        assert_eq!(
            provider.get_friend_decls()[0].module_id,
            Some(consumer_id),
            "the declaration must also carry the resolved id"
        );
    }

    /// A native function outside a special address reaches bytecode generation
    /// unchecked when it arrives as an XIR *target*.
    ///
    /// `native_checker` rejects this for Move sources, but it runs inside
    /// `env_check_and_transform_pipeline`, and `lib.rs` imports XIR targets
    /// *after* that pipeline has finished. Nothing reapplies it.
    ///
    /// This pins down which fix is needed. The module is a primary target and
    /// the rule fires the moment anything runs it, so the gap is the ordering
    /// alone — not a missing target flag.
    #[test]
    fn a_native_function_in_an_xir_target_is_only_checked_if_the_rule_is_run() {
        let mut module = v6_module("natives", None, false);
        module.functions[0].is_native = true;

        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        import_sources(&mut env, &[source_of(&module, true)], &mut targets).unwrap();

        // The fixture's address is not special, which is what the rule keys on.
        assert!(!AccountAddress::from_hex_literal("0x42")
            .unwrap()
            .is_special());
        assert!(
            env.get_modules().any(|module| module.is_primary_target()),
            "an XIR target must be a primary target for the rule to apply"
        );
        assert!(!env.has_errors(), "importing alone reports nothing");

        crate::env_pipeline::native_checker::check_for_native_functions_and_structs(&mut env);
        assert!(
            env.has_errors(),
            "the rule applies to an XIR target; only the pipeline ordering keeps it from running"
        );
    }

    /// The XIR transcription of `tests/checking/visibility-checker/
    /// call_private_function.move`, which the source path rejects with
    /// "function `0xdeadbeef::M::foo` is private to module `0xdeadbeef::M`".
    ///
    /// `M::foo` takes the given visibility, and declares `N` a friend when
    /// `grant_friend`; `N::calls_foo` calls it through `external_functions`.
    fn m_and_n(visibility: &str, grant_friend: bool) -> (XirModule, XirModule) {
        let friends = if grant_friend {
            serde_json::json!([{"address": "0xdeadbeef", "module": "N"}])
        } else {
            serde_json::json!([])
        };
        // module 0xdeadbeef::M { fun foo(): u64 { 1 } }
        let m = serde_json::from_value(serde_json::json!({
            "schema": move_model_exchange::XIR_SCHEMA,
            "version": move_model_exchange::XIR_VERSION,
            "module": {"address": "0xdeadbeef", "name": "M", "dialect": "stackless"},
            "friends": friends,
            "structs": [],
            "functions": [{
                "name": "foo", "visibility": visibility, "is_entry": false,
                "is_native": false, "acquires": [], "params": 0,
                "locals": ["u64"], "returns": ["u64"],
                "blocks": [{
                    "instrs": [{"load": [0, {"num": "1"}]}],
                    "term": {"ret": [0]},
                }],
                "entry": 0, "loops": [],
                "spec": {"requires": [], "modifies": [], "ensures": [], "aborts_if": []},
            }],
        }))
        .unwrap();
        // module 0xdeadbeef::N { fun calls_foo(): u64 { 0xdeadbeef::M::foo() } }
        //
        // `N` declares one function, so local ids end at 0 and the external
        // callee `M::foo` is id 1.
        let n = serde_json::from_value(serde_json::json!({
            "schema": move_model_exchange::XIR_SCHEMA,
            "version": move_model_exchange::XIR_VERSION,
            "module": {"address": "0xdeadbeef", "name": "N", "dialect": "stackless"},
            "structs": [],
            "functions": [{
                "name": "calls_foo", "visibility": "private", "is_entry": false,
                "is_native": false, "acquires": [], "params": 0,
                "locals": ["u64"], "returns": ["u64"],
                "blocks": [{
                    "instrs": [{"call": [[0], {"function": 1}, []]}],
                    "term": {"ret": [0]},
                }],
                "entry": 0, "loops": [],
                "spec": {"requires": [], "modifies": [], "ensures": [], "aborts_if": []},
            }],
            "external_functions": [
                {"address": "0xdeadbeef", "module": "M", "function": "foo"}
            ],
        }))
        .unwrap();
        (m, n)
    }

    /// An XIR target gets the same answer as Move source when it calls a
    /// function it may not see.
    ///
    /// Without the check, this compiles to verified bytecode and is rejected
    /// only by `dependencies::verify_module` — at publication.
    #[test]
    fn an_xir_target_cannot_call_a_function_it_may_not_see() {
        let import = |visibility: &str, grant_friend: bool| {
            let (m, n) = m_and_n(visibility, grant_friend);
            let mut env = GlobalEnv::new();
            let mut targets = FunctionTargetsHolder::default();
            import_sources(
                &mut env,
                &[source_of(&m, true), source_of(&n, true)],
                &mut targets,
            )
            .err()
            .map(|e| format!("{e:#}"))
        };

        assert!(
            import("public", false).is_none(),
            "a public callee is allowed"
        );

        let private = import("private", false).expect("a private callee is rejected");
        assert!(
            private.contains("`0xdeadbeef::M::foo` is private to module `0xdeadbeef::M`"),
            "the error matches what the source path reports: {private}"
        );

        let stranger = import("friend", false).expect("an ungranted friend callee is rejected");
        assert!(
            stranger.contains("not a friend of `0xdeadbeef::M`"),
            "the error says why: {stranger}"
        );

        // The grant is what makes this legal. `N` loads after `M`, whose grant
        // to it resolves only then, so this pins the grant resolving before
        // `N`'s calls are checked.
        assert!(
            import("friend", true).is_none(),
            "a granted friend callee is allowed"
        );
    }

    /// `fun f<T>() { f<Wrapper<T>>() }` grows its type argument without bound.
    ///
    /// The rule for this reads the AST, so it cannot be reapplied to XIR the
    /// way the declaration rules are. This pins where the violation *is*
    /// caught — the bytecode verifier — so the gap is recorded rather than
    /// assumed closed.
    #[test]
    fn a_cyclic_instantiation_in_an_xir_target_reaches_the_bytecode_verifier() {
        // module 0x42::C {
        //     struct Wrapper<T> has drop { f: T }
        //     fun f<T>() { f<Wrapper<T>>() }     // grows the type forever
        // }
        let module: XirModule = serde_json::from_value(serde_json::json!({
            "schema": move_model_exchange::XIR_SCHEMA,
            "version": move_model_exchange::XIR_VERSION,
            "module": {"address": "0x42", "name": "C", "dialect": "stackless"},
            "structs": [{
                "name": "Wrapper", "visibility": "public",
                "type_parameters": [{"name": "T"}],
                "abilities": ["drop"],
                "fields": [{"name": "f", "ty": {"type_parameter": 0}}],
            }],
            "functions": [{
                "name": "f", "visibility": "public", "is_entry": false,
                "is_native": false, "acquires": [], "params": 0,
                "type_parameters": [{"name": "T"}],
                "locals": [], "returns": [],
                "blocks": [{
                    "instrs": [{"call": [
                        [],
                        {"function_inst": [0, [{"struct_inst": [0, [{"type_parameter": 0}]]}]]},
                        []
                    ]}],
                    "term": {"ret": []},
                }],
                "entry": 0, "loops": [],
                "spec": {"requires": [], "modifies": [], "ensures": [], "aborts_if": []},
            }],
        }))
        .unwrap();

        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        import_sources(&mut env, &[source_of(&module, true)], &mut targets).unwrap();
        assert!(!env.has_errors(), "importing alone reports nothing");

        // Unlike the declaration rules, this one reads `get_def()`, so running
        // it here reports nothing. That is why `lib.rs` does not call it.
        crate::env_pipeline::cyclic_instantiation_checker::check_cyclic_instantiations(&env);
        assert!(
            !env.has_errors(),
            "an AST rule cannot see a stackless module"
        );

        let options = Options::default();
        env.set_extension(options.clone());
        crate::run_stackless_bytecode_pipeline(
            &env,
            crate::stackless_bytecode_check_pipeline(&options),
            &mut targets,
        );
        println!("after check pipeline={}", env.has_errors());
        crate::run_stackless_bytecode_pipeline(
            &env,
            crate::stackless_bytecode_optimization_pipeline(&options),
            &mut targets,
        );
        let units = crate::run_file_format_gen(&mut env, &targets);
        assert!(!env.has_errors(), "the compiler itself reports nothing");
        let legacy_move_compiler::compiled_unit::CompiledUnit::Module(unit) = &units[0] else {
            panic!("expected a module")
        };
        assert_eq!(
            move_bytecode_verifier::verify_module(&unit.module)
                .err()
                .map(|e| e.major_status()),
            Some(move_core_types::vm_status::StatusCode::LOOP_IN_INSTANTIATION_GRAPH),
            "the verifier is what rejects a cyclic instantiation"
        );
    }

    /// `struct Foo { f: Foo }` — the source path calls this "cyclic data".
    ///
    /// Reads struct declarations, which XIR carries, so `lib.rs` reruns it
    /// after import instead of needing a stackless rewrite.
    #[test]
    fn a_recursive_struct_in_an_xir_target_is_rejected() {
        // module 0x42::M0 { struct Foo { f: Foo } }
        let module: XirModule = serde_json::from_value(serde_json::json!({
            "schema": move_model_exchange::XIR_SCHEMA,
            "version": move_model_exchange::XIR_VERSION,
            "module": {"address": "0x42", "name": "M0", "dialect": "stackless"},
            "structs": [{
                "name": "Foo", "visibility": "public",
                "abilities": [],
                "fields": [{"name": "f", "ty": {"struct": 0}}],
            }],
            "functions": [],
        }))
        .unwrap();

        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        import_sources(&mut env, &[source_of(&module, true)], &mut targets).unwrap();
        assert!(!env.has_errors(), "importing alone reports nothing");

        crate::env_pipeline::recursive_struct_checker::check_recursive_struct(&env);
        assert!(env.has_errors(), "the rule applies to an XIR target");

        let mut out = codespan_reporting::term::termcolor::Buffer::no_color();
        env.report_diag(&mut out, codespan_reporting::diagnostic::Severity::Error);
        let diags = String::from_utf8_lossy(&out.into_inner()).into_owned();
        assert!(
            diags.contains("cyclic data")
                && diags.contains("field `f` of `Foo` contains `Foo`, which forms a cycle"),
            "the message is the one Move source gets: {diags}"
        );
    }

    /// `struct S<T> { x: u64 }` — a non-phantom parameter no field uses.
    ///
    /// Reads struct declarations, so `lib.rs` reruns it after import. This one
    /// reports a warning rather than an error, so it is the diagnostic that
    /// has to be inspected, not `has_errors`.
    #[test]
    fn an_unused_struct_parameter_in_an_xir_target_is_rejected() {
        let module: XirModule = serde_json::from_value(serde_json::json!({
            "schema": move_model_exchange::XIR_SCHEMA,
            "version": move_model_exchange::XIR_VERSION,
            "module": {"address": "0x42", "name": "M1", "dialect": "stackless"},
            "structs": [{
                "name": "S", "visibility": "public",
                "type_parameters": [{"name": "T"}],
                "abilities": [],
                "fields": [{"name": "x", "ty": "u64"}],
            }],
            "functions": [],
        }))
        .unwrap();

        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        import_sources(&mut env, &[source_of(&module, true)], &mut targets).unwrap();
        assert!(!env.has_errors(), "importing alone reports nothing");

        crate::env_pipeline::unused_params_checker::unused_params_checker(&env);
        let mut out = codespan_reporting::term::termcolor::Buffer::no_color();
        env.report_diag(&mut out, codespan_reporting::diagnostic::Severity::Warning);
        let diags = String::from_utf8_lossy(&out.into_inner()).into_owned();
        assert!(
            diags.contains("unused type parameter") && diags.contains("`T`"),
            "the rule applies to an XIR target: {diags}"
        );
    }

    /// Packing a struct another module owns is rejected, though Move source
    /// accepts it for a `public` struct.
    ///
    /// `external_structs` resolves a foreign type in a *signature*, but
    /// `struct_from_type` accepts only locally-owned types, so no operation can
    /// touch the value. This pins the gap; the companion half is
    /// `move_source_can_pack_a_foreign_public_struct` in `xir_differential.rs`.
    #[test]
    fn a_foreign_struct_cannot_be_packed() {
        // module 0x42::M { public struct S has drop { x: u64 } }
        let m: XirModule = serde_json::from_value(serde_json::json!({
            "schema": move_model_exchange::XIR_SCHEMA,
            "version": move_model_exchange::XIR_VERSION,
            "module": {"address": "0x42", "name": "M", "dialect": "stackless"},
            "structs": [{
                "name": "S", "visibility": "public", "abilities": ["drop"],
                "fields": [{"name": "x", "ty": "u64"}],
            }],
            "functions": [],
        }))
        .unwrap();
        // module 0x42::N { public fun make(): M::S { M::S { x: 1 } } }
        //
        // `N` declares no structs, so struct id 0 is external_structs[0].
        let n: XirModule = serde_json::from_value(serde_json::json!({
            "schema": move_model_exchange::XIR_SCHEMA,
            "version": move_model_exchange::XIR_VERSION,
            "module": {"address": "0x42", "name": "N", "dialect": "stackless"},
            "structs": [],
            "functions": [{
                "name": "make", "visibility": "public", "is_entry": false,
                "is_native": false, "acquires": [], "params": 0,
                "locals": [{"struct": 0}, "u64"], "returns": [{"struct": 0}],
                "blocks": [{
                    "instrs": [
                        {"load": [1, {"num": "1"}]},
                        {"call": [[0], "pack", [1]]},
                    ],
                    "term": {"ret": [0]},
                }],
                "entry": 0, "loops": [],
                "spec": {"requires": [], "modifies": [], "ensures": [], "aborts_if": []},
            }],
            "external_structs": [{"address": "0x42", "module": "M", "name": "S"}],
        }))
        .unwrap();

        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        let error = import_sources(
            &mut env,
            &[source_of(&m, true), source_of(&n, true)],
            &mut targets,
        )
        .err()
        .map(|e| format!("{e:#}"))
        .expect("a foreign struct operation is rejected today");
        assert!(
            error.contains("is not a struct of this module"),
            "the rejection comes from the type checker: {error}"
        );
    }

    /// An unresolved friend is tolerated, unlike on the source path.
    ///
    /// XIR modules are loaded one at a time, and a friend is typically a
    /// *dependent* that need not be present. Being lenient is safe because an
    /// absent friend only withholds an access grant.
    #[test]
    fn an_absent_friend_is_not_an_error() {
        let m = source_of(&v6_module("m", None, true), true);
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        import_sources(&mut env, &[m], &mut targets).unwrap();
        let module = env.get_modules().next().unwrap();
        assert_eq!(module.get_friend_decls().len(), 1);
        assert!(module.get_friend_modules().is_empty());
        assert!(!env.has_errors());
    }

    fn signed_golden() -> String {
        fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/xir/signed.xir.json"),
        )
        .unwrap()
    }

    /// Signed integers reach verified bytecode: every width in a signature,
    /// signed arithmetic, a negative `num` constant, and a signed cast.
    #[test]
    fn signed_integers_run_production_pipeline() {
        let module: XirModule = serde_json::from_str(&signed_golden()).unwrap();
        import_and_verify(module);
    }

    #[test]
    fn account_golden_decodes() {
        let source = parse_source(
            PathBuf::from("account.xir.json"),
            String::new(),
            &account_golden(),
        )
        .unwrap();
        assert_eq!(source.module.module.name, "AccountTest");
    }

    #[test]
    fn account_golden_runs_production_stackless_and_file_format_pipeline() {
        let source = parse_source(
            PathBuf::from("account.xir.json"),
            String::new(),
            &account_golden(),
        )
        .unwrap();
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        import_sources(&mut env, &[source], &mut targets).unwrap();
        let options = Options::default();
        env.set_extension(options.clone());
        crate::run_stackless_bytecode_pipeline(
            &env,
            crate::stackless_bytecode_check_pipeline(&options),
            &mut targets,
        );
        assert!(!env.has_errors());
        crate::run_stackless_bytecode_pipeline(
            &env,
            crate::stackless_bytecode_optimization_pipeline(&options),
            &mut targets,
        );
        assert!(!env.has_errors());
        let units = crate::run_file_format_gen(&mut env, &targets);
        assert!(!env.has_errors());
        assert_eq!(units.len(), 1);
        let legacy_move_compiler::compiled_unit::CompiledUnit::Module(module) = &units[0] else {
            panic!("expected module")
        };
        move_bytecode_verifier::verify_module(&module.module).unwrap();
    }

    #[test]
    fn function_attributes_preserve_arguments() {
        let mut module = account_module();
        let function_name = module.functions[0].name.clone();
        module.functions[0].attributes = vec![
            XirAttribute {
                name: "module_lock".to_owned(),
                args: vec![],
            },
            XirAttribute {
                name: "randomness".to_owned(),
                args: vec![XirAttributeArg::Num {
                    value: "7".to_owned(),
                }],
            },
            XirAttribute {
                name: "test_only".to_owned(),
                args: vec![XirAttributeArg::Bool { value: true }],
            },
            XirAttribute {
                name: "lint.skip".to_owned(),
                args: vec![XirAttributeArg::Name {
                    name: "complexity".to_owned(),
                    args: vec![XirAttributeArg::Name {
                        name: "cyclomatic".to_owned(),
                        args: vec![],
                    }],
                }],
            },
        ];
        let source = parse_source(
            PathBuf::from("attributes.xir.json"),
            String::new(),
            &serde_json::to_string(&module).unwrap(),
        )
        .unwrap();
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        import_sources(&mut env, &[source], &mut targets).unwrap();
        let function_symbol = env.symbol_pool().make(&function_name);
        let function = env
            .get_module(ModuleId::new(0))
            .find_function(function_symbol)
            .unwrap();
        let attributes = function.get_attributes();
        assert_eq!(attributes.len(), 4);

        let pool = env.symbol_pool();
        let assert_name = |attribute: &Attribute, expected_name| {
            assert_eq!(pool.string(attribute.name()).as_str(), expected_name);
        };
        assert_name(&attributes[0], "module_lock");
        assert!(matches!(attributes[0], Attribute::Apply(_, _, ref args) if args.is_empty()));
        assert_name(&attributes[1], "randomness");
        assert!(matches!(
            attributes[1],
            Attribute::Assign(_, _, AttributeValue::Value(_, Value::Number(ref value)))
                if value == &7.into()
        ));
        assert_name(&attributes[2], "test_only");
        assert!(matches!(
            attributes[2],
            Attribute::Assign(_, _, AttributeValue::Value(_, Value::Bool(true)))
        ));
        assert_name(&attributes[3], "lint.skip");
        let Attribute::Apply(_, _, nested) = &attributes[3] else {
            panic!("expected nested name attribute")
        };
        assert_eq!(nested.len(), 1);
        assert_name(&nested[0], "complexity");
        let Attribute::Apply(_, _, nested) = &nested[0] else {
            panic!("expected nested name attribute")
        };
        assert_eq!(nested.len(), 1);
        assert_name(&nested[0], "cyclomatic");
        assert!(matches!(nested[0], Attribute::Apply(_, _, ref args) if args.is_empty()));
    }

    fn import_module(module: &XirModule) -> Result<GlobalEnv> {
        import_module_with(module, None).map(|(env, _)| env)
    }

    /// As [`import_module`], with `options` set before the import reads them,
    /// and keeping the translated targets.
    fn import_module_with(
        module: &XirModule,
        options: Option<&Options>,
    ) -> Result<(GlobalEnv, FunctionTargetsHolder)> {
        let source = parse_source(
            PathBuf::from("test.xir.json"),
            String::new(),
            &serde_json::to_string(module).unwrap(),
        )?;
        let mut env = GlobalEnv::new();
        if let Some(options) = options {
            env.set_extension(options.clone());
        }
        let mut targets = FunctionTargetsHolder::default();
        import_sources(&mut env, &[source], &mut targets)?;
        Ok((env, targets))
    }

    /// The names of `name`'s attributes, with nested arguments in brackets.
    fn struct_attribute_names(env: &GlobalEnv, name: &str) -> Vec<String> {
        fn render(env: &GlobalEnv, attribute: &Attribute) -> String {
            let name = env.symbol_pool().string(attribute.name()).to_string();
            match attribute {
                Attribute::Apply(_, _, args) if args.is_empty() => name,
                Attribute::Apply(_, _, args) => format!(
                    "{name}[{}]",
                    args.iter()
                        .map(|arg| render(env, arg))
                        .collect::<Vec<_>>()
                        .join(",")
                ),
                Attribute::Assign(_, _, AttributeValue::Value(_, value)) => {
                    format!("{name}={value:?}")
                },
                Attribute::Assign(..) => format!("{name}=?"),
            }
        }
        env.get_module(ModuleId::new(0))
            .find_struct(env.symbol_pool().make(name))
            .unwrap()
            .get_attributes()
            .iter()
            .map(|attribute| render(env, attribute))
            .collect()
    }

    #[test]
    fn struct_attributes_reach_the_model() {
        let mut module = account_module();
        let annotated = module.structs[0].name.clone();
        let plain = module.structs[1].name.clone();
        module.structs[0].attributes = vec![
            XirAttribute {
                name: "event".to_owned(),
                args: vec![],
            },
            XirAttribute {
                name: "count".to_owned(),
                args: vec![XirAttributeArg::Num {
                    value: "7".to_owned(),
                }],
            },
            XirAttribute {
                name: "flag".to_owned(),
                args: vec![XirAttributeArg::Bool { value: true }],
            },
            XirAttribute {
                name: "lint::skip".to_owned(),
                args: vec![XirAttributeArg::Name {
                    name: "needless_mutable_reference".to_owned(),
                    args: vec![],
                }],
            },
        ];
        // An enum goes through the same path as a struct; nothing refers to
        // this one, so adding it disturbs no function.
        module.structs.push(StructDecl {
            name: "Tagged".to_owned(),
            visibility: XirVisibility::Private,
            abilities: vec!["drop".to_owned()],
            type_parameters: vec![],
            fields: vec![],
            variants: Some(vec![move_model_exchange::Variant {
                name: "A".to_owned(),
                fields: vec![],
            }]),
            attributes: vec![XirAttribute {
                name: "event".to_owned(),
                args: vec![],
            }],
        });
        let env = import_module(&module).unwrap();
        assert_eq!(struct_attribute_names(&env, &annotated), vec![
            "event",
            "count=Number(7)",
            "flag=Bool(true)",
            "lint::skip[needless_mutable_reference]",
        ]);
        assert_eq!(struct_attribute_names(&env, "Tagged"), vec!["event"]);
        assert!(struct_attribute_names(&env, &plain).is_empty());
    }

    /// A literal mid-list (the `attribute_wire_shape` shape) is skipped with a
    /// warning; the module and the struct's other attributes still load.
    #[test]
    fn a_struct_attribute_the_model_cannot_hold_is_skipped_with_a_warning() {
        let mut module = account_module();
        let name = module.structs[0].name.clone();
        module.structs[0].attributes = vec![
            XirAttribute {
                name: "resource_group".to_owned(),
                args: vec![
                    XirAttributeArg::Name {
                        name: "scope".to_owned(),
                        args: vec![XirAttributeArg::Name {
                            name: "global".to_owned(),
                            args: vec![],
                        }],
                    },
                    XirAttributeArg::Num {
                        value: "7".to_owned(),
                    },
                    XirAttributeArg::Bool { value: true },
                ],
            },
            XirAttribute {
                name: "event".to_owned(),
                args: vec![],
            },
        ];
        let env = import_module(&module).expect("the module still loads");
        assert_eq!(struct_attribute_names(&env, &name), vec!["event"]);
        assert!(!env.has_errors());
        let mut out = codespan_reporting::term::termcolor::Buffer::no_color();
        env.report_diag(&mut out, codespan_reporting::diagnostic::Severity::Warning);
        let warnings = String::from_utf8_lossy(&out.into_inner()).to_string();
        assert!(
            warnings.contains("attribute `resource_group` on struct")
                && warnings.contains("not carried"),
            "{warnings}"
        );
    }

    /// A test-only or verify-only struct or function is rejected unless the
    /// build compiles that code; the file-format generator would otherwise
    /// assert on a test-only item and publish a verify-only one.
    #[test]
    fn test_and_verify_only_items_need_their_build() {
        let mark = |name: &str| {
            vec![XirAttribute {
                name: name.to_owned(),
                args: vec![],
            }]
        };
        // Imports `module` under `options`, then generates the file format.
        let compile = |module: &XirModule, options: Options| -> Result<usize> {
            let (mut env, mut targets) = import_module_with(module, Some(&options))?;
            crate::run_stackless_bytecode_pipeline(
                &env,
                crate::stackless_bytecode_optimization_pipeline(&options),
                &mut targets,
            );
            Ok(crate::run_file_format_gen(&mut env, &targets).len())
        };
        let build = |test: bool, verify: bool| Options {
            compile_test_code: test,
            compile_verify_code: verify,
            ..Options::default()
        };
        let marked = |attribute: &str, on_struct: bool| {
            let mut module = account_module();
            if on_struct {
                module.structs[0].attributes = mark(attribute);
            } else {
                module.functions[1].attributes = mark(attribute);
            }
            module
        };
        let mut wrong = vec![];
        // (label, module, kind, the build that excludes it, the build that includes it)
        for (label, module, kind, without, with) in [
            (
                "test_only struct",
                marked("test_only", true),
                "struct",
                build(false, false),
                build(true, false),
            ),
            (
                "test function",
                marked("test", false),
                "function",
                build(false, false),
                build(true, false),
            ),
            (
                "verify_only struct",
                marked("verify_only", true),
                "struct",
                build(false, false),
                build(false, true),
            ),
            (
                "verify_only function",
                marked("verify_only", false),
                "function",
                build(false, false),
                build(false, true),
            ),
            // The two kinds of build are independent.
            (
                "verify_only function in a test build",
                marked("verify_only", false),
                "function",
                build(true, false),
                build(false, true),
            ),
        ] {
            match compile(&module, without) {
                Err(error) if format!("{error:#}").contains(&format!("{kind} `")) => {},
                result => wrong.push(format!("{label}, excluded: {result:?}")),
            }
            if let Err(error) = compile(&module, with) {
                wrong.push(format!("{label}, included: {error:#}"));
            }
        }
        if let Err(error) = compile(&account_module(), build(false, false)) {
            wrong.push(format!("the golden module: {error:#}"));
        }
        assert!(wrong.is_empty(), "{wrong:#?}");
    }

    /// A copy of the golden module whose first function is replaced by one
    /// with a single instruction and these locals:
    /// l0-l2 `u64`, l3 `u8`, l4 `address`, l5 `u16`, l6 `u32`, l7 `u128`,
    /// l8 `u256`, l9 `bool`, l10 `vector<u64>`, l11 `&u64`, l12 a struct.
    fn instruction_module(instr: Instr) -> XirModule {
        let mut module = account_module();
        // Tests add fields newer than the golden document's version.
        module.version = move_model_exchange::XIR_VERSION;
        let function = &mut module.functions[0];
        function.params = 0;
        function.locals = vec![
            Ty::U64,
            Ty::U64,
            Ty::U64,
            Ty::U8,
            Ty::Address,
            Ty::U16,
            Ty::U32,
            Ty::U128,
            Ty::U256,
            Ty::Bool,
            Ty::Vector(Box::new(Ty::U64)),
            Ty::Ref(Box::new(Ty::U64)),
            Ty::Struct(0),
        ];
        function.local_names = vec![];
        function.returns = vec![];
        function.source_map = None;
        function.entry = 0;
        function.blocks = vec![Block {
            instrs: vec![instr],
            term: Term::Ret(vec![]),
        }];
        module
    }

    fn load_instruction(instr: Instr) -> Result<()> {
        import_module(&instruction_module(instr)).map(|_| ())
    }

    fn two_operand_ops(width: IntType) -> Vec<Oper> {
        vec![
            Oper::Add(width),
            Oper::Sub(width),
            Oper::Mul(width),
            Oper::Div(width),
            Oper::Mod(width),
            Oper::BitAnd(width),
            Oper::BitOr(width),
            Oper::BitXor(width),
        ]
    }

    #[test]
    fn a_consistent_width_annotation_loads() {
        let mut cases: Vec<(Vec<usize>, Oper, Vec<usize>)> = two_operand_ops(IntType::U64)
            .into_iter()
            .map(|oper| (vec![2], oper, vec![0, 1]))
            .collect();
        cases.extend([
            // A shift's amount is a `u8` the annotation does not describe.
            (vec![2], Oper::Shl(IntType::U64), vec![0, 3]),
            (vec![2], Oper::Shr(IntType::U64), vec![0, 3]),
            // A cast's annotation names its result; the operand may be any width.
            (vec![3], Oper::Cast(IntType::U8), vec![0]),
            (vec![0], Oper::Cast(IntType::U64), vec![3]),
        ]);
        let rejected: Vec<_> = cases
            .into_iter()
            .filter_map(|(dsts, oper, srcs)| {
                load_instruction(Instr::Call(dsts, oper.clone(), srcs))
                    .err()
                    .map(|error| format!("{oper:?}: {error:#}"))
            })
            .collect();
        assert!(rejected.is_empty(), "rejected: {rejected:#?}");
    }

    #[test]
    fn a_width_annotation_must_match_the_locals() {
        let mut cases: Vec<(Vec<usize>, Oper, Vec<usize>)> = two_operand_ops(IntType::I64)
            .into_iter()
            .map(|oper| (vec![2], oper, vec![0, 1]))
            .collect();
        cases.extend([
            (vec![2], Oper::Add(IntType::U64), vec![4, 1]), // first operand
            (vec![2], Oper::Add(IntType::U64), vec![0, 4]), // second operand
            (vec![3], Oper::Add(IntType::U64), vec![0, 1]), // destination
            (vec![2], Oper::Shl(IntType::U64), vec![3, 3]), // shl value
            (vec![2], Oper::Shr(IntType::U64), vec![3, 3]), // shr value
            (vec![0], Oper::Cast(IntType::U8), vec![3]),    // cast result
        ]);
        let accepted: Vec<_> = cases
            .into_iter()
            .filter(|(dsts, oper, srcs)| {
                load_instruction(Instr::Call(dsts.clone(), oper.clone(), srcs.clone())).is_ok()
            })
            .map(|(dsts, oper, srcs)| format!("{oper:?} {dsts:?} <- {srcs:?}"))
            .collect();
        assert!(accepted.is_empty(), "accepted: {accepted:?}");
    }

    /// Each integer width, with a local of that type: the matching annotation
    /// loads and a neighbouring width does not. This pins every arm of
    /// `int_type`, not just `u64`.
    #[test]
    fn every_width_is_checked_against_its_own_type() {
        let widths = [
            (3, IntType::U8, IntType::U16),
            (5, IntType::U16, IntType::U32),
            (6, IntType::U32, IntType::U64),
            (0, IntType::U64, IntType::U128),
            (7, IntType::U128, IntType::U256),
            (8, IntType::U256, IntType::U8),
        ];
        let mut wrong = vec![];
        for (local, width, neighbour) in widths {
            let load =
                |w| load_instruction(Instr::Call(vec![local], Oper::Add(w), vec![local, local]));
            if load(width).is_err() {
                wrong.push(format!("{width:?} over its own type was rejected"));
            }
            if load(neighbour).is_ok() {
                wrong.push(format!("{neighbour:?} over {width:?} was accepted"));
            }
        }
        assert!(wrong.is_empty(), "{wrong:#?}");
    }

    /// An operand that is not an integer at all is rejected, whatever its kind.
    #[test]
    fn a_non_integer_operand_is_rejected() {
        let accepted: Vec<_> = [4, 9, 10, 11, 12] // address, bool, vector, reference, struct
            .into_iter()
            .filter(|&operand| {
                load_instruction(Instr::Call(vec![2], Oper::Add(IntType::U64), vec![
                    operand, 1,
                ]))
                .is_ok()
            })
            .collect();
        assert!(
            accepted.is_empty(),
            "accepted non-integer locals: {accepted:?}"
        );
    }

    #[test]
    fn a_width_mismatch_names_the_operation_and_the_local() {
        let error = load_instruction(Instr::Call(vec![2], Oper::Div(IntType::I64), vec![0, 1]))
            .unwrap_err();
        let message = format!("{error:#}");
        assert!(
            message.contains("Div(I64)") && message.contains("l0") && message.contains("`u64`"),
            "{message}"
        );
    }

    /// Why `assign` is checked: `l1 := l0` puts a `u8` in a `u64` local, so
    /// the `u64` shift passes `check_width`. Unchecked, the optimizer
    /// propagates the copy and the verifier accepts a truncating `u8` shift.
    #[test]
    fn an_ill_typed_assign_cannot_change_a_shift_width() {
        let mut module = account_module();
        let function = &mut module.functions[0];
        function.params = 1;
        function.locals = vec![Ty::U8, Ty::U64, Ty::U64, Ty::U64];
        function.local_names = vec![];
        function.returns = vec![Ty::U64];
        function.source_map = None;
        function.entry = 0;
        function.blocks = vec![Block {
            instrs: vec![
                Instr::Assign(1, 0),
                Instr::Call(vec![2], Oper::Shl(IntType::U64), vec![1, 0]),
                Instr::Call(vec![3], Oper::Cast(IntType::U64), vec![2]),
            ],
            term: Term::Ret(vec![3]),
        }];
        let message = format!("{:#}", import_module(&module).unwrap_err());
        assert!(
            message.contains("instruction 0") && message.contains("assign"),
            "{message}"
        );
    }

    /// A reference is the type of a local or a return value, never a field
    /// or part of another type.
    #[test]
    fn a_reference_is_only_the_type_of_a_local_or_a_return_value() {
        #[derive(Debug, Clone, Copy)]
        enum Place {
            Local,
            Return,
            Field,
            VariantField,
        }
        use Place::*;
        let r = |ty: Ty| Ty::Ref(Box::new(ty));
        let v = |ty: Ty| Ty::Vector(Box::new(ty));
        // `G<T> { x: T }` and `enum GE<T> { A { x: T } }`, at indices 2 and 3
        // after the golden module's two structs.
        let g = |ty: Ty| Ty::StructInst(2, vec![ty]);
        let ge = |ty: Ty| Ty::EnumInst(3, vec![ty]);
        let field = |ty: Ty| move_model_exchange::Field {
            name: "x".to_owned(),
            ty,
        };
        let declaration = |name: &str, fields, variants| StructDecl {
            name: name.to_owned(),
            visibility: XirVisibility::Private,
            abilities: vec!["copy".to_owned(), "drop".to_owned()],
            type_parameters: vec![],
            fields,
            variants,
            attributes: vec![],
        };
        let module = |place: Place, ty: Ty| {
            let mut module = instruction_module(Instr::Nop);
            assert_eq!(module.structs.len(), 2);
            let parameter = TypeParameterDecl {
                name: "T".to_owned(),
                abilities: vec![],
                phantom: false,
            };
            let mut generic = declaration("G", vec![field(Ty::TypeParameter(0))], None);
            generic.type_parameters = vec![parameter.clone()];
            module.structs.push(generic);
            let mut generic_enum = declaration(
                "GE",
                vec![],
                Some(vec![move_model_exchange::Variant {
                    name: "A".to_owned(),
                    fields: vec![field(Ty::TypeParameter(0))],
                }]),
            );
            generic_enum.type_parameters = vec![parameter];
            module.structs.push(generic_enum);
            let function = &mut module.functions[0];
            match place {
                Local => function.locals.push(ty),
                Return => {
                    function.locals.push(ty.clone());
                    function.returns = vec![ty];
                    function.blocks[0].term = Term::Ret(vec![function.locals.len() - 1]);
                },
                Field => module.structs.push(declaration("H", vec![field(ty)], None)),
                VariantField => module.structs.push(declaration(
                    "V",
                    vec![],
                    Some(vec![move_model_exchange::Variant {
                        name: "A".to_owned(),
                        fields: vec![field(ty)],
                    }]),
                )),
            }
            module
        };
        let cases = [
            (true, Local, r(Ty::U64)),
            (true, Local, Ty::MutRef(Box::new(g(Ty::U64)))),
            (true, Local, g(v(Ty::U64))),
            (false, Local, v(r(Ty::U64))),
            (false, Local, r(r(Ty::U64))),
            (false, Local, Ty::MutRef(Box::new(r(Ty::U64)))),
            (true, Local, ge(Ty::U64)),
            (false, Local, ge(r(Ty::U64))),
            (false, Local, g(r(Ty::U64))),
            (false, Local, r(g(v(r(Ty::U64))))),
            (true, Return, r(Ty::U64)),
            (false, Return, v(r(Ty::U64))),
            (true, Field, v(Ty::U64)),
            (false, Field, r(Ty::U64)),
            (false, Field, g(r(Ty::U64))),
            (true, VariantField, v(Ty::U64)),
            (false, VariantField, r(Ty::U64)),
        ];
        let wrong: Vec<_> = cases
            .into_iter()
            .filter_map(|(ok, place, ty)| {
                let result = import_module(&module(place, ty.clone())).map(|_| ());
                let as_expected = match &result {
                    Ok(()) => ok,
                    Err(error) => !ok && format!("{error:#}").contains("is a reference"),
                };
                (!as_expected).then(|| format!("{place:?} {ty:?}: {result:?}"))
            })
            .collect();
        assert!(wrong.is_empty(), "{wrong:#?}");
    }

    /// The golden module at the current version, as an interface carries it.
    fn interface_module() -> XirModule {
        let mut module = account_module();
        module.version = move_model_exchange::XIR_VERSION;
        module
    }

    /// The error `module` is refused with as a `.lean` dependency, which keeps
    /// its bodies, if any.
    fn dependency_error(module: &XirModule) -> Option<String> {
        let mut module = module.clone();
        for function in &mut module.functions {
            function.source_map = None;
        }
        parse_source_with_target(
            PathBuf::from("dependency.xir.json"),
            String::new(),
            &serde_json::to_string(&module).unwrap(),
            false,
        )
        .err()
        .map(|error| format!("{error:#}"))
    }

    /// The error `module` is refused with as an interface, if any.
    fn interface_error(module: &XirModule) -> Option<String> {
        parse_interface(
            PathBuf::from("dependency.xir.json"),
            &serde_json::to_string(module).unwrap(),
        )
        .err()
        .map(|error| format!("{error:#}"))
    }

    /// A function type's abilities are validated like any other, since the
    /// interface generator writes them into Move source.
    #[test]
    fn a_function_type_has_only_known_abilities() {
        let payload = || {
            Ty::Function(vec![], vec![], vec![
                "copy } fun injected(): u64 { abort 42; /*".to_owned(),
            ])
        };
        let mut in_local = interface_module();
        in_local.functions[0].locals.push(payload());
        let mut in_field = interface_module();
        in_field.structs[0].fields[0].ty = payload();
        for module in [in_local, in_field] {
            let error = interface_error(&module).expect("an unknown ability is refused");
            assert!(error.contains("invalid ability"), "{error}");
        }
        // `key` is an ability, but not one a function type can have.
        let with_abilities = |abilities: &[&str]| {
            let mut module = interface_module();
            for function in &mut module.functions {
                function.blocks = vec![];
            }
            module.functions[0].locals.push(Ty::Function(
                vec![],
                vec![],
                abilities.iter().map(|a| a.to_string()).collect(),
            ));
            interface_error(&module)
        };
        let key = with_abilities(&["key"]).expect("`key` is refused on a function type");
        assert!(key.contains("function type"), "{key}");
        assert_eq!(with_abilities(&["copy", "drop", "store"]), None);
    }

    /// A function without a body needs version 9, which records its calls;
    /// an older interface would hide what its functions reach.
    #[test]
    fn an_interface_needs_the_version_that_records_calls() {
        let mut module = interface_module();
        for function in &mut module.functions {
            function.blocks = vec![];
        }
        assert!(interface_error(&module).is_none());
        module.version = 8;
        let refused = interface_error(&module).expect("a version 8 interface is refused");
        assert!(refused.contains("version 9"), "{refused}");
    }

    /// A dependency may omit bodies, but a body it does carry is translated,
    /// so its entry block must exist.
    #[test]
    fn a_dependency_body_must_have_its_entry_block() {
        let mut module = interface_module();
        module.functions[0].entry = module.functions[0].blocks.len();
        let error = dependency_error(&module).expect("an out-of-range entry block is refused");
        assert!(error.contains("entry block is out of range"), "{error}");
        // Without a body, there is nothing to translate.
        module.functions[0].blocks = vec![];
        assert!(dependency_error(&module).is_none());
    }

    /// A struct type has as many type arguments as its struct declares,
    /// wherever it is written, including a struct of another module.
    #[test]
    fn a_struct_type_has_its_declared_number_of_type_arguments() {
        #[derive(Debug, Clone, Copy)]
        enum Place {
            Local,
            Return,
            Field,
            TypeArgument,
        }
        use Place::*;
        // Structs 0 and 1 are the golden module's, 2 is `G<T> { x: T }`, and
        // 3 is the standard library's `Option<Element>`.
        let module = |place: Place, ty: Ty| {
            let mut module = instruction_module(Instr::Nop);
            assert_eq!(module.structs.len(), 2);
            let mut generic = module.structs[0].clone();
            generic.name = "G".to_owned();
            generic.type_parameters = vec![TypeParameterDecl {
                name: "T".to_owned(),
                abilities: vec![],
                phantom: false,
            }];
            generic.fields[0].ty = Ty::TypeParameter(0);
            module.structs.push(generic.clone());
            module.external_structs = vec![move_model_exchange::XirExternalStruct {
                address: "0x1".to_owned(),
                module: "option".to_owned(),
                name: "Option".to_owned(),
            }];
            let function = &mut module.functions[0];
            match place {
                Local => function.locals.push(ty),
                Return => {
                    function.locals.push(ty.clone());
                    function.returns = vec![ty];
                    function.blocks[0].term = Term::Ret(vec![function.locals.len() - 1]);
                },
                Field => {
                    let mut with_field = generic;
                    with_field.name = "H".to_owned();
                    with_field.type_parameters = vec![];
                    with_field.fields[0].ty = ty;
                    module.structs.push(with_field);
                },
                // `exists<G<ty>>(l4)` into the `bool` l9.
                TypeArgument => {
                    function.blocks[0].instrs =
                        vec![Instr::Call(vec![9], Oper::ExistsInst(2, vec![ty]), vec![4])]
                },
            }
            module
        };
        let load = |module: &XirModule| -> Result<()> {
            let options = Options {
                dependencies: move_stdlib::move_stdlib_files(),
                named_address_mapping: vec!["std=0x1".to_owned()],
                ..Options::default()
            };
            let mut env = crate::run_checker(options)?;
            let source = parse_source(
                PathBuf::from("test.xir.json"),
                String::new(),
                &serde_json::to_string(module).unwrap(),
            )?;
            import_sources(&mut env, &[source], &mut FunctionTargetsHolder::default())
        };
        let g = |args: Vec<Ty>| Ty::StructInst(2, args);
        let option = |args: Vec<Ty>| Ty::StructInst(3, args);
        let cases = [
            (true, Local, g(vec![Ty::U64])),
            (false, Local, Ty::Struct(2)),
            (false, Local, g(vec![Ty::U64, Ty::U64])),
            (false, Local, Ty::StructInst(0, vec![Ty::U64])),
            (false, Local, Ty::Vector(Box::new(Ty::Struct(2)))),
            (false, Local, Ty::Ref(Box::new(g(vec![])))),
            (false, Local, Ty::Enum(2)),
            (true, Local, option(vec![Ty::U64])),
            (false, Local, Ty::Struct(3)),
            (false, Local, option(vec![g(vec![])])),
            (true, Return, g(vec![Ty::U64])),
            (false, Return, Ty::Struct(2)),
            (true, Field, g(vec![Ty::U64])),
            (false, Field, Ty::Struct(2)),
            (true, TypeArgument, g(vec![Ty::U64])),
            (false, TypeArgument, Ty::Struct(2)),
        ];
        let wrong: Vec<_> = cases
            .into_iter()
            .filter_map(|(ok, place, ty)| {
                let result = load(&module(place, ty.clone()));
                let as_expected = match &result {
                    Ok(()) => ok,
                    Err(error) => !ok && format!("{error:#}").contains("type arguments, but"),
                };
                (!as_expected).then(|| format!("{place:?} {ty:?}: {result:?}"))
            })
            .collect();
        assert!(wrong.is_empty(), "{wrong:#?}");
    }

    /// `<` on an integer is a native comparison and loads without the
    /// standard library; on any other type it is lowered through `std::cmp`.
    #[test]
    fn only_a_non_integer_less_than_needs_cmp() {
        let lt = |local| load_instruction(Instr::Call(vec![9], Oper::Lt, vec![local, local]));
        let mut wrong = vec![];
        for local in [0, 3, 5, 6, 7, 8] {
            if let Err(error) = lt(local) {
                wrong.push(format!("l{local} was rejected: {error:#}"));
            }
        }
        for local in [4, 9, 10, 11, 12] {
            match lt(local) {
                Ok(()) => wrong.push(format!("l{local} loaded without `cmp`")),
                Err(error) if format!("{error:#}").contains("`cmp`") => {},
                Err(error) => wrong.push(format!("l{local}: {error:#}")),
            }
        }
        assert!(wrong.is_empty(), "{wrong:#?}");
    }

    /// With `std::cmp` loaded, an integer `<` translates to a native `Lt` and
    /// a non-integer one to a call into `cmp`, and the function's call graph
    /// agrees with the code. The Move standard library here has no `cmp`, so
    /// the parts the reader uses are copied from Aptos's.
    #[test]
    fn only_a_non_integer_less_than_calls_cmp() {
        struct TempFile(PathBuf);
        impl Drop for TempFile {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
        let cmp =
            TempFile(std::env::temp_dir().join(format!("xir_cmp_{}.move", std::process::id())));
        std::fs::write(
            &cmp.0,
            "module std::cmp {
                enum Ordering has copy, drop { Less, Equal, Greater }
                native public fun compare<T>(first: &T, second: &T): Ordering;
                public fun is_lt(self: &Ordering): bool { self is Ordering::Less }
            }",
        )
        .unwrap();
        let mut dependencies = move_stdlib::move_stdlib_files();
        dependencies.push(cmp.0.to_string_lossy().into_owned());
        // Whether the call graph, and the code, call into `cmp`, and whether
        // the code has a native `Lt`.
        let translate = |local| -> (bool, bool, bool) {
            let module = instruction_module(Instr::Call(vec![9], Oper::Lt, vec![local, local]));
            let options = crate::Options {
                dependencies: dependencies.clone(),
                named_address_mapping: vec!["std=0x1".to_owned()],
                ..crate::Options::default()
            };
            let mut env = crate::run_checker(options).unwrap();
            let source = parse_source(
                PathBuf::from("test.xir.json"),
                String::new(),
                &serde_json::to_string(&module).unwrap(),
            )
            .unwrap();
            let mut targets = FunctionTargetsHolder::default();
            import_sources(&mut env, &[source], &mut targets).unwrap();
            let pool = env.symbol_pool();
            let imported = env
                .get_modules()
                .find(|m| m.get_name().name() == pool.make(&module.module.name))
                .unwrap();
            let function = imported
                .find_function(pool.make(&module.functions[0].name))
                .unwrap();
            let graph = function
                .get_called_functions()
                .unwrap()
                .iter()
                .any(|callee| env.get_module(callee.module_id).is_cmp());
            let target = targets.get_target(&function, &FunctionVariant::Baseline);
            let calls = |test: &dyn Fn(&StacklessOperation) -> bool| {
                target
                    .get_bytecode()
                    .iter()
                    .any(|bytecode| matches!(bytecode, Bytecode::Call(_, _, op, _, _) if test(op)))
            };
            let code = calls(
                &|op| matches!(op, StacklessOperation::Function(mid, _, _) if env.get_module(*mid).is_cmp()),
            );
            let native = calls(&|op| matches!(op, StacklessOperation::Lt));
            (graph, code, native)
        };
        let mut wrong = vec![];
        for local in [0, 3, 5, 6, 7, 8] {
            let outcome = translate(local);
            if outcome != (false, false, true) {
                wrong.push(format!(
                    "integer l{local}: (graph, code, native) = {outcome:?}"
                ));
            }
        }
        for local in [4, 9, 10, 11, 12] {
            let outcome = translate(local);
            if outcome != (true, true, false) {
                wrong.push(format!(
                    "non-integer l{local}: (graph, code, native) = {outcome:?}"
                ));
            }
        }
        assert!(wrong.is_empty(), "{wrong:#?}");
    }

    /// XIR calling into a Move module compiled in the same run, following the
    /// steps of `run_move_compiler` and reporting after each phase as it does:
    /// an ordinary function compiles, its warnings reported by the Move checks
    /// and the XIR function's by the XIR checks; an inline function is rejected.
    #[test]
    fn xir_calls_into_move_compiled_in_the_same_run() {
        struct TempFile(PathBuf);
        impl Drop for TempFile {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
        let math =
            TempFile(std::env::temp_dir().join(format!("xir_math_{}.move", std::process::id())));
        std::fs::write(
            &math.0,
            "module 0x42::math {
                public fun plain(a: u64, b: u64): u64 { let x = a; x = b; x }
                public inline fun max(a: u64, b: u64): u64 { if (a > b) a else b }
            }",
        )
        .unwrap();
        // The result, and the diagnostics reported after the Move checks and
        // after the XIR checks.
        let compile = |callee: &str| -> (Result<usize>, String, String) {
            let options = Options {
                sources: vec![math.0.to_string_lossy().into_owned()],
                dependencies: move_stdlib::move_stdlib_files(),
                named_address_mapping: vec!["std=0x1".to_owned()],
                ..Options::default()
            };
            let mut env = crate::run_checker(options.clone()).unwrap();
            crate::env_check_and_transform_pipeline(&options).run(&mut env);
            let mut targets = crate::run_stackless_bytecode_gen(&env);
            crate::run_stackless_bytecode_pipeline(
                &env,
                crate::stackless_bytecode_check_pipeline(&options),
                &mut targets,
            );
            let report = |env: &GlobalEnv| {
                let mut out = codespan_reporting::term::termcolor::Buffer::no_color();
                env.report_diag(&mut out, codespan_reporting::diagnostic::Severity::Warning);
                String::from_utf8_lossy(&out.into_inner()).to_string()
            };
            let move_checks = report(&env);
            crate::env_optimization_pipeline(&options).run(&mut env);
            let mut targets = crate::run_stackless_bytecode_gen(&env);
            let mut module =
                instruction_module(Instr::Call(vec![2], Oper::Function(1), vec![0, 1]));
            module.functions[0].params = 2;
            // The result goes to a named local that is never read, which the
            // XIR checks report.
            let locals = module.functions[0].locals.len();
            module.functions[0].local_names = (0..locals)
                .map(|local| (local == 2).then(|| "unread".to_owned()))
                .collect();
            module.functions.truncate(1);
            module.external_functions = vec![move_model_exchange::XirExternalFunction {
                address: "0x42".to_owned(),
                module: "math".to_owned(),
                function: callee.to_owned(),
            }];
            let source = parse_source(
                PathBuf::from("calls.xir.json"),
                String::new(),
                &serde_json::to_string(&module).unwrap(),
            )
            .unwrap();
            let checked = crate::import_and_check_xir(&mut env, &options, &[source]);
            let xir_checks = report(&env);
            let result = checked.map(|mut xir_targets| {
                crate::merge_xir_targets(&mut targets, &mut xir_targets);
                crate::run_stackless_bytecode_pipeline(
                    &env,
                    crate::stackless_bytecode_optimization_pipeline(&options),
                    &mut targets,
                );
                let units = crate::annotate_units(crate::run_file_format_gen(&mut env, &targets));
                crate::run_bytecode_verifier(&units, &mut env);
                units.len()
            });
            (result, move_checks, xir_checks)
        };
        let (plain, move_checks, xir_checks) = compile("plain");
        assert!(
            matches!(plain, Ok(2))
                && move_checks.contains("`x` is unused")
                && !xir_checks.contains("`x` is unused")
                && xir_checks.contains("`unread` is unused"),
            "{plain:?}\nMove checks:\n{move_checks}\nXIR checks:\n{xir_checks}"
        );
        let (max, ..) = compile("max");
        let error = format!("{:#}", max.unwrap_err());
        assert!(
            error.contains("`0x42::math::max` has no bytecode"),
            "{error}"
        );
    }

    /// A call into another module follows the source compiler's visibility
    /// rule. A package function is callable from the same package, which makes
    /// the XIR module a friend of the callee, but not from a dependency's.
    #[test]
    fn calls_respect_the_callee_visibility() {
        struct TempFile(PathBuf);
        impl Drop for TempFile {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
        let write = |name: &str, text: &str| {
            let file = TempFile(
                std::env::temp_dir()
                    .join(format!("xir_visibility_{name}_{}.move", std::process::id())),
            );
            std::fs::write(&file.0, text).unwrap();
            file
        };
        let callee_text = |module: &str| {
            format!(
                "module 0x0::{module} {{
                    public fun open(): u64 {{ 1 }}
                    fun closed(): u64 {{ 2 }}
                    public(package) fun for_package(): u64 {{ 4 }}
                }}"
            )
        };
        // `package` and `friendly` are compiled with the XIR module (Move does
        // not allow package and friend functions in one module); `dependency`
        // is only a dependency, loaded because `user` calls it.
        let package = write("package", &callee_text("package"));
        let friendly = write(
            "friendly",
            "module 0x0::friendly { public(friend) fun for_friends(): u64 { 3 } }",
        );
        let dependency = write("dependency", &callee_text("dependency"));
        let user = write(
            "user",
            "module 0x0::user { public fun f(): u64 { 0x0::dependency::open() } }",
        );
        let path = |file: &TempFile| file.0.to_string_lossy().into_owned();
        let cases = [
            ("package", "open", None),
            ("package", "closed", Some("is private to module")),
            ("friendly", "for_friends", Some("(not a friend of")),
            ("package", "for_package", None),
            ("dependency", "open", None),
            (
                "dependency",
                "for_package",
                Some("cannot be called from a different package"),
            ),
        ];
        let mut wrong = vec![];
        for (callee, function, expected) in cases {
            // l0 is a `u64` for the callee's result; the external function's id
            // follows the module's own two.
            let mut module = instruction_module(Instr::Call(vec![0], Oper::Function(2), vec![]));
            module.external_functions = vec![move_model_exchange::XirExternalFunction {
                address: "0x0".to_owned(),
                module: callee.to_owned(),
                function: function.to_owned(),
            }];
            // Sources, not dependencies, are always loaded.
            let options = Options {
                sources: vec![path(&package), path(&friendly), path(&user)],
                dependencies: [move_stdlib::move_stdlib_files(), vec![path(&dependency)]].concat(),
                named_address_mapping: vec!["std=0x1".to_owned()],
                ..Options::default()
            };
            let mut env = crate::run_checker(options.clone()).unwrap();
            // The stubs must be legal Move, as the source checks judge it.
            crate::env_check_and_transform_pipeline(&options).run(&mut env);
            assert!(!env.has_errors(), "the Move stubs do not compile");
            let source = parse_source(
                PathBuf::from("visibility.xir.json"),
                String::new(),
                &serde_json::to_string(&module).unwrap(),
            )
            .unwrap();
            let result = import_sources(&mut env, &[source], &mut FunctionTargetsHolder::default());
            match (expected, result) {
                (None, Ok(())) => {},
                (Some(fragment), Err(error)) if format!("{error:#}").contains(fragment) => {},
                (_, result) => wrong.push(format!("`{callee}::{function}`: {result:?}")),
            }
            // The compiled callee must name the XIR module as a friend exactly
            // when the call relies on package visibility.
            if callee == "package" {
                let pool = env.symbol_pool();
                let find = |name: &str| {
                    env.get_modules()
                        .find(|m| m.get_name().name() == pool.make(name))
                        .unwrap()
                };
                let friended = find("package").has_friend(&find(&module.module.name).get_id());
                if friended != (function == "for_package") {
                    wrong.push(format!("`{callee}::{function}`: friended = {friended}"));
                }
            }
        }
        assert!(wrong.is_empty(), "{wrong:#?}");
    }

    /// Calls to functions without bytecode: a native is callable; an inline
    /// function or a lemma is not, since only source calls get expanded.
    #[test]
    fn calls_to_functions_without_bytecode() {
        struct TempFile(PathBuf);
        impl Drop for TempFile {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
        let helpers = TempFile(
            std::env::temp_dir().join(format!("xir_no_bytecode_{}.move", std::process::id())),
        );
        std::fs::write(
            &helpers.0,
            "module 0x0::helpers {
                public inline fun twice(x: u64): u64 { x + x }
                spec module {
                    lemma stays(x: u64) { ensures x == x; }
                }
            }",
        )
        .unwrap();
        // (address, module, function, type arguments, destinations, sources,
        // expected error); l0 and l1 are `u64`, l10 a `vector<u64>`.
        let cases: [(
            &str,
            &str,
            &str,
            Vec<Ty>,
            Vec<usize>,
            Vec<usize>,
            Option<&str>,
        ); 3] = [
            (
                "0x1",
                "vector",
                "empty",
                vec![Ty::U64],
                vec![10],
                vec![],
                None,
            ),
            (
                "0x0",
                "helpers",
                "twice",
                vec![],
                vec![0],
                vec![1],
                Some("has no bytecode"),
            ),
            (
                "0x0",
                "helpers",
                "stays",
                vec![],
                vec![],
                vec![1],
                Some("has no bytecode"),
            ),
        ];
        let mut wrong = vec![];
        for (address, module_name, function, args, dsts, srcs, expected) in cases {
            let oper = if args.is_empty() {
                Oper::Function(2)
            } else {
                Oper::FunctionInst(2, args)
            };
            let mut module = instruction_module(Instr::Call(dsts, oper, srcs));
            module.external_functions = vec![move_model_exchange::XirExternalFunction {
                address: address.to_owned(),
                module: module_name.to_owned(),
                function: function.to_owned(),
            }];
            // A source, not a dependency: unused dependencies are not loaded.
            let options = Options {
                sources: vec![helpers.0.to_string_lossy().into_owned()],
                dependencies: move_stdlib::move_stdlib_files(),
                named_address_mapping: vec!["std=0x1".to_owned()],
                ..Options::default()
            };
            let mut env = crate::run_checker(options).unwrap();
            let source = parse_source(
                PathBuf::from("no_bytecode.xir.json"),
                String::new(),
                &serde_json::to_string(&module).unwrap(),
            )
            .unwrap();
            let result = import_sources(&mut env, &[source], &mut FunctionTargetsHolder::default());
            match (expected, result) {
                (None, Ok(())) => {},
                (Some(fragment), Err(error)) if format!("{error:#}").contains(fragment) => {},
                (_, result) => wrong.push(format!("`{module_name}::{function}`: {result:?}")),
            }
        }
        assert!(wrong.is_empty(), "{wrong:#?}");
    }

    /// Compiles `module` through the production stackless and file-format
    /// pipelines.
    fn compile_module(module: &XirModule) -> move_binary_format::CompiledModule {
        let source = parse_source(
            PathBuf::from("compiled.xir.json"),
            String::new(),
            &serde_json::to_string(module).unwrap(),
        )
        .unwrap();
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        import_sources(&mut env, &[source], &mut targets).unwrap();
        let options = Options::default();
        env.set_extension(options.clone());
        crate::run_stackless_bytecode_pipeline(
            &env,
            crate::stackless_bytecode_optimization_pipeline(&options),
            &mut targets,
        );
        let units = crate::run_file_format_gen(&mut env, &targets);
        assert!(!env.has_errors());
        let legacy_move_compiler::compiled_unit::CompiledUnit::Module(unit) = &units[0] else {
            panic!("expected module")
        };
        unit.module.clone()
    }

    /// A struct's visibility reaches the model; a document without the field,
    /// written before version 7, reads as private.
    #[test]
    fn struct_visibility_reaches_the_model() {
        let visibility = |module: &XirModule| {
            let env = import_module(module).unwrap();
            let name = env.symbol_pool().make(&module.structs[0].name);
            env.get_module(ModuleId::new(0))
                .find_struct(name)
                .unwrap()
                .get_visibility()
        };
        let mut wrong = vec![];
        for (xir, model) in [
            (XirVisibility::Private, MoveVisibility::Private),
            (XirVisibility::Public, MoveVisibility::Public),
            (XirVisibility::Friend, MoveVisibility::Friend),
        ] {
            let mut module = account_module();
            module.version = move_model_exchange::XIR_VERSION;
            module.structs[0].visibility = xir;
            if visibility(&module) != model {
                wrong.push(format!("{xir:?} did not arrive as {model:?}"));
            }
        }
        assert!(wrong.is_empty(), "{wrong:#?}");
        // A version 6 document has no `visibility` on its structs.
        let mut json = serde_json::to_value({
            let mut module = account_module();
            module.version = 6;
            module.structs[0].visibility = XirVisibility::Public;
            module
        })
        .unwrap();
        json["structs"][0]
            .as_object_mut()
            .unwrap()
            .remove("visibility")
            .unwrap();
        let module: XirModule = serde_json::from_value(json).unwrap();
        assert_eq!(visibility(&module), MoveVisibility::Private);
    }

    /// As in Move source, a resource cannot be public or friend.
    #[test]
    fn a_resource_cannot_be_public_or_friend() {
        let mut wrong = vec![];
        // Struct 1, `Balance`, has `key`; struct 0 does not.
        for (index, visibility, rejected) in [
            (1, XirVisibility::Public, true),
            (1, XirVisibility::Friend, true),
            (1, XirVisibility::Private, false),
            (0, XirVisibility::Public, false),
        ] {
            let mut module = account_module();
            module.version = move_model_exchange::XIR_VERSION;
            module.structs[index].visibility = visibility;
            match (rejected, import_module(&module)) {
                (false, Ok(_)) => {},
                (true, Err(error))
                    if format!("{error:#}").contains("key ability cannot have public") => {},
                (_, result) => wrong.push(format!(
                    "struct {index} {visibility:?}: {:?}",
                    result.map(|_| ())
                )),
            }
        }
        assert!(wrong.is_empty(), "{wrong:#?}");
    }

    #[test]
    fn struct_visibility_needs_its_language_version() {
        let mut module = account_module();
        module.version = move_model_exchange::XIR_VERSION;
        module.structs[1].visibility = XirVisibility::Public;
        let source = parse_source(
            PathBuf::from("old.xir.json"),
            String::new(),
            &serde_json::to_string(&module).unwrap(),
        )
        .unwrap();
        let mut env = GlobalEnv::new();
        env.set_language_version(move_model::metadata::LanguageVersion::V2_3);
        import_sources(&mut env, &[source], &mut FunctionTargetsHolder::default()).unwrap();
        let mut out = codespan_reporting::term::termcolor::Buffer::no_color();
        env.report_diag(&mut out, codespan_reporting::diagnostic::Severity::Warning);
        let warnings = String::from_utf8_lossy(&out.into_inner()).to_string();
        assert!(
            warnings.contains("visibility modifier are only supported at version"),
            "{warnings}"
        );
    }

    /// A public struct gets the pack, unpack and field functions other modules
    /// use; before version 7 the reader made every struct private and they
    /// were not generated.
    #[test]
    fn a_public_struct_gets_its_generated_functions() {
        let functions = |visibility| {
            let mut module = account_module();
            module.version = move_model_exchange::XIR_VERSION;
            module.structs[0].visibility = visibility;
            compile_module(&module).function_defs.len()
        };
        let private = functions(XirVisibility::Private);
        let public = functions(XirVisibility::Public);
        assert!(public > private, "private: {private}, public: {public}");
    }

    /// The call graph is exactly what the translated code calls, including the
    /// library calls operations are lowered to. Locals: l0-l2 `u64`, l4
    /// `address`, l9 `bool`, l10 `vector<u64>`.
    #[test]
    fn the_call_graph_matches_the_translated_code() {
        struct TempFile(PathBuf);
        impl Drop for TempFile {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
        let cmp = TempFile(
            std::env::temp_dir().join(format!("xir_graph_cmp_{}.move", std::process::id())),
        );
        std::fs::write(
            &cmp.0,
            "module std::cmp {
                enum Ordering has copy, drop { Less, Equal, Greater }
                native public fun compare<T>(first: &T, second: &T): Ordering;
                public fun is_lt(self: &Ordering): bool { self is Ordering::Less }
            }",
        )
        .unwrap();
        let cases: [(&str, Instr, &[&str]); 10] = [
            ("vec_len", Instr::Call(vec![0], Oper::VecLen, vec![10]), &[
                "vector::length",
            ]),
            (
                "vec_get",
                Instr::Call(vec![0], Oper::VecGet, vec![10, 1]),
                &["vector::borrow"],
            ),
            (
                "vec_set",
                Instr::Call(vec![10], Oper::VecSet, vec![10, 1, 2]),
                &["vector::borrow_mut"],
            ),
            (
                "vec_push",
                Instr::Call(vec![10], Oper::VecPush, vec![10, 1]),
                &["vector::push_back"],
            ),
            (
                "vec_pop",
                Instr::Call(vec![10, 0], Oper::VecPop, vec![10]),
                &["vector::pop_back"],
            ),
            (
                "vec_insert",
                Instr::Call(vec![10], Oper::VecInsert, vec![10, 1, 2]),
                &["vector::push_back", "vector::swap"],
            ),
            (
                "vec_remove",
                Instr::Call(vec![10, 0], Oper::VecRemove, vec![10, 1]),
                &["vector::swap", "vector::pop_back"],
            ),
            (
                "vec_swap",
                Instr::Call(vec![10], Oper::VecSwap, vec![10, 1, 2]),
                &["vector::swap"],
            ),
            ("integer lt", Instr::Call(vec![9], Oper::Lt, vec![0, 1]), &[
            ]),
            ("address lt", Instr::Call(vec![9], Oper::Lt, vec![4, 4]), &[
                "cmp::compare",
                "cmp::is_lt",
            ]),
        ];
        let mut wrong = vec![];
        for (label, instr, expected) in cases {
            let mut module = instruction_module(instr);
            module.functions[0].params = 13;
            let options = Options {
                dependencies: [move_stdlib::move_stdlib_files(), vec![cmp
                    .0
                    .to_string_lossy()
                    .into_owned()]]
                .concat(),
                named_address_mapping: vec!["std=0x1".to_owned()],
                ..Options::default()
            };
            let mut env = crate::run_checker(options).unwrap();
            let source = parse_source(
                PathBuf::from("graph.xir.json"),
                String::new(),
                &serde_json::to_string(&module).unwrap(),
            )
            .unwrap();
            let mut targets = FunctionTargetsHolder::default();
            if let Err(error) = import_sources(&mut env, &[source], &mut targets) {
                wrong.push(format!("{label}: {error:#}"));
                continue;
            }
            let pool = env.symbol_pool();
            let function = env
                .get_modules()
                .find(|m| m.get_name().name() == pool.make(&module.module.name))
                .unwrap()
                .find_function(pool.make(&module.functions[0].name))
                .unwrap();
            let name = |id: QualifiedId<FunId>| env.get_function(id).get_full_name_str();
            let graph: BTreeSet<String> = function
                .get_called_functions()
                .unwrap()
                .iter()
                .map(|id| name(*id))
                .collect();
            let target = targets.get_target(&function, &FunctionVariant::Baseline);
            let code: BTreeSet<String> = target
                .get_bytecode()
                .iter()
                .filter_map(|bytecode| match bytecode {
                    Bytecode::Call(_, _, StacklessOperation::Function(m, f, _), _, _) => {
                        Some(name(m.qualified(*f)))
                    },
                    _ => None,
                })
                .collect();
            // The transitive-callee targets and the pipelines read the used set.
            let used: BTreeSet<String> = function
                .get_used_functions()
                .unwrap()
                .iter()
                .map(|id| name(*id))
                .collect();
            let expected: BTreeSet<String> = expected.iter().map(|s| s.to_string()).collect();
            if graph != code || used != code || !expected.is_subset(&code) {
                wrong.push(format!(
                    "{label}: called {graph:?}, used {used:?}, code {code:?}"
                ));
            }
        }
        assert!(wrong.is_empty(), "{wrong:#?}");
    }

    /// A vector operation in XIR lowers to a call the Move code of the same
    /// compilation may also make. The callee's target is then in both target
    /// holders, and merging them keeps the Move pipeline's.
    #[test]
    fn xir_and_move_calling_the_same_vector_function_merge() {
        struct TempFile(PathBuf);
        impl Drop for TempFile {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
        let source = TempFile(
            std::env::temp_dir().join(format!("xir_vector_user_{}.move", std::process::id())),
        );
        std::fs::write(
            &source.0,
            "module 0x99::m { public fun f(v: &vector<u64>): u64 { std::vector::length(v) } }",
        )
        .unwrap();
        let options = Options {
            sources: vec![source.0.to_string_lossy().into_owned()],
            dependencies: move_stdlib::move_stdlib_files(),
            named_address_mapping: vec!["std=0x1".to_owned()],
            ..Options::default()
        };
        let mut env = crate::run_checker(options).unwrap();
        let mut targets = crate::run_stackless_bytecode_gen(&env);
        let mut module = instruction_module(Instr::Call(vec![0], Oper::VecLen, vec![10]));
        module.functions[0].params = 13;
        let xir = parse_source(
            PathBuf::from("vector.xir.json"),
            String::new(),
            &serde_json::to_string(&module).unwrap(),
        )
        .unwrap();
        let mut xir_targets = FunctionTargetsHolder::default();
        import_sources(&mut env, &[xir], &mut xir_targets).unwrap();
        let length = env
            .get_modules()
            .find(|m| m.is_std_vector())
            .unwrap()
            .find_function(env.symbol_pool().make("length"))
            .unwrap()
            .get_qualified_id();
        assert!(
            targets.get_funs().any(|id| id == length)
                && xir_targets.get_funs().any(|id| id == length),
            "both holders have `vector::length`"
        );
        crate::merge_xir_targets(&mut targets, &mut xir_targets);
        assert_eq!(targets.get_funs().filter(|id| *id == length).count(), 1);
    }

    /// A `<` without its two operands is an error, not a panic.
    #[test]
    fn a_malformed_less_than_is_an_error() {
        let mut wrong = vec![];
        for srcs in [vec![], vec![0], vec![0, 1, 2]] {
            let result = std::panic::catch_unwind(|| {
                load_instruction(Instr::Call(vec![9], Oper::Lt, srcs.clone()))
            });
            match result {
                Ok(Err(error)) if format!("{error:#}").contains("sources") => {},
                other => wrong.push(format!("{srcs:?}: {other:?}")),
            }
        }
        assert!(wrong.is_empty(), "{wrong:#?}");
    }

    /// As in the source compiler, package visibility applies only to modules
    /// being compiled: their calls are checked and they become package
    /// friends. A dependency was checked in its own build, and is never a
    /// friend of the package, even in whole-program mode, where every module
    /// counts as a target.
    #[test]
    fn package_rules_apply_only_to_modules_being_compiled() {
        struct TempFile(PathBuf);
        impl Drop for TempFile {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
        let write = |name: &str, text: &str| {
            let file = TempFile(
                std::env::temp_dir()
                    .join(format!("xir_package_{name}_{}.move", std::process::id())),
            );
            std::fs::write(&file.0, text).unwrap();
            file
        };
        // `package` is compiled; `library` is a dependency of another package
        // at the same address, kept loaded because `user` calls it.
        let package = write(
            "package",
            "module 0x0::package { public(package) fun for_package(): u64 { 4 } }",
        );
        let library = write(
            "library",
            "module 0x0::library {
                public fun open(): u64 { 1 }
                public(package) fun for_package(): u64 { 4 }
            }",
        );
        let user = write(
            "user",
            "module 0x0::user { public fun f(): u64 { 0x0::library::open() } }",
        );
        let path = |file: &TempFile| file.0.to_string_lossy().into_owned();
        // (case, callee module, whole program)
        let cases = [
            ("a dependency calls its own package", "library", false),
            ("a dependency calls into the package", "package", false),
            ("whole program", "package", true),
        ];
        let mut wrong = vec![];
        for (case, callee, whole_program) in cases {
            let options = Options {
                sources: vec![path(&package), path(&user)],
                dependencies: [move_stdlib::move_stdlib_files(), vec![path(&library)]].concat(),
                named_address_mapping: vec!["std=0x1".to_owned()],
                ..Options::default()
            };
            let mut env = crate::run_checker(options).unwrap();
            if whole_program {
                env.treat_everything_as_target(true);
            }
            let mut module = instruction_module(Instr::Call(vec![0], Oper::Function(2), vec![]));
            module.external_functions = vec![move_model_exchange::XirExternalFunction {
                address: "0x0".to_owned(),
                module: callee.to_owned(),
                function: "for_package".to_owned(),
            }];
            // Loaded as a dependency, not a target.
            let source = parse_source_with_target(
                PathBuf::from("dependency.xir.json"),
                String::new(),
                &serde_json::to_string(&module).unwrap(),
                false,
            )
            .unwrap();
            if let Err(error) =
                import_sources(&mut env, &[source], &mut FunctionTargetsHolder::default())
            {
                wrong.push(format!("{case}: {error:#}"));
                continue;
            }
            let pool = env.symbol_pool();
            let find = |name: &str| {
                env.get_modules()
                    .find(|m| m.get_name().name() == pool.make(name))
                    .unwrap()
            };
            if find(callee).has_friend(&find(&module.module.name).get_id()) {
                wrong.push(format!(
                    "{case}: the dependency was made a friend of `{callee}`"
                ));
            }
        }
        assert!(wrong.is_empty(), "{wrong:#?}");
    }

    #[test]
    fn source_map_locations_reach_stackless_bytecode() {
        let mut module = account_module();
        module.version = move_model_exchange::XIR_VERSION;
        let function = &mut module.functions[0];
        function.local_names = (0..function.locals.len())
            .map(|index| (index == 0).then(|| "owner".to_owned()))
            .collect();
        function.source_map = Some(move_model_exchange::XirFunctionSourceMap {
            span: Some(XirSourceSpan { start: 1, end: 90 }),
            blocks: function
                .blocks
                .iter()
                .map(|block| move_model_exchange::XirBlockSourceMap {
                    instrs: block
                        .instrs
                        .iter()
                        .map(|_| Some(XirSourceSpan { start: 10, end: 20 }))
                        .collect(),
                    term: Some(XirSourceSpan { start: 30, end: 40 }),
                })
                .collect(),
        });
        let source = parse_source(
            PathBuf::from("located.xir.json"),
            " ".repeat(100),
            &serde_json::to_string(&module).unwrap(),
        )
        .unwrap();
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        import_sources(&mut env, &[source], &mut targets).unwrap();
        let module_env = env.get_module(ModuleId::new(0));
        let function = module_env.get_functions().next().unwrap();
        assert_eq!(function.get_loc().span(), Span::new(1, 90));
        let target = targets.get_target(&function, &FunctionVariant::Baseline);
        assert_eq!(
            target
                .get_local_name(0)
                .display(target.symbol_pool())
                .to_string(),
            "owner"
        );
        assert_eq!(target.get_local_name_opt(0).as_deref(), Some("owner"));
        assert_eq!(target.get_local_name_opt(1), None);
        assert_eq!(target.get_local_name_for_error_message(0), "local `owner`");
        assert_eq!(target.get_local_name_for_error_message(1), "value");
        let internal_name = target.symbol_pool().make("_l1");
        assert_eq!(target.get_local_index(internal_name), Some(1));
        let code = target.get_bytecode();
        assert_eq!(
            target.get_bytecode_loc(code[0].get_attr_id()).span(),
            Span::new(1, 90)
        );
        assert_eq!(
            target.get_bytecode_loc(code[1].get_attr_id()).span(),
            Span::new(10, 20)
        );
        assert_eq!(
            target.get_bytecode_loc(code[2].get_attr_id()).span(),
            Span::new(10, 20)
        );
    }

    /// An interface carrying source spans is accepted, because it is parsed
    /// without the text those spans index.
    ///
    /// A producer such as Lean emits a source map for every non-native
    /// function. Validating those spans against the empty string that
    /// `parse_interface` supplies would reject every non-zero one, so a
    /// perfectly good module could not be used as an XIR dependency at all.
    /// The same document is still checked when parsed as a compilation
    /// *target*, where real source text is available.
    #[test]
    fn an_interface_with_source_spans_is_accepted() {
        let mut module = account_module();
        module.version = move_model_exchange::XIR_VERSION;
        let function = &mut module.functions[0];
        function.source_map = Some(move_model_exchange::XirFunctionSourceMap {
            span: Some(XirSourceSpan { start: 0, end: 7 }),
            blocks: function
                .blocks
                .iter()
                .map(|block| move_model_exchange::XirBlockSourceMap {
                    instrs: vec![None; block.instrs.len()],
                    term: None,
                })
                .collect(),
        });
        let json = serde_json::to_string(&module).unwrap();

        parse_interface(PathBuf::from("spans.xir.json"), &json)
            .expect("an interface is parsed without the text its spans index");

        // And the check still bites where the text is actually supplied.
        let error = parse_source(PathBuf::from("spans.xir.json"), String::new(), &json)
            .err()
            .expect("a target with no text cannot satisfy a non-zero span");
        assert!(
            error.to_string().contains("outside the source text"),
            "{error}"
        );
    }

    #[test]
    fn rejects_misaligned_or_out_of_bounds_source_maps() {
        let mut module = account_module();
        module.version = move_model_exchange::XIR_VERSION;
        module.functions[0].source_map = Some(move_model_exchange::XirFunctionSourceMap {
            span: None,
            blocks: vec![],
        });
        let error = parse_source(
            PathBuf::from("misaligned-source-map.xir.json"),
            "source".to_owned(),
            &serde_json::to_string(&module).unwrap(),
        )
        .err()
        .expect("misaligned source map should be rejected");
        assert!(error.to_string().contains("has 0 blocks; expected"));

        let mut module = account_module();
        module.version = move_model_exchange::XIR_VERSION;
        let function = &mut module.functions[0];
        function.source_map = Some(move_model_exchange::XirFunctionSourceMap {
            span: Some(XirSourceSpan { start: 0, end: 7 }),
            blocks: function
                .blocks
                .iter()
                .map(|block| move_model_exchange::XirBlockSourceMap {
                    instrs: vec![None; block.instrs.len()],
                    term: None,
                })
                .collect(),
        });
        let error = parse_source(
            PathBuf::from("out-of-bounds-source-map.xir.json"),
            "source".to_owned(),
            &serde_json::to_string(&module).unwrap(),
        )
        .err()
        .expect("out-of-bounds source map should be rejected");
        assert!(error.to_string().contains("outside the source text"));

        let mut module = account_module();
        module.version = move_model_exchange::XIR_VERSION;
        module.functions[0].local_names = vec![Some("owner".to_owned())];
        let error = parse_source(
            PathBuf::from("misaligned-local-names.xir.json"),
            "source".to_owned(),
            &serde_json::to_string(&module).unwrap(),
        )
        .err()
        .expect("misaligned local names should be rejected");
        assert!(error.to_string().contains("local names; expected"));
    }

    /// Lean producers write names that are not Move identifiers, such as `x'`.
    /// A compilation target with such names reads, as it did before interfaces.
    #[test]
    fn a_target_reads_local_names_that_are_not_identifiers() {
        let mut module = account_module();
        module.version = move_model_exchange::XIR_VERSION;
        let locals = module.functions[0].locals.len();
        module.functions[0].local_names = (0..locals).map(|i| Some(format!("x{i}'"))).collect();
        assert!(parse_source(
            PathBuf::from("lean-names.xir.json"),
            "source".to_owned(),
            &serde_json::to_string(&module).unwrap(),
        )
        .is_ok());
    }

    #[test]
    fn bodyless_native_function_loads_as_native() {
        let mut module = account_module();
        let function_name = module.functions[0].name.clone();
        module.functions[0].is_native = true;
        module.functions[0].blocks.clear();
        module.functions[0].entry = 0;
        let source = parse_source(
            PathBuf::from("native.xir.json"),
            String::new(),
            &serde_json::to_string(&module).unwrap(),
        )
        .unwrap();
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        import_sources(&mut env, &[source], &mut targets).unwrap();
        let function_symbol = env.symbol_pool().make(&function_name);
        let function = env
            .get_module(ModuleId::new(0))
            .find_function(function_symbol)
            .unwrap();
        assert!(function.is_native());
    }

    #[test]
    fn get_field_preserves_non_droppable_struct() {
        let mut module = account_module();
        let function = &mut module.functions[0];
        function.params = 2;
        function.locals = vec![Ty::Signer, Ty::Struct(1), Ty::Struct(0)];
        function.blocks = vec![Block {
            instrs: vec![
                Instr::Call(vec![2], Oper::GetField(0), vec![1]),
                Instr::Call(vec![], Oper::MoveTo(1), vec![0, 1]),
            ],
            term: Term::Ret(vec![]),
        }];
        let source = parse_source(
            PathBuf::from("get-field.xir.json"),
            String::new(),
            &serde_json::to_string(&module).unwrap(),
        )
        .unwrap();
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        import_sources(&mut env, &[source], &mut targets).unwrap();
        let options = Options::default();
        env.set_extension(options.clone());
        crate::run_stackless_bytecode_pipeline(
            &env,
            crate::stackless_bytecode_check_pipeline(&options),
            &mut targets,
        );
        assert!(!env.has_errors());
        crate::run_stackless_bytecode_pipeline(
            &env,
            crate::stackless_bytecode_optimization_pipeline(&options),
            &mut targets,
        );
        assert!(!env.has_errors());
        let units = crate::run_file_format_gen(&mut env, &targets);
        assert!(!env.has_errors());
        let legacy_move_compiler::compiled_unit::CompiledUnit::Module(module) = &units[0] else {
            panic!("expected module")
        };
        move_bytecode_verifier::verify_module(&module.module).unwrap();
    }

    #[test]
    fn move_to_accepts_an_existing_signer_reference() {
        let mut module = account_module();
        let function = &mut module.functions[0];
        function.params = 2;
        function.returns.clear();
        function.locals = vec![Ty::Ref(Box::new(Ty::Signer)), Ty::Struct(1)];
        function.acquires.clear();
        function.blocks = vec![Block {
            instrs: vec![Instr::Call(vec![], Oper::MoveTo(1), vec![0, 1])],
            term: Term::Ret(vec![]),
        }];
        import_and_verify(module);
    }

    #[test]
    fn rejects_invalid_read_ref_arity() {
        let mut module = account_module();
        module.functions[0].blocks[0].instrs[0] = Instr::Call(vec![], Oper::ReadRef, vec![]);
        let source = parse_source(
            PathBuf::from("invalid-read-ref.xir.json"),
            String::new(),
            &serde_json::to_string(&module).unwrap(),
        )
        .unwrap();
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        let error = import_sources(&mut env, &[source], &mut targets).unwrap_err();
        assert!(format!("{error:#}").contains("ReadRef expects 1 destinations and 1 sources"));
    }

    #[test]
    fn rejects_invalid_vector_and_global_operation_arities() {
        for (oper, message) in [
            (Oper::VecPack, "vec_pack expects one destination"),
            (
                Oper::MoveTo(0),
                "MoveTo(0) expects 0 destinations and 2 sources",
            ),
            (
                Oper::MoveFrom(0),
                "MoveFrom(0) expects 1 destinations and 1 sources",
            ),
            (
                Oper::Exists(0),
                "Exists(0) expects 1 destinations and 1 sources",
            ),
            (
                Oper::BorrowGlobal(0),
                "BorrowGlobal(0) expects 1 destinations and 1 sources",
            ),
        ] {
            let mut module = account_module();
            module.functions[0].blocks[0].instrs[0] = Instr::Call(vec![], oper, vec![]);
            let source = parse_source(
                PathBuf::from("invalid-operation-arity.xir.json"),
                String::new(),
                &serde_json::to_string(&module).unwrap(),
            )
            .unwrap();
            let mut env = GlobalEnv::new();
            let mut targets = FunctionTargetsHolder::default();
            let error = import_sources(&mut env, &[source], &mut targets).unwrap_err();
            assert!(format!("{error:#}").contains(message));
        }
    }

    #[test]
    fn rejects_out_of_range_type_parameter() {
        let mut module = account_module();
        module.functions[0].locals[0] = Ty::TypeParameter(0);
        let error = parse_source(
            PathBuf::from("invalid-type-parameter.xir.json"),
            String::new(),
            &serde_json::to_string(&module).unwrap(),
        )
        .err()
        .expect("out-of-range type parameter should be rejected");
        assert!(error
            .to_string()
            .contains("type parameter index 0 is out of range"));
    }

    #[test]
    fn rejects_invalid_entry_and_branch_targets() {
        let mut module = account_module();
        module.functions[0].entry = 1;
        let error = parse_source(
            PathBuf::from("invalid-entry.xir.json"),
            String::new(),
            &serde_json::to_string(&module).unwrap(),
        )
        .err()
        .expect("out-of-range entry block should be rejected");
        assert!(error.to_string().contains("entry block is out of range"));

        let mut module = account_module();
        module.functions[0].blocks[0].term = Term::Jump(99);
        let source = parse_source(
            PathBuf::from("invalid-jump.xir.json"),
            String::new(),
            &serde_json::to_string(&module).unwrap(),
        )
        .unwrap();
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        let error = import_sources(&mut env, &[source], &mut targets).unwrap_err();
        assert!(format!("{error:#}").contains("block 99 is out of range"));
    }

    #[test]
    fn rejects_invalid_local_and_return_arity() {
        let mut module = account_module();
        module.functions[0].blocks[0].instrs[0] = Instr::Assign(0, 99);
        let source = parse_source(
            PathBuf::from("invalid-local.xir.json"),
            String::new(),
            &serde_json::to_string(&module).unwrap(),
        )
        .unwrap();
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        let error = import_sources(&mut env, &[source], &mut targets).unwrap_err();
        assert!(format!("{error:#}").contains("local l99 is out of range"));

        let mut module = account_module();
        module.functions[0].blocks[0].term = Term::Ret(vec![0]);
        let source = parse_source(
            PathBuf::from("invalid-return.xir.json"),
            String::new(),
            &serde_json::to_string(&module).unwrap(),
        )
        .unwrap();
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        let error = import_sources(&mut env, &[source], &mut targets).unwrap_err();
        assert!(format!("{error:#}").contains("return arity mismatch"));
    }

    #[test]
    fn rejects_unknown_struct_and_invalid_address_constant() {
        let mut module = account_module();
        module.functions[0].blocks[0].instrs[0] = Instr::Call(vec![1], Oper::Exists(99), vec![0]);
        let source = parse_source(
            PathBuf::from("invalid-struct.xir.json"),
            String::new(),
            &serde_json::to_string(&module).unwrap(),
        )
        .unwrap();
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        let error = import_sources(&mut env, &[source], &mut targets).unwrap_err();
        assert!(format!("{error:#}").contains("struct id 99 is out of range"));

        let mut module = account_module();
        module.functions[0].locals[0] = Ty::Address;
        module.functions[0].blocks[0].instrs[0] =
            Instr::Load(0, Constant::Address("not-an-address".to_string()));
        let source = parse_source(
            PathBuf::from("invalid-address.xir.json"),
            String::new(),
            &serde_json::to_string(&module).unwrap(),
        )
        .unwrap();
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        let error = import_sources(&mut env, &[source], &mut targets).unwrap_err();
        assert!(format!("{error:#}").contains("invalid address constant `not-an-address`"));
    }

    #[test]
    fn rejects_call_with_wrong_type_argument_count() {
        let mut module = account_module();
        module.functions[1].type_parameters = vec![move_model_exchange::TypeParameter {
            name: "T".to_string(),
            abilities: vec![],
            phantom: false,
        }];
        module.functions[0].blocks[0].instrs[0] =
            Instr::Call(vec![], Oper::Function(1), vec![0, 1]);
        let source = parse_source(
            PathBuf::from("type-arguments.xir.json"),
            String::new(),
            &serde_json::to_string(&module).unwrap(),
        )
        .unwrap();
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        let error = import_sources(&mut env, &[source], &mut targets).unwrap_err();
        assert!(format!("{error:#}").contains("takes 1 type arguments, but the call supplies 0"));
    }

    #[test]
    fn vector_update_peephole_keeps_live_skipped_locals() {
        assert!(local_is_read_after(
            1,
            &[Instr::Assign(2, 1)],
            &Term::Ret(vec![]),
        ));
        assert!(local_is_read_after(1, &[], &Term::Ret(vec![1])));
        assert!(!local_is_read_after(
            1,
            &[Instr::Load(1, Constant::Num("0".to_string()))],
            &Term::Ret(vec![]),
        ));
    }
}
