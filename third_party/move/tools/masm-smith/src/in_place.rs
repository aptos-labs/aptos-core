// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Exercises operations on values reached through references: vector updates, field writes, and
//! container equality.
//!
//! Generated code usually builds a value and consumes it right away. [`try_work_in_place`]
//! instead stores a value in a local and applies a few operations to it through references.
//! Indices stay in bounds and variants match, except for a rare deliberate miss that exercises
//! the VM's error paths.

use crate::{
    abilities::{has_copy, has_drop},
    error::Result,
    instructions::{
        ability_ctx, imm_instr, is_synthesizable_primitive, name_instr, push_synthesized_value,
        simple_op, std_call, type_instr,
    },
    ir::{Instr, Operand, QualifiedStruct, StructLayout, Type, Value},
    state::{FunctionState, LocalInit},
    types::{pick_concrete_primitive, pick_concrete_primitives, substitute},
};
use arbitrary::Unstructured;
use move_core_types::identifier::Identifier;

/// Stores a value in a fresh local and applies one to four in-place operations to it.
pub(crate) fn try_work_in_place(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    let subject = pick_subject(u, state)?;
    let subject_ty = subject.ty();
    let ctx = ability_ctx(state);
    if !has_drop(&subject_ty, &ctx) {
        return Ok(false);
    }
    let copyable = has_copy(&subject_ty, &ctx);
    let Some(local) = state.alloc_scratch_local(subject_ty.clone()) else {
        return Ok(false);
    };
    let mut work = Work {
        local,
        subject,
        copyable,
        shape: Shape::Elements(Vec::new()),
        instrs: Vec::new(),
    };
    work.shape = work.push_subject_value(u, state)?;
    work.emit(name_instr("st_loc", work.local.clone()));
    for _ in 0..u.int_in_range(1..=4)? {
        work.apply_operation(u, state)?;
    }
    let slot = state
        .locals
        .iter_mut()
        .find(|slot| slot.name == work.local)
        .expect("scratch local was just allocated");
    if u.ratio(1u8, 4u8)? {
        slot.init = LocalInit::Moved;
        work.emit(name_instr("move_loc", work.local.clone()));
        work.emit(simple_op("pop"));
    } else {
        // Left for other generators to read, borrow or overwrite.
        slot.init = LocalInit::Available;
    }
    instrs.extend(work.instrs);
    Ok(true)
}

/// Longest vector the generator builds or grows to.
const VECTOR_LEN_MAX: usize = 6;

/// One in 128 indexed or variant-field operations misses (an index one past the end, or a
/// field of another variant), so the VM aborts; a program runs many of these operations, and
/// each miss discards its transaction's writes.
const MISS_PROB_DENOMINATOR: u8 = 128;

/// The value kept in the local.
#[derive(Clone)]
enum Subject {
    Vector(Kind),
    Single(Kind),
}

impl Subject {
    fn ty(&self) -> Type {
        match self {
            Subject::Vector(kind) => Type::Vector(Box::new(kind.ty())),
            Subject::Single(kind) => kind.ty(),
        }
    }
}

/// A vector element type or a single value's type.
#[derive(Clone)]
enum Kind {
    Primitive(Type),
    Struct {
        owner: QualifiedStruct,
        type_args: Vec<Type>,
        fields: Vec<(Identifier, Type)>,
    },
    Enum {
        owner: QualifiedStruct,
        type_args: Vec<Type>,
        variants: Vec<(Identifier, Vec<(Identifier, Type)>)>,
    },
}

impl Kind {
    fn ty(&self) -> Type {
        match self {
            Kind::Primitive(ty) => ty.clone(),
            Kind::Struct {
                owner, type_args, ..
            }
            | Kind::Enum {
                owner, type_args, ..
            } => Type::Struct(owner.clone(), type_args.clone()),
        }
    }
}

/// What is known about the local's current value: the variant of each vector element, or of
/// the single value (`None` for values that are not enums).
#[derive(Clone)]
enum Shape {
    Elements(Vec<Option<usize>>),
    Single(Option<usize>),
}

/// A vector element type or single value: a primitive, or a struct or enum of the current
/// module whose fields are all primitives once its type parameters are bound to primitives.
fn pick_subject(u: &mut Unstructured<'_>, state: &FunctionState<'_, '_>) -> Result<Subject> {
    let composites = composite_kinds(u, state)?;
    let as_vector = composites.is_empty() || u.arbitrary::<bool>()?;
    if as_vector {
        let elem = if !composites.is_empty() && u.arbitrary::<bool>()? {
            u.choose(&composites)?.clone()
        } else {
            Kind::Primitive(pick_concrete_primitive(u)?)
        };
        return Ok(Subject::Vector(elem));
    }
    Ok(Subject::Single(u.choose(&composites)?.clone()))
}

