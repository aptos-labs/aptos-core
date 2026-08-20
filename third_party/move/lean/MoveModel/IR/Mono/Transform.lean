-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Module

/-!
# Verification monomorphization

This pass implements the finite-instance part of the TACAS'22 Move Prover
strategy.  A generic function is represented first by a *given-type* instance:
each still-free declaration parameter becomes a rigid `Ty.typeParam` whose
index is namespaced by the source function.  Extra instances are generated for
the substitutions under which two resource types observed by the function
have the same runtime tag.  Combining compatible substitutions covers
simultaneous collisions such as `R<T> = R<u64>` and `S<U> = S<T>`.

The output is a finite module view.  Function declarations have no type
parameters and instantiated calls are rewritten to ordinary calls to entries
in the plan.  Generic struct operations remain explicitly tagged: those tags
are the observable effects which the coverage argument must preserve.
-/

namespace MoveModel.IR

/-- Whether an observed resource is only read or may be changed.  Discovery
currently compares all observations; retaining the mode documents the data
needed by a future less-conservative analysis. -/
inductive TagEffectMode where
  | read
  | write
  deriving BEq, Repr

/-- One runtime resource-tag observation in code or a specification. -/
structure TagEffect where
  mode : TagEffectMode
  resource : ResourceId
  typeArgs : List Ty
  deriving BEq, Repr

/-- One specialized function in a finite monomorphization plan. -/
structure MonoKey where
  funId : FunId
  typeArgs : List Ty
  deriving Repr

/-- Equality of specialization keys at the runtime type-tag level. -/
def MonoKey.RuntimeEq (lhs rhs : MonoKey) : Prop :=
  lhs.funId = rhs.funId ∧
    lhs.typeArgs.map Ty.toTag = rhs.typeArgs.map Ty.toTag

/-- Executable equality of specialization keys. Types are compared through
their collision-free runtime encodings, which is the identity observed by
generic Move operations and global storage. -/
def MonoKey.beq (lhs rhs : MonoKey) : Bool :=
  lhs.funId == rhs.funId &&
    lhs.typeArgs.map Ty.toTag == rhs.typeArgs.map Ty.toTag

instance : BEq MonoKey := ⟨MonoKey.beq⟩

instance : ReflBEq MonoKey where
  rfl := by
    intro key
    cases key with
    | mk funId typeArgs =>
        change (funId == funId &&
          typeArgs.map Ty.toTag == typeArgs.map Ty.toTag) = true
        rw [beq_self_eq_true funId,
          beq_self_eq_true (typeArgs.map Ty.toTag)]
        rfl

/-- A deterministic list is used instead of a finite map so its position is
also the generated function id. -/
structure MonoPlan where
  entries : List MonoKey
  deriving BEq, Repr

/-- Hard guard against malformed inputs with recursively growing type
instantiations.  Compiler-v2 rejects those cycles before XIR is produced; the
Lean pass checks again rather than relying on that external fact. -/
def maxMonoInstances : Nat := 1024

private def beqMem [BEq α] (x : α) (xs : List α) : Bool :=
  xs.any fun y => x == y

private def dedup [BEq α] : List α → List α
  | [] => []
  | x :: xs =>
      let rest := dedup xs
      if beqMem x rest then rest else x :: rest

/-- Replace one type parameter throughout a type expression. -/
partial def Ty.substituteParam (index : Nat) (replacement : Ty) : Ty → Ty
  | .typeParam i => if i = index then replacement else .typeParam i
  | .structInst r args => .structInst r (args.map (substituteParam index replacement))
  | .enumInst r args => .enumInst r (args.map (substituteParam index replacement))
  | .vector elem => .vector (substituteParam index replacement elem)
  | .ref elem => .ref (substituteParam index replacement elem)
  | .mutRef elem => .mutRef (substituteParam index replacement elem)
  | ty => ty

/-- Occurs check used by runtime-tag unification. -/
partial def Ty.occursParam (index : Nat) : Ty → Bool
  | .typeParam i => i == index
  | .structInst _ args | .enumInst _ args =>
      args.any (occursParam index)
  | .vector elem | .ref elem | .mutRef elem => elem.occursParam index
  | _ => false

/-- The identity substitution for `arity` declaration parameters. -/
def identityTypeArgs (arity : Nat) : List Ty :=
  (List.range arity).map Ty.typeParam

