// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! XIR support for compiler-v2.
//!
//! This reader registers a versioned deployable XIR module in the Move model
//! and translates its function bodies directly to baseline stackless bytecode.
//! Source frontends are responsible for producing the JSON; the ordinary
//! compiler-v2 stackless checks, optimizations, file-format generator, and
//! verifier own all later compilation stages.

use anyhow::{bail, ensure, Context, Result};
use codespan::Span;
use move_binary_format::file_format::Visibility as MoveVisibility;
use move_command_line_common::files::FileHash;
use move_core_types::{
    ability::{Ability, AbilitySet},
    account_address::AccountAddress,
    identifier::Identifier,
};
use move_model::{
    ast::{Address, Attribute, AttributeValue, ModuleName, Value},
    model::{
        FieldData, FunId, FunctionKind, GlobalEnv, Loc, ModuleId, Parameter, QualifiedId, StructId,
        TypeParameter, TypeParameterKind,
    },
    ty::{PrimitiveType, ReferenceKind, Type},
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

pub fn parse_source(path: PathBuf, text: String, json: &str) -> Result<XirSource> {
    parse_source_with_target(path, text, json, true)
}

pub(crate) fn parse_source_with_target(
    path: PathBuf,
    text: String,
    json: &str,
    is_target: bool,
) -> Result<XirSource> {
    let module: XirModule = serde_json::from_str(json)
        .with_context(|| format!("invalid XIR from `{}`", path.display()))?;
    validate(&module)?;
    validate_source_maps(&module, &text)?;
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

fn validate(module: &XirModule) -> Result<()> {
    module.check_version().map_err(anyhow::Error::msg)?;
    ensure!(
        module.module.dialect == XirDialect::Stackless,
        "only stackless XIR is deployable"
    );
    AccountAddress::from_hex_literal(&module.module.address)
        .with_context(|| format!("invalid module address `{}`", module.module.address))?;
    valid_identifier("module", &module.module.name)?;
    let mut struct_names = BTreeSet::new();
    for decl in &module.structs {
        valid_identifier("struct", &decl.name)?;
        ensure!(
            struct_names.insert(&decl.name),
            "duplicate struct `{}`",
            decl.name
        );
        let mut field_names = BTreeSet::new();
        for field in &decl.fields {
            valid_identifier("field", &field.name)?;
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
                for field in &variant.fields {
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
        if !decl.is_native {
            ensure!(
                !decl.blocks.is_empty(),
                "function `{}` has no blocks",
                decl.name
            );
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
    let args = match operation {
        Oper::PackInst(args)
        | Oper::UnpackInst(args)
        | Oper::PackVariantInst(_, args)
        | Oper::UnpackVariantInst(_, args)
        | Oper::TestVariantInst(_, args)
        | Oper::GetFieldInst(_, args)
        | Oper::GetGlobalInst(_, args)
        | Oper::MoveToInst(_, args)
        | Oper::MoveFromInst(_, args)
        | Oper::ExistsInst(_, args)
        | Oper::FunctionInst(_, args)
        | Oper::BorrowFieldInst(_, args)
        | Oper::BorrowGlobalInst(_, args) => args,
        _ => return Ok(()),
    };
    for arg in args {
        validate_type_parameters(arg, count, &format!("function `{function}` operation"))?;
    }
    Ok(())
}

fn valid_identifier(kind: &str, name: &str) -> Result<()> {
    Identifier::new(name)
        .map(|_| ())
        .with_context(|| format!("invalid {kind} identifier `{name}`"))
}

pub fn import_sources(
    env: &mut GlobalEnv,
    sources: &[XirSource],
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
        let source = &sources[index];
        import_source(env, source, targets)
            .with_context(|| format!("loading XIR from `{}`", source.path.display()))?;
        imported[index] = true;
        imported_count += 1;
    }
    env.set_function_size_estimates(targets.compute_function_size_estimates());
    Ok(())
}

fn external_modules_available(env: &GlobalEnv, xir: &XirModule) -> bool {
    xir.external_functions.iter().all(|reference| {
        let Ok(address) = AccountAddress::from_hex_literal(&reference.address) else {
            return false;
        };
        let name = ModuleName::new(
            Address::Numerical(address),
            env.symbol_pool().make(&reference.module),
        );
        env.find_module(&name).is_some()
    })
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
                    XirAttributeArg::Num { .. } | XirAttributeArg::Bool { .. } => {
                        bail!("attribute `{name}` has an unnamed literal argument")
                    },
                })
                .collect::<Result<Vec<_>>>()?,
        )),
    }
}

