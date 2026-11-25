// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Generates struct and enum declarations. Fields are primitives, type parameters, or (see
//! [`Config::rich_field_prob`]) vectors and earlier structs. Every type has either
//! `copy + drop + store` or `drop + store + key`.

use crate::{
    abilities::{implied_struct_abilities, AbilityCtx},
    config::{Config, DimensionFlag},
    error::Result,
    ir::{Field, QualifiedStruct, StructDecl, StructLayout, Type, TypeParam, Variant},
    types::{pick_concrete_primitive, pick_type_param_constraint},
    violations,
};
use arbitrary::Unstructured;
use move_core_types::{
    ability::{Ability, AbilitySet},
    identifier::Identifier,
    language_storage::ModuleId,
};

pub(crate) fn build_structs(
    u: &mut Unstructured<'_>,
    cfg: &Config,
    module_id: &ModuleId,
) -> Result<(Vec<StructDecl>, Vec<DimensionFlag>)> {
    let count = u.int_in_range(0..=cfg.structs_per_module_max)?;
    let mut out = Vec::with_capacity(count as usize);
    let mut injected = Vec::new();
    for i in 0..count {
        let decl = build_simple_struct(u, cfg, module_id, i, &out, &mut injected)?;
        out.push(decl);
    }
    Ok((out, injected))
}

