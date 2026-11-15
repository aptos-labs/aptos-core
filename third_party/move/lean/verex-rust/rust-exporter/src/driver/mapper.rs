// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Mechanical Rustc Public to RawUnit mapping for the supported scalar CFG.

use super::{exchange::*, polyfill};
use rustc_public::{
    abi::{FieldsShape, LayoutShape, Scalar, TagEncoding, VariantsShape},
    crate_def::{CrateDef, CrateDefType, DefId},
    mir::{
        alloc::GlobalAlloc,
        mono::Instance,
        visit::{statement_location, terminator_location, Location as MirLocation, PlaceContext},
        AggregateKind, AssertMessage, BasicBlock, BinOp, Body, BorrowKind as MirBorrowKind,
        CastKind, ConstOperand, Local, LocalDecl as MirLocalDecl, MirVisitor, MutBorrowKind,
        Mutability, Operand, Place as MirPlace, PointerCoercion, ProjectionElem, RawPtrKind,
        Rvalue, Safety, SourceInfo, Statement, StatementKind, Terminator, TerminatorKind, UnOp,
        UnwindAction, VarDebugInfo, VarDebugInfoContents, WithRetag,
    },
    target::MachineInfo,
    ty::{
        Abi, AdtDef, AdtKind, AliasKind, Allocation, AssocContainer, AssocKind, BoundRegionKind,
        ClauseKind, ClosureDef, ConstantKind, EarlyParamRegion, FnDef, FnSig, GenericArgKind,
        GenericArgs, GenericParamDef, GenericParamDefKind, ImplDef, IntTy, MirConst,
        OutlivesPredicate, PolyFnSig, PredicateKind, PredicatePolarity, ProvenanceMap, Region,
        RegionKind, RigidTy, Span, TermKind, TraitDef, Ty as RustTy, TyConst, TyConstKind, TyKind,
        UintTy, UnevaluatedConst, VariantIdx,
    },
    Crate, CrateItem, CrateNum, ItemKind,
};
use serde_json::Number;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fs,
};

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
    bodies: Vec<PreparedBody>,
    /// Why an item's body is not mapped, by item.
    refusals: Vec<Option<Refusal>>,
    /// The local functions whose signatures the unit cannot state.
    opaque_functions: BTreeSet<String>,
    identities: Vec<String>,
    /// Each callable function by its definition path: the local functions and
    /// the dependency functions the bodies call.
    name_ids: BTreeMap<String, QualifiedRef>,
    dependency_functions: Vec<DependencyFunction>,
    constants: EvaluatedConstants,
    /// The type parameters declared Copy in each local function's environment.
    copy_parameters: Vec<BTreeSet<u32>>,
    impls: AdtImpls,
    /// The declared signature of each local function.
    signatures: Vec<DeclaredSignature>,
    method_evidence: BTreeMap<String, Vec<MethodEvidence>>,
    /// How many bound regions' binders each callable function has, by its
    /// definition path.
    bound_lifetimes: BTreeMap<String, usize>,
    resolves_trait_calls: bool,
    names: Vec<QualifiedName>,
    adts: Vec<AdtNames>,
    closures: Vec<ClosureNames>,
    fnmut_adapters: Vec<(usize, QualifiedRef)>,
    fnonce_adapters: Vec<(usize, QualifiedRef)>,
    evidence_adapters: Vec<(usize, usize, bool, QualifiedRef)>,
    scoped_evidence: BTreeSet<(usize, usize)>,
    types: TypeLayout,
    scopes: Scopes,
    namespaces: CrateNamespaces,
    sources: SourceTables,
    arenas: NamespaceArenas,
}

/// The scopes of the local crate's declarations: its modules, and the types,
/// traits, and trait impls associated functions belong to. Rustc Public does
/// not say which definitions are modules, so a module is an enclosing
/// definition that is none of the others an item can be nested in: a body
/// (function, constant, static) or the impl or trait of an associated item.
struct Scopes {
    enclosing: HashSet<DefId>,
    /// Each scope, by its crate and path, with its kind and the span of its
    /// declaration.
    declarations: BTreeMap<(String, Vec<String>), (ScopeKind, Span)>,
}

fn last_segment(path: &str) -> String {
    path.rsplit("::").next().unwrap_or(path).to_owned()
}

impl Scopes {
    fn new() -> Self {
        let local = rustc_public::local_crate();
        let mut enclosing = rustc_public::all_local_items()
            .iter()
            .map(CrateDef::def_id)
            .collect::<HashSet<_>>();
        enclosing.extend(local.trait_decls().iter().map(CrateDef::def_id));
        enclosing.extend(local.trait_impls().iter().map(CrateDef::def_id));
        for adt in local.adts() {
            enclosing.extend(adt.inherent_impls().iter().map(CrateDef::def_id));
        }
        for function in local.fn_defs() {
            if function.associated_item().is_some() {
                enclosing.extend(function.def_id().parent());
            }
        }
        Self {
            enclosing,
            declarations: BTreeMap::new(),
        }
    }

    fn declare(
        &mut self,
        krate: &str,
        path: &[String],
        kind: ScopeKind,
        span: Span,
    ) -> Result<(), String> {
        let key = (krate.to_owned(), path.to_vec());
        match self.declarations.get(&key) {
            Some((existing, _)) if *existing != kind => Err(format!(
                "scope `{}` is both a {existing:?} and a {kind:?} scope",
                path.join("::")
            )),
            Some(_) => Ok(()),
            None => {
                self.declarations.insert(key, (kind, span));
                Ok(())
            },
        }
    }

    /// The scope of a function: an associated function's is that of its
    /// type (an inherent impl's items), its trait, or its trait impl.
    fn function_scope(&mut self, function: FnDef) -> Result<Vec<String>, String> {
        let krate = function.krate().name;
        let Some(item) = function.associated_item() else {
            return Ok(self.module_scope(&krate, function.def_id()));
        };
        let container = function
            .def_id()
            .parent()
            .ok_or_else(|| format!("associated function `{}` has no container", function.name()))?;
        let (krate, mut path, segment, kind, span) = match item.container {
            AssocContainer::InherentImpl => match ImplDef(container).ty().kind() {
                TyKind::RigidTy(RigidTy::Adt(adt, _)) => {
                    let krate = adt.krate().name;
                    let path = self.module_scope(&krate, adt.def_id());
                    let name = last_segment(&adt.name());
                    (krate, path, name, ScopeKind::Inherent, adt.span())
                },
                // A primitive type's inherent impls are in `core`; its
                // associated functions are in the scope `primitive::<type>`.
                TyKind::RigidTy(primitive) => {
                    let name = primitive_name(&primitive).ok_or_else(|| {
                        format!(
                            "inherent impl `{}` is not on a nominal or primitive type",
                            container.name()
                        )
                    })?;
                    let path = vec!["primitive".to_owned()];
                    self.declare(&krate, &path, ScopeKind::Module, container.span())?;
                    (krate, path, name, ScopeKind::Inherent, container.span())
                },
                kind => {
                    return Err(format!(
                        "inherent impl `{}` is on the unsupported type {kind:?}",
                        container.name()
                    ));
                },
            },
            AssocContainer::TraitImpl(_) => {
                let path = self.module_scope(&krate, container);
                let span = container.span();
                (
                    krate,
                    path,
                    container.name(),
                    ScopeKind::Implementation,
                    span,
                )
            },
            AssocContainer::Trait => {
                let path = self.module_scope(&krate, container);
                let name = last_segment(&container.name());
                (krate, path, name, ScopeKind::Trait, container.span())
            },
        };
        path.push(segment);
        self.declare(&krate, &path, kind, span)?;
        Ok(path)
    }

    /// A closure's name and the scope of its body: the name is rustc's path below
    /// the enclosing module, the scope that name under the module.
    fn closure_scope(&mut self, closure: ClosureDef) -> (String, Vec<String>, Vec<String>) {
        let krate = closure.krate().name;
        let definition = closure.def_id();
        let module = self.module_scope(&krate, definition);
        let mut current = definition.parent();
        while let Some(ancestor) = current {
            if !self.enclosing.contains(&ancestor) {
                break;
            }
            current = ancestor.parent();
        }
        let path = definition.name();
        let name = current
            .and_then(|enclosing| {
                path.strip_prefix(&format!("{}::", enclosing.name()))
                    .map(str::to_owned)
            })
            .unwrap_or(path);
        let mut scope = module.clone();
        scope.push(name.clone());
        self.declarations
            .entry((krate, scope.clone()))
            .or_insert_with(|| (ScopeKind::Inherent, closure.span()));
        (name, module, scope)
    }

    /// The module path of a definition relative to the crate root, each
    /// module on it recorded.
    fn module_scope(&mut self, krate: &str, definition: DefId) -> Vec<String> {
        let mut modules = Vec::new();
        let mut current = definition.parent();
        while let Some(ancestor) = current {
            current = ancestor.parent();
            if current.is_some() && !self.enclosing.contains(&ancestor) {
                modules.push(ancestor);
            }
        }
        let mut path = Vec::with_capacity(modules.len());
        for module in modules.into_iter().rev() {
            path.push(last_segment(&module.name()));
            self.declarations
                .entry((krate.to_owned(), path.clone()))
                .or_insert_with(|| (ScopeKind::Module, module.span()));
        }
        path
    }
}

/// The generic parameters of a function in the order of rustc's generic
/// arguments: its impl's or trait's, then its own.
fn flattened_generics(function: FnDef) -> Result<Vec<GenericParamDef>, String> {
    let own = function.generics_of();
    let mut parameters = Vec::new();
    if own.parent.is_some() {
        let (item, container) = function
            .associated_item()
            .zip(function.def_id().parent())
            .ok_or_else(|| {
                format!(
                    "function `{}` has parent generics but no impl or trait",
                    function.name()
                )
            })?;
        parameters = match item.container {
            AssocContainer::Trait => {
                TraitDef::declaration(&TraitDef(container))
                    .generics_of()
                    .params
            },
            AssocContainer::InherentImpl | AssocContainer::TraitImpl(_) => {
                ImplDef(container).generics_of().params
            },
        };
    }
    if parameters.len() != own.parent_count {
        return Err(format!(
            "function `{}` has {} parent generic parameters, expected {}",
            function.name(),
            parameters.len(),
            own.parent_count
        ));
    }
    parameters.extend(own.params);
    // An impl header's elided lifetimes all have the name `'_`, and an `impl Trait`
    // parameter's name is its source text: binders are named apart, by position as a
    // signature's elided lifetime is.
    let taken = parameters
        .iter()
        .map(|parameter| parameter.name.clone())
        .collect::<Vec<_>>();
    for (position, parameter) in parameters.iter_mut().enumerate() {
        if parameter.name == ANONYMOUS_LIFETIME {
            let mut name = format!("'l{position}");
            while taken.contains(&name) {
                name.push('_');
            }
            parameter.name = name;
        } else if matches!(parameter.kind, GenericParamDefKind::Type {
            synthetic: true,
            ..
        }) {
            let mut name = format!("Impl{position}");
            while taken.contains(&name) {
                name.push('_');
            }
            parameter.name = name;
        }
    }
    Ok(parameters)
}

/// The closure a local item is the body of.
fn closure_of(item: CrateItem) -> Option<(ClosureDef, GenericArgs)> {
    match item.ty().kind() {
        TyKind::RigidTy(RigidTy::Closure(definition, arguments)) => Some((definition, arguments)),
        _ => None,
    }
}

/// The generic parameters of a local item: a closure's are those of the
/// function it stands in, without the kind, signature and upvars rustc adds.
fn item_generics(item: CrateItem) -> Result<Vec<GenericParamDef>, String> {
    if closure_of(item).is_none() {
        return flattened_generics(function_def(item)?);
    }
    match FnDef(item.0).generics_of().parent {
        Some(parent) => flattened_generics(FnDef(parent.0)),
        None => Err(format!("closure `{}` has no enclosing item", item.name())),
    }
}

/// A closure's arguments are its parent's, then its kind, signature and the
/// tuple of its upvars.
fn closure_arguments(
    arguments: &GenericArgs,
) -> Result<(Vec<GenericArgKind>, Vec<RustTy>), String> {
    let [parents @ .., _kind, _signature, GenericArgKind::Type(upvars)] = arguments.0.as_slice()
    else {
        return Err(format!("closure arguments {arguments:?} have no upvars"));
    };
    let TyKind::RigidTy(RigidTy::Tuple(upvars)) = upvars.kind() else {
        return Err(format!("closure upvars {upvars:?} are no tuple"));
    };
    Ok((parents.to_vec(), upvars))
}

/// The types of a closure's upvars and those of its parent's arguments.
fn closure_parts(arguments: &GenericArgs) -> Vec<RustTy> {
    let Ok((parents, upvars)) = closure_arguments(arguments) else {
        return vec![];
    };
    type_arguments(&GenericArgs(parents))
        .into_iter()
        .chain(upvars)
        .collect()
}

/// Whether a closure's value is zero-sized: its upvars, if any, are such closures.
/// rustc removes the assignments of such values.
fn is_zero_sized_closure(arguments: &GenericArgs) -> bool {
    closure_arguments(arguments).is_ok_and(|(_, upvars)| {
        upvars.into_iter().all(|upvar| match upvar.kind() {
            TyKind::RigidTy(RigidTy::Closure(_, nested)) => is_zero_sized_closure(&nested),
            _ => false,
        })
    })
}

fn is_zero_sized_closure_type(ty: RustTy) -> bool {
    matches!(ty.kind(), TyKind::RigidTy(RigidTy::Closure(_, arguments))
        if is_zero_sized_closure(&arguments))
}

/// The binder kind of a source generic parameter.
fn binder_kind(parameter: &GenericParamDef) -> BinderKind {
    match parameter.kind {
        GenericParamDefKind::Lifetime => BinderKind::Lifetime,
        GenericParamDefKind::Type { .. } => BinderKind::TypeArg,
        GenericParamDefKind::Const { .. } => BinderKind::Const,
    }
}

fn generic_binder(
    parameter: GenericParamDef,
    copy_parameters: &BTreeSet<u32>,
    loc: LocId,
    types: &mut TypeLayout,
) -> Result<GenericBinder, String> {
    let kind = binder_kind(&parameter);
    // rustc selects Copy in the item's environment, including inherited and
    // supertrait bounds; no inference from MIR operand spellings is needed.
    let abilities = if copy_parameters.contains(&parameter.index) {
        vec![Ability::Copy]
    } else {
        vec![]
    };
    Ok(GenericBinder {
        abilities,
        kind,
        loc,
        name: parameter.name,
        predicates: vec![],
        type_: if matches!(parameter.kind, GenericParamDefKind::Const { .. }) {
            Some(TypeUse {
                loc,
                type_id: types.id(polyfill::const_param_ty(parameter.def_id))?,
            })
        } else {
            None
        },
    })
}

/// The binders of a function's bound regions.
fn bound_lifetime_binders(
    signature: &DeclaredSignature,
    loc: LocId,
) -> impl Iterator<Item = GenericBinder> + '_ {
    signature
        .bound_lifetimes
        .iter()
        .map(move |(_, name)| GenericBinder {
            abilities: vec![],
            kind: BinderKind::Lifetime,
            loc,
            name: name.clone(),
            predicates: vec![],
            type_: None,
        })
}

fn erased() -> Region {
    Region {
        kind: RegionKind::ReErased,
    }
}

/// A type with its regions erased, as the types of MIR locals are. Rustc Public
/// builds every type with its regions erased, whichever it is given.
fn erase_regions(ty: RustTy) -> RustTy {
    match ty.kind() {
        TyKind::RigidTy(RigidTy::Ref(_, referent, mutability)) => {
            RustTy::new_ref(erased(), erase_regions(referent), mutability)
        },
        TyKind::RigidTy(RigidTy::RawPtr(pointee, mutability)) => {
            RustTy::new_ptr(erase_regions(pointee), mutability)
        },
        TyKind::RigidTy(RigidTy::Adt(definition, arguments)) => {
            let arguments = arguments
                .0
                .into_iter()
                .map(|argument| match argument {
                    GenericArgKind::Lifetime(_) => GenericArgKind::Lifetime(erased()),
                    GenericArgKind::Type(argument) => GenericArgKind::Type(erase_regions(argument)),
                    argument => argument,
                })
                .collect();
            RustTy::from_rigid_kind(RigidTy::Adt(definition, GenericArgs(arguments)))
        },
        TyKind::RigidTy(RigidTy::Tuple(elements)) => {
            RustTy::new_tuple(&elements.into_iter().map(erase_regions).collect::<Vec<_>>())
        },
        TyKind::RigidTy(RigidTy::Slice(element)) => {
            RustTy::from_rigid_kind(RigidTy::Slice(erase_regions(element)))
        },
        TyKind::RigidTy(RigidTy::Array(element, length)) => {
            RustTy::from_rigid_kind(RigidTy::Array(erase_regions(element), length))
        },
        _ => ty,
    }
}

/// The regions a type mentions, not those of a function pointer or an alias.
fn collect_regions(ty: RustTy, regions: &mut Vec<Region>) {
    let mut add = |region: Region| {
        if !regions.contains(&region) {
            regions.push(region);
        }
    };
    match ty.kind() {
        TyKind::RigidTy(RigidTy::FnPtr(_)) => return,
        TyKind::RigidTy(RigidTy::Ref(region, ..)) => add(region),
        TyKind::RigidTy(RigidTy::Adt(_, arguments)) => {
            for argument in arguments.0 {
                if let GenericArgKind::Lifetime(region) = argument {
                    add(region);
                }
            }
        },
        _ => {},
    }
    for child in type_children(ty) {
        collect_regions(child, regions);
    }
}

/// Clauses of an item and its enclosing impl/trait, preserving their binders.
fn item_predicates(definition: DefId) -> Vec<rustc_public::ty::Binder<PredicateKind>> {
    let declared = polyfill::explicit_predicates_of(definition);
    let mut predicates = declared.parent.map(item_predicates).unwrap_or_default();
    predicates.extend(
        declared
            .predicates
            .into_iter()
            .map(|(predicate, _)| predicate),
    );
    predicates
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EvidenceReceiver {
    Shared,
    Mutable,
    Owned,
}

/// A method dictionary retains its receiver as an ordinary argument. In
/// particular, a mutable receiver is never encoded as a captured, stateless
/// function. Local single-method traits and declared Fn/FnMut/FnOnce bounds use this interface.
#[derive(Clone)]
struct MethodEvidence {
    method: FnDef,
    arguments: GenericArgs,
    receiver: RustTy,
    name: String,
    tupled: bool,
    receiver_kind: EvidenceReceiver,
    result: RustTy,
    parameters: Vec<RustTy>,
}

impl MethodEvidence {
    /// Whether a borrowed dictionary supplies a weaker callable receiver mode.
    fn forwards_to(&self, required: &Self) -> bool {
        required.tupled
            && self.tupled
            && match required.receiver_kind {
                EvidenceReceiver::Mutable => self.receiver_kind == EvidenceReceiver::Shared,
                EvidenceReceiver::Owned => self.receiver_kind != EvidenceReceiver::Owned,
                EvidenceReceiver::Shared => false,
            }
            && self.arguments == required.arguments
            && self.result == required.result
    }

    /// The dictionary a call of `caller` requires: the declared evidence
    /// substituted at the call's arguments, normalized where the caller's
    /// environment determines an associated type.
    fn instantiate(&self, caller: DefId, arguments: &GenericArgs) -> Self {
        let instantiate =
            |ty: RustTy| normalized(caller, polyfill::instantiate_type(ty, arguments));
        let receiver = instantiate(self.receiver);
        let parameters = self
            .parameters
            .iter()
            .map(|ty| instantiate(*ty))
            .collect::<Vec<_>>();
        let mut trait_arguments = GenericArgs(vec![GenericArgKind::Type(receiver)]);
        if self.tupled {
            trait_arguments
                .0
                .push(GenericArgKind::Type(RustTy::from_rigid_kind(
                    RigidTy::Tuple(parameters.clone()),
                )));
        }
        Self {
            receiver,
            parameters,
            result: instantiate(self.result),
            arguments: trait_arguments,
            ..self.clone()
        }
    }

    fn receiver_type(&self) -> RustTy {
        if self.receiver_kind == EvidenceReceiver::Owned {
            return self.receiver;
        }
        RustTy::from_rigid_kind(RigidTy::Ref(
            erased(),
            self.receiver,
            if self.receiver_kind == EvidenceReceiver::Mutable {
                Mutability::Mut
            } else {
                Mutability::Not
            },
        ))
    }
}

/// The types a dictionary states, in one order for the declaration that binds
/// them and the call that instantiates them.
fn evidence_types(evidence: &[MethodEvidence]) -> Vec<RustTy> {
    evidence
        .iter()
        .flat_map(|evidence| {
            [evidence.receiver_type(), evidence.result]
                .into_iter()
                .chain(evidence.parameters.iter().copied())
        })
        .collect()
}

fn declared_method_evidence(definition: DefId) -> Vec<MethodEvidence> {
    let mut evidence = Vec::new();
    let predicates = item_predicates(definition);
    for predicate in &predicates {
        if !predicate.bound_vars.is_empty() {
            continue;
        }
        let PredicateKind::Clause(ClauseKind::Trait(predicate)) = &predicate.value else {
            continue;
        };
        if predicate.polarity != PredicatePolarity::Positive {
            continue;
        }
        let trait_ = predicate.trait_ref.def_id;
        let shared = Some(trait_) == polyfill::fn_trait();
        let owned = Some(trait_) == polyfill::fn_once_trait();
        if shared || owned || Some(trait_) == polyfill::fn_mut_trait() {
            let arguments = predicate.trait_ref.args();
            let [GenericArgKind::Type(receiver), GenericArgKind::Type(tuple)] =
                arguments.0.as_slice()
            else {
                continue;
            };
            let TyKind::RigidTy(RigidTy::Tuple(parameters)) = tuple.kind() else {
                continue;
            };
            if !matches!(receiver.kind(), TyKind::Param(_)) {
                continue;
            }
            // Fn and FnMut inherit Output from FnOnce. Require the declared equality;
            // neither an observed call nor an unconstrained projection suffices.
            let result = predicates.iter().find_map(|bound| {
                if !bound.bound_vars.is_empty() {
                    return None;
                }
                let PredicateKind::Clause(ClauseKind::Projection(projection)) = &bound.value else {
                    return None;
                };
                if projection.projection_term.args != *arguments
                    || Some(projection.projection_term.def_id.0) != polyfill::fn_once_output()
                {
                    return None;
                }
                let TermKind::Type(result) = projection.term else {
                    return None;
                };
                (collect_signature_adts(result, &[], &mut Vec::new()).is_ok()
                    && parameters
                        .iter()
                        .all(|ty| collect_signature_adts(*ty, &[], &mut Vec::new()).is_ok()))
                .then_some(result)
            });
            let Some(result) = result else { continue };
            // A declared bound names an associated type as it was written. Its
            // dictionary states what rustc normalizes that to in this item's
            // environment, so a declaration and its calls state one type.
            let result = normalized(definition, result);
            let parameters = parameters
                .into_iter()
                .map(|ty| normalized(definition, ty))
                .collect::<Vec<_>>();
            let normalized_arguments = GenericArgs(vec![
                GenericArgKind::Type(*receiver),
                GenericArgKind::Type(RustTy::from_rigid_kind(RigidTy::Tuple(parameters.clone()))),
            ]);
            let method = trait_.associated_items().into_iter().find_map(|item| {
                matches!(&item.kind, AssocKind::Fn { name, has_self: true }
                    if name == if shared { "call" } else if owned { "call_once" } else { "call_mut" })
                .then_some(FnDef(item.def_id.0))
            });
            if let Some(method) = method
                && !evidence.iter().any(|entry: &MethodEvidence| {
                    entry.method == method && entry.arguments == normalized_arguments
                })
            {
                evidence.push(MethodEvidence {
                    method,
                    arguments: normalized_arguments,
                    receiver: *receiver,
                    name: if shared {
                        "__trait_Fn_call"
                    } else if owned {
                        "__trait_FnOnce_call_once"
                    } else {
                        "__trait_FnMut_call_mut"
                    }
                    .to_owned(),
                    tupled: true,
                    receiver_kind: if shared {
                        EvidenceReceiver::Shared
                    } else if owned {
                        EvidenceReceiver::Owned
                    } else {
                        EvidenceReceiver::Mutable
                    },
                    result,
                    parameters,
                });
            }
            continue;
        }
        if !trait_.krate().is_local {
            continue;
        }
        let items = trait_.associated_items();
        let [item] = items.as_slice() else { continue };
        let AssocKind::Fn {
            name,
            has_self: true,
        } = &item.kind
        else {
            continue;
        };
        let receiver = predicate.trait_ref.self_ty();
        if !matches!(receiver.kind(), TyKind::Param(_)) {
            continue;
        }
        let method = FnDef(item.def_id.0);
        if flattened_generics(method).map_or(true, |parameters| parameters.len() != 1) {
            continue;
        }
        let signature = method.fn_sig().value;
        let [input, output] = signature.inputs_and_output.as_slice() else {
            continue;
        };
        let TyKind::RigidTy(RigidTy::Ref(_, referent, Mutability::Mut)) = input.kind() else {
            continue;
        };
        if !matches!(referent.kind(), TyKind::Param(p) if p.index == 0)
            || !matches!(output.kind(), TyKind::RigidTy(RigidTy::Tuple(ts)) if ts.is_empty())
            || signature.safety != Safety::Safe
            || signature.c_variadic
            || predicate.trait_ref.args().0.len() != 1
        {
            continue;
        }
        let entry = MethodEvidence {
            method,
            arguments: predicate.trait_ref.args().clone(),
            receiver,
            name: format!("__trait_{}_{}", last_segment(&trait_.name()), name),
            tupled: false,
            receiver_kind: EvidenceReceiver::Mutable,
            result: *output,
            parameters: vec![],
        };
        if !evidence
            .iter()
            .any(|e: &MethodEvidence| e.method == method && e.receiver == receiver)
        {
            evidence.push(entry);
        }
    }
    evidence
}

/// Explicit region bounds only. A higher-ranked clause is not an edge between
/// this item's generic regions, so it must not lose its quantifier here.
fn lifetime_predicates(definition: DefId, types: &mut TypeLayout) -> Vec<GenericPredicate> {
    item_predicates(definition)
        .into_iter()
        .filter_map(|predicate| {
            if !predicate.bound_vars.is_empty() {
                return None;
            }
            match predicate.value {
                PredicateKind::Clause(ClauseKind::RegionOutlives(OutlivesPredicate(
                    longer,
                    shorter,
                ))) => Some(GenericPredicate::LifetimeOutlives {
                    longer: types.lifetime_id(longer),
                    shorter: types.lifetime_id(shorter),
                }),
                _ => None,
            }
        })
        .collect()
}

/// Well-formed input ADTs imply their declared region bounds, instantiated
/// with the signature's regions (including late-bound lifetime parameters).
fn implied_adt_relations(ty: RustTy, found: &mut Vec<(Region, Region)>) {
    // A function pointer's signature is not a well-formed value held by this
    // function. Its binders and implied bounds belong to the pointed-to call.
    if matches!(ty.kind(), TyKind::RigidTy(RigidTy::FnPtr(_))) {
        return;
    }
    if let TyKind::RigidTy(RigidTy::Adt(definition, arguments)) = ty.kind() {
        let substitute = |region: Region| match &region.kind {
            RegionKind::ReEarlyParam(parameter) => {
                match arguments.0.get(parameter.index as usize) {
                    Some(GenericArgKind::Lifetime(argument)) => argument.clone(),
                    _ => region,
                }
            },
            _ => region,
        };
        for predicate in item_predicates(definition.0) {
            if predicate.bound_vars.is_empty()
                && let PredicateKind::Clause(ClauseKind::RegionOutlives(OutlivesPredicate(
                    longer,
                    shorter,
                ))) = predicate.value
            {
                let relation = (substitute(longer), substitute(shorter));
                if !found.contains(&relation) {
                    found.push(relation);
                }
            }
        }
    }
    for child in type_children(ty) {
        implied_adt_relations(child, found);
    }
}

fn signature_lifetime_predicates(
    definition: DefId,
    signature: &DeclaredSignature,
    types: &mut TypeLayout,
) -> Vec<GenericPredicate> {
    let mut predicates = lifetime_predicates(definition, types);
    let mut implied = Vec::new();
    for ty in signature
        .types
        .iter()
        .take(signature.types.len().saturating_sub(1))
    {
        implied_adt_relations(*ty, &mut implied);
    }
    predicates.extend(implied.into_iter().map(|(longer, shorter)| {
        GenericPredicate::LifetimeOutlives {
            longer: types.lifetime_id(longer),
            shorter: types.lifetime_id(shorter),
        }
    }));
    predicates
}

fn regions_have_early_parameters(types: &[RustTy]) -> bool {
    let mut regions = Vec::new();
    for ty in types {
        collect_regions(*ty, &mut regions);
    }
    regions
        .iter()
        .any(|region| matches!(region.kind, RegionKind::ReEarlyParam(_)))
}

/// The neutral IR does not yet carry nominal variance. Preserve the previous
/// inferred-region boundary for signatures that coerce the same nominal across
/// different lifetimes, rather than teaching the core an unchecked variance rule.
fn needs_nominal_variance(types: &[RustTy]) -> bool {
    fn nominals(ty: RustTy, found: &mut Vec<RustTy>) {
        if matches!(ty.kind(), TyKind::RigidTy(RigidTy::FnPtr(_))) {
            return;
        }
        if matches!(ty.kind(), TyKind::RigidTy(RigidTy::Adt(..))) {
            found.push(ty);
        }
        for child in type_children(ty) {
            nominals(child, found);
        }
    }
    let Some((result, inputs)) = types.split_last() else {
        return false;
    };
    let mut results = Vec::new();
    nominals(*result, &mut results);
    let mut arguments = Vec::new();
    for input in inputs {
        nominals(*input, &mut arguments);
    }
    results.iter().any(|result| {
        arguments.iter().any(|argument| {
            result != argument && erase_regions(*result) == erase_regions(*argument)
        })
    })
}

/// A function's signature as declared: its types with the regions
/// `FnDef::fn_sig` gives them, each bound region a lifetime binder of its own after
/// the function's other binders.
struct DeclaredSignature {
    /// Its parameter types, then its result type.
    types: Vec<RustTy>,
    /// The bound regions it keeps, in the order of their binders, with their names.
    bound_lifetimes: Vec<(Region, String)>,
    /// Its first bound region's binder.
    first_binder: usize,
}

impl DeclaredSignature {
    /// A closure's body, which no item declares: its parameters and result are its
    /// locals, with their regions inferred, and it has no binder of its own after its
    /// parent's. Its struct's binders hold the regions of its upvars.
    fn of_closure(body: &PreparedBody, first_binder: usize) -> Self {
        let locals = &body.mapped_locals;
        let arguments = body.arg_locals().len();
        Self {
            types: locals[1..=arguments]
                .iter()
                .chain(std::iter::once(&locals[0]))
                .map(|local| local.ty)
                .collect(),
            bound_lifetimes: vec![],
            first_binder,
        }
    }

    /// Keep declared early-bound regions and informative late-bound regions.
    /// An elided lifetime relating no result to a parameter stays inferred.
    fn new(function: FnDef, first_binder: usize) -> Self {
        let types = polyfill::declared_signature(function)
            .value
            .inputs_and_output;
        let mut regions = Vec::new();
        for ty in &types {
            collect_regions(*ty, &mut regions);
        }
        let mut output_regions = Vec::new();
        if let Some(output) = types.last() {
            collect_regions(*output, &mut output_regions);
        }
        let named = |region: &Region| !matches!(region.kind, RegionKind::ReErased);
        let informative =
            output_regions.iter().any(named) && regions.iter().filter(|r| named(r)).count() > 1;
        let kept = regions
            .into_iter()
            .filter_map(|region| {
                let RegionKind::ReBound(_, bound) = &region.kind else {
                    return None;
                };
                // An unnamed bound region identifies no definition, so it could
                // not be told from another function's.
                let name = match &bound.kind {
                    BoundRegionKind::BrNamed(_, name) => name.clone(),
                    BoundRegionKind::BrAnon | BoundRegionKind::BrEnv => return None,
                };
                (informative || name != ANONYMOUS_LIFETIME).then_some((region, name))
            })
            .collect::<Vec<_>>();
        // Binders are named apart: an elided lifetime by its position.
        let mut taken = kept
            .iter()
            .map(|(_, name)| name.clone())
            .collect::<Vec<_>>();
        if let Ok(generics) = flattened_generics(function) {
            taken.extend(generics.into_iter().map(|parameter| parameter.name));
        }
        let bound_lifetimes = kept
            .into_iter()
            .enumerate()
            .map(|(position, (region, name))| {
                if name != ANONYMOUS_LIFETIME {
                    return (region, name);
                }
                let mut name = format!("'l{}", first_binder + position);
                while taken.contains(&name) {
                    name.push('_');
                }
                (region, name)
            })
            .collect::<Vec<_>>();
        let types = if needs_nominal_variance(&types)
            || (bound_lifetimes.is_empty() && !regions_have_early_parameters(&types))
        {
            types.into_iter().map(erase_regions).collect()
        } else {
            types
        };
        Self {
            types,
            bound_lifetimes,
            first_binder,
        }
    }

    /// The declared types of a body, each where rustc normalized it to no other
    /// type than the local's. Where it did, the local's erased type stands.
    fn at_locals(mut self, body: &PreparedBody) -> Self {
        let argument_count = body.arg_locals().len();
        let locals = &body.mapped_locals;
        for (index, ty) in self.types.iter_mut().enumerate() {
            let local = if index == argument_count {
                &locals[0]
            } else {
                &locals[index + 1]
            };
            if erase_regions(*ty) != local.ty {
                *ty = local.ty;
            }
        }
        self
    }

    /// Each bound region with what the lifetime table says of it.
    fn region_specs(&self) -> impl Iterator<Item = LifetimeSpec> + '_ {
        self.bound_lifetimes
            .iter()
            .enumerate()
            .map(|(position, (region, name))| LifetimeSpec {
                region: region.clone(),
                kind: LifetimeKind::Parameter {
                    index: self.first_binder + position,
                },
                name: Some(name.clone()),
            })
    }
}

