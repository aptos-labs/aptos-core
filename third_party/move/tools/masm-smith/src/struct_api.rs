// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Generates struct API wrappers: functions such as `pack$S` or `borrow_mut$E$0$1` whose name,
//! attribute, signature, and body must agree exactly. The verifier checks them in modules of
//! bytecode version 10 and later.

use crate::{
    config::{Config, DimensionFlag},
    error::Result,
    instructions::{name_instr, simple_op},
    ir::{
        Block, BlockId, Field, Function, Instr, Local, Operand, QualifiedStruct, StructDecl,
        StructLayout, Type, TypeParam,
    },
    violations,
};
use arbitrary::Unstructured;
use move_binary_format::file_format::{FunctionAttribute, Visibility};
use move_core_types::{ability::Ability, identifier::Identifier, language_storage::ModuleId};

/// Wrapper functions plus the dimensions of any violations that were emitted.
pub(crate) struct WrapperOutput {
    pub functions: Vec<Function>,
    pub injected_violations: Vec<DimensionFlag>,
}

/// Builds a subset of struct API wrappers for the local structs of `module_id`. Under
/// `valid_struct_api = false`, one wrapper may be altered to break exactly one of the checker's
/// rules.
pub(crate) fn build_wrappers(
    u: &mut Unstructured<'_>,
    cfg: &Config,
    module_id: &ModuleId,
    structs: &[StructDecl],
) -> Result<WrapperOutput> {
    let mut functions = Vec::new();
    if !cfg.enable_struct_api {
        return Ok(WrapperOutput {
            functions,
            injected_violations: Vec::new(),
        });
    }
    for decl in structs {
        // Struct APIs are disallowed on `key` structs; a recursive field cannot be named in a
        // wrapper signature without the declaration itself being rejected.
        if decl.abilities.has_ability(Ability::Key) || has_self_reference(decl, module_id) {
            continue;
        }
        if !u.arbitrary::<bool>()? {
            continue;
        }
        for spec in candidate_specs(decl) {
            if u.arbitrary::<bool>()? {
                functions.push(build_wrapper(module_id, decl, &spec));
            }
        }
    }
    let mut injected_violations = Vec::new();
    if !functions.is_empty()
        && violations::should_violate(u, cfg, DimensionFlag::StructApi, &mut injected_violations)?
    {
        let pick = u.int_in_range(0..=functions.len() - 1)?;
        let mutation = u.choose(&ALL_MUTATIONS)?;
        apply_mutation(&mut functions[pick], *mutation);
    }
    Ok(WrapperOutput {
        functions,
        injected_violations,
    })
}

/// One wrapper the checker accepts for a given struct.
enum WrapperSpec {
    Pack,
    Unpack,
    PackVariant {
        variant: u16,
    },
    UnpackVariant {
        variant: u16,
    },
    TestVariant {
        variant: u16,
    },
    BorrowField {
        offset: u16,
        is_mut: bool,
    },
    BorrowVariantField {
        offset: u16,
        type_order: u16,
        is_mut: bool,
    },
}

fn candidate_specs(decl: &StructDecl) -> Vec<WrapperSpec> {
    let mut specs = Vec::new();
    match &decl.layout {
        StructLayout::Singleton(fields) => {
            specs.push(WrapperSpec::Pack);
            specs.push(WrapperSpec::Unpack);
            for offset in 0..fields.len() as u16 {
                for is_mut in [false, true] {
                    specs.push(WrapperSpec::BorrowField { offset, is_mut });
                }
            }
        },
        StructLayout::Variants(variants) => {
            for variant in 0..variants.len() as u16 {
                specs.push(WrapperSpec::PackVariant { variant });
                specs.push(WrapperSpec::UnpackVariant { variant });
                specs.push(WrapperSpec::TestVariant { variant });
            }
            for group in variant_field_groups(decl) {
                for is_mut in [false, true] {
                    specs.push(WrapperSpec::BorrowVariantField {
                        offset: group.offset,
                        type_order: group.type_order,
                        is_mut,
                    });
                }
            }
        },
    }
    specs
}

