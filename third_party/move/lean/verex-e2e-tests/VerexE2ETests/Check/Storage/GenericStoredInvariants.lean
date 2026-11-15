-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! The data invariant of a stored generic resource holds at every frame
(`StoredCaseAt`): a reader at a generic frame takes it there, and a writer,
at a generic frame or a concrete one, establishes it at every frame, as the
invariant reads the stored value only. -/

namespace VerexSL.Tests.Check.Storage.GenericStoredInvariants

verex module 0x42::generic_stored_invariants where
  struct Tally {T : phantom type} has Key where
    count : u64
  spec Tally where
    invariant count > 0

  fun publish_u64(account : &Signer) -> Unit :=
    move_to<Tally<u64> >(account, new Tally<u64> { count := 1 })
  spec publish_u64 where
    modifies global<Tally<u64> >(account.address)
    aborts_if exists<Tally<u64> >(account.address)

  fun publish {T}(account : &Signer, count : u64) -> Unit :=
    move_to<Tally<T> >(account, new Tally<T> { count })
  spec publish where
    requires count > 0
    modifies global<Tally<T> >(account.address)
    aborts_if exists<Tally<T> >(account.address)

  fun publish_empty {T}(account : &Signer) -> Unit :=
    move_to<Tally<T> >(account, new Tally<T> { count := 0 }) -- error: the invariant fails
  spec publish_empty where
    modifies global<Tally<T> >(account.address)
    aborts_if exists<Tally<T> >(account.address)

  fun read {T}(address : Address) -> u64 := Tally<T>[address].count
  spec read where
    aborts_if !exists<Tally<T> >(address)
    ensures result > 0

  fun bump {T}(address : Address) -> Unit := do
    let count := &mut Tally<T>[address].count
    *count := *count + 1
  spec bump where
    pragma aborts_if_is_partial
    modifies global<Tally<T> >(address)
    ensures Tally<T>[address].count > 1

end VerexSL.Tests.Check.Storage.GenericStoredInvariants