/// The binder name of a lifetime its source does not name.
const ANONYMOUS_LIFETIME: &str = "'_";

/// How many regions a type states: of its references and its ADTs' arguments, a
/// closure's being its own.
fn regions_in(ty: RustTy) -> usize {
    match ty.kind() {
        TyKind::RigidTy(RigidTy::Ref(_, referent, _)) => 1 + regions_in(referent),
        TyKind::RigidTy(RigidTy::Adt(_, arguments)) => arguments
            .0
            .into_iter()
            .map(|argument| match argument {
                GenericArgKind::Lifetime(_) => 1,
                GenericArgKind::Type(argument) => regions_in(argument),
                GenericArgKind::Const(_) => 0,
            })
            .sum(),
        TyKind::RigidTy(RigidTy::Tuple(elements)) => elements.into_iter().map(regions_in).sum(),
        TyKind::RigidTy(RigidTy::Slice(element) | RigidTy::Array(element, _)) => {
            regions_in(element)
        },
        _ => 0,
    }
}

/// The types a type is built of. An alias is a leaf.
fn type_children(ty: RustTy) -> Vec<RustTy> {
    let TyKind::RigidTy(rigid) = ty.kind() else {
        return vec![];
    };
    match rigid {
        RigidTy::Ref(_, inner, _)
        | RigidTy::RawPtr(inner, _)
        | RigidTy::Slice(inner)
        | RigidTy::Array(inner, _)
        | RigidTy::Pat(inner, _) => vec![inner],
        RigidTy::Tuple(elements) => elements,
        RigidTy::FnPtr(signature) => signature.value.inputs_and_output,
        RigidTy::Closure(_, arguments) => closure_parts(&arguments),
        RigidTy::Adt(_, arguments)
        | RigidTy::FnDef(_, arguments)
        | RigidTy::Coroutine(_, arguments)
        | RigidTy::CoroutineClosure(_, arguments)
        | RigidTy::CoroutineWitness(_, arguments) => type_arguments(&arguments),
        RigidTy::Bool
        | RigidTy::Char
        | RigidTy::Int(_)
        | RigidTy::Uint(_)
        | RigidTy::Float(_)
        | RigidTy::Foreign(_)
        | RigidTy::Str
        | RigidTy::Never
        | RigidTy::Dynamic(..) => vec![],
    }
}

fn type_arguments(arguments: &GenericArgs) -> Vec<RustTy> {
    arguments
        .0
        .iter()
        .filter_map(|argument| match argument {
            GenericArgKind::Type(argument) => Some(*argument),
            GenericArgKind::Lifetime(_) | GenericArgKind::Const(_) => None,
        })
        .collect()
}

/// The first alias a type names, which only a dependency's signature may.
fn projection(ty: RustTy) -> Option<RustTy> {
    match ty.kind() {
        TyKind::Alias(..) => Some(ty),
        _ => type_children(ty).into_iter().find_map(projection),
    }
}

/// A type naming an associated type, as rustc normalizes it in `owner`'s
/// environment. One naming none is already the type rustc gave us, and keeps
/// its regions; one rustc cannot normalize, because its projection stands on a
/// type parameter, is returned for a binder to carry.
fn normalized(owner: DefId, ty: RustTy) -> RustTy {
    if projection(ty).is_none() {
        return ty;
    }
    polyfill::normalize_type(owner, ty)
}

const PARAMETER_DEPENDENT_CALL: &str = "depends on a type parameter and needs trait evidence: rustc selects its implementation only at a concrete type";

/// The variant a MIR node is, without its contents, whose spans name absolute paths.
fn variant_name(node: &impl std::fmt::Debug) -> String {
    let text = format!("{node:?}");
    text.split([' ', '(']).next().unwrap_or_default().to_owned()
}

/// Whether a type names an associated type of a type parameter.
fn names_parameter_projection(ty: RustTy) -> bool {
    match ty.kind() {
        TyKind::Alias(..) => mentions_parameter(ty),
        _ => type_children(ty)
            .into_iter()
            .any(names_parameter_projection),
    }
}

/// Whether a type names a type parameter, also inside an alias.
fn mentions_parameter(ty: RustTy) -> bool {
    match ty.kind() {
        TyKind::Param(_) => true,
        TyKind::Alias(_, alias) => type_arguments(&alias.args)
            .into_iter()
            .any(mentions_parameter),
        _ => type_children(ty).into_iter().any(mentions_parameter),
    }
}

/// The function a call reaches, as codegen resolves it: a trait method called
/// on a concrete type resolves to its impl's method. rustc selects an impl only
/// for concrete types, so a call that fails here on a type parameter is one a
/// bound of the caller would have to justify.
fn resolve_callee(
    caller: DefId,
    callee: FnDef,
    arguments: &GenericArgs,
) -> Result<(FnDef, GenericArgs), String> {
    // rustc does not answer for an associated type of a parameter: it cannot normalize it.
    if type_arguments(arguments)
        .into_iter()
        .any(names_parameter_projection)
    {
        return Err(PARAMETER_DEPENDENT_CALL.to_owned());
    }
    match polyfill::resolve_in_caller(caller, callee, arguments)? {
        polyfill::ResolvedCallee::Item(function, arguments) => Ok((function, arguments)),
        polyfill::ResolvedCallee::Closure(closure, arguments) => Ok((FnDef(closure.0), arguments)),
        polyfill::ResolvedCallee::TraitParam { trait_method, args } => {
            debug_assert_eq!(trait_method, callee);
            // M2 consumes these normalized arguments with the selected evidence.
            drop(args);
            Err(PARAMETER_DEPENDENT_CALL.to_owned())
        },
        polyfill::ResolvedCallee::Unsupported(reason) => Err(reason.to_owned()),
    }
}

/// The closure a call of an `Fn*` method on a closure type reaches: its body, of
/// the closure's own kind. Any other kind goes through a shim `resolve_callee`
/// rejects.
fn closure_callee(
    caller: DefId,
    definition: FnDef,
    arguments: &GenericArgs,
) -> Option<(ClosureDef, GenericArgs)> {
    let GenericArgKind::Type(receiver) = arguments.0.first()? else {
        return None;
    };
    let TyKind::RigidTy(RigidTy::Closure(closure, closure_arguments)) = receiver.kind() else {
        return None;
    };
    let (resolved, _) = resolve_callee(caller, definition, arguments).ok()?;
    (resolved.def_id() == closure.def_id()).then_some((closure, closure_arguments))
}

/// An associated type a dependency's signature names: a type parameter of the
/// declaration, after the function's generics. A call's instantiation states
/// what rustc normalized it to.
struct AssociatedType {
    alias: RustTy,
    /// Every occurrence is behind a shared reference. Promote the whole
    /// reference so an unsized associated referent is never a sized actual.
    shared_reference: bool,
    /// Its index among the function's generic arguments.
    index: u32,
    name: String,
}

/// A dependency crate's function the bodies call, declared without its body.
struct DependencyFunction {
    definition: FnDef,
    /// Its parameter and result types.
    signature: Vec<RustTy>,
    /// What the dictionaries of its declared `Fn*` bounds state. A bound may
    /// name an associated type its signature does not, so these bind binders too.
    evidence: Vec<RustTy>,
    associated_types: Vec<AssociatedType>,
    /// Whether the unit cannot state its signature or admit its unsafety. It is
    /// then declared with no generics or parameters, diverging: nothing reads
    /// the declaration, because validation rejects a call that survives import.
    opaque: bool,
    /// Its declared signature, whose binders follow its associated types'.
    declared: DeclaredSignature,
}

impl DependencyFunction {
    /// The function's declaration, opaque if its signature or its unsafety is
    /// outside the supported slice.
    fn new(function: FnDef) -> Self {
        Self::declared(function).unwrap_or_else(|_| Self {
            definition: function,
            signature: vec![RustTy::from_rigid_kind(RigidTy::Never)],
            evidence: vec![],
            associated_types: vec![],
            opaque: true,
            declared: DeclaredSignature {
                types: vec![],
                bound_lifetimes: vec![],
                first_binder: 0,
            },
        })
    }

    fn declared(function: FnDef) -> Result<Self, String> {
        let signature = polyfill::declared_signature(function).value;
        if signature.c_variadic || signature.safety != Safety::Safe || signature.abi != Abi::Rust {
            return Err(format!(
                "dependency function `{}` is not a safe Rust-ABI function",
                function.name()
            ));
        }
        let types = signature.inputs_and_output;
        let generics = flattened_generics(function)?;
        let mut declared = Self {
            definition: function,
            signature: types.clone(),
            evidence: evidence_types(&declared_method_evidence(function.0)),
            associated_types: vec![],
            opaque: false,
            declared: DeclaredSignature {
                types: vec![],
                bound_lifetimes: vec![],
                first_binder: 0,
            },
        };
        let stated = types
            .iter()
            .copied()
            .chain(declared.evidence.clone())
            .collect::<Vec<_>>();
        for ty in &stated {
            declared
                .collect_associated_types(*ty, &generics)
                .and_then(|()| collect_signature_adts(*ty, &[], &mut Vec::new()))
                .map_err(|error| {
                    format!(
                        "dependency function `{}` has a signature outside the supported slice: {error}",
                        function.name()
                    )
                })?;
        }
        let signature =
            DeclaredSignature::new(function, generics.len() + declared.associated_types.len());
        declared.signature = signature.types.clone();
        declared.declared = signature;
        Ok(declared)
    }

    fn collect_associated_types(
        &mut self,
        ty: RustTy,
        generics: &[GenericParamDef],
    ) -> Result<(), String> {
        let TyKind::Alias(kind, alias) = ty.kind() else {
            for child in type_children(ty) {
                self.collect_associated_types(child, generics)?;
            }
            return Ok(());
        };
        if kind != AliasKind::Projection {
            return Err(format!("the type alias `{ty}` is not an associated type"));
        }
        if self.associated_types.iter().any(|known| known.alias == ty) {
            return Ok(());
        }
        // Two crates' associated types, or a parameter, may share the item's name.
        fn shared_occurrences(ty: RustTy, alias: RustTy, regions: &mut Vec<Region>) -> bool {
            if ty == alias {
                return false;
            }
            if let TyKind::RigidTy(RigidTy::Ref(region, referent, Mutability::Not)) = ty.kind()
                && referent == alias
            {
                regions.push(region);
                return true;
            }
            type_children(ty)
                .into_iter()
                .all(|child| shared_occurrences(child, alias, regions))
        }
        let mut regions = Vec::new();
        let shared_reference = self
            .signature
            .iter()
            .chain(&self.evidence)
            .all(|stated| shared_occurrences(*stated, ty, &mut regions))
            && regions
                .first()
                .is_some_and(|first| regions.iter().all(|region| region == first));
        let segment = if shared_reference {
            format!("Borrowed{}", last_segment(&alias.def_id.name()))
        } else {
            last_segment(&alias.def_id.name())
        };
        let taken = |name: &str| {
            generics.iter().any(|parameter| parameter.name == name)
                || self.associated_types.iter().any(|known| known.name == name)
        };
        let name = std::iter::once(segment.clone())
            .chain((2..).map(|suffix| format!("{segment}{suffix}")))
            .find(|name| !taken(name))
            .expect("the suffixes are unbounded");
        let index = generics.len() + self.associated_types.len();
        self.associated_types.push(AssociatedType {
            alias: ty,
            shared_reference,
            index: index as u32,
            name,
        });
        Ok(())
    }

    /// What a call instantiates the associated types with, in their order:
    /// the declared signature and the declared dictionaries unified with the
    /// types at the call, which rustc has normalized. `arguments` are the
    /// function's generic arguments.
    fn bind_associated_types(
        &self,
        arguments: &GenericArgs,
        actual: &[RustTy],
        actual_evidence: &[RustTy],
    ) -> Result<Vec<RustTy>, String> {
        if self.signature.len() != actual.len() {
            return Err(format!(
                "the signature of `{}` has {} types, the call {}",
                self.definition.name(),
                self.signature.len(),
                actual.len()
            ));
        }
        if self.evidence.len() != actual_evidence.len() {
            return Err(format!(
                "the dictionaries of `{}` state {} types, the call {}",
                self.definition.name(),
                self.evidence.len(),
                actual_evidence.len()
            ));
        }
        let mut bindings = vec![None; self.associated_types.len()];
        for (declared, actual) in self
            .signature
            .iter()
            .chain(&self.evidence)
            .zip(actual.iter().chain(actual_evidence))
        {
            self.unify(*declared, *actual, arguments, &mut bindings)?;
        }
        bindings
            .into_iter()
            .zip(&self.associated_types)
            .map(|(binding, associated)| {
                binding.ok_or_else(|| {
                    format!(
                        "no type at the call binds the associated type `{}`",
                        associated.alias
                    )
                })
            })
            .collect()
    }

    fn unify(
        &self,
        declared: RustTy,
        actual: RustTy,
        arguments: &GenericArgs,
        bindings: &mut [Option<RustTy>],
    ) -> Result<(), String> {
        let mismatch = || format!("the declared type `{declared}` does not unify with `{actual}`");
        let same = |left: RustTy, right: RustTy| -> Result<bool, String> {
            Ok(erase_regions(left) == erase_regions(right))
        };
        if let TyKind::RigidTy(RigidTy::Ref(_, referent, Mutability::Not)) = declared.kind()
            && let Some(position) = self
                .associated_types
                .iter()
                .position(|associated| associated.shared_reference && associated.alias == referent)
        {
            if !matches!(
                actual.kind(),
                TyKind::RigidTy(RigidTy::Ref(_, _, Mutability::Not))
            ) {
                return Err(mismatch());
            }
            return match bindings[position] {
                Some(bound) if bound != actual => Err(mismatch()),
                Some(_) => Ok(()),
                None => {
                    bindings[position] = Some(actual);
                    Ok(())
                },
            };
        }
        match declared.kind() {
            TyKind::Alias(..) => {
                let position = self
                    .associated_types
                    .iter()
                    .position(|associated| associated.alias == declared)
                    .ok_or_else(|| format!("the type `{declared}` is not declared"))?;
                match bindings[position] {
                    Some(bound) if !same(bound, actual)? => Err(format!(
                        "the associated type `{declared}` is both `{bound}` and `{actual}`"
                    )),
                    Some(_) => Ok(()),
                    None => {
                        bindings[position] = Some(actual);
                        Ok(())
                    },
                }
            },
            TyKind::Param(parameter) => match arguments.0.get(parameter.index as usize) {
                Some(GenericArgKind::Type(argument)) if same(*argument, actual)? => Ok(()),
                _ => Err(mismatch()),
            },
            TyKind::RigidTy(rigid) => {
                let TyKind::RigidTy(other) = actual.kind() else {
                    return Err(mismatch());
                };
                let children = type_children(declared);
                let others = type_children(actual);
                if !same_constructor(&rigid, &other) || children.len() != others.len() {
                    return Err(mismatch());
                }
                for (child, other) in children.into_iter().zip(others) {
                    self.unify(child, other, arguments, bindings)?;
                }
                Ok(())
            },
            TyKind::Bound(..) => Err(mismatch()),
        }
    }
}

/// Whether two rigid types agree on all but the types they are built of.
fn same_constructor(left: &RigidTy, right: &RigidTy) -> bool {
    let constants = |arguments: &GenericArgs| {
        arguments
            .0
            .iter()
            .filter(|argument| matches!(argument, GenericArgKind::Const(_)))
            .cloned()
            .collect::<Vec<_>>()
    };
    match (left, right) {
        (RigidTy::Ref(_, _, left), RigidTy::Ref(_, _, right)) => left == right,
        (RigidTy::Adt(left, left_arguments), RigidTy::Adt(right, right_arguments)) => {
            left == right && constants(left_arguments) == constants(right_arguments)
        },
        (RigidTy::Array(_, left), RigidTy::Array(_, right)) => {
            left.eval_target_usize().ok() == right.eval_target_usize().ok()
        },
        (RigidTy::Slice(_), RigidTy::Slice(_))
        | (RigidTy::Tuple(_), RigidTy::Tuple(_))
        | (RigidTy::FnPtr(_), RigidTy::FnPtr(_)) => true,
        _ => left == right,
    }
}

/// The ADTs a dependency's signature reaches. An associated type reaches none.
fn collect_signature_adts(
    ty: RustTy,
    observed: &[AdtDef],
    adts: &mut Vec<ReachedAdt>,
) -> Result<(), String> {
    if projection(ty).is_none() {
        return collect_type_adts(ty, observed, adts);
    }
    match ty.kind() {
        TyKind::Alias(..) => Ok(()),
        TyKind::RigidTy(
            RigidTy::Ref(..)
            | RigidTy::Slice(_)
            | RigidTy::Array(..)
            | RigidTy::Tuple(_)
            | RigidTy::FnPtr(_),
        ) => {
            for child in type_children(ty) {
                collect_signature_adts(child, observed, adts)?;
            }
            Ok(())
        },
        TyKind::RigidTy(RigidTy::Adt(definition, _)) => {
            for child in type_children(ty) {
                collect_signature_adts(child, observed, adts)?;
            }
            collect_adt(definition, observed, adts)
        },
        kind => Err(format!("unsupported Rust local type: {kind:?}")),
    }
}

/// The largest parts of a signature's type that name no associated type.
fn parts_without_associated_types(ty: RustTy) -> Vec<RustTy> {
    match (projection(ty), ty.kind()) {
        (None, _) => vec![ty],
        (Some(_), TyKind::Alias(..)) => vec![],
        (Some(_), _) => type_children(ty)
            .into_iter()
            .flat_map(parts_without_associated_types)
            .collect(),
    }
}

/// The pragma of a namespace that is not a verification target.
fn verify_false() -> Attribute {
    Attribute::Assign {
        loc: None,
        name: "verify".to_owned(),
        value: AttributeValue::Constant {
            value: ConstValue::Bool { value: false },
        },
    }
}

/// The Rust documentation's name of a primitive type that has inherent
/// impls in `core`, the scope of its associated functions.
fn primitive_name(ty: &RigidTy) -> Option<String> {
    Some(match ty {
        RigidTy::Bool => "bool".to_owned(),
        RigidTy::Char => "char".to_owned(),
        RigidTy::Int(kind) => format!("{kind:?}").to_lowercase(),
        RigidTy::Uint(kind) => format!("{kind:?}").to_lowercase(),
        RigidTy::Float(kind) => format!("{kind:?}").to_lowercase(),
        RigidTy::Str => "str".to_owned(),
        RigidTy::Slice(_) => "slice".to_owned(),
        RigidTy::Array(..) => "array".to_owned(),
        RigidTy::RawPtr(_, Mutability::Not) => "const_ptr".to_owned(),
        RigidTy::RawPtr(_, Mutability::Mut) => "mut_ptr".to_owned(),
        _ => return None,
    })
}

/// What the impls of a crate state of its ADTs.
struct CrateImpls {
    /// The ADTs with a `Copy` impl applying the ADT to the impl's own
    /// parameters in order: those every instantiation of which with `Copy`
    /// arguments is `Copy`, as `#[derive(Copy)]` states it. An impl for one
    /// instantiation (`impl Copy for S<u32>`) has no such statement.
    copy: Vec<AdtDef>,
    /// The ADTs with a `Drop` impl. rustc admits one only in the ADT's own
    /// crate (E0120) and only for all its instantiations (E0366).
    drop: Vec<AdtDef>,
}

fn crate_impls(krate: &Crate) -> CrateImpls {
    let mut impls = CrateImpls {
        copy: vec![],
        drop: vec![],
    };
    for implementation in krate.trait_impls() {
        let header = implementation.trait_impl().value;
        let copy = Some(header.def_id.0) == polyfill::copy_trait();
        let drop = Some(header.def_id.0) == polyfill::drop_trait();
        if !copy && !drop {
            continue;
        }
        let TyKind::RigidTy(RigidTy::Adt(definition, arguments)) = header.self_ty().kind() else {
            continue;
        };
        if drop {
            impls.drop.push(definition);
        }
        let general = arguments
            .0
            .iter()
            .enumerate()
            .all(|(index, argument)| match argument {
                GenericArgKind::Type(ty) => {
                    matches!(ty.kind(), TyKind::Param(parameter) if parameter.index as usize == index)
                },
                GenericArgKind::Lifetime(region) => matches!(
                    &region.kind,
                    RegionKind::ReEarlyParam(parameter) if parameter.index as usize == index
                ),
                GenericArgKind::Const(_) => false,
            });
        if copy && general {
            impls.copy.push(definition);
        }
    }
    impls
}

/// What dropping a value of an ADT reaches through its fields.
#[derive(Clone, Default, PartialEq)]
struct DropReach {
    /// A `Drop` impl of the ADT, or a field whose destruction the unit cannot
    /// see (`dyn`, a projection).
    own: bool,
    /// That, or the same of an ADT its fields reach.
    destructor: bool,
    /// The positions of the type parameters its fields hold: `S<T>(T)` reaches
    /// `T`, `S<T>(PhantomData<T>)` and `S<T>(&T)` do not.
    parameters: BTreeSet<usize>,
}

/// The crates' impls, read once per crate, and the reach of the ADTs seen.
#[derive(Default)]
struct AdtImpls {
    crates: BTreeMap<String, CrateImpls>,
    reach: HashMap<AdtDef, DropReach>,
}

impl AdtImpls {
    fn of_crate(&mut self, krate: Crate) -> &CrateImpls {
        self.crates
            .entry(krate.name.clone())
            .or_insert_with(|| crate_impls(&krate))
    }

    /// Grows the reach of `adts` and of what their fields name to a fixed
    /// point. Field types stay generic, so the ADTs are finitely many even for
    /// a type recursive at growing instantiations, and a reach only grows.
    fn settle_drop_reach(&mut self, adts: &[ReachedAdt]) {
        loop {
            let mut changed = false;
            let mut pending = adts.iter().map(|adt| adt.definition).collect::<Vec<_>>();
            let mut visited = HashSet::new();
            while let Some(definition) = pending.pop() {
                if !visited.insert(definition) {
                    continue;
                }
                let reach = self.fields_reach(definition, &mut pending);
                if self.reach.get(&definition) != Some(&reach) {
                    self.reach.insert(definition, reach);
                    changed = true;
                }
            }
            if !changed {
                return;
            }
        }
    }

    fn fields_reach(&mut self, definition: AdtDef, named: &mut Vec<AdtDef>) -> DropReach {
        let mut reach = DropReach::default();
        if self.of_crate(definition.krate()).drop.contains(&definition) {
            reach.own = true;
            reach.destructor = true;
            return reach;
        }
        for variant in definition.variants() {
            for field in variant.fields() {
                self.type_reach(field.ty(), &mut reach, named);
            }
        }
        reach
    }

    fn type_reach(&self, ty: RustTy, reach: &mut DropReach, named: &mut Vec<AdtDef>) {
        match ty.kind() {
            TyKind::Param(parameter) => {
                reach.parameters.insert(parameter.index as usize);
            },
            TyKind::Alias(..) | TyKind::Bound(..) => {
                reach.own = true;
                reach.destructor = true;
            },
            TyKind::RigidTy(rigid) => match rigid {
                RigidTy::Adt(definition, arguments) => {
                    named.push(definition);
                    let Some(inner) = self.reach.get(&definition) else {
                        return;
                    };
                    reach.destructor |= inner.destructor;
                    for parameter in &inner.parameters {
                        if let Some(GenericArgKind::Type(argument)) = arguments.0.get(*parameter) {
                            self.type_reach(*argument, reach, named);
                        }
                    }
                },
                RigidTy::Tuple(elements) => {
                    for element in elements {
                        self.type_reach(element, reach, named);
                    }
                },
                RigidTy::Array(element, _) | RigidTy::Slice(element) | RigidTy::Pat(element, _) => {
                    self.type_reach(element, reach, named)
                },
                RigidTy::Bool
                | RigidTy::Char
                | RigidTy::Int(_)
                | RigidTy::Uint(_)
                | RigidTy::Float(_)
                | RigidTy::Str
                | RigidTy::Never
                | RigidTy::Foreign(_)
                | RigidTy::RawPtr(..)
                | RigidTy::Ref(..)
                | RigidTy::FnDef(..)
                | RigidTy::FnPtr(_) => {},
                // A closure holds its upvars.
                RigidTy::Closure(_, arguments) => {
                    if let Ok((_, upvars)) = closure_arguments(&arguments) {
                        for upvar in upvars {
                            self.type_reach(upvar, reach, named);
                        }
                    }
                },
                RigidTy::Coroutine(..)
                | RigidTy::CoroutineClosure(..)
                | RigidTy::CoroutineWitness(..)
                | RigidTy::Dynamic(..) => {
                    reach.own = true;
                    reach.destructor = true;
                },
            },
        }
    }

    /// Whether the unit states a destructor of an ADT. A nominal with visible
    /// fields states its own: what its fields reach, whoever drops it reads from
    /// their types, a model of the library among them. An opaque one shows
    /// neither its fields nor its type parameters, so what it reaches and what
    /// it holds of its arguments count as a destructor.
    fn has_destructor(&self, adt: &ReachedAdt) -> bool {
        self.reach.get(&adt.definition).is_some_and(|reach| {
            reach.own || (adt.opaque && (reach.destructor || !reach.parameters.is_empty()))
        })
    }

    /// Whether a type is `Copy` wherever its type arguments are, as a closure
    /// over it is.
    fn is_copy(&mut self, ty: RustTy) -> bool {
        self.copy_requirement(ty).is_some()
    }

    /// The type parameters a type is `Copy` given, none where it never is.
    fn copy_requirement(&mut self, ty: RustTy) -> Option<BTreeSet<u32>> {
        let all = |impls: &mut Self, types: Vec<RustTy>| {
            types
                .into_iter()
                .try_fold(BTreeSet::new(), |mut parameters, ty| {
                    parameters.extend(impls.copy_requirement(ty)?);
                    Some(parameters)
                })
        };
        match ty.kind() {
            TyKind::Param(parameter) => Some(BTreeSet::from([parameter.index])),
            TyKind::RigidTy(rigid) => match rigid {
                RigidTy::Bool
                | RigidTy::Char
                | RigidTy::Int(_)
                | RigidTy::Uint(_)
                | RigidTy::Float(_)
                | RigidTy::Never
                | RigidTy::RawPtr(..)
                | RigidTy::FnDef(..)
                | RigidTy::FnPtr(_) => Some(BTreeSet::new()),
                RigidTy::Ref(_, _, Mutability::Not) => Some(BTreeSet::new()),
                RigidTy::Ref(_, _, Mutability::Mut) => None,
                RigidTy::Tuple(elements) => all(self, elements),
                RigidTy::Array(element, _) => all(self, vec![element]),
                RigidTy::Adt(definition, arguments) => {
                    if self.of_crate(definition.krate()).copy.contains(&definition) {
                        all(self, type_arguments(&arguments))
                    } else {
                        None
                    }
                },
                RigidTy::Closure(_, arguments) => {
                    let (_, upvars) = closure_arguments(&arguments).ok()?;
                    all(self, upvars)
                },
                RigidTy::Str
                | RigidTy::Slice(_)
                | RigidTy::Pat(..)
                | RigidTy::Foreign(_)
                | RigidTy::Dynamic(..)
                | RigidTy::Coroutine(..)
                | RigidTy::CoroutineClosure(..)
                | RigidTy::CoroutineWitness(..) => None,
            },
            TyKind::Alias(..) | TyKind::Bound(..) => None,
        }
    }
}

fn is_abort(definition: FnDef) -> bool {
    definition
        .as_intrinsic()
        .is_some_and(|intrinsic| intrinsic.fn_name() == "abort")
        || polyfill::is_process_abort(definition)
}

/// Whether the mapper lowers a call to a VIR operation (an abort or a
/// primitive) rather than mapping it as a call.
fn lowered_callee(definition: FnDef, arguments: &GenericArgs) -> bool {
    let name = definition.name();
    arguments.0.is_empty()
        && (is_abort(definition)
            || is_string_length(&name)
            || integer_method_primitive(&name).is_some())
}

/// What the bodies' calls reach.
struct ReachedCalls {
    /// The dependency functions, each once, ordered by name.
    functions: Vec<DependencyFunction>,
    /// Whether a call of a trait method resolves to an impl's.
    resolves_trait_calls: bool,
    /// The types the calls instantiate generic parameters with.
    instantiations: Vec<RustTy>,
}

/// The files of the crate's own items, used to recognize local macro call sites.
fn crate_files() -> HashSet<String> {
    let local = rustc_public::local_crate();
    let (items, adts, traits, impls) = (
        rustc_public::all_local_items(),
        local.adts(),
        local.trait_decls(),
        local.trait_impls(),
    );
    items
        .iter()
        .map(CrateDef::span)
        .chain(adts.iter().map(CrateDef::span))
        .chain(traits.iter().map(CrateDef::span))
        .chain(impls.iter().map(CrateDef::span))
        .map(|span| polyfill::source_callsite(span).get_filename())
        .collect()
}

/// `call of f in g at line L` of a call whose callee is `definition`.
fn call_site(definition: FnDef, caller: &str, span: Span, crate_files: &HashSet<String>) -> String {
    let name = definition.name();
    let span = polyfill::source_callsite(span);
    if !crate_files.contains(&span.get_filename()) {
        return format!("call of `{name}` in `{caller}` in a macro expansion");
    }
    let line = span.get_lines();
    format!(
        "call of `{name}` in `{caller}` at line {}:{}",
        line.start_line, line.start_col
    )
}

