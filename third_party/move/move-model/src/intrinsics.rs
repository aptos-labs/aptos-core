// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    ast::{Address, Operation, PropertyBag, PropertyValue, QualifiedSymbol},
    builder::module_builder::SpecBlockContext,
    model::{IntrinsicId, QualifiedId, SpecFunId},
    pragmas::{
        IntrinsicFunDef, INTRINSIC_FUN_MAP_ITER_BORROW_MUT, INTRINSIC_PRAGMA, INTRINSIC_TYPE_MAP,
        INTRINSIC_TYPE_MAP_ASSOC_FUNCTIONS,
    },
    symbol::{Symbol, SymbolPool},
    ty::Type,
    FunId, GlobalEnv, Loc, ModuleBuilder, StructId,
};
use std::{collections::BTreeMap, ops::Deref};

/// An information pack that holds the intrinsic declaration
#[derive(Clone, Debug)]
pub struct IntrinsicDecl {
    move_type: QualifiedId<StructId>,
    intrinsic_type: Symbol,
    intrinsic_to_move_fun: BTreeMap<Symbol, QualifiedId<FunId>>,
    move_fun_to_intrinsic: BTreeMap<QualifiedId<FunId>, Symbol>,
    intrinsic_to_spec_fun: BTreeMap<Symbol, QualifiedId<SpecFunId>>,
    spec_fun_to_intrinsic: BTreeMap<QualifiedId<SpecFunId>, Symbol>,
    /// Maps intrinsic Move function name symbol → intrinsic spec function name symbol,
    /// for pure-spec substitution (read-only functions only).
    move_to_spec_intrinsic: BTreeMap<Symbol, Symbol>,
    /// Maps intrinsic Move function name symbol → intrinsic abort-condition spec function name symbol.
    move_to_abort_spec_intrinsic: BTreeMap<Symbol, Symbol>,
}

impl IntrinsicDecl {
    pub fn get_fun_triple(&self, env: &GlobalEnv, name: &str) -> Option<(Address, String, String)> {
        let symbol_pool = env.symbol_pool();
        let sym = symbol_pool.make(name);
        self.intrinsic_to_move_fun
            .get(&sym)
            .map(|qid| {
                let fun_env = env.get_function(*qid);
                let mod_name = fun_env.module_env.get_name();
                (
                    mod_name.addr().clone(),
                    symbol_pool.string(mod_name.name()).to_string(),
                    symbol_pool.string(fun_env.get_name()).to_string(),
                )
            })
            .or_else(|| {
                self.intrinsic_to_spec_fun.get(&sym).map(|qid| {
                    let mod_env = env.get_module(qid.module_id);
                    let mod_name = mod_env.get_name();
                    let fun_decl = mod_env.get_spec_fun(qid.id);
                    (
                        mod_name.addr().clone(),
                        symbol_pool.string(mod_name.name()).to_string(),
                        symbol_pool.string(fun_decl.name).to_string(),
                    )
                })
            })
    }

    pub fn lookup_spec_fun(&self, env: &GlobalEnv, name: &str) -> Option<QualifiedId<SpecFunId>> {
        let symbol_pool = env.symbol_pool();
        let sym = symbol_pool.make(name);
        self.intrinsic_to_spec_fun.get(&sym).cloned()
    }

    /// Look up the bound Move function for an intrinsic role by name.
    pub fn lookup_move_fun(&self, env: &GlobalEnv, name: &str) -> Option<QualifiedId<FunId>> {
        let symbol_pool = env.symbol_pool();
        let sym = symbol_pool.make(name);
        self.intrinsic_to_move_fun.get(&sym).cloned()
    }
}

