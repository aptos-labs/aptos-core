-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Module invariants within a function

A module invariant is owed after each write of memory it reads and after
each call of a callee that leaves it to its callers, unless the function
declares `disable_invariants_in_body`, which defers it to the exit.
-/

namespace LeanerLang.Tests.Check.Storage.ModuleInvariantChecks

leaner module 0x42::module_invariant_checks where
  struct R has Key where
    dummy_field : Bool

  struct S has Key where
    dummy_field : Bool

  spec module where
    invariant [global, suspendable] forall (addr : Address),
      exists<R>(addr) <==> exists<S>(addr)

  -- The invariant does not hold between the two writes.
  public fun publish_both(s : &Signer) -> Unit := do
    move_to<R>(s, new R { dummy_field := false })
    move_to<S>(s, new S { dummy_field := false })
  spec publish_both where
    modifies *

  public fun publish_both_deferred(s : &Signer) -> Unit := do
    move_to<R>(s, new R { dummy_field := false })
    move_to<S>(s, new S { dummy_field := false })
  spec publish_both_deferred where
    pragma disable_invariants_in_body
    modifies *

  -- Called from a body that defers the invariant, so its callers carry it.
  fun publish_r(s : &Signer) -> Unit := do
    move_to<R>(s, new R { dummy_field := false })
  spec publish_r where
    modifies *

  fun publish_s(s : &Signer) -> Unit := do
    move_to<S>(s, new S { dummy_field := false })
  spec publish_s where
    modifies *

  fun publish_deferred(s : &Signer) -> Unit := do
    publish_r(s)
    publish_s(s)
  spec publish_deferred where
    pragma disable_invariants_in_body
    modifies *

  -- The invariant does not hold after the first call.
  public fun publish_calls(s : &Signer) -> Unit := do
    publish_r(s)
    publish_s(s)
  spec publish_calls where
    modifies *

end LeanerLang.Tests.Check.Storage.ModuleInvariantChecks
