-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Validation.Check
import VerexIR.Semantics.ValueTyping
import VerexIR.Semantics.Operations

namespace VerexIR.Tests.SliceValidation
open VerexIR VerexIR.Import VerexIR.Validation
private def schema : ProfileSchema := { profile := .move, name := "slice-test" }
private def fixture (actual : Nat) : RawUnit where
  tables := {
    locations := #[{}]
    names := #[{ namespaceId := ⟨0⟩, name := "Holder" }]
    namespaces := #[{ segments := #["SliceTest"] }]
    lifetimes := #[{ loc := ⟨0⟩, kind := .static }]
    types := #[.bool, .slice ⟨0⟩,
      .reference { profile := .move, kind := .shared, referent := ⟨1⟩, lifetime := ⟨0⟩ },
      .nominal ⟨0⟩ #[.typeArg {typeId := ⟨actual⟩,loc := ⟨0⟩}],
      .reference { profile := .move, kind := .mutable, referent := ⟨1⟩, lifetime := ⟨0⟩ },
      .unit, .vector ⟨0⟩ none, .vector ⟨0⟩ (some (.integer 2))] }
  profiles := #[{ profile := .move, name := "slice-test" }]
  namespaces := #[{
    loc := ⟨0⟩
    identity := ⟨0⟩
    profile := some .move
    structs := #[{
      loc := ⟨0⟩
      name := ⟨0⟩
      generics := #[{loc := ⟨0⟩,name := "T",kind := .typeArg}]
    }]
  }]

private def unsizedError (unit : RawUnit) : Bool :=
  match validate #[schema] unit with
  | .ok _ => false
  | .error diagnostics => diagnostics.any (fun (diagnostic : Diagnostic) =>
      diagnostic.code == "VIR-TYPE-UNSIZED")

-- Sized generic binders reject Slice itself, but accept a slice reference.
#guard unsizedError (fixture 1)
#guard (validate #[schema] (fixture 2)).toOption.isSome
#guard unsizedError { fixture 2 with tables := { (fixture 2).tables with
    types := (fixture 2).tables.types.push (.vector ⟨1⟩ none) } }
#guard unsizedError { fixture 2 with tables := { (fixture 2).tables with
    types := (fixture 2).tables.types.push (.tuple #[⟨1⟩]) } }
#guard !(Ty.vector ⟨0⟩ none == Ty.slice ⟨0⟩)
#guard !(SemTy.vector .bool none == SemTy.slice .bool)
#guard !SemTy.sized (.slice .bool)
#guard SemTy.sized (.reference .mutable (.slice .bool))

/-- Exercise the semantic checker directly with typed operand placeholders.
The deliberately unsized local tests rejection independently of raw-local
validation, so generic substitution cannot bypass the semantic rule. -/
private def operationAccepted (operation : Operation) (result : SemTy)
    (operandTypes : Array TypeId := #[]) : Bool := (do
  let unit ← (validate #[schema] (fixture 2)).toOption
  let ns : ValidatedNamespace ← unit.namespaces[0]?
  let ns : ValidatedNamespace := { ns with
    places := #[.localVar ⟨0⟩]
    expressions := operandTypes.map fun type =>
      { loc := ⟨0⟩, typeId := type, kind := .value .unit } }
  let context : StaticTyping.Context := {
    unit, pointerWidth := none, ns
    locals := #[{
      id := ⟨0⟩
      name := "slice"
      loc := ⟨0⟩
      type := { typeId := ⟨1⟩, loc := ⟨0⟩ }
      mutable := true }]
    env := #[], results := none, loops := [], required := none, table := {} }
  pure (context.checkOperation result operation #[]
    ((Array.range operandTypes.size).map (fun index => ⟨index⟩)))).getD false

#guard !operationAccepted (.move ⟨0⟩) (.slice .bool)
#guard !operationAccepted (.copy ⟨0⟩) (.slice .bool)
#guard !operationAccepted (.write ⟨0⟩) .unit #[⟨1⟩]
#guard !operationAccepted (.reference .mutate) .unit #[⟨4⟩, ⟨1⟩]
#guard operationAccepted (.read ⟨0⟩) (.slice .bool)
#guard operationAccepted (.borrow .mutable ⟨0⟩) (.reference .mutable (.slice .bool))
#guard !StaticTyping.primitiveTyped none .pushVector [.slice .bool, .bool] (.slice .bool)
#guard !StaticTyping.primitiveTyped none .moveValue [.slice .bool] (.slice .bool)
#guard StaticTyping.Context.sharedBorrowResult (.vector .bool (some (.integer 2)))
  (.reference .shared (.slice .bool))

-- The storage invariant excludes both unconstrained Slice loans and exposing
-- fixed slice storage as a growable vector reference.
example : ¬ BorrowView (.slice .bool) (.slice .bool) := by
  intro view
  cases view with
  | same _ sized => cases sized

example : ¬ BorrowView (.vector .bool none) (.slice .bool) := by
  intro view
  cases view

example : ¬ BorrowView (.vector .bool (some (.integer 2))) (.vector .bool none) := by
  intro view
  cases view

example {storage : SemTy} (view : BorrowView storage (.slice .bool)) :
    ∃ extent : Nat, storage = .vector .bool (some (.integer extent)) := by
  cases view with
  | same _ sized => cases sized
  | slice _ extent => exact ⟨extent, rfl⟩

#guard (RuntimeValue.borrow 7 (.vector #[.bool true])).holeless
#guard !(RuntimeValue.borrow 7 (.loanHole 8)).holeless

-- Slice formation needs the outer sequence, while ordinary sized reborrows
-- retain loan-hole chains. Live references inside a sequence remain valid.
#guard SemanticOperations.borrowStorageShape .bool (.loanHole 8)
#guard !SemanticOperations.borrowStorageShape (.slice ⟨0⟩) (.loanHole 8)
#guard SemanticOperations.borrowStorageShape (.slice ⟨0⟩)
  (.vector #[.borrow 7 (.bool true)])
#guard SemanticOperations.borrowStorageShape (.slice ⟨0⟩) (.vector #[])

end VerexIR.Tests.SliceValidation