/// Variants sharing a field type at one offset. A variant-field borrow wrapper must cover
/// exactly these variants, in ascending order.
struct VariantFieldGroup {
    offset: u16,
    type_order: u16,
    ty: Type,
    variants: Vec<u16>,
}

/// Groups `(offset, type)` pairs, numbering them by first encounter in declaration order to
/// match the checker's type-order assignment.
fn variant_field_groups(decl: &StructDecl) -> Vec<VariantFieldGroup> {
    let StructLayout::Variants(variants) = &decl.layout else {
        return Vec::new();
    };
    let mut groups: Vec<VariantFieldGroup> = Vec::new();
    for (variant_idx, variant) in variants.iter().enumerate() {
        for (offset, field) in variant.fields.iter().enumerate() {
            let offset = offset as u16;
            match groups
                .iter_mut()
                .find(|group| group.offset == offset && group.ty == field.ty)
            {
                Some(group) => group.variants.push(variant_idx as u16),
                None => {
                    let type_order = groups.len() as u16;
                    groups.push(VariantFieldGroup {
                        offset,
                        type_order,
                        ty: field.ty.clone(),
                        variants: vec![variant_idx as u16],
                    });
                },
            }
        }
    }
    groups
}

fn build_wrapper(module_id: &ModuleId, decl: &StructDecl, spec: &WrapperSpec) -> Function {
    let qs = QualifiedStruct {
        module: module_id.clone(),
        name: decl.name.clone(),
    };
    // Wrappers use the canonical instantiation `S<T0, .., Tn-1>` throughout.
    let type_args: Vec<Type> = (0..decl.type_params.len() as u16)
        .map(Type::TypeParam)
        .collect();
    let self_ty = Type::Struct(qs.clone(), type_args.clone());
    let struct_operand = Operand::Struct(qs.clone(), type_args.clone());
    let variant_name = |variant: u16| match &decl.layout {
        StructLayout::Variants(variants) => variants[variant as usize].name.clone(),
        StructLayout::Singleton(_) => unreachable!("variant wrapper on a singleton struct"),
    };
    let (name, attribute, params, return_types, op) = match spec {
        WrapperSpec::Pack => {
            let fields = singleton_fields(decl);
            (
                format!("pack${}", decl.name),
                FunctionAttribute::Pack,
                field_params(fields),
                vec![self_ty],
                Instr {
                    opcode: "pack".to_string(),
                    operands: vec![struct_operand],
                },
            )
        },
        WrapperSpec::Unpack => (
            format!("unpack${}", decl.name),
            FunctionAttribute::Unpack,
            vec![param(0, self_ty)],
            field_types(singleton_fields(decl)),
            Instr {
                opcode: "unpack".to_string(),
                operands: vec![struct_operand],
            },
        ),
        WrapperSpec::PackVariant { variant } => {
            let name = variant_name(*variant);
            (
                format!("pack${}${}", decl.name, name),
                FunctionAttribute::PackVariant(*variant),
                field_params(variant_fields(decl, *variant)),
                vec![self_ty],
                Instr {
                    opcode: "pack_variant".to_string(),
                    operands: vec![struct_operand, Operand::Local(name)],
                },
            )
        },
        WrapperSpec::UnpackVariant { variant } => {
            let name = variant_name(*variant);
            (
                format!("unpack${}${}", decl.name, name),
                FunctionAttribute::UnpackVariant(*variant),
                vec![param(0, self_ty)],
                field_types(variant_fields(decl, *variant)),
                Instr {
                    opcode: "unpack_variant".to_string(),
                    operands: vec![struct_operand, Operand::Local(name)],
                },
            )
        },
        WrapperSpec::TestVariant { variant } => {
            let name = variant_name(*variant);
            (
                format!("test_variant${}${}", decl.name, name),
                FunctionAttribute::TestVariant(*variant),
                vec![param(0, Type::Reference(false, Box::new(self_ty)))],
                vec![Type::Bool],
                Instr {
                    opcode: "test_variant".to_string(),
                    operands: vec![struct_operand, Operand::Local(name)],
                },
            )
        },
        WrapperSpec::BorrowField { offset, is_mut } => {
            let field = &singleton_fields(decl)[*offset as usize];
            (
                format!("{}${}${}", borrow_prefix(*is_mut), decl.name, offset),
                borrow_attribute(*offset, *is_mut),
                vec![param(0, Type::Reference(*is_mut, Box::new(self_ty)))],
                vec![Type::Reference(*is_mut, Box::new(field.ty.clone()))],
                Instr {
                    opcode: borrow_opcode("borrow_field", *is_mut),
                    operands: vec![Operand::Field {
                        owner: qs,
                        type_args,
                        field: field.name.clone(),
                    }],
                },
            )
        },
        WrapperSpec::BorrowVariantField {
            offset,
            type_order,
            is_mut,
        } => {
            let group = variant_field_groups(decl)
                .into_iter()
                .find(|group| group.type_order == *type_order)
                .expect("type order comes from variant_field_groups");
            let StructLayout::Variants(variants) = &decl.layout else {
                unreachable!("variant field wrapper on a singleton struct")
            };
            let fields = group
                .variants
                .iter()
                .map(|variant| {
                    let variant = &variants[*variant as usize];
                    (
                        variant.name.clone(),
                        variant.fields[*offset as usize].name.clone(),
                    )
                })
                .collect();
            (
                format!(
                    "{}${}${}${}",
                    borrow_prefix(*is_mut),
                    decl.name,
                    offset,
                    type_order
                ),
                borrow_attribute(*offset, *is_mut),
                vec![param(0, Type::Reference(*is_mut, Box::new(self_ty)))],
                vec![Type::Reference(*is_mut, Box::new(group.ty))],
                Instr {
                    opcode: borrow_opcode("borrow_variant_field", *is_mut),
                    operands: vec![Operand::VariantFields {
                        owner: qs,
                        type_args,
                        fields,
                    }],
                },
            )
        },
    };
    let mut instrs: Vec<Instr> = params
        .iter()
        .map(|local| name_instr("move_loc", local.name.clone()))
        .collect();
    instrs.push(op);
    instrs.push(simple_op("ret"));
    Function {
        name: Identifier::new(name).expect("struct API names are valid identifiers"),
        visibility: Visibility::Public,
        is_entry: false,
        is_native: false,
        attributes: vec![attribute],
        type_params: wrapper_type_params(&decl.type_params),
        params,
        locals: vec![],
        return_types,
        acquires: vec![],
        body: vec![Block {
            id: BlockId(0),
            instrs,
        }],
    }
}