fn composite_kinds(u: &mut Unstructured<'_>, state: &FunctionState<'_, '_>) -> Result<Vec<Kind>> {
    let Some(module) = state.module_ctx.module_id.clone() else {
        return Ok(Vec::new());
    };
    let mut kinds = Vec::new();
    for decl in &state.module_ctx.structs {
        let type_args = pick_concrete_primitives(u, decl.type_params.len())?;
        let owner = QualifiedStruct {
            module: module.clone(),
            name: decl.name.clone(),
        };
        let concrete = |fields: &[crate::ir::Field]| -> Option<Vec<(Identifier, Type)>> {
            fields
                .iter()
                .map(|field| {
                    let ty = substitute(&field.ty, &type_args);
                    is_synthesizable_primitive(&ty).then(|| (field.name.clone(), ty))
                })
                .collect()
        };
        let kind = match &decl.layout {
            StructLayout::Singleton(fields) => concrete(fields).map(|fields| Kind::Struct {
                owner,
                type_args: type_args.clone(),
                fields,
            }),
            StructLayout::Variants(variants) if !variants.is_empty() => variants
                .iter()
                .map(|variant| {
                    concrete(&variant.fields).map(|fields| (variant.name.clone(), fields))
                })
                .collect::<Option<Vec<_>>>()
                .map(|variants| Kind::Enum {
                    owner,
                    type_args: type_args.clone(),
                    variants,
                }),
            StructLayout::Variants(_) => None,
        };
        kinds.extend(kind);
    }
    let ctx = ability_ctx(state);
    kinds.retain(|kind| has_drop(&kind.ty(), &ctx));
    Ok(kinds)
}

/// The in-progress sequence for one subject.
struct Work {
    local: Identifier,
    subject: Subject,
    copyable: bool,
    shape: Shape,
    instrs: Vec<Instr>,
}

impl Work {
    fn emit(&mut self, instr: Instr) {
        self.instrs.push(instr);
    }

    fn borrow(&mut self, mutable: bool) {
        let opcode = if mutable {
            "mut_borrow_loc"
        } else {
            "borrow_loc"
        };
        self.emit(name_instr(opcode, self.local.clone()));
    }

    fn push_u64(&mut self, value: u64) {
        self.emit(imm_instr("ld_u64", Value::U64(value)));
    }

    /// Pushes a value of `kind`, returning its variant for enums. The generator's stack model
    /// is left unchanged: every operation consumes what it pushes.
    fn push_kind_value(
        &mut self,
        u: &mut Unstructured<'_>,
        state: &mut FunctionState<'_, '_>,
        kind: &Kind,
    ) -> Result<Option<usize>> {
        let fields: Vec<Type>;
        let (opcode, variant) = match kind {
            Kind::Primitive(ty) => {
                push_synthesized_value(u, state, &mut self.instrs, ty)?;
                state.pop();
                return Ok(None);
            },
            Kind::Struct { fields: f, .. } => {
                fields = f.iter().map(|(_, ty)| ty.clone()).collect();
                ("pack", None)
            },
            Kind::Enum { variants, .. } => {
                let variant = u.int_in_range(0..=variants.len() - 1)?;
                fields = variants[variant]
                    .1
                    .iter()
                    .map(|(_, ty)| ty.clone())
                    .collect();
                ("pack_variant", Some(variant))
            },
        };
        for ty in &fields {
            push_synthesized_value(u, state, &mut self.instrs, ty)?;
        }
        for _ in &fields {
            state.pop();
        }
        let (owner, type_args) = match kind {
            Kind::Struct {
                owner, type_args, ..
            }
            | Kind::Enum {
                owner, type_args, ..
            } => (owner.clone(), type_args.clone()),
            Kind::Primitive(_) => unreachable!("primitives returned above"),
        };
        let mut operands = vec![Operand::Struct(owner, type_args)];
        if let (Kind::Enum { variants, .. }, Some(variant)) = (kind, variant) {
            operands.push(Operand::Local(variants[variant].0.clone()));
        }
        self.emit(Instr {
            opcode: opcode.to_string(),
            operands,
        });
        Ok(variant)
    }

