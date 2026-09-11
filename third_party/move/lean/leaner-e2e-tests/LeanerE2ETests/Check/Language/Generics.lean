-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! Port of v0 Language/Generics; see designs/test-organization.md.
Ability bounds inferred by v0 are explicit
here. Storage primitives take the signer's address at the shared boundary.
Generic bodies should verify once over native carriers, not once per caller. -/

set_option leaner.route "native"

leaner module 0x42::language_generics where
  struct Box {T has Copy, Drop, Store} has Copy, Drop, Store where
    value : T
  struct Pair {T has Copy, Drop, Store} {U has Copy, Drop, Store} has Copy, Drop, Store where
    first : T
    second : U
  struct Vault {T has Store} has Key where
    value : T
  enum Choice {T has Copy, Drop, Store} has Copy, Drop, Store where
    | None
    | Some (value : T)

  fun identity {T}(value : T) -> T := value
  spec identity where
    ensures result == value
  verify identity

  fun box {T has Copy, Drop, Store}(value : T) -> Box<T> := new Box<T> { value }
  spec box where
    ensures result == new Box<T> { value }
  verify box

  fun unbox {T has Copy, Drop, Store}(value : Box<T>) -> T := value.value
  spec unbox where
    ensures result == value.value
  verify unbox

  fun swap {T has Copy, Drop, Store} {U has Copy, Drop, Store}(value : Pair<T, U>) -> Pair<U, T> :=
    new Pair<U, T> { first := value.second, second := value.first }
  spec swap where
    ensures result == new Pair<U, T> { first := value.second, second := value.first }
  verify swap

  fun choose_generic {T has Copy, Drop, Store}(fallback : T, choice : Choice<T>) -> T :=
    match choice with
      | Choice<T>::None {} => fallback
      | Choice<T>::Some { value := value } => value
  spec choose_generic where
    ensures result == match choice with
      | Choice<T>::None {} => fallback
      | Choice<T>::Some { value := value } => value
  verify choose_generic

  fun singleton {T}(value : T) -> Vector<T> := vector<T>[value]
  spec singleton where
    ensures result == vector<T>[value]
  verify singleton

  fun equal_generic {T has Copy, Drop}(left : T, right : T) -> Bool := left == right
  spec equal_generic where
    ensures result == (left == right)
  verify equal_generic

  fun equal_u64(left : u64, right : u64) -> Bool := left == right
  spec equal_u64 where
    ensures result == (left == right)
  verify equal_u64

  fun equal_boxes(left : u64, right : u64) -> Bool :=
    equal_generic::<Box<u64> >(new Box<u64> { value := left }, new Box<u64> { value := right })
  spec equal_boxes where
    ensures result == (new Box<u64> { value := left } == new Box<u64> { value := right })
  verify equal_boxes

  fun equal_choices(left : u64, right : u64) -> Bool :=
    equal_generic::<Choice<u64> >(new Choice<u64>::Some { value := left }, new Choice<u64>::Some { value := right })
  spec equal_choices where
    ensures result == (new Choice<u64>::Some { value := left } == new Choice<u64>::Some { value := right })
  verify equal_choices

  fun equal_vectors(left : u64, right : u64) -> Bool :=
    equal_generic::<Vector<u64> >(vector<u64>[left], vector<u64>[right])
  spec equal_vectors where
    ensures result == (vector<u64>[left] == vector<u64>[right])
  verify equal_vectors

  fun wrap(value : u64) -> Box<u64> := new Box<u64> { value := identity::<u64>(value) }
  spec wrap where
    ensures result == new Box<u64> { value }
  verify wrap

  fun unwrap(box : Box<u64>) -> u64 := box.value
  spec unwrap where
    ensures result == box.value
  verify unwrap

  fun choose(fallback : u64, choice : Choice<u64>) -> u64 := choose_generic::<u64>(fallback, choice)
  spec choose where
    ensures result == match choice with
      | Choice<u64>::None {} => fallback
      | Choice<u64>::Some { value := value } => value
  verify choose

  fun swapped(value : u64) -> Pair<u64, u64> :=
    swap::<u64, u64>(new Pair<u64, u64> { first := value, second := value + 1 })
  spec swapped where
    ensures result == new Pair<u64, u64> { first := value + 1, second := value }
  verify swapped

  fun singleton_length(value : u64) -> u64 := singleton::<u64>(value).length
  spec singleton_length where
    ensures result == 1
  verify singleton_length

  fun publish_generic {T has Store}(address : Address, value : T) -> Unit :=
    move_to<Vault<T> >(address, new Vault<T> { value })
  fun has_generic {T has Store}(address : Address) -> Bool := exists<Vault<T> >(address)
  fun tag_interactions {T has Store} {U has Store}(address : Address) -> Bool := do
    let hasT := exists<Vault<T> >(address)
    let hasU := exists<Vault<U> >(address)
    let hasU64 := exists<Vault<u64> >(address)
    return hasT && hasU && hasU64
  fun publish_vault(address : Address, value : u64) -> Unit := publish_generic::<u64>(address, value)
  fun take_vault(address : Address) -> u64 := do
    let Vault<u64> { value := value } := move_from<Vault<u64> >(address)
    return value
  fun has_vault(address : Address) -> Bool := has_generic::<u64>(address)
  fun publish_bool_vault(address : Address, value : Bool) -> Unit := publish_generic::<Bool>(address, value)
  fun take_bool_vault(address : Address) -> Bool := do
    let Vault<Bool> { value := value } := move_from<Vault<Bool> >(address)
    return value
  fun has_bool_vault(address : Address) -> Bool := has_generic::<Bool>(address)

  -- The interpreter enters a concrete wrapper; the generic body stays shared.
  fun identity_u64(value : u64) -> u64 := identity::<u64>(value)

  -- Exercise the five family-collision patterns of v0's verification plan
  -- through the actual shared body and its runtime type instantiation.
  fun tag_distinct(address : Address) -> Bool := tag_interactions::<Bool, u8>(address)
  fun tag_same(address : Address) -> Bool := tag_interactions::<Bool, Bool>(address)
  fun tag_first_fixed(address : Address) -> Bool := tag_interactions::<u64, Bool>(address)
  fun tag_second_fixed(address : Address) -> Bool := tag_interactions::<Bool, u64>(address)
  fun tag_both_fixed(address : Address) -> Bool := tag_interactions::<u64, u64>(address)
  fun has_vector_vaults(address : Address) -> Bool :=
    exists<Vault<Vector<u64> > >(address) && exists<Vault<Vector<Bool> > >(address)

