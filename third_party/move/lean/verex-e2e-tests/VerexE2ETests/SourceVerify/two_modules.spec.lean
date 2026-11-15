-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

mod m where
  spec fun limit() : u64 := 10

  spec lemma limit_positive() where
    ensures limit() > 0

  spec cell where
    ensures result.value == value

  spec one where
    ensures result == 1

  spec helper where
    ensures result == (value < limit())

  spec double where
    requires value + value <= 4294967295
    ensures result == value + value
    ensures n::limit() == 20

mod n where
  spec fun limit() : u64 := 20

  spec lemma limit_positive() where
    ensures limit() > 0

  spec cell where
    ensures result.value == value

  spec add where
    requires left + right <= 4294967295
    ensures result == left + right

  spec helper where
    ensures result == (value < limit())

  spec next where
    requires value + 1 <= 4294967295
    ensures result == value + 1

  spec from_m where
    ensures result == value

spec both where
  ensures result == (value < n::limit())

verify m::one

verify n::next by
  on ensures 1 {
    assert limit() == 20
  }