fn build_simple_struct(
    u: &mut Unstructured<'_>,
    cfg: &Config,
    module_id: &ModuleId,
    idx: u8,
    earlier: &[StructDecl],
    injected: &mut Vec<DimensionFlag>,
) -> Result<StructDecl> {
    let name = Identifier::new(format!("S{}", idx)).expect("ident");

    // 50/50 between singleton struct and enum (when enums are enabled). The two share field
    // construction below.
    let make_enum = cfg.enable_variants && u.arbitrary::<bool>()?;
    // Both structs and enums may have phantom and non-phantom type parameters.
    let generic = cfg.enable_generics && u.arbitrary::<bool>()?;

    // Layout-time dimension violations: duplicated field name (`valid_duplication`),
    // self-referential field (`valid_struct_defs`), and declaring abilities the fields don't
    // support (`valid_abilities` — e.g. a `signer` field, which has no `store`, on a `key`
    // struct). The first two decisions are recorded only where the violating shape actually
    // lands — a positive draw can still produce a clean struct (e.g. `recursive_field` on an
    // enum, or `duplicate_field` on a single-field layout).
    let duplicate_field = violations::wants_violation(u, cfg, DimensionFlag::Duplication)?;
    let recursive_field = violations::wants_violation(u, cfg, DimensionFlag::StructDefs)?;
    let abilities_violation = violations::wants_violation(u, cfg, DimensionFlag::Abilities)?;

    // A limits violation gives a singleton struct 35 fields, exceeding the production cap of 30.
    let oversize_fields =
        !make_enum && violations::should_violate(u, cfg, DimensionFlag::Limits, injected)?;

    // Choose each type parameter's phantom flag independently.
    let n_type_params = if generic {
        let max = cfg.type_params_max.max(1);
        u.int_in_range(1..=max)? as usize
    } else {
        0
    };
    let mut phantoms: Vec<bool> = Vec::with_capacity(n_type_params);
    for _ in 0..n_type_params {
        phantoms.push(u.arbitrary::<bool>()?);
    }
    let mut type_params: Vec<TypeParam> = Vec::with_capacity(phantoms.len());
    for (i, &is_phantom) in phantoms.iter().enumerate() {
        type_params.push(TypeParam {
            name: Identifier::new(format!("T{}", i)).expect("ident"),
            constraints: pick_type_param_constraint(u)?,
            is_phantom,
        });
    }

    let self_qs = QualifiedStruct {
        module: module_id.clone(),
        name: name.clone(),
    };
    // Instantiated with the struct's own type parameters, so the recursive field is
    // well-formed and only the recursion itself is rejected.
    let self_struct_type = Type::Struct(
        self_qs.clone(),
        (0..n_type_params as u16).map(Type::TypeParam).collect(),
    );

    let layout = if make_enum {
        let variant_count = u.int_in_range(1..=cfg.variants_per_enum_max.max(1))?;
        let mut variants: Vec<Variant> = Vec::with_capacity(variant_count as usize);
        // Indices of non-phantom type params — every variant must reference each one in some
        // field so the per-variant `pack_variant` back-derivation can bind it. (`infer_type_args`
        // unifies against the chosen variant's raw field types only, so a non-phantom param
        // missing from a variant's fields can't be inferred at that variant's pack site.)
        let non_phantom_ids: Vec<u16> = phantoms
            .iter()
            .enumerate()
            .filter(|(_, p)| !**p)
            .map(|(i, _)| i as u16)
            .collect();
        for vi in 0..variant_count {
            let variant_name = Identifier::new(format!("V{}", vi)).expect("ident");
            // Field-count budget honors the configured cap while reserving room for one
            // type-param field per non-phantom param.
            let min_fields = non_phantom_ids.len() as u8;
            let upper = cfg.fields_per_struct_max.max(min_fields);
            let field_count = u.int_in_range(min_fields..=upper)?;
            let mut fields: Vec<Field> = Vec::with_capacity(field_count as usize);
            // First, one field per non-phantom param (in declaration order).
            for &idx in &non_phantom_ids {
                fields.push(Field {
                    name: build_field_name(duplicate_field, fields.len() as u8),
                    ty: Type::TypeParam(idx),
                });
            }
            // Then fill up.
            while (fields.len() as u8) < field_count {
                fields.push(Field {
                    name: build_field_name(duplicate_field, fields.len() as u8),
                    ty: pick_field_type(u, cfg, module_id, earlier)?,
                });
            }
            variants.push(Variant {
                name: variant_name,
                fields,
            });
        }
        StructLayout::Variants(variants)
    } else {
        // Plan the field layout in two passes: first the special-purpose fi==0 slot (if any),
        // then one field per remaining non-phantom param, then primitives to fill up. The
        // `infer_type_args` back-derivation requires every non-phantom param to appear in
        // some field's raw type — otherwise the unifier has no place to bind it and `try_pack`
        // can never instantiate the struct.
        let mut planned: Vec<Type> = Vec::new();
        let mut param_0_covered_at_0 = false;
        if recursive_field {
            planned.push(self_struct_type.clone());
            injected.push(DimensionFlag::StructDefs);
        } else if n_type_params > 0 && !phantoms[0] {
            planned.push(Type::TypeParam(0));
            param_0_covered_at_0 = true;
        }
        let mut remaining_nonphantom: Vec<u16> = phantoms
            .iter()
            .enumerate()
            .filter(|(i, is_phantom)| !(**is_phantom || param_0_covered_at_0 && *i == 0_usize))
            .map(|(i, _)| i as u16)
            .collect();
        let min_fields = (planned.len() + remaining_nonphantom.len()).max(1) as u8;
        let field_count = if oversize_fields {
            // 35 > production's max_fields_in_struct = 30, so the limits pass rejects.
            35u8
        } else {
            let upper = cfg.fields_per_struct_max.max(min_fields);
            u.int_in_range(min_fields..=upper)?
        };
        while (planned.len() as u8) < field_count {
            if !remaining_nonphantom.is_empty() {
                let idx = remaining_nonphantom.remove(0);
                planned.push(Type::TypeParam(idx));
            } else {
                planned.push(pick_field_type(u, cfg, module_id, earlier)?);
            }
        }
        let mut fields: Vec<Field> = Vec::with_capacity(field_count as usize);
        for (fi, ty) in planned.into_iter().enumerate() {
            fields.push(Field {
                name: build_field_name(duplicate_field, fi as u8),
                ty,
            });
        }
        StructLayout::Singleton(fields)
    };

    // A duplicated name materializes only when some field container has a second field to
    // alias the first's name (`build_field_name` duplicates for `fi > 0`).
    let duplicate_emitted = duplicate_field
        && match &layout {
            StructLayout::Singleton(fields) => fields.len() > 1,
            StructLayout::Variants(variants) => variants.iter().any(|v| v.fields.len() > 1),
        };
    if duplicate_emitted {
        injected.push(DimensionFlag::Duplication);
    }

    // For singleton non-recursive structs we may grant `Key` (with `Drop + Store` — needed
    // for global ops). Enums and recursive structs can't be `Key`-able under our current
    // surface.
    let key_eligible = cfg.enable_globals
        && matches!(&layout, StructLayout::Singleton(fs) if fs.iter().all(|f| !matches!(&f.ty, Type::Struct(qs, _) if qs.name == name)));
    let want_key = key_eligible && u.int_in_range(0u8..=3)? != 0;
    // A signer field has no `store`, so a key struct containing one violates field-ability
    // requirements. The violation path below retains the unsupported declared abilities.
    let layout = if abilities_violation && want_key {
        if let StructLayout::Singleton(mut fields) = layout {
            if let Some(first) = fields.first_mut() {
                first.ty = Type::Signer;
            }
            StructLayout::Singleton(fields)
        } else {
            layout
        }
    } else {
        layout
    };
    let desired_abilities = if want_key {
        AbilitySet::EMPTY
            .add(Ability::Drop)
            .add(Ability::Store)
            .add(Ability::Key)
    } else {
        AbilitySet::PRIMITIVES
    };
    // Build a temporary declaration to check which desired abilities its fields support.
    let probe = StructDecl {
        name: name.clone(),
        type_params: type_params.clone(),
        abilities: desired_abilities,
        layout: layout.clone(),
    };
    // Struct abilities are conditional on the type arguments, so the verifier checks fields
    // with every type parameter assumed to have all abilities.
    let type_param_constraints: Vec<AbilitySet> = vec![AbilitySet::ALL; type_params.len()];
    // Expose the probe itself for self-lookup so self-referential fields (struct-defs /
    // instantiation-loops layouts) resolve to the struct's *declared* abilities — which is
    // what the verifier's field-ability check reads — instead of falling back to EMPTY and
    // inflating the mismatch below.
    let known: Vec<StructDecl> = earlier.iter().cloned().chain([probe.clone()]).collect();
    let ctx = AbilityCtx {
        local_module_id: Some(module_id),
        structs: &known,
        external_modules: &[],
        type_param_constraints: &type_param_constraints,
    };
    let supported_abilities = desired_abilities.intersect(implied_struct_abilities(&probe, &ctx));
    // Record an abilities violation only when retaining the desired set overstates what
    // the fields support. Otherwise, keep the supported subset.
    let abilities = if abilities_violation {
        if supported_abilities != desired_abilities {
            injected.push(DimensionFlag::Abilities);
        }
        desired_abilities
    } else {
        supported_abilities
    };
    Ok(StructDecl {
        name,
        type_params,
        abilities,
        layout,
    })
}

