-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! The data invariant of a stored generic resource whose fields hold its
type parameter's values is stated at the resource's own type arguments
(`StoredCase`): a writer or reader at a generic frame moves its view of the
value there (`Skolems.reframe`), so it holds at every frame by
construction. -/

namespace VerexSL.Tests.Check.Storage.FrameReadingInvariants

verex module 0x42::frame_reading_invariants where
  struct Pair {T has Copy, Store} has Key where
    first : T
    second : T
  spec Pair where
    invariant first == second

  struct Echo {T has Copy, Store} has Key where
    value : T
  spec Echo where
    invariant value == value

  fun publish_echo {T has Copy, Store}(account : &Signer, value : T) -> Unit :=
    move_to<Echo<T> >(account, new Echo<T> { value })
  spec publish_echo where
    modifies global<Echo<T> >(account.address)
    aborts_if exists<Echo<T> >(account.address)

  fun publish {T has Copy, Store}(account : &Signer, value : T) -> Unit :=
    move_to<Pair<T> >(account, new Pair<T> { first := value, second := value })
  spec publish where
    modifies global<Pair<T> >(account.address)
    aborts_if exists<Pair<T> >(account.address)

  fun publish_u64(account : &Signer) -> Unit :=
    move_to<Pair<u64> >(account, new Pair<u64> { first := 1, second := 1 })
  spec publish_u64 where
    modifies global<Pair<u64> >(account.address)
    aborts_if exists<Pair<u64> >(account.address)

  fun same {T has Copy, Store}(address : Address) -> Bool :=
    Pair<T>[address].first == Pair<T>[address].second
  spec same where
    aborts_if !exists<Pair<T> >(address)
    ensures result

end VerexSL.Tests.Check.Storage.FrameReadingInvariants
