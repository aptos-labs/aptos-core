-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Semantics.State
import Spec.Semantics.ModeledNatives

/-!
# The deterministic core step

TRUSTED. `step P s` executes one instruction of the current frame. It is
defensive (plan §5.2): every rule the static verifier is meant to guarantee
is checked here, and a violation is `wrong` with its reason. A Move-level
failure is `failed`.

Status: plan Phase 4, stage B. The core subset of Phase 1 (integers of every
width, booleans, addresses, locals, branches, non-generic calls, aborts,
constants), with the module lock R4; data by value (Phase 2a): signers,
vectors, and structs and enums, through `Pack`, `Unpack`, `PackVariant`,
`UnpackVariant` (with R1), `VecPack`, `VecUnpack` (with R2) and `AbortMsg`;
references and global storage (Phase 3), through the borrows of locals,
fields and variant fields (with R1), `ReadRef`, `WriteRef`, `FreezeRef`,
`TestVariant`, the vector operations through references (with R6), and
`Exists`, `MoveFrom`, `MoveTo` and the global borrows (with the resource lock
R3 and existence R5); generics (Phase 4, stage A), through `CallGeneric` and
the generic form of every struct, enum and global storage instruction; and
function values (stage B), through `PackClosure`, `PackClosureGeneric` and
`CallClosure`. Copying, dropping and comparing a value check the abilities it
actually has, which come from declarations at the value's instance and, for a
closure, from its target and its captured values (`Value.abilities`). A
closure over a native function is `wrong (unsupported _)`, which the
`Supported` premise of the theorems rules out.

**Function values** (book, `functions.md`, "Function Values"). `PackClosure`
resolves its function as a call does, with the same visibility (plan §5.2,
"Calls and visibility"), and captures the parameters its mask selects, never
references; the closure's abilities are its target's and its captured
values' (`Program.closureValueAbilities`). `CallClosure` takes the closure on top
and the remaining arguments below it, and calls the target with the captured
and the provided arguments mixed by the mask, whatever the target's
visibility: "`CallClosure` invokes whatever function the closure targets,
including a private one" (plan §5.2). Every closure dispatch counts the
target's module as active, "even within one module" (plan §6, R3), so the
resource lock R3 and the module lock R4 apply to code reached through a
closure. A value with more abilities may stand where a function type with
fewer is expected (`RuntimeType.assignable`).

**Natives with references** (language-owner rulings, 2026-11-03;
`Spec.Semantics.ModeledNatives`). A modeled native, of the read, update or
borrow shape, suspends with what it sees of its arguments, the values its
reference arguments designate (`State.nativeView`), which must be valid; its
mutable reference arguments overlap no other reference argument
(`exclusiveArguments`). The native relation then supplies its result
(`resumeNative`, `Spec.Semantics.Run`). The natives of `std::vector` marked
`#[bytecode_instruction]` are among them, with no definition: a call of one is
not its instruction.

**Tables** (language-owner rulings, 2026-11-04). `Pack` of a table type
(`StructIdOf.isTable`) makes a table with no entries (`Value.table`): "packing
a `Table` gives a table with no entries". `Unpack` of a table takes its fields
apart; a table has entries only where the table module never unpacks one
(`Program.tablesHoldEntries`), so an unpacked table has none. Field borrows
reach a table's fields as a struct's. The table natives are modeled natives
(`Spec.Semantics.ModeledNatives`).

**Generic code.** The current frame runs a function instance with closed type
arguments (`Frame.function`). Every type an instruction names, a vector's
element type, a local's type, an instantiation's type arguments, is the
module's token translated under them (`RuntimeType.ofTokenWith`). A generic
form behaves as its non-generic form at the instance its instantiation names
(`file_format.rs`: "Generic version of `Pack`", "See `Pack`", and likewise for
every generic form): it names a generic entity with one type argument per
type parameter (L9, else `badOperand`), and its type arguments satisfy the
entity's constraints ("constraints are checked at call sites", book,
`generics-and-abilities.md`, "Verifying Constraints"; else
`constraintViolation`). Non-generic forms in generic code are unchanged.

**R7, type specialization.** The type arguments of a frame, and the types
translated under them, are finite trees of any size: under cyclic module
dependencies (plan §4.3) a chain of generic calls can build ever larger type
arguments without end. The semantics needs no bound: it never builds a layout,
and each step translates finitely many finite tokens. A VM that bounds the
types it builds stops with a limit, which the stopping relation allows at
every state (`Spec.Semantics.Run`), so its bound refines the semantics.

Every value the semantics creates is well formed (`Value.wellFormed`): `Pack`
and the other constructors check the conformance of what they consume, and
natives and initial arguments are checked or assumed to conform. Instructions
that take a value apart therefore check only its intrinsic type.

An instruction that reads or writes through a reference, or inspects the
value it designates to borrow inside it, accesses its location, which must be
valid (`deref`; plan §5.2: "Using an invalid location is `wrong`"). Copying,
moving, freezing, passing, returning and dropping a reference do not access
its location, and are allowed on a reference that is no longer valid; the
stronger reference-safety properties of D4 are stated over the ghost layer.
A valid location holds a value of the reference's target type: every write
to it conforms to that type.
-/

namespace MoveBytecode

namespace Step

/-- A partial step: an error carries the terminal outcome (`failed` or
`wrong`). -/
abbrev Exec := Except Outcome

def wrong {α : Type} (r : WrongReason) : Exec α := .error (.wrong r)

def fail {α : Type} (f : Failure) : Exec α := .error (.failed f)

def need {α : Type} (o : Option α) (r : WrongReason) : Exec α :=
  match o with
  | some a => pure a
  | none => wrong r

def pop (fr : Frame) : Exec (Value × Frame) :=
  match fr.stack with
  | v :: rest => pure (v, { fr with stack := rest })
  | [] => wrong .stackUnderflow

/-- Pop `n` values, returned in push order (deepest first). -/
def popN (fr : Frame) (n : Nat) : Exec (List Value × Frame) :=
  if fr.stack.length < n then wrong .stackUnderflow
  else pure ((fr.stack.take n).reverse, { fr with stack := fr.stack.drop n })

def popBool (fr : Frame) : Exec (Bool × Frame) := do
  let (v, fr) ← pop fr
  match v with
  | .bool b => pure (b, fr)
  | _ => wrong .typeMismatch

def popInt (fr : Frame) : Exec (IntType × Int × Frame) := do
  let (v, fr) ← pop fr
  match v with
  | .int t x => pure (t, x, fr)
  | _ => wrong .typeMismatch

def push (fr : Frame) (v : Value) : Frame := { fr with stack := v :: fr.stack }

/-- Push values so that the first ends deepest and the last on top: the order
in which `Unpack`, `UnpackVariant` and `VecUnpack` push fields and elements
("stack << field_0 ... stack << field_n-1", `file_format.rs`). -/
def pushAll (fr : Frame) (vs : List Value) : Frame := { fr with stack := vs.reverse ++ fr.stack }

def advance (fr : Frame) : Frame := { fr with pc := fr.pc + 1 }

/-- Continue with the updated current frame. -/
def next (s : State) (rest : List Frame) (fr : Frame) : Exec StepResult :=
  pure (.done (.running { s with frames := fr :: rest }))

