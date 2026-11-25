// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Emits calls to the stdlib functions the test VM implements as natives (`bcs`, `cmp`, `hash`,
//! `mem`, `signer`, `string`, `type_name`, `vector`).
//!
//! Each call sequence starts and ends with types the generator already models, so stdlib types
//! such as `String` never enter its type system.

use crate::{
    borrows::{BorrowRoot, Mutability},
    error::Result,
    instructions::{ability_ctx, imm_instr, name_instr, simple_op, std_call},
    ir::{Instr, MasmTest, Operand, Step, Type, Value},
    state::{FunctionState, LocalInit, LOOP_COUNTER_NAME},
    types::{
        is_reference, is_signer_ref, is_valid_type_arg, mentions_function, pick_concrete_type,
    },
};
use arbitrary::Unstructured;
use move_core_types::{
    ability::AbilitySet, account_address::AccountAddress, identifier::Identifier,
    language_storage::ModuleId,
};

pub(crate) fn try_call_native(
    u: &mut Unstructured<'_>,
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
) -> Result<bool> {
    if !state.cfg.enable_natives {
        return Ok(false);
    }
    let mut candidates: Vec<NativeCall> = Vec::new();
    if let Some(top @ Type::Reference(_, inner)) = state.top() {
        if is_signer_ref(top) {
            candidates.push(NativeCall::SignerAddress);
        }
        // BCS calls require a valid type argument. The generator excludes direct signer values.
        let function_value_ok = state.cfg.enable_function_value_ops || !mentions_function(inner);
        if is_valid_type_arg(inner) && !matches!(**inner, Type::Signer) && function_value_ok {
            candidates.extend([
                NativeCall::BcsToBytes,
                NativeCall::SerializedSize,
                NativeCall::CompareSelf,
            ]);
            if top_two_same_immutable_refs(state) {
                candidates.push(NativeCall::CompareTop);
            }
        }
    } else if let Some(Type::Vector(elem)) = state.top()
        && matches!(**elem, Type::U8)
    {
        candidates.extend([
            NativeCall::Sha2,
            NativeCall::Sha3,
            NativeCall::Utf8RoundTrip,
            NativeCall::SubString,
            NativeCall::IndexOf,
        ]);
    }
    let swappable = swappable_locals(state);
    if !swappable.is_empty() {
        candidates.push(NativeCall::Swap);
    }
    let vector_pairs = vector_local_pairs(state);
    if !vector_pairs.is_empty() {
        candidates.push(NativeCall::MoveRange);
    }
    // `type_name::get` needs no stack input, so offering it everywhere would crowd out the
    // stack-driven calls.
    if candidates.is_empty() {
        if !state.cfg.enable_type_name_get || !u.arbitrary::<bool>()? {
            return Ok(false);
        }
        candidates.push(NativeCall::TypeName);
    }
    let call = *u.choose(&candidates)?;
    match call {
        NativeCall::BcsToBytes => emit_bcs_call(state, instrs, "to_bytes", bytes_ty()),
        NativeCall::SerializedSize => emit_bcs_call(state, instrs, "serialized_size", Type::U64),
        NativeCall::CompareSelf => {
            // The reference goes through a local so it can be passed twice: the value is
            // compared with itself, which traverses all of it.
            let Some(Type::Reference(is_mut, inner)) = state.top().cloned() else {
                unreachable!("candidate requires a reference on top")
            };
            let Some(reference) = state.alloc_scratch_local(Type::Reference(false, inner.clone()))
            else {
                return Ok(false);
            };
            let Some(ordering) = state.alloc_scratch_local(ordering_ty()) else {
                return Ok(false);
            };
            state.pop();
            if is_mut {
                instrs.push(simple_op("freeze_ref"));
            }
            instrs.push(name_instr("st_loc", reference.clone()));
            instrs.push(name_instr("copy_loc", reference.clone()));
            instrs.push(name_instr("move_loc", reference.clone()));
            emit_call(instrs, STD_CMP, "compare", vec![*inner]);
            emit_is_lt(instrs, &ordering);
            state.push(Type::Bool);
        },
        NativeCall::CompareTop => {
            let Some(ordering) = state.alloc_scratch_local(ordering_ty()) else {
                return Ok(false);
            };
            state.pop();
            let Some(Type::Reference(_, inner)) = state.pop() else {
                unreachable!("candidate requires two references on top")
            };
            emit_call(instrs, STD_CMP, "compare", vec![*inner]);
            emit_is_lt(instrs, &ordering);
            state.push(Type::Bool);
        },
        NativeCall::Swap => {
            let (left, right) = *u.choose(&swappable)?;
            let ty = state.locals[left].ty.clone();
            instrs.push(name_instr(
                "mut_borrow_loc",
                state.locals[left].name.clone(),
            ));
            instrs.push(name_instr(
                "mut_borrow_loc",
                state.locals[right].name.clone(),
            ));
            emit_call(instrs, STD_MEM, "swap", vec![ty]);
        },
        NativeCall::MoveRange => {
            let (from, to) = *u.choose(&vector_pairs)?;
            let Type::Vector(elem) = state.locals[from].ty.clone() else {
                unreachable!("pairs hold vector locals")
            };
            // Small positions and lengths keep many calls within the vectors' bounds; the
            // others abort in the native.
            instrs.push(name_instr(
                "mut_borrow_loc",
                state.locals[from].name.clone(),
            ));
            push_small_u64(u, instrs, 2)?;
            push_small_u64(u, instrs, 2)?;
            instrs.push(name_instr("mut_borrow_loc", state.locals[to].name.clone()));
            push_small_u64(u, instrs, 1)?;
            emit_call(instrs, STD_VECTOR, "move_range", vec![*elem]);
        },
        NativeCall::Sha2 => emit_call(instrs, STD_HASH, "sha2_256", vec![]),
        NativeCall::Sha3 => emit_call(instrs, STD_HASH, "sha3_256", vec![]),
        NativeCall::SignerAddress => {
            state.pop();
            emit_call(instrs, STD_SIGNER, "address_of", vec![]);
            state.push(Type::Address);
        },
        NativeCall::TypeName => {
            let ty = pick_type_name_arg(u, state)?;
            emit_call(instrs, STD_TYPE_NAME, "get", vec![ty]);
            emit_call(instrs, STD_TYPE_NAME, "into_string", vec![]);
            emit_call(instrs, STD_ASCII, "into_bytes", vec![]);
            state.push(bytes_ty());
        },
        NativeCall::Utf8RoundTrip => {
            // `utf8` aborts on invalid bytes; `try_utf8` is not used because it returns an
            // `Option<String>` the generator cannot consume.
            emit_call(instrs, STD_STRING, "utf8", vec![]);
            emit_call(instrs, STD_STRING, "into_bytes", vec![]);
        },
        NativeCall::SubString => {
            // `sub_string` takes `&String`, so the string goes through a fresh local.
            let Some(local) = state.alloc_scratch_local(string_ty()) else {
                return Ok(false);
            };
            emit_call(instrs, STD_STRING, "utf8", vec![]);
            instrs.push(name_instr("st_loc", local.clone()));
            instrs.push(name_instr("borrow_loc", local.clone()));
            // Small indices keep `sub_string` in bounds often enough to reach the native;
            // out-of-range indices abort in the Move wrapper.
            push_small_u64(u, instrs, 8)?;
            push_small_u64(u, instrs, 8)?;
            emit_call(instrs, STD_STRING, "sub_string", vec![]);
            emit_call(instrs, STD_STRING, "into_bytes", vec![]);
        },
        NativeCall::IndexOf => {
            let Some(haystack) = state.alloc_scratch_local(string_ty()) else {
                return Ok(false);
            };
            emit_call(instrs, STD_STRING, "utf8", vec![]);
            instrs.push(name_instr("st_loc", haystack.clone()));
            instrs.push(name_instr("borrow_loc", haystack.clone()));
            instrs.push(name_instr("borrow_loc", haystack.clone()));
            emit_call(instrs, STD_STRING, "index_of", vec![]);
            state.pop();
            state.push(Type::U64);
        },
    }
    Ok(true)
}