/// A single-rule violation applied to an otherwise valid wrapper.
#[derive(Clone, Copy)]
enum Mutation {
    /// Struct API name without the attribute.
    DropAttribute,
    /// Attribute on a function whose name is not a struct API name.
    RenameWithoutApiName,
    /// Two struct API attributes on one function.
    DuplicateAttribute,
    /// Variant index or field offset in the attribute disagrees with the name.
    ShiftAttributeIndex,
    /// An extra instruction breaks the `move_loc* <op> ret` shape.
    ExtraInstruction,
    /// The wrapper declares one more type parameter than its struct.
    ExtraTypeParam,
}

const ALL_MUTATIONS: [Mutation; 6] = [
    Mutation::DropAttribute,
    Mutation::RenameWithoutApiName,
    Mutation::DuplicateAttribute,
    Mutation::ShiftAttributeIndex,
    Mutation::ExtraInstruction,
    Mutation::ExtraTypeParam,
];

fn apply_mutation(function: &mut Function, mutation: Mutation) {
    match mutation {
        Mutation::DropAttribute => function.attributes.clear(),
        Mutation::RenameWithoutApiName => {
            function.name = Identifier::new(format!("not_api_{}", function.name.as_str()))
                .expect("prefixed struct API names are valid identifiers");
        },
        Mutation::DuplicateAttribute => {
            let duplicate = match function.attributes[0] {
                FunctionAttribute::Pack => FunctionAttribute::Unpack,
                _ => FunctionAttribute::Pack,
            };
            function.attributes.push(duplicate);
        },
        Mutation::ShiftAttributeIndex => {
            let shifted = match function.attributes[0] {
                FunctionAttribute::PackVariant(idx) => {
                    FunctionAttribute::PackVariant(idx.wrapping_add(1))
                },
                FunctionAttribute::UnpackVariant(idx) => {
                    FunctionAttribute::UnpackVariant(idx.wrapping_add(1))
                },
                FunctionAttribute::TestVariant(idx) => {
                    FunctionAttribute::TestVariant(idx.wrapping_add(1))
                },
                FunctionAttribute::BorrowFieldImmutable(offset) => {
                    FunctionAttribute::BorrowFieldImmutable(offset.wrapping_add(1))
                },
                FunctionAttribute::BorrowFieldMutable(offset) => {
                    FunctionAttribute::BorrowFieldMutable(offset.wrapping_add(1))
                },
                // Pack/Unpack carry no index; swap the attribute kind instead.
                FunctionAttribute::Pack => FunctionAttribute::Unpack,
                FunctionAttribute::Unpack => FunctionAttribute::Pack,
                FunctionAttribute::Persistent | FunctionAttribute::ModuleLock => {
                    unreachable!("wrappers carry exactly one struct API attribute")
                },
            };
            function.attributes[0] = shifted;
        },
        Mutation::ExtraInstruction => {
            function.body[0].instrs.insert(0, simple_op("nop"));
        },
        Mutation::ExtraTypeParam => {
            let idx = function.type_params.len();
            function.type_params.push(TypeParam {
                name: Identifier::new(format!("T{}", idx)).expect("ident"),
                constraints: move_core_types::ability::AbilitySet::EMPTY,
                is_phantom: false,
            });
        },
    }
}

