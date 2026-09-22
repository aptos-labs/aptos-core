-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Syntax
import LeanerMove.Frontend.Xast

/-!
# Move names at the LIR/backend boundary

One checked conversion from compilation-unit name tables to the transitional
Move printer's module and qualified-name structures.
-/

namespace LeanerMove.Frontend.LIR.MoveNames

private def requireSome (value : Option α) (message : String) : Except String α :=
  match value with | some value => .ok value | none => .error message

def resolvedName? (tables : LeanerIR.Tables) (reference : LeanerIR.QualifiedRef) :
    Option (LeanerIR.NamespaceRef × String) := do
  let name ← tables.names[reference.name.index]?
  if name.namespaceId != reference.namespaceId then none else
    let namespaceRef ← tables.namespaces[reference.namespaceId.index]?
    some (namespaceRef, name.name)

def moduleRef (tables : LeanerIR.Tables) (id : LeanerIR.NamespaceId) :
    Except String LeanerMove.Frontend.Xast.ModuleRef := do
  let reference ← requireSome tables.namespaces[id.index]?
    s!"invalid namespace reference {id.index}"
  match reference.segments with
  | #[address, alias, name] =>
      return { address, addressAlias := if alias.isEmpty then none else some alias, name }
  | #[address, name] => return { address, addressAlias := none, name }
  | _ => throw "Move namespace reference does not have address/name or address/alias/name segments"

def qualifiedName (tables : LeanerIR.Tables) (id : LeanerIR.NameId) :
    Except String LeanerMove.Frontend.Xast.QualifiedName := do
  let name ← requireSome tables.names[id.index]? s!"invalid qualified name {id.index}"
  return { module := ← moduleRef tables name.namespaceId, name := name.name }

def qualifiedRef (tables : LeanerIR.Tables) (reference : LeanerIR.QualifiedRef) :
    Except String LeanerMove.Frontend.Xast.QualifiedName := do
  let name ← requireSome tables.names[reference.name.index]?
    s!"invalid qualified name {reference.name.index}"
  unless name.namespaceId == reference.namespaceId do
    throw "qualified reference namespace disagrees with interned name"
  return { module := ← moduleRef tables reference.namespaceId, name := name.name }

end LeanerMove.Frontend.LIR.MoveNames