pub(crate) fn process_intrinsic_declaration(
    builder: &mut ModuleBuilder,
    loc: &Loc,
    context: &SpecBlockContext,
    props: &mut PropertyBag,
) {
    // intrinsic declarations only appears in struct spec block
    let type_qsym = match context {
        SpecBlockContext::Struct(qsym) => qsym.clone(),
        _ => {
            return;
        },
    };

    // search for intrinsic declarations
    let symbol_pool = builder.symbol_pool();
    let pragma_symbol = symbol_pool.make(INTRINSIC_PRAGMA);
    let target = match props.get_mut(&pragma_symbol) {
        None => {
            // this is not an intrinsic declaration
            return;
        },
        Some(val) => {
            match val {
                PropertyValue::Symbol(sym) => symbol_pool.string(*sym),
                PropertyValue::QualifiedSymbol(_) => {
                    builder
                        .parent
                        .error(loc, "expect a boolean value or a valid intrinsic type");
                    return;
                },
                _ => {
                    // this is the true/false pragma
                    return;
                },
            }
        },
    };

    // obtain the associated functions map
    let associated_funs = match target.as_str() {
        INTRINSIC_TYPE_MAP => INTRINSIC_TYPE_MAP_ASSOC_FUNCTIONS.deref(),
        _ => {
            builder
                .parent
                .error(loc, &format!("unknown intrinsic type: {}", target.as_str()));
            return;
        },
    };

    // prepare the decl
    let type_entry = builder.parent.struct_table.get(&type_qsym).expect("struct");
    let move_type = type_entry.module_id.qualified(type_entry.struct_id);

    let mut decl = IntrinsicDecl {
        move_type,
        intrinsic_type: symbol_pool.make(target.as_str()),
        intrinsic_to_move_fun: BTreeMap::new(),
        move_fun_to_intrinsic: BTreeMap::new(),
        intrinsic_to_spec_fun: BTreeMap::new(),
        spec_fun_to_intrinsic: BTreeMap::new(),
        move_to_spec_intrinsic: BTreeMap::new(),
        move_to_abort_spec_intrinsic: BTreeMap::new(),
    };

    // construct the pack
    populate_intrinsic_decl(builder, loc, associated_funs, props, &mut decl);

    // add the decl back
    builder.parent.intrinsics.push(decl);
}

fn populate_intrinsic_decl(
    builder: &mut ModuleBuilder,
    loc: &Loc,
    associated_funs: &BTreeMap<&str, IntrinsicFunDef>,
    props: &mut PropertyBag,
    decl: &mut IntrinsicDecl,
) {
    let symbol_pool = builder.symbol_pool();
    for (&name, fun_def) in associated_funs {
        let key_sym = symbol_pool.make(name);

        // look-up the target of the declaration, if present
        let target_sym = match props.remove(&key_sym) {
            None => {
                continue;
            },
            Some(PropertyValue::Value(_)) => {
                builder.parent.error(
                    loc,
                    &format!("invalid intrinsic function mapping: {}", name),
                );
                continue;
            },
            Some(PropertyValue::Symbol(val_sym)) => val_sym,
            Some(PropertyValue::QualifiedSymbol(qual_sym)) => {
                if qual_sym.module_name != builder.module_name {
                    builder.parent.error(
                        loc,
                        &format!(
                            "an intrinsic function mapping can only refer to functions \
                            declared in the same module while `{}` is not",
                            qual_sym.display(builder.parent.env)
                        ),
                    );
                    continue;
                }
                qual_sym.symbol
            },
        };
        let qualified_sym = QualifiedSymbol {
            module_name: builder.module_name.clone(),
            symbol: target_sym,
        };

        // check presence
        if fun_def.is_move_fun {
            match builder.parent.fun_table.get(&qualified_sym) {
                None => {
                    builder.parent.error(
                        loc,
                        &format!(
                            "unable to find move function for intrinsic mapping: {}",
                            qualified_sym.display(builder.parent.env)
                        ),
                    );
                    continue;
                },
                Some(entry) => {
                    // TODO: in theory, we should also do some type checking on the function
                    // signature. This is implicitly done by Boogie right now, but we may want to
                    // make it more explicit and do the checking ourselves.
                    let qid = entry.module_id.qualified(entry.fun_id);
                    decl.intrinsic_to_move_fun.insert(key_sym, qid);
                    if decl.move_fun_to_intrinsic.insert(qid, key_sym).is_some() {
                        builder.parent.error(
                            loc,
                            &format!(
                                "duplicated intrinsic mapping for move function: {}",
                                qualified_sym.display(builder.parent.env)
                            ),
                        );
                        continue;
                    }
                    // Populate the direct Move→spec and Move→abort-spec maps from the
                    // IntrinsicFunDef so callers don't need separate static lookup tables.
                    if let Some(spec_name) = fun_def.spec_fun {
                        let spec_sym = symbol_pool.make(spec_name);
                        decl.move_to_spec_intrinsic.insert(key_sym, spec_sym);
                    }
                    if let Some(abort_name) = fun_def.abort_spec_fun {
                        let abort_sym = symbol_pool.make(abort_name);
                        decl.move_to_abort_spec_intrinsic.insert(key_sym, abort_sym);
                    }
                },
            }
        } else {
            match builder.parent.spec_fun_table.get(&qualified_sym) {
                None => {
                    builder.parent.error(
                        loc,
                        &format!(
                            "unable to find spec function for intrinsic mapping: {}",
                            qualified_sym.display(builder.parent.env)
                        ),
                    );
                    continue;
                },
                Some(entries) => {
                    if entries.len() != 1 {
                        builder.parent.error(
                            loc,
                            &format!(
                                "unable to find a unique spec function for intrinsic mapping: {}",
                                qualified_sym.display(builder.parent.env)
                            ),
                        );
                        continue;
                    }
                    let entry = &entries[0];

                    // TODO: in theory, we should also do some type checking on the function
                    // signature. This is implicitly done by Boogie right now, but we may want to
                    // make it more explicit and do the checking ourselves.
                    if let Operation::SpecFunction(mid, fid, ..) = &entry.oper {
                        let qid = mid.qualified(*fid);
                        decl.intrinsic_to_spec_fun.insert(key_sym, qid);
                        if decl.spec_fun_to_intrinsic.insert(qid, key_sym).is_some() {
                            builder.parent.error(
                                loc,
                                &format!(
                                    "duplicated intrinsic mapping for spec function: {}",
                                    qualified_sym.display(builder.parent.env)
                                ),
                            );
                            continue;
                        }
                    }
                },
            }
        }
    }
}

