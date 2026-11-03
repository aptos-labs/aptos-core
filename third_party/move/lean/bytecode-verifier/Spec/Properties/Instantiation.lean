-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Semantics.Program

/-!
# Local instantiation (L7)

TRUSTED. Property L7 of one module's text (plan §7, group L): "there is no
type-growing cycle among the module's own generic functions, through calls or
closure creation".

**What the language promises.** "Move allows generic functions to be called
recursively. However, when used in combination with generic structs, this
could create an infinite number of types in certain cases, and allowing this
means adding unnecessary complexity to the compiler, VM, and other language
components. Therefore, such recursions are forbidden" (book,
`generics-and-abilities.md`, "Advanced Topic: Type-level Recursions"). Its
examples are judged by the number of instances: `foo1<T> -> foo1<T>` and
`foo2<T> -> foo2<A<u64>>` are "finitely many types -- allowed";
`foo<T> -> foo<A<T>>` and the two-function cycle `foo<T1, T2> -> bar<T2, T1> ->
foo<T2, A<T1>>` are "infinitely many types -- NOT allowed". And "the check for
type-level recursions is based on a conservative analysis of the call sites
and does NOT take control flow or runtime values into account": a call that
never runs counts.

**The statement.** L7 is stated as that promise, not as the graph property a
verifier checks: from any function of the module at any closed type arguments,
the function instances reachable through the module's own call instructions,
every one of them, reachable code or not, are finitely many
(`InstantiationsFinite`). The graph property (no cycle of the module's type
parameters through a call that wraps a type parameter in a larger type) is a
means to it, and the standard argument that it implies finiteness belongs to
the proof, not to the trusted statement. The statement is what a consumer
monomorphizing the module relies on, and what the book's examples decide.

Calls of other modules' functions are not followed: a module is verified
alone, and with cyclic module dependencies (plan §4.3) a cycle through another
module can grow types without end, so global finiteness is K4, under
`Acyclic P`, and otherwise R7 bounds it at run time.

**Closure creation** (Phase 4, stage B) is followed as a call is: a closure
names a function instance, fixed when the closure is created, and running the
closure runs that instance, so a consumer that compiles every instance a
module may run compiles the instances its closures name. `PackClosure` of an
own function names it without type arguments, `PackClosureGeneric` at the
instantiation's type arguments. `CallClosure` names no function: which one it
runs is the closure's, created elsewhere, and is followed at its creation.
-/

namespace MoveBytecode

namespace CompiledModule

/-- The definition of this module's own function that function handle `h`
names: `h` is a handle of this module (`selfModuleHandleIdx`), and a
definition has it as its handle. By L2 there is at most one. -/
def ownFunctionAt? (m : CompiledModule) (h : FunctionHandleIndex) :
    Option FunctionDefinitionIndex := do
  let fh ← m.functionHandles[h]?
  if fh.module = m.selfModuleHandleIdx then
    (List.range m.functionDefs.size).find? fun d => match m.functionDefs[d]? with
      | some fd => fd.function == h
      | none => false
  else none

end CompiledModule

