// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Mechanical Rustc Public to RawUnit mapping for the supported scalar CFG.

use super::exchange::*;
use rustc_public::{
    crate_def::{CrateDef, CrateDefType},
    mir::{
        AggregateKind, AssertMessage, BinOp, Body, BorrowKind as MirBorrowKind, CastKind,
        ConstOperand, Mutability, Operand, PointerCoercion, ProjectionElem, Rvalue, Safety,
        StatementKind, TerminatorKind, UnOp, UnwindAction, WithRetag,
    },
    target::MachineInfo,
    ty::{
        Abi, AdtDef, AdtKind, ConstantKind, FnDef, FnSig, GenericArgKind, GenericParamDefKind,
        IntTy, PolyFnSig, Region, RegionKind, RigidTy, Span, Ty as RustTy, TyConst, TyConstKind,
        TyKind, UintTy, VariantIdx,
    },
    CrateItem, ItemKind,
};
use serde_json::Number;
use std::{collections::BTreeMap, fs};

pub(super) fn raw_unit_json() -> Result<String, String> {
    let unit = UnitMapper::new()?.map()?;
    let text = super::exchange::encode(&unit)?;
    let decoded = super::exchange::decode(&text)?;
    let round_trip = super::exchange::encode(&decoded)?;
    if round_trip == text {
        Ok(text)
    } else {
        Err("Rust RawUnit mirror is not deterministically round-trippable".to_owned())
    }
}

struct UnitMapper {
    items: Vec<CrateItem>,
    identities: Vec<String>,
    name_ids: BTreeMap<String, NameId>,
    names: Vec<QualifiedName>,
    adts: Vec<AdtNames>,
    types: TypeLayout,
    sources: SourceTables,
    arenas: NamespaceArenas,
}

fn function_def(item: CrateItem) -> Result<FnDef, String> {
    let ty = item.ty();
    ty.kind()
        .fn_def()
        .map(|(definition, _)| definition)
        .ok_or_else(|| {
            format!(
                "function item `{}` does not have a function-definition type",
                item.name()
            )
        })
}

impl UnitMapper {
    fn new() -> Result<Self, String> {
        let mut items = rustc_public::all_local_items()
            .into_iter()
            .filter(|item| item.kind() == ItemKind::Fn)
            .collect::<Vec<_>>();
        items.sort_by_key(CrateDef::name);
        if items.is_empty() {
            return Err("the Rust mapper requires at least one local function".to_owned());
        }
        for item in &items {
            let generics = function_def(*item)?.generics_of();
            if item.requires_monomorphization() && generics.params.is_empty() {
                return Err(format!(
                    "function `{}` requires monomorphization without public generic binders",
                    item.name()
                ));
            }
            for parameter in &generics.params {
                match parameter.kind {
                    GenericParamDefKind::Lifetime
                    | GenericParamDefKind::Type {
                        synthetic: false, ..
                    } => {},
                    GenericParamDefKind::Type {
                        synthetic: true, ..
                    } => {
                        return Err(format!(
                            "function `{}` has synthetic type parameter `{}`; `impl Trait` generic binders are not yet supported",
                            item.name(), parameter.name
                        ));
                    },
                    GenericParamDefKind::Const { .. } => {
                        return Err(format!(
                            "function `{}` has const parameter `{}` whose declared type is not exposed by the pinned Rustc Public API",
                            item.name(), parameter.name
                        ));
                    },
                }
            }
            if item.body().is_none() {
                return Err(format!("function `{}` has no public MIR body", item.name()));
            }
        }
        let identities = items.iter().map(CrateDef::name).collect::<Vec<_>>();
        let name_ids = identities
            .iter()
            .enumerate()
            .map(|(index, identity)| (identity.clone(), NameId::new(index)))
            .collect();
        let bodies = items
            .iter()
            .map(|item| item.body().expect("checked above"))
            .collect::<Vec<_>>();
        let adt_defs = collect_adts(&bodies)?;
        let (names, adts) = build_names(&identities, &adt_defs);
        let types = TypeLayout::from_bodies(&items, &bodies, &adts)?;
        Ok(Self {
            items,
            identities,
            name_ids,
            names,
            adts,
            types,
            sources: SourceTables::default(),
            arenas: NamespaceArenas::default(),
        })
    }

    fn map(mut self) -> Result<RawUnit, String> {
        let mut functions = Vec::with_capacity(self.items.len());
        let mut origins = Vec::with_capacity(self.items.len());
        let mut alignments = Vec::with_capacity(self.items.len());
        for index in 0..self.items.len() {
            let item = self.items[index];
            let identity = self.identities[index].clone();
            let body = item.body().expect("checked in UnitMapper::new");
            let function_loc = self.sources.intern(item.span())?;
            let generics = function_def(item)?
                .generics_of()
                .params
                .into_iter()
                .map(|parameter| {
                    let kind = match parameter.kind {
                        GenericParamDefKind::Lifetime => BinderKind::Lifetime,
                        GenericParamDefKind::Type {
                            synthetic: false, ..
                        } => BinderKind::TypeArg,
                        GenericParamDefKind::Type {
                            synthetic: true, ..
                        }
                        | GenericParamDefKind::Const { .. } => {
                            unreachable!(
                                "unsupported function binders were rejected in UnitMapper::new"
                            )
                        },
                    };
                    GenericBinder {
                        abilities: vec![],
                        kind,
                        loc: function_loc,
                        name: parameter.name,
                        predicates: vec![],
                        type_: None,
                    }
                })
                .collect();
            let mapped = FunctionMapper::new(
                &body,
                &self.types,
                &self.name_ids,
                &self.adts,
                &mut self.sources,
                &mut self.arenas,
            )?
            .map()?;
            functions.push(FunctionDecl {
                alignment: AlignmentId::new(index),
                attributes: vec![],
                body: mapped.body,
                contract: empty_contract(),
                doc: String::new(),
                loc: function_loc,
                locals: mapped.locals,
                name: NameId::new(index),
                origin: OriginId::new(index),
                pragmas: vec![],
                profile: Profile::Rust,
                profile_data: vec![],
                signature: Signature {
                    generics,
                    parameters: mapped.parameters,
                    predicates: vec![],
                    results: vec![TypeUse {
                        loc: mapped.result_loc,
                        type_id: mapped.result_type,
                    }],
                },
            });
            origins.push(Origin {
                description: "Rustc Public CrateItem::body".to_owned(),
                kind: OriginKind::RustMir,
                location: function_loc,
                source_identity: Some(identity),
            });
            alignments.push(Alignment {
                description: "Rustc Public optimized generic MIR".to_owned(),
                source: OriginId::new(index),
                trust: Trust::Checked,
            });
        }

        let namespace_loc = functions[0].loc;
        let structs = map_structs(&self.adts, &self.types, &mut self.sources)?;
        let comments = self.sources.comments();
        let local_crate = rustc_public::local_crate();
        let target_pointer_width = MachineInfo::target_pointer_width().bits();
        let lifetimes = self
            .types
            .lifetimes
            .iter()
            .map(|lifetime| Lifetime {
                kind: lifetime.kind.clone(),
                loc: namespace_loc,
                name: lifetime.name.clone(),
            })
            .collect();
        Ok(RawUnit {
            dependencies: vec![],
            evidence: vec![ImportEvidence {
                description: format!(
                    "rustc accepted this crate through analysis and exposed optimized generic MIR for a {target_pointer_width}-bit pointer target"
                ),
                producer: "leaner-rust-export via rustc Public".to_owned(),
                trusted: true,
            }],
            namespaces: vec![RawNamespace {
                associated_items: vec![],
                attributes: vec![],
                comments,
                constants: vec![],
                doc: String::new(),
                expressions: self.arenas.expressions,
                functions,
                identity: NamespaceId::new(0),
                implementations: vec![],
                imports: vec![],
                intrinsics: vec![],
                invariants: vec![],
                loc: namespace_loc,
                patterns: vec![],
                places: self.arenas.places,
                pragmas: vec![],
                profile: Some(Profile::Rust),
                profile_metadata: vec![],
                spec_functions: vec![],
                spec_vars: vec![],
                structs,
                traits: vec![],
            }],
            profiles: vec![ProfileConfig {
                name: "rust".to_owned(),
                options: vec![
                    ("panic".to_owned(), "abort".to_owned()),
                    ("unsafe".to_owned(), "reject".to_owned()),
                    (
                        "target_pointer_width".to_owned(),
                        target_pointer_width.to_string(),
                    ),
                ],
                profile: Profile::Rust,
                version: 2,
            }],
            tables: Tables {
                alignments,
                files: self.sources.files,
                lifetimes,
                locations: self.sources.locations,
                names: self.names,
                namespaces: vec![NamespaceRef {
                    segments: vec![local_crate.name],
                }],
                origins,
                types: self.types.types,
            },
            version: Version { major: 1, minor: 1 },
        })
    }
}

struct AdtNames {
    definition: AdtDef,
    name: NameId,
    variants: Vec<VariantNames>,
}

struct VariantNames {
    index: VariantIdx,
    ordinal: usize,
    name: Option<NameId>,
    source_name: Option<String>,
    fields: Vec<NameId>,
}

fn collect_adts(bodies: &[Body]) -> Result<Vec<AdtDef>, String> {
    let mut adts = Vec::new();
    for body in bodies {
        for local in body.locals() {
            collect_type_adts(local.ty, &mut adts)?;
        }
    }
    adts.sort_by_key(CrateDef::name);
    Ok(adts)
}

fn collect_type_adts(ty: RustTy, adts: &mut Vec<AdtDef>) -> Result<(), String> {
    match ty.kind() {
        TyKind::RigidTy(RigidTy::Tuple(elements)) => {
            for element in elements {
                collect_type_adts(element, adts)?;
            }
            Ok(())
        },
        TyKind::RigidTy(RigidTy::Array(element, _) | RigidTy::Slice(element)) => {
            collect_type_adts(element, adts)
        },
        TyKind::RigidTy(RigidTy::FnPtr(signature)) => {
            let signature = supported_function_pointer_signature(&signature)?;
            for type_ in &signature.inputs_and_output {
                collect_type_adts(*type_, adts)?;
            }
            Ok(())
        },
        TyKind::RigidTy(RigidTy::Bool)
        | TyKind::RigidTy(RigidTy::Char)
        | TyKind::RigidTy(RigidTy::Never)
        | TyKind::RigidTy(RigidTy::Str)
        | TyKind::RigidTy(RigidTy::Uint(_))
        | TyKind::RigidTy(RigidTy::Int(_)) => Ok(()),
        TyKind::Param(_) => Ok(()),
        TyKind::RigidTy(RigidTy::Ref(_, referent, _)) => collect_type_adts(referent, adts),
        TyKind::RigidTy(RigidTy::Adt(definition, arguments)) => {
            if definition.krate() != rustc_public::local_crate() {
                return Err(format!(
                    "dependency ADT `{}` requires dependency namespace import",
                    definition.name()
                ));
            }
            for argument in &arguments.0 {
                if let GenericArgKind::Type(argument) = argument {
                    collect_type_adts(*argument, adts)?;
                }
            }
            if definition.kind() == AdtKind::Union {
                return Err(format!(
                    "unsupported Rust ADT kind for `{}`: {}",
                    definition.name(),
                    definition.kind()
                ));
            }
            if adts.contains(&definition) {
                return Ok(());
            }
            adts.push(definition);
            for variant in definition.variants() {
                for field in variant.fields() {
                    collect_type_adts(field.ty(), adts)?;
                }
            }
            Ok(())
        },
        kind => Err(format!("unsupported Rust local type: {kind:?}")),
    }
}

fn supported_function_pointer_signature(signature: &PolyFnSig) -> Result<&FnSig, String> {
    if !signature.bound_vars.is_empty() {
        return Err("higher-ranked Rust function pointers are not supported".to_owned());
    }
    let signature = &signature.value;
    if signature.c_variadic {
        return Err("variadic Rust function pointers are not supported".to_owned());
    }
    if signature.safety != Safety::Safe || signature.abi != Abi::Rust {
        return Err(format!(
            "only safe Rust-ABI function pointers are supported: safety={:?}, abi={:?}",
            signature.safety, signature.abi
        ));
    }
    Ok(signature)
}

fn build_names(
    identities: &[String],
    definitions: &[AdtDef],
) -> (Vec<QualifiedName>, Vec<AdtNames>) {
    let mut names = identities
        .iter()
        .map(|identity| QualifiedName {
            name: identity.rsplit("::").next().unwrap_or(identity).to_owned(),
            namespace_id: NamespaceId::new(0),
        })
        .collect::<Vec<_>>();
    let mut adts = Vec::with_capacity(definitions.len());
    for definition in definitions {
        let name = NameId::new(names.len());
        let identity = definition.name();
        names.push(QualifiedName {
            name: identity.rsplit("::").next().unwrap_or(&identity).to_owned(),
            namespace_id: NamespaceId::new(0),
        });
        let mut variants = Vec::new();
        for (ordinal, variant) in definition.variants().into_iter().enumerate() {
            let source_name = (definition.kind() == AdtKind::Enum).then(|| variant.name());
            let variant_name = source_name.as_ref().map(|source_name| {
                let id = NameId::new(names.len());
                names.push(QualifiedName {
                    name: source_name.clone(),
                    namespace_id: NamespaceId::new(0),
                });
                id
            });
            let fields = variant
                .fields()
                .into_iter()
                .map(|field| {
                    let id = NameId::new(names.len());
                    names.push(QualifiedName {
                        name: field.name,
                        namespace_id: NamespaceId::new(0),
                    });
                    id
                })
                .collect();
            variants.push(VariantNames {
                index: variant.idx(),
                ordinal,
                name: variant_name,
                source_name,
                fields,
            });
        }
        adts.push(AdtNames {
            definition: *definition,
            name,
            variants,
        });
    }
    (names, adts)
}

