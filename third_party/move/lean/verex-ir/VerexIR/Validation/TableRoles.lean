-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Validation.Validated

/-!
# Table owners and their roles

An intrinsic map whose owner holds a handle is a table: the runtime keeps
its entries in native storage and provides the roles its module binds, not
running their bodies (`designs/intrinsic-maps.md`, "Tables in the runtime
semantics"). This module names the owners (`tableLayout?`) and the role
functions (`tableRoleOf?`), which preparation, the runtime, and the
denotation's hidden contents field share.
-/

namespace VerexIR.Validation

/-- The layout of a table owner: its declaration, whether it holds a handle,
and whether it caches its length (`designs/intrinsic-maps.md`, "Views"). -/
structure TableLayout where
  namespaceId : NamespaceId
  structId : Nat
  handle : Bool
  length : Bool
  deriving Repr, Inhabited

/-- The table layout of a declaration: an intrinsic map owner with a key and
a value parameter, no variants, and fields that are none, a handle, or a
handle and a cached length. -/
def tableLayout? (unit : ValidatedUnit) (namespaceId : NamespaceId) (structId : Nat) :
    Option TableLayout := do
  let ns ← unit.namespaces[namespaceId.index]?
  let declaration ← ns.structs[structId]?
  guard (ns.intrinsics.any fun intrinsic =>
    intrinsic.model == "map" && intrinsic.owner == declaration.name)
  guard (declaration.variants.isEmpty && declaration.generics.size == 2)
  let fieldTypes : List Ty ← declaration.fields.toList.mapM fun field =>
    unit.tables.types[field.type.typeId.index]?
  let (handle, length) ← match fieldTypes with
    | [] => some (false, false)
    | [Ty.address] => some (true, false)
    | [Ty.address, Ty.integer (.bits 64) false] => some (true, true)
    | _ => none
  some ⟨namespaceId, structId, handle, length⟩

/-- The executable map role a function plays for a table owner with a handle
of its namespace, with the owner's layout: a function the runtime provides. -/
def tableRoleOf? (unit : ValidatedUnit) (namespaceId : NamespaceId) (functionId : FunctionId) :
    Option (String × TableLayout) := do
  let ns ← unit.namespaces[namespaceId.index]?
  let declaration ← ns.functions[functionId.index]?
  let target : QualifiedRef := { namespaceId, name := declaration.name }
  ns.intrinsics.findSome? fun intrinsic => do
    guard (intrinsic.model == "map")
    let binding ← intrinsic.executableBindings.find? (·.target == target)
    let structId ← ns.structs.findIdx? (·.name == intrinsic.owner)
    let layout ← tableLayout? unit namespaceId structId
    guard layout.handle
    some (binding.role, layout)

/-- The functions the unit's runs call directly from a body the runtime
runs: every body but a table role's, which the runtime provides. A native
no such body calls, the one behind a role, is never run and needs no
implementation. -/
def calledFromRuns (unit : ValidatedUnit) : Array (Nat × Nat) := Id.run do
  let mut called : Array (Nat × Nat) := #[]
  for ns in unit.namespaces, nsIndex in [0:unit.namespaces.size] do
    for function in ns.functions, functionIndex in [0:ns.functions.size] do
      let .structured root := function.body | continue
      if (tableRoleOf? unit ⟨nsIndex⟩ ⟨functionIndex⟩).isSome then continue
      let mut work := [root]
      let mut visited : Array Nat := #[]
      for _ in [0:ns.expressions.size + 1] do
        match work with
        | [] => break
        | id :: rest =>
            work := rest
            if visited.contains id.index then continue
            visited := visited.push id.index
            let some expression := ns.expressions[id.index]? | continue
            if let .operation (.call (.function reference)) _ _ _ := expression.kind then
              if let some functionId := unit.resolution.function? reference.name then
                called := called.push (reference.namespaceId.index, functionId.index)
            work := (expressionChildren expression.kind).toList ++ work
  return called

end VerexIR.Validation