private def typeEquations (lhs rhs : Ty) : Option (List (Ty × Ty)) :=
  if lhs == rhs then some [] else
  match lhs, rhs with
  | .typeParam _, _ | _, .typeParam _ => some [(lhs, rhs)]
  | .structInst r xs, .structInst r' ys
  | .enumInst r xs, .enumInst r' ys =>
      if r == r' && xs.length == ys.length then some (xs.zip ys) else none
  | .vector x, .vector y | .ref x, .ref y | .mutRef x, .mutRef y =>
      some [(x, y)]
  | _, _ => none

partial def unifyTypeEquations (arity : Nat) :
    List (Ty × Ty) → List Ty → Option (List Ty)
  | [], subst => some subst
  | (lhs, rhs) :: equations, subst =>
      let lhs := lhs.instantiate subst
      let rhs := rhs.instantiate subst
      if lhs == rhs then
        unifyTypeEquations arity equations subst
      else
        match lhs, rhs with
        | .typeParam index, ty =>
            if index < arity && !ty.occursParam index then
              let subst := subst.map (Ty.substituteParam index ty)
              let equations := equations.map fun (x, y) =>
                (Ty.substituteParam index ty x, Ty.substituteParam index ty y)
              unifyTypeEquations arity equations subst
            else none
        | ty, .typeParam index =>
            if index < arity && !ty.occursParam index then
              let subst := subst.map (Ty.substituteParam index ty)
              let equations := equations.map fun (x, y) =>
                (Ty.substituteParam index ty x, Ty.substituteParam index ty y)
              unifyTypeEquations arity equations subst
            else none
        | _, _ =>
            match typeEquations lhs rhs with
            | some more => unifyTypeEquations arity (more ++ equations) subst
            | none => none

/-- Unify two resource type-argument lists, solving only the declaration's
own type parameters. -/
def unifyTypeArgs (arity : Nat) (lhs rhs : List Ty) : Option (List Ty) :=
  if lhs.length != rhs.length then none
  else unifyTypeEquations arity (lhs.zip rhs) (identityTypeArgs arity)

/-- Combine two compatible partial instantiations. -/
def combineTypeArgs (arity : Nat) (lhs rhs : List Ty) : Option (List Ty) :=
  if lhs.length != arity || rhs.length != arity then none
  else
    let vars := identityTypeArgs arity
    unifyTypeEquations arity ((vars.zip lhs) ++ (vars.zip rhs)) vars

private partial def closeTypeArgs.go (arity : Nat)
    (known work : List (List Ty)) : List (List Ty) :=
  match work with
  | [] => known
  | current :: pending =>
      let generated := known.filterMap (combineTypeArgs arity current)
      let fresh := generated.filter fun args => !beqMem args known
      let known := known ++ fresh
      go arity known (pending ++ fresh)

/-- Close partial substitutions under compatible combinations. -/
def closeTypeArgs (arity : Nat) (instances : List (List Ty)) : List (List Ty) :=
  let instances := dedup (identityTypeArgs arity :: instances)
  closeTypeArgs.go arity instances instances

/-- A collision substitution for each pair of observations of the same
resource, closed under simultaneous combinations. -/
def discoverCollisionArgs (arity : Nat) (effects : List TagEffect) : List (List Ty) :=
  let candidates := effects.flatMap fun lhs =>
    effects.filterMap fun rhs =>
      if lhs.resource == rhs.resource then
        unifyTypeArgs arity lhs.typeArgs rhs.typeArgs
      else none
  closeTypeArgs arity candidates

/-- Namespace an unbound parameter index.  The specialized declaration has
no binders, so the resulting `typeParam` is a rigid given type. -/
def givenTypeIndex (owner index : Nat) : Nat :=
  let diagonal := owner + index
  diagonal * (diagonal + 1) / 2 + index

/-- Turn the free parameters left by partial unification into rigid given
types owned by a verification root. -/
def Ty.rigidify (owner : FunId) : Ty → Ty
  | .typeParam i => .typeParam (givenTypeIndex owner i)
  | .structInst r args => .structInst r (args.map (rigidify owner))
  | .enumInst r args => .enumInst r (args.map (rigidify owner))
  | .vector elem => .vector (elem.rigidify owner)
  | .ref elem => .ref (elem.rigidify owner)
  | .mutRef elem => .mutRef (elem.rigidify owner)
  | ty => ty