    /// Pushes a new value of the subject's type, returning its shape.
    fn push_subject_value(
        &mut self,
        u: &mut Unstructured<'_>,
        state: &mut FunctionState<'_, '_>,
    ) -> Result<Shape> {
        match self.subject.clone() {
            Subject::Vector(elem) => {
                let len = u.int_in_range(0..=VECTOR_LEN_MAX - 2)?;
                let elements = self.push_elements(u, state, &elem, len)?;
                Ok(Shape::Elements(elements))
            },
            Subject::Single(kind) => Ok(Shape::Single(self.push_kind_value(u, state, &kind)?)),
        }
    }

    /// Pushes `len` elements of `elem` and packs them into a vector.
    fn push_elements(
        &mut self,
        u: &mut Unstructured<'_>,
        state: &mut FunctionState<'_, '_>,
        elem: &Kind,
        len: usize,
    ) -> Result<Vec<Option<usize>>> {
        let elements = (0..len)
            .map(|_| self.push_kind_value(u, state, elem))
            .collect::<Result<Vec<_>>>()?;
        self.emit(Instr {
            opcode: "vec_pack".to_string(),
            operands: vec![
                Operand::Type(elem.ty()),
                Operand::Imm(Value::U64(len as u64)),
            ],
        });
        Ok(elements)
    }

    /// An index into a vector of `len` elements: in bounds, or rarely one past the end.
    fn index(&self, u: &mut Unstructured<'_>, len: usize) -> Result<u64> {
        if len == 0 || u.ratio(1u8, MISS_PROB_DENOMINATOR)? {
            return Ok(len as u64);
        }
        Ok(u.int_in_range(0..=len - 1)? as u64)
    }

    fn apply_operation(
        &mut self,
        u: &mut Unstructured<'_>,
        state: &mut FunctionState<'_, '_>,
    ) -> Result<()> {
        // Whole-value operations apply to every subject.
        match u.int_in_range(0u8..=5)? {
            0 => return self.compare_whole(u),
            1 => return self.write_whole(u, state),
            _ => {},
        }
        match (self.subject.clone(), self.shape.clone()) {
            (Subject::Vector(elem), Shape::Elements(elements)) => {
                self.vector_operation(u, state, &elem, elements)
            },
            (Subject::Single(kind), Shape::Single(variant)) => {
                self.single_operation(u, state, &kind, variant)
            },
            (Subject::Vector(_), Shape::Single(_)) | (Subject::Single(_), Shape::Elements(_)) => {
                unreachable!("shape always follows the subject")
            },
        }
    }

    /// `==` or `!=` of the value with itself, through references or as copies.
    fn compare_whole(&mut self, u: &mut Unstructured<'_>) -> Result<()> {
        if self.copyable && u.arbitrary::<bool>()? {
            self.emit(name_instr("copy_loc", self.local.clone()));
            self.emit(name_instr("copy_loc", self.local.clone()));
        } else {
            self.borrow(false);
            self.borrow(false);
        }
        self.emit(simple_op(if u.arbitrary::<bool>()? { "eq" } else { "neq" }));
        self.emit(simple_op("pop"));
        Ok(())
    }

    /// Replaces the whole value through a mutable reference.
    fn write_whole(
        &mut self,
        u: &mut Unstructured<'_>,
        state: &mut FunctionState<'_, '_>,
    ) -> Result<()> {
        self.shape = self.push_subject_value(u, state)?;
        self.borrow(true);
        self.emit(simple_op("write_ref"));
        Ok(())
    }

