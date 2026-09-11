-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Validation.Validated

/-!
# LIR type unification

One matching engine decides whether ground call-site types fit a generic
declaration signature. Explicitly supplied generic arguments enter the
`Solution` as pre-bound slots and are checked; elided arguments enter as
unbound slots and are solved by two-sided matching against the argument and
result occurrences of the corresponding binder. With every slot bound the
engine degenerates to instantiation checking, so the semantic scan's explicit
and inferred paths share exactly one comparison semantics.

The logical mode mirrors the specification projection: references erase and
every integer width lives in the unbounded integer domain. Slots never bind
through a lossy logical relaxation — a projected occurrence can confirm a
binding but not create one — so inferred instantiations are always ground
types the frontend authored.
-/

namespace LeanerIR.Validation

/-- Resolve an interned name to its owning namespace reference and string. -/
def resolvedName? (ns : ValidatedNamespace) (nameId : NameId) :
    Option (NamespaceRef × String) := do
  let name ← ns.tables.names[nameId.index]?
  let namespaceRef ← ns.tables.namespaces[name.namespaceId.index]?
  some (namespaceRef, name.name)

/-- Fuel-bounded comparison protects semantic preparation even if an invalid
unit somehow bypasses the structural type-cycle check. -/
def typeIdsEquivalentFuel : Nat → ValidatedNamespace → TypeId →
    ValidatedNamespace → TypeId → Bool
  | 0, _, _, _, _ => false
  | fuel + 1, leftNs, left, rightNs, right =>
  match leftNs.tables.types[left.index]?, rightNs.tables.types[right.index]? with
  | some .unit, some .unit | some .never, some .never | some .bool, some .bool |
      some .character, some .character | some .string, some .string |
      some .bytes, some .bytes | some .address, some .address |
      some .signer, some .signer |
      some .range, some .range | some .eventStore, some .eventStore |
      some .stateDomain, some .stateDomain => true
  | some (.integer leftWidth leftSigned), some (.integer rightWidth rightSigned) =>
      leftWidth == rightWidth && leftSigned == rightSigned
  | some (.tuple leftElements), some (.tuple rightElements) =>
      leftElements.size == rightElements.size &&
        (leftElements.zip rightElements).all fun pair =>
          typeIdsEquivalentFuel fuel leftNs pair.1 rightNs pair.2
  | some (.vector leftElement leftLength), some (.vector rightElement rightLength) =>
      leftLength == rightLength &&
        typeIdsEquivalentFuel fuel leftNs leftElement rightNs rightElement
  | some (.typeDomain leftType), some (.typeDomain rightType) =>
      typeIdsEquivalentFuel fuel leftNs leftType rightNs rightType
  | some (.resourceDomain leftName leftArguments),
      some (.resourceDomain rightName rightArguments) =>
      resolvedName? leftNs leftName == resolvedName? rightNs rightName &&
        match leftArguments, rightArguments with
        | none, none => true
        | some leftTypes, some rightTypes =>
            leftTypes.size == rightTypes.size &&
              (leftTypes.zip rightTypes).all fun pair =>
                typeIdsEquivalentFuel fuel leftNs pair.1 rightNs pair.2
        | _, _ => false
  | some (.nominal leftName leftArguments), some (.nominal rightName rightArguments) =>
      resolvedName? leftNs leftName == resolvedName? rightNs rightName &&
        leftArguments.size == rightArguments.size &&
        (leftArguments.zip rightArguments).all fun pair => match pair with
          | (.typeArg left, .typeArg right) =>
              typeIdsEquivalentFuel fuel leftNs left.typeId rightNs right.typeId
          | (.const left, .const right) => left == right
          | (.lifetime left, .lifetime right) =>
              match leftNs.tables.lifetimes[left.index]?,
                  rightNs.tables.lifetimes[right.index]? with
              | some leftLifetime, some rightLifetime =>
                  leftLifetime.kind == rightLifetime.kind
              | _, _ => false
          | (.evidence left, .evidence right) => left == right
          | _ => false
  | some (.function leftArguments leftResult leftAbilities),
      some (.function rightArguments rightResult rightAbilities) =>
      leftArguments.size == rightArguments.size &&
        (leftArguments.zip rightArguments).all (fun pair =>
          typeIdsEquivalentFuel fuel leftNs pair.1 rightNs pair.2) &&
        typeIdsEquivalentFuel fuel leftNs leftResult rightNs rightResult &&
        leftAbilities == rightAbilities
  | some (.typeParameter leftIndex), some (.typeParameter rightIndex) =>
      leftIndex == rightIndex
  | some (.reference leftReference), some (.reference rightReference) =>
      leftReference.profile == rightReference.profile &&
        leftReference.kind == rightReference.kind &&
        typeIdsEquivalentFuel fuel leftNs leftReference.referent
          rightNs rightReference.referent &&
        match leftNs.tables.lifetimes[leftReference.lifetime.index]?,
            rightNs.tables.lifetimes[rightReference.lifetime.index]? with
        | some leftLifetime, some rightLifetime => leftLifetime.kind == rightLifetime.kind
        | _, _ => false
  | some (.profile leftValue), some (.profile rightValue) => leftValue == rightValue
  | _, _ => false