/-- Pop two integers of the same type `t` with `allow t`, apply `op`, push. -/
def intBinop (fr : Frame) (allow : IntType → Bool) (op : IntType → Int → Int → Option Int) :
    Exec Frame := do
  let (t', y, fr) ← popInt fr
  let (t, x, fr) ← popInt fr
  if t != t' || !allow t then wrong .typeMismatch
  else
    match op t x y with
    | some r => pure (advance (push fr (.int t r)))
    | none => fail .arithmetic

/-- Pop two integers of the same type and push the comparison result. -/
def intCompare (fr : Frame) (op : Int → Int → Bool) : Exec Frame := do
  let (t', y, fr) ← popInt fr
  let (t, x, fr) ← popInt fr
  if t != t' then wrong .typeMismatch
  else pure (advance (push fr (.bool (op x y))))

def boolBinop (fr : Frame) (op : Bool → Bool → Bool) : Exec Frame := do
  let (b, fr) ← popBool fr
  let (a, fr) ← popBool fr
  pure (advance (push fr (.bool (op a b))))

/-- `Shl`/`Shr`: the shift amount is a `u8` on top; the shifted value is an
unsigned integer below it. -/
def shift (fr : Frame) (op : IntType → Int → Nat → Option Int) : Exec Frame := do
  let (ta, n, fr) ← popInt fr
  let (t, x, fr) ← popInt fr
  if ta != .u8 || t.isSigned then wrong .typeMismatch
  else
    match op t x n.toNat with
    | some r => pure (advance (push fr (.int t r)))
    | none => fail .arithmetic

/-- Casts accept an integer of any type and check the target range. -/
def cast (fr : Frame) (target : IntType) : Exec Frame := do
  let (_, x, fr) ← popInt fr
  match IntType.cast target x with
  | some r => pure (advance (push fr (.int target r)))
  | none => fail .arithmetic

/-- The value a reference designates in state `s`. Accessing a location that
is not valid is `wrong` (plan §5.2). -/
def deref (s : State) (r : Reference) : Exec Value :=
  need (s.deref? r.location r.since) .invalidReference

/-- `Eq`/`Neq` consume both operands, which must have the same type, and both
must have `drop`: "Both `==` and `!=` consume the value when comparing them.
As a result, the type system enforces that the type must have `drop`" (book,
`equality-and-comparison.md`). Equality is structural for every type:
"Equality and non-equality also work over user-defined types".

References compare the values they designate: with `i = &0` and `m = &mut 1`,
"`i == freeze(m)`" is `false` and "`m == m`" is `true` (book, "Typing with
references"). Comparing references reads both locations, which must be
valid; the designated values need neither `copy` nor `drop`: "a programmer can
_always_ borrow the value first instead of directly comparing the value, and
reference types have the `drop` ability". The operands' types must be equal,
mutability included ("assert right_ty == left_ty", `file_format.rs`): a source
comparison of `&T` with `&mut T` is "equivalent to applying an explicit
freeze". `s` is the state before the step, whose locations are those of the
step.

Function values compare by "the name of the underlying function behind a
runtime value" and then "the captured values" (book, `functions.md`): two
closures are equal when they are the same function instance with the same
mask and captured values (`Value.beq`). The operands' types must be assignable
to a common type (`RuntimeType.comparable`): two closures' types may differ in
abilities, and so may the targets of two immutable references to closures,
since `&` is covariant; both conform to the type the code compares them at. -/
def equality (P : Program) (s : State) (fr : Frame) (positive : Bool) : Exec Frame := do
  let (b, fr) ← pop fr
  let (a, fr) ← pop fr
  if !RuntimeType.comparable a.type b.type then wrong .typeMismatch
  else if !(a.abilities P).drop || !(b.abilities P).drop then wrong .dropWithoutAbility
  else
    match a, b with
    | .ref ra, .ref rb => do
      let x ← deref s ra
      let y ← deref s rb
      pure (advance (push fr (.bool (if positive then x == y else x != y))))
    | _, _ => pure (advance (push fr (.bool (if positive then a == b else a != b))))

def jump (code : CodeUnit) (fr : Frame) (off : CodeOffset) : Exec Frame :=
  if off < code.code.size then pure { fr with pc := off } else wrong .invalidJump

def ldInt (fr : Frame) (t : IntType) {n : Nat} (v : BitVec n) : Frame :=
  advance (push fr (.int t (t.ofBitVec v)))

/-- The closed type arguments a generic instruction form supplies: the tokens
of its instantiation's signature `sig` in module `m`, translated under the
type arguments `tys` of the running instance (an instantiation "can be
partial", mentioning the enclosing function's type parameters,
`file_format.rs`, "Instantiations"). A generic form names a generic entity
(L9), so it supplies at least one; none is `badOperand`. -/
def genericArgs (m : CompiledModule) (tys : List RuntimeType) (sig : SignatureIndex) :
    Exec (List RuntimeType) := do
  let toks ← need m.signatures[sig]? .badIndex
  match ← need (RuntimeType.ofTokenListWith m tys toks) (.unsupported "type arguments") with
  | [] => wrong .badOperand
  | t :: ts => pure (t :: ts)

/-- The struct or enum type defined by definition `d` of module `mi`, the
current function's module, at the instance with type arguments `typeArgs`
(none for a non-generic form), with its declaration. `Pack`, `Unpack` and the
variant forms name a definition of the current module, so they only ever run
in the type's defining module (book, `structs-and-enums.md`, "Privileged
Struct Operations"). The definition must exist (L1), its name must resolve
back to it (L2, `Program.definedStruct?`), it must have one type parameter per
type argument (L9: a non-generic form names a non-generic definition, a
generic form a generic one with all its type arguments), and the type
arguments must be types of the program satisfying its constraints
(`RuntimeType.wellFormed`). -/
def localStruct (P : Program) (mi : Nat) (d : StructDefinitionIndex)
    (typeArgs : List RuntimeType := []) : Exec (StructId × StructInfo) := do
  let _ ← need (P.struct? ⟨mi, d⟩) .badIndex
  let sid ← need (P.definedStruct? mi d) .unresolvedStruct
  let sid := { sid with typeArgs }
  let info ← need (P.structType? sid) .badOperand
  if (RuntimeType.struct sid).wellFormed P then pure (sid, info) else wrong .constraintViolation

/-- The element type of a vector instruction: its signature operand is a
single type (L9, `CompiledModule.vectorElemTypeWith`), here of the subset,
translated under the running instance's type arguments `tys`. -/
def elemType (m : CompiledModule) (sig : SignatureIndex) (tys : List RuntimeType := []) :
    Exec RuntimeType := do
  match ← need m.signatures[sig]? .badIndex with
  | [t] => need (RuntimeType.ofTokenWith m tys t) (.unsupported "vector element type")
  | [] => wrong .badOperand
  | _ :: _ :: _ => wrong .badOperand

/-- The bytes of an abort message: the elements of a `vector<u8>`. -/
def messageBytes : List Value → Option (List UInt8)
  | [] => some []
  | .int .u8 x :: rest => do pure (UInt8.ofNat x.toNat :: (← messageBytes rest))
  | _ :: _ => none

/-! ### Structs and enums by value -/

/-- `Pack`, `PackGeneric`: "The values of the fields of the struct, in the
order they appear in the struct declaration, must be pushed on the stack. All
fields must be provided" (`file_format.rs`, `Pack`), at the instance with type
arguments `typeArgs`. A table type's value is a table with these fields and no
entries (language-owner ruling 1, 2026-11-04). -/
def packStruct (P : Program) (s : State) (fr : Frame) (rest : List Frame) (d : StructDefinitionIndex)
    (typeArgs : List RuntimeType := []) : Exec StepResult := do
  let (sid, sinfo) ← localStruct P fr.function.module d typeArgs
  match sinfo.definition.fieldInformation with
  | .declared _ =>
    let tys ← need (P.fieldTypes sid) (.unsupported "field types")
    let (vals, fr) ← popN fr tys.length
    if !conformsAll P vals tys then wrong .typeMismatch
    else
      let v := if sid.isTable then .table sid vals [] [] else .struct sid vals
      next s rest (advance (push fr v))
  | .declaredVariants _ => wrong .badOperand
  | .native => wrong .badOperand

/-- `Unpack`, `UnpackGeneric`: "Destroy an instance of a struct and push the
values bound to each field onto the stack" (`file_format.rs`); the value must
be of the instance the instruction names. A table pushes its fields: it has no
entries wherever a table module unpacks one (`Program.tablesHoldEntries`). -/
def unpackStruct (P : Program) (s : State) (fr : Frame) (rest : List Frame)
    (d : StructDefinitionIndex) (typeArgs : List RuntimeType := []) : Exec StepResult := do
  let (sid, sinfo) ← localStruct P fr.function.module d typeArgs
  match sinfo.definition.fieldInformation with
  | .declared _ =>
    let (v, fr) ← pop fr
    match v with
    | .struct sid' fields | .table sid' fields _ _ =>
      if sid' != sid then wrong .typeMismatch else next s rest (advance (pushAll fr fields))
    | _ => wrong .typeMismatch
  | .declaredVariants _ => wrong .badOperand
  | .native => wrong .badOperand

/-- `PackVariant`, `PackVariantGeneric`: a variant handle names an enum
definition of this module and one of its variants (L1); "the values of the
fields of the variant, in the order they are determined by the declaration,
must be pushed on the stack" (`file_format.rs`). -/
def packEnum (P : Program) (s : State) (fr : Frame) (rest : List Frame) (info : FunctionInfo)
    (h : StructVariantHandleIndex) (typeArgs : List RuntimeType := []) : Exec StepResult := do
  let vh ← need info.module.structVariantHandles[h]? .badIndex
  let (sid, sinfo) ← localStruct P fr.function.module vh.structIndex typeArgs
  match sinfo.definition.fieldInformation with
  | .declaredVariants variants =>
    let _ ← need variants[vh.variant]? .badIndex
    let tys ← need (P.variantFieldTypes sid vh.variant) (.unsupported "field types")
    let (vals, fr) ← popN fr tys.length
    if !conformsAll P vals tys then wrong .typeMismatch
    else next s rest (advance (push fr (.variant sid vh.variant vals)))
  | .declared _ => wrong .badIndex
  | .native => wrong .badIndex

/-- `UnpackVariant`, `UnpackVariantGeneric`, R1: "If the value on the stack is
of the specified variant, destroy it and push the values bound to each field
onto the stack. Aborts if the value is not of the specified variant"
(`file_format.rs`). -/
def unpackEnum (P : Program) (s : State) (fr : Frame) (rest : List Frame) (info : FunctionInfo)
    (h : StructVariantHandleIndex) (typeArgs : List RuntimeType := []) : Exec StepResult := do
  let vh ← need info.module.structVariantHandles[h]? .badIndex
  let (sid, sinfo) ← localStruct P fr.function.module vh.structIndex typeArgs
  match sinfo.definition.fieldInformation with
  | .declaredVariants variants =>
    let _ ← need variants[vh.variant]? .badIndex
    let (v, fr) ← pop fr
    match v with
    | .variant sid' tag fields =>
      if sid' != sid then wrong .typeMismatch
      else if tag != vh.variant then fail .variantMismatch
      else next s rest (advance (pushAll fr fields))
    | _ => wrong .typeMismatch
  | .declared _ => wrong .badIndex
  | .native => wrong .badIndex

/-! ### References -/

/-- Pop a reference. -/
def popRef (fr : Frame) : Exec (Reference × Frame) := do
  let (v, fr) ← pop fr
  match v with
  | .ref r => pure (r, fr)
  | _ => wrong .typeMismatch

/-- Pop a reference to a value of type `t`, which must be a mutable reference
if `mutable` is set. An instruction that only reads accepts `&T` and `&mut T`
alike ("assert ty == &struct_ty or ty == &mut struct_ty", `file_format.rs`). -/
def popRefTo (fr : Frame) (t : RuntimeType) (mutable : Bool) : Exec (Reference × Frame) := do
  let (r, fr) ← popRef fr
  if r.target != t then wrong .typeMismatch
  else if mutable && !r.mutable then wrong .immutableReference
  else pure (r, fr)

/-- A new reference to location `l` in state `s`, of mutability `mutable` and
target type `t`. -/
def borrow (s : State) (mutable : Bool) (t : RuntimeType) (l : Location) : Value :=
  .ref { mutable, target := t, location := l, since := s.invalidations.length }

/-- Change the value at location `l` in place to `v`: every location strictly
inside the old value becomes invalid (`Invalidation.mutated`). -/
def update (s : State) (l : Location) (v : Value) : Exec State := do
  pure ((← need (s.write? l v) .invalidReference).invalidate (.mutated l))

/-- End the instruction in state `s`, whose current frame is the head: push
`vals` (the first deepest) and advance. -/
def finish (s : State) (vals : List Value) : Exec StepResult :=
  match s.frames with
  | fr :: rest => pure (.done (.running { s with frames := advance (pushAll fr vals) :: rest }))
  | [] => wrong .noFrame

/-- `MutBorrowLoc`, `ImmBorrowLoc`: "Load a mutable [an immutable] reference to
a local identified by the local index" (`file_format.rs`). The local must be
available ("Variables must be assigned before use", book, `variables.md`)
and must not hold a reference: `&e` takes "`e: T` and `T` is a non-reference
type", and "references to references are not allowed" (book,
`references.md`). The reference's target type is the local's declared type at
the frame's instance, which the local's value conforms to: a closure in it may
have more abilities than the declared function type, and a write through a
mutable reference may store any value that conforms to the declared type. For
a value without function types the two are the same type. -/
def borrowLoc (P : Program) (s : State) (fr : Frame) (rest : List Frame) (mutable : Bool)
    (i : LocalIndex) : Exec StepResult := do
  match ← need fr.locals[i]? .localOutOfRange with
  | none => wrong .unavailableLocal
  | some _ =>
    let tys ← need (P.localTypes fr.function) (.unsupported "local types")
    let t ← need tys[i]? .localOutOfRange
    if !t.isValue then wrong .typeMismatch
    else next s rest (advance (push fr (borrow s mutable t ⟨.local_ fr.id i, []⟩)))

/-- `MutBorrowField`, `ImmBorrowField` and their generic forms: "Consume the
reference to a struct at the top of the stack, and load a mutable [an
immutable] reference to the field identified by the field handle index"
(`file_format.rs`), the generic forms at the instance with type arguments
`typeArgs`: "Consume the reference to a generic struct". `ImmBorrowField`
accepts `&` and `&mut`. The field handle names a field of a struct definition
of the current module (L1), so fields are borrowed only in the type's defining
module: "The fields of a struct are only accessible inside the module that
defines the struct" (book, `structs-and-enums.md`, "Privileged Struct
Operations"). -/
def borrowField (P : Program) (s : State) (fr : Frame) (rest : List Frame) (info : FunctionInfo)
    (mutable : Bool) (h : FieldHandleIndex) (typeArgs : List RuntimeType := []) :
    Exec StepResult := do
  let fh ← need info.module.fieldHandles[h]? .badIndex
  let (sid, sinfo) ← localStruct P fr.function.module fh.owner typeArgs
  match sinfo.definition.fieldInformation with
  | .declared _ =>
    let tys ← need (P.fieldTypes sid) (.unsupported "field types")
    let t ← need tys[fh.field]? .badIndex
    let (r, fr) ← popRefTo fr (.struct sid) mutable
    let s := { s with frames := fr :: rest }
    let _ ← deref s r
    finish s [borrow s mutable t (r.location.child (.field fh.field))]
  | .declaredVariants _ => wrong .badIndex
  | .native => wrong .badIndex

/-- `MutBorrowVariantField`, `ImmBorrowVariantField` and their generic forms:
"Consume the reference to a struct at the top of the stack, and provided that
the struct is of the given variant, load a mutable [an immutable] reference to
the field of the variant. Aborts execution if the operand is not of the given
variant" (R1, `file_format.rs`). The handle names an enum of the current
module and "the sequence of variants which share the field at the given field
offset" (`VariantFieldHandle`), each of which has a field at that offset (L1).
The reference's target type is the field's type in the value's variant, at
the instance with type arguments `typeArgs`. -/
def borrowVariantField (P : Program) (s : State) (fr : Frame) (rest : List Frame)
    (info : FunctionInfo) (mutable : Bool) (h : VariantFieldHandleIndex)
    (typeArgs : List RuntimeType := []) : Exec StepResult := do
  let vfh ← need info.module.variantFieldHandles[h]? .badIndex
  let (sid, sinfo) ← localStruct P fr.function.module vfh.structIndex typeArgs
  match sinfo.definition.fieldInformation with
  | .declaredVariants variants =>
    let tys ← vfh.variants.mapM fun v => do
      let _ ← need variants[v]? .badIndex
      let fields ← need (P.variantFieldTypes sid v) (.unsupported "field types")
      need fields[vfh.field]? .badIndex
    let (r, fr) ← popRefTo fr (.struct sid) mutable
    let s := { s with frames := fr :: rest }
    match ← deref s r with
    | .variant _ tag _ =>
      match (vfh.variants.zip tys).find? (·.1 == tag) with
      | some (_, t) => finish s [borrow s mutable t (r.location.child (.variantField tag vfh.field))]
      | none => fail .variantMismatch
    | _ => wrong .typeMismatch
  | .declared _ => wrong .badIndex
  | .native => wrong .badIndex

/-- `TestVariant`, `TestVariantGeneric`: "Tests whether the reference value on
the stack is of the specified variant" (`file_format.rs`), of an enum of the
current module at the instance with type arguments `typeArgs`; `&` or
`&mut`. -/
def testEnum (P : Program) (s : State) (fr : Frame) (rest : List Frame) (info : FunctionInfo)
    (h : StructVariantHandleIndex) (typeArgs : List RuntimeType := []) : Exec StepResult := do
  let vh ← need info.module.structVariantHandles[h]? .badIndex
  let (sid, sinfo) ← localStruct P fr.function.module vh.structIndex typeArgs
  match sinfo.definition.fieldInformation with
  | .declaredVariants variants =>
    let _ ← need variants[vh.variant]? .badIndex
    let (r, fr) ← popRefTo fr (.struct sid) false
    let s := { s with frames := fr :: rest }
    match ← deref s r with
    | .variant _ tag _ => finish s [.bool (tag == vh.variant)]
    | _ => wrong .typeMismatch
  | .declared _ => wrong .badIndex
  | .native => wrong .badIndex

/-- Pop a `u64` index. -/
def popIndex (fr : Frame) : Exec (Nat × Frame) := do
  let (t, i, fr) ← popInt fr
  if t != .u64 then wrong .typeMismatch else pure (i.toNat, fr)

/-- The elements of the vector a reference designates. -/
def derefVector (s : State) (r : Reference) : Exec (List Value) := do
  match ← deref s r with
  | .vector _ elems => pure elems
  | _ => wrong .typeMismatch

/-- `VecImmBorrow`, `VecMutBorrow`: "Acquire an immutable [a mutable] reference
to the element at a given index of the vector. Abort the execution if the
index is out of bounds" (R6, `file_format.rs`). The index is on top, the
reference to the vector below it; `VecImmBorrow` accepts `&` and `&mut`. -/
def borrowElement (s : State) (fr : Frame) (rest : List Frame) (info : FunctionInfo)
    (mutable : Bool) (sig : SignatureIndex) : Exec StepResult := do
  let t ← elemType info.module sig fr.function.typeArgs
  let (i, fr) ← popIndex fr
  let (r, fr) ← popRefTo fr (.vector t) mutable
  let s := { s with frames := fr :: rest }
  if i < (← derefVector s r).length then finish s [borrow s mutable t (r.location.child (.element i))]
  else fail .vectorIndex

/-! ### Global storage -/

/-- The type of a global storage operation: a struct or enum definition of
the current module, at the instance with type arguments `typeArgs`, with
`key`. "Each of these instructions is parameterized by a type `T` with the
`key` ability. However, each type `T` _must be declared in the current
module_" (book, `global-storage.md`); the operand is a struct definition of
the current module, so the operations run only in the type's defining module.
"Global storage operations can be applied to generic resources with both
instantiated and uninstantiated generic type parameters" ("Global storage
operators with generics"), and the instance must have `key`: "`exists<
MyResource<NoAbilities>>`" is invalid, "'MyResource<NoAbilities>' does not
have 'key'" (book, `generics-and-abilities.md`, "Example: conditional
`key`"). -/
def globalType (P : Program) (mi : Nat) (d : StructDefinitionIndex)
    (typeArgs : List RuntimeType := []) : Exec StructId := do
  let (sid, _) ← localStruct P mi d typeArgs
  if ((RuntimeType.struct sid).abilities P).key then pure sid else wrong .globalWithoutKey

/-- R3, the resource lock on reentrancy: a global storage operation on a type
of a module that is active more than once fails (plan §6; "The Move VM
dynamically detects reentrancy of a module and _locks_ all resources declared
in that module from being accessed", book, `functions.md`, "Reentrancy
Check"). The lock covers all five operations, `Exists` and `MoveTo`
included, and is checked before existence (R5): a locked `MoveFrom` of a
missing resource, or a locked `MoveTo` onto an existing one, fails as locked
(`resourceLocked`), not as missing or existing (language-owner ruling
2026-11-03). -/
def unlocked (s : State) (sid : StructId) : Exec Unit :=
  if s.activeCount sid.module > 1 then fail .resourceLocked else pure ()

/-- Pop an address. -/
def popAddress (fr : Frame) : Exec (Address × Frame) := do
  let (v, fr) ← pop fr
  match v with
  | .address a => pure (a, fr)
  | _ => wrong .typeMismatch

/-- `MutBorrowGlobal`, `ImmBorrowGlobal` and their generic forms: "Return a
mutable [an immutable] reference to an instance of the specified type under
the address passed as argument. Abort execution if such an object does not
exist" (R5, `file_format.rs`). The lock (R3, `unlocked`) is checked first. -/
def borrowGlobal (P : Program) (s : State) (fr : Frame) (rest : List Frame) (mutable : Bool)
    (d : StructDefinitionIndex) (typeArgs : List RuntimeType := []) : Exec StepResult := do
  let sid ← globalType P fr.function.module d typeArgs
  let (a, fr) ← popAddress fr
  unlocked s sid
  if (s.storage.get? a sid).isSome then
    next s rest (advance (push fr (borrow s mutable (.struct sid) ⟨.global a sid, []⟩)))
  else fail .missingResource

/-- `Exists`, `ExistsGeneric`: "Check whether or not a given address in the
global storage has an object of the specified type already"
(`file_format.rs`). -/
def existsResource (P : Program) (s : State) (fr : Frame) (rest : List Frame)
    (d : StructDefinitionIndex) (typeArgs : List RuntimeType := []) : Exec StepResult := do
  let sid ← globalType P fr.function.module d typeArgs
  let (a, fr) ← popAddress fr
  unlocked s sid
  next s rest (advance (push fr (.bool (s.storage.get? a sid).isSome)))

/-- `MoveFrom`, `MoveFromGeneric`: "Move the value of the specified type under
the address in the global storage onto the top of the stack. Abort execution
if such an value does not exist" (`file_format.rs`). The lock (R3,
`unlocked`) is checked first. The resource's locations become invalid. -/
def moveResourceFrom (P : Program) (s : State) (fr : Frame) (rest : List Frame)
    (d : StructDefinitionIndex) (typeArgs : List RuntimeType := []) : Exec StepResult := do
  let sid ← globalType P fr.function.module d typeArgs
  let (a, fr) ← popAddress fr
  unlocked s sid
  match s.storage.get? a sid with
  | some v =>
    let s := { s with frames := fr :: rest, storage := s.storage.remove a sid }
    finish (s.invalidate (.removed (.global a sid))) [v]
  | none => fail .missingResource

/-- `MoveTo`, `MoveToGeneric`: "Move the value at the top of the stack into
the global storage, under the address of the `signer` on the stack below it.
Abort execution if an object of the same type already exists under that
address" (`file_format.rs`). The reference is `&signer` ("assert ty2 ==
&signer"), and the value has `key` all the way down. The lock (R3,
`unlocked`) is checked before existence. -/
def moveResourceTo (P : Program) (s : State) (fr : Frame) (rest : List Frame)
    (d : StructDefinitionIndex) (typeArgs : List RuntimeType := []) : Exec StepResult := do
  let sid ← globalType P fr.function.module d typeArgs
  let (v, fr) ← pop fr
  let (r, fr) ← popRefTo fr .signer false
  if r.mutable then wrong .typeMismatch
  else if !v.conforms P (.struct sid) then wrong .typeMismatch
  else if !(v.abilities P).key then wrong .globalWithoutKey
  else
    let s := { s with frames := fr :: rest }
    match ← deref s r with
    | .signer a =>
      unlocked s sid
      if (s.storage.get? a sid).isSome then fail .resourceExists
      else finish { s with storage := s.storage.insert a sid v } []
    | _ => wrong .typeMismatch