/// The dependency functions the bodies call and what else their calls reach. A
/// call whose callee cannot be declared is reported here, at the call.
fn collect_calls(items: &[CrateItem], bodies: &[PreparedBody]) -> Result<ReachedCalls, String> {
    let mut reached = ReachedCalls {
        functions: vec![],
        resolves_trait_calls: false,
        instantiations: vec![],
    };
    for (item, body) in items.iter().zip(bodies) {
        let locals = &body.mapped_locals;
        let folds = &body.box_folds;
        for (index, block) in body.blocks.iter().enumerate() {
            if folds.frees.contains(&index) {
                continue;
            }
            let TerminatorKind::Call { func, .. } = &block.terminator.kind else {
                continue;
            };
            let Ok(callee_type) = func.ty(locals) else {
                continue;
            };
            let TyKind::RigidTy(RigidTy::FnDef(definition, arguments)) = callee_type.kind() else {
                continue;
            };
            if lowered_callee(definition, &arguments) {
                continue;
            }
            // A call that resolves to nothing the unit states refuses its function.
            let Ok((resolved, resolved_arguments)) = resolve_callee(item.0, definition, &arguments)
            else {
                continue;
            };
            reached.resolves_trait_calls |= resolved != definition;
            if let Some((_, closure_args)) = closure_callee(item.0, definition, &arguments) {
                let (parents, _) = closure_arguments(&closure_args)?;
                reached
                    .instantiations
                    .extend(type_arguments(&GenericArgs(parents)));
                continue;
            }
            if !resolved.krate().is_local
                && !reached
                    .functions
                    .iter()
                    .any(|function| function.definition == resolved)
            {
                reached.functions.push(DependencyFunction::new(resolved));
            }
            let opaque = reached
                .functions
                .iter()
                .any(|function| function.definition == resolved && function.opaque);
            if !opaque {
                reached
                    .instantiations
                    .extend(type_arguments(&resolved_arguments));
            }
        }
    }
    reached
        .functions
        .sort_by_key(|function| function.definition.name());
    Ok(reached)
}

/// A MIR constant, evaluated at export.
#[derive(Clone)]
enum EvaluatedConstant {
    /// A scalar, a string, or unit.
    Literal { ty: RustTy, value: ConstValue },
    Tuple {
        ty: RustTy,
        elements: Vec<EvaluatedConstant>,
    },
    /// A fixed array, or a slice behind a reference.
    Array {
        ty: RustTy,
        elements: Vec<EvaluatedConstant>,
    },
    Adt {
        ty: RustTy,
        definition: AdtDef,
        variant: VariantIdx,
        arguments: GenericArgs,
        fields: Vec<EvaluatedConstant>,
    },
    /// A shared reference, which is the value it observes.
    Shared {
        ty: RustTy,
        referent: Box<EvaluatedConstant>,
    },
    /// A zero-sized closure: its upvars, if any, are such closures.
    Closure {
        ty: RustTy,
        definition: ClosureDef,
        arguments: GenericArgs,
        upvars: Vec<EvaluatedConstant>,
    },
    /// A constant the unit cannot state, at its type if the unit can state that.
    Opaque {
        ty: Option<RustTy>,
        description: String,
    },
}

impl EvaluatedConstant {
    /// The types it is built of, and the ADTs it states the shape of.
    fn collect(&self, types: &mut Vec<RustTy>, adts: &mut Vec<AdtDef>) {
        let parts = match self {
            Self::Literal { ty, .. } => {
                types.push(*ty);
                return;
            },
            Self::Opaque { ty, .. } => {
                types.extend(ty);
                return;
            },
            Self::Tuple { ty, elements } | Self::Array { ty, elements } => {
                types.push(*ty);
                elements.as_slice()
            },
            Self::Adt {
                ty,
                definition,
                fields,
                ..
            } => {
                types.push(*ty);
                if !adts.contains(definition) {
                    adts.push(*definition);
                }
                fields.as_slice()
            },
            Self::Shared { ty, referent } => {
                types.push(*ty);
                std::slice::from_ref(referent.as_ref())
            },
            Self::Closure { ty, upvars, .. } => {
                types.push(*ty);
                upvars.as_slice()
            },
        };
        for part in parts {
            part.collect(types, adts);
        }
    }

    /// The constant, or, if a type it needs has no mapping or a shape it states is
    /// not visible, the opaque constant at its type. `observed` holds the ADTs the
    /// bodies observe.
    fn or_opaque(self, ty: RustTy, observed: &[AdtDef]) -> Self {
        let (mut types, mut adts) = (vec![], vec![]);
        self.collect(&mut types, &mut adts);
        let mappable = adts
            .iter()
            .all(|definition| has_visible_shape(*definition, observed))
            && types
                .iter()
                .all(|ty| collect_type_adts(*ty, &adts, &mut Vec::new()).is_ok());
        if mappable {
            self
        } else {
            Self::opaque(ty)
        }
    }

    fn opaque(ty: RustTy) -> Self {
        let mappable = collect_type_adts(ty, &[], &mut Vec::new()).is_ok();
        Self::Opaque {
            ty: mappable.then_some(ty),
            description: format!("the constant of type `{ty}` has no mapping in the unit"),
        }
    }
}

/// Whether all of an ADT's variants and fields are visible to the crate, so that a
/// constant may state its shape. Rustc Public exposes no field's visibility, so a
/// dependency's struct is known to be visible only without fields, or where rustc
/// admitted a body observing it.
fn has_visible_shape(definition: AdtDef, observed: &[AdtDef]) -> bool {
    definition.krate().is_local
        || definition.kind() == AdtKind::Enum
        || observed.contains(&definition)
        || definition
            .variants()
            .iter()
            .all(|variant| variant.fields().is_empty())
}

enum ConstantFailure {
    /// The constant has no statement in the unit, so it is opaque.
    Unmappable,
    /// The constant refuses its function, for this reason.
    Rejected(String),
}

/// The deepest chain of references a constant is followed through; a static
/// may refer to itself.
const MAX_CONSTANT_DEPTH: usize = 64;

fn empty_allocation() -> Allocation {
    Allocation {
        bytes: vec![],
        provenance: ProvenanceMap { ptrs: vec![] },
        align: 1,
        mutability: Mutability::Not,
    }
}

fn read_bytes(memory: &Allocation, offset: usize, size: usize) -> Result<u128, ConstantFailure> {
    memory
        .read_partial_uint(offset..offset + size)
        .map_err(|_| ConstantFailure::Unmappable)
}

fn layout_shape(ty: RustTy) -> Result<LayoutShape, ConstantFailure> {
    ty.layout()
        .map(|layout| layout.shape())
        .map_err(|_| ConstantFailure::Unmappable)
}

/// The size of an enum's tag.
fn scalar_size(scalar: &Scalar) -> usize {
    let (Scalar::Initialized { value, .. } | Scalar::Union { value }) = scalar;
    value.size(&MachineInfo::target()).bytes()
}

/// Whether a pointer is stored at an offset of memory, whose bytes are then its
/// offset into its allocation rather than its address.
fn pointer_at(memory: &Allocation, offset: usize) -> Option<GlobalAlloc> {
    memory
        .provenance
        .ptrs
        .iter()
        .find(|(at, _)| *at == offset)
        .map(|(_, provenance)| GlobalAlloc::from(provenance.0))
}

/// The variant a value holds and the offsets of its fields, from its layout.
fn read_variant(
    definition: AdtDef,
    shape: &LayoutShape,
    memory: &Allocation,
    offset: usize,
) -> Result<(VariantIdx, Vec<usize>), ConstantFailure> {
    let offsets = match &shape.fields {
        FieldsShape::Primitive => vec![],
        FieldsShape::Arbitrary { offsets } => offsets.iter().map(|offset| offset.bytes()).collect(),
        FieldsShape::Union(_) | FieldsShape::Array { .. } => {
            return Err(ConstantFailure::Unmappable);
        },
    };
    let (tag, tag_encoding, tag_field, variants) = match &shape.variants {
        VariantsShape::Single { index } => return Ok((*index, offsets)),
        VariantsShape::Empty => return Err(ConstantFailure::Unmappable),
        VariantsShape::Multiple {
            tag,
            tag_encoding,
            tag_field,
            variants,
        } => (tag, tag_encoding, *tag_field, variants),
    };
    let tag_offset = offset + offsets.get(tag_field).ok_or(ConstantFailure::Unmappable)?;
    let size = scalar_size(tag);
    let mask = if size >= 16 {
        u128::MAX
    } else {
        (1_u128 << (size * 8)) - 1
    };
    let declared = definition.variants();
    let position = |index: VariantIdx| declared.iter().position(|variant| variant.idx() == index);
    let selected = match tag_encoding {
        TagEncoding::Direct => {
            let value = read_bytes(memory, tag_offset, size)?;
            declared
                .iter()
                .find(|variant| {
                    definition.discriminant_for_variant(variant.idx()).val & mask == value
                })
                .map(|variant| variant.idx())
                .ok_or(ConstantFailure::Unmappable)?
        },
        TagEncoding::Niche {
            untagged_variant,
            niche_variants,
            niche_start,
        } => {
            // A pointer in the niche is not null, so the niche holds no variant.
            if pointer_at(memory, tag_offset).is_some() {
                *untagged_variant
            } else {
                let value = read_bytes(memory, tag_offset, size)?;
                let relative = value.wrapping_sub(*niche_start) & mask;
                let start = position(*niche_variants.start()).ok_or(ConstantFailure::Unmappable)?;
                let end = position(*niche_variants.end()).ok_or(ConstantFailure::Unmappable)?;
                if relative <= (end - start) as u128 {
                    declared[start + relative as usize].idx()
                } else {
                    *untagged_variant
                }
            }
        },
    };
    let fields = position(selected)
        .and_then(|index| variants.get(index))
        .ok_or(ConstantFailure::Unmappable)?;
    Ok((
        selected,
        fields.offsets.iter().map(|offset| offset.bytes()).collect(),
    ))
}

/// The value of a type stored at an offset of evaluated memory, driven by the
/// type's layout.
fn read_value(
    ty: RustTy,
    memory: &Allocation,
    offset: usize,
    depth: usize,
) -> Result<EvaluatedConstant, ConstantFailure> {
    let literal = |value| Ok(EvaluatedConstant::Literal { ty, value });
    let TyKind::RigidTy(rigid) = ty.kind() else {
        return Err(ConstantFailure::Unmappable);
    };
    match rigid {
        RigidTy::Bool => match read_bytes(memory, offset, 1)? {
            0 => literal(ConstValue::Bool { value: false }),
            1 => literal(ConstValue::Bool { value: true }),
            _ => Err(ConstantFailure::Unmappable),
        },
        RigidTy::Char => literal(ConstValue::Character {
            value: character_value(read_bytes(memory, offset, 4)?)
                .map_err(|_| ConstantFailure::Unmappable)?,
        }),
        RigidTy::Uint(_) | RigidTy::Int(_) => {
            let size = layout_shape(ty)?.size.bytes();
            literal(ConstValue::Integer {
                value: integer_value(read_bytes(memory, offset, size)?, ty)
                    .map_err(|_| ConstantFailure::Unmappable)?,
            })
        },
        RigidTy::Tuple(elements) if elements.is_empty() => literal(ConstValue::Unit),
        RigidTy::Tuple(elements) => {
            let FieldsShape::Arbitrary { offsets } = layout_shape(ty)?.fields else {
                return Err(ConstantFailure::Unmappable);
            };
            let elements = elements
                .into_iter()
                .zip(offsets)
                .map(|(element, at)| read_value(element, memory, offset + at.bytes(), depth))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(EvaluatedConstant::Tuple { ty, elements })
        },
        RigidTy::Array(element, _) => {
            let FieldsShape::Array { stride, count } = layout_shape(ty)?.fields else {
                return Err(ConstantFailure::Unmappable);
            };
            let elements = read_elements(
                element,
                stride.bytes(),
                count as usize,
                memory,
                offset,
                depth,
            )?;
            Ok(EvaluatedConstant::Array { ty, elements })
        },
        RigidTy::Adt(definition, arguments) => {
            if definition.kind() == AdtKind::Union {
                return Err(ConstantFailure::Unmappable);
            }
            let shape = layout_shape(ty)?;
            let (variant, offsets) = read_variant(definition, &shape, memory, offset)?;
            let declared = definition
                .variants()
                .into_iter()
                .find(|declared| declared.idx() == variant)
                .ok_or(ConstantFailure::Unmappable)?
                .fields();
            if declared.len() != offsets.len() {
                return Err(ConstantFailure::Unmappable);
            }
            let fields = declared
                .iter()
                .zip(offsets)
                .map(|(field, at)| {
                    read_value(field.ty_with_args(&arguments), memory, offset + at, depth)
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(EvaluatedConstant::Adt {
                ty,
                definition,
                variant,
                arguments,
                fields,
            })
        },
        RigidTy::Closure(definition, arguments) if is_zero_sized_closure(&arguments) => {
            let (_, upvars) =
                closure_arguments(&arguments).map_err(|_| ConstantFailure::Unmappable)?;
            let upvars = upvars
                .into_iter()
                .map(|upvar| read_value(upvar, memory, offset, depth))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(EvaluatedConstant::Closure {
                ty,
                definition,
                arguments,
                upvars,
            })
        },
        RigidTy::Ref(_, referent, Mutability::Not) => {
            let referent = read_referent(referent, memory, offset, depth)?;
            Ok(EvaluatedConstant::Shared {
                ty,
                referent: Box::new(referent),
            })
        },
        _ => Err(ConstantFailure::Unmappable),
    }
}

fn read_elements(
    element: RustTy,
    stride: usize,
    count: usize,
    memory: &Allocation,
    offset: usize,
    depth: usize,
) -> Result<Vec<EvaluatedConstant>, ConstantFailure> {
    (0..count)
        .map(|index| read_value(element, memory, offset + index * stride, depth))
        .collect()
}

/// The value a reference stored at an offset of memory refers to.
fn read_referent(
    referent: RustTy,
    memory: &Allocation,
    offset: usize,
    depth: usize,
) -> Result<EvaluatedConstant, ConstantFailure> {
    if depth >= MAX_CONSTANT_DEPTH {
        return Err(ConstantFailure::Unmappable);
    }
    let pointer_size = MachineInfo::target_pointer_width().bytes();
    let address = read_bytes(memory, offset, pointer_size)? as usize;
    let (target, start) = match pointer_at(memory, offset) {
        // A pointer without provenance refers to zero-sized memory.
        None => (empty_allocation(), 0),
        Some(GlobalAlloc::Memory(target)) => (target, address),
        Some(GlobalAlloc::Static(definition)) => {
            let target = definition
                .eval_initializer()
                .map_err(|_| ConstantFailure::Unmappable)?;
            if target.mutability == Mutability::Mut {
                return Err(ConstantFailure::Rejected(format!(
                    "reads the mutable static `{}`, which has no value at export",
                    definition.name()
                )));
            }
            (target, address)
        },
        Some(_) => return Err(ConstantFailure::Unmappable),
    };
    let length = || read_bytes(memory, offset + pointer_size, pointer_size).map(|len| len as usize);
    match referent.kind() {
        TyKind::RigidTy(RigidTy::Str) => {
            let bytes = target
                .bytes
                .get(start..start + length()?)
                .ok_or(ConstantFailure::Unmappable)?
                .iter()
                .copied()
                .collect::<Option<Vec<_>>>()
                .ok_or(ConstantFailure::Unmappable)?;
            let value = String::from_utf8(bytes).map_err(|_| ConstantFailure::Unmappable)?;
            Ok(EvaluatedConstant::Literal {
                ty: referent,
                value: ConstValue::String { value },
            })
        },
        TyKind::RigidTy(RigidTy::Slice(element)) => {
            let stride = layout_shape(element)?.size.bytes();
            let elements = read_elements(element, stride, length()?, &target, start, depth + 1)?;
            Ok(EvaluatedConstant::Array {
                ty: referent,
                elements,
            })
        },
        _ => read_value(referent, &target, start, depth + 1),
    }
}

/// Evaluate constants, using the confined bridge for promoted MIR. Constants
/// depending on type or const parameters remain refused.
fn evaluate_constant(constant: &ConstOperand) -> Result<EvaluatedConstant, ConstantFailure> {
    let ty = constant.ty();
    let generic = |what: String| {
        ConstantFailure::Rejected(format!(
            "{what}, which has no value before monomorphization"
        ))
    };
    // A zero-sized closure of a generic function has its one value at any arguments.
    if mentions_parameter(ty) && !is_zero_sized_closure_type(ty) {
        return Err(generic(format!("has the generic type `{ty}`")));
    }
    match constant.const_.kind() {
        ConstantKind::Allocated(allocation) => read_value(ty, allocation, 0, 0),
        ConstantKind::ZeroSized => read_value(ty, &empty_allocation(), 0, 0),
        ConstantKind::Param(parameter) => Err(generic(format!(
            "is the const parameter `{}`",
            parameter.name
        ))),
        ConstantKind::Unevaluated(UnevaluatedConst {
            promoted: Some(_), ..
        }) => {
            let evaluated = polyfill::eval_promoted_const(&constant.const_, constant.span)
                .map_err(ConstantFailure::Rejected)?;
            match evaluated.kind() {
                ConstantKind::Allocated(allocation) => read_value(ty, allocation, 0, 0),
                ConstantKind::ZeroSized => read_value(ty, &empty_allocation(), 0, 0),
                _ => Err(ConstantFailure::Unmappable),
            }
        },
        ConstantKind::Unevaluated(UnevaluatedConst { def, .. }) => {
            let instance = Instance::try_from(CrateItem(def.0))
                .map_err(|_| generic(format!("is the generic definition `{}`", def.name())))?;
            let allocation = instance
                .try_const_eval(ty)
                .map_err(|_| ConstantFailure::Unmappable)?;
            read_value(ty, &allocation, 0, 0)
        },
        ConstantKind::Ty(value) => match value.kind() {
            TyConstKind::Value(_, allocation) => read_value(ty, allocation, 0, 0),
            TyConstKind::ZSTValue(_) => read_value(ty, &empty_allocation(), 0, 0),
            kind => Err(generic(format!("is the generic constant {kind:?}"))),
        },
    }
}

/// The constants the bodies use, each evaluated, and what they reach: the types
/// a type of a local need not name (`v.get(0)` with no other `usize` in the
/// body) and the ADTs whose shape they state.
struct EvaluatedConstants {
    values: HashMap<MirConst, EvaluatedConstant>,
    /// The one value of each zero-sized type a body reads unassigned. Optimized MIR
    /// assigns no zero-sized place, so a body returns without assigning its return
    /// place and borrows a closure or a unit struct in a local nothing assigned.
    zero_sized_values: HashMap<RustTy, EvaluatedConstant>,
    types: Vec<RustTy>,
    adts: Vec<AdtDef>,
}

/// Evaluates the constants of a body. A constant of a function-item type is the
/// callee of a call or the source of a reification, which map it themselves.
struct ConstantEvaluator<'a> {
    observed: &'a [AdtDef],
    constants: &'a mut EvaluatedConstants,
}

impl MirVisitor for ConstantEvaluator<'_> {
    fn visit_const_operand(&mut self, constant: &ConstOperand, _location: MirLocation) {
        let ty = constant.ty();
        if matches!(ty.kind(), TyKind::RigidTy(RigidTy::FnDef(..)))
            || self.constants.values.contains_key(&constant.const_)
        {
            return;
        }
        let evaluated = match evaluate_constant(constant) {
            Ok(evaluated) => evaluated.or_opaque(ty, self.observed),
            Err(ConstantFailure::Unmappable) => EvaluatedConstant::opaque(ty),
            Err(ConstantFailure::Rejected(reason)) => {
                let EvaluatedConstant::Opaque { ty: mapped, .. } = EvaluatedConstant::opaque(ty)
                else {
                    unreachable!("`opaque` builds an opaque constant");
                };
                EvaluatedConstant::Opaque {
                    ty: mapped,
                    description: format!("the constant of type `{ty}` {reason}"),
                }
            },
        };
        evaluated.collect(&mut self.constants.types, &mut self.constants.adts);
        self.constants
            .values
            .insert(constant.const_.clone(), evaluated);
    }

    // Debug info names constants the code does not use.
    fn visit_var_debug_info(&mut self, _var_debug_info: &VarDebugInfo) {}
}

impl EvaluatedConstants {
    fn new(bodies: &[PreparedBody], observed: &[AdtDef]) -> Self {
        let mut constants = Self {
            values: HashMap::new(),
            zero_sized_values: HashMap::new(),
            types: vec![],
            adts: vec![],
        };
        for body in bodies {
            let mut evaluator = ConstantEvaluator {
                observed,
                constants: &mut constants,
            };
            evaluator.visit_body(body);
            constants.add_zero_sized_value(body.ret_local().ty, observed);
            for local in unassigned_locals(body) {
                constants.add_zero_sized_value(body.locals()[local].ty, observed);
            }
        }
        constants
    }