/-- Compare type structure without applying a declaration instantiation. -/
def typeIdsEquivalent? (leftNs : ValidatedNamespace) (left : TypeId)
    (rightNs : ValidatedNamespace) (right : TypeId) : Bool :=
  typeIdsEquivalentFuel (leftNs.tables.types.size + rightNs.tables.types.size + 1)
    leftNs left rightNs right

def lifetimeKindsEquivalent? (leftNs : ValidatedNamespace) (left : LifetimeId)
    (rightNs : ValidatedNamespace) (right : LifetimeId) : Bool :=
  match leftNs.tables.lifetimes[left.index]?, rightNs.tables.lifetimes[right.index]? with
  | some leftLifetime, some rightLifetime => leftLifetime.kind == rightLifetime.kind
  | _, _ => false

def isAnyIntegerType (ns : ValidatedNamespace) (id : TypeId) : Bool :=
  (ns.tables.types[id.index]?).any fun ty => ty matches .integer ..

namespace Unify

/-- One in-progress instantiation of a declaration's generic binders. A
`none` slot is an unsolved inference variable; matching may bind it exactly
once, and later occurrences of its binder must agree with the binding. -/
structure Solution where
  slots : Array (Option GenericArgument)
  deriving Repr, BEq, Inhabited

/-- Every slot pre-bound by an explicit instantiation: checking mode. -/
def Solution.bound (arguments : Array GenericArgument) : Solution :=
  { slots := arguments.map some }

/-- Every slot open: full inference mode. -/
def Solution.unbound (count : Nat) : Solution :=
  { slots := .replicate count none }

/-- The solved instantiation, or `none` while any slot remains open. -/
def Solution.arguments? (solution : Solution) : Option (Array GenericArgument) :=
  solution.slots.mapM id

/-- Bind or confirm a type slot against a ground source type. -/
private def bindTypeSlot (sourceNs : ValidatedNamespace) (source : TypeId)
    (bindLoc : LocId) (index : Nat) (solution : Solution) : Option Solution :=
  match solution.slots[index]? with
  | some (some (.typeArg instantiated)) =>
      if typeIdsEquivalent? sourceNs source sourceNs instantiated.typeId then some solution
      else none
  | some (some _) => none
  | some none => some { slots :=
      solution.slots.set! index (some (.typeArg { typeId := source, loc := bindLoc })) }
  | none => none

/-- Bind or confirm a lifetime slot. Lifetimes compare by kind: frontends
intern distinct inference lifetimes for equal reference spellings. -/
private def bindLifetimeSlot (sourceNs : ValidatedNamespace) (source : LifetimeId)
    (index : Nat) (solution : Solution) : Option Solution :=
  match solution.slots[index]? with
  | some (some (.lifetime instantiated)) =>
      if lifetimeKindsEquivalent? sourceNs source sourceNs instantiated then some solution
      else none
  | some (some _) => none
  | some none => some { slots := solution.slots.set! index (some (.lifetime source)) }
  | none => none

private def matchLifetime (sourceNs : ValidatedNamespace) (source : LifetimeId)
    (targetNs : ValidatedNamespace) (target : LifetimeId)
    (solution : Solution) : Option Solution :=
  match targetNs.tables.lifetimes[target.index]? with
  | some { kind := .parameter index, .. } =>
      bindLifetimeSlot sourceNs source index solution
  | some _ =>
      if lifetimeKindsEquivalent? sourceNs source targetNs target then some solution
      else none
  | none => none

