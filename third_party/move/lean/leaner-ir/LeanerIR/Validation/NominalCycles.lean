-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Import.Raw
import LeanerIR.Validation.Diagnostic
import Std.Data.HashMap

namespace LeanerIR.Validation

open LeanerIR.Import

private def children : Ty → Array TypeId
  | .tuple elements => elements
  | .vector element _ | .typeDomain element => #[element]
  | .resourceDomain _ arguments => arguments.getD #[]
  | .nominal _ arguments => arguments.filterMap fun
      | .typeArg value => some value.typeId
      | _ => none
  | .function arguments result _ => arguments.push result
  | .reference reference => #[reference.referent]
  | .unit | .never | .bool | .character | .string | .bytes | .address |
      .signer | .integer .. | .range | .eventStore | .stateDomain |
      .typeParameter _ | .profile _ => #[]

/-- Reject recursive Move nominal definitions, including cycles through
vectors, instantiated arguments, variant fields, and function signatures.
The VM's `struct_defs.rs` follows all of these type dependencies, including
phantom arguments; the type arena's own acyclicity is not sufficient.

Use one graph of interned types and Move declarations, rather than expanding
nominal fields at every occurrence. Iterative sink removal visits each node
and edge once, and shared type subgraphs do not multiply the work. Rust
declarations are not subject to Move's recursive-type prohibition. -/
def checkMoveNominalCycles (unit : RawUnit) : Array Diagnostic := Id.run do
  let mut declarations : Array StructDecl := #[]
  for ns in unit.namespaces do
    if ns.profile == some .move then declarations := declarations ++ ns.structs
  for dependency in unit.dependencies do
    if dependency.profile == some .move then
      declarations := declarations ++ dependency.structs
  if declarations.isEmpty then return #[]
  let typeCount := unit.tables.types.size
  let size := typeCount + declarations.size
  let mut declarationNodes : Std.HashMap Nat Nat := {}
  for (declaration, index) in declarations.zipIdx do
    declarationNodes := declarationNodes.insert declaration.name.index (typeCount + index)
  let mut edges : Array (Array Nat) := Array.replicate size #[]
  for (type, index) in unit.tables.types.zipIdx do
    let mut targets := (children type).filterMap fun child =>
      if child.index < typeCount then some child.index else none
    if let .nominal name _ := type then
      if let some target := declarationNodes[name.index]? then
        targets := targets.push target
    edges := edges.set! index targets
  for (declaration, index) in declarations.zipIdx do
    let fields := declaration.variants.foldl (fun fields variant => fields ++ variant.fields)
      declaration.fields
    edges := edges.set! (typeCount + index) <| fields.filterMap fun field =>
      if field.type.typeId.index < typeCount then some field.type.typeId.index else none
  let mut remaining := edges.map (·.size)
  let mut parents : Array (Array Nat) := Array.replicate size #[]
  let mut ready : Array Nat := #[]
  for (targets, source) in edges.zipIdx do
    if targets.isEmpty then ready := ready.push source
    for target in targets do
      parents := parents.modify target (·.push source)
  let mut cursor := 0
  while cursor < ready.size do
    let node := ready[cursor]!
    cursor := cursor + 1
    for parent in parents[node]! do
      let count := remaining[parent]! - 1
      remaining := remaining.set! parent count
      if count == 0 then ready := ready.push parent
  -- Nodes left over either belong to a cycle or contain a cyclic type.
  -- Report at a nominal declaration, not an unpositioned interned type.
  for (declaration, index) in declarations.zipIdx do
    if remaining[typeCount + index]! != 0 then
      return #[.at "LIR-MOVE-RECURSIVE-TYPE"
        "Move nominal fields contain a recursive type definition" declaration.loc]
  return #[]

end LeanerIR.Validation