fn import_source(
    env: &mut GlobalEnv,
    source: &XirSource,
    targets: &mut FunctionTargetsHolder,
) -> Result<()> {
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
                ty: model_type(&field.ty, module_id, &struct_ids)?,
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
                        ty: model_type(&field.ty, module_id, &struct_ids)?,
                        is_ghost: false,
                        init: None,
                    });
                }
            }
        }
        structs.push(ModelXirStructData {
            name: struct_id.symbol(),
            loc: loc.clone(),
            abilities: ability_set(decl)?,
            type_parameters: model_type_parameters(env, &loc, &decl.type_parameters)?,
            fields,
            variants,
            visibility: MoveVisibility::Private,
        });
    }

    let mut functions = vec![];
    for (decl, fun_id) in xir.functions.iter().zip(&function_ids) {
        let function_loc = loc_for_span(&loc, decl.source_map.as_ref().and_then(|map| map.span));
        let local_types = decl
            .locals
            .iter()
            .map(|ty| model_type(ty, module_id, &struct_ids))
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
        let returns = Type::tuple(
            decl.returns
                .iter()
                .map(|ty| model_type(ty, module_id, &struct_ids))
                .collect::<Result<Vec<_>>>()?,
        );
        let acquired = decl
            .acquires
            .iter()
            .map(|id| struct_at(&struct_ids, *id, &decl.name))
            .collect::<Result<BTreeSet<_>>>()?;
        let called = called_functions(env, xir, decl, module_id, &function_ids)?;
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
            called_funs: called,
        });
    }

    let added_id = env.load_xir_module(ModelXirModuleData {
        loc,
        name: module_name,
        structs,
        functions,
    })?;
    ensure!(
        added_id == module_id,
        "model assigned an unexpected module id"
    );
    for (decl, fun_id) in xir.functions.iter().zip(&function_ids) {
        let qid = module_id.qualified(*fun_id);
        let data = translate_function(env, xir, module_id, &struct_ids, &function_ids, decl, qid)?;
        targets.insert_target_data(&qid, FunctionVariant::Baseline, data);
    }
    add_transitive_callee_targets(env, module_id, targets);
    Ok(())
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