/-- Match a ground source type against a declaration type whose
`typeParameter`/parameter-lifetime occurrences refer to the solution's
slots. The comparison is structural and does not monomorphize or allocate
new arena nodes. -/
def matchTypeFuel : Nat → (sourceNs : ValidatedNamespace) → TypeId →
    (targetNs : ValidatedNamespace) → TypeId → LocId → Solution → Option Solution
  | 0, _, _, _, _, _, _ => none
  | fuel + 1, sourceNs, source, targetNs, target, bindLoc, solution =>
      match targetNs.tables.types[target.index]? with
      | some (.typeParameter index) =>
          bindTypeSlot sourceNs source bindLoc index solution
      | targetType => match sourceNs.tables.types[source.index]?, targetType with
          | some .unit, some .unit | some .never, some .never |
              some .bool, some .bool | some .character, some .character |
              some .string, some .string | some .bytes, some .bytes |
              some .address, some .address |
              some .signer, some .signer | some .range, some .range |
              some .eventStore, some .eventStore | some .stateDomain, some .stateDomain =>
              some solution
          | some (.integer sourceWidth sourceSigned),
              some (.integer targetWidth targetSigned) =>
              if sourceWidth == targetWidth && sourceSigned == targetSigned then some solution
              else none
          | some (.tuple sourceElements), some (.tuple targetElements) =>
              if sourceElements.size != targetElements.size then none else
              (sourceElements.zip targetElements).foldlM (init := solution) fun solution pair =>
                matchTypeFuel fuel sourceNs pair.1 targetNs pair.2 bindLoc solution
          | some (.vector sourceElement sourceLength),
              some (.vector targetElement targetLength) =>
              if sourceLength != targetLength then none else
              matchTypeFuel fuel sourceNs sourceElement targetNs targetElement bindLoc solution
          | some (.typeDomain sourceType), some (.typeDomain targetType) =>
              matchTypeFuel fuel sourceNs sourceType targetNs targetType bindLoc solution
          | some (.resourceDomain sourceName sourceArguments),
              some (.resourceDomain targetName targetArguments) =>
              if resolvedName? sourceNs sourceName != resolvedName? targetNs targetName then
                none
              else match sourceArguments, targetArguments with
                | none, none => some solution
                | some sourceTypes, some targetTypes =>
                    if sourceTypes.size != targetTypes.size then none else
                    (sourceTypes.zip targetTypes).foldlM (init := solution) fun solution pair =>
                      matchTypeFuel fuel sourceNs pair.1 targetNs pair.2 bindLoc solution
                | _, _ => none
          | some (.nominal sourceName sourceArguments),
              some (.nominal targetName targetArguments) =>
              if resolvedName? sourceNs sourceName != resolvedName? targetNs targetName ||
                  sourceArguments.size != targetArguments.size then none else
              (sourceArguments.zip targetArguments).foldlM (init := solution)
                fun solution pair => match pair with
                  | (.typeArg source, .typeArg target) =>
                      matchTypeFuel fuel sourceNs source.typeId targetNs target.typeId
                        bindLoc solution
                  | (.const source, .const target) =>
                      if source == target then some solution else none
                  | (.lifetime source, .lifetime target) =>
                      matchLifetime sourceNs source targetNs target solution
                  | (.evidence source, .evidence target) =>
                      if source == target then some solution else none
                  | _ => none
          | some (.function sourceArguments sourceResult sourceAbilities),
              some (.function targetArguments targetResult targetAbilities) =>
              if sourceAbilities != targetAbilities ||
                  sourceArguments.size != targetArguments.size then none else do
              let solution ← (sourceArguments.zip targetArguments).foldlM (init := solution)
                fun solution pair =>
                  matchTypeFuel fuel sourceNs pair.1 targetNs pair.2 bindLoc solution
              matchTypeFuel fuel sourceNs sourceResult targetNs targetResult bindLoc solution
          | some (.reference sourceReference), some (.reference targetReference) =>
              if sourceReference.profile != targetReference.profile ||
                  sourceReference.kind != targetReference.kind then none else do
              let solution ← matchTypeFuel fuel sourceNs sourceReference.referent targetNs
                targetReference.referent bindLoc solution
              matchLifetime sourceNs sourceReference.lifetime targetNs
                targetReference.lifetime solution
          | some (.profile source), some (.profile target) =>
              if source == target then some solution else none
          | some (.typeParameter sourceIndex), some (.typeParameter targetIndex) =>
              if sourceIndex == targetIndex then some solution else none
          | _, _ => none

