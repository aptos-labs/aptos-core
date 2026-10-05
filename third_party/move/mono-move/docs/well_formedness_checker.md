# Well-formedness Checker

The well-formedness checker (`core/src/well_formedness.rs`) is a static
checker for lowered functions. It is distinct from the Move bytecode
verifier, which runs on bytecode before lowering; this checker runs on the
lowered micro-ops after it. The loader runs it on every lowered
`Function` before the function is cached, so a lowering it rejects is never
executed. This document is the specification of what it checks, written so
that each check is a precise statement about the function's data and can be
turned into a formal specification later. The implementation names each
check by its identifier below.

The checker is defence in depth against bugs in the specializer. The
bytecode it lowers has already passed the Move bytecode verifier, so a
rejected lowering is a VM bug and surfaces as an invariant violation
(`LoaderInvariantViolation::NotWellFormed`), never as a user
error.

## 1. Notation

For a function `F`:

| Symbol | Meaning |
|---|---|
| `code` | `F.code.ops()`, the micro-op sequence; `N = |code|`; `code[pc]` is the op at `pc` |
| `origins` | `F.code.origins()`, bytecode provenance, one entry per op or empty |
| `S` | `F.param_and_local_sizes_sum`, size of the params + locals region |
| `M` | `FRAME_METADATA_SIZE`, currently 24 |
| `E` | `F.extended_frame_size` |
| `P` | `F.param_region_size` |
| `A` | `MAX_ALIGN`, currently 8 |
| `frame_size` | `S + M` |
| `Data` | the byte range `[0, S)` of the frame: parameters and locals |
| `Meta` | `[S, S + M)`: saved pc, fp, and function pointer, written only by call and return |
| `Callee` | `[S + M, E)`: the callee's argument and return region; the callee's frame pointer is `fp + S + M` |
| `params` | `F.param_slots`, a list of sized slots; `param_tys` the matching types |
| `rets` | `F.return_slots`; `ret_tys` the matching types |
| `base` | `F.frame_layout.heap_ptr_offsets`, pointer slots the GC scans at every pc |
| `sps` | `F.safe_point_layouts.entries()`, each `(code_offset, heap_ptr_offsets)` |
| `zero_frame` | `F.zero_frame` |
| `desc(id)` | the descriptor provider's entry for `id`, or none |
| `layout(ty)` | the layout provider's `(size, align)` for `ty`, or none |
| `const_ty(idx)` | the constant pool provider's type for `idx` in `F`'s module, or none |

A **sized slot** is `s = (o, w, a)`: byte offset `o` from the frame pointer,
width `w`, and alignment `a`. `end(s) = o + w`, computed without overflow by
widening to `usize`.

An **access** is `(o, w, a)` with the same meaning but no identity. An access
is **valid** in `F` iff

- `o + w` does not overflow `usize`,
- `o + w <= E` (inside the extended frame, which includes `Callee`: that is how
  arguments and return values are passed),
- `[o, o + w)` does not intersect `Meta`,
- `o mod a = 0`.

A slot `(o, w, a)` is **well-formed** iff `w > 0`, `a` is a power of two with
`1 <= a <= A`, and `o mod a = 0`.

Two intervals are **disjoint** iff they do not intersect. A list of slots is
**ascending and disjoint** iff for every consecutive pair `s_i, s_{i+1}`:
`end(s_i) <= o_{i+1}`.

`x mod y = 0` for `usize`/`u32` values is `is_multiple_of`.

## 2. Operand kinds

The frame operands of each micro-op and the kind of access the interpreter
performs on them are declared once, in `core/src/instruction/operands.rs`,
as `MicroOp::frame_operands`. The schema describes the interpreter's access,
not the layout convention: an operand the interpreter reads unaligned has no
alignment requirement even if the layout pass would align it.

| `OperandKind` | width | align | interpreter access |
|---|---|---|---|
| `Bool` | 1 | 1 | `read_bool` / `write_bool` |
| `Byte` | 1 | 1 | `read_u8` (shift amounts) |
| `U64` | 8 | 8 | `read_u64` / `write_u64` |
| `Ptr` | 8 | 8 | `read_ptr` / `write_ptr` |
| `FatPtr` | 16 | 8 | `read_fat_ptr` / `write_fat_ptr` |
| `Address` | 32 | 1 | `read_account_address` (align-1 type) |
| `Bytes(n)` | `n` | 1 | `ptr::copy`, `copy_nonoverlapping`, or an explicit unaligned load/store |
| `Int(w)` | `w` | `w` if `w <= A`, else 1 | `read_int<T>` / `write_int<T>`: aligned up to `MAX_ALIGN`, unaligned beyond |
| `Value(ty)` | `layout(ty).size` | `layout(ty).align` | by-value comparison operand |
| `Constant(idx)` | `layout(const_ty(idx)).size` | `layout(const_ty(idx)).align` | `StoreImmVec` destination |