def rigidifyTypeArgs (owner : FunId) (args : List Ty) : List Ty :=
  args.map (Ty.rigidify owner)

/-- A concrete source instantiation contains no declaration-scoped type
parameters.  Rigid given types deliberately do *not* satisfy this predicate:
they are representatives produced by the verification pass, not source
instantiations quantified over by the coverage obligation. -/
partial def Ty.isClosed : Ty → Bool
  | .typeParam _ => false
  | .structInst _ args | .enumInst _ args => args.all Ty.isClosed
  | .vector elem | .ref elem | .mutRef elem => elem.isClosed
  | _ => true

/-- The domain of concrete substitutions represented by a finite
monomorphization plan. -/
def ClosedTypeArgs (args : List Ty) : Prop :=
  args.all Ty.isClosed = true

/-- Resource effects directly represented by an operation. -/
def Oper.tagEffects : Oper → List TagEffect
  | .getGlobal r | .exists_ r | .borrowGlobal r =>
      [⟨.read, r, []⟩]
  | .getGlobalInst r args | .existsInst r args | .borrowGlobalInst r args =>
      [⟨.read, r, args⟩]
  | .writeGlobal r | .moveTo r | .moveFrom r | .mkMutGlobal r =>
      [⟨.write, r, []⟩]
  | .moveToInst r args | .moveFromInst r args =>
      [⟨.write, r, args⟩]
  | _ => []

def Instr.tagEffects : Instr → List TagEffect
  | .call _ op _ => op.tagEffects
  | _ => []

/-- Resource reads appearing in a deep specification expression. -/
def SpecExp.tagEffects : SpecExp → List TagEffect
  | .global r _ address | .exists_ r _ address =>
      ⟨.read, r, []⟩ :: address.tagEffects
  | .binop _ lhs rhs => lhs.tagEffects ++ rhs.tagEffects
  | .not e | .select _ e | .len e | .mutVal e => e.tagEffects
  | .ite c t e => c.tagEffects ++ t.tagEffects ++ e.tagEffects
  | .quant _ _ body => body.tagEffects
  | _ => []

def Contract.tagEffects (contract : Contract) : List TagEffect :=
  contract.requires.tagEffects ++
  contract.aborts.toList.flatMap SpecExp.tagEffects ++
  contract.ensures.tagEffects ++
  contract.modifies.flatMap fun (resource, address) =>
    ⟨.write, resource, []⟩ :: address.tagEffects

def LoopSpec.tagEffects (spec : LoopSpec) : List TagEffect :=
  spec.inv.tagEffects

/-- All statically visible runtime-tag effects of a function. -/
def FunDecl.tagEffects (d : FunDecl) : List TagEffect :=
  let blockEffects := (List.range d.body.size).flatMap fun blockId =>
    match d.body.blocks blockId with
    | some block => block.instrs.flatMap Instr.tagEffects
    | none => []
  let loopEffects := (List.range d.body.size).flatMap fun blockId =>
    match d.loopSpecs blockId with
    | some spec => spec.tagEffects
    | none => []
  blockEffects ++ loopEffects ++ d.contract.tagEffects

/-- Function calls directly represented by an operation. -/
def Oper.calledInstance? : Oper → Option MonoKey
  | .function f => some ⟨f, []⟩
  | .functionInst f args => some ⟨f, args⟩
  | _ => none

def FunDecl.calledInstances (d : FunDecl) : List MonoKey :=
  (List.range d.body.size).flatMap fun blockId =>
    match d.body.blocks blockId with
    | some block => block.instrs.filterMap fun instr =>
        match instr with
        | .call _ op _ => op.calledInstance?
        | _ => none
    | none => []

/-- The concrete runtime storage key denoted by an effect under one function
type instantiation. -/
def TagEffect.resourceKeyAt (typeArgs : List Ty) (effect : TagEffect) : ResourceKey :=
  resourceKey effect.resource (instantiateTypes typeArgs effect.typeArgs)