    fn vector_operation(
        &mut self,
        u: &mut Unstructured<'_>,
        state: &mut FunctionState<'_, '_>,
        elem: &Kind,
        mut elements: Vec<Option<usize>>,
    ) -> Result<()> {
        let elem_ty = elem.ty();
        let len = elements.len();
        let natives = state.cfg.enable_natives;
        match u.int_in_range(0u8..=13)? {
            // push_back
            0 if len < VECTOR_LEN_MAX => {
                self.borrow(true);
                let variant = self.push_kind_value(u, state, elem)?;
                self.emit(vec_op("vec_push_back", &elem_ty));
                elements.push(variant);
            },
            // pop_back
            1 if len > 0 => {
                self.borrow(true);
                self.emit(vec_op("vec_pop_back", &elem_ty));
                self.emit(simple_op("pop"));
                elements.pop();
            },
            // swap
            2 if len > 0 => {
                let (first, second) = (self.index(u, len)?, self.index(u, len)?);
                self.borrow(true);
                self.push_u64(first);
                self.push_u64(second);
                self.emit(vec_op("vec_swap", &elem_ty));
                if first < len as u64 && second < len as u64 {
                    elements.swap(first as usize, second as usize);
                }
            },
            // write an element
            3 if len > 0 => {
                let index = self.index(u, len)?;
                let variant = self.push_kind_value(u, state, elem)?;
                self.borrow(true);
                self.push_u64(index);
                self.emit(vec_op("vec_mut_borrow", &elem_ty));
                self.emit(simple_op("write_ref"));
                if let Some(slot) = elements.get_mut(index as usize) {
                    *slot = variant;
                }
            },
            // compare two elements through references
            4 if len > 0 => {
                let (first, second) = (self.index(u, len)?, self.index(u, len)?);
                for index in [first, second] {
                    self.borrow(false);
                    self.push_u64(index);
                    self.emit(vec_op("vec_borrow", &elem_ty));
                }
                self.emit(simple_op(if u.arbitrary::<bool>()? { "eq" } else { "neq" }));
                self.emit(simple_op("pop"));
            },
            // read an element
            5 if len > 0 && self.copyable => {
                let index = self.index(u, len)?;
                self.borrow(false);
                self.push_u64(index);
                self.emit(vec_op("vec_borrow", &elem_ty));
                self.emit(simple_op("read_ref"));
                self.emit(simple_op("pop"));
            },
            // write a field of a struct element
            7 if len > 0 => {
                if let Kind::Struct {
                    owner,
                    type_args,
                    fields,
                } = elem
                    && !fields.is_empty()
                {
                    let index = self.index(u, len)?;
                    let field = u.choose(fields)?.clone();
                    push_synthesized_value(u, state, &mut self.instrs, &field.1)?;
                    state.pop();
                    self.borrow(true);
                    self.push_u64(index);
                    self.emit(vec_op("vec_mut_borrow", &elem_ty));
                    self.emit(field_op("mut_borrow_field", owner, type_args, &field.0));
                    self.emit(simple_op("write_ref"));
                }
            },
            // test or write a variant field of an enum element
            8 if len > 0 => {
                if let Kind::Enum {
                    owner,
                    type_args,
                    variants,
                } = elem
                {
                    let index = self.index(u, len)?;
                    let known = elements.get(index as usize).copied().flatten();
                    self.borrow_variant_field_or_test(
                        u,
                        state,
                        owner,
                        type_args,
                        variants,
                        known,
                        |work| {
                            work.borrow(true);
                            work.push_u64(index);
                            work.emit(vec_op("vec_mut_borrow", &elem_ty));
                        },
                        |work| {
                            work.borrow(false);
                            work.push_u64(index);
                            work.emit(vec_op("vec_borrow", &elem_ty));
                        },
                    )?;
                }
            },
            // stdlib: reverse
            9 if natives => {
                self.borrow(true);
                self.emit(vector_call("reverse", &elem_ty));
                elements.reverse();
            },
            // stdlib: contains or index_of an element of the vector
            10 if natives && len > 0 => {
                let index = self.index(u, len)?;
                let index_of = u.arbitrary::<bool>()?;
                self.borrow(false);
                self.borrow(false);
                self.push_u64(index);
                self.emit(vec_op("vec_borrow", &elem_ty));
                self.emit(vector_call(
                    if index_of { "index_of" } else { "contains" },
                    &elem_ty,
                ));
                self.emit(simple_op("pop"));
                if index_of {
                    self.emit(simple_op("pop"));
                }
            },
            // stdlib: append another vector
            11 if natives && len < VECTOR_LEN_MAX => {
                let extra = u.int_in_range(0..=VECTOR_LEN_MAX - len)?;
                self.borrow(true);
                let appended = self.push_elements(u, state, elem, extra)?;
                self.emit(vector_call("append", &elem_ty));
                elements.extend(appended);
            },
            // stdlib: remove or swap_remove
            12 if natives && len > 0 => {
                let index = self.index(u, len)?;
                let swap_remove = u.arbitrary::<bool>()?;
                self.borrow(true);
                self.push_u64(index);
                self.emit(vector_call(
                    if swap_remove { "swap_remove" } else { "remove" },
                    &elem_ty,
                ));
                self.emit(simple_op("pop"));
                if (index as usize) < len {
                    if swap_remove {
                        elements.swap_remove(index as usize);
                    } else {
                        elements.remove(index as usize);
                    }
                }
            },
            // stdlib: is_empty
            13 if natives => {
                self.borrow(false);
                self.emit(vector_call("is_empty", &elem_ty));
                self.emit(simple_op("pop"));
            },
            // length, also the fallback when the chosen operation does not apply
            _ => {
                self.borrow(false);
                self.emit(vec_op("vec_len", &elem_ty));
                self.emit(simple_op("pop"));
            },
        }
        self.shape = Shape::Elements(elements);
        Ok(())
    }