/-! ### Calls -/

/-- No mutable reference among the arguments `args` of a native call overlaps
another reference argument (`Location.overlaps`): "Move prevents from having
two mutable references to the same value, so `left` and `right` references are
always distinct" (`mem.move`, `swap`; likewise `vector.move`, `move_range`),
and Move is "strict in ensuring unique ownership of mutable references before
writes" (book, `references.md`, "Ownership"). A mutable one overlaps no
immutable one either (language-owner ruling, 2026-11-04). Immutable reference
arguments may overlap one another. -/
def exclusiveArguments (args : List Value) : Bool :=
  let refs := (args.filterMap fun v => match v with
    | .ref r => some r
    | _ => none).zipIdx
  refs.all fun (r, i) => !r.mutable || refs.all fun (r', j) => i == j || !r.location.overlaps r'.location

/-- A direct call, `Call` or `CallGeneric`, of the function that handle `h`
names, with type arguments `typeArgs` (none for `Call`). Resolution,
agreement of the caller's handle with the callee's definition, visibility,
the type arguments, argument conformance, and the module lock (R4) are
checked. The handle has one type parameter per type argument (L9), and so
does the definition (K1); the type arguments satisfy the definition's
constraints ("constraints are checked at call sites", book,
`generics-and-abilities.md`, "Verifying Constraints"). The callee runs as the
instance at the type arguments ("`ty_args = if func.is_generic then
func.ty_args else []`", "`new_frame_from_func(func, ty_args, ...)`",
`file_format.rs`, `Call`), and a native receives them with its arguments
("`call_native(func.name, ty_args, args)`"). A cross-module call counts the
callee's module as active; re-entering an active module while the lock is
held fails. The callee's frame gets a new identity. Arguments may be
references, which are passed as they are.

A native is called as follows (language-owner rulings, 2026-11-03;
`Spec.Semantics.ModeledNatives`):

- a modeled native, of the read, update or borrow shape, is suspended with its arguments
  and what it sees of them (`State.nativeView`): every reference argument must
  be valid (`invalidReference`), and no mutable reference argument may overlap
  another reference argument (`exclusiveArguments`, `overlappingReferences`);
- a native that takes and returns values only is suspended as before;
- any other native with a reference parameter or result is outside the subset
  (`unsupported`): what it reads and writes, and which references it returns,
  is not described by these shapes. -/
def call (P : Program) (s : State) (fr : Frame) (rest : List Frame) (info : FunctionInfo)
    (h : FunctionHandleIndex) (typeArgs : List RuntimeType := []) : Exec StepResult := do
  let callee ← need (P.resolve fr.function.module h) .unresolvedFunction
  if !P.visibleFrom info.moduleId callee then wrong .visibility
  else
    let calleeInfo ← need (P.function? callee) .badFunctionRef
    let fh ← need info.module.functionHandles[h]? .badIndex
    if fh.typeParameters.length != typeArgs.length then wrong .badOperand else
    if calleeInfo.handle.typeParameters.length != typeArgs.length then wrong .linkMismatch else
    if !RuntimeType.argumentsWellFormed P typeArgs calleeInfo.handle.typeParameters then
      wrong .constraintViolation
    else
      let callee := { callee with typeArgs }
      let paramTys ← need (P.paramTypes callee) (.unsupported "parameter types")
      let retTys ← need (P.returnTypes callee) (.unsupported "return types")
      let handleParams ← need (P.handleParamTypesAt fr.function.module h typeArgs)
        (.unsupported "parameter types")
      let handleReturns ← need (P.handleReturnTypesAt fr.function.module h typeArgs)
        (.unsupported "return types")
      if handleParams != paramTys || handleReturns != retTys then wrong .linkMismatch else
      if calleeInfo.definition.code.isNone && (P.nativeShape? callee).isNone &&
          !(paramTys ++ retTys).all RuntimeType.isValue then
        wrong (.unsupported "native with references")
      else
      let (args, fr) ← popN fr paramTys.length
      if !conformsAll P args paramTys then wrong .typeMismatch
      else
        match calleeInfo.definition.code with
        | none =>
          let s := { s with frames := fr :: rest }
          let views ← need (s.nativeView args) .invalidReference
          if !exclusiveArguments args then wrong .overlappingReferences
          else pure (.callNative s callee args views)
        | some _ =>
          let localTys ← need (P.localTypes callee) (.unsupported "local types")
          let locking := calleeInfo.handle.attributes.contains .moduleLock
          let lockDepth := if locking then s.lockDepth + 1 else s.lockDepth
          let cross := calleeInfo.moduleId != info.moduleId
          if cross && lockDepth > 0 && s.activeCount calleeInfo.moduleId > 0 then
            fail .moduleLocked
          else
            let s := { s with lockDepth }
            let s := if cross then s.incActive calleeInfo.moduleId else s
            let locals := (args.map some ++ List.replicate (localTys.length - args.length) none).toArray
            let frame : Frame :=
              { function := callee, id := s.nextFrame, pc := 0, locals, stack := [],
                countedActive := cross, lockEntered := locking }
            pure (.done (.running
              { s with frames := frame :: fr :: rest, nextFrame := s.nextFrame + 1 }))

/-- `Ret`: the frame's segment holds exactly the return values, which conform
to the declared return types at the frame's instance; every value left in a
local must have `drop` ("when the function returns, foo still contains a
value. This destruction requires the 'drop' ability", book,
`structs-and-enums.md`, "Ownership"). The frame's locals cease to exist: a
reference into them, returned or not, is no longer valid. The top-level
return commits global storage. -/
def ret (P : Program) (s : State) (fr : Frame) (rest : List Frame) (info : FunctionInfo) :
    Exec StepResult := do
  let retTys ← need (P.returnTypes fr.function) (.unsupported "return types")
  if fr.stack.length < retTys.length then wrong .stackUnderflow
  else if fr.stack.length > retTys.length then wrong .stackNotEmptyAtReturn
  else
    let vals := fr.stack.reverse
    if !conformsAll P vals retTys then wrong .typeMismatch
    else if fr.locals.any (fun slot => match slot with
        | some v => !(v.abilities P).drop
        | none => false) then wrong .dropWithoutAbility
    else
      let s := if fr.countedActive then s.decActive info.moduleId else s
      let s := if fr.lockEntered then { s with lockDepth := s.lockDepth - 1 } else s
      match rest with
      | [] => pure (.done (.returned vals s.storage))
      | caller :: rest' =>
        let caller := { caller with stack := fr.stack ++ caller.stack, pc := caller.pc + 1 }
        pure (.done (.running { s with frames := caller :: rest' }))

/-! ### Function values -/

/-- `PackClosure`, `PackClosureGeneric`: "`PackClosure(fun, mask)` creates a
closure for a given function handle as controlled by the given `mask`", which
"describes which of the arguments of `fun` are captured by the closure"; "the
values on the stack must match the types in the function signature which have
the bit to be captured set in the mask", and a captured type "is not a
reference" (`file_format.rs`). The function is resolved, linked and
instantiated as by a call (`call`): the handle and the definition have one
type parameter per type argument (L9, K1), the type arguments satisfy the
definition's constraints, the handle's types are the definition's, and the
function is visible from the current module: "`PackClosure`/
`PackClosureGeneric` must respect visibility" (plan §5.2). The mask names
only parameters the function has. The captured values are popped, the last
captured parameter's on top, and the closure records its type: the remaining
parameters, the results, and the abilities of its target and of its captured
values' types (`Program.closureValueAbilities`), which may exceed the
parameters' types'. A closure over a native is outside the subset. -/
def packClosure (P : Program) (s : State) (fr : Frame) (rest : List Frame) (info : FunctionInfo)
    (h : FunctionHandleIndex) (mask : BitVec 64) (typeArgs : List RuntimeType := []) :
    Exec StepResult := do
  let target ← need (P.resolve fr.function.module h) .unresolvedFunction
  if !P.visibleFrom info.moduleId target then wrong .visibility
  else
    let targetInfo ← need (P.function? target) .badFunctionRef
    let fh ← need info.module.functionHandles[h]? .badIndex
    if fh.typeParameters.length != typeArgs.length then wrong .badOperand else
    if targetInfo.handle.typeParameters.length != typeArgs.length then wrong .linkMismatch else
    if !RuntimeType.argumentsWellFormed P typeArgs targetInfo.handle.typeParameters then
      wrong .constraintViolation
    else
      let target := { target with typeArgs }
      let paramTys ← need (P.paramTypes target) (.unsupported "parameter types")
      let retTys ← need (P.returnTypes target) (.unsupported "return types")
      let handleParams ← need (P.handleParamTypesAt fr.function.module h typeArgs)
        (.unsupported "parameter types")
      let handleReturns ← need (P.handleReturnTypesAt fr.function.module h typeArgs)
        (.unsupported "return types")
      if handleParams != paramTys || handleReturns != retTys then wrong .linkMismatch else
      if targetInfo.definition.code.isNone then wrong (.unsupported "closure over a native") else
      if !maskWithin mask paramTys.length then wrong .badOperand else
      let (capturedTys, remaining) := maskSplit mask paramTys
      if !capturedTys.all RuntimeType.isValue then wrong .capturedReference else
      let (captured, fr) ← popN fr capturedTys.length
      if !conformsAll P captured capturedTys then wrong .typeMismatch
      else
        next s rest (advance (push fr
          (.closure target mask captured remaining retTys (P.closureValueAbilities target captured))))

/-- `CallClosure(|t1..tn|r has a)`: "evaluates a closure of the given function
type, taking the captured arguments and mixing in the provided ones on the
stack. On top of the stack is the closure being evaluated, underneath the
arguments: `[c,vn,..,v1] + stack`. The type of the closure must match the
type specified in the instruction, with abilities `a` a subset of the
abilities of the closure value. A value `vi` on the stack must have type `ti`"
(`file_format.rs`). The signature operand is one function type (L9),
translated under the frame's type arguments. The closure is consumed ("during
evaluation, the function value is _consumed_", book, `functions.md`,
"Operations on Functions"), and its target runs with the captured and the
provided arguments in parameter order (`maskCompose`), whatever its
visibility. Dispatch counts the target's module as active even when it is the
caller's (R3), and re-entering an active module under the module lock fails
(R4). The target's frame returns the closure's results. -/
def callClosure (P : Program) (s : State) (fr : Frame) (rest : List Frame) (info : FunctionInfo)
    (sig : SignatureIndex) : Exec StepResult := do
  let (ps, rs, a) ← match ← need info.module.signatures[sig]? .badIndex with
    | [.function ps rs a] => do
      let ps ← need (RuntimeType.ofSlotTokensWith info.module fr.function.typeArgs ps)
        (.unsupported "closure type")
      let rs ← need (RuntimeType.ofSlotTokensWith info.module fr.function.typeArgs rs)
        (.unsupported "closure type")
      pure (ps, rs, a)
    | _ => wrong .badOperand
  let (c, fr) ← pop fr
  match c with
  | .closure target mask captured cps crs ca =>
    if cps != ps || crs != rs || !a.subset ca then wrong .typeMismatch else
    let (provided, fr) ← popN fr ps.length
    if !conformsAll P provided ps then wrong .typeMismatch else
    let targetInfo ← need (P.function? target) .badFunctionRef
    match targetInfo.definition.code with
    | none => wrong (.unsupported "closure over a native")
    | some _ =>
      let args := maskCompose mask captured provided
      let paramTys ← need (P.paramTypes target) (.unsupported "parameter types")
      let retTys ← need (P.returnTypes target) (.unsupported "return types")
      if !conformsAll P args paramTys || retTys != rs then wrong .typeMismatch else
      let localTys ← need (P.localTypes target) (.unsupported "local types")
      let locking := targetInfo.handle.attributes.contains .moduleLock
      let lockDepth := if locking then s.lockDepth + 1 else s.lockDepth
      if lockDepth > 0 && s.activeCount targetInfo.moduleId > 0 then fail .moduleLocked
      else
        let s := { s with lockDepth }
        let s := s.incActive targetInfo.moduleId
        let locals := (args.map some ++ List.replicate (localTys.length - args.length) none).toArray
        let frame : Frame :=
          { function := target, id := s.nextFrame, pc := 0, locals, stack := [],
            countedActive := true, lockEntered := locking }
        pure (.done (.running
          { s with frames := frame :: fr :: rest, nextFrame := s.nextFrame + 1 }))
  | _ => wrong .typeMismatch

def exec (P : Program) (s : State) (fr : Frame) (rest : List Frame) (info : FunctionInfo)
    (code : CodeUnit) : Bytecode → Exec StepResult
  | .pop => do
    let (v, fr) ← pop fr
    if !(v.abilities P).drop then wrong .dropWithoutAbility else next s rest (advance fr)
  | .ret => ret P s fr rest info
  | .brTrue off => do
    let (b, fr) ← popBool fr
    next s rest (← if b then jump code fr off else pure (advance fr))
  | .brFalse off => do
    let (b, fr) ← popBool fr
    next s rest (← if b then pure (advance fr) else jump code fr off)
  | .branch off => do next s rest (← jump code fr off)
  | .ldU8 v => next s rest (ldInt fr .u8 v)
  | .ldU16 v => next s rest (ldInt fr .u16 v)
  | .ldU32 v => next s rest (ldInt fr .u32 v)
  | .ldU64 v => next s rest (ldInt fr .u64 v)
  | .ldU128 v => next s rest (ldInt fr .u128 v)
  | .ldU256 v => next s rest (ldInt fr .u256 v)
  | .ldI8 v => next s rest (ldInt fr .i8 v)
  | .ldI16 v => next s rest (ldInt fr .i16 v)
  | .ldI32 v => next s rest (ldInt fr .i32 v)
  | .ldI64 v => next s rest (ldInt fr .i64 v)
  | .ldI128 v => next s rest (ldInt fr .i128 v)
  | .ldI256 v => next s rest (ldInt fr .i256 v)
  | .castU8 => do next s rest (← cast fr .u8)
  | .castU16 => do next s rest (← cast fr .u16)
  | .castU32 => do next s rest (← cast fr .u32)
  | .castU64 => do next s rest (← cast fr .u64)
  | .castU128 => do next s rest (← cast fr .u128)
  | .castU256 => do next s rest (← cast fr .u256)
  | .castI8 => do next s rest (← cast fr .i8)
  | .castI16 => do next s rest (← cast fr .i16)
  | .castI32 => do next s rest (← cast fr .i32)
  | .castI64 => do next s rest (← cast fr .i64)
  | .castI128 => do next s rest (← cast fr .i128)
  | .castI256 => do next s rest (← cast fr .i256)
  -- A constant's type is closed (L8): no type parameter is in scope.
  | .ldConst i => do
    let c ← need info.module.constantPool[i]? .badIndex
    let t ← need (RuntimeType.ofToken info.module c.type) (.unsupported "constant type")
    let v ← need (decodeConstant t c.data) .badConstant
    next s rest (advance (push fr v))
  | .ldTrue => next s rest (advance (push fr (.bool true)))
  | .ldFalse => next s rest (advance (push fr (.bool false)))
  | .copyLoc i => do
    match ← need fr.locals[i]? .localOutOfRange with
    | none => wrong .unavailableLocal
    | some v =>
      if (v.abilities P).copy then next s rest (advance (push fr v))
      else wrong .copyWithoutAbility
  -- Moving a local out removes its value: every location in it becomes invalid.
  | .moveLoc i => do
    match ← need fr.locals[i]? .localOutOfRange with
    | none => wrong .unavailableLocal
    | some v =>
      next (s.invalidate (.removed (.local_ fr.id i))) rest
        (advance (push { fr with locals := fr.locals.setIfInBounds i none } v))
  -- Overwriting a local changes it in place: the local stays a valid location,
  -- every location inside its old value becomes invalid. The local's type is
  -- its declared type at the frame's instance.
  | .stLoc i => do
    let (v, fr) ← pop fr
    let tys ← need (P.localTypes fr.function) (.unsupported "local types")
    let t ← need tys[i]? .localOutOfRange
    let slot ← need fr.locals[i]? .localOutOfRange
    if !v.conforms P t then wrong .typeMismatch
    else
      match slot with
      | some old =>
        if !(old.abilities P).drop then wrong .dropWithoutAbility
        else
          next (s.invalidate (.mutated ⟨.local_ fr.id i, []⟩)) rest
            (advance { fr with locals := fr.locals.setIfInBounds i (some v) })
      | none => next s rest (advance { fr with locals := fr.locals.setIfInBounds i (some v) })
  | .call h => call P s fr rest info h
  -- "Generic version of `Call`" (`file_format.rs`): the instantiation names the
  -- function's handle and its type arguments.
  | .callGeneric i => do
    let x ← need info.module.functionInstantiations[i]? .badIndex
    call P s fr rest info x.handle (← genericArgs info.module fr.function.typeArgs x.typeParameters)
  | .add => do next s rest (← intBinop fr (fun _ => true) IntType.add)
  | .sub => do next s rest (← intBinop fr (fun _ => true) IntType.sub)
  | .mul => do next s rest (← intBinop fr (fun _ => true) IntType.mul)
  | .div => do next s rest (← intBinop fr (fun _ => true) IntType.div)
  | .mod => do next s rest (← intBinop fr (fun _ => true) IntType.rem)
  | .bitOr => do
    next s rest (← intBinop fr (fun t => !t.isSigned) (fun _ a b => some (IntType.bitOr a b)))
  | .bitAnd => do
    next s rest (← intBinop fr (fun t => !t.isSigned) (fun _ a b => some (IntType.bitAnd a b)))
  | .xor => do
    next s rest (← intBinop fr (fun t => !t.isSigned) (fun _ a b => some (IntType.bitXor a b)))
  | .shl => do next s rest (← shift fr IntType.shl)
  | .shr => do next s rest (← shift fr IntType.shr)
  | .or => do next s rest (← boolBinop fr (· || ·))
  | .and => do next s rest (← boolBinop fr (· && ·))
  | .not => do
    let (b, fr) ← popBool fr
    next s rest (advance (push fr (.bool !b)))
  | .eq => do next s rest (← equality P s fr true)
  | .neq => do next s rest (← equality P s fr false)
  | .lt => do next s rest (← intCompare fr (fun a b => decide (a < b)))
  | .gt => do next s rest (← intCompare fr (fun a b => decide (a > b)))
  | .le => do next s rest (← intCompare fr (fun a b => decide (a ≤ b)))
  | .ge => do next s rest (← intCompare fr (fun a b => decide (a ≥ b)))
  | .negate => do
    let (t, x, fr) ← popInt fr
    if !t.isSigned then wrong .typeMismatch
    else
      match t.negate x with
      | some r => next s rest (advance (push fr (.int t r)))
      | none => fail .arithmetic
  | .abort => do
    let (v, _) ← pop fr
    match v with
    | .int .u64 c => fail (.abort c.toNat)
    | _ => wrong .typeMismatch
  -- "stack >> (error_message: vector<u8>); stack >> (error_code: u64); abort
  -- transaction with error_code and error_message" (`file_format.rs`). The
  -- message is checked when the abort executes (book, "Abort messages").
  | .abortMsg => do
    let (msg, fr) ← pop fr
    let (code, _) ← pop fr
    match code, msg with
    | .int .u64 c, .vector (.int .u8) elems =>
      let bytes ← need (messageBytes elems) .typeMismatch
      if bytes.length ≤ maxAbortMessageLength && (ByteArray.mk bytes.toArray).validateUTF8 then
        fail (.abortMessage c.toNat bytes)
      else fail .invalidAbortMessage
    | _, _ => wrong .typeMismatch
  | .nop => next s rest (advance fr)
  -- Structs and enums by value, and their generic forms at the instance their
  -- instantiation names.
  | .pack d => packStruct P s fr rest d
  | .packGeneric i => do
    let x ← need info.module.structDefInstantiations[i]? .badIndex
    packStruct P s fr rest x.def_ (← genericArgs info.module fr.function.typeArgs x.typeParameters)
  | .unpack d => unpackStruct P s fr rest d
  | .unpackGeneric i => do
    let x ← need info.module.structDefInstantiations[i]? .badIndex
    unpackStruct P s fr rest x.def_ (← genericArgs info.module fr.function.typeArgs x.typeParameters)
  | .packVariant h => packEnum P s fr rest info h
  | .packVariantGeneric i => do
    let x ← need info.module.structVariantInstantiations[i]? .badIndex
    packEnum P s fr rest info x.handle (← genericArgs info.module fr.function.typeArgs x.typeParameters)
  | .unpackVariant h => unpackEnum P s fr rest info h
  | .unpackVariantGeneric i => do
    let x ← need info.module.structVariantInstantiations[i]? .badIndex
    unpackEnum P s fr rest info x.handle (← genericArgs info.module fr.function.typeArgs x.typeParameters)
  -- Vectors: `VecPack` packs "a statically known number of elements from the
  -- stack" of the type "identified by the `elem_ty_idx`"; R2: `VecUnpack`
  -- "Abort[s] if the vector does not have a length `n`" (`file_format.rs`).
  | .vecPack sig n => do
    let t ← elemType info.module sig fr.function.typeArgs
    let (vals, fr) ← popN fr n
    if !vals.all (fun v => v.conforms P t) then wrong .typeMismatch
    else next s rest (advance (push fr (.vector t vals)))
  | .vecUnpack sig n => do
    let t ← elemType info.module sig fr.function.typeArgs
    let (v, fr) ← pop fr
    match v with
    | .vector t' elems =>
      if t' != t then wrong .typeMismatch
      else if elems.length != n then fail .vectorLength
      else next s rest (advance (pushAll fr elems))
    | _ => wrong .typeMismatch
  -- References. "Both mutable and immutable references can be read to produce
  -- a copy of the referenced value", which "must have the `copy` ability as
  -- reading the reference creates a new copy of the value" (book,
  -- `references.md`).
  | .readRef => do
    let (r, fr) ← popRef fr
    let s := { s with frames := fr :: rest }
    let v ← deref s r
    if !(v.abilities P).copy then wrong .copyWithoutAbility else finish s [v]
  -- "Only mutable references can be written. A write `*x = v` discards the
  -- value previously stored in `x`", which "must have the `drop` ability"
  -- (book). "stack >> ref; stack >> val; *ref = val" (`file_format.rs`).
  | .writeRef => do
    let (r, fr) ← popRef fr
    if !r.mutable then wrong .immutableReference else
    let (v, fr) ← pop fr
    if !v.conforms P r.target then wrong .typeMismatch else
    let s := { s with frames := fr :: rest }
    let old ← deref s r
    if !(old.abilities P).drop then wrong .dropWithoutAbility
    else finish (← update s r.location v) []
  -- "Convert a mutable reference into an immutable reference" (`file_format.rs`).
  -- Freezing does not access the location.
  | .freezeRef => do
    let (r, fr) ← popRef fr
    if !r.mutable then wrong .immutableReference
    else next s rest (advance (push fr (.ref { r with mutable := false })))
  | .mutBorrowLoc i => borrowLoc P s fr rest true i
  | .immBorrowLoc i => borrowLoc P s fr rest false i
  | .mutBorrowField h => borrowField P s fr rest info true h
  | .immBorrowField h => borrowField P s fr rest info false h
  | .mutBorrowFieldGeneric i => do
    let x ← need info.module.fieldInstantiations[i]? .badIndex
    borrowField P s fr rest info true x.handle
      (← genericArgs info.module fr.function.typeArgs x.typeParameters)
  | .immBorrowFieldGeneric i => do
    let x ← need info.module.fieldInstantiations[i]? .badIndex
    borrowField P s fr rest info false x.handle
      (← genericArgs info.module fr.function.typeArgs x.typeParameters)
  | .mutBorrowVariantField h => borrowVariantField P s fr rest info true h
  | .immBorrowVariantField h => borrowVariantField P s fr rest info false h
  | .mutBorrowVariantFieldGeneric i => do
    let x ← need info.module.variantFieldInstantiations[i]? .badIndex
    borrowVariantField P s fr rest info true x.handle
      (← genericArgs info.module fr.function.typeArgs x.typeParameters)
  | .immBorrowVariantFieldGeneric i => do
    let x ← need info.module.variantFieldInstantiations[i]? .badIndex
    borrowVariantField P s fr rest info false x.handle
      (← genericArgs info.module fr.function.typeArgs x.typeParameters)
  | .testVariant h => testEnum P s fr rest info h
  | .testVariantGeneric i => do
    let x ← need info.module.structVariantInstantiations[i]? .badIndex
    testEnum P s fr rest info x.handle (← genericArgs info.module fr.function.typeArgs x.typeParameters)
  -- Vectors through references (`file_format.rs`, R6). The length is a `u64`
  -- (`vector::length<T>(self: &vector<T>): u64`, book, `vector.md`); a vector
  -- too long for one, which no VM holds in memory, fails like an arithmetic
  -- overflow rather than produce an out-of-range `u64`.
  | .vecLen sig => do
    let t ← elemType info.module sig fr.function.typeArgs
    let (r, fr) ← popRefTo fr (.vector t) false
    let s := { s with frames := fr :: rest }
    let n := (← derefVector s r).length
    if IntType.u64.inRange n then finish s [.int .u64 n] else fail .arithmetic
  | .vecImmBorrow sig => borrowElement s fr rest info false sig
  | .vecMutBorrow sig => borrowElement s fr rest info true sig
  -- "stack >> val; stack >> vec_ref; (*vec_ref) << val".
  | .vecPushBack sig => do
    let t ← elemType info.module sig fr.function.typeArgs
    let (v, fr) ← pop fr
    let (r, fr) ← popRefTo fr (.vector t) true
    if !v.conforms P t then wrong .typeMismatch else
    let s := { s with frames := fr :: rest }
    let elems ← derefVector s r
    finish (← update s r.location (.vector t (elems ++ [v]))) []
  -- "Pop an element from the end of vector. Aborts if the vector is empty."
  | .vecPopBack sig => do
    let t ← elemType info.module sig fr.function.typeArgs
    let (r, fr) ← popRefTo fr (.vector t) true
    let s := { s with frames := fr :: rest }
    match (← derefVector s r).reverse with
    | last :: init => finish (← update s r.location (.vector t init.reverse)) [last]
    | [] => fail .emptyVector
  -- "Swaps the elements at two indices in the vector. Abort the execution if any
  -- of the indices are out of bounds." "stack >> j; stack >> i; stack >> vec_ref".
  | .vecSwap sig => do
    let t ← elemType info.module sig fr.function.typeArgs
    let (j, fr) ← popIndex fr
    let (i, fr) ← popIndex fr
    let (r, fr) ← popRefTo fr (.vector t) true
    let s := { s with frames := fr :: rest }
    let elems ← derefVector s r
    match elems[i]?, elems[j]? with
    | some x, some y => finish (← update s r.location (.vector t ((elems.set i y).set j x))) []
    | some _, none => fail .vectorIndex
    | none, _ => fail .vectorIndex
  -- Global storage (book, `global-storage.md`; `file_format.rs`), and the
  -- generic forms at the instance their instantiation names.
  | .exists_ d => existsResource P s fr rest d
  | .existsGeneric i => do
    let x ← need info.module.structDefInstantiations[i]? .badIndex
    existsResource P s fr rest x.def_ (← genericArgs info.module fr.function.typeArgs x.typeParameters)
  | .moveFrom d => moveResourceFrom P s fr rest d
  | .moveFromGeneric i => do
    let x ← need info.module.structDefInstantiations[i]? .badIndex
    moveResourceFrom P s fr rest x.def_
      (← genericArgs info.module fr.function.typeArgs x.typeParameters)
  | .moveTo d => moveResourceTo P s fr rest d
  | .moveToGeneric i => do
    let x ← need info.module.structDefInstantiations[i]? .badIndex
    moveResourceTo P s fr rest x.def_ (← genericArgs info.module fr.function.typeArgs x.typeParameters)
  | .mutBorrowGlobal d => borrowGlobal P s fr rest true d
  | .immBorrowGlobal d => borrowGlobal P s fr rest false d
  | .mutBorrowGlobalGeneric i => do
    let x ← need info.module.structDefInstantiations[i]? .badIndex
    borrowGlobal P s fr rest true x.def_
      (← genericArgs info.module fr.function.typeArgs x.typeParameters)
  | .immBorrowGlobalGeneric i => do
    let x ← need info.module.structDefInstantiations[i]? .badIndex
    borrowGlobal P s fr rest false x.def_
      (← genericArgs info.module fr.function.typeArgs x.typeParameters)
  -- Function values (Phase 4, stage B).
  | .packClosure h mask => packClosure P s fr rest info h mask
  -- "Same as `PackClosure` but for the instantiation of a generic function"
  -- (`file_format.rs`).
  | .packClosureGeneric i mask => do
    let x ← need info.module.functionInstantiations[i]? .badIndex
    packClosure P s fr rest info x.handle mask
      (← genericArgs info.module fr.function.typeArgs x.typeParameters)
  | .callClosure sig => callClosure P s fr rest info sig

end Step

/-- One step of the current frame. -/
def step (P : Program) (s : State) : StepResult :=
  match s.frames with
  | [] => .done (.wrong .noFrame)
  | fr :: rest =>
    match P.function? fr.function with
    | none => .done (.wrong .badFunctionRef)
    | some info =>
      match info.definition.code with
      | none => .done (.wrong .nativeFrame)
      | some code =>
        match code.code[fr.pc]? with
        | none => .done (.wrong .fellOffEnd)
        | some instr =>
          match Step.exec P s fr rest info code instr with
          | .ok r => r
          | .error o => .done o

end MoveBytecode