open Lean Elab Command in
run_cmd do
  for function in ["identity", "box", "unbox", "swap", "choose_generic", "singleton",
      "equal_generic", "equal_u64", "equal_boxes", "equal_choices", "equal_vectors",
      "wrap", "unwrap", "choose", "swapped", "singleton_length"] do
    let name := ((`«0x42».language_generics).str function).str "verified"
    unless (← getEnv).contains name do throwError "missing generic proof: {name}"
    if (← collectAxioms name).contains ``sorryAx then
      throwError "generic proof contains an admission: {name}"

open Lean Elab Command LeanerIR in
run_cmd do
  let env ← getEnv
  let some unit := LeanerLang.registeredUnit? env `«0x42».language_generics
    | throwError "missing generic language module"
  let ns := unit.namespaces[0]!
  let function (name : String) := ns.functions.find? fun declaration =>
    ns.tables.names[declaration.name.index]?.any (·.name == name)
  -- V0 checked an acquires list. At the shared boundary, pin the actual
  -- primitive kinds: existence/publishing do not borrow or remove data.
  for (name, expected) in [("has_generic", GlobalKind.contains),
      ("publish_generic", GlobalKind.publish), ("take_vault", GlobalKind.take)] do
    let some declaration := function name | throwError "missing {name}"
    let .structured root := declaration.body | throwError "missing body of {name}"
    let mut stack := #[root]
    let mut seen : Std.HashSet Nat := {}
    let mut globals : Array GlobalKind := #[]
    while !stack.isEmpty do
      let id := stack.back!
      stack := stack.pop
      if seen.contains id.index then continue
      seen := seen.insert id.index
      let expression := ns.expressions[id.index]!
      stack := stack ++ Validation.expressionChildren expression.kind
      if let .operation (.global kind) _ _ _ := expression.kind then
        globals := globals.push kind
    unless globals == #[expected] do throwError "wrong resource access in {name}"
  -- Unlike V0's binder-free verification clone, this route retains one
  -- generic body and specializes only the invocation's type map.
  unless ns.functions.size == 32 do throwError "generic bodies were duplicated"
  for (name, parameters) in [("identity", 1), ("publish_generic", 1),
      ("tag_interactions", 2), ("swap", 2)] do
    let some declaration := function name | throwError "missing {name}"
    unless declaration.signature.generics.size == parameters do
      throwError "generic binders were erased from {name}"
  for (name, expected) in [("publish_vault", Ty.integer (.bits 64) false),
      ("publish_bool_vault", Ty.bool)] do
    let some declaration := function name | throwError "missing {name}"
    let .structured root := declaration.body | throwError "missing body of {name}"
    let expression := ns.expressions[root.index]!
    let .operation (.call (.function target)) #[.typeArg argument] _ _ := expression.kind
      | throwError "generic publisher invocation was erased in {name}"
    unless ns.tables.names[target.name.index]?.any (·.name == "publish_generic") &&
        ns.tables.types[argument.typeId.index]? == some expected do
      throwError "wrong generic publisher instantiation in {name}"
  let .ok printed := LeanerLang.Print.render env unit
    | throwError "generics did not render"
  let formatted ← match LeanerLang.Print.formatSource env printed with
    | .ok formatted => pure formatted
    | .error error => throwError "generics did not re-import: {error}\n{printed}"
  unless formatted == printed do
    throwError "generics are not a canonical fixed point:\n{printed}\n{formatted}"

-- The public runtime boundary states the Move data domain explicitly.
-- Abstract codecs themselves remain unrestricted, and loan-bearing values
-- are not silently admitted as source-level generic data arguments.
open LeanerIR LeanerIR.Proofs in
set_option maxHeartbeats 1000 in
example (value : RuntimeValue) (initial : RuntimeState) :
    («0x42».language_generics.identity.rawContract #[]
      (fun _ => Codec.identity RuntimeValue)).requires #[value] initial ↔
        SemanticOperations.FreshGlobalLoanIds initial ∧ SemanticOperations.Plain value := by
  simp [«0x42».language_generics.identity.rawContract, Codec.identity, and_assoc]

open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  let box (value : Int) : RuntimeValue := .nominal ⟨⟨0⟩, 0⟩ none #[.integer value]
  let pair (left right : Int) : RuntimeValue := .nominal ⟨⟨0⟩, 1⟩ none #[.integer left, .integer right]
  let choice (variant : String) (values : Array RuntimeValue) : RuntimeValue :=
    .nominal ⟨⟨0⟩, 3⟩ (some variant) values
  assertRuns `«0x42».language_generics #[
    ⟨"identity_u64", #[.integer 11], .returned #[.integer 11], {}⟩,
    ⟨"wrap", #[.integer 12], .returned #[box 12], {}⟩,
    ⟨"unwrap", #[box 13], .returned #[.integer 13], {}⟩,
    ⟨"choose", #[.integer 4, choice "None" #[]], .returned #[.integer 4], {}⟩,
    ⟨"choose", #[.integer 4, choice "Some" #[.integer 15]], .returned #[.integer 15], {}⟩,
    ⟨"swapped", #[.integer 16], .returned #[pair 17 16], {}⟩,
    ⟨"singleton_length", #[.integer 17], .returned #[.integer 1], {}⟩,
    ⟨"equal_u64", #[.integer 17, .integer 17], .returned #[.bool true], {}⟩,
    ⟨"equal_u64", #[.integer 17, .integer 18], .returned #[.bool false], {}⟩,
    ⟨"equal_boxes", #[.integer 17, .integer 17], .returned #[.bool true], {}⟩,
    ⟨"equal_boxes", #[.integer 17, .integer 18], .returned #[.bool false], {}⟩,
    ⟨"equal_choices", #[.integer 19, .integer 19], .returned #[.bool true], {}⟩,
    ⟨"equal_choices", #[.integer 19, .integer 20], .returned #[.bool false], {}⟩,
    ⟨"equal_vectors", #[.integer 21, .integer 21], .returned #[.bool true], {}⟩,
    ⟨"equal_vectors", #[.integer 21, .integer 22], .returned #[.bool false], {}⟩]

open Lean Elab Command LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  let some unit := LeanerLang.registeredUnit? (← getEnv) `«0x42».language_generics
    | throwError "missing generic language module"
  let ns := unit.namespaces[0]!
  let vaultName := ns.structs[2]!.name
  let vaultKey (argument : Ty) : CommandElabM GlobalKey := do
    let some argumentIndex := ns.tables.types.findIdx? (· == argument)
      | throwError "missing vault argument type"
    let some typeIndex := ns.tables.types.findIdx? fun
        | .nominal name #[.typeArg argument] =>
            name == vaultName && argument.typeId.index == argumentIndex
        | _ => false
      | throwError "missing instantiated vault type"
    return ⟨⟨0⟩, ⟨typeIndex⟩, .address "7"⟩
  let integerKey ← vaultKey (.integer (.bits 64) false)
  let boolKey ← vaultKey .bool
  if integerKey == boolKey then throwError "generic storage families collided"
  let vectorKey (element : Ty) : CommandElabM GlobalKey := do
    let some index := ns.tables.types.findIdx? (· == element)
      | throwError "missing vector element type"
    vaultKey (.vector ⟨index⟩)
  if (← vectorKey (.integer (.bits 64) false)) == (← vectorKey .bool) then
    throwError "nested generic storage families collided"
  let vault (value : RuntimeValue) : RuntimeValue := .nominal ⟨⟨0⟩, 2⟩ none #[value]
  let integerState (value : Int) : RuntimeState :=
    { globals := ({} : GlobalMap).insert integerKey (vault (.integer value)) }
  let bothState (value : Int) : RuntimeState :=
    { globals := (integerState value).globals.insert boolKey (vault (.bool true)) }
  assertRunsState `«0x42».language_generics #[
    ⟨"publish_vault", #[.address "7", .integer 18], .returned #[], {}, integerState 18⟩,
    ⟨"take_vault", #[.address "7"], .returned #[.integer 19], integerState 19, {}⟩,
    ⟨"has_vault", #[.address "7"], .returned #[.bool true], integerState 20, integerState 20⟩,
    ⟨"has_vault", #[.address "7"], .returned #[.bool false], {}, {}⟩,
    ⟨"publish_bool_vault", #[.address "7", .bool true], .returned #[], integerState 21, bothState 21⟩,
    ⟨"has_vault", #[.address "7"], .returned #[.bool true], bothState 22, bothState 22⟩,
    ⟨"has_bool_vault", #[.address "7"], .returned #[.bool true], bothState 23, bothState 23⟩,
    ⟨"take_bool_vault", #[.address "7"], .returned #[.bool true], bothState 24, integerState 24⟩]
  let byteKey ← vaultKey (.integer (.bits 8) false)
  let allState : RuntimeState :=
    { globals := (bothState 25).globals.insert byteKey (vault (.integer 1)) }
  for function in ["tag_distinct", "tag_same", "tag_first_fixed", "tag_second_fixed", "tag_both_fixed"] do
    assertRunsState `«0x42».language_generics #[
      ⟨function, #[.address "7"], .returned #[.bool true], allState, allState⟩,
      ⟨function, #[.address "7"], .returned #[.bool false], {}, {}⟩]
  -- Each nontrivial collision still reads the other, distinct family.
  for function in ["tag_distinct", "tag_same", "tag_first_fixed", "tag_second_fixed"] do
    assertRunsState `«0x42».language_generics #[
      ⟨function, #[.address "7"], .returned #[.bool false], integerState 26, integerState 26⟩]
  assertRunsState `«0x42».language_generics #[
    ⟨"tag_distinct", #[.address "7"], .returned #[.bool false], bothState 27, bothState 27⟩,
    ⟨"tag_both_fixed", #[.address "7"], .returned #[.bool true], integerState 28, integerState 28⟩]
