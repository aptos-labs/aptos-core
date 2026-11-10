-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec SimpleMap where
  invariant ∀ (i in 0 .. data.length; j in 0 .. data.length), data[i].key == data[j].key ==> i == j

spec fun has_key {V}(data : Vector<Element<V> >, key : u64) : Bool :=
  ∃ (i in 0 .. data.length), data[i].key == key

impl SimpleMap where
  spec new where
    ensures result.data.length == 0

  spec length where
    ensures result == self.data.length

  spec find where
    pragma opaque
    ensures (result is Some) == has_key(self.data, key)
    ensures result is Some ==> result.0 < self.data.length && self.data[result.0].key == key
    ensures ∀ (i in 0 .. self.data.length), result is None ==> self.data[i].key != key
    loop 1 where
      invariant iter.start <= iter.end && iter.end == self.data.length
      invariant ∀ (i in 0 .. iter.start), self.data[i].key != key

  spec contains_key where
    ensures result == has_key(self.data, key)

  spec add where
    panics_if has_key(old(self).data, key)
    ensures self.data.length == old(self).data.length + 1
    ensures ∃ (i in 0 .. self.data.length), self.data[i].key == key
    ensures self.data[self.data.length - 1].key == key
    ensures self.data[self.data.length - 1].value == value
    ensures ∀ (i in 0 .. old(self).data.length), self.data[i] == old(self).data[i]

  spec remove where
    panics_if !has_key(old(self).data, key)
    ensures spec.result[0] == key
    ensures self.data.length == old(self).data.length - 1
    ensures ∀ (i in 0 .. self.data.length), self.data[i].key != key

  spec borrow where
    panics_if !has_key(self.data, key)
    ensures ∃ (i in 0 .. self.data.length), self.data[i].key == key && self.data[i].value == result

  spec borrow_mut where
    panics_if !has_key(old(self).data, key)
    ensures ∃ (i in 0 .. old(self).data.length), old(self).data[i].key == key && old(self).data[i].value == result

impl SimpleMap where
  verify borrow_mut by
    on data invariant {
      assert self'.data.length == old(self).data.length by solve
      assert i < old(self).data.length && j < old(self).data.length by solve
      assert old(self).data[i].key == old(self).data[j].key ==> i == j
      assert self'.data[index].key == old(self).data[index].key by lean =>
        have written := congrArg Prod.fst index_mut_ensures_1
        simp_all
      split i == index
      split j == index
      lean => verex_solve
    }
    on _ { lean => verex_solve }

  verify add by
    on ensures 2 {
      witness old(self).data.length
      lean => verex_solve
    }
    on _ { lean => verex_solve }

  verify remove by
    on data invariant {
      let last := old(self).data.length - 1
      assert old(self).data[last].key == old(self).data[j].key ==> last == j
      assert old(self).data[last].key == old(self).data[i].key ==> last == i
      assert old(self).data[i].key == old(self).data[j].key ==> i == j
      assert i != index ==> self'.data[i] == old(self).data[i]
      assert j != index ==> self'.data[j] == old(self).data[j]
      assert self'.data.length > index ==> self'.data[index] == old(self).data[last]
      split i == index
      split j == index
    }
    on ensures 3 {
      let last := old(self).data.length - 1
      assert old(self).data[index].key == key by solve
      assert old(self).data[last].key == old(self).data[index].key ==> last == index
      assert old(self).data[i].key == old(self).data[index].key ==> i == index
      assert i != index ==> self'.data[i] == old(self).data[i]
      assert self'.data.length > index ==> self'.data[index] == old(self).data[last]
      split i == index
      lean => verex_solve
    }
    on _ { lean => verex_solve }