fn build_field_name(duplicate: bool, fi: u8) -> Identifier {
    if duplicate && fi > 0 {
        Identifier::new("f0").expect("ident")
    } else {
        Identifier::new(format!("f{}", fi)).expect("ident")
    }
}

/// A field type: a primitive, or with probability [`Config::rich_field_prob`] a vector of a
/// primitive, or a struct or enum declared earlier in the module (or a vector of one) whose
/// values can be copied, dropped and stored.
fn pick_field_type(
    u: &mut Unstructured<'_>,
    cfg: &Config,
    module_id: &ModuleId,
    earlier: &[StructDecl],
) -> Result<Type> {
    if cfg.rich_field_prob == 0 || !violations::chance(u, cfg.rich_field_prob)? {
        return pick_concrete_primitive(u);
    }
    let nestable_abilities = AbilitySet::PRIMITIVES;
    let nestable: Vec<&StructDecl> = earlier
        .iter()
        .filter(|decl| decl.type_params.is_empty() && nestable_abilities.is_subset(decl.abilities))
        .collect();
    if nestable.is_empty() || u.arbitrary::<bool>()? {
        return Ok(Type::Vector(Box::new(pick_concrete_primitive(u)?)));
    }
    let decl = nestable[u.int_in_range(0..=nestable.len() - 1)?];
    let nested = Type::Struct(
        QualifiedStruct {
            module: module_id.clone(),
            name: decl.name.clone(),
        },
        Vec::new(),
    );
    Ok(if u.arbitrary::<bool>()? {
        nested
    } else {
        Type::Vector(Box::new(nested))
    })
}