fn model_type(ty: &Ty, module_id: ModuleId, structs: &[StructId]) -> Result<Type> {
    Ok(match ty {
        Ty::Bool => Type::Primitive(PrimitiveType::Bool),
        Ty::U8 => Type::Primitive(PrimitiveType::U8),
        Ty::U16 => Type::Primitive(PrimitiveType::U16),
        Ty::U32 => Type::Primitive(PrimitiveType::U32),
        Ty::U64 => Type::Primitive(PrimitiveType::U64),
        Ty::U128 => Type::Primitive(PrimitiveType::U128),
        Ty::U256 => Type::Primitive(PrimitiveType::U256),
        Ty::I8 | Ty::I16 | Ty::I32 | Ty::I64 | Ty::I128 | Ty::I256 => {
            bail!("signed integer types are not representable in the move-model type system")
        },
        Ty::Address => Type::Primitive(PrimitiveType::Address),
        Ty::Signer => Type::Primitive(PrimitiveType::Signer),
        Ty::TypeParameter(index) => {
            ensure!(
                *index <= u16::MAX as usize,
                "type parameter index is too large"
            );
            Type::TypeParameter(*index as u16)
        },
        Ty::Struct(id) => Type::Struct(
            module_id,
            *structs
                .get(*id)
                .with_context(|| format!("struct id {id} is out of range"))?,
            vec![],
        ),
        Ty::StructInst(id, args) => Type::Struct(
            module_id,
            *structs
                .get(*id)
                .with_context(|| format!("struct id {id} is out of range"))?,
            args.iter()
                .map(|arg| model_type(arg, module_id, structs))
                .collect::<Result<Vec<_>>>()?,
        ),
        Ty::Enum(id) => Type::Struct(
            module_id,
            *structs
                .get(*id)
                .with_context(|| format!("enum id {id} is out of range"))?,
            vec![],
        ),
        Ty::EnumInst(id, args) => Type::Struct(
            module_id,
            *structs
                .get(*id)
                .with_context(|| format!("enum id {id} is out of range"))?,
            args.iter()
                .map(|arg| model_type(arg, module_id, structs))
                .collect::<Result<Vec<_>>>()?,
        ),
        Ty::Vector(element) => Type::Vector(Box::new(model_type(element, module_id, structs)?)),
        Ty::Ref(referent) => Type::Reference(
            ReferenceKind::Immutable,
            Box::new(model_type(referent, module_id, structs)?),
        ),
        Ty::MutRef(referent) => Type::Reference(
            ReferenceKind::Mutable,
            Box::new(model_type(referent, module_id, structs)?),
        ),
    })
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

fn called_functions(
    env: &GlobalEnv,
    xir: &XirModule,
    decl: &FunctionDecl,
    module_id: ModuleId,
    functions: &[FunId],
) -> Result<BTreeSet<QualifiedId<FunId>>> {
    let mut called = BTreeSet::new();
    let mut uses_generic_comparison = false;
    for block in &decl.blocks {
        for instr in &block.instrs {
            if let Instr::Call(_, Oper::Function(id) | Oper::FunctionInst(id, _), _) = instr {
                called.insert(function_at(env, xir, module_id, functions, *id)?);
            }
            if let Instr::Call(_, Oper::Lt, srcs) = instr {
                let source_type = srcs
                    .first()
                    .and_then(|id| decl.locals.get(*id))
                    .with_context(|| format!("malformed comparison in `{}`", decl.name))?;
                uses_generic_comparison |= !matches!(source_type, Ty::U64);
            }
        }
    }
    if uses_generic_comparison {
        let module = env
            .get_modules()
            .find(|module| module.is_cmp())
            .context("generic comparison requires the standard `cmp` module")?;
        for name in ["compare", "is_lt"] {
            let function = module
                .find_function(env.symbol_pool().make(name))
                .with_context(|| format!("the standard `cmp` module has no `{name}` function"))?;
            called.insert(module.get_id().qualified(function.get_id()));
        }
    }
    Ok(called)
}

fn struct_at(structs: &[StructId], id: usize, function: &str) -> Result<StructId> {
    structs
        .get(id)
        .copied()
        .with_context(|| format!("struct id {id} is out of range in `{function}`"))
}

fn translate_function(
    env: &GlobalEnv,
    xir: &XirModule,
    module_id: ModuleId,
    struct_ids: &[StructId],
    function_ids: &[FunId],
    decl: &FunctionDecl,
    qid: QualifiedId<FunId>,
) -> Result<TargetFunctionData> {
    let func_env = env.get_function(qid);
    let mut translator = FunctionTranslator {
        env,
        xir,
        module_id,
        struct_ids,
        function_ids,
        decl,
        loc: func_env.get_loc(),
        function_loc: func_env.get_loc(),
        code: vec![],
        locations: BTreeMap::new(),
        local_types: decl
            .locals
            .iter()
            .map(|ty| model_type(ty, module_id, struct_ids))
            .collect::<Result<Vec<_>>>()?,
        next_attr: 0,
        next_label: decl.blocks.len(),
    };
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
            .map(|ty| model_type(ty, module_id, struct_ids))
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
    struct_ids: &'a [StructId],
    function_ids: &'a [FunId],
    decl: &'a FunctionDecl,
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
            Oper::Lt if !self.local(srcs[0])?.is_number() => {
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
            Oper::Add(_)
            | Oper::Sub(_)
            | Oper::Mul(_)
            | Oper::Div(_)
            | Oper::Mod(_)
            | Oper::BitAnd(_)
            | Oper::BitOr(_)
            | Oper::BitXor(_)
            | Oper::Shl(_)
            | Oper::Shr(_)
            | Oper::Lt
            | Oper::Le
            | Oper::Eq
            | Oper::And
            | Oper::Or => {
                arity(dsts, srcs, 1, 2, oper)?;
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
                match target {
                    IntType::U8 => StacklessOperation::CastU8,
                    IntType::U16 => StacklessOperation::CastU16,
                    IntType::U32 => StacklessOperation::CastU32,
                    IntType::U64 => StacklessOperation::CastU64,
                    IntType::U128 => StacklessOperation::CastU128,
                    IntType::U256 => StacklessOperation::CastU256,
                    IntType::I8
                    | IntType::I16
                    | IntType::I32
                    | IntType::I64
                    | IntType::I128
                    | IntType::I256 => {
                        bail!("signed integer casts are not representable in stackless bytecode")
                    },
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
            Oper::Function(id) => {
                let target =
                    function_at(self.env, self.xir, self.module_id, self.function_ids, *id)?;
                StacklessOperation::Function(target.module_id, target.id, vec![])
            },
            Oper::FunctionInst(id, args) => {
                let target =
                    function_at(self.env, self.xir, self.module_id, self.function_ids, *id)?;
                StacklessOperation::Function(target.module_id, target.id, self.type_args(args)?)
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
        args.iter()
            .map(|arg| model_type(arg, self.module_id, self.struct_ids))
            .collect()
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
