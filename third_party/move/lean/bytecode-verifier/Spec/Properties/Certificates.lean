-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Properties.Safety

/-!
# Static certificates C1, C2

TRUSTED. A certificate is a static fact a consumer can compute once and check
without running anything: a condition at the entry and a condition along
every edge from a reachable instruction (plan §7, group C). The semantic
statements then say that every execution agrees with the certificate.

Certificates speak only about instructions reachable from the entry in the
control-flow graph. Verified code may contain unreachable instructions that
no analysis examines, so a consumer that translates unreachable code must not
assume them (plan §10.4).

**Generic code** (plan Phase 4, stage A). A frame runs a function instance
(`FunctionRef.typeArgs`), and a certificate gives the types of each instance:
closed runtime types, which values conform to as they do in non-generic code.
C1 asks that they be "expressed over the function's type parameters" (plan
§7): every instance's types are the same tokens of the function's module,
translated under its type arguments (`TypeCertificate.Parametric`), so a
consumer computes them once per function and substitutes. Static consistency
is required of every instance whose type arguments satisfy the function's
constraints, which are the only instances that run (`Program.typeArgsOk`,
checked at every call and at entry).
-/

namespace MoveBytecode

/-- The instruction the current frame is about to execute. -/
def currentInstr (P : Program) (fr : Frame) : Option Bytecode := do
  let info ← P.function? fr.function
  let code ← info.definition.code
  code.code[fr.pc]?

/-- Instructions reachable from the entry in the control-flow graph. -/
def CodeReachable (code : Array Bytecode) (pc : Nat) : Prop :=
  ∃ p, Walk (successors code) 0 pc p

/-! ## Operand-stack types (C1) -/

/-- Pop an integer of any type and push one of type `target`. -/
def castTo (target : IntType) : List RuntimeType → Option (List RuntimeType)
  | .int _ :: rest => some (.int target :: rest)
  | _ => none

/-- Pop two integers of one type `t` with `allow t` and push `result t`. -/
def intBinary (allow : IntType → Bool) (result : IntType → RuntimeType) :
    List RuntimeType → Option (List RuntimeType)
  | .int t' :: .int t :: rest => if t = t' && allow t then some (result t :: rest) else none
  | _ => none

/-- Pop values assignable to the types `tys` (in push order, so the last is on
top) and push `result`. -/
def popPush (tys : List RuntimeType) (result : RuntimeType) (stack : List RuntimeType) :
    Option (List RuntimeType) :=
  if RuntimeType.assignableAll (stack.take tys.length) tys.reverse then
    some (result :: stack.drop tys.length)
  else none

/-- Pop a value of type `t` and push the types `tys` (in push order, so the
last ends on top). -/
def popPushAll (t : RuntimeType) (tys : List RuntimeType) : List RuntimeType →
    Option (List RuntimeType)
  | t' :: rest => if t' = t then some (tys.reverse ++ rest) else none
  | [] => none

/-- The type of the field that a variant field handle borrows: the field at
its offset in each variant it lists, which must all have one type; `none` if
they do not, or the list is empty. -/
def Program.variantFieldType? (P : Program) (s : StructId) (h : VariantFieldHandle) :
    Option RuntimeType := do
  let tys ← h.variants.mapM fun v => do (← P.variantFieldTypes s v)[h.field]?
  match tys with
  | t :: ts => if ts.all (· == t) then some t else none
  | [] => none

/-- The operand-stack types after an instruction of function instance `f` in
module `m`, top first, from the types before it; `none` if the instruction
does not apply to them. This is the static typing discipline of the supported
subset, written from the semantics: the types of struct and enum fields are
those the semantics checks (`Program.fieldTypes`, `Program.variantFieldTypes`)
at the instance, and abilities are those of the types
(`RuntimeType.abilities`). `UnpackVariant` pushes the named variant's field
types; whether the value is of that variant is R1, a dynamic check. A variant
field borrow pushes a reference to the field type its listed variants share
(`Program.variantFieldType?`). An instruction that only reads through a
reference accepts `&T` and `&mut T`; one that writes or borrows mutably
requires `&mut T`.

