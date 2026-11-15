-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Semantics.TableRoles

namespace VerexIR.Tests.TableRoles
open SemanticOperations Validation

/-- A table with a handle: its handle field, then its entries. -/
private def layout : Maps.Layout :=
  { owner := ⟨⟨0⟩, 0⟩, variant := none, entry := none, handle := true, length := false,
    dummy := false }

private def table (value : Int) : RuntimeValue :=
  layout.build (.address "table-a") [(.integer 7, .integer value), (.integer 8, .integer 17)]

-- An entry's loan leaves its hole in the table; the loan's final value fills
-- it, as the model's update of the key.
#guard withEntryHole (table 5) (.integer 7) 42 ==
  some (.nominal ⟨⟨0⟩, 0⟩ none #[.address "table-a",
    .vector #[.tuple #[.integer 7, .loanHole 42], .tuple #[.integer 8, .integer 17]]])
#guard ((withEntryHole (table 5) (.integer 7) 42).bind (fillHole? 42 (.integer 9))) ==
  some (Maps.update layout .sequence (table 5) (.integer 7) (.integer 9))
#guard Maps.update layout .sequence (table 5) (.integer 7) (.integer 9) == table 9

-- A fresh handle is one no table had, a destroyed one included.
#guard freshTableHandle #["table-0", "table-1"] == "table-2"
#guard freshTableHandle #["table-1"] == "table-0"

-- Every executable role has a runtime step; preparation resolves a binding
-- through the same names.
#guard MapOperation.all.all fun operation => MapOperation.ofName? operation.name == some operation
#guard MapSpecRole.all.all fun role => MapSpecRole.ofName? role.name == some role
#guard (MapRole.all.map (·.name)).toList.Nodup
#guard MapRole.ofName? "map_destroy_unchecked" == some (.executable .destroyUnchecked)
#guard MapRole.ofName? "map_spec_get" == some (.specification .get)
#guard (MapRole.ofName? "map_future_role").isNone

-- Runtime abort codes must agree with the VM, independently of contracts.
private def role (name : String) (arguments : Array RuntimeValue) : Option RoleStep :=
  roleStep default default default name
    { namespaceId := ⟨0⟩, structId := 0, handle := true, length := false,
      dummy := false, ordered := false, ghosts := 0 } #[] arguments

private def abortCode (name : String) (arguments : Array RuntimeValue) : Option Int :=
  match role name arguments with
  | some (.abort code) => some code
  | _ => none

#guard abortCode "map_borrow" #[table 5, .integer 99] == some 25863
#guard abortCode "map_borrow_mut" #[table 5, .integer 99] == some 25863
#guard abortCode "map_del_must_exist" #[table 5, .integer 99] == some 25863
#guard abortCode "map_add_no_override" #[table 5, .integer 7, .integer 0] == some 25607
#guard abortCode "map_destroy_empty" #[table 5] == some 196710
#guard match role "map_destroy_unchecked" #[table 5] with
  | some (.returned .unit none none) => true
  | _ => false

private def keys : RuntimeValue := .vector #[.integer 1, .integer 2]
private def shortValues : RuntimeValue := .vector #[.integer 10]
#guard abortCode "map_new_from" #[keys, shortValues] == some 103
#guard abortCode "map_add_all" #[table 5, keys, shortValues] == some 103
#guard abortCode "map_upsert_all" #[table 5, keys, shortValues] == some 103
#guard abortCode "map_new_from"
  #[.vector #[.integer 1, .integer 1], .vector #[.integer 10, .integer 20]] == some 25607

-- A configured map rejects a nonzero degree outside its range and is
-- otherwise the fresh map `map_new` builds.
#guard abortCode "map_new_with_config" #[.integer 3, .integer 0, .bool false] == some 65547
#guard abortCode "map_new_with_config" #[.integer 0, .integer 4097, .bool true] == some 65547
private def allocates (name : String) (arguments : Array RuntimeValue) : Bool :=
  match role name arguments with
  | some (.returned _ none (some "table-0")) => true
  | _ => false
#guard allocates "map_new" #[]
#guard allocates "map_new_with_config" #[.integer 0, .integer 0, .bool false]
#guard allocates "map_new_with_config" #[.integer 4, .integer 4096, .bool true]
#guard !allocates "map_new_with_config" #[.integer 3, .integer 0, .bool false]

end VerexIR.Tests.TableRoles
