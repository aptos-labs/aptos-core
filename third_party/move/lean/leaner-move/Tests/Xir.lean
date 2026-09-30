-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerMove

/-!
# XIR backend

Lowering Leaner Move modules to deployable XIR: the shape of the emitted
module, and the located rejection of constructs Move bytecode cannot express.
Execution of the emitted bytecode is covered by compiler-v2's transactional
tests over `.lean` sources.
-/

namespace LeanerMove.Tests.Xir

open LeanerIR.Move.Xir

leaner module 0x42::xir_shapes where
  use std::vector

  struct Counter has Key where
    value : u64

  struct Marker has Drop where

  enum Choice has Copy, Drop where
    | Left (value : u64)
    | Right

  fun read(addr : Address) -> u64 := do
    let counter := &Counter[addr]
    counter.value

  -- Acquires what `read` acquires, through the call.
  fun forward(addr : Address) -> u64 := read(addr)

  fun owner(account : &Signer) -> Address := account.address

  fun has_value(values : &Vector<u64>, value : &u64) -> Bool :=
    (vector::contains(values, value) : Bool)

  fun mark() -> Unit := do
    let Marker {} := new Marker {}

  fun pick(choice : Choice) -> u64 :=
    match choice with
      | Choice::Left { value := value } => value
      | Choice::Right {} => 0

open Lean Elab Command in
run_cmd do
  let some unit := LeanerLang.registeredUnit? (← getEnv) `«0x42».xir_shapes
    | throwError "the module was not registered"
  let module ← match lowerModule unit ⟨0⟩ with
    | .ok module => pure module
    | .error failure => throwError failure.message
  let function (name : String) : CommandElabM Function := do
    let some function := module.functions.find? (·.name == name)
      | throwError s!"no function `{name}`"
    pure function
  unless (module.address, module.name) == ("0x42", "xir_shapes") do
    throwError "the module identity is its namespace's address and name"
  unless module.structs.map (·.variants.isSome) == #[false, false, true] do
    throwError "a struct has no variants and an enum lists them"
  unless module.structs[1]!.fields.map (·.name) == #["dummy_field"] do
    throwError "a structure without fields has Move's dummy field"
  unless (← function "read").acquires == #[0] && (← function "forward").acquires == #[0] do
    throwError "a borrowed resource is acquired, through calls as well"
  unless module.externalFunctions.map (fun e => (e.address, e.module, e.name)) ==
      #[("0x1", "signer", "address_of"), ("0x1", "vector", "contains")] do
    throwError s!"unexpected external functions {repr module.externalFunctions}"
  unless (← function "pick").localNames[0]? == some (some "choice") do
    throwError "parameters keep their source names"

-- Source attributes reach the bytecode: the VM takes the module lock of a
-- `module_lock` function, for example.
leaner module 0x42::xir_attributes where
  @[event]
  struct Emitted has Drop, Store where
    value : u64

  @[module_lock]
  fun locked() -> u64 := 1

  @[persistent]
  public fun kept() -> u64 := 2

open Lean Elab Command in
run_cmd do
  let some unit := LeanerLang.registeredUnit? (← getEnv) `«0x42».xir_attributes
    | throwError "the module was not registered"
  let module ← match lowerModule unit ⟨0⟩ with
    | .ok module => pure module
    | .error failure => throwError failure.message
  let attributes (name : String) : CommandElabM (Array String) := do
    let some function := module.functions.find? (·.name == name)
      | throwError s!"no function `{name}`"
    pure (function.attributes.map (·.name))
  unless (← attributes "locked") == #["module_lock"] && (← attributes "kept") == #["persistent"] do
    throwError "a function keeps its source attributes, and only those"
  unless module.structs.map (·.attributes.map (·.name)) == #[#["event"]] do
    throwError "a struct keeps its source attributes"
  let some locked := module.functions.find? (·.name == "locked") | throwError "no `locked`"
  unless locked.toJson.getObjValD "attributes" == Json.arr #[Json.mkObj [("name", "module_lock")]] do
    throwError s!"unexpected attributes JSON {locked.toJson.getObjValD "attributes"}"

leaner module 0x42::xir_unbounded where
  fun count(value : Nat) -> u64 := 1

/-- error: Move bytecode has no unbounded or pointer-width integers -/
#guard_msgs in
#leaner_xir «0x42».xir_unbounded

leaner module 0x42::xir_package where
  package fun shared() -> u64 := 1

/-- error: Move bytecode has no package visibility; declare it `friend` -/
#guard_msgs in
#leaner_xir «0x42».xir_package

leaner module 0x42::xir_publish where
  struct Counter has Key where
    value : u64

  fun publish(addr : Address) -> Unit := move_to<Counter>(addr, new Counter { value := 0 })

/-- error: Move bytecode publishes a resource under a `&signer` -/
#guard_msgs in
#leaner_xir «0x42».xir_publish

end LeanerMove.Tests.Xir