`Move8` and the frame side of `HeapMoveFrom8` / `HeapMoveTo8` are
`Bytes(8)`, not `U64`, because the interpreter uses `read_unaligned` /
`write_unaligned` there. `DeepCopyHeapPtrs` reports `Ptr` at
`base + off` saturated at `u32::MAX`; the overflow itself is check `Z3`.

## 3. Checks

Every check is a predicate on `F` and the providers. The checker evaluates
all of them and reports every violation; no check depends on another having
passed. Identifiers are stable and are cited in the implementation.

### 3.1 Function shape

- **F1 (non-empty)**: `N >= 1`.
- **F2 (terminator)**: `code[N - 1]` is one of `Return`, `Abort`, `AbortMsg`,
  `Jump`. Rationale: every other op falls through to `pc + 1`, and a call
  returns to `call_pc + 1`, so anything else runs off the end.
- **F3 (frame fits)**: `S + M <= E`.
- **F4 (params inside data)**: `P <= S`.
- **F5 (metadata alignment)**: `S mod A = 0`. The call protocol writes `Meta`
  with aligned 8-byte stores at `fp + S`.
- **F6 (callee fp alignment)**: `(S + M) mod A = 0`. The callee's frame
  pointer is `fp + S + M` and every callee slot offset assumes an aligned fp.
- **F7 (origins)**: `|origins| = 0` or `|origins| = N`.

### 3.2 Parameter and return slots

- **P1 (count)**: `|params| = |param_tys|`.
- **P2 (well-formed)**: every `s` in `params` is well-formed.
- **P3 (inside the parameter region)**: for every `s` in `params`,
  `end(s) <= P`.
- **P4 (ascending and disjoint)**: `params` is ascending and disjoint.
- **R1 (count)**: `|rets| = |ret_tys|`.
- **R2 (well-formed)**: every `s` in `rets` is well-formed.
- **R3 (inside the data region)**: for every `s` in `rets`, `end(s) <= S`.
- **R4 (ascending and disjoint)**: `rets` is ascending and disjoint.

Rationale for P3: `CallClosure` and `CallBuilder` write `w` bytes at
`callee_fp + o` for each parameter slot with only the stack-overflow check on
`E` as a guard, and `call_unchecked` then zeroes `[P, E)`, so a parameter
slot outside `[0, P)` is either overwritten or outside the frame.

### 3.3 GC layouts

- **G1 (base layout slots)**: every `o` in `base` is a valid access
  `(o, 8, 8)`.
- **G2 (base layout order)**: `base` is strictly increasing.
- **G3 (zero_frame)**: if some `o` in `base` has `o >= P`, then
  `zero_frame = true`. Rationale: the GC scans `base` of every frame
  unconditionally; a slot past the parameter region is not written by the
  caller and must start out null rather than holding the previous frame's
  bytes.
- **G4 (safe-point order)**: the `code_offset`s of `sps` are strictly
  increasing.
- **G5 (safe-point position)**: for every entry, `code_offset < N` and
  `code[code_offset].is_allocating()`.
- **G6 (safe-point slots)**: every entry's `heap_ptr_offsets` satisfies G1
  and G2.
- **G7 (safe-point disjoint from base)**: no offset appears in both an
  entry's `heap_ptr_offsets` and `base`.

### 3.4 Operand accesses

- **O1 (operand access)**: for every `pc` and every `(o, kind)` reported by
  `code[pc].frame_operands`, the access `(o, width(kind), align(kind))` is
  valid, with `width` and `align` from the table in section 2.
- **O2 (value operand layout)**: for `kind = Value(ty)`, `layout(ty)` exists.
- **O3 (value operand type)**: for `kind = Value(ty)`, and for the `ty` of
  `ValueRefCmp` and `JumpValueRefCmp`, `ty` is not a reference type.
  Rationale: structural comparison is defined on values; the interpreter
  reports a reference type as an invariant violation.