/-- The function a direct call or a closure creation names: its handle for
`Call` and `PackClosure`, its function instantiation for `CallGeneric` and
`PackClosureGeneric`; `none` for every other instruction, `CallClosure`
included. -/
def Bytecode.directCall? : Bytecode → Option (FunctionHandleIndex ⊕ FunctionInstantiationIndex)
  | .call h => some (.inl h)
  | .callGeneric i => some (.inr i)
  | .pop | .ret | .brTrue _ | .brFalse _ | .branch _ => none
  | .ldU8 _ | .ldU16 _ | .ldU32 _ | .ldU64 _ | .ldU128 _ | .ldU256 _ => none
  | .ldI8 _ | .ldI16 _ | .ldI32 _ | .ldI64 _ | .ldI128 _ | .ldI256 _ => none
  | .castU8 | .castU16 | .castU32 | .castU64 | .castU128 | .castU256 => none
  | .castI8 | .castI16 | .castI32 | .castI64 | .castI128 | .castI256 => none
  | .ldConst _ | .ldTrue | .ldFalse | .copyLoc _ | .moveLoc _ | .stLoc _ => none
  | .add | .sub | .mul | .mod | .div | .bitOr | .bitAnd | .xor | .or | .and | .not => none
  | .eq | .neq | .lt | .gt | .le | .ge | .abort | .nop | .shl | .shr | .negate => none
  | .packClosure h _ => some (.inl h)
  | .packClosureGeneric i _ => some (.inr i)
  | .callClosure _ => none
  | .pack _ | .packGeneric _ | .packVariant _ | .packVariantGeneric _ => none
  | .unpack _ | .unpackGeneric _ | .unpackVariant _ | .unpackVariantGeneric _ => none
  | .testVariant _ | .testVariantGeneric _ => none
  | .readRef | .writeRef | .freezeRef | .mutBorrowLoc _ | .immBorrowLoc _ => none
  | .mutBorrowField _ | .mutBorrowVariantField _ | .mutBorrowFieldGeneric _
  | .mutBorrowVariantFieldGeneric _ => none
  | .immBorrowField _ | .immBorrowVariantField _ | .immBorrowFieldGeneric _
  | .immBorrowVariantFieldGeneric _ => none
  | .mutBorrowGlobal _ | .mutBorrowGlobalGeneric _ | .immBorrowGlobal _
  | .immBorrowGlobalGeneric _ => none
  | .exists_ _ | .existsGeneric _ | .moveFrom _ | .moveFromGeneric _ | .moveTo _
  | .moveToGeneric _ => none
  | .vecPack _ _ | .vecLen _ | .vecImmBorrow _ | .vecMutBorrow _ | .vecPushBack _
  | .vecPopBack _ | .vecUnpack _ _ | .vecSwap _ => none
  | .abortMsg => none

/-- An instance of a function of a module: a definition index and closed type
arguments. -/
abbrev LocalInstance := FunctionDefinitionIndex × List RuntimeType

namespace CompiledModule

/-- The instances of the module's own functions that function `d` of this
module calls, or creates closures over, when it runs at type arguments `tys`:
for each `Call` or `PackClosure` of an own function, that function without
type arguments; for each `CallGeneric` or `PackClosureGeneric` of one, that
function at the instantiation's type arguments, translated under `tys`
(`typeArgsWith`). Every such instruction of the code counts, reachable or
not. -/
def ownCalls (m : CompiledModule) (d : FunctionDefinitionIndex) (tys : List RuntimeType) :
    List LocalInstance :=
  match m.functionDefs[d]? with
  | none => []
  | some fd =>
    match fd.code with
    | none => []
    | some code => code.code.toList.filterMap fun instr =>
      match instr.directCall? with
      | some (.inl h) => do pure (← m.ownFunctionAt? h, [])
      | some (.inr i) => do
        let x ← m.functionInstantiations[i]?
        pure (← m.ownFunctionAt? x.handle, ← m.typeArgsWith tys x.typeParameters)
      | none => none

/-- Instance `j` is reachable from instance `i` through the module's own
calls and closure creations, in zero or more steps. -/
inductive ReachesInstance (m : CompiledModule) : LocalInstance → LocalInstance → Prop where
  | refl (i : LocalInstance) : m.ReachesInstance i i
  | call {i j k : LocalInstance} : m.ReachesInstance i j → k ∈ m.ownCalls j.1 j.2 →
      m.ReachesInstance i k

/-- **L7.** From every function of the module, at any closed type arguments,
one per type parameter, finitely many instances of the module's functions are
reachable through its own calls and closure creations (module docstring). -/
def InstantiationsFinite (m : CompiledModule) : Prop :=
  ∀ d fd fh (tys : List RuntimeType), m.functionDefs[d]? = some fd →
    m.functionHandles[fd.function]? = some fh → tys.length = fh.typeParameters.length →
    ∃ bound : List LocalInstance, ∀ i, m.ReachesInstance (d, tys) i → i ∈ bound

end CompiledModule

end MoveBytecode