Every type an instruction names is translated under `f`'s type arguments. A
generic form applies as its non-generic form at the instance its
instantiation names, which must be a type of the program
(`RuntimeType.wellFormed`); `CallGeneric`'s type arguments must satisfy the
constraints of the caller's handle, as "constraints are checked at call
sites" (book, `generics-and-abilities.md`, "Verifying Constraints"), and K1
makes those at least the definition's. `Call` names a non-generic handle.

A value moves into a typed slot (a local, a parameter, a field, a vector
element, a return value, a written location) when its type is assignable to
the slot's (`RuntimeType.assignable`): a function type with more abilities to
one with fewer. `PackClosure` pushes the closure's type: the remaining
parameters, the results, and the abilities of its target, `store` from the
definition (`Program.closureAbilities`), and of the captured values' types on
the stack, which may exceed the captured parameters' (as
`Program.closureValueAbilities` takes them at run time).
`CallClosure` takes a closure whose type has the instruction's parameters and
results and at least its abilities ("with abilities `a` a subset of the
abilities of the closure value", `file_format.rs`), and the arguments
below it. -/
def typeAfter (P : Program) (f : FunctionRef) (m : CompiledModule) (instr : Bytecode)
    (stack : List RuntimeType) : Option (List RuntimeType) :=
  let local? (i : Nat) : Option RuntimeType := do (← P.localTypes f)[i]?
  -- The type arguments a generic form supplies, at least one.
  let typeArgs (sig : SignatureIndex) : Option (List RuntimeType) := do
    match ← m.typeArgsWith f.typeArgs sig with
    | [] => none
    | t :: ts => some (t :: ts)
  -- The type that definition `d` defines, at the type arguments of
  -- instantiation signature `sig`: a type of the program.
  let instanceAt (d : StructDefinitionIndex) (sig : SignatureIndex) : Option StructId := do
    let s ← P.definedStruct? f.module d
    let s := { s with typeArgs := ← typeArgs sig }
    if (RuntimeType.struct s).wellFormed P then some s else none
  -- A reference of mutability `mutable` to type `t`, borrowed from a reference
  -- to a value of struct type `s` on top.
  let borrowIn (s : StructId) (mutable : Bool) (t : RuntimeType) : Option (List RuntimeType) :=
    match stack with
    | .ref mut' (.struct s') :: rest =>
      if s' = s && (mut' || !mutable) then some (.ref mutable t :: rest) else none
    | _ => none
  let fieldBorrow (h : FieldHandleIndex) (mutable : Bool) : Option (List RuntimeType) := do
    let fh ← m.fieldHandles[h]?
    let s ← P.definedStruct? f.module fh.owner
    borrowIn s mutable (← (← P.fieldTypes s)[fh.field]?)
  let fieldBorrowGeneric (i : FieldInstantiationIndex) (mutable : Bool) :
      Option (List RuntimeType) := do
    let x ← m.fieldInstantiations[i]?
    let fh ← m.fieldHandles[x.handle]?
    let s ← instanceAt fh.owner x.typeParameters
    borrowIn s mutable (← (← P.fieldTypes s)[fh.field]?)
  let variantFieldBorrow (h : VariantFieldHandleIndex) (mutable : Bool) :
      Option (List RuntimeType) := do
    let vfh ← m.variantFieldHandles[h]?
    let s ← P.definedStruct? f.module vfh.structIndex
    borrowIn s mutable (← P.variantFieldType? s vfh)
  let variantFieldBorrowGeneric (i : VariantFieldInstantiationIndex) (mutable : Bool) :
      Option (List RuntimeType) := do
    let x ← m.variantFieldInstantiations[i]?
    let vfh ← m.variantFieldHandles[x.handle]?
    let s ← instanceAt vfh.structIndex x.typeParameters
    borrowIn s mutable (← P.variantFieldType? s vfh)
  let elementBorrow (sig : SignatureIndex) (mutable : Bool) : Option (List RuntimeType) := do
    let t ← m.vectorElemTypeWith f.typeArgs sig
    match stack with
    | .int .u64 :: .ref mut' (.vector t') :: rest =>
      if t' = t && (mut' || !mutable) then some (.ref mutable t :: rest) else none
    | _ => none
  -- The type of a global storage operation, which must have `key`.
  let keyType (d : StructDefinitionIndex) : Option StructId := do
    let s ← P.definedStruct? f.module d
    if ((RuntimeType.struct s).abilities P).key then some s else none
  let keyTypeGeneric (i : StructDefInstantiationIndex) : Option StructId := do
    let x ← m.structDefInstantiations[i]?
    let s ← instanceAt x.def_ x.typeParameters
    if ((RuntimeType.struct s).abilities P).key then some s else none
  let globalBorrow (d : StructDefinitionIndex) (mutable : Bool) : Option (List RuntimeType) := do
    let s ← keyType d
    match stack with
    | .address :: rest => some (.ref mutable (.struct s) :: rest)
    | _ => none
  let globalBorrowGeneric (i : StructDefInstantiationIndex) (mutable : Bool) :
      Option (List RuntimeType) := do
    let s ← keyTypeGeneric i
    match stack with
    | .address :: rest => some (.ref mutable (.struct s) :: rest)
    | _ => none
  -- A call of the function handle `h` with type arguments `tys`.
  let callOf (h : FunctionHandleIndex) (tys : List RuntimeType) : Option (List RuntimeType) := do
    let params ← P.handleParamTypesAt f.module h tys
    let returns ← P.handleReturnTypesAt f.module h tys
    if RuntimeType.assignableAll (stack.take params.length) params.reverse then
      some (returns.reverse ++ stack.drop params.length)
    else none
  -- A closure over the function handle `h` at type arguments `tys`, capturing
  -- the parameters `mask` selects.
  let closureOf (h : FunctionHandleIndex) (mask : BitVec 64) (tys : List RuntimeType) :
      Option (List RuntimeType) := do
    let params ← P.handleParamTypesAt f.module h tys
    let returns ← P.handleReturnTypesAt f.module h tys
    let target ← P.resolve f.module h
    if !maskWithin mask params.length then none else
    let (captured, remaining) := maskSplit mask params
    if !captured.all RuntimeType.isValue then none else
    let abilities := (P.closureAbilities { target with typeArgs := tys }).inter
      (RuntimeType.capturedAbilities P (stack.take captured.length))
    popPush captured (.function remaining returns abilities) stack
  match instr with
  | .pop => match stack with
    | t :: rest => if (t.abilities P).drop then some rest else none
    | [] => none
  | .ret => do
    if RuntimeType.assignableAll stack (← P.returnTypes f).reverse then some [] else none
  | .brTrue _ | .brFalse _ => match stack with
    | .bool :: rest => some rest
    | _ => none
  | .branch _ | .nop => some stack
  | .ldU8 _ => some (.int .u8 :: stack)
  | .ldU16 _ => some (.int .u16 :: stack)
  | .ldU32 _ => some (.int .u32 :: stack)
  | .ldU64 _ => some (.int .u64 :: stack)
  | .ldU128 _ => some (.int .u128 :: stack)
  | .ldU256 _ => some (.int .u256 :: stack)
  | .ldI8 _ => some (.int .i8 :: stack)
  | .ldI16 _ => some (.int .i16 :: stack)
  | .ldI32 _ => some (.int .i32 :: stack)
  | .ldI64 _ => some (.int .i64 :: stack)
  | .ldI128 _ => some (.int .i128 :: stack)
  | .ldI256 _ => some (.int .i256 :: stack)
  | .ldTrue | .ldFalse => some (.bool :: stack)
  | .ldConst i => do some ((← RuntimeType.ofToken m (← m.constantPool[i]?).type) :: stack)
  | .castU8 => castTo .u8 stack
  | .castU16 => castTo .u16 stack
  | .castU32 => castTo .u32 stack
  | .castU64 => castTo .u64 stack
  | .castU128 => castTo .u128 stack
  | .castU256 => castTo .u256 stack
  | .castI8 => castTo .i8 stack
  | .castI16 => castTo .i16 stack
  | .castI32 => castTo .i32 stack
  | .castI64 => castTo .i64 stack
  | .castI128 => castTo .i128 stack
  | .castI256 => castTo .i256 stack
  | .copyLoc i => do
    let t ← local? i
    if (t.abilities P).copy then some (t :: stack) else none
  | .moveLoc i => do some ((← local? i) :: stack)
  | .stLoc i => do
    let t ← local? i
    match stack with
    | t' :: rest => if t'.assignable t then some rest else none
    | [] => none
  | .call h => do
    let fh ← m.functionHandles[h]?
    if !fh.typeParameters.isEmpty then none else callOf h []
  | .callGeneric i => do
    let x ← m.functionInstantiations[i]?
    let fh ← m.functionHandles[x.handle]?
    let tys ← typeArgs x.typeParameters
    if !RuntimeType.argumentsWellFormed P tys fh.typeParameters then none else callOf x.handle tys
  | .add | .sub | .mul | .div | .mod => intBinary (fun _ => true) .int stack
  | .bitOr | .bitAnd | .xor => intBinary (fun t => !t.isSigned) .int stack
  | .lt | .gt | .le | .ge => intBinary (fun _ => true) (fun _ => .bool) stack
  | .shl | .shr => match stack with
    | .int .u8 :: .int t :: rest => if !t.isSigned then some (.int t :: rest) else none
    | _ => none
  | .or | .and => match stack with
    | .bool :: .bool :: rest => some (.bool :: rest)
    | _ => none
  | .not => match stack with
    | .bool :: rest => some (.bool :: rest)
    | _ => none
  | .eq | .neq => match stack with
    | t' :: t :: rest =>
      if RuntimeType.comparable t t' && (t.abilities P).drop && (t'.abilities P).drop then
        some (.bool :: rest)
      else none
    | _ => none
  | .negate => match stack with
    | .int t :: rest => if t.isSigned then some (.int t :: rest) else none
    | _ => none
  | .abort => match stack with
    | .int .u64 :: rest => some rest
    | _ => none
  | .abortMsg => match stack with
    | .vector (.int .u8) :: .int .u64 :: rest => some rest
    | _ => none
  | .pack d => do
    let s ← P.definedStruct? f.module d
    popPush (← P.fieldTypes s) (.struct s) stack
  | .unpack d => do
    let s ← P.definedStruct? f.module d
    popPushAll (.struct s) (← P.fieldTypes s) stack
  | .packVariant h => do
    let vh ← m.structVariantHandles[h]?
    let s ← P.definedStruct? f.module vh.structIndex
    popPush (← P.variantFieldTypes s vh.variant) (.struct s) stack
  | .unpackVariant h => do
    let vh ← m.structVariantHandles[h]?
    let s ← P.definedStruct? f.module vh.structIndex
    popPushAll (.struct s) (← P.variantFieldTypes s vh.variant) stack
  | .packGeneric i => do
    let x ← m.structDefInstantiations[i]?
    let s ← instanceAt x.def_ x.typeParameters
    popPush (← P.fieldTypes s) (.struct s) stack
  | .unpackGeneric i => do
    let x ← m.structDefInstantiations[i]?
    let s ← instanceAt x.def_ x.typeParameters
    popPushAll (.struct s) (← P.fieldTypes s) stack
  | .packVariantGeneric i => do
    let x ← m.structVariantInstantiations[i]?
    let vh ← m.structVariantHandles[x.handle]?
    let s ← instanceAt vh.structIndex x.typeParameters
    popPush (← P.variantFieldTypes s vh.variant) (.struct s) stack
  | .unpackVariantGeneric i => do
    let x ← m.structVariantInstantiations[i]?
    let vh ← m.structVariantHandles[x.handle]?
    let s ← instanceAt vh.structIndex x.typeParameters
    popPushAll (.struct s) (← P.variantFieldTypes s vh.variant) stack
  | .vecPack sig n => do
    let t ← m.vectorElemTypeWith f.typeArgs sig
    popPush (List.replicate n t) (.vector t) stack
  | .vecUnpack sig n => do
    let t ← m.vectorElemTypeWith f.typeArgs sig
    popPushAll (.vector t) (List.replicate n t) stack
  -- References.
  | .readRef => match stack with
    | .ref _ t :: rest => if (t.abilities P).copy then some (t :: rest) else none
    | _ => none
  | .writeRef => match stack with
    | .ref true t :: t' :: rest => if t'.assignable t && (t.abilities P).drop then some rest else none
    | _ => none
  | .freezeRef => match stack with
    | .ref true t :: rest => some (.ref false t :: rest)
    | _ => none
  | .mutBorrowLoc i => do
    let t ← local? i
    if t.isValue then some (.ref true t :: stack) else none
  | .immBorrowLoc i => do
    let t ← local? i
    if t.isValue then some (.ref false t :: stack) else none
  | .mutBorrowField h => fieldBorrow h true
  | .immBorrowField h => fieldBorrow h false
  | .mutBorrowVariantField h => variantFieldBorrow h true
  | .immBorrowVariantField h => variantFieldBorrow h false
  | .mutBorrowFieldGeneric i => fieldBorrowGeneric i true
  | .immBorrowFieldGeneric i => fieldBorrowGeneric i false
  | .mutBorrowVariantFieldGeneric i => variantFieldBorrowGeneric i true
  | .immBorrowVariantFieldGeneric i => variantFieldBorrowGeneric i false
  | .testVariant h => do
    let vh ← m.structVariantHandles[h]?
    let s ← P.definedStruct? f.module vh.structIndex
    match stack with
    | .ref _ (.struct s') :: rest => if s' = s then some (.bool :: rest) else none
    | _ => none
  | .testVariantGeneric i => do
    let x ← m.structVariantInstantiations[i]?
    let vh ← m.structVariantHandles[x.handle]?
    let s ← instanceAt vh.structIndex x.typeParameters
    match stack with
    | .ref _ (.struct s') :: rest => if s' = s then some (.bool :: rest) else none
    | _ => none
  | .vecLen sig => do
    let t ← m.vectorElemTypeWith f.typeArgs sig
    match stack with
    | .ref _ (.vector t') :: rest => if t' = t then some (.int .u64 :: rest) else none
    | _ => none
  | .vecImmBorrow sig => elementBorrow sig false
  | .vecMutBorrow sig => elementBorrow sig true
  | .vecPushBack sig => do
    let t ← m.vectorElemTypeWith f.typeArgs sig
    match stack with
    | t' :: .ref true (.vector t'') :: rest => if t'.assignable t && t'' = t then some rest else none
    | _ => none
  | .vecPopBack sig => do
    let t ← m.vectorElemTypeWith f.typeArgs sig
    match stack with
    | .ref true (.vector t') :: rest => if t' = t then some (t :: rest) else none
    | _ => none
  | .vecSwap sig => do
    let t ← m.vectorElemTypeWith f.typeArgs sig
    match stack with
    | .int .u64 :: .int .u64 :: .ref true (.vector t') :: rest => if t' = t then some rest else none
    | _ => none
  -- Global storage.
  | .exists_ d => do
    let _ ← keyType d
    match stack with
    | .address :: rest => some (.bool :: rest)
    | _ => none
  | .moveFrom d => do
    let s ← keyType d
    match stack with
    | .address :: rest => some (.struct s :: rest)
    | _ => none
  | .moveTo d => do
    let s ← keyType d
    match stack with
    | .struct s' :: .ref false .signer :: rest => if s' = s then some rest else none
    | _ => none
  | .mutBorrowGlobal d => globalBorrow d true
  | .immBorrowGlobal d => globalBorrow d false
  | .existsGeneric i => do
    let _ ← keyTypeGeneric i
    match stack with
    | .address :: rest => some (.bool :: rest)
    | _ => none
  | .moveFromGeneric i => do
    let s ← keyTypeGeneric i
    match stack with
    | .address :: rest => some (.struct s :: rest)
    | _ => none
  | .moveToGeneric i => do
    let s ← keyTypeGeneric i
    match stack with
    | .struct s' :: .ref false .signer :: rest => if s' = s then some rest else none
    | _ => none
  | .mutBorrowGlobalGeneric i => globalBorrowGeneric i true
  | .immBorrowGlobalGeneric i => globalBorrowGeneric i false
  -- Function values.
  | .packClosure h mask => do
    let fh ← m.functionHandles[h]?
    if !fh.typeParameters.isEmpty then none else closureOf h mask []
  | .packClosureGeneric i mask => do
    let x ← m.functionInstantiations[i]?
    let fh ← m.functionHandles[x.handle]?
    let tys ← typeArgs x.typeParameters
    if !RuntimeType.argumentsWellFormed P tys fh.typeParameters then none
    else closureOf x.handle mask tys
  | .callClosure sig => do
    match ← m.signatures[sig]? with
    | [.function ps rs a] =>
      let ps ← RuntimeType.ofSlotTokensWith m f.typeArgs ps
      let rs ← RuntimeType.ofSlotTokensWith m f.typeArgs rs
      match stack with
      | .function ps' rs' a' :: rest =>
        if ps' = ps && rs' = rs && a.subset a' &&
            RuntimeType.assignableAll (rest.take ps.length) ps.reverse then
          some (rs.reverse ++ rest.drop ps.length)
        else none
      | _ => none
    | _ => none

/-- A certificate of operand-stack types: for each function instance and pc,
the types of the frame's own segment, top first. -/
abbrev TypeCertificate := FunctionRef → Nat → Option (List RuntimeType)

/-- Static consistency of a type certificate: for every instance of a function
with type arguments satisfying its constraints, the stack is empty at entry,
and every reachable instruction applies to the certified types, with its
result assignable to the types certified at each successor
(`RuntimeType.assignableAll`): a successor may certify a function type with
fewer abilities than the instruction produced. Values conform to the
certificate up to assignability (`StaticTypes`), and an instruction's result
may have more abilities than one token can name for every instance: a closure
capturing a value of type `T` has the abilities of `T`'s instance, while the
certificate is one token for every instance (`TypeCertificate.Parametric`). -/
def TypeCertificate.Consistent (P : Program) (cert : TypeCertificate) : Prop :=
  ∀ f info code, P.function? f = some info → info.definition.code = some code →
    P.typeArgsOk f = true →
    cert f 0 = some [] ∧
    ∀ pc instr, CodeReachable code.code pc → code.code[pc]? = some instr →
      ∃ before after, cert f pc = some before ∧
        typeAfter P f info.module instr before = some after ∧
        ∀ s ∈ successors code.code pc,
          ∃ certified, cert f s = some certified ∧ RuntimeType.assignableAll after certified = true

/-- **C1** is expressed over the function's type parameters (plan §7, C1): the
types a certificate gives an instance are the slot tokens `σ` gives the
function and pc, the same for every instance, translated under the instance's
type arguments. For a non-generic function, they are the translations of
closed tokens. -/
def TypeCertificate.Parametric (P : Program) (cert : TypeCertificate) : Prop :=
  ∃ σ : Nat → FunctionDefinitionIndex → Nat → Option Signature,
    ∀ f info, P.function? f = some info → ∀ pc,
      cert f pc = (σ f.module f.index pc).bind (RuntimeType.ofSignatureWith info.module f.typeArgs)

/-- **C1** (agreement). In every reachable state, the current frame's segment
conforms to the certificate, and every available local conforms to its
declared type, at the frame's instance. -/
def StaticTypes (P : Program) (N : NativeRel) (cert : TypeCertificate) : Prop :=
  ∀ s₀, AdmissibleInitial P s₀ → ∀ s, Reachable P N s₀ s → ∀ fr rest, s.frames = fr :: rest →
    (∃ tys : List RuntimeType, cert fr.function fr.pc = some tys ∧
      conformsAll P fr.stack tys = true) ∧
    (∃ localTys : List RuntimeType, P.localTypes fr.function = some localTys ∧
      ∀ (i : Nat) (v : Value), fr.locals[i]? = some (some v) →
        ∃ t : RuntimeType, localTys[i]? = some t ∧ v.conforms P t = true)

/-! ## Local availability (C2) -/

inductive Availability where
  | available
  | unavailable
  | maybe
  deriving DecidableEq, Repr

/-- `a` is at least as precise as `b`: equal, or `b` is `maybe`. -/
def Availability.Refines (a b : Availability) : Prop := a = b ∨ b = .maybe

/-- A certificate of local availability: for each function instance, pc and
local. Availability does not depend on types, so a certificate may give every
instance of a function the same. -/
abbrev AvailabilityCertificate := FunctionRef → Nat → LocalIndex → Availability

/-- The local an instruction reads, if any. -/
def Bytecode.readsLocal : Bytecode → Option LocalIndex
  | .copyLoc i | .moveLoc i | .mutBorrowLoc i | .immBorrowLoc i => some i
  | _ => none

/-- Availability after an instruction: `StLoc` makes a local available,
`MoveLoc` makes it unavailable. -/
def availabilityAfter (instr : Bytecode) (before : LocalIndex → Availability) :
    LocalIndex → Availability :=
  match instr with
  | .stLoc i => fun j => if j = i then .available else before j
  | .moveLoc i => fun j => if j = i then .unavailable else before j
  | _ => before

/-- Static consistency of an availability certificate: parameters are
available and other locals unavailable at entry, every reachable instruction
reads only available locals, and the availability after it refines into the
certificate at each successor. -/
def AvailabilityCertificate.Consistent (P : Program) (cert : AvailabilityCertificate) : Prop :=
  ∀ f info code, P.function? f = some info → info.definition.code = some code →
    ∃ params : Signature, info.module.signatures[info.handle.parameters]? = some params ∧
      (∀ i, Availability.Refines (if i < params.length then .available else .unavailable)
        (cert f 0 i)) ∧
      ∀ pc instr, CodeReachable code.code pc → code.code[pc]? = some instr →
        (∀ i, instr.readsLocal = some i → cert f pc i = .available) ∧
        ∀ s ∈ successors code.code pc, ∀ i,
          Availability.Refines (availabilityAfter instr (cert f pc) i) (cert f s i)

/-- **C2** (agreement). In every reachable state, the current frame's locals
agree with the certificate. -/
def StaticAvailability (P : Program) (N : NativeRel) (cert : AvailabilityCertificate) : Prop :=
  ∀ s₀, AdmissibleInitial P s₀ → ∀ s, Reachable P N s₀ s → ∀ fr rest, s.frames = fr :: rest →
    (∀ i, cert fr.function fr.pc i = .available →
      ∃ v : Value, fr.locals[i]? = some (some v)) ∧
    (∀ i, cert fr.function fr.pc i = .unavailable → fr.locals[i]? = some none)

end MoveBytecode