- **O4 (constant exists)**: for `kind = Constant(idx)`, `const_ty(idx)`
  exists.
- **O5 (constant layout)**: for `kind = Constant(idx)`,
  `layout(const_ty(idx))` exists.

### 3.5 Instruction-local invariants

- **I1 (unchecked divisor)**: `DivU64Imm` and `ModU64Imm` have `imm != 0`.
- **I2 (unchecked shift)**: `ShlU64Imm` and `ShrU64Imm` have `imm < 64`.
- **I3 (bitwise signedness)**: `IntBitAnd`, `IntBitOr`, `IntBitXor` have an
  unsigned `rhs`.
- **I4 (shift signedness)**: `IntShl`, `IntShr` have an unsigned `ty`.
- **I5 (negate signedness)**: `IntNegate` has a signed `ty`.
- **Z1 (non-zero widths)**: `size > 0` for `Move`, `ReadRef`, `WriteRef`,
  `ReadRefOffset`, `WriteRefOffset`, `HeapReadOffset`, `HeapWriteOffset`,
  `HeapMoveFrom`, `HeapMoveTo`, `EnumReadVariantFieldByTag`,
  `EnumWriteVariantFieldByTag`; `elem_size > 0` for `VecPushBack`,
  `VecPopBack`, `VecLoadElem`, `VecStoreElem`, `VecSwap`, `VecBorrow`,
  `VecPack`, `VecUnpack`.
- **Z2 (offset windows)**: `offset + size` does not overflow `u32` for
  `HeapMoveFrom`, `HeapMoveTo`, `HeapReadOffset`, `HeapWriteOffset`,
  `ReadRefOffset`, `WriteRefOffset`; `offset + 8` does not overflow for
  `HeapMoveFrom8`, `HeapMoveTo8`, `HeapMoveToImm8`; for
  `EnumReadVariantFieldByTag` and `EnumWriteVariantFieldByTag`, every
  present entry `Some(offset)` of the table satisfies the same with `size`.
- **Z3 (deep-copy offsets)**: for `DeepCopyHeapPtrs`, `base + off` does not
  overflow `u32` for every `off`.
- **J1 (jump targets)**: for every op with a `target` (`Jump`, the
  `Jump*U64*` and `Jump*Byte` family, `JumpIntCmp`, `JumpValueCmp`,
  `JumpValueRefCmp`), `target < N`.
- **D1 (multi-destination disjointness)**: for every op that writes more than
  one frame destination, the destination intervals are pairwise disjoint.
  Today the only such op is `VecUnpack`, whose destinations are
  `[d, d + elem_size)` for each `d` in `dsts`; the element copies are
  independent `copy_nonoverlapping`s.
- **B1 (SlotBorrow base)**: for `SlotBorrow`, `local < S`. The op carries no
  width, so the borrowed extent is not checked (see section 4).

### 3.6 Descriptors

- **K1 (HeapNew)**: `desc(descriptor_id)` exists and is `Struct` or `Enum`.
- **K2 (EnumNew)**: `desc(descriptor_id)` exists, is `Enum`, and
  `variant < |variant_pointer_offsets|`.
- **K3 (vector allocation)**: for `VecPushBack` and `VecPack`,
  `desc(descriptor_id)` exists and is either `Trivial`, or `Vector` with a
  non-empty `elem_pointer_offsets` and `elem_size` equal to the op's
  `elem_size`. Rationale: the GC strides the data region by the descriptor's
  element size.

### 3.7 Calls

- **C1 (native region)**: for `CallNative` with ABI `abi`,
  `frame_size + abi.total_frame_size <= E`.
- **C2 (native pointer slots)**: for every `o` in `abi.heap_ptr_offsets`:
  `o + 8 <= abi.total_frame_size`, `o mod 8 = 0`, and some argument slot
  `a` of `abi` has `a.offset <= o` and `o + 8 <= a.offset + a.size`.
  Rationale: the GC reads these with aligned `read_ptr` at the native's
  frame pointer.
- **C3 (native descriptors)**: every `id` in `abi.required_descriptors` has
  `desc(id)`.
- **C4 (direct callee fits)**: for `CallDirect` to `callee`,
  `callee.P <= E - frame_size` and `end(s) <= E - frame_size` for every `s`
  in `callee.rets`. (`E - frame_size` is taken saturating; F3 reports the
  underlying violation.)

### 3.8 Closures