/// Hosts all intrinsic declarations
#[derive(Clone, Debug, Default)]
pub struct IntrinsicsAnnotation {
    /// Intrinsic declarations
    decls: BTreeMap<IntrinsicId, IntrinsicDecl>,
    /// A map from intrinsic types to intrinsic decl
    intrinsic_structs: BTreeMap<QualifiedId<StructId>, IntrinsicId>,
    /// A map from intrinsic move functions to intrinsic decl
    intrinsic_move_funs: BTreeMap<QualifiedId<FunId>, IntrinsicId>,
    /// A map from intrinsic spec functions to intrinsic decl
    intrinsic_spec_funs: BTreeMap<QualifiedId<SpecFunId>, IntrinsicId>,
}

impl IntrinsicsAnnotation {
    /// Add a declaration pack into the annotation set
    pub fn add_decl(&mut self, decl: &IntrinsicDecl) {
        let id = IntrinsicId::new(self.decls.len());
        self.intrinsic_structs.insert(decl.move_type, id);
        for move_fid in decl.move_fun_to_intrinsic.keys() {
            self.intrinsic_move_funs.insert(*move_fid, id);
        }
        for spec_fid in decl.spec_fun_to_intrinsic.keys() {
            self.intrinsic_spec_funs.insert(*spec_fid, id);
        }
        self.decls.insert(id, decl.clone());
    }

    /// Get the intrinsic decl for struct
    pub fn get_decl_for_struct(&self, qid: &QualifiedId<StructId>) -> Option<&IntrinsicDecl> {
        self.intrinsic_structs
            .get(qid)
            .map(|id| self.decls.get(id).unwrap())
    }

    /// Get the intrinsic decl for a move function
    pub fn get_decl_for_move_fun(&self, qid: &QualifiedId<FunId>) -> Option<&IntrinsicDecl> {
        self.intrinsic_move_funs
            .get(qid)
            .map(|id| self.decls.get(id).unwrap())
    }

    /// Given a Move function qualified ID, return the spec function qualified ID that
    /// corresponds to it via the intrinsic map pairing encoded in `IntrinsicDecl`.
    pub fn get_spec_fun_for_move_fun(
        &self,
        move_qid: &QualifiedId<FunId>,
    ) -> Option<QualifiedId<SpecFunId>> {
        let decl = self.get_decl_for_move_fun(move_qid)?;
        let move_intrinsic_sym = decl.move_fun_to_intrinsic.get(move_qid)?;
        let spec_intrinsic_sym = decl.move_to_spec_intrinsic.get(move_intrinsic_sym)?;
        decl.intrinsic_to_spec_fun.get(spec_intrinsic_sym).cloned()
    }

