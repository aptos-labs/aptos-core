-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.SimpAttrs
import LeanerIR.Semantics.Operations

/-!
# Global storage in the specification vocabulary

Global memory is a keyed map, and both a program and a contract address it
through the same lookup.  Symbolic execution therefore never has to resolve a
search: the read a `borrow_global` performs *is* the term a clause writes,
and the theorems here are the unfoldings that expose it.

The evaluator matches its instantiation and argument rows against array
literals, whose `match` equations Lean cannot generate.  At a literal row the
match reduces definitionally, so the unfoldings are `rfl` and every
characterization below rewrites with them instead of unfolding the
evaluator.
-/

namespace LeanerIR.Proofs

open LeanerIR.Validation
open SemanticOperations

theorem evaluateGlobalOperation?_contains_unfold
    {unit : ValidatedUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    {site : ExprId} {resource : TypeUse} {key : RuntimeValue}
    {frame : RuntimeFrame} {state : RuntimeState} :
    evaluateGlobalOperation? unit ns resultType site .contains
        #[.typeArg resource] #[key] frame state
      = (do
          let _ ← key.storageKey?
          some (.value frame state
            (.bool (globalExists state ns.identity resource.typeId key)))) :=
  rfl

theorem evaluateGlobalOperation?_borrow_unfold
    {unit : ValidatedUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    {site : ExprId} {resource : TypeUse} {key : RuntimeValue} {kind : BorrowKind}
    {frame : RuntimeFrame} {state : RuntimeState} :
    evaluateGlobalOperation? unit ns resultType site (.borrow kind)
        #[.typeArg resource] #[key] frame state
      = (do
          let _ ← key.storageKey?
          match globalValue? state ns.identity resource.typeId key with
          | none => some (.throw_ frame state .abort)
          | some _ => do
              let .reference referenceType ← ns.tables.types[resultType.index]? | none
              let (frame, state, value) ← borrowRuntimePlace? unit ns site
                referenceType kind frame state
                { root := .global (globalKey ns.identity resource.typeId key) }
              some (.value frame state value)) :=
  rfl

theorem evaluateGlobalOperation?_take_unfold
    {unit : ValidatedUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    {site : ExprId} {resource : TypeUse} {key : RuntimeValue}
    {frame : RuntimeFrame} {state : RuntimeState} :
    evaluateGlobalOperation? unit ns resultType site .take
        #[.typeArg resource] #[key] frame state
      = (do
          let _ ← key.storageKey?
          match globalValue? state ns.identity resource.typeId key with
          | none => some (.throw_ frame state .abort)
          | some value =>
              some (.value frame
                { state with
                  globals :=
                    state.globals.erase (globalKey ns.identity resource.typeId key) }
                value)) :=
  rfl

theorem evaluateGlobalOperation?_publish_unfold
    {unit : ValidatedUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    {site : ExprId} {resource : TypeUse} {key value : RuntimeValue}
    {frame : RuntimeFrame} {state : RuntimeState} :
    evaluateGlobalOperation? unit ns resultType site .publish
        #[.typeArg resource] #[key, value] frame state
      = (do
          let _ ← key.storageKey?
          match globalValue? state ns.identity resource.typeId key with
          | some _ => some (.throw_ frame state .abort)
          | none =>
              some (.value frame
                { state with
                  globals := state.globals.insert
                    (globalKey ns.identity resource.typeId key) value }
                .unit)) :=
  rfl

/-- `ReferenceKind` derives `BEq` without a lawfulness instance, so its
reflexivity is a decision rather than a rewrite. -/
private theorem bne_shared_self :
    (ReferenceKind.shared != ReferenceKind.shared) = false := by decide

/-- A shared borrow of a resource is the keyed lookup: certified exclusivity
makes the observed value the reference, so neither frame nor state moves. -/
theorem evaluateGlobalOperation?_borrowImmutable
    {unit : ValidatedUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    {site : ExprId} {resource : TypeUse} {key : RuntimeValue}
    {frame : RuntimeFrame} {state : RuntimeState} {referenceType : ReferenceType}
    {storageKey : StorageKey}
    (shaped : key.storageKey? = some storageKey)
    (type_eq : ns.tables.types[resultType.index]? = some (.reference referenceType))
    (kind_eq : referenceType.kind = .shared) :
    evaluateGlobalOperation? unit ns resultType site (.borrow .immutable)
        #[.typeArg resource] #[key] frame state
      = some (match globalValue? state ns.identity resource.typeId key with
          | none => .throw_ frame state .abort
          | some value => .value frame state value) := by
  rw [evaluateGlobalOperation?_borrow_unfold]
  cases found : globalValue? state ns.identity resource.typeId key with
  | none => simp [shaped, found]
  | some published =>
      simp only [globalValue?] at found
      simp [shaped, type_eq, borrowRuntimePlace?, kind_eq, readRuntimePlace?,
        readRoot?, readProjections?, bne_shared_self, found]

end LeanerIR.Proofs