fn has_self_reference(decl: &StructDecl, module_id: &ModuleId) -> bool {
    decl.layout
        .all_fields()
        .any(|field| mentions_struct(&field.ty, module_id, &decl.name))
}

fn mentions_struct(ty: &Type, module_id: &ModuleId, name: &Identifier) -> bool {
    ty.any(&|ty| matches!(ty, Type::Struct(qs, _) if &qs.module == module_id && &qs.name == name))
}

/// Wrapper type parameters mirror the struct's, minus phantom-ness (functions have none).
fn wrapper_type_params(struct_params: &[TypeParam]) -> Vec<TypeParam> {
    struct_params
        .iter()
        .map(|tp| TypeParam {
            name: tp.name.clone(),
            constraints: tp.constraints,
            is_phantom: false,
        })
        .collect()
}

fn singleton_fields(decl: &StructDecl) -> &[Field] {
    match &decl.layout {
        StructLayout::Singleton(fields) => fields,
        StructLayout::Variants(_) => unreachable!("singleton wrapper on an enum"),
    }
}

fn variant_fields(decl: &StructDecl, variant: u16) -> &[Field] {
    match &decl.layout {
        StructLayout::Variants(variants) => &variants[variant as usize].fields,
        StructLayout::Singleton(_) => unreachable!("variant wrapper on a singleton struct"),
    }
}

fn param(idx: usize, ty: Type) -> Local {
    Local {
        name: Identifier::new(format!("p{}", idx)).expect("ident"),
        ty,
    }
}

fn field_params(fields: &[Field]) -> Vec<Local> {
    fields
        .iter()
        .enumerate()
        .map(|(idx, field)| param(idx, field.ty.clone()))
        .collect()
}

fn field_types(fields: &[Field]) -> Vec<Type> {
    fields.iter().map(|field| field.ty.clone()).collect()
}

fn borrow_prefix(is_mut: bool) -> &'static str {
    if is_mut {
        "borrow_mut"
    } else {
        "borrow"
    }
}

fn borrow_attribute(offset: u16, is_mut: bool) -> FunctionAttribute {
    if is_mut {
        FunctionAttribute::BorrowFieldMutable(offset)
    } else {
        FunctionAttribute::BorrowFieldImmutable(offset)
    }
}

fn borrow_opcode(base: &str, is_mut: bool) -> String {
    if is_mut {
        format!("mut_{}", base)
    } else {
        base.to_string()
    }
}