    fn add_zero_sized_value(&mut self, result: RustTy, observed: &[AdtDef]) {
        // A type naming a parameter has no layout: rustc does not answer.
        if self.zero_sized_values.contains_key(&result) || mentions_parameter(result) {
            return;
        }
        let zero_sized = is_zero_sized_closure_type(result)
            || layout_shape(result).is_ok_and(|shape| shape.is_sized() && shape.size.bytes() == 0);
        if !zero_sized {
            return;
        }
        let Ok(value) = read_value(result, &empty_allocation(), 0, 0) else {
            return;
        };
        let value = value.or_opaque(result, observed);
        if !matches!(value, EvaluatedConstant::Opaque { .. }) {
            value.collect(&mut self.types, &mut self.adts);
            self.zero_sized_values.insert(result, value);
        }
    }
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
        let MappedBodies {
            items,
            bodies,
            refusals,
        } = mapped_bodies(
            rustc_public::all_local_items()
                .into_iter()
                .filter(|item| item.kind() == ItemKind::Fn)
                .collect(),
        )?;
        if items.is_empty() {
            return Err("the Rust mapper requires at least one local function".to_owned());
        }
        for (item, refusal) in items.iter().zip(&refusals) {
            if refusal.as_ref().is_some_and(Refusal::is_signature) {
                continue;
            }
            let generics = item_generics(*item)?;
            // A closure's own parameters are its kind, signature and upvars,
            // which its arguments state.
            if closure_of(*item).is_none()
                && item.requires_monomorphization()
                && generics.is_empty()
            {
                return Err(format!(
                    "function `{}` requires monomorphization without public generic binders",
                    item.name()
                ));
            }
        }
        let identities = items.iter().map(CrateDef::name).collect::<Vec<_>>();
        let opaque_functions = identities
            .iter()
            .zip(&refusals)
            .filter(|(_, refusal)| refusal.as_ref().is_some_and(Refusal::is_signature))
            .map(|(identity, _)| identity.clone())
            .collect::<BTreeSet<_>>();
        let crate_files = crate_files();
        let ReachedCalls {
            functions: dependency_functions,
            resolves_trait_calls,
            mut instantiations,
        } = collect_calls(&items, &bodies)?;
        let method_evidence = items
            .iter()
            .zip(&bodies)
            .map(|(item, body)| {
                let mut evidence = declared_method_evidence(item.0);
                let mut names = body
                    .var_debug_info
                    .iter()
                    .map(|info| info.name.clone())
                    .collect::<BTreeSet<_>>();
                for entry in &mut evidence {
                    let base = entry.name.clone();
                    let mut suffix = 0;
                    while !names.insert(entry.name.clone()) {
                        suffix += 1;
                        entry.name = format!("{base}_{suffix}");
                    }
                }
                (item.name(), evidence)
            })
            .chain(dependency_functions.iter().map(|function| {
                // A dependency declaration has no body to clash with, so its
                // evidence keeps the name its trait gives it. Its predicates
                // come from the same polyfill lookup as an item's.
                (
                    function.definition.name(),
                    declared_method_evidence(function.definition.0),
                )
            }))
            .collect::<BTreeMap<_, _>>();
        let mut dictionary_types = method_evidence
            .values()
            .flatten()
            .cloned()
            .collect::<Vec<_>>();
        let mut shared_adapters = BTreeSet::new();
        let mut scoped_evidence = BTreeSet::new();
        let mut evidence_flows = Vec::new();
        for (item_index, (item, body)) in items.iter().zip(&bodies).enumerate() {
            let locals = &body.mapped_locals;
            for block in &body.blocks {
                let TerminatorKind::Call { func, .. } = &block.terminator.kind else {
                    continue;
                };
                let Ok(callee) = func.ty(locals) else {
                    continue;
                };
                let TyKind::RigidTy(RigidTy::FnDef(definition, arguments)) = callee.kind() else {
                    continue;
                };
                let Ok((resolved, arguments)) = resolve_callee(item.0, definition, &arguments)
                else {
                    continue;
                };
                if let Some(evidence) = method_evidence.get(&resolved.name()) {
                    for (required_index, declared) in evidence.iter().enumerate() {
                        let required = declared.instantiate(item.0, &arguments);
                        // Match evidence_for's choice: exact dictionaries take
                        // precedence over borrowed receiver adapters.
                        let available = &method_evidence[&item.name()];
                        let exact = available.iter().position(|entry| {
                            entry.method == required.method && entry.arguments == required.arguments
                        });
                        let supplying = exact.or_else(|| {
                            available
                                .iter()
                                .position(|entry| entry.forwards_to(&required))
                        });
                        if let Some(supplying) = supplying {
                            if exact.is_none() && required.receiver_kind == EvidenceReceiver::Owned
                            {
                                scoped_evidence.insert((item_index, supplying));
                            } else if required.tupled
                                && required.receiver_kind != EvidenceReceiver::Owned
                            {
                                let callee_index = identities
                                    .iter()
                                    .position(|name| *name == resolved.name())
                                    .ok_or("local method evidence has no item")?;
                                evidence_flows.push((
                                    (callee_index, required_index),
                                    (item_index, supplying),
                                ));
                            }
                        }
                        if required.tupled {
                            for (index, available) in
                                method_evidence[&item.name()].iter().enumerate()
                            {
                                if available.forwards_to(&required) {
                                    shared_adapters.insert((
                                        item_index,
                                        index,
                                        required.receiver_kind == EvidenceReceiver::Owned,
                                    ));
                                }
                            }
                        }
                        dictionary_types.push(required);
                    }
                }
            }
        }
        // A scoped callee requirement follows the dictionary supplied by its
        // caller, including Fn-to-FnMut forwarding. Unrelated borrowed bounds
        // retain the existing, unrestricted method-evidence interface.
        loop {
            let mut changed = false;
            for (required, supplying) in &evidence_flows {
                if scoped_evidence.contains(required) {
                    changed |= scoped_evidence.insert(*supplying);
                }
            }
            if !changed {
                break;
            }
        }
        // A dictionary of a concrete call is a type the unit reaches. One a
        // declaration states generically may name an associated type, which
        // its binder carries: those go the way of a declared signature.
        for ty in evidence_types(&dictionary_types) {
            if projection(ty).is_none() {
                instantiations.push(ty);
            }
        }
        let impls = AdtImpls::default();
        let copy_parameters = copy_parameters(&items);
        let observed = observed_adts(&bodies);
        let constants = EvaluatedConstants::new(&bodies, &observed);
        let signature_types = dependency_functions
            .iter()
            .flat_map(|function| function.signature.iter().chain(&function.evidence).copied())
            .collect::<Vec<_>>();
        let reached = collect_adts(
            &bodies,
            observed,
            &signature_types,
            &instantiations,
            &constants,
        )?;
        let namespaces = CrateNamespaces::new(
            reached.iter().map(|adt| adt.definition.krate()).chain(
                dependency_functions
                    .iter()
                    .map(|function| function.definition.krate()),
            ),
        )?;
        let mut scopes = Scopes::new();
        let item_scopes = items
            .iter()
            .map(|item| match closure_of(*item) {
                Some((closure, _)) => Ok(scopes.closure_scope(closure).2),
                None => scopes.function_scope(function_def(*item)?),
            })
            .collect::<Result<Vec<_>, String>>()?;
        let item_names = items
            .iter()
            .zip(&bodies)
            .map(|(item, body)| match closure_of(*item) {
                Some(_) => closure_body_name(item, body),
                None => Ok(last_segment(&item.name())),
            })
            .collect::<Result<Vec<_>, String>>()?;
        let (mut names, adts) = build_names(
            &item_names,
            &item_scopes,
            &reached,
            &mut scopes,
            &namespaces,
        )?;
        let closures = build_closures(
            &items,
            &bodies,
            &item_names,
            &item_scopes,
            &mut names,
            &mut scopes,
        )?;
        let mut name_ids = identities
            .iter()
            .enumerate()
            .map(|(index, identity)| {
                (identity.clone(), QualifiedRef {
                    name: NameId::new(index),
                    namespace_id: NamespaceId::new(0),
                })
            })
            .collect::<BTreeMap<_, _>>();
        let mut fnmut_adapters = Vec::new();
        for (index, item) in items.iter().enumerate() {
            let Some((closure, _)) = closure_of(*item) else {
                continue;
            };
            if item_names[index] != "call" || !dictionary_types.iter().any(|e| e.tupled && e.receiver_kind == EvidenceReceiver::Mutable &&
                matches!(e.receiver.kind(), TyKind::RigidTy(RigidTy::Closure(def, _)) if def == closure)) {
                continue;
            }
            let name = NameId::new(names.len());
            let mut adapter_name = format!("__fnmut_adapter_{}", item_scopes[index].join("::"));
            while names
                .iter()
                .any(|name| name.scope.is_empty() && name.name == adapter_name)
            {
                adapter_name.push('_');
            }
            names.push(QualifiedName {
                name: adapter_name,
                scope: vec![],
                namespace_id: NamespaceId::new(0),
            });
            let target = QualifiedRef {
                name,
                namespace_id: NamespaceId::new(0),
            };
            name_ids.insert(format!("{}::__fnmut_adapter", item.name()), target.clone());
            fnmut_adapters.push((index, target));
        }
        let mut fnonce_adapters = Vec::new();
        for (index, item) in items.iter().enumerate() {
            let Some((closure, _)) = closure_of(*item) else {
                continue;
            };
            if item_names[index] == "call_once" || !dictionary_types.iter().any(|e|
                e.tupled && e.receiver_kind == EvidenceReceiver::Owned &&
                matches!(e.receiver.kind(), TyKind::RigidTy(RigidTy::Closure(def, _)) if def == closure)) {
                continue;
            }
            let name = NameId::new(names.len());
            let mut adapter_name = format!("__fnonce_adapter_{}", item_scopes[index].join("::"));
            while names
                .iter()
                .any(|name| name.scope.is_empty() && name.name == adapter_name)
            {
                adapter_name.push('_');
            }
            names.push(QualifiedName {
                name: adapter_name,
                scope: vec![],
                namespace_id: NamespaceId::new(0),
            });
            let target = QualifiedRef {
                name,
                namespace_id: NamespaceId::new(0),
            };
            name_ids.insert(format!("{}::__fnonce_adapter", item.name()), target.clone());
            fnonce_adapters.push((index, target));
        }
        let mut evidence_adapters = Vec::new();
        for (item_index, evidence_index, consuming) in shared_adapters {
            let prefix = if consuming {
                "__borrowed_to_fnonce"
            } else {
                "__fn_to_fnmut"
            };
            let name = NameId::new(names.len());
            let mut adapter_name = format!(
                "{prefix}_{}_{evidence_index}",
                item_scopes[item_index]
                    .iter()
                    .chain(std::iter::once(&item_names[item_index]))
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("::")
            );
            while names
                .iter()
                .any(|name| name.scope.is_empty() && name.name == adapter_name)
            {
                adapter_name.push('_');
            }
            names.push(QualifiedName {
                name: adapter_name,
                scope: vec![],
                namespace_id: NamespaceId::new(0),
            });
            let target = QualifiedRef {
                name,
                namespace_id: NamespaceId::new(0),
            };
            name_ids.insert(
                format!("{}::{prefix}_{evidence_index}", items[item_index].name()),
                target.clone(),
            );
            evidence_adapters.push((item_index, evidence_index, consuming, target));
        }
        for function in &dependency_functions {
            let namespace_id = namespaces.id(&function.definition.krate().name)?;
            let name = NameId::new(names.len());
            names.push(QualifiedName {
                name: last_segment(&function.definition.name()),
                namespace_id,
                scope: scopes.function_scope(function.definition)?,
            });
            name_ids.insert(function.definition.name(), QualifiedRef {
                name,
                namespace_id,
            });
        }
        let signatures = items
            .iter()
            .zip(&bodies)
            .zip(&refusals)
            .map(|((item, body), refusal)| {
                if refusal.as_ref().is_some_and(Refusal::is_signature) {
                    return Ok(DeclaredSignature {
                        types: vec![RustTy::from_rigid_kind(RigidTy::Never)],
                        bound_lifetimes: vec![],
                        first_binder: 0,
                    });
                }
                if closure_of(*item).is_some() {
                    return Ok(DeclaredSignature::of_closure(
                        body,
                        item_generics(*item)?.len(),
                    ));
                }
                let function = function_def(*item)?;
                let binders = flattened_generics(function)?.len();
                Ok(DeclaredSignature::new(function, binders).at_locals(body))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let bound_lifetimes = identities
            .iter()
            .zip(&signatures)
            .map(|(identity, signature)| (identity.clone(), signature.bound_lifetimes.len()))
            .chain(dependency_functions.iter().map(|function| {
                (
                    function.definition.name(),
                    function.declared.bound_lifetimes.len(),
                )
            }))
            .collect();
        let mut types = TypeLayout::from_bodies(
            &items,
            &bodies,
            &adts,
            &closures,
            &dependency_functions,
            &signatures,
            &instantiations,
            &constants,
        )?;
        types.intern_method_evidence(&dictionary_types)?;
        Ok(Self {
            items,
            bodies,
            refusals,
            opaque_functions,
            identities,
            name_ids,
            dependency_functions,
            constants,
            copy_parameters,
            impls,
            signatures,
            method_evidence,
            bound_lifetimes,
            resolves_trait_calls,
            names,
            adts,
            closures,
            fnmut_adapters,
            fnonce_adapters,
            evidence_adapters,
            scoped_evidence,
            types,
            scopes,
            namespaces,
            sources: SourceTables {
                crate_files,
                ..SourceTables::default()
            },
            arenas: NamespaceArenas::default(),
        })
    }

    fn map(mut self) -> Result<RawUnit, String> {
        let mut functions = Vec::with_capacity(self.items.len());
        let mut origins = Vec::with_capacity(self.items.len());
        let mut alignments = Vec::with_capacity(self.items.len());
        let mut expression_ranges = Vec::new();
        for index in 0..self.items.len() {
            let expression_start = self.arenas.expressions.len();
            let item = self.items[index];
            let identity = self.identities[index].clone();
            let body = self.bodies[index].clone();
            let function_loc = self.sources.intern(item.span())?;
            let refusal = self.refusals[index].clone();
            let opaque = refusal.as_ref().is_some_and(Refusal::is_signature);
            let parameters = if opaque { vec![] } else { item_generics(item)? };
            let mut generics = parameters
                .into_iter()
                .map(|parameter| {
                    generic_binder(
                        parameter,
                        &self.copy_parameters[index],
                        function_loc,
                        &mut self.types,
                    )
                })
                .collect::<Result<Vec<_>, String>>()?;
            generics.extend(bound_lifetime_binders(
                &self.signatures[index],
                function_loc,
            ));
            let mut mapper = FunctionMapper::new(
                &identity,
                item,
                &body,
                &self.signatures[index],
                &self.types,
                &self.name_ids,
                &self.bound_lifetimes,
                Callees {
                    dependencies: &self.dependency_functions,
                    opaque: &self.opaque_functions,
                    method_evidence: &self.method_evidence,
                },
                &self.constants,
                &self.adts,
                &self.closures,
                CopyFacts {
                    parameters: &self.copy_parameters[index],
                    impls: &mut self.impls,
                },
                &mut self.sources,
                &mut self.arenas,
            )?;
            if let Some(refusal) = &refusal {
                mapper.body_failure = Some(refusal.reason().to_owned());
            }
            let mapped = mapper.map()?;
            expression_ranges.push(expression_start..self.arenas.expressions.len());
            let evidence_count = self.method_evidence.get(&identity).map_or(0, Vec::len);
            let mut evidence_attributes: Vec<_> = (0..evidence_count)
                .map(|index| Attribute::Assign {
                    loc: Some(function_loc),
                    name: "method_evidence".to_owned(),
                    value: AttributeValue::Constant {
                        value: ConstValue::Integer {
                            value: Number::from(mapped.parameters.len() - evidence_count + index),
                        },
                    },
                })
                .collect();
            // Only forwarding paths that consume a borrowed callable need the
            // receiver-scoping refinement; ordinary Fn/FnMut remain unchanged.
            for evidence_index in 0..evidence_count {
                if self.scoped_evidence.contains(&(index, evidence_index)) {
                    evidence_attributes.push(Attribute::Assign {
                        loc: Some(function_loc),
                        name: "scoped_method_evidence".to_owned(),
                        value: AttributeValue::Constant {
                            value: ConstValue::Integer {
                                value: Number::from(
                                    mapped.parameters.len() - evidence_count + evidence_index,
                                ),
                            },
                        },
                    });
                }
            }
            functions.push(FunctionDecl {
                alignment: AlignmentId::new(index),
                attributes: evidence_attributes,
                body: mapped.body,
                contract: empty_contract(),
                doc: String::new(),
                loc: function_loc,
                locals: mapped.locals,
                name: NameId::new(index),
                origin: OriginId::new(index),
                pragmas: vec![],
                profile: Profile::Rust,
                profile_data: opaque
                    .then(|| ProfileValue {
                        payload: String::new(),
                        profile: Profile::Rust,
                        tag: "function.opaque".to_owned(),
                    })
                    .into_iter()
                    .collect(),
                signature: Signature {
                    generics,
                    parameters: mapped.parameters,
                    predicates: if opaque {
                        vec![]
                    } else {
                        signature_lifetime_predicates(
                            item.0,
                            &self.signatures[index],
                            &mut self.types,
                        )
                    },
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

        // Receiver conversions are checked bodies, not relabeled function
        // types. Consuming adapters borrow the owned closure, invoke its actual
        // body, and explicitly drop the receiver after the borrow ends.
        let adapters =
            self.fnmut_adapters
                .iter()
                .map(|(index, target)| (*index, None, false, target))
                .chain(
                    self.fnonce_adapters
                        .iter()
                        .map(|(index, target)| (*index, None, true, target)),
                )
                .chain(self.evidence_adapters.iter().map(
                    |(index, evidence, consuming, target)| {
                        (*index, Some(*evidence), *consuming, target)
                    },
                ));
        for (index, evidence_index, consuming, target) in adapters {
            let expression_start = self.arenas.expressions.len();
            let mut original = functions[index].clone();
            let loc = original.loc;
            if let Some(evidence_index) = evidence_index {
                let evidence = &self.method_evidence[&self.identities[index]][evidence_index];
                let Ty::Function {
                    arguments, result, ..
                } = self.types.evidence_function(evidence)?
                else {
                    return Err("method evidence is not a function".to_owned());
                };
                original.signature.parameters = arguments
                    .into_iter()
                    .enumerate()
                    .map(|(index, type_id)| Parameter {
                        mutable: false,
                        name: if index == 0 {
                            "receiver".to_owned()
                        } else {
                            format!("arg_{index}")
                        },
                        type_use: TypeUse { loc, type_id },
                    })
                    .collect();
                let dictionary_index = original.signature.parameters.len();
                original.signature.parameters.push(Parameter {
                    mutable: false,
                    name: evidence.name.clone(),
                    type_use: TypeUse {
                        loc,
                        type_id: self.types.method_evidence_type(evidence)?,
                    },
                });
                original.signature.results = vec![TypeUse {
                    loc,
                    type_id: result,
                }];
                original.attributes = vec![Attribute::Assign {
                    name: "method_evidence".to_owned(),
                    value: AttributeValue::Constant {
                        value: ConstValue::Integer {
                            value: Number::from(dictionary_index),
                        },
                    },
                    loc: None,
                }];
                if consuming || self.scoped_evidence.contains(&(index, evidence_index)) {
                    // A borrowed dictionary cannot retain its receiver, which a
                    // consuming adapter drops after the invocation.
                    original.attributes.push(Attribute::Assign {
                        name: "scoped_method_evidence".to_owned(),
                        value: AttributeValue::Constant {
                            value: ConstValue::Integer {
                                value: Number::from(dictionary_index),
                            },
                        },
                        loc: None,
                    });
                }
                // Source verification synthesizes and proves the forwarding
                // contract for this checked body; this is not a trusted axiom.
                original.attributes.push(Attribute::Assign {
                    loc: None,
                    name: if consuming {
                        "consuming_method_adapter"
                    } else {
                        "shared_method_adapter"
                    }
                    .to_owned(),
                    value: AttributeValue::Constant {
                        value: ConstValue::Integer {
                            value: Number::from(dictionary_index),
                        },
                    },
                });
            }
            if evidence_index.is_none() {
                let name = &self.names[original.name.index];
                let target = name
                    .scope
                    .iter()
                    .chain(std::iter::once(&name.name))
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("::");
                original.attributes.push(Attribute::Assign {
                    loc: None,
                    name: "closure_method_adapter".to_owned(),
                    value: AttributeValue::Constant {
                        value: ConstValue::String { value: target },
                    },
                });
            }
            let shared_type = original.signature.parameters[0].type_use.type_id;
            let Ty::Reference { value: shared } = self.types.types[shared_type.index].clone()
            else {
                return Err("Fn closure receiver is not a reference".to_owned());
            };
            let mutable = Ty::Reference {
                value: ReferenceType {
                    kind: ReferenceKind::Mutable,
                    ..shared
                },
            };
            let mutable_type =
                if let Some(index) = self.types.types.iter().position(|ty| *ty == mutable) {
                    TypeId::new(index)
                } else {
                    let id = TypeId::new(self.types.types.len());
                    self.types.types.push(mutable);
                    id
                };
            let mut signature = original.signature.clone();
            signature.parameters[0].type_use.type_id = if consuming {
                shared.referent
            } else {
                mutable_type
            };
            signature.parameters[0].mutable = consuming && shared.kind == ReferenceKind::Mutable;
            let mut locals = signature
                .parameters
                .iter()
                .enumerate()
                .map(|(index, p)| LocalDecl {
                    id: LocalId::new(index),
                    loc,
                    mutable: p.mutable,
                    name: p.name.clone(),
                    type_use: p.type_use.clone(),
                })
                .collect::<Vec<_>>();
            let receiver = self.arenas.place(LocalId::new(0));
            let deref = if consuming {
                receiver
            } else {
                let deref = PlaceId::new(self.arenas.places.len());
                self.arenas.places.push(Place::Deref { base: receiver });
                deref
            };
            let receiver = self
                .arenas
                .expression(loc, shared_type, ExprKind::Operation {
                    operation: Operation::Borrow {
                        kind: if consuming && shared.kind == ReferenceKind::Mutable {
                            BorrowKind::Mutable
                        } else {
                            BorrowKind::Immutable
                        },
                        place: deref,
                    },
                    arguments: vec![],
                    instantiations: vec![],
                    surface: None,
                });
            let mut arguments = vec![receiver];
            let parameter_count =
                signature.parameters.len() - usize::from(evidence_index.is_some());
            for (index, parameter) in signature
                .parameters
                .iter()
                .enumerate()
                .take(parameter_count)
                .skip(1)
            {
                let place = self.arenas.place(LocalId::new(index));
                arguments.push(self.arenas.expression(
                    loc,
                    parameter.type_use.type_id,
                    ExprKind::Operation {
                        operation: Operation::Move { place },
                        arguments: vec![],
                        instantiations: vec![],
                        surface: None,
                    },
                ));
            }
            let (kind, instantiations) = if evidence_index.is_some() {
                let parameter = &signature.parameters[parameter_count];
                let place = self.arenas.place(LocalId::new(parameter_count));
                let dictionary =
                    self.arenas
                        .expression(loc, parameter.type_use.type_id, ExprKind::Operation {
                            operation: Operation::Copy { place },
                            arguments: vec![],
                            instantiations: vec![],
                            surface: None,
                        });
                arguments.insert(0, dictionary);
                (CallKind::Invoke, vec![])
            } else {
                let (_, closure_args) =
                    closure_of(self.items[index]).ok_or("missing Fn closure")?;
                let mut instantiations = self
                    .types
                    .generic_arguments(&GenericArgs(closure_arguments(&closure_args)?.0), loc)?;
                let bound_count = self.signatures[index].bound_lifetimes.len();
                if bound_count > 0 {
                    let value = self
                        .types
                        .interned_lifetime(&erased())
                        .ok_or("missing adapter lifetime")?;
                    instantiations.extend(vec![GenericArgument::Lifetime { value }; bound_count]);
                }
                (
                    CallKind::Function {
                        callee: QualifiedRef {
                            name: original.name,
                            namespace_id: NamespaceId::new(0),
                        },
                    },
                    instantiations,
                )
            };
            let result_type = signature.results[0].clone();
            let call = self
                .arenas
                .expression(loc, result_type.type_id, ExprKind::Operation {
                    operation: Operation::Call { kind },
                    arguments,
                    instantiations,
                    surface: None,
                });
            let result_id = LocalId::new(locals.len());
            locals.push(LocalDecl {
                id: result_id,
                loc,
                mutable: true,
                name: "result".to_owned(),
                type_use: result_type.clone(),
            });
            let result_place = self.arenas.place(result_id);
            let result = self
                .arenas
                .expression(loc, result_type.type_id, ExprKind::Operation {
                    operation: Operation::Move {
                        place: result_place,
                    },
                    arguments: vec![],
                    instantiations: vec![],
                    surface: None,
                });
            // Unit has no observable payload. Returning its literal avoids
            // retaining a dead call-result temporary across the receiver drop.
            let result = if consuming && result_type.type_id == self.types.unit {
                self.arenas
                    .expression(loc, result_type.type_id, ExprKind::Value {
                        source_constant: None,
                        value: ConstValue::Unit,
                    })
            } else {
                result
            };
            let mut blocks = vec![
                RawBasicBlock {
                    loc,
                    statements: vec![],
                    terminator: RawTerminator::Call {
                        call,
                        destination: Some(RawCallDestination {
                            place: result_place,
                            target: BlockId::new(1),
                        }),
                        unwind: RawUnwindAction::Unreachable,
                    },
                },
                RawBasicBlock {
                    loc,
                    statements: vec![],
                    terminator: RawTerminator::Return {
                        values: vec![result],
                    },
                },
            ];
            if consuming {
                blocks.insert(1, RawBasicBlock {
                    loc,
                    statements: vec![],
                    terminator: RawTerminator::Drop {
                        place: self.arenas.place(LocalId::new(0)),
                        target: BlockId::new(2),
                        unwind: RawUnwindAction::Unreachable,
                    },
                });
            }
            functions.push(FunctionDecl {
                name: target.name,
                signature,
                locals,
                contract: empty_contract(),
                body: RawBody::Cfg {
                    graph: RawCfg {
                        entry: BlockId::new(0),
                        blocks,
                    },
                },
                ..original
            });
            expression_ranges.push(expression_start..self.arenas.expressions.len());
        }

        self.types.complete_invocation_types(
            &functions,
            &self.arenas.expressions,
            &expression_ranges,
        )?;

        let namespace_loc = functions[0].loc;
        let mut structs = Vec::new();
        // A dependency crate is a namespace of the unit holding the
        // declarations the crate reaches; it is not a verification target.
        let mut dependencies = Vec::with_capacity(self.namespaces.dependencies.len());
        for (name, (_, namespace_id)) in &self.namespaces.dependencies {
            dependencies.push(RawNamespace {
                associated_items: vec![],
                attributes: vec![],
                comments: vec![],
                constants: vec![],
                doc: String::new(),
                expressions: vec![],
                functions: vec![],
                identity: *namespace_id,
                implementations: vec![],
                imports: vec![],
                intrinsics: vec![],
                invariants: vec![],
                lemmas: vec![],
                loc: self.sources.dependency(name),
                patterns: vec![],
                places: vec![],
                pragmas: vec![verify_false()],
                profile: Some(Profile::Rust),
                profile_metadata: vec![],
                scopes: vec![],
                spec_functions: vec![],
                spec_vars: vec![],
                structs: vec![],
                traits: vec![],
            });
        }
        for (declaration, adt) in map_structs(&self.adts, &mut self.types, &mut self.sources)?
            .into_iter()
            .zip(&self.adts)
        {
            match adt.namespace_id.index {
                0 => structs.push(declaration),
                index => dependencies[index - 1].structs.push(declaration),
            }
        }
        structs.extend(map_closures(
            &self.closures,
            &self.adts,
            &mut self.types,
            &mut self.sources,
        )?);
        // A called dependency function is declared by its generic signature;
        // its body is not exported.
        for function in &self.dependency_functions {
            let identity = function.definition.name();
            let callee = self.name_ids[&identity].clone();
            let loc = self.sources.dependency(&identity);
            let origin = OriginId::new(origins.len());
            origins.push(Origin {
                description: "Rustc Public FnDef::fn_sig".to_owned(),
                kind: OriginKind::RustMir,
                location: loc,
                source_identity: Some(identity.clone()),
            });
            let alignment = AlignmentId::new(alignments.len());
            alignments.push(Alignment {
                description: if function.opaque {
                    "Rustc Public callee the unit cannot state".to_owned()
                } else {
                    "Rustc Public generic signature without its body".to_owned()
                },
                source: origin,
                trust: Trust::Checked,
            });
            let parameters = if function.opaque {
                vec![]
            } else {
                flattened_generics(function.definition)?
            };
            let mut generics = parameters
                .into_iter()
                .map(|parameter| {
                    generic_binder(
                        parameter,
                        &polyfill::copy_parameters(function.definition.0)
                            .into_iter()
                            .collect(),
                        loc,
                        &mut self.types,
                    )
                })
                .collect::<Result<Vec<_>, String>>()?;
            generics.extend(
                function
                    .associated_types
                    .iter()
                    .map(|associated| GenericBinder {
                        abilities: vec![],
                        kind: BinderKind::TypeArg,
                        loc,
                        name: associated.name.clone(),
                        predicates: vec![],
                        type_: None,
                    }),
            );
            generics.extend(bound_lifetime_binders(&function.declared, loc));
            let (output, inputs) = function
                .signature
                .split_last()
                .ok_or_else(|| format!("dependency function `{identity}` has no result type"))?;
            let parameters = inputs
                .iter()
                .enumerate()
                .map(|(index, ty)| {
                    Ok(Parameter {
                        mutable: false,
                        name: format!("_{}", index + 1),
                        type_use: TypeUse {
                            loc,
                            type_id: self.types.signature_type_id(
                                *ty,
                                &function.associated_types,
                                &self.adts,
                            )?,
                        },
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            // A declared `Fn*` bound is a dictionary the caller supplies, as
            // it is for an item with a body: the evidence follows the real
            // arguments, so a contract can name its behaviour.
            let mut parameters = parameters;
            for evidence in self
                .method_evidence
                .get(&function.definition.name())
                .map(Vec::as_slice)
                .unwrap_or_default()
            {
                parameters.push(Parameter {
                    mutable: false,
                    name: evidence.name.clone(),
                    type_use: TypeUse {
                        loc,
                        type_id: self.types.signature_evidence_type(
                            evidence,
                            &function.associated_types,
                            &self.adts,
                        )?,
                    },
                });
            }
            let locals = parameters
                .iter()
                .enumerate()
                .map(|(index, parameter)| LocalDecl {
                    id: LocalId::new(index),
                    loc,
                    mutable: false,
                    name: parameter.name.clone(),
                    type_use: parameter.type_use.clone(),
                })
                .collect();
            dependencies[callee.namespace_id.index - 1]
                .functions
                .push(FunctionDecl {
                    alignment,
                    attributes: vec![],
                    body: RawBody::Absent,
                    contract: empty_contract(),
                    doc: String::new(),
                    loc,
                    locals,
                    name: callee.name,
                    origin,
                    // A function's pragmas merge its namespace's.
                    pragmas: vec![verify_false()],
                    profile: Profile::Rust,
                    profile_data: std::iter::once("function.dependency")
                        .chain(function.opaque.then_some("function.opaque"))
                        .chain(
                            (!function.associated_types.is_empty()).then_some("function.shapeOnly"),
                        )
                        .map(|tag| ProfileValue {
                            payload: String::new(),
                            profile: Profile::Rust,
                            tag: tag.to_owned(),
                        })
                        .collect(),
                    signature: Signature {
                        generics,
                        parameters,
                        predicates: if function.opaque {
                            vec![]
                        } else {
                            signature_lifetime_predicates(
                                function.definition.0,
                                &function.declared,
                                &mut self.types,
                            )
                        },
                        results: vec![TypeUse {
                            loc,
                            type_id: self.types.signature_type_id(
                                *output,
                                &function.associated_types,
                                &self.adts,
                            )?,
                        }],
                    },
                });
        }
        let mut scopes = Vec::new();
        for ((krate, path), (kind, span)) in std::mem::take(&mut self.scopes.declarations) {
            let namespace_id = self.namespaces.id(&krate)?;
            let scope = ScopeDecl {
                kind,
                loc: self.sources.declaration(
                    namespace_id,
                    &format!("{krate}::{}", path.join("::")),
                    span,
                )?,
                path,
            };
            match namespace_id.index {
                0 => scopes.push(scope),
                index => dependencies[index - 1].scopes.push(scope),
            }
        }
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
        let mut evidence = vec![ImportEvidence {
            description: format!(
                "rustc accepted this crate through analysis and exposed optimized generic MIR for a {target_pointer_width}-bit pointer target"
            ),
            producer: polyfill::PRODUCER.to_owned(),
            trusted: true,
        }];
        if self.resolves_trait_calls {
            evidence.push(ImportEvidence {
                description: "calls of trait methods on concrete types resolve to their impls' methods as rustc's codegen resolves them".to_owned(),
                producer: "verex-rust-export via rustc Public Instance::resolve".to_owned(),
                trusted: true,
            });
        }
        let mut namespace_refs = vec![NamespaceRef {
            segments: vec![local_crate.name],
        }];
        namespace_refs.extend(
            self.namespaces
                .dependencies
                .keys()
                .map(|name| NamespaceRef {
                    segments: vec![name.clone()],
                }),
        );
        let mut namespaces = vec![RawNamespace {
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
            lemmas: vec![],
            loc: namespace_loc,
            patterns: vec![],
            places: self.arenas.places,
            pragmas: vec![],
            profile: Some(Profile::Rust),
            profile_metadata: vec![],
            scopes,
            spec_functions: vec![],
            spec_vars: vec![],
            structs,
            traits: vec![],
        }];
        namespaces.extend(dependencies);
        Ok(RawUnit {
            dependencies: vec![],
            evidence,
            namespaces,
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
                version: 5,
            }],
            tables: Tables {
                alignments,
                files: self.sources.files,
                lifetimes,
                locations: self.sources.locations,
                names: self.names,
                namespaces: namespace_refs,
                origins,
                types: self.types.types,
            },
            version: Version { major: 1, minor: 9 },
        })
    }
}

struct AdtNames {
    definition: AdtDef,
    name: NameId,
    namespace_id: NamespaceId,
    /// Whether it is `Copy` wherever its type arguments are.
    copy: bool,
    /// Whether dropping it runs a destructor whatever its type arguments are.
    destructor: bool,
    /// A dependency's ADT the unit names without its shape; it has no
    /// variants here.
    opaque: bool,
    variants: Vec<VariantNames>,
}

/// A closure of the crate: a struct of its upvars, whose fields are named by the
/// body's debug info, and the body in the struct's scope.
struct ClosureNames {
    definition: ClosureDef,
    item: CrateItem,
    name: NameId,
    /// Each upvar's field name and its type, regions erased.
    fields: Vec<(NameId, RustTy)>,
    /// One binder per region of the upvar types, after the parent's generics.
    lifetimes: Vec<String>,
    copy: bool,
    /// The definition of the body rustc nests it in, and that body's path in the
    /// namespace: the rendering spells the closure there.
    parent: DefId,
    parent_path: Vec<String>,
    zero_sized: bool,
    /// Its generic arguments in its parent's body.
    arguments: GenericArgs,
}

/// Declared Copy bounds, including those inherited from an enclosing impl.
fn copy_parameters(items: &[CrateItem]) -> Vec<BTreeSet<u32>> {
    items
        .iter()
        .map(|item| polyfill::copy_parameters(item.0).into_iter().collect())
        .collect()
}

/// The closures among the items.
fn build_closures(
    items: &[CrateItem],
    bodies: &[PreparedBody],
    item_names: &[String],
    item_scopes: &[Vec<String>],
    names: &mut Vec<QualifiedName>,
    scopes: &mut Scopes,
) -> Result<Vec<ClosureNames>, String> {
    let mut closures = Vec::new();
    let mut impls = AdtImpls::default();
    for (item, body) in items.iter().zip(bodies) {
        let Some((definition, arguments)) = closure_of(*item) else {
            continue;
        };
        let (closure_name, module, _) = scopes.closure_scope(definition);
        let name = NameId::new(names.len());
        names.push(QualifiedName {
            name: closure_name,
            namespace_id: NamespaceId::new(0),
            scope: module.clone(),
        });
        let (_, upvars) = closure_arguments(&arguments)?;
        let copy = upvars.iter().all(|ty| impls.is_copy(*ty));
        let generics = item_generics(*item)?;
        let region_count = upvars.iter().map(|upvar| regions_in(*upvar)).sum::<usize>();
        let lifetimes = (0..)
            .map(|suffix| format!("'u{suffix}"))
            .filter(|candidate| {
                !generics
                    .iter()
                    .any(|parameter| parameter.name == *candidate)
            })
            .take(region_count)
            .collect();
        let mut fields = Vec::with_capacity(upvars.len());
        for (index, upvar) in upvars.into_iter().enumerate() {
            let field = NameId::new(names.len());
            names.push(QualifiedName {
                name: upvar_name(body, index),
                namespace_id: NamespaceId::new(0),
                scope: module.clone(),
            });
            fields.push((field, upvar));
        }
        let parent = definition
            .def_id()
            .parent()
            .ok_or_else(|| format!("closure `{}` has no parent", item.name()))?;
        let parent_index = items
            .iter()
            .position(|candidate| candidate.0 == parent)
            .ok_or_else(|| {
                format!(
                    "closure `{}` stands in `{}`, which is no function of the unit",
                    item.name(),
                    parent.name()
                )
            })?;
        let mut parent_path = item_scopes[parent_index].clone();
        parent_path.push(item_names[parent_index].clone());
        closures.push(ClosureNames {
            definition,
            item: *item,
            name,
            fields,
            lifetimes,
            copy,
            parent,
            parent_path,
            zero_sized: is_zero_sized_closure(&arguments),
            arguments,
        });
    }
    Ok(closures)
}

/// An upvar's source name: the debug info of the closure's body names each by a
/// place through its first parameter, `(*_1).i` or `_1.i`.
fn upvar_name(body: &Body, index: usize) -> String {
    body.var_debug_info
        .iter()
        .find_map(|info| {
            let VarDebugInfoContents::Place(place) = &info.value else {
                return None;
            };
            let selected = place
                .projection
                .iter()
                .find_map(|projection| match projection {
                    ProjectionElem::Field(field, _) => Some(*field),
                    _ => None,
                })?;
            (place.local == 1 && selected == index).then(|| info.name.clone())
        })
        .unwrap_or_else(|| index.to_string())
}

/// The name of a closure's body, by how it takes the closure: `call` by shared
/// reference, `call_mut` by mutable one, `call_once` by value.
fn closure_body_name(item: &CrateItem, body: &Body) -> Result<String, String> {
    let receiver = body
        .arg_locals()
        .first()
        .ok_or_else(|| format!("closure `{}` takes no receiver", item.name()))?
        .ty;
    Ok(match receiver.kind() {
        TyKind::RigidTy(RigidTy::Ref(_, _, Mutability::Not)) => "call",
        TyKind::RigidTy(RigidTy::Ref(_, _, Mutability::Mut)) => "call_mut",
        _ => "call_once",
    }
    .to_owned())
}

fn map_closures(
    closures: &[ClosureNames],
    adts: &[AdtNames],
    types: &mut TypeLayout,
    sources: &mut SourceTables,
) -> Result<Vec<StructDecl>, String> {
    closures
        .iter()
        .map(|closure| {
            let loc = sources.intern(closure.definition.span())?;
            let mut generics = item_generics(closure.item)?
                .into_iter()
                .map(|parameter| {
                    generic_binder(
                        parameter,
                        &polyfill::copy_parameters(closure.item.0)
                            .into_iter()
                            .collect(),
                        loc,
                        types,
                    )
                })
                .collect::<Result<Vec<_>, String>>()?;
            let parents = generics.len();
            let mut binders = closure.lifetimes.iter().enumerate();
            let mut next_region = || {
                let (index, name) = binders.next().expect("one binder per region");
                Region {
                    kind: RegionKind::ReEarlyParam(EarlyParamRegion {
                        index: (parents + index) as u32,
                        name: name.clone(),
                    }),
                }
            };
            generics.extend(closure.lifetimes.iter().map(|name| GenericBinder {
                abilities: vec![],
                kind: BinderKind::Lifetime,
                loc,
                name: name.clone(),
                predicates: vec![],
                type_: None,
            }));
            let fields = closure
                .fields
                .iter()
                .map(|(name, ty)| {
                    Ok(FieldDecl {
                        doc: String::new(),
                        loc,
                        name: *name,
                        type_use: TypeUse {
                            loc,
                            type_id: types.rebuilt_type_id(
                                *ty,
                                &[],
                                adts,
                                &mut |_| next_region(),
                                &|ty| regions_in(ty) == 0,
                            )?,
                        },
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok(StructDecl {
                abilities: if closure.copy {
                    vec![Ability::Copy]
                } else {
                    vec![]
                },
                attributes: vec![],
                contract: empty_contract(),
                doc: String::new(),
                fields,
                generics,
                ghosts: vec![],
                loc,
                locals: vec![],
                name: closure.name,
                properties: vec![ProfileValue {
                    payload: serde_json::to_string(&closure.parent_path)
                        .map_err(|error| format!("encode a closure's parent: {error}"))?,
                    profile: Profile::Rust,
                    tag: "struct.closure".to_owned(),
                }],
                variants: vec![],
            })
        })
        .collect()
}

struct VariantNames {
    index: VariantIdx,
    ordinal: usize,
    name: Option<NameId>,
    source_name: Option<String>,
    fields: Vec<NameId>,
}

/// The closures a body names by type, and the closures it constructs.
#[derive(Default)]
struct ClosureUses {
    mentioned: Vec<ClosureDef>,
    constructed: Vec<ClosureDef>,
}

impl ClosureUses {
    /// A zero-sized closure's value holds the zero-sized closures it captures.
    fn construct_zero_sized(&mut self, closure: ClosureDef, arguments: &GenericArgs) {
        self.constructed.push(closure);
        if let Ok((_, upvars)) = closure_arguments(arguments) {
            for upvar in upvars {
                if let TyKind::RigidTy(RigidTy::Closure(nested, nested_args)) = upvar.kind() {
                    self.construct_zero_sized(nested, &nested_args);
                }
            }
        }
    }

    fn observe(&mut self, ty: RustTy) {
        if let TyKind::RigidTy(RigidTy::Closure(closure, _)) = ty.kind() {
            self.mentioned.push(closure);
        }
        for child in type_children(ty) {
            self.observe(child);
        }
    }
}

impl MirVisitor for ClosureUses {
    fn visit_ty(&mut self, ty: &RustTy, _location: MirLocation) {
        self.observe(*ty);
    }

    fn visit_const_operand(&mut self, constant: &ConstOperand, _location: MirLocation) {
        self.observe(constant.ty());
        // A zero-sized closure is a constant of its type.
        if let TyKind::RigidTy(RigidTy::Closure(closure, arguments)) = constant.ty().kind() {
            self.construct_zero_sized(closure, &arguments);
        }
    }

    fn visit_rvalue(&mut self, rvalue: &Rvalue, location: MirLocation) {
        if let Rvalue::Aggregate(AggregateKind::Closure(closure, _), _) = rvalue {
            self.mentioned.push(*closure);
            self.constructed.push(*closure);
        }
        self.super_rvalue(rvalue, location);
    }

    fn visit_var_debug_info(&mut self, _var_debug_info: &VarDebugInfo) {}
}

/// The locals a body reads without assigning: rustc removes the assignments of
/// zero-sized values.
fn unassigned_locals(body: &Body) -> Vec<usize> {
    let assigned = |index: usize| {
        body.blocks.iter().any(|block| {
            block.statements.iter().any(|statement| {
                matches!(&statement.kind, StatementKind::Assign(place, _) if place.local == index)
            }) || matches!(&block.terminator.kind,
                TerminatorKind::Call { destination, .. } if destination.local == index)
        })
    };
    let arguments = body.arg_locals().len();
    (0..body.locals().len())
        .filter(|index| *index == 0 || *index > arguments)
        .filter(|index| {
            let read = if *index == 0 {
                body.blocks
                    .iter()
                    .any(|block| matches!(block.terminator.kind, TerminatorKind::Return))
            } else {
                let mut mentions = LocalMentions {
                    local: *index,
                    mentions: 0,
                };
                mentions.visit_body(body);
                mentions.mentions > storage_mentions(body, *index)
            };
            read && !assigned(*index)
        })
        .collect()
}

/// The functions and the closures they reach, with the closures those stand in: a
/// closure no body names is dead code. A sized closure is constructed in its parent, so
/// one no reached body constructs has no expression; a zero-sized one is a value of its
/// type wherever it is used, and its parent spells it. A refused item's body reaches
/// nothing, and a body holding a dependency's closure is refused.
fn reached_items(
    items: &[CrateItem],
    refusals: &mut HashMap<DefId, Refusal>,
) -> Result<Vec<CrateItem>, String> {
    let (closures, mut reached): (Vec<_>, Vec<_>) = items
        .iter()
        .copied()
        .partition(|item| closure_of(*item).is_some());
    let mut constructed: HashSet<DefId> = HashSet::new();
    let mut scanned = 0;
    while let Some(item) = reached.get(scanned).copied() {
        scanned += 1;
        if refusals.contains_key(&item.0) {
            continue;
        }
        let Some(body) = item.body() else { continue };
        let mut uses = ClosureUses::default();
        uses.visit_body(&body);
        constructed.extend(uses.constructed.iter().map(CrateDef::def_id));
        for closure in uses.mentioned {
            if !closure.krate().is_local {
                refusals.insert(
                    item.0,
                    Refusal::Body(format!(
                        "it holds closure `{}` of the dependency crate `{}`, whose body the unit does not export",
                        closure.name(),
                        closure.krate().name
                    )),
                );
                break;
            }
            let mut ancestor = Some(closure.def_id());
            while let Some(found) =
                ancestor.and_then(|def| closures.iter().find(|candidate| candidate.0 == def))
            {
                if !reached.contains(found) {
                    reached.push(*found);
                }
                ancestor = found.0.parent();
            }
        }
    }
    for item in &reached {
        if let Some((_, arguments)) = closure_of(*item)
            && !is_zero_sized_closure(&arguments)
            && !constructed.contains(&item.0)
        {
            return Err(format!(
                "closure `{}` is held but no reached body constructs it",
                item.name()
            ));
        }
    }
    Ok(reached)
}

/// Immutable analysis shared by signature discovery, type collection and mapping.
/// A changed/refused MIR body receives a fresh analysis in `mapped_bodies`.
#[derive(Clone)]
struct PreparedBody {
    body: Body,
    mapped_locals: Vec<MirLocalDecl>,
    box_folds: BoxFolds,
}

impl PreparedBody {
    fn new(body: Body) -> Self {
        let box_folds = box_folds(&body);
        let mapped_locals = mapped_locals(&body, &box_folds);
        Self {
            body,
            mapped_locals,
            box_folds,
        }
    }
}

impl std::ops::Deref for PreparedBody {
    type Target = Body;

    fn deref(&self) -> &Body {
        &self.body
    }
}

/// The items the unit maps, by name, with the bodies mapped for them.
struct MappedBodies {
    items: Vec<CrateItem>,
    bodies: Vec<PreparedBody>,
    refusals: Vec<Option<Refusal>>,
}

/// Why an item is not mapped as it stands.
#[derive(Clone)]
enum Refusal {
    /// Its body states what the unit cannot: its signature stands, for a contract.
    Body(String),
    /// Its signature does: the function is opaque, and a call of it refuses the caller.
    Signature(String),
}

impl Refusal {
    fn reason(&self) -> &str {
        match self {
            Self::Body(reason) | Self::Signature(reason) => reason,
        }
    }

    fn is_signature(&self) -> bool {
        matches!(self, Self::Signature(_))
    }
}

/// The reached items and their bodies, each refusal made before anything reads a body.
/// Nothing else reads what is inside a body. A signature that cannot be mapped leaves
/// nothing a caller can read, so the function is opaque and the rest of the crate stands.
/// Refusing can only unreach items.
fn mapped_bodies(candidates: Vec<CrateItem>) -> Result<MappedBodies, String> {
    let mut refusals = HashMap::new();
    loop {
        let known = refusals.len();
        let mut changed = false;
        let mut items = reached_items(&candidates, &mut refusals)?;
        items.sort_by_key(CrateDef::name);
        let bodies = items
            .iter()
            .map(|item| {
                let body = item
                    .body()
                    .ok_or_else(|| format!("function `{}` has no public MIR body", item.name()))?;
                Ok(PreparedBody::new(match refusals.get(&item.0) {
                    Some(Refusal::Signature(_)) => opaque_body(&body),
                    Some(Refusal::Body(_)) => refused_body(&body),
                    None => body,
                }))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let observed = observed_adts(&bodies);
        for (item, body) in items.iter().zip(&bodies) {
            if refusals.get(&item.0).is_some_and(Refusal::is_signature) {
                continue;
            }
            let refusal = unmodelled_signature(*item, body, &observed)
                .map(Refusal::Signature)
                .or_else(|| {
                    (!refusals.contains_key(&item.0))
                        .then(|| unmodelled_body_type(*item, body, &observed).map(Refusal::Body))
                        .flatten()
                });
            if let Some(refusal) = refusal {
                refusals.insert(item.0, refusal);
                changed = true;
            }
        }
        changed |= refusals.len() != known;
        if !changed {
            let refusals = items
                .iter()
                .map(|item| refusals.get(&item.0).cloned())
                .collect();
            return Ok(MappedBodies {
                items,
                bodies,
                refusals,
            });
        }
    }
}

/// What a function's signature states and the unit cannot: a binder or a type. Rustc has
/// normalized a type where it can; an associated type of a parameter it cannot.
fn unmodelled_signature(
    item: CrateItem,
    body: &PreparedBody,
    observed: &[AdtDef],
) -> Option<String> {
    if closure_of(item).is_some() {
        return None;
    }
    let function = function_def(item).ok()?;
    let generics = flattened_generics(function).ok()?;
    DeclaredSignature::new(function, generics.len())
        .at_locals(body)
        .types
        .into_iter()
        .find(|ty| collect_type_adts(*ty, observed, &mut Vec::new()).is_err())
        .map(|ty| {
            format!("its signature states `{ty}`, which is not modelled, so a call of it refuses its caller")
        })
}

/// A type a body states and the unit cannot: of a local that is no parameter or result,
/// of a call's instantiation, or of what a closure it holds takes or returns.
fn unmodelled_body_type(
    item: CrateItem,
    body: &PreparedBody,
    observed: &[AdtDef],
) -> Option<String> {
    let mappable = |ty: RustTy| collect_type_adts(ty, observed, &mut Vec::new()).is_ok();
    let locals = &body.mapped_locals;
    if let Some(local) = locals
        .iter()
        .skip(body.arg_locals().len() + 1)
        .find(|local| !mappable(local.ty))
    {
        return Some(format!("a local of type `{}` is not modelled", local.ty));
    }
    let mut uses = ClosureUses::default();
    uses.visit_body(body);
    let closure_type = uses
        .mentioned
        .iter()
        .filter_map(|closure| CrateItem(closure.def_id()).body())
        .find_map(|held| {
            let arguments = held.arg_locals().len();
            PreparedBody::new(held)
                .mapped_locals
                .iter()
                .take(arguments + 1)
                .map(|local| local.ty)
                .find(|ty| !mappable(*ty))
        });
    if let Some(ty) = closure_type {
        return Some(format!(
            "it holds a closure that takes or returns `{ty}`, which is not modelled"
        ));
    }
    let calls = collect_calls(std::slice::from_ref(&item), std::slice::from_ref(body)).ok()?;
    calls
        .instantiations
        .into_iter()
        .find(|ty| !mappable(*ty))
        .map(|ty| {
            format!("a call instantiates a generic parameter with `{ty}`, which is not modelled")
        })
}

/// A body with the parameters and result of `body` and nothing else, so that nothing
/// inside `body` reaches the unit.
fn refused_body(body: &Body) -> Body {
    let arguments = body.arg_locals().len();
    let kept = |info: &&VarDebugInfo| matches!(&info.value, VarDebugInfoContents::Place(place) if place.local <= arguments);
    Body::new(
        vec![BasicBlock {
            statements: vec![],
            terminator: Terminator {
                kind: TerminatorKind::Unreachable,
                source_info: SourceInfo {
                    span: body.span,
                    scope: 0,
                },
            },
        }],
        body.locals()[..=arguments].to_vec(),
        arguments,
        body.var_debug_info.iter().filter(kept).cloned().collect(),
        body.spread_arg(),
        body.span,
    )
}

/// The body of an opaque function: no parameters, and the result nothing can read.
fn opaque_body(body: &Body) -> Body {
    let never = MirLocalDecl {
        ty: RustTy::from_rigid_kind(RigidTy::Never),
        span: body.span,
        mutability: Mutability::Not,
    };
    Body::new(
        refused_body(body).blocks,
        vec![never],
        0,
        vec![],
        None,
        body.span,
    )
}

/// The argument tuples closure calls untuple, by local: `t = (a, b)` before
/// `Fn*::call*(s, move t)` in its block with nothing but drop flags between, `t`
/// having no other use. Each argument is then read at the call.
fn closure_call_tuples(
    caller: &str,
    item: CrateItem,
    body: &Body,
    locals: &[MirLocalDecl],
    crate_files: &HashSet<String>,
) -> Result<HashMap<usize, Vec<Operand>>, String> {
    let mut folds = HashMap::new();
    for block in &body.blocks {
        let TerminatorKind::Call { func, args, .. } = &block.terminator.kind else {
            continue;
        };
        let Ok(callee_type) = func.ty(locals) else {
            continue;
        };
        let TyKind::RigidTy(RigidTy::FnDef(definition, arguments)) = callee_type.kind() else {
            continue;
        };
        if closure_callee(item.0, definition, &arguments).is_none()
            && !declared_method_evidence(item.0)
                .iter()
                .any(|e| e.tupled && e.method == definition && e.arguments == arguments)
        {
            continue;
        }
        let at_call = |reason: &str| {
            format!(
                "{} {reason}",
                call_site(
                    definition,
                    caller,
                    block.terminator.source_info.span,
                    crate_files,
                )
            )
        };
        let [_, tuple] = args.as_slice() else {
            return Err(at_call("does not take a receiver and an argument tuple"));
        };
        let tuple = match tuple {
            Operand::Move(place) | Operand::Copy(place) if place.projection.is_empty() => {
                place.local
            },
            Operand::Constant(constant)
                if matches!(
                    constant.ty().kind(),
                    TyKind::RigidTy(RigidTy::Tuple(elements)) if elements.is_empty()
                ) =>
            {
                continue;
            },
            _ => return Err(at_call("takes an argument tuple it does not build")),
        };
        let mut built = None;
        for statement in block.statements.iter().rev() {
            match &statement.kind {
                StatementKind::Assign(place, Rvalue::Aggregate(AggregateKind::Tuple, operands))
                    if place.local == tuple && place.projection.is_empty() =>
                {
                    built = Some(operands.clone());
                    break;
                },
                StatementKind::Assign(_, Rvalue::Use(Operand::Constant(_), _))
                | StatementKind::StorageLive(_)
                | StatementKind::StorageDead(_)
                | StatementKind::Nop => {},
                _ => break,
            }
        }
        let mut mentions = LocalMentions {
            local: tuple,
            mentions: 0,
        };
        mentions.visit_body(body);
        let Some(operands) = built else {
            return Err(at_call("takes an argument tuple it does not build"));
        };
        if mentions.mentions != storage_mentions(body, tuple) + 2 {
            return Err(at_call("uses its argument tuple elsewhere"));
        }
        folds.insert(tuple, operands);
    }
    Ok(folds)
}

/// Whether a body writes or mutably borrows a place of a local, other than through a
/// reference. A drop only ends it.
struct LocalWrites {
    local: Local,
    written: bool,
}

impl MirVisitor for LocalWrites {
    fn visit_terminator(&mut self, terminator: &Terminator, location: MirLocation) {
        if !matches!(terminator.kind, TerminatorKind::Drop { .. }) {
            self.super_terminator(terminator, location);
        }
    }

    fn visit_place(&mut self, place: &MirPlace, context: PlaceContext, _location: MirLocation) {
        self.written |= place.local == self.local
            && context.is_mutating()
            && !place
                .projection
                .iter()
                .any(|projection| matches!(projection, ProjectionElem::Deref));
    }
}

fn local_written(body: &Body, local: Local) -> bool {
    let mut writes = LocalWrites {
        local,
        written: false,
    };
    writes.visit_body(body);
    writes.written
}

/// Counts the mentions of one local.
struct LocalMentions {
    local: Local,
    mentions: usize,
}

impl MirVisitor for LocalMentions {
    fn visit_local(&mut self, local: &Local, _context: PlaceContext, _location: MirLocation) {
        if *local == self.local {
            self.mentions += 1;
        }
    }
}

/// What undoes rustc's elaboration of `Box`. Optimized MIR has no deref of a box:
/// `*b` reads `p = copy (b.0.0) as *const T (Transmute)`, one pointer per use, behind
/// a reference first `t = copy *r`, and a box whose content moved out is freed by
/// `f = &mut b; <Box<T> as Drop>::drop(move f)`. Only this exact shape folds, by the
/// box's language item, to a deref of the box; every other raw pointer stays
/// unsupported. The free only releases the allocation and has no meaning in a value
/// semantics: rustc rejects it in source (E0040), and it is erased only for a box
/// whose content a folded pointer reads, the elaborated free after its content's drop.
#[derive(Clone, Default)]
struct BoxFolds {
    /// A pointer local and the box it points into.
    pointers: HashMap<usize, MirPlace>,
    /// A temporary copy of a box read only for its pointers, and the place it copies.
    aliases: HashMap<usize, MirPlace>,
    /// The blocks ending in an elaborated free.
    frees: HashSet<usize>,
    /// The borrows of the box those frees take.
    free_borrows: HashSet<usize>,
}

impl BoxFolds {
    /// A place with its pointers and aliases replaced, a deref of a box remaining.
    fn resolve(&self, place: &MirPlace) -> MirPlace {
        let mut place = place.clone();
        loop {
            let base = match place.projection.first() {
                Some(ProjectionElem::Deref) => self.pointers.get(&place.local),
                _ => None,
            }
            .or_else(|| self.aliases.get(&place.local));
            let Some(base) = base else {
                return place;
            };
            let mut projection = base.projection.clone();
            projection.extend(place.projection.iter().cloned());
            place = MirPlace {
                local: base.local,
                projection,
            };
        }
    }

    /// Whether a statement defines what folds, so that nothing reads it.
    fn folds_statement(&self, kind: &StatementKind) -> bool {
        let StatementKind::Assign(destination, _) = kind else {
            return false;
        };
        destination.projection.is_empty()
            && (self.pointers.contains_key(&destination.local)
                || self.aliases.contains_key(&destination.local)
                || self.free_borrows.contains(&destination.local))
    }
}

fn is_box_type(ty: RustTy) -> bool {
    ty.kind().is_box()
}

/// The box a pointer's assignment reads: `copy (b.0.0) as *const T (Transmute)`.
fn box_pointer_source(rvalue: &Rvalue, locals: &[MirLocalDecl]) -> Option<MirPlace> {
    let Rvalue::Cast(CastKind::Transmute, Operand::Copy(source), _) = rvalue else {
        return None;
    };
    let [prefix @ .., ProjectionElem::Field(0, _), ProjectionElem::Field(0, _)] =
        source.projection.as_slice()
    else {
        return None;
    };
    let base = MirPlace {
        local: source.local,
        projection: prefix.to_vec(),
    };
    is_box_type(base.ty(locals).ok()?).then_some(base)
}

/// Whether a statement changes a local or lends it mutably.
fn statement_changes(kind: &StatementKind, local: usize) -> bool {
    match kind {
        StatementKind::Assign(destination, rvalue) => {
            destination.local == local
                || matches!(rvalue,
                    Rvalue::Ref(_, MirBorrowKind::Mut { .. }, place) if place.local == local)
                || matches!(rvalue,
                    Rvalue::AddressOf(RawPtrKind::Mut, place) if place.local == local)
        },
        StatementKind::SetDiscriminant { place, .. } => place.local == local,
        StatementKind::StorageDead(dead) => *dead == local,
        _ => false,
    }
}

fn terminator_changes(kind: &TerminatorKind, local: usize) -> bool {
    match kind {
        TerminatorKind::Drop { place, .. } => place.local == local,
        TerminatorKind::Call { destination, .. } => destination.local == local,
        _ => false,
    }
}

/// Whether no path from a pointer's assignment changes the box's local before a
/// use of the pointer, so that the fold reads the box the pointer points into.
fn box_unchanged_until_uses(body: &Body, pointer: usize, root: usize) -> bool {
    let mentions_in = |visit: &dyn Fn(&mut LocalMentions)| {
        let mut mentions = LocalMentions {
            local: pointer,
            mentions: 0,
        };
        visit(&mut mentions);
        mentions.mentions != 0
    };
    let defines = |kind: &StatementKind| {
        matches!(kind, StatementKind::Assign(destination, _)
            if destination.local == pointer && destination.projection.is_empty())
    };
    let mut pending = vec![];
    for (block, data) in body.blocks.iter().enumerate() {
        for (index, statement) in data.statements.iter().enumerate() {
            if defines(&statement.kind) {
                pending.push((block, index + 1, false));
            }
        }
    }
    let mut visited = HashSet::new();
    while let Some((block, start, mut changed)) = pending.pop() {
        if start == 0 && !visited.insert((block, changed)) {
            continue;
        }
        let data = &body.blocks[block];
        let mut ended = false;
        for (index, statement) in data.statements.iter().enumerate().skip(start) {
            if defines(&statement.kind) {
                ended = true;
                break;
            }
            let location = statement_location(body, &block, index);
            let used = mentions_in(&|mentions| mentions.visit_statement(statement, location));
            if used && !matches!(statement.kind, StatementKind::StorageDead(_)) && changed {
                return false;
            }
            changed |= statement_changes(&statement.kind, root);
        }
        if ended {
            continue;
        }
        let location = terminator_location(body, &block);
        let used = mentions_in(&|mentions| mentions.visit_terminator(&data.terminator, location));
        if used && changed {
            return false;
        }
        changed |= terminator_changes(&data.terminator.kind, root);
        for successor in data.terminator.successors() {
            pending.push((successor, 0, changed));
        }
    }
    true
}

/// Whether a callee is `Drop::drop` at a box.
fn is_box_drop_call(func: &Operand, locals: &[MirLocalDecl]) -> bool {
    let Ok(callee) = func.ty(locals) else {
        return false;
    };
    let TyKind::RigidTy(RigidTy::FnDef(definition, arguments)) = callee.kind() else {
        return false;
    };
    Some(definition.0) == polyfill::drop_method()
        && matches!(arguments.0.first(),
            Some(GenericArgKind::Type(self_type)) if is_box_type(*self_type))
}

fn box_folds(body: &Body) -> BoxFolds {
    let locals = body.locals();
    let mut folds = BoxFolds::default();
    let count_mentions = |local: usize| {
        let mut mentions = LocalMentions { local, mentions: 0 };
        mentions.visit_body(body);
        mentions.mentions - storage_mentions(body, local)
    };
    let mut sources = vec![];
    for block in &body.blocks {
        for statement in &block.statements {
            if let StatementKind::Assign(destination, rvalue) = &statement.kind
                && destination.projection.is_empty()
                && matches!(
                    locals[destination.local].ty.kind(),
                    TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Not))
                )
                && let Some(source) = box_pointer_source(rvalue, locals)
            {
                sources.push((destination.local, source));
            }
        }
    }
    if sources.is_empty() {
        return folds;
    }
    // Each pointer: its one assignment, and otherwise only derefs.
    for (pointer, source) in &sources {
        let mut derefs = LocalDerefs {
            local: *pointer,
            derefs: 0,
        };
        derefs.visit_body(body);
        if count_mentions(*pointer) == derefs.derefs + 1 {
            folds.pointers.insert(*pointer, source.clone());
        }
    }
    // A temporary box: one bitwise copy, otherwise read only by the pointers.
    for block in &body.blocks {
        for statement in &block.statements {
            let StatementKind::Assign(destination, rvalue) = &statement.kind else {
                continue;
            };
            let (Rvalue::Use(Operand::Copy(copied), WithRetag::No) | Rvalue::CopyForDeref(copied)) =
                rvalue
            else {
                continue;
            };
            let readers = folds
                .pointers
                .values()
                .filter(|source| source.local == destination.local)
                .count();
            if destination.projection.is_empty()
                && is_box_type(locals[destination.local].ty)
                && copied
                    .projection
                    .iter()
                    .any(|projection| matches!(projection, ProjectionElem::Deref))
                && readers > 0
                && count_mentions(destination.local) == readers + 1
            {
                folds.aliases.insert(destination.local, copied.clone());
            }
        }
    }
    let checked = folds.pointers.keys().all(|pointer| {
        let root = folds
            .resolve(&MirPlace {
                local: *pointer,
                projection: vec![ProjectionElem::Deref],
            })
            .local;
        box_unchanged_until_uses(body, *pointer, root)
    });
    if !checked
        || sources
            .iter()
            .any(|(pointer, _)| !folds.pointers.contains_key(pointer))
    {
        return BoxFolds::default();
    }
    // An elaborated free: its borrow and call in one block, of a box a pointer reads.
    for (index, block) in body.blocks.iter().enumerate() {
        let TerminatorKind::Call { func, args, .. } = &block.terminator.kind else {
            continue;
        };
        if !is_box_drop_call(func, locals) {
            continue;
        }
        let [Operand::Move(borrow)] = args.as_slice() else {
            continue;
        };
        if !borrow.projection.is_empty() || count_mentions(borrow.local) != 2 {
            continue;
        }
        let borrowed = block
            .statements
            .iter()
            .find_map(|statement| match &statement.kind {
                StatementKind::Assign(
                    destination,
                    Rvalue::Ref(_, MirBorrowKind::Mut { .. }, place),
                ) if destination.local == borrow.local && destination.projection.is_empty() => {
                    Some(place)
                },
                _ => None,
            });
        let Some(borrowed) = borrowed else {
            continue;
        };
        let content_read = folds
            .pointers
            .values()
            .any(|source| folds.resolve(source) == folds.resolve(borrowed));
        if content_read {
            folds.frees.insert(index);
            folds.free_borrows.insert(borrow.local);
        }
    }
    folds
}

/// Counts the places derefing one local at their first step.
struct LocalDerefs {
    local: Local,
    derefs: usize,
}

impl MirVisitor for LocalDerefs {
    fn visit_place(&mut self, place: &MirPlace, _context: PlaceContext, _location: MirLocation) {
        if place.local == self.local
            && matches!(place.projection.first(), Some(ProjectionElem::Deref))
        {
            self.derefs += 1;
        }
    }
}

/// The locals of a body as they are mapped. A pointer of a folded box dereference
/// reads as the reference it stands for (`BoxFolds`). rustc reads a slice's length for
/// a bounds check through a pointer it creates for that read alone,
/// `p = &raw const (fake) *r` followed by `PtrMetadata(p)`. A pointer local
/// assigned only such borrows and used only by `PtrMetadata` is mapped as a
/// shared borrow of the same place, whose metadata is the same length. The
/// MIR validator does not enforce this use, so it is checked here; every
/// other raw pointer remains unsupported.
fn mapped_locals(body: &Body, folds: &BoxFolds) -> Vec<MirLocalDecl> {
    let mut locals = body.locals().to_vec();
    for (index, declaration) in locals.iter_mut().enumerate() {
        let TyKind::RigidTy(RigidTy::RawPtr(pointee, Mutability::Not)) = declaration.ty.kind()
        else {
            continue;
        };
        if folds.pointers.contains_key(&index) {
            declaration.ty = RustTy::new_ref(
                Region {
                    kind: RegionKind::ReErased,
                },
                pointee,
                Mutability::Not,
            );
            continue;
        }
        let mut permitted = 0;
        let mut assigned = false;
        for block in &body.blocks {
            for statement in &block.statements {
                match &statement.kind {
                    StatementKind::Assign(place, rvalue)
                        if place.local == index && place.projection.is_empty() =>
                    {
                        if matches!(rvalue, Rvalue::AddressOf(RawPtrKind::FakeForPtrMetadata, _)) {
                            assigned = true;
                            permitted += 1;
                        }
                    },
                    StatementKind::Assign(
                        _,
                        Rvalue::UnaryOp(
                            UnOp::PtrMetadata,
                            Operand::Move(operand) | Operand::Copy(operand),
                        ),
                    ) if operand.local == index && operand.projection.is_empty() => permitted += 1,
                    StatementKind::StorageLive(local) | StatementKind::StorageDead(local)
                        if *local == index =>
                    {
                        permitted += 1
                    },
                    _ => {},
                }
            }
        }
        let mut mentions = LocalMentions {
            local: index,
            mentions: 0,
        };
        mentions.visit_body(body);
        if assigned && mentions.mentions == permitted {
            declaration.ty = RustTy::new_ref(
                Region {
                    kind: RegionKind::ReErased,
                },
                pointee,
                Mutability::Not,
            );
        }
    }
    locals
}

/// A profile borrow kind the structurizer settles.
fn structurized_borrow_kind(tag: &str) -> BorrowKind {
    BorrowKind::Profile {
        value: ProfileValue {
            payload: String::new(),
            profile: Profile::Rust,
            tag: tag.to_owned(),
        },
    }
}

/// A mutable borrow's kind: rustc's two-phase borrows activate at their use.
fn mutable_borrow_kind(kind: &MutBorrowKind) -> BorrowKind {
    match kind {
        MutBorrowKind::TwoPhaseBorrow => structurized_borrow_kind("borrow.two_phase"),
        MutBorrowKind::Default | MutBorrowKind::ClosureCapture => BorrowKind::Mutable,
    }
}

/// The borrow temporaries an unsizing coercion folds into its borrow, by
/// local: `_t = &P` directly followed by `_c = _t as &[T]`, `_t` having no other
/// use. The coercion then borrows the unsized place itself, so no loan is ever
/// held at the array's type and retyped.
fn unsize_folds(body: &Body, locals: &[MirLocalDecl]) -> HashMap<usize, (BorrowKind, MirPlace)> {
    let mut folds = HashMap::new();
    for block in &body.blocks {
        let mut borrow: Option<(usize, BorrowKind, &MirPlace)> = None;
        for statement in &block.statements {
            match &statement.kind {
                StatementKind::Assign(destination, Rvalue::Ref(_, kind, place))
                    if destination.projection.is_empty() =>
                {
                    let kind = match kind {
                        MirBorrowKind::Shared => Some(BorrowKind::Immutable),
                        MirBorrowKind::Mut { kind } => Some(mutable_borrow_kind(kind)),
                        MirBorrowKind::Fake(_) => None,
                    };
                    borrow = kind.map(|kind| (destination.local, kind, place));
                },
                StatementKind::Assign(
                    _,
                    Rvalue::Cast(
                        CastKind::PointerCoercion(PointerCoercion::Unsize),
                        Operand::Copy(operand) | Operand::Move(operand),
                        target,
                    ),
                ) => {
                    if let Some((local, kind, place)) = borrow.take()
                        && operand.local == local
                        && operand.projection.is_empty()
                        && is_array_to_slice(locals[local].ty, *target)
                    {
                        let mut mentions = LocalMentions { local, mentions: 0 };
                        mentions.visit_body(body);
                        // The storage markers, the borrow's destination and
                        // the coercion's operand.
                        let storage = storage_mentions(body, local);
                        if mentions.mentions == storage + 2 {
                            folds.insert(local, (kind, place.clone()));
                        }
                    }
                },
                StatementKind::StorageLive(_)
                | StatementKind::StorageDead(_)
                | StatementKind::Nop => {},
                _ => borrow = None,
            }
        }
    }
    folds
}

/// A promoted immutable array reference immediately coerced to a slice.
/// Keep its constant backing as a computed shared view instead of inventing a
/// frame-owned loan through the temporary holding the static reference.
fn constant_unsize_folds(
    body: &Body,
    locals: &[MirLocalDecl],
    constants: &EvaluatedConstants,
) -> HashMap<usize, EvaluatedConstant> {
    let mut folds = HashMap::new();
    for block in &body.blocks {
        let mut constant = None;
        for statement in &block.statements {
            match &statement.kind {
                StatementKind::Assign(destination, Rvalue::Use(Operand::Constant(value), _))
                    if destination.projection.is_empty() =>
                {
                    constant = Some((destination.local, value));
                },
                StatementKind::Assign(
                    _,
                    Rvalue::Cast(
                        CastKind::PointerCoercion(PointerCoercion::Unsize),
                        Operand::Copy(operand) | Operand::Move(operand),
                        target,
                    ),
                ) => {
                    if let Some((local, constant)) = constant.take()
                        && operand.local == local
                        && operand.projection.is_empty()
                        && matches!(
                            target.kind(),
                            TyKind::RigidTy(RigidTy::Ref(_, _, Mutability::Not))
                        )
                        && is_array_to_slice(locals[local].ty, *target)
                        && let Some(EvaluatedConstant::Shared { referent, .. }) =
                            constants.values.get(&constant.const_)
                        && matches!(referent.as_ref(), EvaluatedConstant::Array { .. })
                    {
                        let mut mentions = LocalMentions { local, mentions: 0 };
                        mentions.visit_body(body);
                        if mentions.mentions == storage_mentions(body, local) + 2 {
                            folds.insert(local, EvaluatedConstant::Shared {
                                ty: *target,
                                referent: referent.clone(),
                            });
                        }
                    }
                },
                StatementKind::StorageLive(_)
                | StatementKind::StorageDead(_)
                | StatementKind::Nop => {},
                _ => constant = None,
            }
        }
    }
    folds
}

/// The storage markers of a local in a body.
fn storage_mentions(body: &Body, local: usize) -> usize {
    body.blocks
        .iter()
        .flat_map(|block| &block.statements)
        .filter(|statement| {
            matches!(
                statement.kind,
                StatementKind::StorageLive(marked) | StatementKind::StorageDead(marked)
                    if marked == local
            )
        })
        .count()
}

/// Whether a coercion's source and target are `&[T; N]` and `&[T]`.
fn is_array_to_slice(source: RustTy, target: RustTy) -> bool {
    let (TyKind::RigidTy(RigidTy::Ref(_, source, _)), TyKind::RigidTy(RigidTy::Ref(_, target, _))) =
        (source.kind(), target.kind())
    else {
        return false;
    };
    matches!(source.kind(), TyKind::RigidTy(RigidTy::Array(..)))
        && matches!(target.kind(), TyKind::RigidTy(RigidTy::Slice(_)))
}

/// The ADTs a body observes the shape of: those it constructs, reads the
/// discriminant of, downcasts, or projects a field of.
struct ObservedAdts<'a> {
    locals: &'a [MirLocalDecl],
    folds: &'a BoxFolds,
    adts: Vec<AdtDef>,
}

impl ObservedAdts<'_> {
    fn observe(&mut self, ty: RustTy) {
        if let TyKind::RigidTy(RigidTy::Adt(definition, _)) = ty.kind()
            && !self.adts.contains(&definition)
        {
            self.adts.push(definition);
        }
    }
}

impl MirVisitor for ObservedAdts<'_> {
    fn visit_statement(&mut self, statement: &Statement, location: MirLocation) {
        if !self.folds.folds_statement(&statement.kind) {
            self.super_statement(statement, location);
        }
    }

    fn visit_place(&mut self, place: &MirPlace, _context: PlaceContext, _location: MirLocation) {
        for (index, projection) in place.projection.iter().enumerate() {
            if matches!(
                projection,
                ProjectionElem::Downcast(_) | ProjectionElem::Field(..)
            ) {
                let base = MirPlace {
                    local: place.local,
                    projection: place.projection[..index].to_vec(),
                };
                if let Ok(ty) = base.ty(self.locals) {
                    self.observe(ty);
                }
            }
        }
    }

    fn visit_rvalue(&mut self, rvalue: &Rvalue, location: MirLocation) {
        match rvalue {
            Rvalue::Aggregate(AggregateKind::Adt(definition, ..), _)
                if !self.adts.contains(definition) =>
            {
                self.adts.push(*definition)
            },
            Rvalue::Discriminant(place) => {
                if let Ok(ty) = place.ty(self.locals) {
                    self.observe(ty);
                }
            },
            _ => {},
        }
        self.super_rvalue(rvalue, location);
    }
}

/// An ADT the unit reaches. A dependency's struct whose shape no body observes
/// is opaque: the unit names it without its fields. An enum's variants are
/// public, so its shape is carried unless a payload's type has no mapping.
#[derive(Clone, Copy)]
struct ReachedAdt {
    definition: AdtDef,
    opaque: bool,
}

/// The ADTs the bodies observe the shape of.
fn observed_adts(bodies: &[PreparedBody]) -> Vec<AdtDef> {
    let mut observed = Vec::new();
    for body in bodies {
        let locals = &body.mapped_locals;
        let folds = &body.box_folds;
        let mut visitor = ObservedAdts {
            locals,
            folds,
            adts: observed,
        };
        visitor.visit_body(body);
        observed = visitor.adts;
    }
    observed
}

/// The ADTs the bodies' locals, the called dependency functions' signatures,
/// the types calls instantiate with, and the evaluated constants reach. `observed`
/// holds those the bodies observe; a constant also states the shape of those it holds.
fn collect_adts(
    bodies: &[PreparedBody],
    mut observed: Vec<AdtDef>,
    signatures: &[RustTy],
    instantiations: &[RustTy],
    constants: &EvaluatedConstants,
) -> Result<Vec<ReachedAdt>, String> {
    observed.extend(constants.adts.iter().copied());
    let mut adts = Vec::new();
    for body in bodies {
        for local in &body.mapped_locals {
            collect_type_adts(local.ty, &observed, &mut adts)?;
        }
    }
    for ty in signatures {
        collect_signature_adts(*ty, &observed, &mut adts)?;
    }
    for ty in instantiations.iter().chain(&constants.types) {
        collect_type_adts(*ty, &observed, &mut adts)?;
    }
    adts.sort_by_key(|adt| adt.definition.name());
    Ok(adts)
}

fn collect_type_adts(
    ty: RustTy,
    observed: &[AdtDef],
    adts: &mut Vec<ReachedAdt>,
) -> Result<(), String> {
    match ty.kind() {
        TyKind::RigidTy(RigidTy::Tuple(elements)) => {
            for element in elements {
                collect_type_adts(element, observed, adts)?;
            }
            Ok(())
        },
        TyKind::RigidTy(RigidTy::Array(element, length)) => {
            length
                .eval_target_usize()
                .map_err(|error| format!("symbolic array length is not modelled: {error:?}"))?;
            collect_type_adts(element, observed, adts)
        },
        TyKind::RigidTy(RigidTy::Slice(element)) => collect_type_adts(element, observed, adts),
        TyKind::RigidTy(RigidTy::FnPtr(signature)) => {
            let signature = supported_function_pointer_signature(&signature)?;
            for type_ in &signature.inputs_and_output {
                collect_type_adts(*type_, observed, adts)?;
            }
            Ok(())
        },
        TyKind::RigidTy(RigidTy::Closure(_, arguments)) => {
            for part in closure_parts(&arguments) {
                collect_type_adts(part, observed, adts)?;
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
        TyKind::RigidTy(RigidTy::Ref(_, referent, _)) => {
            collect_type_adts(referent, observed, adts)
        },
        TyKind::RigidTy(RigidTy::Adt(definition, arguments)) => {
            for argument in &arguments.0 {
                if let GenericArgKind::Type(argument) = argument {
                    collect_type_adts(*argument, observed, adts)?;
                }
            }
            collect_adt(definition, observed, adts)
        },
        kind => Err(format!("unsupported Rust local type: {kind:?}")),
    }
}

/// An ADT, and what its fields reach if the unit carries its shape.
fn collect_adt(
    definition: AdtDef,
    observed: &[AdtDef],
    adts: &mut Vec<ReachedAdt>,
) -> Result<(), String> {
    if definition.kind() == AdtKind::Union {
        return Err(format!(
            "unsupported Rust ADT kind for `{}`: {}",
            definition.name(),
            definition.kind()
        ));
    }
    if adts.iter().any(|adt| adt.definition == definition) {
        return Ok(());
    }
    let required =
        definition.krate() == rustc_public::local_crate() || observed.contains(&definition);
    if required || definition.kind() == AdtKind::Enum {
        let mut shaped = adts.clone();
        shaped.push(ReachedAdt {
            definition,
            opaque: false,
        });
        let carried = definition.variants().into_iter().try_for_each(|variant| {
            variant
                .fields()
                .into_iter()
                .try_for_each(|field| collect_type_adts(field.ty(), observed, &mut shaped))
        });
        match carried {
            Ok(()) => {
                *adts = shaped;
                return Ok(());
            },
            Err(message) if required => return Err(message),
            Err(_) => {},
        }
    }
    adts.push(ReachedAdt {
        definition,
        opaque: true,
    });
    Ok(())
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
    item_names: &[String],
    item_scopes: &[Vec<String>],
    reached: &[ReachedAdt],
    scopes: &mut Scopes,
    namespaces: &CrateNamespaces,
) -> Result<(Vec<QualifiedName>, Vec<AdtNames>), String> {
    let mut names = item_names
        .iter()
        .zip(item_scopes)
        .map(|(name, scope)| QualifiedName {
            name: name.clone(),
            namespace_id: NamespaceId::new(0),
            scope: scope.clone(),
        })
        .collect::<Vec<_>>();
    let mut adts = Vec::with_capacity(reached.len());
    let mut impls = AdtImpls::default();
    impls.settle_drop_reach(reached);
    for adt in reached {
        let definition = adt.definition;
        let copy = impls
            .of_crate(definition.krate())
            .copy
            .contains(&definition);
        let destructor = impls.has_destructor(adt);
        let name = NameId::new(names.len());
        let krate = definition.krate().name;
        let namespace_id = namespaces.id(&krate)?;
        // Variants and fields are named in the module of their type.
        let scope = scopes.module_scope(&krate, definition.def_id());
        names.push(QualifiedName {
            name: last_segment(&definition.name()),
            namespace_id,
            scope: scope.clone(),
        });
        let mut variants = Vec::new();
        let shape = if adt.opaque {
            vec![]
        } else {
            definition.variants()
        };
        for (ordinal, variant) in shape.into_iter().enumerate() {
            let source_name = (definition.kind() == AdtKind::Enum).then(|| variant.name());
            let variant_name = source_name.as_ref().map(|source_name| {
                let id = NameId::new(names.len());
                names.push(QualifiedName {
                    name: source_name.clone(),
                    namespace_id,
                    scope: scope.clone(),
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
                        namespace_id,
                        scope: scope.clone(),
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
            definition,
            name,
            namespace_id,
            copy,
            destructor,
            opaque: adt.opaque,
            variants,
        });
    }
    Ok((names, adts))
}

/// The namespaces of a unit: the local crate's first, then one per dependency
/// crate the unit reaches, ordered by name.
struct CrateNamespaces {
    local: String,
    dependencies: BTreeMap<String, (CrateNum, NamespaceId)>,
}

impl CrateNamespaces {
    fn new(crates: impl IntoIterator<Item = Crate>) -> Result<Self, String> {
        let local = rustc_public::local_crate().name;
        let mut reached = BTreeMap::<String, CrateNum>::new();
        for krate in crates {
            if krate.is_local {
                continue;
            }
            match reached.get(&krate.name) {
                Some(id) if *id != krate.id => {
                    return Err(format!("two dependency crates are named `{}`", krate.name));
                },
                Some(_) => {},
                None => {
                    reached.insert(krate.name, krate.id);
                },
            }
        }
        let dependencies = reached
            .into_iter()
            .enumerate()
            .map(|(index, (name, id))| (name, (id, NamespaceId::new(index + 1))))
            .collect();
        Ok(Self {
            local,
            dependencies,
        })
    }

    fn id(&self, krate: &str) -> Result<NamespaceId, String> {
        if krate == self.local {
            return Ok(NamespaceId::new(0));
        }
        self.dependencies
            .get(krate)
            .map(|(_, id)| *id)
            .ok_or_else(|| format!("crate `{krate}` has no namespace"))
    }
}

fn map_structs(
    adts: &[AdtNames],
    types: &mut TypeLayout,
    sources: &mut SourceTables,
) -> Result<Vec<StructDecl>, String> {
    adts.iter()
        .map(|adt| {
            let loc = sources.declaration(
                adt.namespace_id,
                &adt.definition.name(),
                adt.definition.span(),
            )?;
            let copy_parameters = polyfill::copy_parameters(adt.definition.0)
                .into_iter()
                .collect();
            let mut generics = adt
                .definition
                .generics_of()
                .params
                .into_iter()
                .map(|parameter| generic_binder(parameter, &copy_parameters, loc, types))
                .collect::<Result<Vec<_>, String>>()?;
            if let Some(first) = generics.first_mut() {
                first.predicates = lifetime_predicates(adt.definition.0, types);
            }
            let (fields, variants) = if adt.opaque {
                (vec![], vec![])
            } else if adt.definition.kind() == AdtKind::Struct {
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
                abilities: if adt.copy {
                    vec![Ability::Copy]
                } else {
                    vec![]
                },
                attributes: vec![],
                contract: empty_contract(),
                doc: String::new(),
                fields,
                generics,
                ghosts: vec![],
                loc,
                locals: vec![],
                name: adt.name,
                properties: [
                    (adt.destructor, "struct.destructor"),
                    (adt.opaque, "struct.opaque"),
                ]
                .into_iter()
                .filter(|(stated, _)| *stated)
                .map(|(_, tag)| ProfileValue {
                    payload: String::new(),
                    profile: Profile::Rust,
                    tag: tag.to_owned(),
                })
                .collect(),
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
            let field_loc =
                sources.declaration(adt.namespace_id, &adt.definition.name(), field.span())?;
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
    /// Each function's bound regions, as binders of that function.
    bound_regions: Vec<LifetimeSpec>,
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

#[derive(Clone)]
struct LifetimeSpec {
    region: Region,
    kind: LifetimeKind,
    name: Option<String>,
}

impl TypeLayout {
    /// Complete only the types consulted along actual generic call edges. A
    /// nested frame must be able to look up the same instantiated type as its
    /// caller; interning a receiver/result cross product does not establish this.
    fn complete_invocation_types(
        &mut self,
        functions: &[FunctionDecl],
        expressions: &[Expr],
        ranges: &[std::ops::Range<usize>],
    ) -> Result<(), String> {
        let mut required = vec![BTreeSet::new(); functions.len()];
        let mut edges = vec![Vec::new(); functions.len()];
        for (index, range) in ranges.iter().enumerate() {
            for expression in &expressions[range.clone()] {
                let ExprKind::Operation {
                    operation: Operation::Call { kind },
                    instantiations,
                    ..
                } = &expression.kind
                else {
                    continue;
                };
                let reference = match kind {
                    CallKind::Function { callee } => callee,
                    CallKind::Closure { function, .. } => function,
                    _ => continue,
                };
                for arg in instantiations {
                    if let GenericArgument::TypeArg { value } = arg {
                        required[index].insert(value.type_id.index);
                    }
                }
                if reference.namespace_id.index == 0
                    && let Some(target) = functions.iter().position(|f| f.name == reference.name)
                {
                    edges[index].push((target, instantiations.clone()));
                }
            }
        }
        // As with static validation, growing polymorphic recursion has no finite
        // frame table. Bound the fixed point rather than hanging on such input.
        let bound = functions.len() * (self.types.len() + 1) + 1;
        for _ in 0..bound {
            let mut changed = false;
            for (index, calls) in edges.iter().enumerate() {
                for (target, arguments) in calls {
                    for ty in required[*target].clone() {
                        let instance = self.instantiate_field(TypeId::new(ty), arguments);
                        changed |= required[index].insert(instance.index);
                    }
                }
            }
            if !changed {
                return Ok(());
            }
        }
        Err("generic invocation types do not have a finite closure".to_owned())
    }

    fn evidence_function(&self, evidence: &MethodEvidence) -> Result<Ty, String> {
        let mut arguments = vec![self.id(evidence.receiver_type())?];
        arguments.extend(
            evidence
                .parameters
                .iter()
                .map(|ty| self.id(*ty))
                .collect::<Result<Vec<_>, _>>()?,
        );
        Ok(Ty::Function {
            abilities: vec![Ability::Copy, Ability::Drop],
            arguments,
            result: self.id(evidence.result)?,
        })
    }

    fn intern_method_evidence(&mut self, evidence: &[MethodEvidence]) -> Result<(), String> {
        for entry in evidence {
            // A dictionary naming an associated type has no identity as a Rust
            // type: the declaration that binds it interns it at its binders.
            if evidence_types(std::slice::from_ref(entry))
                .into_iter()
                .any(|ty| projection(ty).is_some())
            {
                continue;
            }
            let function = self.evidence_function(entry)?;
            if !self.types.contains(&function) {
                self.types.push(function);
            }
        }
        Ok(())
    }

    /// The type of a dictionary a dependency declares, in which an associated
    /// type is the type parameter of its binder, as its signature's is.
    fn signature_evidence_type(
        &mut self,
        evidence: &MethodEvidence,
        associated_types: &[AssociatedType],
        adts: &[AdtNames],
    ) -> Result<TypeId, String> {
        let mut arguments =
            vec![self.signature_type_id(evidence.receiver_type(), associated_types, adts)?];
        for parameter in &evidence.parameters {
            arguments.push(self.signature_type_id(*parameter, associated_types, adts)?);
        }
        let result = self.signature_type_id(evidence.result, associated_types, adts)?;
        let function = Ty::Function {
            abilities: vec![Ability::Copy, Ability::Drop],
            arguments,
            result,
        };
        Ok(
            match self.types.iter().position(|known| *known == function) {
                Some(index) => TypeId::new(index),
                None => {
                    self.types.push(function);
                    TypeId::new(self.types.len() - 1)
                },
            },
        )
    }

    fn method_evidence_type(&self, evidence: &MethodEvidence) -> Result<TypeId, String> {
        let function = self.evidence_function(evidence)?;
        self.types
            .iter()
            .position(|ty| *ty == function)
            .map(TypeId::new)
            .ok_or_else(|| "method evidence type was not interned".to_owned())
    }

    fn from_bodies(
        items: &[CrateItem],
        bodies: &[PreparedBody],
        adts: &[AdtNames],
        closures: &[ClosureNames],
        dependency_functions: &[DependencyFunction],
        declared: &[DeclaredSignature],
        instantiations: &[RustTy],
        constants: &EvaluatedConstants,
    ) -> Result<Self, String> {
        let mut needs_never = false;
        let mut needs_bool = false;
        let mut needs_character = false;
        let mut needed_integers = Vec::new();
        let mut needed_nominals = Vec::new();
        let mut needed_composites = Vec::new();
        let mut pending_references = Vec::new();
        let locals = bodies.iter().flat_map(|body| {
            body.mapped_locals
                .iter()
                .map(|local| local.ty)
                .collect::<Vec<_>>()
        });
        let signatures = dependency_functions
            .iter()
            .flat_map(|function| function.signature.iter().chain(&function.evidence).copied())
            .flat_map(parts_without_associated_types);
        let const_types = items
            .iter()
            .flat_map(|item| item_generics(*item).unwrap_or_default())
            .chain(
                dependency_functions
                    .iter()
                    .filter(|function| !function.opaque)
                    .flat_map(|function| {
                        flattened_generics(function.definition).unwrap_or_default()
                    }),
            )
            .chain(
                adts.iter()
                    .flat_map(|adt| adt.definition.generics_of().params),
            )
            .filter(|parameter| matches!(parameter.kind, GenericParamDefKind::Const { .. }))
            .map(|parameter| polyfill::const_param_ty(parameter.def_id));
        let reached = locals
            .chain(const_types)
            .chain(signatures)
            .chain(
                declared
                    .iter()
                    .flat_map(|signature| signature.types.iter().copied()),
            )
            .chain(instantiations.iter().copied())
            .chain(constants.types.iter().copied());
        for ty in reached {
            {
                observe_type(
                    ty,
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
        for (_, ty) in closures.iter().flat_map(|closure| &closure.fields) {
            observe_type(
                *ty,
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
        for adt in adts.iter().filter(|adt| !adt.opaque) {
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
            if adts
                .iter()
                .any(|adt| adt.definition == definition && adt.opaque)
            {
                continue;
            }
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
        let bound_regions = declared
            .iter()
            .chain(
                dependency_functions
                    .iter()
                    .map(|function| &function.declared),
            )
            .flat_map(DeclaredSignature::region_specs)
            .collect::<Vec<_>>();
        let spec = |region: Region| match bound_regions.iter().find(|bound| bound.region == region)
        {
            Some(bound) => bound.clone(),
            None => lifetime_spec(region),
        };
        let mut lifetimes = lifetime_regions.into_iter().map(spec).collect::<Vec<_>>();
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
        let functions = items.iter().map(|item| item_generics(*item)).chain(
            dependency_functions
                .iter()
                .filter(|function| !function.opaque)
                .map(|function| flattened_generics(function.definition)),
        );
        for generics in functions {
            for parameter in generics? {
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
        parameter_indices.extend(
            dependency_functions
                .iter()
                .flat_map(|function| function.associated_types.iter())
                .map(|associated| associated.index),
        );
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
        let mut composites = Vec::new();
        let mut references = Vec::new();
        // A nominal's arguments, a composite's elements and a referent depend on
        // each other in any nesting (`Option<&u32>`, `&Option<u32>`).
        loop {
            let before = nominals.len() + composites.len() + references.len();
            // A nominal's type arguments may be composites and references,
            // which may in turn hold nominals, so each kind waits for what it
            // names.
            for ty in &needed_nominals {
                if nominals.iter().any(|entry: &NominalEntry| entry.ty == *ty) {
                    continue;
                }
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
                        GenericArgKind::Type(argument) => Ok(interned_type_id(
                            TypeId::new(0),
                            never,
                            bool_,
                            character,
                            &type_parameters,
                            &integers,
                            &nominals,
                            &composites,
                            &references,
                            argument,
                        )
                        .map(|type_id| GenericArgument::TypeArg {
                            value: TypeUse {
                                loc: LocId::new(0),
                                type_id,
                            },
                        })),
                        GenericArgKind::Lifetime(region) => lifetimes
                            .iter()
                            .position(|entry| entry.region == region)
                            .map(|index| {
                                Some(GenericArgument::Lifetime {
                                    value: LifetimeId::new(index),
                                })
                            })
                            .ok_or_else(|| "generic ADT lifetime was not interned".to_owned()),
                        GenericArgKind::Const(value) => const_generic_value(&value)
                            .map(|value| Some(GenericArgument::Const { value })),
                    })
                    .collect::<Result<Option<Vec<_>>, String>>()?;
                let Some(arguments) = arguments else {
                    continue;
                };
                let id = TypeId::new(types.len());
                types.push(Ty::Nominal {
                    arguments,
                    name: adt.name,
                });
                nominals.push(NominalEntry {
                    definition,
                    ty: *ty,
                    id,
                });
            }
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
                    .map(|element| Ty::Slice { element }),
                    TyKind::RigidTy(RigidTy::Str) => Some(Ty::String),
                    // A closure is the struct of its upvars at its parent's
                    // arguments, its own binders inferred.
                    TyKind::RigidTy(RigidTy::Closure(definition, arguments)) => {
                        let closure = closures
                            .iter()
                            .find(|closure| closure.definition == definition)
                            .ok_or_else(|| {
                                format!("closure `{}` was not interned", definition.name())
                            })?;
                        let mut mapped = Some(Vec::new());
                        for argument in closure_arguments(&arguments)?.0 {
                            let argument = match argument {
                                GenericArgKind::Type(argument) => interned_type_id(
                                    TypeId::new(0),
                                    never,
                                    bool_,
                                    character,
                                    &type_parameters,
                                    &integers,
                                    &nominals,
                                    &composites,
                                    &references,
                                    argument,
                                )
                                .map(|type_id| GenericArgument::TypeArg {
                                    value: TypeUse {
                                        loc: LocId::new(0),
                                        type_id,
                                    },
                                }),
                                GenericArgKind::Lifetime(region) => {
                                    Some(GenericArgument::Lifetime {
                                        value: interned_lifetime(&mut lifetimes, region),
                                    })
                                },
                                GenericArgKind::Const(value) => Some(GenericArgument::Const {
                                    value: const_generic_value(&value)?,
                                }),
                            };
                            match (argument, &mut mapped) {
                                (Some(argument), Some(mapped)) => mapped.push(argument),
                                _ => mapped = None,
                            }
                        }
                        mapped.map(|mut arguments| {
                            for _ in &closure.lifetimes {
                                arguments.push(GenericArgument::Lifetime {
                                    value: interned_lifetime(&mut lifetimes, erased()),
                                });
                            }
                            Ty::Nominal {
                                arguments,
                                name: closure.name,
                            }
                        })
                    },
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
                        lifetimes.push(spec(region.clone()));
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
            let unresolved_nominal = needed_nominals
                .iter()
                .find(|ty| !nominals.iter().any(|entry| entry.ty == **ty));
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
            if unresolved_nominal.is_none()
                && unresolved_composite.is_none()
                && unresolved_reference.is_none()
            {
                break;
            }
            if nominals.len() + composites.len() + references.len() == before {
                return Err(format!(
                    "could not intern nominal/composite/reference type dependencies: nominal={:?}, composite={:?}, reference={:?}",
                    unresolved_nominal.map(|ty| ty.kind()),
                    unresolved_composite.map(|ty| ty.kind()),
                    unresolved_reference.map(|(_, referent, _)| referent.kind())
                ));
            }
        }
        let mut layout = Self {
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
            bound_regions,
        };
        layout.intern_instantiated_fields(adts)?;
        layout.intern_slice_backings(constants)?;
        Ok(layout)
    }

    /// Slice constants borrow an explicitly sized backing value. Sort the
    /// required arrays by their interned element and length so HashMap traversal
    /// order cannot affect the exchange's type table.
    fn intern_slice_backings(&mut self, constants: &EvaluatedConstants) -> Result<(), String> {
        fn collect(value: &EvaluatedConstant, found: &mut Vec<(RustTy, usize)>) {
            let parts = match value {
                EvaluatedConstant::Array { ty, elements } => {
                    if let TyKind::RigidTy(RigidTy::Slice(element)) = ty.kind() {
                        found.push((element, elements.len()));
                    }
                    elements.as_slice()
                },
                EvaluatedConstant::Tuple { elements, .. } => elements.as_slice(),
                EvaluatedConstant::Adt { fields, .. } => fields.as_slice(),
                EvaluatedConstant::Shared { referent, .. } => {
                    std::slice::from_ref(referent.as_ref())
                },
                EvaluatedConstant::Closure { upvars, .. } => upvars.as_slice(),
                EvaluatedConstant::Literal { .. } | EvaluatedConstant::Opaque { .. } => return,
            };
            for part in parts {
                collect(part, found);
            }
        }
        let mut found = Vec::new();
        for value in constants
            .values
            .values()
            .chain(constants.zero_sized_values.values())
        {
            collect(value, &mut found);
        }
        let arrays = found
            .into_iter()
            .map(|(element, length)| Ok((self.id(element)?.index, length)))
            .collect::<Result<BTreeSet<_>, String>>()?;
        for (element, length) in arrays {
            let backing = Ty::Vector {
                element: TypeId::new(element),
                length: Some(ConstValue::Integer {
                    value: Number::from(length),
                }),
            };
            if !self.types.contains(&backing) {
                self.types.push(backing);
            }
        }
        Ok(())
    }

    /// Interns the fields of each nominal whose arguments hold a lifetime of a
    /// signature, which a place through it reads. Rustc Public erases the regions of
    /// a type it builds, so `FieldDef::ty_with_args` cannot state them.
    fn intern_instantiated_fields(&mut self, adts: &[AdtNames]) -> Result<(), String> {
        let mut index = 0;
        while index < self.types.len() {
            let current = self.types[index].clone();
            index += 1;
            let Ty::Nominal { arguments, name } = current else {
                continue;
            };
            if !arguments
                .iter()
                .any(|argument| self.holds_declared_lifetime(argument))
            {
                continue;
            }
            let Some(adt) = adts.iter().find(|adt| adt.name == name && !adt.opaque) else {
                continue;
            };
            for variant in adt.definition.variants() {
                for field in variant.fields() {
                    let declared = self.id(field.ty())?;
                    self.instantiate_field(declared, &arguments);
                }
            }
        }
        Ok(())
    }

    fn holds_declared_lifetime(&self, argument: &GenericArgument) -> bool {
        let erased = self.interned_lifetime(&erased());
        let mut pending = vec![];
        match argument {
            GenericArgument::Lifetime { value } => return Some(*value) != erased,
            GenericArgument::TypeArg { value } => pending.push(value.type_id),
            GenericArgument::Const { .. } | GenericArgument::Evidence { .. } => {},
        }
        while let Some(id) = pending.pop() {
            match &self.types[id.index] {
                Ty::Reference { value } => {
                    if Some(value.lifetime) != erased {
                        return true;
                    }
                    pending.push(value.referent);
                },
                Ty::Tuple { elements } => pending.extend(elements),
                Ty::Vector { element, .. } | Ty::Slice { element } => pending.push(*element),
                Ty::Nominal { arguments, .. } => {
                    if arguments
                        .iter()
                        .any(|argument| self.holds_declared_lifetime(argument))
                    {
                        return true;
                    }
                },
                Ty::Unit
                | Ty::Never
                | Ty::Bool
                | Ty::Character
                | Ty::String
                | Ty::Integer { .. }
                | Ty::TypeParameter { .. }
                | Ty::Function { .. } => {},
            }
        }
        false
    }

    /// A declared field type at a nominal's arguments, interned as validation reads it.
    fn instantiate_field(&mut self, declared: TypeId, arguments: &[GenericArgument]) -> TypeId {
        let instantiated = match self.types[declared.index].clone() {
            Ty::TypeParameter { index } => {
                return match arguments.get(index) {
                    Some(GenericArgument::TypeArg { value }) => value.type_id,
                    _ => declared,
                };
            },
            Ty::Reference { value } => Ty::Reference {
                value: ReferenceType {
                    lifetime: match self.lifetimes[value.lifetime.index].kind {
                        LifetimeKind::Parameter { index } => match arguments.get(index) {
                            Some(GenericArgument::Lifetime { value }) => *value,
                            _ => value.lifetime,
                        },
                        LifetimeKind::Static | LifetimeKind::Inference | LifetimeKind::Local => {
                            value.lifetime
                        },
                    },
                    referent: self.instantiate_field(value.referent, arguments),
                    ..value
                },
            },
            Ty::Function {
                abilities,
                arguments: parameters,
                result,
            } => Ty::Function {
                abilities,
                arguments: parameters
                    .into_iter()
                    .map(|ty| self.instantiate_field(ty, arguments))
                    .collect(),
                result: self.instantiate_field(result, arguments),
            },
            Ty::Tuple { elements } => Ty::Tuple {
                elements: elements
                    .into_iter()
                    .map(|element| self.instantiate_field(element, arguments))
                    .collect(),
            },
            Ty::Vector { element, length } => Ty::Vector {
                element: self.instantiate_field(element, arguments),
                length,
            },
            Ty::Slice { element } => Ty::Slice {
                element: self.instantiate_field(element, arguments),
            },
            Ty::Nominal {
                arguments: nominal,
                name,
            } => Ty::Nominal {
                arguments: nominal
                    .into_iter()
                    .map(|argument| match argument {
                        GenericArgument::TypeArg { value } => GenericArgument::TypeArg {
                            value: TypeUse {
                                loc: value.loc,
                                type_id: self.instantiate_field(value.type_id, arguments),
                            },
                        },
                        GenericArgument::Lifetime { value } => GenericArgument::Lifetime {
                            value: match self.lifetimes[value.index].kind {
                                LifetimeKind::Parameter { index } => match arguments.get(index) {
                                    Some(GenericArgument::Lifetime { value }) => *value,
                                    _ => value,
                                },
                                LifetimeKind::Static
                                | LifetimeKind::Inference
                                | LifetimeKind::Local => value,
                            },
                        },
                        argument @ (GenericArgument::Const { .. }
                        | GenericArgument::Evidence { .. }) => argument,
                    })
                    .collect(),
                name,
            },
            Ty::Unit | Ty::Never | Ty::Bool | Ty::Character | Ty::String | Ty::Integer { .. } => {
                return declared
            },
        };
        match self.types.iter().position(|ty| *ty == instantiated) {
            Some(index) => TypeId::new(index),
            None => {
                self.types.push(instantiated);
                TypeId::new(self.types.len() - 1)
            },
        }
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
                RigidTy::Array(_, _)
                | RigidTy::Slice(_)
                | RigidTy::Str
                | RigidTy::FnPtr(_)
                | RigidTy::Closure(..),
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

    /// The type of a dependency's signature, in which an associated type is the
    /// type parameter of its binder. Such a type depends on the binder's index,
    /// so it has no identity as a Rust type.
    fn signature_type_id(
        &mut self,
        ty: RustTy,
        associated_types: &[AssociatedType],
        adts: &[AdtNames],
    ) -> Result<TypeId, String> {
        self.rebuilt_type_id(ty, associated_types, adts, &mut |region| region, &|ty| {
            projection(ty).is_none()
        })
    }

    /// A type built here rather than looked up, with its regions as `region` maps
    /// them, since a Rust type cannot carry a region it was not given by rustc: a part
    /// `settled` takes has its identity as a Rust type.
    fn rebuilt_type_id(
        &mut self,
        ty: RustTy,
        associated_types: &[AssociatedType],
        adts: &[AdtNames],
        region: &mut dyn FnMut(Region) -> Region,
        settled: &dyn Fn(RustTy) -> bool,
    ) -> Result<TypeId, String> {
        if settled(ty) {
            return self.id(ty);
        }
        let mapped = match ty.kind() {
            TyKind::Alias(..) => {
                let associated = associated_types
                    .iter()
                    .find(|associated| associated.alias == ty)
                    .ok_or_else(|| format!("the type `{ty}` is not declared"))?;
                return self
                    .type_parameters
                    .iter()
                    .find(|(index, _)| *index == associated.index)
                    .map(|(_, id)| *id)
                    .ok_or_else(|| format!("associated type `{ty}` was not interned"));
            },
            TyKind::RigidTy(RigidTy::Ref(own, referent, mutability)) => {
                if mutability == Mutability::Not
                    && let Some(associated) = associated_types.iter().find(|associated| {
                        associated.shared_reference && associated.alias == referent
                    })
                {
                    return self
                        .type_parameters
                        .iter()
                        .find(|(index, _)| *index == associated.index)
                        .map(|(_, id)| *id)
                        .ok_or_else(|| format!("associated reference `{ty}` was not interned"));
                }
                let lifetime = self.lifetime_id(region(own));
                let referent =
                    self.rebuilt_type_id(referent, associated_types, adts, region, settled)?;
                Ty::Reference {
                    value: ReferenceType {
                        kind: if mutability == Mutability::Mut {
                            ReferenceKind::Mutable
                        } else {
                            ReferenceKind::Shared
                        },
                        lifetime,
                        profile: Profile::Rust,
                        referent,
                    },
                }
            },
            TyKind::RigidTy(RigidTy::Tuple(elements)) => Ty::Tuple {
                elements: elements
                    .into_iter()
                    .map(|element| {
                        self.rebuilt_type_id(element, associated_types, adts, region, settled)
                    })
                    .collect::<Result<_, _>>()?,
            },
            TyKind::RigidTy(RigidTy::Slice(element)) => Ty::Slice {
                element: self.rebuilt_type_id(element, associated_types, adts, region, settled)?,
            },
            TyKind::RigidTy(RigidTy::Array(element, length)) => Ty::Vector {
                element: self.rebuilt_type_id(element, associated_types, adts, region, settled)?,
                length: Some(ConstValue::Integer {
                    value: Number::from(
                        length
                            .eval_target_usize()
                            .map_err(|error| format!("evaluate fixed array length: {error:?}"))?,
                    ),
                }),
            },
            TyKind::RigidTy(RigidTy::FnPtr(signature)) => {
                let signature = supported_function_pointer_signature(&signature)?;
                let mut types = signature
                    .inputs_and_output
                    .iter()
                    .map(|type_| {
                        self.rebuilt_type_id(*type_, associated_types, adts, region, settled)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let result = types
                    .pop()
                    .ok_or_else(|| "function pointer has no result type".to_owned())?;
                Ty::Function {
                    abilities: vec![Ability::Copy, Ability::Drop],
                    arguments: types,
                    result,
                }
            },
            TyKind::RigidTy(RigidTy::Adt(definition, arguments)) => {
                let name = adts
                    .iter()
                    .find(|adt| adt.definition == definition)
                    .ok_or_else(|| format!("ADT `{}` was not interned", definition.name()))?
                    .name;
                let arguments = arguments
                    .0
                    .into_iter()
                    .map(|argument| {
                        Ok(match argument {
                            GenericArgKind::Type(argument) => GenericArgument::TypeArg {
                                value: TypeUse {
                                    loc: LocId::new(0),
                                    type_id: self.rebuilt_type_id(
                                        argument,
                                        associated_types,
                                        adts,
                                        region,
                                        settled,
                                    )?,
                                },
                            },
                            GenericArgKind::Lifetime(own) => GenericArgument::Lifetime {
                                value: self.lifetime_id(region(own)),
                            },
                            GenericArgKind::Const(value) => GenericArgument::Const {
                                value: const_generic_value(&value)?,
                            },
                        })
                    })
                    .collect::<Result<_, String>>()?;
                Ty::Nominal { arguments, name }
            },
            kind => return Err(format!("unsupported Rust signature type: {kind:?}")),
        };
        Ok(match self.types.iter().position(|known| *known == mapped) {
            Some(index) => TypeId::new(index),
            None => {
                self.types.push(mapped);
                TypeId::new(self.types.len() - 1)
            },
        })
    }

    fn interned_lifetime(&self, region: &Region) -> Option<LifetimeId> {
        self.lifetimes
            .iter()
            .position(|entry| entry.region == *region)
            .map(LifetimeId::new)
    }

    fn lifetime_id(&mut self, region: Region) -> LifetimeId {
        match self
            .lifetimes
            .iter()
            .position(|entry| entry.region == region)
        {
            Some(index) => LifetimeId::new(index),
            None => {
                let spec = self
                    .bound_regions
                    .iter()
                    .find(|bound| bound.region == region)
                    .cloned()
                    .unwrap_or_else(|| lifetime_spec(region));
                self.lifetimes.push(spec);
                LifetimeId::new(self.lifetimes.len() - 1)
            },
        }
    }

    fn integer_id(&self, kind: IntegerType) -> Result<TypeId, String> {
        self.integers
            .iter()
            .find(|entry| entry.kind == kind)
            .map(|entry| entry.id)
            .ok_or_else(|| format!("{kind:?} type was not interned"))
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

fn interned_lifetime(lifetimes: &mut Vec<LifetimeSpec>, region: Region) -> LifetimeId {
    match lifetimes.iter().position(|entry| entry.region == region) {
        Some(index) => LifetimeId::new(index),
        None => {
            lifetimes.push(lifetime_spec(region));
            LifetimeId::new(lifetimes.len() - 1)
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
            | RigidTy::FnPtr(_)
            | RigidTy::Closure(..),
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
        TyKind::RigidTy(RigidTy::Closure(_, arguments)) => {
            for part in closure_parts(&arguments) {
                observe_type(
                    part,
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
        // A bound region without a binder relates nothing, or is a function
        // pointer's.
        RegionKind::ReBound(..) | RegionKind::RePlaceholder(_) | RegionKind::ReErased => {
            (LifetimeKind::Inference, None)
        },
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
    crate_files: HashSet<String>,
    files: Vec<SourceFile>,
    file_contents: Vec<String>,
    locations: Vec<Location>,
    file_ids: BTreeMap<String, FileId>,
    location_ids: BTreeMap<(usize, usize, usize), LocId>,
    generated_location_ids: BTreeMap<usize, LocId>,
    expansion_location_ids: BTreeMap<(usize, usize, usize), LocId>,
    dependency_location_ids: BTreeMap<String, LocId>,
}

impl SourceTables {
    /// The location of a declaration: its span in the local crate, or, in a
    /// dependency crate, whose sources the unit does not record, a generated
    /// location naming the definition.
    fn declaration(
        &mut self,
        namespace: NamespaceId,
        definition: &str,
        span: Span,
    ) -> Result<LocId, String> {
        if namespace.index == 0 {
            self.intern(span)
        } else {
            Ok(self.dependency(definition))
        }
    }

    /// The generated location of a definition in a dependency crate.
    fn dependency(&mut self, definition: &str) -> LocId {
        let description = format!("rustc dependency `{definition}`");
        if let Some(id) = self.dependency_location_ids.get(&description) {
            return *id;
        }
        let id = LocId::new(self.locations.len());
        self.locations.push(Location {
            expansion: vec![],
            generated_by: Some(description.clone()),
            parent: None,
            primary: None,
            related: vec![],
        });
        self.dependency_location_ids.insert(description, id);
        id
    }

    fn file_id(&mut self, filename: String) -> Result<FileId, String> {
        if let Some(id) = self.file_ids.get(&filename) {
            return Ok(*id);
        }
        let source = fs::read_to_string(&filename)
            .map_err(|error| format!("read Rust source `{filename}`: {error}"))?;
        let id = FileId::new(self.files.len());
        self.files.push(SourceFile {
            content_hash: String::new(),
            name: filename.clone(),
        });
        self.file_contents.push(source);
        self.file_ids.insert(filename, id);
        Ok(id)
    }

    fn in_crate(&self, span: Span) -> bool {
        self.crate_files
            .contains(&polyfill::source_callsite(span).get_filename())
    }

    /// The location of what a macro of another crate expanded to, whose own span
    /// is not stated: the crate's source it is reached after. It has no primary
    /// range, since it is not where the node was written.
    fn expansion(&mut self, reached_after: Span) -> Result<LocId, String> {
        let reached_after = polyfill::source_callsite(reached_after);
        let file = self.file_id(reached_after.get_filename())?;
        let source = &self.file_contents[file.index];
        let lines = reached_after.get_lines();
        let start_byte = byte_offset(source, lines.start_line, lines.start_col)?;
        let end_byte = byte_offset(source, lines.end_line, lines.end_col)?;
        let key = (file.index, start_byte, end_byte);
        if let Some(id) = self.expansion_location_ids.get(&key) {
            return Ok(*id);
        }
        let id = LocId::new(self.locations.len());
        self.locations.push(Location {
            expansion: vec![SourceRange {
                end_byte,
                file,
                start_byte,
            }],
            generated_by: Some("macro expansion".to_owned()),
            parent: None,
            primary: None,
            related: vec![],
        });
        self.expansion_location_ids.insert(key, id);
        Ok(id)
    }

    fn intern(&mut self, span: Span) -> Result<LocId, String> {
        let span = polyfill::source_callsite(span);
        let file_id = self.file_id(span.get_filename())?;
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
    } else {
        None
    }
}

fn is_string_length(name: &str) -> bool {
    name == "core::str::<impl str>::len"
}

/// The immediate dominator of each block of a body; none for its entry and a block
/// it does not reach. Cooper, Harvey and Kennedy's iteration over reverse postorder.
fn immediate_dominators(body: &Body) -> Vec<Option<usize>> {
    let count = body.blocks.len();
    let successors = |block: usize| body.blocks[block].terminator.successors();
    let mut postorder = Vec::with_capacity(count);
    let mut visited = vec![false; count];
    let mut stack = vec![(0, successors(0).into_iter())];
    visited[0] = true;
    while let Some((block, pending)) = stack.last_mut() {
        match pending.next() {
            Some(next) if !visited[next] => {
                visited[next] = true;
                stack.push((next, successors(next).into_iter()));
            },
            Some(_) => {},
            None => {
                postorder.push(*block);
                stack.pop();
            },
        }
    }
    let mut position = vec![0; count];
    for (index, block) in postorder.iter().enumerate() {
        position[*block] = index;
    }
    let mut predecessors = vec![Vec::new(); count];
    for block in &postorder {
        for next in successors(*block) {
            predecessors[next].push(*block);
        }
    }
    let mut dominators: Vec<Option<usize>> = vec![None; count];
    dominators[0] = Some(0);
    let mut changed = true;
    while changed {
        changed = false;
        for block in postorder.iter().rev().filter(|block| **block != 0) {
            let mut new = None;
            for predecessor in &predecessors[*block] {
                if dominators[*predecessor].is_none() {
                    continue;
                }
                let Some(mut left) = new else {
                    new = Some(*predecessor);
                    continue;
                };
                let mut right = *predecessor;
                while left != right {
                    while position[left] < position[right] {
                        left = dominators[left].expect("processed in reverse postorder");
                    }
                    while position[right] < position[left] {
                        right = dominators[right].expect("processed in reverse postorder");
                    }
                }
                new = Some(left);
            }
            if dominators[*block] != new {
                dominators[*block] = new;
                changed = true;
            }
        }
    }
    dominators[0] = None;
    dominators
}

/// How the paths from a block end, as the import's collapse reads them.
#[derive(Clone, Copy, PartialEq)]
enum RegionEnding {
    Unknown,
    /// Every path fails or cannot happen, and none holds a call.
    NoCall,
    /// The calls that do not return are all in macros of other crates.
    MacroCalls,
    /// A call that does not return is in the crate.
    CrateCall,
}

/// The blocks of the regions the import collapses to a panic and only a macro of
/// another crate raises. The collapse keeps the location of a region's entry, which
/// may be a format argument's, so a region's blocks are located here.
fn macro_panic_blocks(body: &Body, crate_files: &HashSet<String>) -> Vec<bool> {
    let own = |block: &BasicBlock| match &block.terminator.kind {
        TerminatorKind::Call { target: None, .. } => {
            if crate_files.contains(&block.terminator.source_info.span.get_filename()) {
                RegionEnding::CrateCall
            } else {
                RegionEnding::MacroCalls
            }
        },
        TerminatorKind::Abort | TerminatorKind::Unreachable => RegionEnding::NoCall,
        _ => RegionEnding::Unknown,
    };
    let mut endings = body.blocks.iter().map(own).collect::<Vec<_>>();
    for _ in 0..=body.blocks.len() {
        let next = body
            .blocks
            .iter()
            .zip(&endings)
            .map(|(block, current)| {
                let flows = matches!(
                    block.terminator.kind,
                    TerminatorKind::Goto { .. }
                        | TerminatorKind::SwitchInt { .. }
                        | TerminatorKind::Call {
                            target: Some(_),
                            ..
                        }
                        | TerminatorKind::Drop { .. }
                        | TerminatorKind::Assert { .. }
                );
                let targets = block.terminator.successors();
                if !flows || targets.is_empty() {
                    return *current;
                }
                let of = |ending| targets.iter().any(|target| endings[*target] == ending);
                if of(RegionEnding::Unknown) {
                    *current
                } else if of(RegionEnding::CrateCall) {
                    RegionEnding::CrateCall
                } else if of(RegionEnding::MacroCalls) {
                    RegionEnding::MacroCalls
                } else {
                    RegionEnding::NoCall
                }
            })
            .collect::<Vec<_>>();
        if next == endings {
            break;
        }
        endings = next;
    }
    endings
        .into_iter()
        .map(|ending| ending == RegionEnding::MacroCalls)
        .collect()
}

struct MappedFunction {
    result_type: TypeId,
    result_loc: LocId,
    locals: Vec<LocalDecl>,
    parameters: Vec<Parameter>,
    body: RawBody,
}

/// What a function's mapper reads of `Copy`.
struct CopyFacts<'a> {
    /// The type parameters its body shows to be `Copy`.
    parameters: &'a BTreeSet<u32>,
    impls: &'a mut AdtImpls,
}

struct FunctionMapper<'a> {
    identity: &'a str,
    item: CrateItem,
    /// A closure's body has `self` as its first parameter.
    is_closure: bool,
    /// The locals holding a zero-sized value no statement assigned.
    zero_sized_locals: Vec<usize>,
    body: &'a Body,
    signature: &'a DeclaredSignature,
    locals: Vec<MirLocalDecl>,
    types: &'a TypeLayout,
    name_ids: &'a BTreeMap<String, QualifiedRef>,
    bound_lifetimes: &'a BTreeMap<String, usize>,
    dependency_functions: &'a [DependencyFunction],
    opaque_functions: &'a BTreeSet<String>,
    method_evidence: &'a BTreeMap<String, Vec<MethodEvidence>>,
    constants: &'a EvaluatedConstants,
    adts: &'a [AdtNames],
    closures: &'a [ClosureNames],
    copies: CopyFacts<'a>,
    sources: &'a mut SourceTables,
    arenas: &'a mut NamespaceArenas,
    rust_to_vir: Vec<LocalId>,
    unsize_folds: HashMap<usize, (BorrowKind, MirPlace)>,
    constant_unsize_folds: HashMap<usize, EvaluatedConstant>,
    call_tuples: HashMap<usize, Vec<Operand>>,
    box_folds: BoxFolds,
    dominators: Vec<Option<usize>>,
    macro_panics: Vec<bool>,
    /// Why the body cannot be mapped at all, which refuses the function.
    body_failure: Option<String>,
}

/// What a body can call beside its own crate's mapped functions.
struct Callees<'a> {
    dependencies: &'a [DependencyFunction],
    /// The crate's functions whose signatures the unit cannot state.
    opaque: &'a BTreeSet<String>,
    method_evidence: &'a BTreeMap<String, Vec<MethodEvidence>>,
}

impl<'a> FunctionMapper<'a> {
    fn evidence_operand(&mut self, index: usize, loc: LocId) -> Result<ExprId, String> {
        let evidence = &self.method_evidence[self.identity][index];
        let type_id = self.types.method_evidence_type(evidence)?;
        let place = self
            .arenas
            .place(LocalId::new(self.body.arg_locals().len() + index));
        Ok(self.arenas.expression(loc, type_id, ExprKind::Operation {
            operation: Operation::Copy { place },
            arguments: vec![],
            instantiations: vec![],
            surface: None,
        }))
    }

    fn evidence_for(&mut self, evidence: &MethodEvidence, loc: LocId) -> Result<ExprId, String> {
        let receiver = evidence.receiver;
        if let Some(index) = self.method_evidence.get(self.identity).and_then(|methods| {
            methods.iter().position(|entry| {
                entry.method == evidence.method && entry.arguments == evidence.arguments
            })
        }) {
            return self.evidence_operand(index, loc);
        }
        // Derive a weaker callable dictionary with a checked adapter body.
        // Consuming adapters require scoped evidence for their borrowed receiver.
        if evidence.tupled
            && let Some(index) = self
                .method_evidence
                .get(self.identity)
                .and_then(|methods| methods.iter().position(|entry| entry.forwards_to(evidence)))
        {
            let prefix = if evidence.receiver_kind == EvidenceReceiver::Owned {
                "__borrowed_to_fnonce"
            } else {
                "__fn_to_fnmut"
            };
            let key = format!("{}::{prefix}_{index}", self.identity);
            let function = self
                .name_ids
                .get(&key)
                .cloned()
                .ok_or("missing generic callable receiver adapter")?;
            let dictionary = self.evidence_operand(index, loc)?;
            let arguments = match self.item.ty().kind() {
                TyKind::RigidTy(RigidTy::FnDef(_, arguments)) => arguments,
                TyKind::RigidTy(RigidTy::Closure(_, arguments)) => {
                    GenericArgs(closure_arguments(&arguments)?.0)
                },
                _ => return Err("callable adapter has no generic environment".to_owned()),
            };
            let mut instantiations = self.types.generic_arguments(&arguments, loc)?;
            instantiations.extend(self.bound_lifetime_arguments(self.identity)?);
            let mask = 1u64
                .checked_shl((1 + evidence.parameters.len()) as u32)
                .ok_or("too many callable adapter parameters")?;
            return Ok(self.arenas.expression(
                loc,
                self.types.method_evidence_type(evidence)?,
                ExprKind::Operation {
                    operation: Operation::Call {
                        kind: CallKind::Closure { function, mask },
                    },
                    arguments: vec![dictionary],
                    instantiations,
                    surface: None,
                },
            ));
        }
        let (resolved, arguments) = if evidence.tupled
            && evidence.receiver_kind == EvidenceReceiver::Owned
        {
            let TyKind::RigidTy(RigidTy::Closure(closure, arguments)) = receiver.kind() else {
                return Err("consuming callable evidence requires a concrete closure".to_owned());
            };
            (FnDef(closure.def_id()), arguments)
        } else {
            resolve_callee(self.item.0, evidence.method, &evidence.arguments)?
        };
        let arguments = if evidence.tupled {
            let TyKind::RigidTy(RigidTy::Closure(closure, closure_args)) = receiver.kind() else {
                return Err("callable evidence requires a concrete closure".to_owned());
            };
            if resolved.def_id() != closure.def_id() {
                return Err("callable evidence did not resolve to the closure body".to_owned());
            }
            GenericArgs(closure_arguments(&closure_args)?.0)
        } else {
            arguments
        };
        let name = resolved.name();
        let mut captures = Vec::new();
        let mut mask = 0u64;
        for (index, nested) in self
            .method_evidence
            .get(&name)
            .cloned()
            .unwrap_or_default()
            .iter()
            .enumerate()
        {
            captures.push(self.evidence_for(&nested.instantiate(self.item.0, &arguments), loc)?);
            mask |= 1u64
                .checked_shl((1 + evidence.parameters.len() + index) as u32)
                .ok_or("too many captured method dictionaries")?;
        }
        let target_name = if evidence.tupled
            && evidence.receiver_kind == EvidenceReceiver::Owned
            && self
                .name_ids
                .contains_key(&format!("{name}::__fnonce_adapter"))
        {
            format!("{name}::__fnonce_adapter")
        } else if evidence.tupled
            && evidence.receiver_kind == EvidenceReceiver::Mutable
            && self
                .name_ids
                .contains_key(&format!("{name}::__fnmut_adapter"))
        {
            format!("{name}::__fnmut_adapter")
        } else {
            name.clone()
        };
        let function = self
            .name_ids
            .get(&target_name)
            .cloned()
            .ok_or_else(|| format!("method implementation `{target_name}` is not mapped"))?;
        let mut instantiations = self.types.generic_arguments(&arguments, loc)?;
        instantiations.extend(self.bound_lifetime_arguments(&name)?);
        let type_id = self.types.method_evidence_type(evidence)?;
        Ok(self.arenas.expression(loc, type_id, ExprKind::Operation {
            operation: Operation::Call {
                kind: CallKind::Closure { function, mask },
            },
            arguments: captures,
            instantiations,
            surface: None,
        }))
    }

    fn new(
        identity: &'a str,
        item: CrateItem,
        body: &'a PreparedBody,
        signature: &'a DeclaredSignature,
        types: &'a TypeLayout,
        name_ids: &'a BTreeMap<String, QualifiedRef>,
        bound_lifetimes: &'a BTreeMap<String, usize>,
        callees: Callees<'a>,
        constants: &'a EvaluatedConstants,
        adts: &'a [AdtNames],
        closures: &'a [ClosureNames],
        copies: CopyFacts<'a>,
        sources: &'a mut SourceTables,
        arenas: &'a mut NamespaceArenas,
    ) -> Result<Self, String> {
        let argument_count = body.arg_locals().len();
        let mut rust_to_vir = vec![LocalId::new(0); body.locals().len()];
        rust_to_vir[0] = LocalId::new(argument_count);
        for (rust_local, vir_local) in rust_to_vir
            .iter_mut()
            .enumerate()
            .take(argument_count + 1)
            .skip(1)
        {
            *vir_local = LocalId::new(rust_local - 1);
        }
        for (rust_local, vir_local) in rust_to_vir.iter_mut().enumerate().skip(argument_count + 1) {
            *vir_local = LocalId::new(rust_local);
        }
        let evidence_count = callees.method_evidence.get(identity).map_or(0, Vec::len);
        for local in &mut rust_to_vir {
            if local.index >= argument_count {
                local.index += evidence_count;
            }
        }
        let locals = &body.mapped_locals;
        let (call_tuples, body_failure) =
            match closure_call_tuples(identity, item, body, locals, &sources.crate_files) {
                Ok(call_tuples) => (call_tuples, None),
                Err(reason) => (HashMap::new(), Some(reason)),
            };
        Ok(Self {
            identity,
            item,
            is_closure: closure_of(item).is_some(),
            zero_sized_locals: unassigned_locals(body)
                .into_iter()
                .filter(|local| {
                    constants
                        .zero_sized_values
                        .contains_key(&body.locals()[*local].ty)
                })
                .collect(),
            body,
            signature,
            unsize_folds: unsize_folds(body, locals),
            constant_unsize_folds: constant_unsize_folds(body, locals, constants),
            call_tuples,
            box_folds: body.box_folds.clone(),
            body_failure,
            dominators: immediate_dominators(body),
            macro_panics: macro_panic_blocks(body, &sources.crate_files),
            locals: locals.clone(),
            types,
            name_ids,
            bound_lifetimes,
            dependency_functions: callees.dependencies,
            opaque_functions: callees.opaque,
            method_evidence: callees.method_evidence,
            constants,
            adts,
            closures,
            copies,
            sources,
            arenas,
            rust_to_vir,
        })
    }

    fn map(mut self) -> Result<MappedFunction, String> {
        let (locals, parameters) = self.map_locals()?;
        let blocks = match self.body_failure.take() {
            Some(reason) => Err(reason),
            None => self.map_blocks(),
        };
        let blocks = match blocks {
            Ok(blocks) => blocks,
            Err(reason) => {
                let loc = self.local_location(self.body.span)?;
                let expression = self.opaque_value(reason, loc);
                vec![RawBasicBlock {
                    loc,
                    statements: vec![RawStatement::Execute { expression }],
                    terminator: RawTerminator::Unreachable,
                }]
            },
        };
        self.map_function_result(locals, parameters, blocks)
    }

    fn map_blocks(&mut self) -> Result<Vec<RawBasicBlock>, String> {
        let mut blocks = Vec::with_capacity(self.body.blocks.len());
        let mut entry_statements = self.unused_closure_statements()?;
        let body = self.body;
        for local in self.zero_sized_locals.clone() {
            if self.is_zero_sized_borrow_root(local)
                && !body.blocks.iter().any(|block| {
                    block.statements.iter().any(|statement| {
                    matches!(statement.kind, StatementKind::StorageLive(marked) if marked == local)
                })
                })
            {
                let loc = self.block_location(0, body.blocks[0].terminator.source_info.span)?;
                self.materialize_zero_sized_root(local, loc, &mut entry_statements)?;
            }
        }
        for (index, block) in body.blocks.iter().enumerate() {
            let block_loc = self.block_location(index, block.terminator.source_info.span)?;
            let mut statements = std::mem::take(&mut entry_statements);
            for statement in &block.statements {
                let loc = self.block_location(index, statement.source_info.span)?;
                if let Err(reason) = self.map_statement(statement, loc, &mut statements) {
                    let expression = self.opaque_value(reason, loc);
                    statements.push(RawStatement::Execute { expression });
                }
            }
            let mapped = match &block.terminator.kind {
                TerminatorKind::Call {
                    target: Some(target),
                    ..
                } if self.box_folds.frees.contains(&index) => Ok(RawTerminator::Goto {
                    target: BlockId::new(*target),
                }),
                _ => self.map_terminator(&block.terminator, block_loc),
            };
            let terminator = match mapped {
                Ok(terminator) => terminator,
                Err(reason) => {
                    self.refused_terminator(&block.terminator, block_loc, reason, &mut statements)
                },
            };
            blocks.push(RawBasicBlock {
                loc: block_loc,
                statements,
                terminator,
            });
        }
        Ok(blocks)
    }

    fn map_function_result(
        mut self,
        locals: Vec<LocalDecl>,
        parameters: Vec<Parameter>,
        blocks: Vec<RawBasicBlock>,
    ) -> Result<MappedFunction, String> {
        let result = &self.body.ret_local();
        let declared_result = self
            .signature
            .types
            .last()
            .copied()
            .ok_or_else(|| format!("`{}` has no result type", self.identity))?;
        Ok(MappedFunction {
            result_type: self.types.id(declared_result)?,
            result_loc: self.local_location(result.span)?,
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

    /// A node the unit cannot state, which refuses the function if it survives the
    /// collapse of diverging regions.
    fn opaque_value(&mut self, description: String, loc: LocId) -> ExprId {
        self.arenas
            .expression(loc, self.types.unit, ExprKind::Value {
                source_constant: None,
                value: ConstValue::Opaque { description },
            })
    }

    /// A terminator the unit cannot state, with its successors: the collapse reads a
    /// region's paths through them.
    fn refused_terminator(
        &mut self,
        terminator: &Terminator,
        loc: LocId,
        reason: String,
        statements: &mut Vec<RawStatement>,
    ) -> RawTerminator {
        let opaque = self.opaque_value(reason, loc);
        if let TerminatorKind::Call {
            destination,
            target,
            unwind,
            ..
        } = &terminator.kind
        {
            let destination = match target {
                None => Some(None),
                Some(target) => self.map_place(destination).ok().map(|place| {
                    Some(RawCallDestination {
                        place,
                        target: BlockId::new(*target),
                    })
                }),
            };
            if let Some(destination) = destination {
                return RawTerminator::Call {
                    call: opaque,
                    destination,
                    unwind: map_unwind(unwind),
                };
            }
        }
        statements.push(RawStatement::Execute { expression: opaque });
        match terminator.successors().as_slice() {
            [] => RawTerminator::Unreachable,
            [target] => RawTerminator::Goto {
                target: BlockId::new(*target),
            },
            [first, rest @ ..] => RawTerminator::Switch {
                cases: rest
                    .iter()
                    .enumerate()
                    .map(|(index, target)| {
                        (
                            ConstValue::Integer {
                                value: Number::from(index),
                            },
                            BlockId::new(*target),
                        )
                    })
                    .collect(),
                default_target: BlockId::new(*first),
                scrutinee: opaque,
            },
        }
    }

    fn map_statement(
        &mut self,
        statement: &Statement,
        loc: LocId,
        statements: &mut Vec<RawStatement>,
    ) -> Result<(), String> {
        match &statement.kind {
            kind if self.box_folds.folds_statement(kind) => {},
            // The coercion it folds into is the borrow. Its backing root is
            // initialized once at StorageLive (or entry when MIR omits it).
            StatementKind::Assign(destination, Rvalue::Ref(_, _, _))
                if destination.projection.is_empty()
                    && self.unsize_folds.contains_key(&destination.local) => {},
            // The cast materializes the same static backing, so the single-use
            // reference temporary must not evaluate that constant a second time.
            StatementKind::Assign(destination, Rvalue::Use(Operand::Constant(_), _))
                if destination.projection.is_empty()
                    && self.constant_unsize_folds.contains_key(&destination.local) => {},
            // The closure call reads the elements.
            StatementKind::Assign(destination, Rvalue::Aggregate(AggregateKind::Tuple, _))
                if destination.projection.is_empty()
                    && self.call_tuples.contains_key(&destination.local) => {},
            StatementKind::Assign(destination, rvalue) => {
                let value = self.map_rvalue(rvalue, loc)?;
                self.mark_reference_copy(destination, rvalue, value);
                let place = self.map_place(destination)?;
                let expression = self
                    .arenas
                    .expression(loc, self.types.unit, ExprKind::Assign { place, value });
                statements.push(RawStatement::Execute { expression });
            },
            StatementKind::StorageLive(local) => {
                statements.push(RawStatement::StorageLive {
                    local_id: self.local_id(*local)?,
                });
                if self.is_zero_sized_borrow_root(*local) {
                    self.materialize_zero_sized_root(*local, loc, statements)?;
                }
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
                let place_type = place
                    .ty(&self.locals)
                    .map_err(|error| format!("query set-discriminant place type: {error:?}"))?;
                let TyKind::RigidTy(RigidTy::Adt(definition, _)) = place_type.kind() else {
                    return Err(format!(
                        "cannot set the discriminant of MIR place type {place_type:?}"
                    ));
                };
                let adt = self
                    .adts
                    .iter()
                    .find(|entry| entry.definition == definition)
                    .ok_or_else(|| format!("enum `{}` was not interned", definition.name()))?;
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
                    .ty(&self.locals)
                    .map_err(|error| format!("query ascribed place type: {error:?}"))?)?;
                statements.push(RawStatement::AscribeUserType {
                    place: self.map_place(place)?,
                    r#type: TypeUse { loc, type_id },
                });
            },
            StatementKind::Nop | StatementKind::Coverage(_) | StatementKind::ConstEvalCounter => {},
            kind => return Err(format!("unsupported MIR statement: {}", variant_name(kind))),
        }
        Ok(())
    }

    /// The location of a node of a block: its span in the crate, or, in a macro of
    /// another crate or a panic region only such a macro raises, what it is reached
    /// after: the nearest dominator outside the region with a terminator in the
    /// crate, else the body.
    fn block_location(&mut self, block: usize, span: Span) -> Result<LocId, String> {
        if self.sources.in_crate(span) {
            return self.sources.intern(span);
        }
        let mut current = self.dominators[block];
        while let Some(dominator) = current {
            let terminator = self.body.blocks[dominator].terminator.source_info.span;
            if !self.macro_panics[dominator] && self.sources.in_crate(terminator) {
                return self.sources.expansion(terminator);
            }
            current = self.dominators[dominator];
        }
        self.sources.expansion(self.body.span)
    }

    /// The location of a local, which a macro of another crate may have introduced.
    fn local_location(&mut self, span: Span) -> Result<LocId, String> {
        if self.sources.in_crate(span) {
            self.sources.intern(span)
        } else {
            self.sources.expansion(self.body.span)
        }
    }

    fn map_locals(&mut self) -> Result<(Vec<LocalDecl>, Vec<Parameter>), String> {
        let argument_count = self.body.arg_locals().len();
        let rust_order = (1..=argument_count)
            .chain(std::iter::once(0))
            .chain(argument_count + 1..self.locals.len());
        let mut locals = Vec::with_capacity(self.locals.len());
        for rust_local in rust_order {
            let id = self.rust_to_vir[rust_local];
            let loc = self.local_location(self.locals[rust_local].span)?;
            let declaration = &self.locals[rust_local];
            // A parameter's local has the signature's type, the others stay erased.
            let declared_type = match rust_local {
                1.. if rust_local <= argument_count => self.signature.types[rust_local - 1],
                _ => declaration.ty,
            };
            let type_id = self.types.id(declared_type)?;
            locals.push(LocalDecl {
                id,
                loc,
                // MIR return and temporary locals are storage cells assigned by
                // the body even when their source binding was not `mut`.
                mutable: rust_local == 0
                    || rust_local > argument_count
                    || if self.is_closure && rust_local == 1 {
                        local_written(self.body, rust_local)
                    } else {
                        declaration.mutability == Mutability::Mut
                    },
                name: self.local_name(rust_local),
                type_use: TypeUse { loc, type_id },
            });
        }
        let evidence = self
            .method_evidence
            .get(self.identity)
            .cloned()
            .unwrap_or_default();
        for (index, evidence) in evidence.iter().enumerate() {
            if locals.iter().any(|local| local.name == evidence.name) {
                return Err(format!(
                    "method evidence name `{}` conflicts with a local",
                    evidence.name
                ));
            }
            locals.insert(argument_count + index, LocalDecl {
                id: LocalId::new(argument_count + index),
                mutable: false,
                loc: self.local_location(self.body.span)?,
                name: evidence.name.clone(),
                type_use: TypeUse {
                    loc: self.local_location(self.body.span)?,
                    type_id: self.types.method_evidence_type(evidence)?,
                },
            });
        }
        let parameters = locals[..argument_count + evidence.len()]
            .iter()
            .map(|local| Parameter {
                mutable: local.mutable,
                name: local.name.clone(),
                type_use: local.type_use.clone(),
            })
            .collect();
        Ok((locals, parameters))
    }

    /// What a use of a function states for its bound regions, which MIR's
    /// generic arguments do not name: a lifetime the borrow analysis infers.
    fn bound_lifetime_arguments(&self, function: &str) -> Result<Vec<GenericArgument>, String> {
        let count = self.bound_lifetimes.get(function).copied().unwrap_or(0);
        if count == 0 {
            return Ok(vec![]);
        }
        let value = self.types.interned_lifetime(&erased()).ok_or_else(|| {
            format!("`{function}` has lifetime binders, but no erased lifetime was interned")
        })?;
        Ok(vec![GenericArgument::Lifetime { value }; count])
    }

    fn local_name(&self, rust_local: usize) -> String {
        if self.is_closure && rust_local == 1 {
            return "self".to_owned();
        }
        self.body
            .var_debug_info
            .iter()
            .find_map(|info| (info.local() == Some(rust_local)).then(|| info.name.clone()))
            .unwrap_or_else(|| format!("_{rust_local}"))
    }

    fn local_id(&self, rust_local: usize) -> Result<LocalId, String> {
        self.rust_to_vir
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
                    .ty(&self.locals)
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
                    .ty(&self.locals)
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
                let result_type = self.types.id(rvalue.ty(&self.locals).map_err(|error| {
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
                    .ty(&self.locals)
                    .map_err(|error| format!("query unary operation result type: {error:?}"))?)?;
                if *operation == UnOp::PtrMetadata {
                    let operand_type = operand.ty(&self.locals).map_err(|error| {
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
                let kind = match (operation, operand.ty(&self.locals)) {
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
                    .ty(&self.locals)
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
                let function = self.name_ids.get(&callee_name).cloned().ok_or_else(|| {
                    format!(
                        "function-pointer target `{callee_name}` is not a mapped local function"
                    )
                })?;
                let result_type = self.types.id(*target)?;
                let instantiations = self.bound_lifetime_arguments(&callee_name)?;
                Ok(self
                    .arenas
                    .expression(loc, result_type, ExprKind::Operation {
                        arguments: vec![],
                        instantiations,
                        operation: Operation::Call {
                            // A function pointer captures nothing.
                            kind: CallKind::Closure { function, mask: 0 },
                        },
                        surface: None,
                    }))
            },
            Rvalue::Cast(
                CastKind::PointerCoercion(PointerCoercion::Unsize),
                Operand::Copy(operand) | Operand::Move(operand),
                target,
            ) if is_array_to_slice(
                operand
                    .ty(&self.locals)
                    .map_err(|error| format!("query unsizing source type: {error:?}"))?,
                *target,
            ) =>
            {
                if operand.projection.is_empty()
                    && let Some(constant) = self.constant_unsize_folds.get(&operand.local).cloned()
                {
                    return self.constant_expression(&constant, loc);
                }
                let (kind, base) = match self.unsize_folds.get(&operand.local).cloned() {
                    Some((kind, place)) if operand.projection.is_empty() => {
                        (kind, self.map_place(&place)?)
                    },
                    _ => {
                        let TyKind::RigidTy(RigidTy::Ref(_, _, mutability)) = target.kind() else {
                            return Err(format!("unsizing to a non-reference {target:?}"));
                        };
                        let kind = match mutability {
                            Mutability::Not => BorrowKind::Immutable,
                            Mutability::Mut => BorrowKind::Mutable,
                        };
                        let reference = self.map_place(operand)?;
                        let base = PlaceId::new(self.arenas.places.len());
                        self.arenas.places.push(Place::Deref { base: reference });
                        (kind, base)
                    },
                };
                let place = PlaceId::new(self.arenas.places.len());
                self.arenas.places.push(Place::Unsize { base });
                let result_type = self.types.id(*target)?;
                Ok(self
                    .arenas
                    .expression(loc, result_type, ExprKind::Operation {
                        arguments: vec![],
                        instantiations: vec![],
                        operation: Operation::Borrow { kind, place },
                        surface: None,
                    }))
            },
            Rvalue::Cast(CastKind::IntToInt, operand, target) => {
                let source = operand
                    .ty(&self.locals)
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
            Rvalue::AddressOf(RawPtrKind::FakeForPtrMetadata, place) => {
                // Only a pointer local `mapped_locals` types as a shared
                // borrow reaches this: it is that borrow.
                let referent = place
                    .ty(&self.locals)
                    .map_err(|error| format!("query metadata borrow type: {error:?}"))?;
                let result_type = self.types.id(RustTy::new_ref(
                    Region {
                        kind: RegionKind::ReErased,
                    },
                    referent,
                    Mutability::Not,
                ))?;
                let place = self.map_place(place)?;
                Ok(self
                    .arenas
                    .expression(loc, result_type, ExprKind::Operation {
                        arguments: vec![],
                        instantiations: vec![],
                        operation: Operation::Borrow {
                            kind: BorrowKind::Immutable,
                            place,
                        },
                        surface: None,
                    }))
            },
            Rvalue::Ref(_, borrow_kind, place)
                if self.zero_sized_local(place).is_some()
                    && !(matches!(borrow_kind, MirBorrowKind::Mut { .. })
                        && place.projection.is_empty()) =>
            {
                let kind = match borrow_kind {
                    MirBorrowKind::Shared => BorrowKind::Immutable,
                    MirBorrowKind::Mut { .. } => BorrowKind::Mutable,
                    MirBorrowKind::Fake(kind) => {
                        return Err(format!("unsupported fake MIR borrow: {kind:?}"))
                    },
                };
                let constant = self.zero_sized_local(place).expect("checked");
                let value = self.constant_expression(&constant, loc)?;
                let result_type = self.types.id(rvalue
                    .ty(&self.locals)
                    .map_err(|error| format!("query borrow result type: {error:?}"))?)?;
                Ok(self
                    .arenas
                    .expression(loc, result_type, ExprKind::Operation {
                        arguments: vec![value],
                        instantiations: vec![],
                        operation: Operation::Reference {
                            kind: ReferenceOperation::Borrow { kind },
                        },
                        surface: None,
                    }))
            },
            Rvalue::Ref(_, borrow_kind, place) => {
                let kind = match borrow_kind {
                    MirBorrowKind::Shared => BorrowKind::Immutable,
                    MirBorrowKind::Mut { kind } => mutable_borrow_kind(kind),
                    MirBorrowKind::Fake(kind) => {
                        return Err(format!("unsupported fake MIR borrow: {kind:?}"))
                    },
                };
                let place = self.map_place(place)?;
                let result_type = self.types.id(rvalue
                    .ty(&self.locals)
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
                    .ty(&self.locals)
                    .map_err(|error| format!("query discriminant place type: {error:?}"))?;
                let TyKind::RigidTy(RigidTy::Adt(definition, _)) = place_type.kind() else {
                    return Err(format!(
                        "discriminant place does not have an enum type: {:?}",
                        place_type.kind()
                    ));
                };
                if !self.adts.iter().any(|entry| entry.definition == definition) {
                    return Err(format!("enum `{}` was not interned", definition.name()));
                }
                let place = self.map_place(place)?;
                let result_type = self.types.id(rvalue
                    .ty(&self.locals)
                    .map_err(|error| format!("query discriminant result type: {error:?}"))?)?;
                Ok(self
                    .arenas
                    .expression(loc, result_type, ExprKind::Operation {
                        arguments: vec![],
                        instantiations: vec![],
                        operation: Operation::ReadDiscriminant { place },
                        surface: None,
                    }))
            },
            Rvalue::Len(place) => {
                let place_type = place
                    .ty(&self.locals)
                    .map_err(|error| format!("query length place type: {error:?}"))?;
                let result_type = self.types.id(rvalue
                    .ty(&self.locals)
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
                    .ty(&self.locals)
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
                    .ty(&self.locals)
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
                    .ty(&self.locals)
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
            Rvalue::Aggregate(AggregateKind::Closure(definition, arguments), operands) => {
                let arguments_of = operands
                    .iter()
                    .map(|operand| self.map_operand(operand, loc))
                    .collect::<Result<Vec<_>, _>>()?;
                self.closure_value(*definition, arguments, arguments_of, loc)
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
                let fields = operands
                    .iter()
                    .map(|operand| self.map_operand(operand, loc))
                    .collect::<Result<Vec<_>, _>>()?;
                let result_type = self.types.id(rvalue
                    .ty(&self.locals)
                    .map_err(|error| format!("query aggregate result type: {error:?}"))?)?;
                self.constructor(
                    *definition,
                    *variant_index,
                    arguments,
                    fields,
                    result_type,
                    loc,
                )
            },
            kind => Err(format!("unsupported MIR rvalue: {}", variant_name(kind))),
        }
    }

    /// The construction of a variant of an ADT from its fields.
    fn constructor(
        &mut self,
        definition: AdtDef,
        variant_index: VariantIdx,
        arguments: &GenericArgs,
        fields: Vec<ExprId>,
        result_type: TypeId,
        loc: LocId,
    ) -> Result<ExprId, String> {
        let adt = self
            .adts
            .iter()
            .find(|entry| entry.definition == definition)
            .ok_or_else(|| format!("ADT `{}` was not interned", definition.name()))?;
        let variant = adt
            .variants
            .iter()
            .find(|variant| variant.index == variant_index)
            .ok_or_else(|| {
                format!(
                    "ADT `{}` has no variant {:?}",
                    definition.name(),
                    variant_index
                )
            })?;
        let instantiations = self.types.generic_arguments(arguments, loc)?;
        Ok(self
            .arenas
            .expression(loc, result_type, ExprKind::Operation {
                arguments: fields,
                instantiations,
                operation: Operation::Call {
                    kind: CallKind::Constructor {
                        constructor: QualifiedRef {
                            name: adt.name,
                            namespace_id: adt.namespace_id,
                        },
                        variant: variant.source_name.clone(),
                    },
                },
                surface: None,
            }))
    }

    /// Whether a type is not `Copy`, or only by a parameter the body does not show to be.
    fn copy_is_move(&mut self, ty: RustTy) -> bool {
        self.copies
            .impls
            .copy_requirement(ty)
            .is_none_or(|requirement| !requirement.is_subset(self.copies.parameters))
    }

    /// Whether a type is a closure's that the unit does not state to be `Copy`.
    fn closure_consumed_by_copy(&self, ty: RustTy) -> bool {
        let TyKind::RigidTy(RigidTy::Closure(definition, arguments)) = ty.kind() else {
            return false;
        };
        let known = self
            .closures
            .iter()
            .any(|closure| closure.definition == definition && closure.copy);
        !known
            || closure_parts(&arguments)
                .into_iter()
                .any(mentions_parameter)
    }

    /// Marks the reborrow a copy of a `&mut` reads as, where a temporary holds it.
    fn mark_reference_copy(&mut self, destination: &MirPlace, rvalue: &Rvalue, value: ExprId) {
        let Rvalue::Use(Operand::Copy(_), _) = rvalue else {
            return;
        };
        let local = destination.local;
        let temporary = destination.projection.is_empty()
            && local > self.body.arg_locals().len()
            && !self
                .body
                .var_debug_info
                .iter()
                .any(|info| info.local() == Some(local));
        if !temporary {
            return;
        }
        if let ExprKind::Operation {
            operation: Operation::Borrow { kind, .. },
            ..
        } = &mut self.arenas.expressions[value.index].kind
            && matches!(kind, BorrowKind::Mutable)
        {
            *kind = structurized_borrow_kind("borrow.reference_copy");
        }
    }

    /// Only roots actually borrowed as places need materialized storage.
    /// Projected places retain the existing conservative initialization rules.
    fn is_zero_sized_borrow_root(&self, local: usize) -> bool {
        self.zero_sized_locals.contains(&local)
            && self.body.blocks.iter().any(|block| block.statements.iter().any(|statement| {
                matches!(&statement.kind,
                    StatementKind::Assign(_, Rvalue::Ref(_, MirBorrowKind::Shared | MirBorrowKind::Mut { .. }, place))
                    if place.local == local && place.projection.is_empty())
            }))
    }

    /// Initialize each unassigned zero-sized root once per storage lifetime,
    /// before any borrow. Reborrowing must not write through a live shared alias.
    fn materialize_zero_sized_root(
        &mut self,
        local: usize,
        loc: LocId,
        statements: &mut Vec<RawStatement>,
    ) -> Result<(), String> {
        let borrowed = MirPlace {
            local,
            projection: vec![],
        };
        if let Some(constant) = self.zero_sized_local(&borrowed) {
            let value = self.constant_expression(&constant, loc)?;
            let place = self.map_place(&borrowed)?;
            let expression = self
                .arenas
                .expression(loc, self.types.unit, ExprKind::Assign { place, value });
            statements.push(RawStatement::Execute { expression });
        }
        Ok(())
    }

    /// The value a place holds, where it is a local holding a zero-sized value or a field
    /// of one: reading it is that value.
    fn zero_sized_local(&self, place: &MirPlace) -> Option<EvaluatedConstant> {
        if !self.zero_sized_locals.contains(&place.local)
            || !place
                .projection
                .iter()
                .all(|projection| matches!(projection, ProjectionElem::Field(..)))
        {
            return None;
        }
        read_value(place.ty(&self.locals).ok()?, &empty_allocation(), 0, 0).ok()
    }

    /// The closures standing in this body whose value it never uses, which still
    /// stand here: their values, as statements at its entry.
    fn unused_closure_statements(&mut self) -> Result<Vec<RawStatement>, String> {
        let mut uses = ClosureUses::default();
        uses.visit_body(self.body);
        for local in &self.zero_sized_locals {
            if let TyKind::RigidTy(RigidTy::Closure(closure, arguments)) =
                self.locals[*local].ty.kind()
            {
                uses.construct_zero_sized(closure, &arguments);
            }
        }
        let closures = self.closures;
        let mut statements = Vec::new();
        for closure in closures {
            if closure.parent == self.item.0
                && closure.zero_sized
                && !uses.constructed.contains(&closure.definition)
            {
                let loc = self.sources.intern(closure.definition.span())?;
                let ty = RustTy::from_rigid_kind(RigidTy::Closure(
                    closure.definition,
                    closure.arguments.clone(),
                ));
                let value = read_value(ty, &empty_allocation(), 0, 0)
                    .map_err(|_| format!("closure `{}` has no value", closure.item.name()))?;
                let expression = self.constant_expression(&value, loc)?;
                statements.push(RawStatement::Execute { expression });
            }
        }
        Ok(statements)
    }

    /// The construction of a closure's struct.
    fn closure_value(
        &mut self,
        definition: ClosureDef,
        arguments: &GenericArgs,
        values: Vec<ExprId>,
        loc: LocId,
    ) -> Result<ExprId, String> {
        let closure = self
            .closures
            .iter()
            .find(|closure| closure.definition == definition)
            .ok_or_else(|| format!("closure `{}` was not interned", definition.name()))?;
        let (parents, _) = closure_arguments(arguments)?;
        let mut instantiations = self.types.generic_arguments(&GenericArgs(parents), loc)?;
        let inferred = self.types.generic_arguments(
            &GenericArgs(vec![
                GenericArgKind::Lifetime(erased());
                closure.lifetimes.len()
            ]),
            loc,
        )?;
        instantiations.extend(inferred);
        let result_type = self.types.id(RustTy::from_rigid_kind(RigidTy::Closure(
            definition,
            arguments.clone(),
        )))?;
        Ok(self
            .arenas
            .expression(loc, result_type, ExprKind::Operation {
                arguments: values,
                instantiations,
                operation: Operation::Call {
                    kind: CallKind::Constructor {
                        constructor: QualifiedRef {
                            name: closure.name,
                            namespace_id: NamespaceId::new(0),
                        },
                        variant: None,
                    },
                },
                surface: None,
            }))
    }

    fn map_operand(&mut self, operand: &Operand, loc: LocId) -> Result<ExprId, String> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                if let Some(constant) = self.zero_sized_local(place) {
                    return self.constant_expression(&constant, loc);
                }
                let rust_type = place
                    .ty(&self.locals)
                    .map_err(|error| format!("query operand place type: {error:?}"))?;
                let type_id = self.types.id(rust_type)?;
                let place_id = self.map_place(place)?;
                let operation = match operand {
                    // Likewise of a closure, which a by-value call takes: a copy
                    // only where it is `Copy` whatever the parameters are.
                    Operand::Copy(_) if self.closure_consumed_by_copy(rust_type) => {
                        Operation::Move { place: place_id }
                    },
                    // `&mut T` is never Copy in Rust. Optimized MIR spells a
                    // reborrow `&mut *r` as `copy r` once the reborrow is
                    // propagated, also where `r` is used again; the reborrow
                    // is the meaning of every such copy, the last one included.
                    Operand::Copy(_)
                        if matches!(
                            rust_type.kind(),
                            TyKind::RigidTy(RigidTy::Ref(_, _, Mutability::Mut))
                        ) =>
                    {
                        let referent = PlaceId::new(self.arenas.places.len());
                        self.arenas.places.push(Place::Deref { base: place_id });
                        Operation::Borrow {
                            kind: BorrowKind::Mutable,
                            place: referent,
                        }
                    },
                    // Optimized MIR spells a final move of a local `copy`. At a
                    // type not `Copy`, or only by parameters the body does not
                    // show to be, a copy keeps the source's consuming meaning.
                    Operand::Copy(_) if self.copy_is_move(rust_type) => {
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
        let constants = self.constants;
        let evaluated = constants
            .values
            .get(&constant.const_)
            .ok_or_else(|| format!("unsupported MIR constant type: {:?}", constant.ty().kind()))?;
        self.constant_expression(evaluated, loc)
    }

    /// The expression of an evaluated constant: a literal, or the construction
    /// of its parts.
    fn constant_expression(
        &mut self,
        constant: &EvaluatedConstant,
        loc: LocId,
    ) -> Result<ExprId, String> {
        let primitive = |kind| Operation::Primitive { kind };
        let (ty, arguments, operation) = match constant {
            EvaluatedConstant::Literal { ty, value } => {
                let type_id = self.types.id(*ty)?;
                return Ok(self.arenas.expression(loc, type_id, ExprKind::Value {
                    source_constant: None,
                    value: value.clone(),
                }));
            },
            EvaluatedConstant::Opaque { ty, description } => {
                let type_id = match ty {
                    Some(ty) => self.types.id(*ty)?,
                    None => self.types.unit,
                };
                return Ok(self.arenas.expression(loc, type_id, ExprKind::Value {
                    source_constant: None,
                    value: ConstValue::Opaque {
                        description: description.clone(),
                    },
                }));
            },
            EvaluatedConstant::Adt {
                ty,
                definition,
                variant,
                arguments,
                fields,
            } => {
                let type_id = self.types.id(*ty)?;
                let fields = fields
                    .iter()
                    .map(|field| self.constant_expression(field, loc))
                    .collect::<Result<Vec<_>, _>>()?;
                return self.constructor(*definition, *variant, arguments, fields, type_id, loc);
            },
            EvaluatedConstant::Tuple { ty, elements } => (
                *ty,
                elements.as_slice(),
                primitive(PrimitiveOperation::Tuple),
            ),
            EvaluatedConstant::Array { ty, elements } => {
                if matches!(ty.kind(), TyKind::RigidTy(RigidTy::Slice(_))) {
                    return Err("a slice constant requires a shared backing view".to_owned());
                }
                (
                    *ty,
                    elements.as_slice(),
                    primitive(PrimitiveOperation::Vector),
                )
            },
            EvaluatedConstant::Shared { ty, referent } => {
                if let EvaluatedConstant::Array {
                    ty: referent_ty,
                    elements,
                } = referent.as_ref()
                    && let TyKind::RigidTy(RigidTy::Slice(element)) = referent_ty.kind()
                {
                    let backing = Ty::Vector {
                        element: self.types.id(element)?,
                        length: Some(ConstValue::Integer {
                            value: Number::from(elements.len()),
                        }),
                    };
                    let backing_type = self
                        .types
                        .types
                        .iter()
                        .position(|ty| *ty == backing)
                        .map(TypeId::new)
                        .ok_or_else(|| {
                            "a slice constant's backing array was not interned".to_owned()
                        })?;
                    let elements = elements
                        .iter()
                        .map(|element| self.constant_expression(element, loc))
                        .collect::<Result<Vec<_>, _>>()?;
                    let backing = self
                        .arenas
                        .expression(loc, backing_type, ExprKind::Operation {
                            arguments: elements,
                            instantiations: vec![],
                            operation: primitive(PrimitiveOperation::Vector),
                            surface: None,
                        });
                    return Ok(self.arenas.expression(
                        loc,
                        self.types.id(*ty)?,
                        ExprKind::Operation {
                            arguments: vec![backing],
                            instantiations: vec![],
                            operation: Operation::Reference {
                                kind: ReferenceOperation::Borrow {
                                    kind: BorrowKind::Immutable,
                                },
                            },
                            surface: None,
                        },
                    ));
                }
                (
                    *ty,
                    std::slice::from_ref(referent.as_ref()),
                    Operation::Reference {
                        kind: ReferenceOperation::Borrow {
                            kind: BorrowKind::Immutable,
                        },
                    },
                )
            },
            EvaluatedConstant::Closure {
                definition,
                arguments,
                upvars,
                ..
            } => {
                let values = upvars
                    .iter()
                    .map(|upvar| self.constant_expression(upvar, loc))
                    .collect::<Result<Vec<_>, _>>()?;
                return self.closure_value(*definition, arguments, values, loc);
            },
        };
        let type_id = self.types.id(ty)?;
        let arguments = arguments
            .iter()
            .map(|argument| self.constant_expression(argument, loc))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self.arenas.expression(loc, type_id, ExprKind::Operation {
            arguments,
            instantiations: vec![],
            operation,
            surface: None,
        }))
    }

    fn map_place(&mut self, place: &MirPlace) -> Result<PlaceId, String> {
        let place = &self.box_folds.resolve(place);
        let local_id = self
            .rust_to_vir
            .get(place.local)
            .copied()
            .ok_or_else(|| format!("MIR local {} is out of bounds", place.local))?;
        let mut current = self.arenas.place(local_id);
        let mut current_type = self.locals[place.local].ty;
        let mut selected_variant = None;
        for projection in &place.projection {
            current = match projection {
                ProjectionElem::Deref => {
                    let id = PlaceId::new(self.arenas.places.len());
                    self.arenas.places.push(Place::Deref { base: current });
                    current_type = match current_type.kind() {
                        TyKind::RigidTy(RigidTy::Ref(_, referent, _)) => referent,
                        // Import reads it as the box's content (`Library.applyModels`).
                        TyKind::RigidTy(RigidTy::Adt(definition, arguments))
                            if definition.is_box() =>
                        {
                            let Some(GenericArgKind::Type(content)) = arguments.0.first() else {
                                return Err(format!(
                                    "cannot dereference MIR place type {current_type:?}"
                                ));
                            };
                            *content
                        },
                        _ => {
                            return Err(format!(
                                "cannot dereference MIR place type {current_type:?}"
                            ));
                        },
                    };
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
                        let loc = self.local_location(self.locals[place.local].span)?;
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
                    if let TyKind::RigidTy(RigidTy::Closure(definition, _)) = current_type.kind() {
                        let closure = self
                            .closures
                            .iter()
                            .find(|closure| closure.definition == definition)
                            .ok_or_else(|| {
                                format!("closure `{}` was not interned", definition.name())
                            })?;
                        let (field, _) = closure.fields.get(*index).ok_or_else(|| {
                            format!("closure `{}` has no upvar {index}", definition.name())
                        })?;
                        let id = PlaceId::new(self.arenas.places.len());
                        self.arenas.places.push(Place::Field {
                            base: current,
                            owner: QualifiedRef {
                                name: closure.name,
                                namespace_id: NamespaceId::new(0),
                            },
                            field: *field,
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
                            namespace_id: adt.namespace_id,
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
                    let index_loc = self.local_location(self.locals[*index_local].span)?;
                    let index_type = self.types.id(self.locals[*index_local].ty)?;
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
                    let loc = self.local_location(self.locals[place.local].span)?;
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

    /// The types of a call's operands, then its destination's, as MIR states
    /// them: normalized.
    fn call_types(&self, args: &[Operand], destination: &MirPlace) -> Result<Vec<RustTy>, String> {
        let mut types = args
            .iter()
            .map(|argument| argument.ty(&self.locals))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("query call operand type: {error:?}"))?;
        types.push(
            destination
                .ty(&self.locals)
                .map_err(|error| format!("query call destination type: {error:?}"))?,
        );
        Ok(types)
    }

    fn map_terminator(
        &mut self,
        terminator: &Terminator,
        loc: LocId,
    ) -> Result<RawTerminator, String> {
        match &terminator.kind {
            TerminatorKind::Goto { target } => Ok(RawTerminator::Goto {
                target: BlockId::new(*target),
            }),
            TerminatorKind::SwitchInt { discr, targets } => {
                let scrutinee = self.map_operand(discr, loc)?;
                let discr_type = discr
                    .ty(&self.locals)
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
                    .ty(&self.locals)
                    .map_err(|error| format!("query callee type: {error:?}"))?;
                if let TyKind::RigidTy(RigidTy::FnDef(definition, generic_arguments)) =
                    callee_type.kind()
                {
                    let callee_name = definition.name();
                    if is_abort(definition) {
                        if !generic_arguments.0.is_empty() || !args.is_empty() || target.is_some() {
                            return Err(format!(
                                "unexpected MIR shape for abort intrinsic `{callee_name}`"
                            ));
                        }
                        return Ok(RawTerminator::Abort);
                    }
                }
                let closure_call = match callee_type.kind() {
                    TyKind::RigidTy(RigidTy::FnDef(definition, generic_arguments)) => {
                        closure_callee(self.item.0, definition, &generic_arguments)
                    },
                    _ => None,
                };
                let tupled_evidence = match callee_type.kind() {
                    TyKind::RigidTy(RigidTy::FnDef(definition, arguments)) => self
                        .method_evidence
                        .get(self.identity)
                        .is_some_and(|methods| {
                            methods.iter().any(|evidence| {
                                evidence.tupled
                                    && evidence.method == definition
                                    && evidence.arguments == arguments
                            })
                        }),
                    _ => false,
                };
                // A closure's body takes its parameters spread.
                let operands = match (closure_call.is_some() || tupled_evidence, args.as_slice()) {
                    (true, [receiver, Operand::Move(tuple) | Operand::Copy(tuple)]) => {
                        let elements = self.call_tuples.get(&tuple.local).ok_or_else(|| {
                            format!(
                                "the argument tuple of a closure call in `{}` was not folded",
                                self.identity
                            )
                        })?;
                        std::iter::once(receiver.clone())
                            .chain(elements.iter().cloned())
                            .collect()
                    },
                    (true, [receiver, Operand::Constant(tuple)]) if matches!(tuple.ty().kind(), TyKind::RigidTy(RigidTy::Tuple(ts)) if ts.is_empty()) =>
                    {
                        vec![receiver.clone()]
                    },
                    (true, _) => {
                        return Err(
                            "closure evidence requires a receiver and a built argument tuple"
                                .to_owned(),
                        )
                    },
                    (false, _) => args.clone(),
                };
                let mut arguments = operands
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
                            let argument_type = argument.ty(&self.locals).map_err(|error| {
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
                        } else if let Some(index) =
                            self.method_evidence.get(self.identity).and_then(|methods| {
                                methods.iter().position(|evidence| {
                                    evidence.method == definition
                                        && evidence.arguments == generic_arguments
                                })
                            })
                        {
                            let callable = self.evidence_operand(index, loc)?;
                            arguments.insert(0, callable);
                            Operation::Call {
                                kind: CallKind::Invoke,
                            }
                        } else {
                            let (resolved, resolved_arguments) =
                                resolve_callee(self.item.0, definition, &generic_arguments)
                                    .map_err(|reason| {
                                        format!(
                                            "{} {reason}",
                                            call_site(
                                                definition,
                                                self.identity,
                                                terminator.source_info.span,
                                                &self.sources.crate_files
                                            )
                                        )
                                    })?;
                            let resolved_name = resolved.name();
                            let callee =
                                self.name_ids.get(&resolved_name).cloned().ok_or_else(|| {
                                    format!("callee `{resolved_name}` is not a mapped function")
                                })?;
                            let dependency = self
                                .dependency_functions
                                .iter()
                                .find(|function| function.definition == resolved);
                            if let Some((_, closure_args)) = &closure_call {
                                instantiations = self.types.generic_arguments(
                                    &GenericArgs(closure_arguments(closure_args)?.0),
                                    loc,
                                )?;
                            } else if !dependency.is_some_and(|dependency| dependency.opaque)
                                && !self.opaque_functions.contains(&resolved_name)
                            {
                                instantiations =
                                    self.types.generic_arguments(&resolved_arguments, loc)?;
                            }
                            if let Some(dependency) = dependency
                                && !dependency.associated_types.is_empty()
                            {
                                // The dictionaries the call supplies state what
                                // rustc normalized the callee's associated types
                                // to, where its signature does not name them.
                                let actual_evidence = evidence_types(
                                    &self
                                        .method_evidence
                                        .get(&resolved_name)
                                        .map(Vec::as_slice)
                                        .unwrap_or_default()
                                        .iter()
                                        .map(|evidence| {
                                            evidence.instantiate(self.item.0, &resolved_arguments)
                                        })
                                        .collect::<Vec<_>>(),
                                );
                                let bindings = self
                                    .call_types(args, destination)
                                    .and_then(|actual| {
                                        dependency.bind_associated_types(
                                            &resolved_arguments,
                                            &actual,
                                            &actual_evidence,
                                        )
                                    })
                                    .map_err(|reason| {
                                        format!(
                                            "{}: {reason}",
                                            call_site(
                                                definition,
                                                self.identity,
                                                terminator.source_info.span,
                                                &self.sources.crate_files
                                            )
                                        )
                                    })?;
                                for binding in bindings {
                                    instantiations.push(GenericArgument::TypeArg {
                                        value: TypeUse {
                                            loc,
                                            type_id: self.types.id(binding)?,
                                        },
                                    });
                                }
                            }
                            instantiations.extend(self.bound_lifetime_arguments(&resolved_name)?);
                            for evidence in self
                                .method_evidence
                                .get(&resolved_name)
                                .cloned()
                                .unwrap_or_default()
                            {
                                arguments.push(self.evidence_for(
                                    &evidence.instantiate(self.item.0, &resolved_arguments),
                                    loc,
                                )?);
                            }
                            Operation::Call {
                                kind: CallKind::Function { callee },
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
                    .ty(&self.locals)
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
                let return_place = MirPlace {
                    local: 0,
                    projection: vec![],
                };
                let return_type = return_place
                    .ty(&self.locals)
                    .map_err(|error| format!("query return place type: {error:?}"))?;
                // Optimized MIR never assigns a zero-sized return place. Its
                // value is nevertheless determined, so export the value itself
                // rather than a return-place move that definite-initialization
                // checking must reject.
                let constants = self.constants;
                let result = match constants.zero_sized_values.get(&return_type) {
                    Some(value) => self.constant_expression(value, loc)?,
                    None => self.map_operand(&Operand::Move(return_place), loc)?,
                };
                Ok(RawTerminator::Return {
                    values: vec![result],
                })
            },
            TerminatorKind::Unreachable => Ok(RawTerminator::Unreachable),
            TerminatorKind::Resume => Ok(RawTerminator::Resume),
            TerminatorKind::Abort => Ok(RawTerminator::Abort),
            kind => Err(format!(
                "unsupported MIR terminator: {}",
                variant_name(kind)
            )),
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

fn empty_contract() -> FunctionContract {
    FunctionContract {
        conditions: vec![],
        has_frame: false,
        loc: None,
        modifies: vec![],
        modifies_all: false,
        parameter_frames: vec![],
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