/-- Two function instantiations expose the same observable equality pattern
between all runtime resource tags used by the function.  This—not syntactic
equality of the type arguments—is the semantic quotient behind verification
monomorphization. -/
def SameTagInteractions (effects : List TagEffect)
    (lhsArgs rhsArgs : List Ty) : Prop :=
  ∀ lhs ∈ effects, ∀ rhs ∈ effects,
    (lhs.resourceKeyAt lhsArgs = rhs.resourceKeyAt lhsArgs) ↔
      (lhs.resourceKeyAt rhsArgs = rhs.resourceKeyAt rhsArgs)

/-- Coverage obligation for one generic declaration: every concrete type
substitution has a planned representative with the same runtime-tag
interactions. -/
def CoversTagInteractions (funId : FunId) (d : FunDecl)
    (entries : List MonoKey) : Prop :=
  ∀ concreteArgs, ClosedTypeArgs concreteArgs →
    concreteArgs.length = d.typeParams.length →
    ∃ representative ∈ entries,
      representative.funId = funId ∧
      SameTagInteractions d.tagEffects concreteArgs representative.typeArgs

/-- Proof-facing certificate for a computed plan.  The executable discovery
pass is deliberately separate: the eventual end-to-end theorem consumes this
certificate, so the finite-instance coverage argument cannot become an
untracked trusted assumption. -/
structure MonoPlan.Certificate (m : Module) (plan : MonoPlan) : Prop where
  tagCoverage : ∀ f d, m.program.funs f = some d →
    CoversTagInteractions f d plan.entries
  callClosure : ∀ caller ∈ plan.entries, ∀ d,
    m.program.funs caller.funId = some d → ∀ call ∈ d.calledInstances,
    ∃ callee ∈ plan.entries,
      callee.RuntimeEq
        ⟨call.funId, instantiateTypes caller.typeArgs call.typeArgs⟩

/-- Look up a declaration within the finite function range of a module. -/
def Module.decl? (m : Module) (f : FunId) : Option FunDecl :=
  if f < m.numFuns then m.program.funs f else none

/-- Locally discovered given-type and tag-collision roots. -/
def Module.localMonoKeys (m : Module) : List MonoKey :=
  (List.range m.numFuns).flatMap fun f =>
    match m.decl? f with
    | none => []
    | some d =>
        (discoverCollisionArgs d.typeParams.length d.tagEffects).map fun args =>
          ⟨f, rigidifyTypeArgs f args⟩

/-- Calls made by one specialized instance after substituting its type arguments. -/
def Module.instantiatedCalls (m : Module) (key : MonoKey) : List MonoKey :=
  match m.decl? key.funId with
  | none => []
  | some d =>
      d.calledInstances.map fun call =>
        ⟨call.funId, instantiateTypes key.typeArgs call.typeArgs⟩

/-- Check the finite, executable part of a monomorphization certificate.
Every locally discovered given/collision case must be present, every entry
must have the declaration's arity, and instantiated calls must be closed in
the plan.  `MonoPlan.Certificate.tagCoverage` remains the proof-facing
semantic theorem connecting these syntactic checks to all closed types. -/
def MonoPlan.validate (m : Module) (plan : MonoPlan) : Except String Unit := do
  if plan.entries.length > maxMonoInstances then
    throw s!"monomorphization exceeds {maxMonoInstances} instances"
  unless (dedup plan.entries == plan.entries) do
    throw "monomorphization plan contains duplicate entries"
  for seed in m.localMonoKeys do
    unless beqMem seed plan.entries do
      throw s!"monomorphization plan omits local collision case for function {seed.funId}"
  for key in plan.entries do
    let declaration ← match m.decl? key.funId with
      | some declaration => pure declaration
      | none => throw s!"monomorphization references missing function {key.funId}"
    unless key.typeArgs.length = declaration.typeParams.length do
      throw s!"function {key.funId} expects {declaration.typeParams.length} type arguments, got {key.typeArgs.length}"
    for call in m.instantiatedCalls key do
      unless beqMem call plan.entries do
        throw s!"monomorphization plan is not call-closed at function {key.funId}"

private partial def discoverMonoPlan.go (m : Module)
    (known work : List MonoKey) : Except String MonoPlan := do
  if known.length > maxMonoInstances then
    throw s!"monomorphization exceeds {maxMonoInstances} instances; check for a cyclic type instantiation"
  match work with
  | [] => pure ⟨known⟩
  | key :: pending =>
      let declaration ← match m.decl? key.funId with
        | some declaration => pure declaration
        | none => throw s!"monomorphization references missing function {key.funId}"
      unless key.typeArgs.length = declaration.typeParams.length do
        throw s!"function {key.funId} expects {declaration.typeParams.length} type arguments, got {key.typeArgs.length}"
      let calls := (m.instantiatedCalls key).filter fun call => !beqMem call known
      go m (known ++ calls) (pending ++ calls)

