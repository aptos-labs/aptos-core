-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang
import LeanerMove.Profile

/-! Port of v0 Negative/BorrowGlobals: removing a resource cannot
invalidate a shared reference that is read afterward. -/

leaner module 0x42::borrow_owner_lifetimes where
  struct Resource has Key where
    value : u64
  struct Other has Key where
    value : u64

  fun take_after_last_read(addr : Address) -> u64 := do
    let observation := &Resource[addr]
    let observed := observation.value
    let Resource { value := removed } := move_from<Resource>(addr)
    return removed

  fun take_other_family(addr : Address) -> u64 := do
    let observation := &Resource[addr]
    let Other { value := removed } := move_from<Other>(addr)
    return observation.value

  fun contains_while_borrowed(addr : Address) -> u64 := do
    let observation := &mut Resource[addr]
    let present := exists<Resource>(addr)
    let value := &observation.value
    return *value

#leaner_unit 0x42::borrow_owner_lifetimes

open Lean Elab Command LeanerIR in
run_cmd do
  let some unit := LeanerLang.registeredUnit? (← getEnv) `«0x42».borrow_owner_lifetimes
    | throwError "missing positive borrow-lifetime module"
  if let .error diagnostics := Validation.prepareExecution #[Move.semantics] unit then
    throwError "valid global ownership change was rejected: {repr diagnostics}"

leaner module 0x42::borrow_invalidation where
  struct BorrowedResource has Key where
    value : u64

  fun invalidate_global_owner(addr : Address) -> u64 := do
    let observation := &BorrowedResource[addr]
    let BorrowedResource { value := removed } := move_from<BorrowedResource>(addr)
    let ignored := *observation
    return removed

  spec invalidate_global_owner where
    requires exists<BorrowedResource>(addr)
    ensures true

#leaner_unit 0x42::borrow_invalidation

/- Borrow diagnostics belong to execution preparation, not structural unit
materialization. Assert its actual rejection and the related loan location. -/
open Lean Elab Command LeanerIR in
run_cmd do
  let some unit := LeanerLang.registeredUnit? (← getEnv) `«0x42».borrow_invalidation
    | throwError "missing negative global-borrow module"
  match Validation.prepareExecution #[Move.semantics] unit with
  | .ok _ => throwError "removing an owner with a live shared loan was accepted"
  | .error diagnostics =>
    unless diagnostics.size == 1 do
      throwError "unexpected global-borrow diagnostics: {repr diagnostics}"
    let diagnostic := diagnostics[0]!
    unless diagnostic.code == "LIR-SEMANTIC-BORROW-CONFLICT" &&
        diagnostic.message == "global owner write conflicts with an active LeanerIR.ReferenceKind.shared loan" &&
        diagnostic.primary.isSome && diagnostic.related.size == 1 &&
        diagnostic.related[0]!.message == "active loan originates here" do
      throwError "wrong global-borrow rejection: {repr diagnostic}"
