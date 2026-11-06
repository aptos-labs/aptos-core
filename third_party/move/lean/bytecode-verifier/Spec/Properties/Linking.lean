-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Semantics.Step
import Spec.Properties.Structure

/-!
# Linked-program properties K1, K2

TRUSTED. Properties of a set of modules after link checks (plan §7, group K).

Status: plan Phase 4, stage B. K1 covers function handles, each of which
must be a conservative view of its definition: as many type parameters, at
least the definition's constraints, and the same parameter and return types at
every instance (compared as runtime types, struct types by resolved name);
and struct handles, each of which must be a conservative view of its
definition (`StructHandle.ConservativeFor`). K2 covers `Call`, `CallGeneric`,
`PackClosure` and `PackClosureGeneric`, and the storability of the functions
a handle calls persistent (`PersistenceResolution`).
-/

namespace MoveBytecode

namespace Program

/-- Module ids in a program are distinct, so a module id names one module. -/
def DistinctModuleIds (P : Program) : Prop :=
  (P.modules.toList.map (·.selfId)).Nodup

/-- **K1** for function handles. Every function handle of every module
resolves to a definition (plan §7, K1: "with the same name and signature,
whose type-parameter constraints the handle's include"):

- with as many type parameters as the handle: the number of type arguments a
  call supplies is the callee's arity;
- whose constraints are each included in the handle's: the calling module
  checks the type arguments it supplies against its handle's constraints
  ("constraints are checked at call sites", book,
  `generics-and-abilities.md`, "Verifying Constraints"), which then satisfy the
  definition's. A handle may require more, as a struct handle may
  (`StructHandle.ConservativeFor`): only the presence of an ability is ever
  relied on;
- whose declared parameter and return types are those of the handle at every
  instance: under any closed type arguments of the arity, both translate to
  the same runtime types. The types must exist on both sides, so outside the
  supported subset (where translation gives `none`) the property does not hold
  vacuously. -/
def HandleResolution (P : Program) : Prop :=
  ∀ mi m, P.modules[mi]? = some m → ∀ h fh, m.functionHandles[h]? = some fh →
    ∃ f info, P.resolve mi h = some f ∧ P.function? f = some info ∧
      info.handle.typeParameters.length = fh.typeParameters.length ∧
      (∀ (i : Nat) (c d : AbilitySet), fh.typeParameters[i]? = some c →
        info.handle.typeParameters[i]? = some d → d.subset c = true) ∧
      ∀ tys : List RuntimeType, tys.length = fh.typeParameters.length →
        (∃ ps, P.handleParamTypesAt mi h tys = some ps ∧
          P.paramTypes { f with typeArgs := tys } = some ps) ∧
        (∃ rs, P.handleReturnTypesAt mi h tys = some rs ∧
          P.returnTypes { f with typeArgs := tys } = some rs)

end Program

/-- Type parameter `p` of a struct handle is a conservative view of parameter
`q` of the definition: a parameter the handle treats as phantom is phantom in
the definition, and the definition constrains it by no ability the handle
does not. -/
def StructTypeParameter.ConservativeFor (p q : StructTypeParameter) : Prop :=
  (p.isPhantom = true → q.isPhantom = true) ∧ q.constraints.subset p.constraints = true

/-- Struct handle `h` is a conservative view of the definition's own handle
`d`: it declares no ability `d` lacks, it has as many type parameters, and
each is a conservative view of `d`'s (`StructTypeParameter.ConservativeFor`).

A module is checked against the abilities, phantom flags and constraints its
own handle declares: a struct handle "carries the abilities of the struct
itself so that the verifier can check ability semantics without having to
load the referenced type" (`file_format.rs`, `StructHandle`). Each part of
the relation keeps those checks sound for the definition, which is what the
semantics uses (`RuntimeType.abilities`):

- **Abilities.** Every use of an ability requires its presence: `copy` to
  copy, `drop` to discard or compare, `store` and `key` by the field rule, and
  each ability when it is a type argument's constraint (book,
  `generics-and-abilities.md`, "Abilities"). No rule requires an ability's
  absence. A handle that declares fewer abilities only makes the importing
  module's code more conservative.
- **Phantom flags.** "The arguments to phantom parameters are excluded when
  deriving the struct abilities", and "a phantom type parameter can only
  appear in phantom position" (book, "Phantom Type Parameters"). Treating a
  parameter that is not phantom as phantom would overstate an instance's
  abilities and admit a phantom parameter where it may not appear; the
  converse understates abilities and admits less.