#[derive(Clone, Copy)]
enum NativeCall {
    BcsToBytes,
    SerializedSize,
    /// `cmp::compare` of the referenced value with itself.
    CompareSelf,
    /// `cmp::compare` of the two references on top of the stack.
    CompareTop,
    /// `mem::swap` of two locals of the same type.
    Swap,
    /// `vector::move_range` between two vector locals with the same element type.
    MoveRange,
    Sha2,
    Sha3,
    SignerAddress,
    TypeName,
    Utf8RoundTrip,
    SubString,
    IndexOf,
}

const STD_ASCII: &str = "ascii";
const STD_BCS: &str = "bcs";
const STD_CMP: &str = "cmp";
const STD_HASH: &str = "hash";
const STD_MEM: &str = "mem";
const STD_VECTOR: &str = "vector";
const STD_SIGNER: &str = "signer";
const STD_STRING: &str = "string";
const STD_TYPE_NAME: &str = "type_name";

impl MasmTest {
    /// Whether any function calls `type_name::get`, which [`crate::Config::enable_type_name_get`]
    /// gates.
    pub fn calls_type_name_get(&self) -> bool {
        self.steps
            .iter()
            .flat_map(Step::functions)
            .flat_map(|function| &function.body)
            .flat_map(|block| &block.instrs)
            .any(|instr| calls_std(instr, STD_TYPE_NAME, "get"))
    }

