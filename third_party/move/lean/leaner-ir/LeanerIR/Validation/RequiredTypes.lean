-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Validation.StaticTyping

/-!
# The types generic frames require

A generic function's frame at a call consults its types instantiated at the
call's arguments (`requiredTypes`), and the type table must hold each: the
runtime names concrete types by their place in it. A source writes the
instantiations it names; one a callee consults through a further generic
call it does not. A unit is closed under them by interning each
instantiation the table lacks, structurally, as `instantiatePlaceFieldType?`
locates it. Appended types are new identifiers; nothing that names an
existing one changes.
-/

namespace LeanerIR.Validation

open StaticTyping (unitFacts TypeTable)

/-- The index of a type the table holds, by `same`, appending `node` where
it holds none. The table is location-free, so `node` is appended erased. -/
private def internType (types : Array Ty) (node : Ty) (same : Ty → Bool) : Array Ty × TypeId :=
  match types.findIdx? same with
  | some index => (types, ⟨index⟩)
  | none => (types.push node.eraseLocs, ⟨types.size⟩)

/-- A type instantiated at generic arguments, each node interned as
`instantiatePlaceFieldTypeFuel?` locates it, with the table that holds it. -/
private def internInstantiated (lifetimes : Array Lifetime)
    (instantiations : Array GenericArgument) : Nat → Array Ty → TypeId → Option (Array Ty × TypeId)
  | 0, _, _ => none
  | fuel + 1, types, typeId => do
      let instantiateAll (types : Array Ty) (ids : Array TypeId) :
          Option (Array Ty × Array TypeId) :=
        ids.foldlM (init := (types, #[])) fun (types, done) id => do
          let (types, instantiated) ← internInstantiated lifetimes instantiations fuel types id
          pure (types, done.push instantiated)
      let lifetimeOf (lifetime : LifetimeId) : Option LifetimeId := do
        let declaration ← lifetimes[lifetime.index]?
        match declaration.kind with
        | .parameter index => match instantiations[index]? with
            | some (.lifetime value) => some value
            | _ => none
        | .static | .inference | .local => some lifetime
      match ← types[typeId.index]? with
      | .typeParameter index => match instantiations[index]? with
          | some (.typeArg value) => some (types, value.typeId)
          | _ => none
      | .tuple elements => do
          let (types, elements) ← instantiateAll types elements
          let node := Ty.tuple elements
          pure (internType types node (· == node))
      | .vector element length => do
          let (types, element) ← internInstantiated lifetimes instantiations fuel types element
          let node := Ty.vector element length
          pure (internType types node (· == node))
      | .typeDomain nested => do
          let (types, nested) ← internInstantiated lifetimes instantiations fuel types nested
          let node := Ty.typeDomain nested
          pure (internType types node (· == node))
      | .resourceDomain resource arguments => do
          let (types, arguments) ← match arguments with
            | some arguments => do
                let (types, arguments) ← instantiateAll types arguments
                pure (types, some arguments)
            | none => pure (types, none)
          let node := Ty.resourceDomain resource arguments
          pure (internType types node (· == node))
      | .nominal name arguments => do
          let (types, arguments) ← arguments.foldlM (init := (types, #[]))
            fun (types, done) argument => match argument with
              | .typeArg value => do
                  let (types, typeId) ←
                    internInstantiated lifetimes instantiations fuel types value.typeId
                  pure (types, done.push (.typeArg { value with typeId }))
              | .lifetime value => do
                  pure (types, done.push (.lifetime (← lifetimeOf value)))
              | .const value => pure (types, done.push (.const value))
              | .evidence value => pure (types, done.push (.evidence value))
          let node := Ty.nominal name arguments
          pure (internType types node fun candidate => match candidate with
            | .nominal candidateName candidateArguments =>
                candidateName == name && sameGenericArgumentValues candidateArguments arguments
            | _ => false)
      | .function arguments result abilities => do
          let (types, arguments) ← instantiateAll types arguments
          let (types, result) ← internInstantiated lifetimes instantiations fuel types result
          let node := Ty.function arguments result abilities
          pure (internType types node (· == node))
      | .reference reference => do
          let (types, referent) ←
            internInstantiated lifetimes instantiations fuel types reference.referent
          let lifetime ← lifetimeOf reference.lifetime
          let node := Ty.reference { reference with referent, lifetime }
          pure (internType types node (· == node))
      | _ => some (types, typeId)

/-- The unit with every type its generic frames require interned
(`requiredTypes`), iterated to the closure. Where the closure does not
settle within the bound `requiredTypes` iterates, or an instantiation reads
an argument a call does not give, the unit is unchanged and the static
check reports it. -/
def ValidatedUnit.internRequiredTypes (unit : ValidatedUnit) : ValidatedUnit := Id.run do
  let facts := unitFacts unit
  let lifetimes := unit.tables.lifetimes
  let mut types := unit.tables.types
  let mut required : TypeTable := facts.map (·.map (·.consulted))
  let bound := (facts.foldl (· + ·.size) 0) * (unit.tables.types.size + 1) + 1
  let mut closed := false
  for _ in [0:bound] do
    let mut changed := false
    for namespaceFacts in facts, nsIndex in [0:facts.size] do
      for functionFacts in namespaceFacts, functionIndex in [0:namespaceFacts.size] do
        for (target, instantiations) in functionFacts.edges do
          for typeId in required.at target do
            let some (next, instantiated) :=
                internInstantiated lifetimes instantiations (types.size + 1) types typeId
              | return unit
            types := next
            unless (required.at (nsIndex, functionIndex)).contains instantiated do
              required := required.modify nsIndex (·.modify functionIndex (·.push instantiated))
              changed := true
    if !changed then
      closed := true
      break
  if !closed || types.size == unit.tables.types.size then return unit
  let tables := { unit.tables with types }
  Internal.mkValidatedUnit tables unit.profiles (unit.namespaces.map ({ · with tables }))
    unit.dependencies unit.evidence unit.indexes unit.structurizationWitnesses unit.resolution
    unit.initializationCertificates unit.borrowCertificates unit.borrowRejections

end LeanerIR.Validation
