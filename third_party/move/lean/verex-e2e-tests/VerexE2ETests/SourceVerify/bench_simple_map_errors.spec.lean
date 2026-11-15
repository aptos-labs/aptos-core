-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec SimpleMap where
  invariant ∀ (i in 0 .. data.length; j in 0 .. data.length), data[i].key == data[j].key ==> i == j

impl SimpleMap where
  spec find where
    pragma opaque
    ensures result is Some ==> result.0 < self.data.length && self.data[result.0].key == key
    ensures ∀ (i in 0 .. self.data.length), result is None ==> self.data[i].key != key
    loop 1 where
      invariant iter.start <= iter.end && iter.end == self.data.length
      invariant ∀ (i in 0 .. iter.start), self.data[i].key != key

  spec first where
    ensures result is Some ==> result.0 == 0
    loop 1 where
      invariant iter.start <= iter.end && iter.end == self.data.length

  spec add_checked where
    ensures self.data.length == old(self).data.length + 1

  spec add_unchecked where
    ensures self.data.length == old(self).data.length + 1

  spec remove where
    panics_if ∀ (i in 0 .. old(self).data.length), old(self).data[i].key != key
    ensures self.data.length == old(self).data.length

  spec borrow where
    panics_if ∀ (i in 0 .. self.data.length), self.data[i].key != key
    ensures result == self.data[0].value