    /// Replaces each `type_name::get` call sequence the generator emits with an empty
    /// `vector<u8>`, the sequence's result type, so a stored test runs without the native.
    /// Returns whether no call to `type_name::get` remains.
    pub fn replace_type_name_get(&mut self) -> bool {
        for step in &mut self.steps {
            for block in step
                .functions_mut()
                .iter_mut()
                .flat_map(|function| &mut function.body)
            {
                let mut index = 0;
                while index < block.instrs.len() {
                    if is_type_name_sequence(&block.instrs[index..]) {
                        block
                            .instrs
                            .splice(index..index + TYPE_NAME_SEQUENCE.len(), [Instr {
                                opcode: "vec_pack".to_string(),
                                operands: vec![
                                    Operand::Type(Type::U8),
                                    Operand::Imm(Value::U64(0)),
                                ],
                            }]);
                    }
                    index += 1;
                }
            }
        }
        !self.calls_type_name_get()
    }
}

/// The calls [`NativeCall::TypeName`] emits, which together push a `vector<u8>`.
const TYPE_NAME_SEQUENCE: [(&str, &str); 3] = [
    (STD_TYPE_NAME, "get"),
    (STD_TYPE_NAME, "into_string"),
    (STD_ASCII, "into_bytes"),
];

fn is_type_name_sequence(instrs: &[Instr]) -> bool {
    instrs.len() >= TYPE_NAME_SEQUENCE.len()
        && instrs
            .iter()
            .zip(TYPE_NAME_SEQUENCE)
            .all(|(instr, (module, name))| calls_std(instr, module, name))
}

/// Whether `instr` calls the stdlib function `module::name`.
fn calls_std(instr: &Instr, module: &str, name: &str) -> bool {
    instr.opcode == "call"
        && matches!(
            instr.operands.first(),
            Some(Operand::Function(target, _))
                if target.module.address() == &AccountAddress::ONE
                    && target.module.name().as_str() == module
                    && target.name.as_str() == name
        )
}

/// A concrete type for `type_name::get<T>`, which accepts any type argument.
fn pick_type_name_arg(u: &mut Unstructured<'_>, state: &FunctionState<'_, '_>) -> Result<Type> {
    let Some(module_id) = state.module_ctx.module_id.as_ref() else {
        return crate::types::pick_concrete_primitive(u);
    };
    let ty = pick_concrete_type(
        u,
        &state.module_ctx.structs,
        &state.module_ctx.external_modules,
        module_id,
        AbilitySet::EMPTY,
        state.cfg.max_type_depth.saturating_sub(1),
    )?;
    debug_assert!(
        AbilitySet::EMPTY.is_subset(crate::abilities::type_abilities(&ty, &ability_ctx(state)))
    );
    Ok(ty)
}

fn emit_call(instrs: &mut Vec<Instr>, module: &str, name: &str, type_args: Vec<Type>) {
    instrs.push(std_call(module, name, type_args));
}