/-- Compute the finite given-type/collision/call-closure plan for a module. -/
def discoverMonoPlan (m : Module) : Except String MonoPlan :=
  let seeds := dedup m.localMonoKeys
  do
    let plan ← discoverMonoPlan.go m seeds seeds
    plan.validate m
    pure plan

def MonoPlan.generatedFunId? (plan : MonoPlan) (key : MonoKey) : Option FunId :=
  plan.entries.findIdx? fun entry => entry == key

/-- Rewrite a generic call target to its generated monomorphic function id. -/
def MonoPlan.rewriteOper (plan : MonoPlan) : Oper → Oper
  | .function f =>
      match plan.generatedFunId? ⟨f, []⟩ with
      | some generated => .function generated
      | none => .function f
  | .functionInst f args =>
      match plan.generatedFunId? ⟨f, args⟩ with
      | some generated => .function generated
      | none => .functionInst f args
  | op => op

/-- Rewrite call targets in one instruction. -/
def MonoPlan.rewriteInstr (plan : MonoPlan) : Instr → Instr
  | .call dsts op srcs => .call dsts (plan.rewriteOper op) srcs
  | instr => instr

/-- Rewrite every call target in a specialized CFG. -/
def MonoPlan.rewriteCfg (plan : MonoPlan) (cfg : Cfg) : Cfg :=
  { cfg with blocks := fun blockId =>
      (cfg.blocks blockId).map fun block =>
        { block with instrs := block.instrs.map plan.rewriteInstr } }

def Instr.hasInstantiatedCall : Instr → Bool
  | .call _ (.functionInst _ _) _ => true
  | _ => false

/-- Check the invariant expected by the monomorphic IVL boundary. -/
def Module.hasNoInstantiatedCalls (m : Module) : Bool :=
  (List.range m.numFuns).all fun f =>
    match m.program.funs f with
    | none => false
    | some d =>
        (List.range d.body.size).all fun b =>
          match d.body.blocks b with
          | none => false
          | some block => !block.instrs.any Instr.hasInstantiatedCall

/-- Instantiate one source declaration and rewrite its calls through a plan. -/
def Module.specializeFun? (m : Module) (plan : MonoPlan)
    (key : MonoKey) : Option FunDecl := do
  let source ← m.decl? key.funId
  let instantiated := source.instantiate key.typeArgs
  pure { instantiated with body := plan.rewriteCfg instantiated.body }

/-- Materialize a plan as a finite module.  Generated function ids are the
positions of `MonoPlan.entries`; metadata retains the source visibility and
entry status. -/
def Module.monomorphize (m : Module) (plan : MonoPlan) : Module where
  address := m.address
  name := m.name
  program := {
    structs := m.program.structs
    funs := fun generated => do
      let key ← plan.entries[generated]?
      m.specializeFun? plan key
  }
  numStructs := m.numStructs
  numFuns := plan.entries.length
  structMeta := m.structMeta
  funMeta := fun generated => do
    let key ← plan.entries[generated]?
    let info ← m.funMeta key.funId
    pure { info with name := s!"{info.name}$mono{generated}" }
  dialect := m.dialect

/-- Discover and materialize the monomorphic verification module. -/
def Module.monomorphizeForVerification (m : Module) : Except String (MonoPlan × Module) := do
  let plan ← discoverMonoPlan m
  let specialized := m.monomorphize plan
  unless specialized.hasNoInstantiatedCalls do
    throw "monomorphization left an unresolved generic function call"
  pure (plan, specialized)

/-- Every generated declaration is binder-free by construction. -/
theorem specializeFun_typeParams_empty {m : Module} {plan : MonoPlan}
    {key : MonoKey} {d : FunDecl}
    (h : m.specializeFun? plan key = some d) : d.typeParams = [] := by
  unfold Module.specializeFun? at h
  cases hsource : m.decl? key.funId with
  | none => simp [hsource] at h
  | some source =>
      simp [hsource] at h
      rw [← h]
      rfl

end MoveModel.IR
