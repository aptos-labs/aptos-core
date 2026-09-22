-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR

/-!
# Canonical LeanerLang builtin knowledge

This module contains the small amount of source-level signature knowledge
needed for names which are part of the Move standard surface.  In particular,
it lets the source backend omit a type argument exactly when the ordinary
argument types determine it, and lets lowering reconstruct that same LIR
instantiation.

This is deliberately a signature table, not a spelling heuristic: unknown
external functions retain explicit type arguments until dependency interfaces
carry their authoritative signatures.
-/

namespace LeanerLang

open LeanerIR

private def isOneAddress (value : String) : Bool :=
  if !value.startsWith "0x" then false else
    List.dropWhile (· == '0') (value.toList.drop 2) == ['1']

def isMoveStdModule (profile : Option Profile)
    (namespaceRef : NamespaceRef) (moduleName : String) : Bool :=
  profile == some .move && namespaceRef.segments.size >= 2 &&
    namespaceRef.segments.size <= 3 &&
    namespaceRef.segments[0]?.any isOneAddress &&
    (namespaceRef.segments.size == 2 ||
      namespaceRef.segments[1]? == some "std" ||
      namespaceRef.segments[1]? == some "") &&
    namespaceRef.segments.back? == some moduleName

private partial def vectorElementType? (tables : Tables) (typeId : TypeId)
    (fuel : Nat) : Option TypeId := do
  guard (fuel > 0)
  match tables.types[typeId.index]? with
  | some (.reference reference) =>
      vectorElementType? tables reference.referent (fuel - 1)
  | some (.vector element none) => some element
  | _ => none

private def referenceReferent? (tables : Tables) (typeId : TypeId) : Option TypeId := do
  let .reference reference ← tables.types[typeId.index]? | none
  some reference.referent

/-- How a known standard function receives its first `self` argument.  This
is shared by source recovery and lowering so receiver notation never relies on
a printer-only spelling guess while dependency interfaces remain incomplete. -/
inductive StandardReceiverMode where
  | byValue
  | shared
  | mutable
  deriving Repr, BEq

/-- Receiver mode for the Move standard vector surface.  The inventory is the
set of functions whose first source parameter is named `self`; free functions
such as `empty`, `singleton`, `move_range`, and `range` deliberately return
`none`. -/
def standardCallReceiverMode? (profile : Option Profile)
    (namespaceRef : NamespaceRef) (functionName : String) :
    Option StandardReceiverMode :=
  if !isMoveStdModule profile namespaceRef "vector" then none
  else if functionName ∈ [
      "length", "borrow", "last", "is_empty", "contains", "index_of"] then
    some .shared
  else if functionName ∈ [
      "push_back", "borrow_mut", "pop_back", "swap", "last_mut", "reverse",
      "reverse_slice", "reverse_append", "append", "trim", "trim_reverse",
      "insert", "remove", "remove_value", "swap_remove", "replace", "rotate",
      "rotate_slice"] then
    some .mutable
  else if functionName == "destroy_empty" then some .byValue
  else none

/-- Infer the complete type-instantiation list of a known Move standard
function from its value argument types. `none` means that the signature is
unknown or at least one type parameter does not occur in an argument type. -/
def standardCallInferredTypes? (tables : Tables) (profile : Option Profile)
    (namespaceRef : NamespaceRef) (functionName : String)
    (argumentTypes : Array TypeId) : Option (Array TypeId) := do
  if isMoveStdModule profile namespaceRef "vector" then
    if functionName == "singleton" then
      let typeId ← argumentTypes[0]?
      return #[typeId]
    if (standardCallReceiverMode? profile namespaceRef functionName).isSome then
      let container ← argumentTypes[0]?
      let element ← vectorElementType? tables container (tables.types.size + 1)
      return #[element]
    none
  else if isMoveStdModule profile namespaceRef "mem" && functionName == "replace" then
    let reference ← argumentTypes[0]?
    let referent ← referenceReferent? tables reference
    return #[referent]
  else if isMoveStdModule profile namespaceRef "option" && functionName == "some" then
    let typeId ← argumentTypes[0]?
    return #[typeId]
  else if isMoveStdModule profile namespaceRef "bcs" && functionName == "to_bytes" then
    let reference ← argumentTypes[0]?
    let referent ← referenceReferent? tables reference
    return #[referent]
  else
    none

end LeanerLang