    fn single_operation(
        &mut self,
        u: &mut Unstructured<'_>,
        state: &mut FunctionState<'_, '_>,
        kind: &Kind,
        variant: Option<usize>,
    ) -> Result<()> {
        match kind {
            Kind::Struct {
                owner,
                type_args,
                fields,
            } if !fields.is_empty() => {
                let field = u.choose(fields)?.clone();
                if u.arbitrary::<bool>()? {
                    push_synthesized_value(u, state, &mut self.instrs, &field.1)?;
                    state.pop();
                    self.borrow(true);
                    self.emit(field_op("mut_borrow_field", owner, type_args, &field.0));
                    self.emit(simple_op("write_ref"));
                } else {
                    self.borrow(false);
                    self.emit(field_op("borrow_field", owner, type_args, &field.0));
                    self.emit(simple_op("read_ref"));
                    self.emit(simple_op("pop"));
                }
            },
            Kind::Enum {
                owner,
                type_args,
                variants,
            } => {
                self.borrow_variant_field_or_test(
                    u,
                    state,
                    owner,
                    type_args,
                    variants,
                    variant,
                    |work| work.borrow(true),
                    |work| work.borrow(false),
                )?;
            },
            Kind::Struct { .. } | Kind::Primitive(_) => self.compare_whole(u)?,
        }
        Ok(())
    }

    /// Tests the variant of an enum reached by `borrow_imm`, or writes or reads one of its
    /// variant fields. Field accesses use the enum's current variant when it is `known`, and
    /// rarely another one, which aborts with a variant mismatch.
    #[allow(clippy::too_many_arguments)]
    fn borrow_variant_field_or_test(
        &mut self,
        u: &mut Unstructured<'_>,
        state: &mut FunctionState<'_, '_>,
        owner: &QualifiedStruct,
        type_args: &[Type],
        variants: &[(Identifier, Vec<(Identifier, Type)>)],
        known: Option<usize>,
        borrow_mut: impl Fn(&mut Work),
        borrow_imm: impl Fn(&mut Work),
    ) -> Result<()> {
        let miss = u.ratio(1u8, MISS_PROB_DENOMINATOR)?;
        let variant = match known {
            Some(variant) if !miss => variant,
            _ => u.int_in_range(0..=variants.len() - 1)?,
        };
        let (variant_name, fields) = &variants[variant];
        if fields.is_empty() || u.ratio(1u8, 3u8)? {
            let tested = u.int_in_range(0..=variants.len() - 1)?;
            borrow_imm(self);
            self.emit(Instr {
                opcode: "test_variant".to_string(),
                operands: vec![
                    Operand::Struct(owner.clone(), type_args.to_vec()),
                    Operand::Local(variants[tested].0.clone()),
                ],
            });
            self.emit(simple_op("pop"));
            return Ok(());
        }
        let (field_name, field_ty) = u.choose(fields)?.clone();
        let operand = Operand::VariantFields {
            owner: owner.clone(),
            type_args: type_args.to_vec(),
            fields: vec![(variant_name.clone(), field_name)],
        };
        if u.arbitrary::<bool>()? {
            push_synthesized_value(u, state, &mut self.instrs, &field_ty)?;
            state.pop();
            borrow_mut(self);
            self.emit(Instr {
                opcode: "mut_borrow_variant_field".to_string(),
                operands: vec![operand],
            });
            self.emit(simple_op("write_ref"));
        } else {
            borrow_imm(self);
            self.emit(Instr {
                opcode: "borrow_variant_field".to_string(),
                operands: vec![operand],
            });
            self.emit(simple_op("read_ref"));
            self.emit(simple_op("pop"));
        }
        Ok(())
    }
}

fn vec_op(opcode: &str, elem_ty: &Type) -> Instr {
    type_instr(opcode, elem_ty.clone())
}

fn field_op(
    opcode: &str,
    owner: &QualifiedStruct,
    type_args: &[Type],
    field: &Identifier,
) -> Instr {
    Instr {
        opcode: opcode.to_string(),
        operands: vec![Operand::Field {
            owner: owner.clone(),
            type_args: type_args.to_vec(),
            field: field.clone(),
        }],
    }
}

/// A call to a Move-defined function of the stdlib `vector` module.
fn vector_call(name: &str, elem_ty: &Type) -> Instr {
    std_call("vector", name, vec![elem_ty.clone()])
}