    /// Given a Move function qualified ID, return the abort-condition spec function qualified ID
    /// that corresponds to it via the intrinsic abort-spec pairing encoded in `IntrinsicDecl`.
    pub fn get_abort_spec_fun_for_move_fun(
        &self,
        move_qid: &QualifiedId<FunId>,
    ) -> Option<QualifiedId<SpecFunId>> {
        let decl = self.get_decl_for_move_fun(move_qid)?;
        let move_intrinsic_sym = decl.move_fun_to_intrinsic.get(move_qid)?;
        let abort_intrinsic_sym = decl.move_to_abort_spec_intrinsic.get(move_intrinsic_sym)?;
        decl.intrinsic_to_spec_fun.get(abort_intrinsic_sym).cloned()
    }

    /// Get the intrinsic decl for a spec function
    pub fn get_decl_for_spec_fun(&self, qid: &QualifiedId<SpecFunId>) -> Option<&IntrinsicDecl> {
        self.intrinsic_spec_funs
            .get(qid)
            .map(|id| self.decls.get(id).unwrap())
    }

    /// Test whether a struct is an intrinsic of a specific name
    pub fn is_intrinsic_of_for_struct(
        &self,
        symbol_pool: &SymbolPool,
        qid: &QualifiedId<StructId>,
        intrinsic_name: &str,
    ) -> bool {
        self.intrinsic_structs.get(qid).is_some_and(|id| {
            let decl = self.decls.get(id).expect("intrinsic decl");
            let sym = symbol_pool.make(intrinsic_name);
            decl.intrinsic_type == sym
        })
    }

    /// Test whether a move function is an intrinsic of a specific name
    pub fn is_intrinsic_of_for_move_fun(
        &self,
        symbol_pool: &SymbolPool,
        qid: &QualifiedId<FunId>,
        intrinsic_name: &str,
    ) -> bool {
        self.intrinsic_move_funs
            .get(qid)
            .and_then(|id| {
                self.decls
                    .get(id)
                    .expect("intrinsic decl")
                    .move_fun_to_intrinsic
                    .get(qid)
            })
            .is_some_and(|sym| sym == &symbol_pool.make(intrinsic_name))
    }

    /// Test whether a spec function is an intrinsic of a specific name
    pub fn is_intrinsic_of_for_spec_fun(
        &self,
        symbol_pool: &SymbolPool,
        qid: &QualifiedId<SpecFunId>,
        intrinsic_name: &str,
    ) -> bool {
        self.intrinsic_spec_funs
            .get(qid)
            .and_then(|id| {
                self.decls
                    .get(id)
                    .expect("intrinsic decl")
                    .spec_fun_to_intrinsic
                    .get(qid)
            })
            .is_some_and(|sym| sym == &symbol_pool.make(intrinsic_name))
    }
}

/// The flavor of a tracked map iterator type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IteratorKind {
    /// Key-typed iterator (the parameter-0 enum of the map's bound
    /// `map_iter_borrow_mut`); validity is tracked per (map, key type).
    Keyed,
    /// Key-agnostic iterator, identified by its use at a plain (non
    /// field-indirected) `iterator_use`/`iterator_create` position of a
    /// function taking a tracked map (e.g. a leaf iterator); validity is
    /// tracked per map type.
    Unkeyed,
}