/// Whether the two topmost stack values are immutable references to the same type, a valid
/// type argument that is not `signer`.
fn top_two_same_immutable_refs(state: &FunctionState<'_, '_>) -> bool {
    let [.., Type::Reference(false, below), Type::Reference(false, top)] =
        state.ty_stack.as_slice()
    else {
        return false;
    };
    below == top && is_valid_type_arg(top) && !matches!(**top, Type::Signer)
}

/// Indices of locals a native may borrow mutably right now: available values of a valid type
/// argument other than `signer`, with no outstanding borrow. The loop counter is excluded so
/// generated loops still terminate.
fn mutably_borrowable_locals(state: &FunctionState<'_, '_>) -> Vec<usize> {
    state
        .locals
        .iter()
        .enumerate()
        .filter(|(index, local)| {
            local.init == LocalInit::Available
                && !is_reference(&local.ty)
                && is_valid_type_arg(&local.ty)
                && !matches!(local.ty, Type::Signer)
                && local.name.as_str() != LOOP_COUNTER_NAME
                && state
                    .borrows
                    .can_borrow(&BorrowRoot::Local(*index as u16), &[], Mutability::Mut)
        })
        .map(|(index, _)| index)
        .collect()
}

/// Pairs of distinct locals of the same type that `mem::swap` can take.
fn swappable_locals(state: &FunctionState<'_, '_>) -> Vec<(usize, usize)> {
    let locals = mutably_borrowable_locals(state);
    let mut pairs = Vec::new();
    for (position, &left) in locals.iter().enumerate() {
        for &right in &locals[position + 1..] {
            if state.locals[left].ty == state.locals[right].ty {
                pairs.push((left, right));
            }
        }
    }
    pairs
}

/// Ordered pairs of distinct vector locals with the same element type, for
/// `vector::move_range`.
fn vector_local_pairs(state: &FunctionState<'_, '_>) -> Vec<(usize, usize)> {
    let vectors: Vec<usize> = mutably_borrowable_locals(state)
        .into_iter()
        .filter(|&index| matches!(state.locals[index].ty, Type::Vector(_)))
        .collect();
    let mut pairs = Vec::new();
    for &from in &vectors {
        for &to in &vectors {
            if from != to && state.locals[from].ty == state.locals[to].ty {
                pairs.push((from, to));
            }
        }
    }
    pairs
}

/// Stores the `Ordering` on top of the stack in `ordering` and replaces it with `is_lt` of it.
fn emit_is_lt(instrs: &mut Vec<Instr>, ordering: &Identifier) {
    instrs.push(name_instr("st_loc", ordering.clone()));
    instrs.push(name_instr("borrow_loc", ordering.clone()));
    emit_call(instrs, STD_CMP, "is_lt", vec![]);
}

/// Pops the reference on top and calls `0x1::bcs::<name>` on it, which returns `result`.
fn emit_bcs_call(
    state: &mut FunctionState<'_, '_>,
    instrs: &mut Vec<Instr>,
    name: &str,
    result: Type,
) {
    let Some(Type::Reference(is_mut, inner)) = state.pop() else {
        unreachable!("candidate requires a reference on top")
    };
    // The natives take `&T`; the verifier does not freeze `&mut T` implicitly.
    if is_mut {
        instrs.push(simple_op("freeze_ref"));
    }
    emit_call(instrs, STD_BCS, name, vec![*inner]);
    state.push(result);
}

fn push_small_u64(u: &mut Unstructured<'_>, instrs: &mut Vec<Instr>, max: u64) -> Result<()> {
    let value = u.int_in_range(0..=max)?;
    instrs.push(imm_instr("ld_u64", Value::U64(value)));
    Ok(())
}

fn bytes_ty() -> Type {
    Type::Vector(Box::new(Type::U8))
}

fn ordering_ty() -> Type {
    std_struct_ty(STD_CMP, "Ordering")
}

fn string_ty() -> Type {
    std_struct_ty(STD_STRING, "String")
}

/// The non-generic struct `0x1::<module>::<name>`.
fn std_struct_ty(module: &str, name: &str) -> Type {
    Type::Struct(
        crate::ir::QualifiedStruct {
            module: ModuleId::new(AccountAddress::ONE, Identifier::new(module).expect("ident")),
            name: Identifier::new(name).expect("ident"),
        },
        vec![],
    )
}