fn map_structs(
    adts: &[AdtNames],
    types: &TypeLayout,
    sources: &mut SourceTables,
) -> Result<Vec<StructDecl>, String> {
    adts.iter()
        .map(|adt| {
            let loc = sources.intern(adt.definition.span())?;
            let generics = adt
                .definition
                .generics_of()
                .params
                .into_iter()
                .map(|parameter| {
                    let (kind, type_) = match parameter.kind {
                        GenericParamDefKind::Type { .. } => (BinderKind::TypeArg, None),
                        GenericParamDefKind::Lifetime => (BinderKind::Lifetime, None),
                        GenericParamDefKind::Const { .. } => (
                            BinderKind::Const,
                            Some(types.const_parameter_type(
                                adt.definition,
                                parameter.index,
                                loc,
                            )?),
                        ),
                    };
                    Ok(GenericBinder {
                        abilities: vec![],
                        kind,
                        loc,
                        name: parameter.name,
                        predicates: vec![],
                        type_,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            let (fields, variants) = if adt.definition.kind() == AdtKind::Struct {
                let names = adt.variants.first().ok_or_else(|| {
                    format!(
                        "struct `{}` has no field-bearing variant",
                        adt.definition.name()
                    )
                })?;
                (map_fields(adt, names, types, sources)?, vec![])
            } else {
                let variants = adt
                    .variants
                    .iter()
                    .map(|names| {
                        let discriminant = adt.definition.discriminant_for_variant(names.index);
                        Ok(VariantDecl {
                            discriminant: Some(integer_value(discriminant.val, discriminant.ty)?),
                            fields: map_fields(adt, names, types, sources)?,
                            loc,
                            name: names.name.ok_or_else(|| {
                                format!(
                                    "enum `{}` variant {} has no interned name",
                                    adt.definition.name(),
                                    names.ordinal
                                )
                            })?,
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                (vec![], variants)
            };
            Ok(StructDecl {
                abilities: vec![],
                attributes: vec![],
                contract: empty_contract(),
                doc: String::new(),
                fields,
                generics,
                loc,
                locals: vec![],
                name: adt.name,
                properties: vec![],
                variants,
            })
        })
        .collect()
}

fn map_fields(
    adt: &AdtNames,
    names: &VariantNames,
    types: &TypeLayout,
    sources: &mut SourceTables,
) -> Result<Vec<FieldDecl>, String> {
    let variant = adt.definition.variant(names.index).ok_or_else(|| {
        format!(
            "ADT `{}` lost variant {}",
            adt.definition.name(),
            names.ordinal
        )
    })?;
    variant
        .fields()
        .into_iter()
        .zip(&names.fields)
        .map(|(field, name)| {
            let field_loc = sources.intern(field.span())?;
            Ok(FieldDecl {
                doc: String::new(),
                loc: field_loc,
                name: *name,
                type_use: TypeUse {
                    loc: field_loc,
                    type_id: types.id(field.ty())?,
                },
            })
        })
        .collect()
}

struct TypeLayout {
    types: Vec<Ty>,
    unit: TypeId,
    never: Option<TypeId>,
    bool_: Option<TypeId>,
    character: Option<TypeId>,
    type_parameters: Vec<(u32, TypeId)>,
    integers: Vec<IntegerEntry>,
    nominals: Vec<NominalEntry>,
    composites: Vec<CompositeEntry>,
    references: Vec<ReferenceEntry>,
    lifetimes: Vec<LifetimeSpec>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IntegerType {
    Unsigned(UintTy),
    Signed(IntTy),
}

struct IntegerEntry {
    kind: IntegerType,
    id: TypeId,
}

struct NominalEntry {
    definition: AdtDef,
    ty: RustTy,
    id: TypeId,
}

struct CompositeEntry {
    ty: RustTy,
    id: TypeId,
}

struct ReferenceEntry {
    region: Region,
    referent: RustTy,
    mutability: Mutability,
    id: TypeId,
}

struct LifetimeSpec {
    region: Region,
    kind: LifetimeKind,
    name: Option<String>,
}

impl TypeLayout {
    fn from_bodies(
        items: &[CrateItem],
        bodies: &[Body],
        adts: &[AdtNames],
    ) -> Result<Self, String> {
        let mut needs_never = false;
        let mut needs_bool = false;
        let mut needs_character = false;
        let mut needed_integers = Vec::new();
        let mut needed_nominals = Vec::new();
        let mut needed_composites = Vec::new();
        let mut pending_references = Vec::new();
        for body in bodies {
            for local in body.locals() {
                observe_type(
                    local.ty,
                    &mut needs_never,
                    &mut needs_bool,
                    &mut needs_character,
                    &mut needed_integers,
                    &mut needed_nominals,
                    &mut needed_composites,
                    adts,
                    &mut pending_references,
                )?;
            }
        }
        for adt in adts {
            for variant in adt.definition.variants() {
                for field in variant.fields() {
                    observe_type(
                        field.ty(),
                        &mut needs_never,
                        &mut needs_bool,
                        &mut needs_character,
                        &mut needed_integers,
                        &mut needed_nominals,
                        &mut needed_composites,
                        adts,
                        &mut pending_references,
                    )?;
                }
            }
        }
        let mut nominal_index = 0;
        while let Some(ty) = needed_nominals.get(nominal_index).copied() {
            nominal_index += 1;
            let TyKind::RigidTy(RigidTy::Adt(definition, arguments)) = ty.kind() else {
                continue;
            };
            for variant in definition.variants() {
                for field in variant.fields() {
                    observe_type(
                        field.ty_with_args(&arguments),
                        &mut needs_never,
                        &mut needs_bool,
                        &mut needs_character,
                        &mut needed_integers,
                        &mut needed_nominals,
                        &mut needed_composites,
                        adts,
                        &mut pending_references,
                    )?;
                }
            }
        }
        needed_integers.sort_by_key(|kind| integer_order(*kind));
        needed_integers.dedup();
        let mut lifetime_regions = Vec::new();
        for (region, _, _) in &pending_references {
            if !lifetime_regions.contains(region) {
                lifetime_regions.push(region.clone());
            }
        }
        for ty in &needed_nominals {
            if let TyKind::RigidTy(RigidTy::Adt(_, arguments)) = ty.kind() {
                for argument in arguments.0 {
                    if let GenericArgKind::Lifetime(region) = argument
                        && !lifetime_regions.contains(&region)
                    {
                        lifetime_regions.push(region);
                    }
                }
            }
        }
        let mut lifetimes = lifetime_regions
            .into_iter()
            .map(lifetime_spec)
            .collect::<Vec<_>>();
        let mut types = vec![Ty::Unit];
        let never = needs_never.then(|| {
            let id = TypeId::new(types.len());
            types.push(Ty::Never);
            id
        });
        let bool_ = needs_bool.then(|| {
            let id = TypeId::new(types.len());
            types.push(Ty::Bool);
            id
        });
        let character = needs_character.then(|| {
            let id = TypeId::new(types.len());
            types.push(Ty::Character);
            id
        });
        let integers = needed_integers
            .into_iter()
            .map(|kind| {
                let id = TypeId::new(types.len());
                types.push(Ty::Integer {
                    signed: matches!(kind, IntegerType::Signed(_)),
                    width: integer_width(kind),
                });
                IntegerEntry { kind, id }
            })
            .collect::<Vec<_>>();
        let mut parameter_indices = Vec::new();
        for item in items {
            for parameter in function_def(*item)?.generics_of().params {
                if matches!(parameter.kind, GenericParamDefKind::Type { .. }) {
                    parameter_indices.push(parameter.index);
                }
            }
        }
        for adt in adts {
            for parameter in adt.definition.generics_of().params {
                if matches!(parameter.kind, GenericParamDefKind::Type { .. }) {
                    parameter_indices.push(parameter.index);
                }
            }
        }
        parameter_indices.sort_unstable();
        parameter_indices.dedup();
        let type_parameters = parameter_indices
            .into_iter()
            .map(|index| {
                let id = TypeId::new(types.len());
                types.push(Ty::TypeParameter {
                    index: index as usize,
                });
                (index, id)
            })
            .collect::<Vec<_>>();
        let mut nominals = Vec::new();
        for ty in needed_nominals {
            let TyKind::RigidTy(RigidTy::Adt(definition, arguments)) = ty.kind() else {
                return Err(format!("nominal Rust type is not an ADT: {:?}", ty.kind()));
            };
            let adt = adts
                .iter()
                .find(|adt| adt.definition == definition)
                .ok_or_else(|| format!("ADT `{}` was not interned", definition.name()))?;
            let arguments = arguments
                .0
                .into_iter()
                .map(|argument| match argument {
                    GenericArgKind::Type(argument) => interned_type_id(
                        TypeId::new(0),
                        never,
                        bool_,
                        character,
                        &type_parameters,
                        &integers,
                        &nominals,
                        &[],
                        &[],
                        argument,
                    )
                    .map(|type_id| GenericArgument::TypeArg {
                        value: TypeUse {
                            loc: LocId::new(0),
                            type_id,
                        },
                    })
                    .ok_or_else(|| {
                        format!(
                            "generic ADT type argument is not yet interned: {:?}",
                            argument.kind()
                        )
                    }),
                    GenericArgKind::Lifetime(region) => lifetimes
                        .iter()
                        .position(|entry| entry.region == region)
                        .map(|index| GenericArgument::Lifetime {
                            value: LifetimeId::new(index),
                        })
                        .ok_or_else(|| "generic ADT lifetime was not interned".to_owned()),
                    GenericArgKind::Const(value) => {
                        const_generic_value(&value).map(|value| GenericArgument::Const { value })
                    },
                })
                .collect::<Result<Vec<_>, _>>()?;
            let id = TypeId::new(types.len());
            types.push(Ty::Nominal {
                arguments,
                name: adt.name,
            });
            nominals.push(NominalEntry { definition, ty, id });
        }
        let mut composites = Vec::new();
        let mut references = Vec::new();
        loop {
            let before = composites.len() + references.len();
            for ty in &needed_composites {
                if composites
                    .iter()
                    .any(|entry: &CompositeEntry| entry.ty == *ty)
                {
                    continue;
                }
                let mapped = match ty.kind() {
                    TyKind::RigidTy(RigidTy::Tuple(elements)) => elements
                        .into_iter()
                        .map(|element| {
                            interned_type_id(
                                TypeId::new(0),
                                never,
                                bool_,
                                character,
                                &type_parameters,
                                &integers,
                                &nominals,
                                &composites,
                                &references,
                                element,
                            )
                        })
                        .collect::<Option<Vec<_>>>()
                        .map(|elements| Ty::Tuple { elements }),
                    TyKind::RigidTy(RigidTy::Array(element, length)) => {
                        let element = interned_type_id(
                            TypeId::new(0),
                            never,
                            bool_,
                            character,
                            &type_parameters,
                            &integers,
                            &nominals,
                            &composites,
                            &references,
                            element,
                        );
                        match element {
                            Some(element) => Some(Ty::Vector {
                                element,
                                length: Some(ConstValue::Integer {
                                    value: Number::from(length.eval_target_usize().map_err(
                                        |error| format!("evaluate fixed array length: {error:?}"),
                                    )?),
                                }),
                            }),
                            None => None,
                        }
                    },
                    TyKind::RigidTy(RigidTy::Slice(element)) => interned_type_id(
                        TypeId::new(0),
                        never,
                        bool_,
                        character,
                        &type_parameters,
                        &integers,
                        &nominals,
                        &composites,
                        &references,
                        element,
                    )
                    .map(|element| Ty::Vector {
                        element,
                        length: None,
                    }),
                    TyKind::RigidTy(RigidTy::Str) => Some(Ty::String),
                    TyKind::RigidTy(RigidTy::FnPtr(signature)) => {
                        let signature = supported_function_pointer_signature(&signature)?;
                        let mut mapped = signature
                            .inputs_and_output
                            .iter()
                            .map(|type_| {
                                interned_type_id(
                                    TypeId::new(0),
                                    never,
                                    bool_,
                                    character,
                                    &type_parameters,
                                    &integers,
                                    &nominals,
                                    &composites,
                                    &references,
                                    *type_,
                                )
                            })
                            .collect::<Option<Vec<_>>>();
                        mapped.as_mut().and_then(|types| {
                            let result = types.pop()?;
                            Some(Ty::Function {
                                abilities: vec![Ability::Copy, Ability::Drop],
                                arguments: std::mem::take(types),
                                result,
                            })
                        })
                    },
                    kind => return Err(format!("unsupported composite Rust type: {kind:?}")),
                };
                if let Some(mapped) = mapped {
                    let id = TypeId::new(types.len());
                    types.push(mapped);
                    composites.push(CompositeEntry { ty: *ty, id });
                }
            }
            for (region, referent, mutability) in &pending_references {
                if references.iter().any(|entry: &ReferenceEntry| {
                    entry.region == *region
                        && entry.referent == *referent
                        && entry.mutability == *mutability
                }) {
                    continue;
                }
                let Some(referent_id) = interned_type_id(
                    TypeId::new(0),
                    never,
                    bool_,
                    character,
                    &type_parameters,
                    &integers,
                    &nominals,
                    &composites,
                    &references,
                    *referent,
                ) else {
                    continue;
                };
                let lifetime = match lifetimes.iter().position(|entry| entry.region == *region) {
                    Some(index) => LifetimeId::new(index),
                    None => {
                        let id = LifetimeId::new(lifetimes.len());
                        lifetimes.push(lifetime_spec(region.clone()));
                        id
                    },
                };
                let id = TypeId::new(types.len());
                types.push(Ty::Reference {
                    value: ReferenceType {
                        kind: if *mutability == Mutability::Mut {
                            ReferenceKind::Mutable
                        } else {
                            ReferenceKind::Shared
                        },
                        lifetime,
                        profile: Profile::Rust,
                        referent: referent_id,
                    },
                });
                references.push(ReferenceEntry {
                    region: region.clone(),
                    referent: *referent,
                    mutability: *mutability,
                    id,
                });
            }
            let unresolved_composite = needed_composites
                .iter()
                .find(|ty| !composites.iter().any(|entry| entry.ty == **ty));
            let unresolved_reference = pending_references.iter().find(|candidate| {
                !references.iter().any(|entry| {
                    entry.region == candidate.0
                        && entry.referent == candidate.1
                        && entry.mutability == candidate.2
                })
            });
            if unresolved_composite.is_none() && unresolved_reference.is_none() {
                break;
            }
            if composites.len() + references.len() == before {
                return Err(format!(
                    "could not intern composite/reference type dependencies: composite={:?}, reference={:?}",
                    unresolved_composite.map(|ty| ty.kind()),
                    unresolved_reference.map(|(_, referent, _)| referent.kind())
                ));
            }
        }
        Ok(Self {
            types,
            unit: TypeId::new(0),
            never,
            bool_,
            character,
            type_parameters,
            integers,
            nominals,
            composites,
            references,
            lifetimes,
        })
    }

    fn id(&self, ty: rustc_public::ty::Ty) -> Result<TypeId, String> {
        match ty.kind() {
            TyKind::RigidTy(RigidTy::Tuple(elements)) if elements.is_empty() => Ok(self.unit),
            TyKind::RigidTy(RigidTy::Never) => self
                .never
                .ok_or_else(|| "never type was not interned".to_owned()),
            TyKind::RigidTy(RigidTy::Bool) => self
                .bool_
                .ok_or_else(|| "Bool type was not interned".to_owned()),
            TyKind::RigidTy(RigidTy::Char) => self
                .character
                .ok_or_else(|| "character type was not interned".to_owned()),
            TyKind::Param(parameter) => self
                .type_parameters
                .iter()
                .find(|(index, _)| *index == parameter.index)
                .map(|(_, id)| *id)
                .ok_or_else(|| format!("type parameter {} was not interned", parameter.index)),
            TyKind::RigidTy(RigidTy::Uint(kind)) => self
                .integers
                .iter()
                .find(|entry| entry.kind == IntegerType::Unsigned(kind))
                .map(|entry| entry.id)
                .ok_or_else(|| format!("{kind:?} type was not interned")),
            TyKind::RigidTy(RigidTy::Int(kind)) => self
                .integers
                .iter()
                .find(|entry| entry.kind == IntegerType::Signed(kind))
                .map(|entry| entry.id)
                .ok_or_else(|| format!("{kind:?} type was not interned")),
            TyKind::RigidTy(RigidTy::Tuple(elements)) if !elements.is_empty() => self
                .composites
                .iter()
                .find(|entry| entry.ty == ty)
                .map(|entry| entry.id)
                .ok_or_else(|| "tuple type was not interned".to_owned()),
            TyKind::RigidTy(
                RigidTy::Array(_, _) | RigidTy::Slice(_) | RigidTy::Str | RigidTy::FnPtr(_),
            ) => self
                .composites
                .iter()
                .find(|entry| entry.ty == ty)
                .map(|entry| entry.id)
                .ok_or_else(|| "composite type was not interned".to_owned()),
            TyKind::RigidTy(RigidTy::Adt(definition, _)) => self
                .nominals
                .iter()
                .find(|entry| entry.definition == definition && entry.ty == ty)
                .map(|entry| entry.id)
                .ok_or_else(|| format!("enum `{}` was not interned", definition.name())),
            TyKind::RigidTy(RigidTy::Ref(region, referent, mutability)) => self
                .references
                .iter()
                .find(|entry| {
                    entry.region == region
                        && entry.referent == referent
                        && entry.mutability == mutability
                })
                .map(|entry| entry.id)
                .ok_or_else(|| "reference type was not interned".to_owned()),
            kind => Err(format!("unsupported Rust scalar type: {kind:?}")),
        }
    }

    fn integer_id(&self, kind: IntegerType) -> Result<TypeId, String> {
        self.integers
            .iter()
            .find(|entry| entry.kind == kind)
            .map(|entry| entry.id)
            .ok_or_else(|| format!("{kind:?} type was not interned"))
    }

    fn const_parameter_type(
        &self,
        definition: AdtDef,
        parameter_index: u32,
        loc: LocId,
    ) -> Result<TypeUse, String> {
        let mut inferred: Option<TypeId> = None;
        for nominal in self
            .nominals
            .iter()
            .filter(|entry| entry.definition == definition)
        {
            let TyKind::RigidTy(RigidTy::Adt(_, arguments)) = nominal.ty.kind() else {
                continue;
            };
            let Some(GenericArgKind::Const(argument)) = arguments.0.get(parameter_index as usize)
            else {
                continue;
            };
            let Some(argument_type) = const_generic_type(argument) else {
                continue;
            };
            let type_id = self.id(argument_type)?;
            match inferred {
                Some(previous) if previous.index != type_id.index => {
                    return Err(format!(
                        "const parameter {} of `{}` has inconsistent observed types",
                        parameter_index,
                        definition.name()
                    ));
                },
                Some(_) => {},
                None => inferred = Some(type_id),
            }
        }
        inferred
            .map(|type_id| TypeUse { loc, type_id })
            .ok_or_else(|| {
                format!(
                    "const parameter {} of `{}` has no concrete observed type",
                    parameter_index,
                    definition.name()
                )
            })
    }

    fn generic_arguments(
        &self,
        arguments: &rustc_public::ty::GenericArgs,
        loc: LocId,
    ) -> Result<Vec<GenericArgument>, String> {
        arguments
            .0
            .iter()
            .map(|argument| match argument {
                GenericArgKind::Type(argument) => Ok(GenericArgument::TypeArg {
                    value: TypeUse {
                        loc,
                        type_id: self.id(*argument)?,
                    },
                }),
                GenericArgKind::Lifetime(region) => self
                    .lifetimes
                    .iter()
                    .position(|entry| entry.region == *region)
                    .map(|index| GenericArgument::Lifetime {
                        value: LifetimeId::new(index),
                    })
                    .ok_or_else(|| "generic lifetime argument was not interned".to_owned()),
                GenericArgKind::Const(value) => {
                    const_generic_value(value).map(|value| GenericArgument::Const { value })
                },
            })
            .collect()
    }
}

fn integer_order(kind: IntegerType) -> usize {
    match kind {
        IntegerType::Unsigned(UintTy::U8) => 0,
        IntegerType::Unsigned(UintTy::U16) => 1,
        IntegerType::Unsigned(UintTy::U32) => 2,
        IntegerType::Unsigned(UintTy::U64) => 3,
        IntegerType::Unsigned(UintTy::U128) => 4,
        IntegerType::Unsigned(UintTy::Usize) => 5,
        IntegerType::Signed(IntTy::I8) => 6,
        IntegerType::Signed(IntTy::I16) => 7,
        IntegerType::Signed(IntTy::I32) => 8,
        IntegerType::Signed(IntTy::I64) => 9,
        IntegerType::Signed(IntTy::I128) => 10,
        IntegerType::Signed(IntTy::Isize) => 11,
    }
}

fn integer_width(kind: IntegerType) -> IntWidth {
    match kind {
        IntegerType::Unsigned(UintTy::Usize) | IntegerType::Signed(IntTy::Isize) => {
            IntWidth::Pointer
        },
        IntegerType::Unsigned(kind) => IntWidth::Bits {
            width: kind.num_bytes() * 8,
        },
        IntegerType::Signed(kind) => IntWidth::Bits {
            width: kind.num_bytes() * 8,
        },
    }
}

fn interned_type_id(
    unit: TypeId,
    never: Option<TypeId>,
    bool_: Option<TypeId>,
    character: Option<TypeId>,
    type_parameters: &[(u32, TypeId)],
    integers: &[IntegerEntry],
    nominals: &[NominalEntry],
    composites: &[CompositeEntry],
    references: &[ReferenceEntry],
    ty: RustTy,
) -> Option<TypeId> {
    match ty.kind() {
        TyKind::RigidTy(RigidTy::Tuple(elements)) if elements.is_empty() => Some(unit),
        TyKind::RigidTy(RigidTy::Never) => never,
        TyKind::RigidTy(RigidTy::Bool) => bool_,
        TyKind::RigidTy(RigidTy::Char) => character,
        TyKind::Param(parameter) => type_parameters
            .iter()
            .find(|(index, _)| *index == parameter.index)
            .map(|(_, id)| *id),
        TyKind::RigidTy(RigidTy::Uint(kind)) => integers
            .iter()
            .find(|entry| entry.kind == IntegerType::Unsigned(kind))
            .map(|entry| entry.id),
        TyKind::RigidTy(RigidTy::Int(kind)) => integers
            .iter()
            .find(|entry| entry.kind == IntegerType::Signed(kind))
            .map(|entry| entry.id),
        TyKind::RigidTy(RigidTy::Adt(definition, _)) => nominals
            .iter()
            .find(|entry| entry.definition == definition && entry.ty == ty)
            .map(|entry| entry.id),
        TyKind::RigidTy(
            RigidTy::Tuple(_)
            | RigidTy::Array(_, _)
            | RigidTy::Slice(_)
            | RigidTy::Str
            | RigidTy::FnPtr(_),
        ) => composites
            .iter()
            .find(|entry| entry.ty == ty)
            .map(|entry| entry.id),
        TyKind::RigidTy(RigidTy::Ref(region, referent, mutability)) => references
            .iter()
            .find(|entry| {
                entry.region == region
                    && entry.referent == referent
                    && entry.mutability == mutability
            })
            .map(|entry| entry.id),
        _ => None,
    }
}

fn observe_type(
    ty: RustTy,
    needs_never: &mut bool,
    needs_bool: &mut bool,
    needs_character: &mut bool,
    needed_integers: &mut Vec<IntegerType>,
    needed_nominals: &mut Vec<RustTy>,
    needed_composites: &mut Vec<RustTy>,
    adts: &[AdtNames],
    references: &mut Vec<(Region, RustTy, Mutability)>,
) -> Result<(), String> {
    match ty.kind() {
        TyKind::RigidTy(RigidTy::Tuple(elements)) if elements.is_empty() => Ok(()),
        TyKind::RigidTy(RigidTy::Never) => {
            *needs_never = true;
            Ok(())
        },
        TyKind::RigidTy(RigidTy::Tuple(elements)) => {
            needed_integers.push(IntegerType::Unsigned(UintTy::U32));
            for element in elements {
                observe_type(
                    element,
                    needs_never,
                    needs_bool,
                    needs_character,
                    needed_integers,
                    needed_nominals,
                    needed_composites,
                    adts,
                    references,
                )?;
            }
            if !needed_composites.contains(&ty) {
                needed_composites.push(ty);
            }
            Ok(())
        },
        TyKind::RigidTy(RigidTy::Array(element, _)) => {
            observe_type(
                element,
                needs_never,
                needs_bool,
                needs_character,
                needed_integers,
                needed_nominals,
                needed_composites,
                adts,
                references,
            )?;
            if !needed_composites.contains(&ty) {
                needed_composites.push(ty);
            }
            Ok(())
        },
        TyKind::RigidTy(RigidTy::Slice(element)) => {
            observe_type(
                element,
                needs_never,
                needs_bool,
                needs_character,
                needed_integers,
                needed_nominals,
                needed_composites,
                adts,
                references,
            )?;
            if !needed_composites.contains(&ty) {
                needed_composites.push(ty);
            }
            Ok(())
        },
        TyKind::RigidTy(RigidTy::Str) => {
            if !needed_composites.contains(&ty) {
                needed_composites.push(ty);
            }
            Ok(())
        },
        TyKind::RigidTy(RigidTy::FnPtr(signature)) => {
            let signature = supported_function_pointer_signature(&signature)?;
            for type_ in &signature.inputs_and_output {
                observe_type(
                    *type_,
                    needs_never,
                    needs_bool,
                    needs_character,
                    needed_integers,
                    needed_nominals,
                    needed_composites,
                    adts,
                    references,
                )?;
            }
            if !needed_composites.contains(&ty) {
                needed_composites.push(ty);
            }
            Ok(())
        },
        TyKind::RigidTy(RigidTy::Bool) => {
            *needs_bool = true;
            Ok(())
        },
        TyKind::RigidTy(RigidTy::Char) => {
            *needs_character = true;
            Ok(())
        },
        TyKind::RigidTy(RigidTy::Uint(kind)) => {
            needed_integers.push(IntegerType::Unsigned(kind));
            Ok(())
        },
        TyKind::RigidTy(RigidTy::Int(kind)) => {
            needed_integers.push(IntegerType::Signed(kind));
            Ok(())
        },
        TyKind::RigidTy(RigidTy::Adt(definition, arguments))
            if adts.iter().any(|entry| entry.definition == definition) =>
        {
            for argument in arguments.0 {
                let argument_type = match argument {
                    GenericArgKind::Type(argument) => Some(argument),
                    GenericArgKind::Const(argument) => const_generic_type(&argument),
                    GenericArgKind::Lifetime(_) => None,
                };
                if let Some(argument_type) = argument_type {
                    observe_type(
                        argument_type,
                        needs_never,
                        needs_bool,
                        needs_character,
                        needed_integers,
                        needed_nominals,
                        needed_composites,
                        adts,
                        references,
                    )?;
                }
            }
            if !needed_nominals.contains(&ty) {
                needed_nominals.push(ty);
            }
            Ok(())
        },
        TyKind::Param(_) => Ok(()),
        TyKind::RigidTy(RigidTy::Ref(region, referent, mutability)) => {
            observe_type(
                referent,
                needs_never,
                needs_bool,
                needs_character,
                needed_integers,
                needed_nominals,
                needed_composites,
                adts,
                references,
            )?;
            if !references.contains(&(region.clone(), referent, mutability)) {
                references.push((region, referent, mutability));
            }
            Ok(())
        },
        kind => Err(format!("unsupported Rust local type: {kind:?}")),
    }
}

fn lifetime_spec(region: Region) -> LifetimeSpec {
    let (kind, name) = match &region.kind {
        RegionKind::ReStatic => (LifetimeKind::Static, Some("'static".to_owned())),
        RegionKind::ReEarlyParam(parameter) => (
            LifetimeKind::Parameter {
                index: parameter.index as usize,
            },
            Some(parameter.name.clone()),
        ),
        RegionKind::ReBound(_, bound) => (
            LifetimeKind::Parameter {
                index: bound.var as usize,
            },
            None,
        ),
        RegionKind::RePlaceholder(_) | RegionKind::ReErased => (LifetimeKind::Inference, None),
    };
    LifetimeSpec { region, kind, name }
}

fn const_generic_value(value: &TyConst) -> Result<ConstValue, String> {
    let TyConstKind::Value(ty, allocation) = value.kind() else {
        return Err(format!(
            "generic ADT const argument is not an evaluated scalar: {:?}",
            value.kind()
        ));
    };
    match ty.kind() {
        TyKind::RigidTy(RigidTy::Bool) => allocation
            .read_bool()
            .map(|value| ConstValue::Bool { value })
            .map_err(|error| format!("read generic ADT Bool argument: {error:?}")),
        TyKind::RigidTy(RigidTy::Char) => allocation
            .read_uint()
            .map_err(|error| format!("read generic ADT character argument: {error:?}"))
            .and_then(character_value)
            .map(|value| ConstValue::Character { value }),
        TyKind::RigidTy(RigidTy::Uint(_) | RigidTy::Int(_)) => allocation
            .read_uint()
            .map_err(|error| format!("read generic ADT integer argument: {error:?}"))
            .and_then(|value| integer_value(value, *ty))
            .map(|value| ConstValue::Integer { value }),
        kind => Err(format!(
            "unsupported evaluated generic ADT const argument type: {kind:?}"
        )),
    }
}

fn const_generic_type(value: &TyConst) -> Option<RustTy> {
    match value.kind() {
        TyConstKind::Value(ty, _) | TyConstKind::ZSTValue(ty) => Some(*ty),
        TyConstKind::Param(_) | TyConstKind::Bound(..) | TyConstKind::Unevaluated(..) => None,
    }
}

#[derive(Default)]
struct SourceTables {
    files: Vec<SourceFile>,
    file_contents: Vec<String>,
    locations: Vec<Location>,
    file_ids: BTreeMap<String, FileId>,
    location_ids: BTreeMap<(usize, usize, usize), LocId>,
    generated_location_ids: BTreeMap<usize, LocId>,
}

impl SourceTables {
    fn intern(&mut self, span: Span) -> Result<LocId, String> {
        let filename = span.get_filename();
        let file_id = match self.file_ids.get(&filename).copied() {
            Some(id) => id,
            None => {
                let source = fs::read_to_string(&filename)
                    .map_err(|error| format!("read Rust source `{filename}`: {error}"))?;
                let id = FileId::new(self.files.len());
                self.files.push(SourceFile {
                    content_hash: String::new(),
                    name: filename.clone(),
                });
                self.file_contents.push(source);
                self.file_ids.insert(filename.clone(), id);
                id
            },
        };
        let source = &self.file_contents[file_id.index];
        let lines = span.get_lines();
        if lines.start_line == 0
            || lines.start_col == 0
            || lines.end_line == 0
            || lines.end_col == 0
        {
            if let Some(id) = self.generated_location_ids.get(&file_id.index) {
                return Ok(*id);
            }
            let id = LocId::new(self.locations.len());
            self.locations.push(Location {
                expansion: vec![],
                generated_by: Some("rustc-generated span".to_owned()),
                parent: None,
                primary: None,
                related: vec![],
            });
            self.generated_location_ids.insert(file_id.index, id);
            return Ok(id);
        }
        let start_byte = byte_offset(source, lines.start_line, lines.start_col)?;
        let end_byte = byte_offset(source, lines.end_line, lines.end_col)?;
        let key = (file_id.index, start_byte, end_byte);
        if let Some(id) = self.location_ids.get(&key) {
            return Ok(*id);
        }
        let id = LocId::new(self.locations.len());
        self.locations.push(Location {
            expansion: vec![],
            generated_by: None,
            parent: None,
            primary: Some(SourceRange {
                end_byte,
                file: file_id,
                start_byte,
            }),
            related: vec![],
        });
        self.location_ids.insert(key, id);
        Ok(id)
    }

    fn comments(&mut self) -> Vec<Comment> {
        let mut comments = Vec::new();
        for file_index in 0..self.file_contents.len() {
            let source = self.file_contents[file_index].clone();
            for comment in rust_comments(&source) {
                if generated_preamble_comment(&comment.text) {
                    continue;
                }
                let loc = self.intern_range(FileId::new(file_index), comment.start, comment.end);
                comments.push(Comment {
                    is_doc: comment.is_doc,
                    loc,
                    own_line: comment.own_line,
                    text: comment.text,
                });
            }
        }
        comments
    }

    fn intern_range(&mut self, file: FileId, start_byte: usize, end_byte: usize) -> LocId {
        let key = (file.index, start_byte, end_byte);
        if let Some(id) = self.location_ids.get(&key) {
            return *id;
        }
        let id = LocId::new(self.locations.len());
        self.locations.push(Location {
            expansion: vec![],
            generated_by: None,
            parent: None,
            primary: Some(SourceRange {
                end_byte,
                file,
                start_byte,
            }),
            related: vec![],
        });
        self.location_ids.insert(key, id);
        id
    }
}

struct ScannedComment {
    start: usize,
    end: usize,
    text: String,
    is_doc: bool,
    own_line: bool,
}

fn generated_preamble_comment(text: &str) -> bool {
    matches!(
        text.trim(),
        "// Copyright © Aptos Foundation" | "// SPDX-License-Identifier: Apache-2.0"
    )
}

fn rust_comments(source: &str) -> Vec<ScannedComment> {
    let bytes = source.as_bytes();
    let mut comments = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if let Some(end) = raw_string_end(bytes, index) {
            index = end;
            continue;
        }
        if bytes[index] == b'b' && bytes.get(index + 1) == Some(&b'\'') {
            index = quoted_end(bytes, index + 1, b'\'');
            continue;
        }
        if bytes[index] == b'"' {
            index = quoted_end(bytes, index, b'"');
            continue;
        }
        if bytes[index] == b'\'' && is_character_literal(bytes, index) {
            index = quoted_end(bytes, index, b'\'');
            continue;
        }
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
            let start = index;
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' && bytes[index] != b'\r' {
                index += 1;
            }
            comments.push(scanned_comment(source, start, index));
            continue;
        }
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            let start = index;
            index += 2;
            let mut depth = 1;
            while index < bytes.len() && depth > 0 {
                if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
                    depth += 1;
                    index += 2;
                } else if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                    depth -= 1;
                    index += 2;
                } else {
                    index += 1;
                }
            }
            comments.push(scanned_comment(source, start, index));
            continue;
        }
        index += 1;
    }
    comments
}

fn scanned_comment(source: &str, start: usize, end: usize) -> ScannedComment {
    let text = source[start..end].to_owned();
    let line_start = source[..start]
        .rfind(['\n', '\r'])
        .map_or(0, |newline| newline + 1);
    let own_line = source[line_start..start].chars().all(char::is_whitespace);
    let is_doc = (text.starts_with("///") && !text.starts_with("////"))
        || text.starts_with("//!")
        || text.starts_with("/*!")
        || (text.starts_with("/**") && !text.starts_with("/***"));
    ScannedComment {
        start,
        end,
        text,
        is_doc,
        own_line,
    }
}

fn raw_string_end(bytes: &[u8], index: usize) -> Option<usize> {
    if index > 0 && (bytes[index - 1].is_ascii_alphanumeric() || bytes[index - 1] == b'_') {
        return None;
    }
    let raw = if bytes.get(index) == Some(&b'r') {
        index
    } else if matches!(bytes.get(index), Some(b'b' | b'c')) && bytes.get(index + 1) == Some(&b'r') {
        index + 1
    } else {
        return None;
    };
    let mut quote = raw + 1;
    while bytes.get(quote) == Some(&b'#') {
        quote += 1;
    }
    if bytes.get(quote) != Some(&b'"') {
        return None;
    }
    let hashes = quote - raw - 1;
    let mut cursor = quote + 1;
    while cursor < bytes.len() {
        if bytes[cursor] == b'"'
            && (0..hashes).all(|offset| bytes.get(cursor + 1 + offset) == Some(&b'#'))
        {
            return Some(cursor + 1 + hashes);
        }
        cursor += 1;
    }
    Some(bytes.len())
}

fn quoted_end(bytes: &[u8], quote: usize, delimiter: u8) -> usize {
    let mut cursor = quote + 1;
    while cursor < bytes.len() {
        if bytes[cursor] == b'\\' {
            cursor = (cursor + 2).min(bytes.len());
        } else if bytes[cursor] == delimiter {
            return cursor + 1;
        } else {
            cursor += 1;
        }
    }
    bytes.len()
}

fn is_character_literal(bytes: &[u8], quote: usize) -> bool {
    match bytes.get(quote + 1) {
        None => false,
        Some(b'\\') => bytes.get(quote + 3) == Some(&b'\''),
        Some(_) => bytes.get(quote + 2) == Some(&b'\''),
    }
}

#[cfg(test)]
mod comment_tests {
    use super::*;

    #[test]
    fn scans_rust_comments_without_treating_literals_as_comments() {
        let source = r##"/// declaration docs
fn borrowed<'a>(value: &'a str) {
    let _ordinary = "// not a comment";
    let _raw = r#"/* not a comment */"#;
    let _slash = '/'; // trailing comment
    /* outer
       /* nested */
    */
}
"##;
        let comments = rust_comments(source);
        assert_eq!(comments.len(), 3);
        assert_eq!(comments[0].text, "/// declaration docs");
        assert!(comments[0].is_doc);
        assert!(comments[0].own_line);
        assert_eq!(comments[1].text, "// trailing comment");
        assert!(!comments[1].is_doc);
        assert!(!comments[1].own_line);
        assert!(comments[2].text.starts_with("/* outer"));
        assert!(comments[2].text.contains("/* nested */"));
        assert!(comments[2].own_line);
    }
}

#[derive(Default)]
struct NamespaceArenas {
    expressions: Vec<Expr>,
    places: Vec<Place>,
}

impl NamespaceArenas {
    fn expression(&mut self, loc: LocId, type_id: TypeId, kind: ExprKind) -> ExprId {
        let id = ExprId::new(self.expressions.len());
        self.expressions.push(Expr { kind, loc, type_id });
        id
    }

    fn place(&mut self, local_id: LocalId) -> PlaceId {
        let id = PlaceId::new(self.places.len());
        self.places.push(Place::LocalVar { local_id });
        id
    }
}

fn integer_method_primitive(name: &str) -> Option<PrimitiveOperation> {
    if !name.starts_with("core::num::<impl ") {
        return None;
    }
    if name.ends_with(">::wrapping_add") {
        Some(PrimitiveOperation::Add)
    } else if name.ends_with(">::overflowing_add") {
        Some(PrimitiveOperation::OverflowingAdd)
    } else if name.ends_with(">::wrapping_sub") {
        Some(PrimitiveOperation::Subtract)
    } else if name.ends_with(">::overflowing_sub") {
        Some(PrimitiveOperation::OverflowingSubtract)
    } else if name.ends_with(">::wrapping_mul") {
        Some(PrimitiveOperation::Multiply)
    } else if name.ends_with(">::overflowing_mul") {
        Some(PrimitiveOperation::OverflowingMultiply)
    } else if name.ends_with(">::wrapping_shl") {
        Some(PrimitiveOperation::ShiftLeft)
    } else if name.ends_with(">::wrapping_shr") {
        Some(PrimitiveOperation::ShiftRight)
    } else {
        None
    }
}

fn is_string_length(name: &str) -> bool {
    name == "core::str::<impl str>::len"
}

struct MappedFunction {
    result_type: TypeId,
    result_loc: LocId,
    locals: Vec<LocalDecl>,
    parameters: Vec<Parameter>,
    body: RawBody,
}

struct FunctionMapper<'a> {
    body: &'a Body,
    types: &'a TypeLayout,
    name_ids: &'a BTreeMap<String, NameId>,
    adts: &'a [AdtNames],
    sources: &'a mut SourceTables,
    arenas: &'a mut NamespaceArenas,
    rust_to_lir: Vec<LocalId>,
}

impl<'a> FunctionMapper<'a> {
    fn new(
        body: &'a Body,
        types: &'a TypeLayout,
        name_ids: &'a BTreeMap<String, NameId>,
        adts: &'a [AdtNames],
        sources: &'a mut SourceTables,
        arenas: &'a mut NamespaceArenas,
    ) -> Result<Self, String> {
        let argument_count = body.arg_locals().len();
        let mut rust_to_lir = vec![LocalId::new(0); body.locals().len()];
        rust_to_lir[0] = LocalId::new(argument_count);
        for (rust_local, lir_local) in rust_to_lir
            .iter_mut()
            .enumerate()
            .take(argument_count + 1)
            .skip(1)
        {
            *lir_local = LocalId::new(rust_local - 1);
        }
        for (rust_local, lir_local) in rust_to_lir.iter_mut().enumerate().skip(argument_count + 1) {
            *lir_local = LocalId::new(rust_local);
        }
        Ok(Self {
            body,
            types,
            name_ids,
            adts,
            sources,
            arenas,
            rust_to_lir,
        })
    }

    fn map(mut self) -> Result<MappedFunction, String> {
        let (locals, parameters) = self.map_locals()?;
        let mut blocks = Vec::with_capacity(self.body.blocks.len());
        for block in &self.body.blocks {
            let block_loc = self.sources.intern(block.terminator.source_info.span)?;
            let mut statements = Vec::new();
            for statement in &block.statements {
                let loc = self.sources.intern(statement.source_info.span)?;
                match &statement.kind {
                    StatementKind::Assign(destination, rvalue) => {
                        let value = self.map_rvalue(rvalue, loc)?;
                        let place = self.map_place(destination)?;
                        let expression =
                            self.arenas
                                .expression(loc, self.types.unit, ExprKind::Assign {
                                    place,
                                    value,
                                });
                        statements.push(RawStatement::Execute { expression });
                    },
                    StatementKind::StorageLive(local) => {
                        statements.push(RawStatement::StorageLive {
                            local_id: self.local_id(*local)?,
                        });
                    },
                    StatementKind::StorageDead(local) => {
                        statements.push(RawStatement::StorageDead {
                            local_id: self.local_id(*local)?,
                        });
                    },
                    StatementKind::SetDiscriminant {
                        place,
                        variant_index,
                    } => {
                        let place_type = place.ty(self.body.locals()).map_err(|error| {
                            format!("query set-discriminant place type: {error:?}")
                        })?;
                        let TyKind::RigidTy(RigidTy::Adt(definition, _)) = place_type.kind() else {
                            return Err(format!(
                                "cannot set the discriminant of MIR place type {place_type:?}"
                            ));
                        };
                        let adt = self
                            .adts
                            .iter()
                            .find(|entry| entry.definition == definition)
                            .ok_or_else(|| {
                                format!("enum `{}` was not interned", definition.name())
                            })?;
                        let variant = adt
                            .variants
                            .iter()
                            .find(|variant| variant.index == *variant_index)
                            .and_then(|variant| variant.name)
                            .ok_or_else(|| {
                                format!(
                                    "enum `{}` has no named variant {:?}",
                                    definition.name(),
                                    variant_index
                                )
                            })?;
                        statements.push(RawStatement::SetDiscriminant {
                            place: self.map_place(place)?,
                            variant,
                        });
                    },
                    StatementKind::PlaceMention(place) => {
                        statements.push(RawStatement::PlaceMention {
                            place: self.map_place(place)?,
                        });
                    },
                    StatementKind::FakeRead(_, place) => {
                        statements.push(RawStatement::PlaceMention {
                            place: self.map_place(place)?,
                        });
                    },
                    StatementKind::AscribeUserType { place, .. } => {
                        let type_id = self.types.id(place
                            .ty(self.body.locals())
                            .map_err(|error| format!("query ascribed place type: {error:?}"))?)?;
                        statements.push(RawStatement::AscribeUserType {
                            place: self.map_place(place)?,
                            r#type: TypeUse { loc, type_id },
                        });
                    },
                    StatementKind::Nop
                    | StatementKind::Coverage(_)
                    | StatementKind::ConstEvalCounter => {},
                    kind => return Err(format!("unsupported MIR statement: {kind:?}")),
                }
            }
            let terminator = self.map_terminator(&block.terminator.kind, block_loc)?;
            blocks.push(RawBasicBlock {
                loc: block_loc,
                statements,
                terminator,
            });
        }
        let result = &self.body.ret_local();
        Ok(MappedFunction {
            result_type: self.types.id(result.ty)?,
            result_loc: self.sources.intern(result.span)?,
            locals,
            parameters,
            body: RawBody::Cfg {
                graph: RawCfg {
                    blocks,
                    entry: BlockId::new(0),
                },
            },
        })
    }

    fn map_locals(&mut self) -> Result<(Vec<LocalDecl>, Vec<Parameter>), String> {
        let argument_count = self.body.arg_locals().len();
        let rust_order = (1..=argument_count)
            .chain(std::iter::once(0))
            .chain(argument_count + 1..self.body.locals().len());
        let mut locals = Vec::with_capacity(self.body.locals().len());
        for rust_local in rust_order {
            let declaration = &self.body.locals()[rust_local];
            let id = self.rust_to_lir[rust_local];
            let loc = self.sources.intern(declaration.span)?;
            let type_id = self.types.id(declaration.ty)?;
            locals.push(LocalDecl {
                id,
                loc,
                // MIR return and temporary locals are storage cells assigned by
                // the body even when their source binding was not `mut`.
                mutable: rust_local == 0
                    || rust_local > argument_count
                    || declaration.mutability == Mutability::Mut,
                name: self.local_name(rust_local),
                type_use: TypeUse { loc, type_id },
            });
        }
        let parameters = locals[..argument_count]
            .iter()
            .map(|local| Parameter {
                mutable: local.mutable,
                name: local.name.clone(),
                type_use: local.type_use.clone(),
            })
            .collect();
        Ok((locals, parameters))
    }

    fn local_name(&self, rust_local: usize) -> String {
        self.body
            .var_debug_info
            .iter()
            .find_map(|info| (info.local() == Some(rust_local)).then(|| info.name.clone()))
            .unwrap_or_else(|| format!("_{rust_local}"))
    }

    fn local_id(&self, rust_local: usize) -> Result<LocalId, String> {
        self.rust_to_lir
            .get(rust_local)
            .copied()
            .ok_or_else(|| format!("MIR local {rust_local} is out of bounds"))
    }

    fn map_rvalue(&mut self, rvalue: &Rvalue, loc: LocId) -> Result<ExprId, String> {
        match rvalue {
            Rvalue::Use(operand, WithRetag::No) => self.map_operand(operand, loc),
            // Retagging has no observable meaning in the admitted safe profile:
            // rustc admission is retained and `unsafe=reject`. An unsafe/provenance
            // profile must preserve this marker rather than use this normalization.
            Rvalue::Use(operand, WithRetag::Yes) => self.map_operand(operand, loc),
            Rvalue::CopyForDeref(place) => {
                let result_type = self.types.id(rvalue
                    .ty(self.body.locals())
                    .map_err(|error| format!("query copy-for-deref result type: {error:?}"))?)?;
                let place = self.map_place(place)?;
                Ok(self
                    .arenas
                    .expression(loc, result_type, ExprKind::Operation {
                        arguments: vec![],
                        instantiations: vec![],
                        operation: Operation::Read { place },
                        surface: None,
                    }))
            },
            Rvalue::BinaryOp(operation, left, right) => {
                let left_id = self.map_operand(left, loc)?;
                let right_id = self.map_operand(right, loc)?;
                let result_type = self.types.id(rvalue
                    .ty(self.body.locals())
                    .map_err(|error| format!("query binary operation result type: {error:?}"))?)?;
                let operation = primitive_operation(*operation)?;
                Ok(self
                    .arenas
                    .expression(loc, result_type, ExprKind::Operation {
                        arguments: vec![left_id, right_id],
                        instantiations: vec![],
                        operation: Operation::Primitive { kind: operation },
                        surface: None,
                    }))
            },
            Rvalue::CheckedBinaryOp(operation, left, right) => {
                let left_id = self.map_operand(left, loc)?;
                let right_id = self.map_operand(right, loc)?;
                let result_type =
                    self.types
                        .id(rvalue.ty(self.body.locals()).map_err(|error| {
                            format!("query checked binary operation result type: {error:?}")
                        })?)?;
                let operation = match operation {
                    BinOp::Add => PrimitiveOperation::OverflowingAdd,
                    BinOp::Sub => PrimitiveOperation::OverflowingSubtract,
                    BinOp::Mul => PrimitiveOperation::OverflowingMultiply,
                    kind => {
                        return Err(format!("unsupported checked MIR binary operation {kind:?}"))
                    },
                };
                Ok(self
                    .arenas
                    .expression(loc, result_type, ExprKind::Operation {
                        arguments: vec![left_id, right_id],
                        instantiations: vec![],
                        operation: Operation::Primitive { kind: operation },
                        surface: None,
                    }))
            },
            Rvalue::UnaryOp(operation, operand) => {
                let argument = self.map_operand(operand, loc)?;
                let result_type = self.types.id(rvalue
                    .ty(self.body.locals())
                    .map_err(|error| format!("query unary operation result type: {error:?}"))?)?;
                if *operation == UnOp::PtrMetadata {
                    let operand_type = operand.ty(self.body.locals()).map_err(|error| {
                        format!("query pointer-metadata operand type: {error:?}")
                    })?;
                    let TyKind::RigidTy(RigidTy::Ref(_, referent, _)) = operand_type.kind() else {
                        return Err(format!(
                            "pointer metadata is not yet supported for operand type {operand_type:?}"
                        ));
                    };
                    if !matches!(
                        referent.kind(),
                        TyKind::RigidTy(RigidTy::Slice(_) | RigidTy::Str)
                    ) {
                        return Err(format!(
                            "pointer metadata is not yet supported for referent type {referent:?}"
                        ));
                    }
                    let referent_type = self.types.id(referent)?;
                    let dereferenced =
                        self.arenas
                            .expression(loc, referent_type, ExprKind::Operation {
                                arguments: vec![argument],
                                instantiations: vec![],
                                operation: Operation::Reference {
                                    kind: ReferenceOperation::Dereference,
                                },
                                surface: None,
                            });
                    return Ok(self
                        .arenas
                        .expression(loc, result_type, ExprKind::Operation {
                            arguments: vec![dereferenced],
                            instantiations: vec![],
                            operation: Operation::Primitive {
                                kind: PrimitiveOperation::Length,
                            },
                            surface: None,
                        }));
                }
                let kind = match (operation, operand.ty(self.body.locals())) {
                    (UnOp::Not, Ok(ty)) if is_bool(ty) => PrimitiveOperation::LogicalNot,
                    (UnOp::Not, Ok(ty)) if is_integer(ty) => PrimitiveOperation::BitwiseNot,
                    (UnOp::Neg, Ok(ty)) if is_signed_integer(ty) => PrimitiveOperation::Negate,
                    (_, result) => {
                        return Err(format!(
                            "unsupported scalar unary operation {operation:?} on {result:?}"
                        ))
                    },
                };
                Ok(self
                    .arenas
                    .expression(loc, result_type, ExprKind::Operation {
                        arguments: vec![argument],
                        instantiations: vec![],
                        operation: Operation::Primitive { kind },
                        surface: None,
                    }))
            },
            Rvalue::Cast(
                CastKind::PointerCoercion(PointerCoercion::ReifyFnPointer(Safety::Safe)),
                operand,
                target,
            ) => {
                let source = operand
                    .ty(self.body.locals())
                    .map_err(|error| format!("query function-item cast source type: {error:?}"))?;
                let TyKind::RigidTy(RigidTy::FnDef(definition, generic_arguments)) = source.kind()
                else {
                    return Err(format!(
                        "safe function-pointer reification has non-function source: {:?}",
                        source.kind()
                    ));
                };
                if !generic_arguments.0.is_empty() {
                    return Err(
                        "generic function-pointer reification requires explicit instantiations"
                            .to_owned(),
                    );
                }
                let callee_name = definition.name();
                let name = self.name_ids.get(&callee_name).copied().ok_or_else(|| {
                    format!(
                        "function-pointer target `{callee_name}` is not a mapped local function"
                    )
                })?;
                let result_type = self.types.id(*target)?;
                Ok(self
                    .arenas
                    .expression(loc, result_type, ExprKind::Operation {
                        arguments: vec![],
                        instantiations: vec![],
                        operation: Operation::Call {
                            kind: CallKind::Closure {
                                function: QualifiedRef {
                                    name,
                                    namespace_id: NamespaceId::new(0),
                                },
                            },
                        },
                        surface: None,
                    }))
            },
            Rvalue::Cast(CastKind::IntToInt, operand, target) => {
                let source = operand
                    .ty(self.body.locals())
                    .map_err(|error| format!("query integer cast source type: {error:?}"))?;
                let integer_cast = is_integer(source) && is_integer(*target);
                let character_to_integer = is_character(source) && is_integer(*target);
                let ascii_to_character = is_u8(source) && is_character(*target);
                if !integer_cast && !character_to_integer && !ascii_to_character {
                    return Err(format!(
                        "unsupported integer/character MIR cast: {source:?} to {target:?}"
                    ));
                }
                let argument = self.map_operand(operand, loc)?;
                let result_type = self.types.id(*target)?;
                Ok(self
                    .arenas
                    .expression(loc, result_type, ExprKind::Operation {
                        arguments: vec![argument],
                        instantiations: vec![],
                        operation: Operation::Primitive {
                            kind: PrimitiveOperation::Cast,
                        },
                        surface: None,
                    }))
            },
            Rvalue::Ref(_, borrow_kind, place) => {
                let kind = match borrow_kind {
                    MirBorrowKind::Shared => BorrowKind::Immutable,
                    MirBorrowKind::Mut { .. } => BorrowKind::Mutable,
                    MirBorrowKind::Fake(kind) => {
                        return Err(format!("unsupported fake MIR borrow: {kind:?}"))
                    },
                };
                let place = self.map_place(place)?;
                let result_type = self.types.id(rvalue
                    .ty(self.body.locals())
                    .map_err(|error| format!("query borrow result type: {error:?}"))?)?;
                Ok(self
                    .arenas
                    .expression(loc, result_type, ExprKind::Operation {
                        arguments: vec![],
                        instantiations: vec![],
                        operation: Operation::Borrow { kind, place },
                        surface: None,
                    }))
            },
            Rvalue::Discriminant(place) => {
                let place_type = place
                    .ty(self.body.locals())
                    .map_err(|error| format!("query discriminant place type: {error:?}"))?;
                let TyKind::RigidTy(RigidTy::Adt(definition, _)) = place_type.kind() else {
                    return Err(format!(
                        "discriminant place does not have an enum type: {:?}",
                        place_type.kind()
                    ));
                };
                let adt = self
                    .adts
                    .iter()
                    .find(|entry| entry.definition == definition)
                    .ok_or_else(|| format!("enum `{}` was not interned", definition.name()))?;
                let place = self.map_place(place)?;
                let operand_type = self.types.id(place_type)?;
                let operand = self
                    .arenas
                    .expression(loc, operand_type, ExprKind::Operation {
                        arguments: vec![],
                        instantiations: vec![],
                        operation: Operation::Read { place },
                        surface: None,
                    });
                let result_type = self.types.id(rvalue
                    .ty(self.body.locals())
                    .map_err(|error| format!("query discriminant result type: {error:?}"))?)?;
                Ok(self
                    .arenas
                    .expression(loc, result_type, ExprKind::Operation {
                        arguments: vec![operand],
                        instantiations: vec![],
                        operation: Operation::Data {
                            kind: DataOperation::Discriminant {
                                type_: QualifiedRef {
                                    name: adt.name,
                                    namespace_id: NamespaceId::new(0),
                                },
                            },
                        },
                        surface: None,
                    }))
            },
            Rvalue::Len(place) => {
                let place_type = place
                    .ty(self.body.locals())
                    .map_err(|error| format!("query length place type: {error:?}"))?;
                let result_type = self.types.id(rvalue
                    .ty(self.body.locals())
                    .map_err(|error| format!("query length result type: {error:?}"))?)?;
                match place_type.kind() {
                    TyKind::RigidTy(RigidTy::Array(_, length)) => {
                        let length = length
                            .eval_target_usize()
                            .map_err(|error| format!("evaluate fixed array length: {error:?}"))?;
                        Ok(self.arenas.expression(loc, result_type, ExprKind::Value {
                            source_constant: None,
                            value: ConstValue::Integer {
                                value: Number::from(length),
                            },
                        }))
                    },
                    TyKind::RigidTy(RigidTy::Slice(_)) => {
                        let place_type = self.types.id(place_type)?;
                        let place = self.map_place(place)?;
                        let slice = self
                            .arenas
                            .expression(loc, place_type, ExprKind::Operation {
                                arguments: vec![],
                                instantiations: vec![],
                                operation: Operation::Read { place },
                                surface: None,
                            });
                        Ok(self
                            .arenas
                            .expression(loc, result_type, ExprKind::Operation {
                                arguments: vec![slice],
                                instantiations: vec![],
                                operation: Operation::Primitive {
                                    kind: PrimitiveOperation::Length,
                                },
                                surface: None,
                            }))
                    },
                    kind => Err(format!("cannot take MIR length of place type {kind:?}")),
                }
            },
            Rvalue::Aggregate(AggregateKind::Tuple, operands) => {
                let arguments = operands
                    .iter()
                    .map(|operand| self.map_operand(operand, loc))
                    .collect::<Result<Vec<_>, _>>()?;
                let result_type = self.types.id(rvalue
                    .ty(self.body.locals())
                    .map_err(|error| format!("query tuple result type: {error:?}"))?)?;
                Ok(self
                    .arenas
                    .expression(loc, result_type, ExprKind::Operation {
                        arguments,
                        instantiations: vec![],
                        operation: Operation::Primitive {
                            kind: PrimitiveOperation::Tuple,
                        },
                        surface: None,
                    }))
            },
            Rvalue::Aggregate(AggregateKind::Array(_), operands) => {
                let arguments = operands
                    .iter()
                    .map(|operand| self.map_operand(operand, loc))
                    .collect::<Result<Vec<_>, _>>()?;
                let result_type = self.types.id(rvalue
                    .ty(self.body.locals())
                    .map_err(|error| format!("query array result type: {error:?}"))?)?;
                Ok(self
                    .arenas
                    .expression(loc, result_type, ExprKind::Operation {
                        arguments,
                        instantiations: vec![],
                        operation: Operation::Primitive {
                            kind: PrimitiveOperation::Vector,
                        },
                        surface: None,
                    }))
            },
            Rvalue::Repeat(operand, _) => {
                let argument = self.map_operand(operand, loc)?;
                let result_type = self.types.id(rvalue
                    .ty(self.body.locals())
                    .map_err(|error| format!("query repeated array result type: {error:?}"))?)?;
                Ok(self
                    .arenas
                    .expression(loc, result_type, ExprKind::Operation {
                        arguments: vec![argument],
                        instantiations: vec![],
                        operation: Operation::Primitive {
                            kind: PrimitiveOperation::RepeatVector,
                        },
                        surface: None,
                    }))
            },
            Rvalue::Aggregate(kind, operands) => {
                let AggregateKind::Adt(definition, variant_index, arguments, _, active_field) =
                    kind
                else {
                    return Err(format!("unsupported MIR aggregate: {kind:?}"));
                };
                if active_field.is_some() {
                    return Err(format!(
                        "union aggregate `{}` is not supported",
                        definition.name()
                    ));
                }
                let adt = self
                    .adts
                    .iter()
                    .find(|entry| entry.definition == *definition)
                    .ok_or_else(|| format!("ADT `{}` was not interned", definition.name()))?;
                let variant = adt
                    .variants
                    .iter()
                    .find(|variant| variant.index == *variant_index)
                    .ok_or_else(|| {
                        format!(
                            "ADT `{}` has no variant {:?}",
                            definition.name(),
                            variant_index
                        )
                    })?;
                let instantiations = self.types.generic_arguments(arguments, loc)?;
                let arguments = operands
                    .iter()
                    .map(|operand| self.map_operand(operand, loc))
                    .collect::<Result<Vec<_>, _>>()?;
                let result_type = self.types.id(rvalue
                    .ty(self.body.locals())
                    .map_err(|error| format!("query aggregate result type: {error:?}"))?)?;
                Ok(self
                    .arenas
                    .expression(loc, result_type, ExprKind::Operation {
                        arguments,
                        instantiations,
                        operation: Operation::Call {
                            kind: CallKind::Constructor {
                                constructor: QualifiedRef {
                                    name: adt.name,
                                    namespace_id: NamespaceId::new(0),
                                },
                                variant: variant.source_name.clone(),
                            },
                        },
                        surface: None,
                    }))
            },
            kind => Err(format!("unsupported MIR rvalue: {kind:?}")),
        }
    }

    fn map_operand(&mut self, operand: &Operand, loc: LocId) -> Result<ExprId, String> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                let rust_type = place
                    .ty(self.body.locals())
                    .map_err(|error| format!("query operand place type: {error:?}"))?;
                let type_id = self.types.id(rust_type)?;
                let place_id = self.map_place(place)?;
                let operation = match operand {
                    // Optimized generic MIR may spell a final, consuming use
                    // of an unconstrained type parameter as `Copy` after the
                    // source move and return-place forwarding have been
                    // contracted. The shared LIR requires actual Copy
                    // evidence, so retain the source-valid consuming meaning.
                    // A body that uses the same parameter again still fails
                    // definite-initialization checking until rustc Public can
                    // expose its declared Copy predicate.
                    Operand::Copy(_) if matches!(rust_type.kind(), TyKind::Param(_)) => {
                        Operation::Move { place: place_id }
                    },
                    // `&mut T` is never Copy in Rust. Optimized MIR can still
                    // use the `Copy` operand spelling for its final return-
                    // forwarding use; rustc admission proves there is no
                    // duplicating source use, so retain the consuming move.
                    Operand::Copy(_)
                        if matches!(
                            rust_type.kind(),
                            TyKind::RigidTy(RigidTy::Ref(_, _, Mutability::Mut))
                        ) =>
                    {
                        Operation::Move { place: place_id }
                    },
                    Operand::Copy(_) => Operation::Copy { place: place_id },
                    Operand::Move(_) => Operation::Move { place: place_id },
                    _ => unreachable!(),
                };
                Ok(self.arenas.expression(loc, type_id, ExprKind::Operation {
                    arguments: vec![],
                    instantiations: vec![],
                    operation,
                    surface: None,
                }))
            },
            Operand::Constant(constant) => self.map_constant(constant, loc),
            Operand::RuntimeChecks(check) => {
                Err(format!("unsupported runtime-check operand: {check:?}"))
            },
        }
    }

    fn map_constant(&mut self, constant: &ConstOperand, loc: LocId) -> Result<ExprId, String> {
        let type_id = self.types.id(constant.ty())?;
        let value = match constant.ty().kind() {
            TyKind::RigidTy(RigidTy::Bool) => ConstValue::Bool {
                value: read_bool_constant(constant)?,
            },
            TyKind::RigidTy(RigidTy::Char) => ConstValue::Character {
                value: read_character_constant(constant)?,
            },
            TyKind::RigidTy(RigidTy::Uint(_)) | TyKind::RigidTy(RigidTy::Int(_)) => {
                ConstValue::Integer {
                    value: read_integer_constant(constant)?,
                }
            },
            TyKind::RigidTy(RigidTy::Tuple(elements)) if elements.is_empty() => ConstValue::Unit,
            kind => return Err(format!("unsupported MIR constant type: {kind:?}")),
        };
        Ok(self.arenas.expression(loc, type_id, ExprKind::Value {
            source_constant: None,
            value,
        }))
    }

    fn map_place(&mut self, place: &rustc_public::mir::Place) -> Result<PlaceId, String> {
        let local_id = self
            .rust_to_lir
            .get(place.local)
            .copied()
            .ok_or_else(|| format!("MIR local {} is out of bounds", place.local))?;
        let mut current = self.arenas.place(local_id);
        let mut current_type = self.body.locals()[place.local].ty;
        let mut selected_variant = None;
        for projection in &place.projection {
            current = match projection {
                ProjectionElem::Deref => {
                    let id = PlaceId::new(self.arenas.places.len());
                    self.arenas.places.push(Place::Deref { base: current });
                    let TyKind::RigidTy(RigidTy::Ref(_, referent, _)) = current_type.kind() else {
                        return Err(format!(
                            "cannot dereference MIR place type {current_type:?}"
                        ));
                    };
                    current_type = referent;
                    selected_variant = None;
                    id
                },
                ProjectionElem::Downcast(index) => {
                    let TyKind::RigidTy(RigidTy::Adt(definition, _)) = current_type.kind() else {
                        return Err(format!("cannot downcast MIR place type {current_type:?}"));
                    };
                    let adt = self
                        .adts
                        .iter()
                        .find(|entry| entry.definition == definition)
                        .ok_or_else(|| format!("enum `{}` was not interned", definition.name()))?;
                    let variant = adt
                        .variants
                        .iter()
                        .find(|variant| variant.index == *index)
                        .ok_or_else(|| {
                            format!("enum `{}` has no variant {:?}", definition.name(), index)
                        })?;
                    let id = PlaceId::new(self.arenas.places.len());
                    self.arenas.places.push(Place::Downcast {
                        base: current,
                        variant: variant.name.ok_or_else(|| {
                            format!("struct `{}` cannot be downcast", definition.name())
                        })?,
                    });
                    selected_variant = Some(*index);
                    id
                },
                ProjectionElem::Field(index, field_type) => {
                    if let TyKind::RigidTy(RigidTy::Tuple(elements)) = current_type.kind() {
                        let expected = elements.get(*index).ok_or_else(|| {
                            format!("tuple place has no element at index {index}")
                        })?;
                        if *expected != *field_type {
                            return Err(format!(
                                "tuple field {index} has MIR type {field_type:?}, expected {expected:?}"
                            ));
                        }
                        let loc = self.sources.intern(self.body.locals()[place.local].span)?;
                        let index_type =
                            self.types.integer_id(IntegerType::Unsigned(UintTy::U32))?;
                        let index_expression =
                            self.arenas.expression(loc, index_type, ExprKind::Value {
                                source_constant: None,
                                value: ConstValue::Integer {
                                    value: Number::from_u128(*index as u128).ok_or_else(|| {
                                        format!("tuple field index {index} is out of range")
                                    })?,
                                },
                            });
                        let id = PlaceId::new(self.arenas.places.len());
                        self.arenas.places.push(Place::Index {
                            base: current,
                            index: index_expression,
                        });
                        current = id;
                        current_type = *field_type;
                        selected_variant = None;
                        continue;
                    }
                    let TyKind::RigidTy(RigidTy::Adt(definition, _)) = current_type.kind() else {
                        return Err(format!(
                            "cannot select a field of MIR place type {current_type:?}"
                        ));
                    };
                    let adt = self
                        .adts
                        .iter()
                        .find(|entry| entry.definition == definition)
                        .ok_or_else(|| format!("enum `{}` was not interned", definition.name()))?;
                    let variant_index = selected_variant.unwrap_or_else(|| {
                        adt.variants
                            .first()
                            .map(|variant| variant.index)
                            .expect("Rust ADTs have at least one variant")
                    });
                    let variant = adt
                        .variants
                        .iter()
                        .find(|variant| variant.index == variant_index)
                        .expect("selected variant came from this ADT");
                    let field = variant.fields.get(*index).copied().ok_or_else(|| {
                        format!(
                            "variant {:?} of `{}` has no field {index}",
                            variant_index,
                            definition.name()
                        )
                    })?;
                    let id = PlaceId::new(self.arenas.places.len());
                    // The projection names the ADT it selects from, so
                    // resolving it is a static decision rather than a
                    // question about the value at the base.
                    self.arenas.places.push(Place::Field {
                        base: current,
                        owner: QualifiedRef {
                            name: adt.name,
                            namespace_id: NamespaceId::new(0),
                        },
                        field,
                    });
                    current_type = *field_type;
                    selected_variant = None;
                    id
                },
                ProjectionElem::Index(index_local) => {
                    let element = match current_type.kind() {
                        TyKind::RigidTy(RigidTy::Array(element, _) | RigidTy::Slice(element)) => {
                            element
                        },
                        _ => {
                            return Err(format!(
                            "dynamic MIR index is not yet supported for place type {current_type:?}"
                        ))
                        },
                    };
                    let index_local_id = self.local_id(*index_local)?;
                    let index_place = self.arenas.place(index_local_id);
                    let index_loc = self.sources.intern(self.body.locals()[*index_local].span)?;
                    let index_type = self.types.id(self.body.locals()[*index_local].ty)?;
                    let index_expression =
                        self.arenas
                            .expression(index_loc, index_type, ExprKind::Operation {
                                arguments: vec![],
                                instantiations: vec![],
                                operation: Operation::Copy { place: index_place },
                                surface: None,
                            });
                    let id = PlaceId::new(self.arenas.places.len());
                    self.arenas.places.push(Place::Index {
                        base: current,
                        index: index_expression,
                    });
                    current_type = element;
                    selected_variant = None;
                    id
                },
                ProjectionElem::ConstantIndex {
                    offset,
                    min_length,
                    from_end,
                } => {
                    let loc = self.sources.intern(self.body.locals()[place.local].span)?;
                    let index_type = self
                        .types
                        .integer_id(IntegerType::Unsigned(UintTy::Usize))?;
                    let (element, index_expression) = match current_type.kind() {
                        TyKind::RigidTy(RigidTy::Array(element, length)) => {
                            let length = length.eval_target_usize().map_err(|error| {
                                format!("evaluate indexed array length: {error:?}")
                            })?;
                            if length < *min_length || (*from_end && length < *offset) {
                                return Err(format!(
                                    "invalid MIR constant index offset={offset} min_length={min_length} from_end={from_end} for array length {length}"
                                ));
                            }
                            let index = if *from_end { length - *offset } else { *offset };
                            let expression = self.arenas.expression(
                                loc,
                                index_type,
                                ExprKind::Value {
                                    source_constant: None,
                                    value: ConstValue::Integer {
                                        value: Number::from(index),
                                    },
                                },
                            );
                            (element, expression)
                        },
                        TyKind::RigidTy(RigidTy::Slice(element)) if !*from_end => {
                            let expression = self.arenas.expression(
                                loc,
                                index_type,
                                ExprKind::Value {
                                    source_constant: None,
                                    value: ConstValue::Integer {
                                        value: Number::from(*offset),
                                    },
                                },
                            );
                            (element, expression)
                        },
                        TyKind::RigidTy(RigidTy::Slice(element)) => {
                            if offset > min_length {
                                return Err(format!(
                                    "invalid from-end slice index offset={offset} min_length={min_length}"
                                ));
                            }
                            let slice_type = self.types.id(current_type)?;
                            let slice = self.arenas.expression(
                                loc,
                                slice_type,
                                ExprKind::Operation {
                                    arguments: vec![],
                                    instantiations: vec![],
                                    operation: Operation::Read { place: current },
                                    surface: None,
                                },
                            );
                            let length = self.arenas.expression(
                                loc,
                                index_type,
                                ExprKind::Operation {
                                    arguments: vec![slice],
                                    instantiations: vec![],
                                    operation: Operation::Primitive {
                                        kind: PrimitiveOperation::Length,
                                    },
                                    surface: None,
                                },
                            );
                            let offset = self.arenas.expression(
                                loc,
                                index_type,
                                ExprKind::Value {
                                    source_constant: None,
                                    value: ConstValue::Integer {
                                        value: Number::from(*offset),
                                    },
                                },
                            );
                            let expression = self.arenas.expression(
                                loc,
                                index_type,
                                ExprKind::Operation {
                                    arguments: vec![length, offset],
                                    instantiations: vec![],
                                    operation: Operation::Primitive {
                                        kind: PrimitiveOperation::Subtract,
                                    },
                                    surface: None,
                                },
                            );
                            (element, expression)
                        },
                        _ => return Err(format!(
                            "constant MIR index is not yet supported for place type {current_type:?} with from_end={from_end}"
                        )),
                    };
                    let id = PlaceId::new(self.arenas.places.len());
                    self.arenas.places.push(Place::Index {
                        base: current,
                        index: index_expression,
                    });
                    current_type = element;
                    selected_variant = None;
                    id
                },
                ProjectionElem::Subslice { from, to, from_end } => {
                    if !matches!(
                        current_type.kind(),
                        TyKind::RigidTy(RigidTy::Array(_, _) | RigidTy::Slice(_))
                    ) {
                        return Err(format!("cannot subslice MIR place type {current_type:?}"));
                    }
                    let result_type = projection
                        .ty(current_type)
                        .map_err(|error| format!("query MIR subslice result type: {error:?}"))?;
                    let id = PlaceId::new(self.arenas.places.len());
                    self.arenas.places.push(Place::Subslice {
                        base: current,
                        start: usize::try_from(*from)
                            .map_err(|_| format!("subslice start {from} is out of range"))?,
                        stop: usize::try_from(*to)
                            .map_err(|_| format!("subslice end {to} is out of range"))?,
                        from_end: *from_end,
                    });
                    current_type = result_type;
                    selected_variant = None;
                    id
                },
                kind => return Err(format!("unsupported MIR place projection: {kind:?}")),
            };
        }
        Ok(current)
    }

    fn map_terminator(
        &mut self,
        terminator: &TerminatorKind,
        loc: LocId,
    ) -> Result<RawTerminator, String> {
        match terminator {
            TerminatorKind::Goto { target } => Ok(RawTerminator::Goto {
                target: BlockId::new(*target),
            }),
            TerminatorKind::SwitchInt { discr, targets } => {
                let scrutinee = self.map_operand(discr, loc)?;
                let discr_type = discr
                    .ty(self.body.locals())
                    .map_err(|error| format!("query SwitchInt type: {error:?}"))?;
                if is_bool(discr_type) {
                    let branches = targets.branches().collect::<Vec<_>>();
                    let [(value, target)] = branches.as_slice() else {
                        return Err(format!(
                            "Bool SwitchInt requires one explicit target, found {branches:?}"
                        ));
                    };
                    let (then_target, else_target) = match value {
                        0 => (targets.otherwise(), *target),
                        1 => (*target, targets.otherwise()),
                        _ => return Err(format!("invalid Bool SwitchInt value {value}")),
                    };
                    Ok(RawTerminator::Branch {
                        condition: scrutinee,
                        else_target: BlockId::new(else_target),
                        then_target: BlockId::new(then_target),
                    })
                } else if is_integer(discr_type) || is_character(discr_type) {
                    let cases = targets
                        .branches()
                        .map(|(value, target)| -> Result<_, String> {
                            let value = if is_character(discr_type) {
                                ConstValue::Character {
                                    value: character_value(value)?,
                                }
                            } else {
                                ConstValue::Integer {
                                    value: integer_value(value, discr_type)?,
                                }
                            };
                            Ok((value, BlockId::new(target)))
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(RawTerminator::Switch {
                        cases,
                        default_target: BlockId::new(targets.otherwise()),
                        scrutinee,
                    })
                } else {
                    Err(format!("unsupported SwitchInt type: {discr_type:?}"))
                }
            },
            TerminatorKind::Call {
                func,
                args,
                destination,
                target,
                unwind,
            } => {
                let callee_type = func
                    .ty(self.body.locals())
                    .map_err(|error| format!("query callee type: {error:?}"))?;
                if let TyKind::RigidTy(RigidTy::FnDef(definition, generic_arguments)) =
                    callee_type.kind()
                {
                    let callee_name = definition.name();
                    if matches!(
                        callee_name.as_str(),
                        "core::intrinsics::abort"
                            | "std::intrinsics::abort"
                            | "std::process::abort"
                    ) {
                        if !generic_arguments.0.is_empty() || !args.is_empty() || target.is_some() {
                            return Err(format!(
                                "unexpected MIR shape for abort intrinsic `{callee_name}`"
                            ));
                        }
                        return Ok(RawTerminator::Abort);
                    }
                }
                let mut arguments = args
                    .iter()
                    .map(|argument| self.map_operand(argument, loc))
                    .collect::<Result<Vec<_>, _>>()?;
                let mut instantiations = vec![];
                let operation = match callee_type.kind() {
                    TyKind::RigidTy(RigidTy::FnDef(definition, generic_arguments)) => {
                        let callee_name = definition.name();
                        if is_string_length(&callee_name) && generic_arguments.0.is_empty() {
                            let [argument] = args.as_slice() else {
                                return Err(format!(
                                    "unexpected MIR argument count for `{callee_name}`"
                                ));
                            };
                            let argument_type =
                                argument.ty(self.body.locals()).map_err(|error| {
                                    format!("query `{callee_name}` receiver type: {error:?}")
                                })?;
                            let TyKind::RigidTy(RigidTy::Ref(_, referent, _)) =
                                argument_type.kind()
                            else {
                                return Err(format!(
                                    "`{callee_name}` has non-reference receiver {argument_type:?}"
                                ));
                            };
                            if !matches!(referent.kind(), TyKind::RigidTy(RigidTy::Str)) {
                                return Err(format!(
                                    "`{callee_name}` has non-string receiver {argument_type:?}"
                                ));
                            }
                            let dereferenced = self.arenas.expression(
                                loc,
                                self.types.id(referent)?,
                                ExprKind::Operation {
                                    arguments: vec![arguments[0]],
                                    instantiations: vec![],
                                    operation: Operation::Reference {
                                        kind: ReferenceOperation::Dereference,
                                    },
                                    surface: None,
                                },
                            );
                            arguments[0] = dereferenced;
                            Operation::Primitive {
                                kind: PrimitiveOperation::Length,
                            }
                        } else if let Some(kind) = integer_method_primitive(&callee_name)
                            && generic_arguments.0.is_empty()
                        {
                            Operation::Primitive { kind }
                        } else {
                            let name = self.name_ids.get(&callee_name).copied().ok_or_else(|| {
                                if generic_arguments.0.is_empty() {
                                    format!(
                                        "direct callee `{callee_name}` is not a mapped local function"
                                    )
                                } else {
                                    format!(
                                        "generic or associated callee `{callee_name}` is not a mapped local function; function predicates and implementation-selection evidence are unavailable from the pinned Rustc Public API"
                                    )
                                }
                            })?;
                            instantiations =
                                self.types.generic_arguments(&generic_arguments, loc)?;
                            Operation::Call {
                                kind: CallKind::Function {
                                    callee: QualifiedRef {
                                        name,
                                        namespace_id: NamespaceId::new(0),
                                    },
                                },
                            }
                        }
                    },
                    TyKind::RigidTy(RigidTy::FnPtr(signature)) => {
                        supported_function_pointer_signature(&signature)?;
                        let callable = self.map_operand(func, loc)?;
                        arguments.insert(0, callable);
                        Operation::Call {
                            kind: CallKind::Invoke,
                        }
                    },
                    kind => return Err(format!("unsupported callee type: {kind:?}")),
                };
                let result_type = self.types.id(destination
                    .ty(self.body.locals())
                    .map_err(|error| format!("query call destination type: {error:?}"))?)?;
                let call = self
                    .arenas
                    .expression(loc, result_type, ExprKind::Operation {
                        arguments,
                        instantiations,
                        operation,
                        surface: None,
                    });
                let destination = match target {
                    Some(target) => Some(RawCallDestination {
                        place: self.map_place(destination)?,
                        target: BlockId::new(*target),
                    }),
                    None => None,
                };
                Ok(RawTerminator::Call {
                    call,
                    destination,
                    unwind: map_unwind(unwind),
                })
            },
            TerminatorKind::Drop {
                place,
                target,
                unwind,
            } => Ok(RawTerminator::Drop {
                place: self.map_place(place)?,
                target: BlockId::new(*target),
                unwind: map_unwind(unwind),
            }),
            TerminatorKind::Assert {
                cond,
                expected,
                msg,
                target,
                unwind,
            } => Ok(RawTerminator::Assert {
                condition: self.map_operand(cond, loc)?,
                expected: *expected,
                kind: map_assert_kind(msg)?,
                target: BlockId::new(*target),
                unwind: map_unwind(unwind),
            }),
            TerminatorKind::Return => {
                let return_place = rustc_public::mir::Place {
                    local: 0,
                    projection: vec![],
                };
                let return_type = return_place
                    .ty(self.body.locals())
                    .map_err(|error| format!("query return place type: {error:?}"))?;
                // Optimized MIR never assigns the unit return place of a
                // `()` function. Its value is nevertheless determined, so
                // export the unit value itself rather than a return-place
                // move that definite-initialization checking must reject.
                let result = match return_type.kind() {
                    TyKind::RigidTy(RigidTy::Tuple(elements)) if elements.is_empty() => self
                        .arenas
                        .expression(loc, self.types.unit, ExprKind::Value {
                            source_constant: None,
                            value: ConstValue::Unit,
                        }),
                    _ => self.map_operand(&Operand::Move(return_place), loc)?,
                };
                Ok(RawTerminator::Return {
                    values: vec![result],
                })
            },
            TerminatorKind::Unreachable => Ok(RawTerminator::Unreachable),
            TerminatorKind::Resume => Ok(RawTerminator::Resume),
            TerminatorKind::Abort => Ok(RawTerminator::Abort),
            kind => Err(format!("unsupported MIR terminator: {kind:?}")),
        }
    }
}

fn map_assert_kind(message: &AssertMessage) -> Result<RawAssertKind, String> {
    match message {
        AssertMessage::BoundsCheck { .. } => Ok(RawAssertKind::BoundsCheck),
        AssertMessage::Overflow(..) | AssertMessage::OverflowNeg(_) => Ok(RawAssertKind::Overflow),
        AssertMessage::DivisionByZero(_) => Ok(RawAssertKind::DivisionByZero),
        AssertMessage::RemainderByZero(_) => Ok(RawAssertKind::RemainderByZero),
        AssertMessage::MisalignedPointerDereference { .. } => {
            Ok(RawAssertKind::MisalignedPointerDereference)
        },
        kind => Err(format!("unsupported MIR assertion kind: {kind:?}")),
    }
}

fn primitive_operation(operation: BinOp) -> Result<PrimitiveOperation, String> {
    match operation {
        BinOp::Add | BinOp::AddUnchecked => Ok(PrimitiveOperation::Add),
        BinOp::Sub | BinOp::SubUnchecked => Ok(PrimitiveOperation::Subtract),
        BinOp::Mul | BinOp::MulUnchecked => Ok(PrimitiveOperation::Multiply),
        BinOp::Div => Ok(PrimitiveOperation::Divide),
        BinOp::Rem => Ok(PrimitiveOperation::Modulo),
        BinOp::BitXor => Ok(PrimitiveOperation::BitwiseXor),
        BinOp::BitAnd => Ok(PrimitiveOperation::BitwiseAnd),
        BinOp::BitOr => Ok(PrimitiveOperation::BitwiseOr),
        BinOp::Shl | BinOp::ShlUnchecked => Ok(PrimitiveOperation::ShiftLeft),
        BinOp::Shr | BinOp::ShrUnchecked => Ok(PrimitiveOperation::ShiftRight),
        BinOp::Eq => Ok(PrimitiveOperation::Equal),
        BinOp::Lt => Ok(PrimitiveOperation::Less),
        BinOp::Le => Ok(PrimitiveOperation::LessEqual),
        BinOp::Ne => Ok(PrimitiveOperation::NotEqual),
        BinOp::Ge => Ok(PrimitiveOperation::GreaterEqual),
        BinOp::Gt => Ok(PrimitiveOperation::Greater),
        BinOp::Cmp | BinOp::Offset => {
            Err(format!("unsupported scalar binary operation {operation:?}"))
        },
    }
}

fn map_unwind(unwind: &UnwindAction) -> RawUnwindAction {
    match unwind {
        UnwindAction::Continue => RawUnwindAction::Continue,
        UnwindAction::Unreachable => RawUnwindAction::Unreachable,
        UnwindAction::Terminate => RawUnwindAction::Terminate {
            reason: "rustc terminate".to_owned(),
        },
        UnwindAction::Cleanup(target) => RawUnwindAction::Cleanup {
            target: BlockId::new(*target),
        },
    }
}

fn is_bool(ty: rustc_public::ty::Ty) -> bool {
    ty.kind() == TyKind::RigidTy(RigidTy::Bool)
}

fn is_character(ty: rustc_public::ty::Ty) -> bool {
    ty.kind() == TyKind::RigidTy(RigidTy::Char)
}

fn is_integer(ty: rustc_public::ty::Ty) -> bool {
    matches!(
        ty.kind(),
        TyKind::RigidTy(RigidTy::Uint(_) | RigidTy::Int(_))
    )
}

fn is_u8(ty: rustc_public::ty::Ty) -> bool {
    ty.kind() == TyKind::RigidTy(RigidTy::Uint(UintTy::U8))
}

fn is_signed_integer(ty: rustc_public::ty::Ty) -> bool {
    matches!(ty.kind(), TyKind::RigidTy(RigidTy::Int(_)))
}

fn integer_number(value: i128) -> Result<Number, String> {
    Number::from_i128(value)
        .ok_or_else(|| format!("cannot represent signed Rust integer value {value} as JSON"))
}

fn integer_value(value: u128, ty: RustTy) -> Result<Number, String> {
    match ty.kind() {
        TyKind::RigidTy(RigidTy::Uint(_)) => Number::from_u128(value)
            .ok_or_else(|| format!("cannot represent unsigned Rust integer value {value} as JSON")),
        TyKind::RigidTy(RigidTy::Int(kind)) => {
            let bits = kind.num_bytes() * 8;
            if bits == 128 {
                return integer_number(value as i128);
            }
            let sign = 1_u128 << (bits - 1);
            let modulus = 1_u128 << bits;
            let truncated = value & (modulus - 1);
            if truncated & sign == 0 {
                integer_number(truncated as i128)
            } else {
                integer_number(truncated as i128 - modulus as i128)
            }
        },
        kind => Err(format!("unsupported integer representation type: {kind:?}")),
    }
}

fn character_value(value: u128) -> Result<usize, String> {
    let value = u32::try_from(value)
        .map_err(|_| format!("character value {value} exceeds the Unicode code-point space"))?;
    char::from_u32(value)
        .map(|_| value as usize)
        .ok_or_else(|| format!("character value U+{value:04X} is not a Unicode scalar value"))
}

fn read_bool_constant(constant: &ConstOperand) -> Result<bool, String> {
    match constant.const_.kind() {
        ConstantKind::Allocated(allocation) => allocation
            .read_bool()
            .map_err(|error| format!("read Bool MIR constant: {error:?}")),
        kind => Err(format!(
            "unsupported Bool MIR constant representation: {kind:?}"
        )),
    }
}

fn read_character_constant(constant: &ConstOperand) -> Result<usize, String> {
    match constant.const_.kind() {
        ConstantKind::Allocated(allocation) => allocation
            .read_uint()
            .map_err(|error| format!("read character MIR constant: {error:?}"))
            .and_then(character_value),
        ConstantKind::Ty(value) => match value.kind() {
            TyConstKind::Value(_, allocation) => allocation
                .read_uint()
                .map_err(|error| format!("read typed character MIR constant: {error:?}"))
                .and_then(character_value),
            kind => Err(format!(
                "unsupported typed character MIR constant representation: {kind:?}"
            )),
        },
        kind => Err(format!(
            "unsupported character MIR constant representation: {kind:?}"
        )),
    }
}

fn read_integer_constant(constant: &ConstOperand) -> Result<Number, String> {
    match constant.const_.kind() {
        ConstantKind::Allocated(allocation) => {
            let value = allocation
                .read_uint()
                .map_err(|error| format!("read integer MIR constant: {error:?}"))?;
            integer_value(value, constant.ty())
        },
        ConstantKind::Ty(value) => match value.kind() {
            TyConstKind::Value(_, allocation) => {
                let value = allocation
                    .read_uint()
                    .map_err(|error| format!("read typed integer MIR constant: {error:?}"))?;
                integer_value(value, constant.ty())
            },
            kind => Err(format!(
                "unsupported typed integer MIR constant representation: {kind:?}"
            )),
        },
        kind => Err(format!(
            "unsupported integer MIR constant representation: {kind:?}"
        )),
    }
}

fn empty_contract() -> FunctionContract {
    FunctionContract {
        conditions: vec![],
        has_frame: false,
        loc: None,
        modifies: vec![],
        modifies_all: false,
        pragmas: vec![],
        reads: vec![],
        reads_all: false,
    }
}

fn byte_offset(source: &str, line: usize, column: usize) -> Result<usize, String> {
    if line == 0 || column == 0 {
        return Err(format!("invalid one-based source position {line}:{column}"));
    }
    let mut line_start = 0;
    let mut selected = None;
    for (index, text) in source.split_inclusive('\n').enumerate() {
        if index + 1 == line {
            selected = Some((line_start, text));
            break;
        }
        line_start += text.len();
    }
    if selected.is_none() && line == source.lines().count() + 1 && source.ends_with('\n') {
        selected = Some((source.len(), ""));
    }
    let (line_start, text) =
        selected.ok_or_else(|| format!("source line {line} is out of bounds"))?;
    let zero_based = column - 1;
    let within_line = text
        .char_indices()
        .map(|(offset, _)| offset)
        .nth(zero_based)
        .or_else(|| (zero_based == text.chars().count()).then_some(text.len()))
        .ok_or_else(|| format!("source column {column} is out of bounds on line {line}"))?;
    Ok(line_start + within_line)
}

#[cfg(test)]
mod tests {
    use super::byte_offset;

    #[test]
    fn converts_one_based_unicode_positions_to_bytes() {
        let source = "aé\nxyz\n";
        assert_eq!(byte_offset(source, 1, 1), Ok(0));
        assert_eq!(byte_offset(source, 1, 2), Ok(1));
        assert_eq!(byte_offset(source, 1, 3), Ok(3));
        assert_eq!(byte_offset(source, 2, 2), Ok(5));
        assert_eq!(byte_offset(source, 3, 1), Ok(source.len()));
    }
}