- **Constraints.** "Constraints are checked at call sites" (book,
  "Verifying Constraints"): the importing module checks every type argument
  it supplies against the handle's constraints, which include the
  definition's.
- **Type-parameter count** is the type's arity, which both sides must agree
  on for a type to mean the same thing.

Triage (2026-10-02, plan §10.1): K1 first required equality of abilities and
type parameters, reading the format's "an error is reported if there is a
mismatch with the definition" as inequality. Link checking accepts exactly
this weaker relation, which is sound by the reasons above, so equality was a
specification defect and is corrected here. A module's handles for its own
definitions are the definitions' handles, for which the relation is
reflexive. -/
def StructHandle.ConservativeFor (h d : StructHandle) : Prop :=
  h.abilities.subset d.abilities = true ∧
  h.typeParameters.length = d.typeParameters.length ∧
  ∀ (i : Nat) (p q : StructTypeParameter), h.typeParameters[i]? = some p →
    d.typeParameters[i]? = some q → p.ConservativeFor q

namespace Program

/-- **K1** for struct handles. Every struct handle of every module, imported
or not, names a type that resolves to a definition of which the handle is a
conservative view (`StructHandle.ConservativeFor`). The semantics takes a
struct type's abilities from its definition (`RuntimeType.abilities`); this
property makes every ability check a module's code passed against its own
handles hold for the definitions too. -/
def StructHandleResolution (P : Program) : Prop :=
  ∀ (mi : Nat) (m : CompiledModule), P.modules[mi]? = some m →
    ∀ (h : StructHandleIndex) (sh : StructHandle), m.structHandles[h]? = some sh →
    ∃ s r info, m.structIdAt h = some s ∧ P.resolveStruct s = some r ∧ P.struct? r = some info ∧
      sh.ConservativeFor info.handle

/-- **K2.** Every direct call, `Call` or `CallGeneric`, and every closure
creation, `PackClosure` or `PackClosureGeneric`, targets a function visible
from the calling module (plan §7, K2; §5.2: "Direct `Call`/`CallGeneric` and
`PackClosure`/`PackClosureGeneric` must respect visibility"). Invoking an
already obtained closure is not subject to visibility. -/
def CallVisibility (P : Program) : Prop :=
  ∀ mi m, P.modules[mi]? = some m → ∀ id, m.selfId = some id →
    ∀ d ∈ m.functionDefs.toList, ∀ code, d.code = some code →
      (∀ h, Bytecode.call h ∈ code.code.toList →
        ∃ f, P.resolve mi h = some f ∧ P.visibleFrom id f = true) ∧
      (∀ i x, Bytecode.callGeneric i ∈ code.code.toList → m.functionInstantiations[i]? = some x →
        ∃ f, P.resolve mi x.handle = some f ∧ P.visibleFrom id f = true) ∧
      (∀ h mask, Bytecode.packClosure h mask ∈ code.code.toList →
        ∃ f, P.resolve mi h = some f ∧ P.visibleFrom id f = true) ∧
      (∀ i mask x, Bytecode.packClosureGeneric i mask ∈ code.code.toList →
        m.functionInstantiations[i]? = some x →
        ∃ f, P.resolve mi x.handle = some f ∧ P.visibleFrom id f = true)

/-- **K2**, storability: every function handle that carries the `persistent`
attribute resolves to a storable function (`Program.storable`). "Only storable
functions (public or `persistent`) are packed into storable closures" (plan §7,
K2): the semantics derives a closure's `store` ability from the definition
(`Program.closureAbilities`), so what a module's handle says of the target
must hold of the definition, as K1 makes a struct handle's abilities hold of
its definition. `persistent` is an attribute an imported handle cannot claim
without the definition: it is among the attributes that "cannot be removed",
checked between "an imported function handle and the matching definition"
(`file_format.rs`, `FunctionAttribute::is_compatible_with`). How else a module
may know an imported function to be storable (`public` is not in the handle)
is not stated by the sources; the static typing that relies on it is C1's
(`typeAfter` takes `store` from the definition). -/
def PersistenceResolution (P : Program) : Prop :=
  ∀ mi m, P.modules[mi]? = some m → ∀ h fh, m.functionHandles[h]? = some fh →
    FunctionAttribute.persistent ∈ fh.attributes →
      ∃ f, P.resolve mi h = some f ∧ P.storable f = true

end Program

end MoveBytecode