For `PackClosure` with fields `dst`, `func_ref`, `mask`, `cdd`
(`captured_data_descriptor_id`), `values_size`, `captured`:

- **L1 (captured-data presence)**: `cdd` is `Some` iff `captured` is
  non-empty.
- **L2 (captured-data descriptor)**: if `cdd = Some(id)`, `desc(id)` exists
  and is `Trivial` or `CapturedData`.
- **L3 (captured-data pointer offsets)**: if `desc(id)` is
  `CapturedData { pointer_offsets }`, every `off` satisfies
  `off + 8 <= values_size`. Rationale: captured-data descriptors are shared
  across closures with different `values_size`, so the descriptor's own
  bound does not apply.
- **L4 (captured slots well-formed)**: every `s` in `captured` is
  well-formed. Rationale: its `align` drives `align_up` in the captured-data
  layout, which is undefined for zero or non-power-of-two.
- **L5 (mask count)**: `|captured| = popcount(mask)`.
- **L6 (resolved callee mask)**: if `func_ref = Resolved(callee)`:
  `|callee.params| <= 64` and `mask >> |callee.params| = 0`.
- **L7 (resolved callee layout)**: if `func_ref = Resolved(callee)`, for the
  `k`-th set bit `i` of `mask` (ascending), `captured[k].w =
  callee.params[i].w` and `captured[k].a = callee.params[i].a`. Rationale:
  captured values are written with the captured slot's `(w, a)` and read
  back at the callee parameter's natural-aligned offset.
- **L8 (values size)**: if every `s` in `captured` has a valid `a`,
  `values_size = captured_values_size(captured)`.

For `CallClosure`:

- **L9 (provided args well-formed)**: every `s` in `provided_args` is
  well-formed.

When `func_ref = Unresolved(_)`, L6 and L7 are deferred to call time, where
the interpreter validates the resolved callee.

## 4. Out of scope

These properties depend on runtime data or on dataflow and are not checked
statically:

- the extent of a `SlotBorrow` (the op carries no width);
- heap `offset` against the pointee's size for ops that carry only a pointer
  (`HeapBorrow`, `HeapMove*`, `HeapRead/WriteOffset`, `DeriveRefOffsetImm`,
  `Read/WriteRefOffset`);
- `elem_size` against the real stride of the vector a reference points to;
- enum offset tables against the pointee's variant count;
- whether a slot listed in a GC layout actually holds a pointer at a given
  pc, or whether a slot is written before it is read;
- the contents of descriptors, which `ObjectDescriptor`'s constructors
  validate at publish time;
- gas fields on branches (TODO(metering)).

## 5. Structure and pass separation

The checks group naturally by the inputs they read:

| Group | Checks | Inputs |
|---|---|---|
| Function shape | F1–F7 | `F`'s scalar fields, `code` |
| Slots | P1–P4, R1–R4 | `params`, `rets`, `S`, `P` |
| GC layouts | G1–G7 | `base`, `sps`, `code` (for `is_allocating`) |
| Operand accesses | O1–O5 | `code`, the operand schema, `layout`, `const_ty` |
| Instruction-local | I1–I5, Z1–Z3, J1, D1, B1 | `code` only |
| Descriptors | K1–K3, L2–L3, C3 | `code`, `desc` |
| Calls and closures | C1–C2, C4, L1, L4–L9 | `code`, the callee `Function`s behind `CallDirect` and `Resolved` |

Every check is a pure predicate over immutable inputs and the result is the
union of all violations, so the groups are already independent: none reads
another's result, and evaluating them in any order, or separately, gives
the same answer. Splitting the implementation into one pass per group is
therefore feasible and cheap. Checking is linear in `N` plus the sizes
of the layouts and slot lists; several passes over `code` instead of one
change the constant only, and the checker runs once per cached lowering.

What a split would buy:

- each pass owns one section of this document and one block of tests;
- passes that need no provider (shape, slots, instruction-local) can run in
  tools that have none, such as a specializer self-check or a micro-op
  fuzzer;
- a future dataflow pass for the section 4 items (borrow extents,
  write-before-read, pointer-slot typing) slots in beside the others instead
  of threading state through the existing match.

What it would cost: a small amount of plumbing to share the error sink and
the function context between passes, and a second iteration over `code`.
The one shared primitive, access validity (section 1), stays in one place.

The implementation today is a single `Checker` whose methods map
one-to-one onto the groups above; a pass split is a mechanical follow-up.
