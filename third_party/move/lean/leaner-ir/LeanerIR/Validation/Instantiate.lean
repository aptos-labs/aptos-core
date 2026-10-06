-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Validation.Validated

/-!
# Instantiation in the interned type arena

A generic declaration's types are instantiated by locating the structurally
instantiated node the arena already holds: the runtime's frames, global
keys, and closures name concrete types this way.
-/

namespace LeanerIR.Validation

def instantiateLifetime? (ns : ValidatedNamespace)
    (instantiations : Array GenericArgument) (lifetime : LifetimeId) : Option LifetimeId := do
  let declaration ← ns.tables.lifetimes[lifetime.index]?
  match declaration.kind with
  | .parameter index => match instantiations[index]? with
      | some (.lifetime value) => some value
      | _ => none
  | .static | .inference | .local => some lifetime

theorem instantiateLifetime?_eraseLoc (ns : ValidatedNamespace)
    (instantiations : Array GenericArgument) (lifetime : LifetimeId) :
    instantiateLifetime? ns (instantiations.map GenericArgument.eraseLoc) lifetime =
      instantiateLifetime? ns instantiations lifetime := by
  simp only [instantiateLifetime?, Array.getElem?_map]
  cases ns.tables.lifetimes[lifetime.index]? with
  | none => rfl
  | some declaration =>
      simp only [Option.bind_eq_bind, Option.bind_some]
      cases declaration.kind with
      | parameter index =>
          dsimp only
          cases instantiations[index]? with
          | none => rfl
          | some argument => cases argument <;> rfl
      | _ => rfl

/-- Source locations distinguish occurrences, not instantiated nominal
types.  Structural lookup in the shared type arena therefore compares a
type argument by its `TypeId` while retaining ordinary equality for the
other generic-argument kinds. -/
def sameGenericArgumentValue : GenericArgument → GenericArgument → Bool
  | .typeArg left, .typeArg right => left.typeId == right.typeId
  | .const left, .const right => left == right
  | .lifetime left, .lifetime right => left == right
  | .evidence left, .evidence right => left == right
  | _, _ => false

def sameGenericArgumentValues
    (left right : Array GenericArgument) : Bool :=
  left.size == right.size &&
    (left.zip right).all fun (left, right) => sameGenericArgumentValue left right

/-- Resolve a declaration-local generic field type to an already interned
concrete arena type. RawUnit remains non-monomorphized: this only locates the
structurally instantiated node emitted for the use site. -/
def instantiatePlaceFieldTypeFuel? (ns : ValidatedNamespace)
    (instantiations : Array GenericArgument) : Nat → TypeId → Option TypeId
  | 0, _ => none
  | fuel + 1, typeId => do
      let type ← ns.tables.types[typeId.index]?
      match type with
      | .typeParameter index => match instantiations[index]? with
          | some (.typeArg value) => some value.typeId
          | _ => none
      | .tuple elements => do
          let instantiated ← elements.mapM
            (instantiatePlaceFieldTypeFuel? ns instantiations fuel)
          let index ← ns.tables.types.findIdx? (fun candidate => candidate == .tuple instantiated)
          some ⟨index⟩
      | .vector element length => do
          let element ← instantiatePlaceFieldTypeFuel? ns instantiations fuel element
          let index ← ns.tables.types.findIdx? fun candidate =>
            candidate == .vector element length
          some ⟨index⟩
      | .typeDomain nested => do
          let nested ← instantiatePlaceFieldTypeFuel? ns instantiations fuel nested
          let index ← ns.tables.types.findIdx? (fun candidate => candidate == .typeDomain nested)
          some ⟨index⟩
      | .resourceDomain resource arguments => do
          let arguments ← arguments.mapM fun arguments =>
            arguments.mapM (instantiatePlaceFieldTypeFuel? ns instantiations fuel)
          let index ← ns.tables.types.findIdx? fun candidate =>
            candidate == .resourceDomain resource arguments
          some ⟨index⟩
      | .nominal name arguments => do
          let arguments ← arguments.mapM fun argument => match argument with
            | .typeArg value => do
                let typeId ← instantiatePlaceFieldTypeFuel? ns instantiations fuel value.typeId
                some (.typeArg { value with typeId })
            | .lifetime value => .lifetime <$> instantiateLifetime? ns instantiations value
            | .const value => some (.const value)
            | .evidence value => some (.evidence value)
          let index ← ns.tables.types.findIdx? fun candidate => match candidate with
            | .nominal candidateName candidateArguments =>
                candidateName == name &&
                  sameGenericArgumentValues candidateArguments arguments
            | _ => false
          some ⟨index⟩
      | .function arguments result abilities => do
          let arguments ← arguments.mapM
            (instantiatePlaceFieldTypeFuel? ns instantiations fuel)
          let result ← instantiatePlaceFieldTypeFuel? ns instantiations fuel result
          let index ← ns.tables.types.findIdx? fun candidate =>
            candidate == .function arguments result abilities
          some ⟨index⟩
      | .reference reference => do
          let referent ← instantiatePlaceFieldTypeFuel? ns instantiations fuel
            reference.referent
          let lifetime ← instantiateLifetime? ns instantiations reference.lifetime
          let instantiated := .reference { reference with referent, lifetime }
          let index ← ns.tables.types.findIdx? (fun candidate => candidate == instantiated)
          some ⟨index⟩
      | _ => some typeId

/-- An instantiation reads its type arguments' identities, not where they
occur. -/
theorem instantiatePlaceFieldTypeFuel?_eraseLoc (ns : ValidatedNamespace)
    (instantiations : Array GenericArgument) (fuel : Nat) :
    instantiatePlaceFieldTypeFuel? ns (instantiations.map GenericArgument.eraseLoc) fuel =
      instantiatePlaceFieldTypeFuel? ns instantiations fuel := by
  induction fuel with
  | zero => rfl
  | succ fuel ih =>
      funext typeId
      simp only [instantiatePlaceFieldTypeFuel?, ih, instantiateLifetime?_eraseLoc]
      cases ns.tables.types[typeId.index]? with
      | none => rfl
      | some type =>
          simp only [Option.bind_eq_bind, Option.bind_some, Array.getElem?_map]
          cases type with
          | typeParameter index =>
              dsimp only
              cases instantiations[index]? with
              | none => rfl
              | some argument => cases argument <;> rfl
          | _ => rfl

/-- Instantiate a declaration-local field type using the generic arguments of
its nominal use, locating the already interned concrete type in the arena. -/
def instantiatePlaceFieldType? (ns : ValidatedNamespace)
    (instantiations : Array GenericArgument) (typeId : TypeId) : Option TypeId :=
  instantiatePlaceFieldTypeFuel? ns instantiations (ns.tables.types.size + 1) typeId

end LeanerIR.Validation