/// If `ty` is an iterator type of an intrinsic map with iterator-validity
/// tracking declared in the same module, returns the map struct and the
/// iterator's kind. Backends key the validity ghost state by the map struct
/// and, for keyed iterators, the key type.
pub fn find_iterator_map_decl(
    env: &GlobalEnv,
    ty: &Type,
) -> Option<(QualifiedId<StructId>, IteratorKind)> {
    let Type::Struct(mid, sid, targs) = ty else {
        return None;
    };
    let iter_qid = mid.qualified(*sid);
    // Keyed: the parameter-0 enum of a map's bound `map_iter_borrow_mut`.
    if !targs.is_empty() && env.get_struct(iter_qid).has_variants() {
        for map_struct in env.get_module(*mid).get_structs() {
            let map_qid = map_struct.get_qualified_id();
            let Some(decl) = env.intrinsics.get_decl_for_struct(&map_qid) else {
                continue;
            };
            let Some(fun_qid) = decl.lookup_move_fun(env, INTRINSIC_FUN_MAP_ITER_BORROW_MUT) else {
                continue;
            };
            if let Some(param0) = env.get_function(fun_qid).get_parameter_types().first() {
                if let Type::Struct(pmid, psid, _) = param0.skip_reference() {
                    if pmid.qualified(*psid) == iter_qid {
                        return Some((map_qid, IteratorKind::Keyed));
                    }
                }
            }
        }
    }
    // Unkeyed: appears directly as the consumed parameter 0 of a function with
    // a plain `iterator_use` pragma, or in the result of one with a plain
    // `iterator_create` pragma, where the function also takes a tracked map.
    // Field-indirected (symbol-valued) pragmas are excluded: there the
    // parameter/result is a container of an iterator, not an iterator itself.
    let pool = env.symbol_pool();
    let use_sym = pool.make(crate::pragmas::ITERATOR_USE_PRAGMA);
    let create_sym = pool.make(crate::pragmas::ITERATOR_CREATE_PRAGMA);
    let is_iter_ty = |t: &Type| {
        matches!(t.skip_reference(),
            Type::Struct(pmid, psid, _) if pmid.qualified(*psid) == iter_qid)
    };
    for fun in env.get_module(*mid).get_functions() {
        let spec = fun.get_spec();
        let plain_pragma = |sym: &Symbol| matches!(spec.properties.get(sym), Some(v) if !matches!(v, PropertyValue::Symbol(_)));
        let mut positions = vec![];
        if plain_pragma(&use_sym) {
            positions.extend(fun.get_parameter_types().first().cloned());
        }
        if plain_pragma(&create_sym) {
            match fun.get_result_type() {
                Type::Tuple(ts) => positions.extend(ts),
                t => positions.push(t),
            }
        }
        if !positions.iter().any(is_iter_ty) {
            continue;
        }
        let map_qid = fun.get_parameter_types().iter().find_map(|p| {
            let Type::Struct(pmid, psid, _) = p.skip_reference() else {
                return None;
            };
            let q = pmid.qualified(*psid);
            (env.get_struct(q).is_intrinsic_of(INTRINSIC_TYPE_MAP)
                && map_has_iterator_tracking(env, q))
            .then_some(q)
        });
        if let Some(map_qid) = map_qid {
            return Some((map_qid, IteratorKind::Unkeyed));
        }
    }
    None
}

/// Whether an intrinsic map struct has iterator-validity tracking, i.e. binds
/// the `map_iter_borrow_mut` role. Only such maps declare validity ghost state.
pub fn map_has_iterator_tracking(env: &GlobalEnv, map_qid: QualifiedId<StructId>) -> bool {
    env.intrinsics
        .get_decl_for_struct(&map_qid)
        .and_then(|decl| decl.lookup_move_fun(env, INTRINSIC_FUN_MAP_ITER_BORROW_MUT))
        .is_some()
}

/// If `ty` holds a tracked map iterator — directly, behind a reference, or as
/// a plain struct with a direct iterator field (e.g. the `IteratorPtrWithPath`
/// companion) — returns the iterator's type together with the projection
/// field for wrapped iterators. Temps of such types carry validity shadows
/// and participate in the implicit loop invariant.
pub fn find_iterator_target(env: &GlobalEnv, ty: &Type) -> Option<(Type, Option<Symbol>)> {
    let ty = ty.skip_reference();
    if find_iterator_map_decl(env, ty).is_some() {
        return Some((ty.clone(), None));
    }
    let Type::Struct(mid, sid, targs) = ty else {
        return None;
    };
    let struct_env = env.get_struct(mid.qualified(*sid));
    if struct_env.has_variants() {
        return None;
    }
    struct_env.get_fields().find_map(|f| {
        let field_ty = f.get_type().instantiate(targs);
        find_iterator_map_decl(env, &field_ty)
            .is_some()
            .then(|| (field_ty, Some(f.get_name())))
    })
}