def matchType (sourceNs : ValidatedNamespace) (source : TypeId)
    (targetNs : ValidatedNamespace) (target : TypeId) (bindLoc : LocId)
    (solution : Solution) : Option Solution :=
  matchTypeFuel (sourceNs.tables.types.size + targetNs.tables.types.size + 1)
    sourceNs source targetNs target bindLoc solution

/-- Match through the specification projection: references erase on either
side, and any source integer occupies the unbounded integer domain of any
declared integer or integer-instantiated parameter. Relaxed arms confirm a
solution without binding slots, so inference never records a projected type
as the instantiation. -/
def specMatchFuel : Nat → (sourceNs : ValidatedNamespace) → (logical : Bool) → TypeId →
    (targetNs : ValidatedNamespace) → TypeId → LocId → Solution → Option Solution
  | 0, _, _, _, _, _, _, _ => none
  | fuel + 1, sourceNs, logical, source, targetNs, target, bindLoc, solution =>
    matchTypeFuel fuel sourceNs source targetNs target bindLoc solution <|>
      (if !logical then none else
        -- The projection is shape-sensitive: a tuple is a multi-value
        -- boundary whose components are projected, so matching one descends
        -- into it and applies the relaxed rules to each component.
        (match sourceNs.tables.types[source.index]?,
            targetNs.tables.types[target.index]? with
          | some (.tuple sourceElements), some (.tuple targetElements) =>
              if sourceElements.size != targetElements.size then none else
              (sourceElements.zip targetElements).foldlM (init := solution)
                fun solution pair =>
                  specMatchFuel fuel sourceNs logical pair.1 targetNs pair.2 bindLoc solution
          | _, _ => none) <|>
        (match sourceNs.tables.types[source.index]? with
          | some (.reference reference) =>
              specMatchFuel fuel sourceNs logical reference.referent targetNs target
                bindLoc solution
          | _ => none) <|>
        (match targetNs.tables.types[target.index]? with
          | some (.reference reference) =>
              specMatchFuel fuel sourceNs logical source targetNs reference.referent
                bindLoc solution
          | some (.integer ..) =>
              if isAnyIntegerType sourceNs source then some solution else none
          | some (.typeParameter index) =>
              if isAnyIntegerType sourceNs source &&
                  ((solution.slots[index]?).any fun slot => slot.any fun
                    | .typeArg use =>
                        (sourceNs.tables.types[use.typeId.index]?).any fun ty =>
                          (ty matches .integer ..)
                    | _ => false) then some solution
              else none
          | _ => none))

def specMatch (sourceNs : ValidatedNamespace) (logical : Bool) (source : TypeId)
    (targetNs : ValidatedNamespace) (target : TypeId) (bindLoc : LocId)
    (solution : Solution) : Option Solution :=
  specMatchFuel (sourceNs.tables.types.size + targetNs.tables.types.size + 1)
    sourceNs logical source targetNs target bindLoc solution

/-- Outcome of solving one call site's generic instantiation. -/
inductive CallSolveResult where
  | solved (arguments : Array GenericArgument)
  | mismatch
  | undetermined
  deriving Repr, BEq, Inhabited

/-- Solve a call's elided generic instantiation from its ground occurrence
pairs `(call-site type, declaration type)` — value arguments first, then
results. Matching is order-independent for solvability, but earlier pairs
bind first, so diagnostics attribute inconsistencies to later occurrences. -/
def solveCall (sourceNs : ValidatedNamespace) (logical : Bool)
    (targetNs : ValidatedNamespace) (binders : Array GenericBinder) (bindLoc : LocId)
    (pairs : Array (TypeId × TypeId)) : CallSolveResult :=
  let solved := pairs.foldlM (init := Solution.unbound binders.size) fun solution pair =>
    specMatch sourceNs logical pair.1 targetNs pair.2 bindLoc solution
  match solved with
  | none => .mismatch
  | some solution => match solution.arguments? with
      | some arguments => .solved arguments
      | none => .undetermined

end Unify

end LeanerIR.Validation
