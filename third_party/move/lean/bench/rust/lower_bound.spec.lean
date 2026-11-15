-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec binary_search where
  requires start <= end && end <= entries.length
  requires ∀ (i in start .. end; j in start .. end), i <= j ==> entries[i].key <= entries[j].key
  ensures start <= result && result <= end
  ensures ∀ (i in start .. result), entries[i].key < key
  ensures ∀ (i in result .. end), entries[i].key >= key
  loop 1 where
    invariant start <= low && low <= high && high <= end
    invariant ∀ (i in start .. low), entries[i].key < key
    invariant ∀ (i in high .. end), entries[i].key >= key

spec lower_bound where
  requires ∀ (i in 0 .. entries.length; j in 0 .. entries.length), i <= j ==> entries[i].key <= entries[j].key
  ensures result <= entries.length
  ensures ∀ (i in 0 .. result), entries[i].key < key
  ensures ∀ (i in result .. entries.length), entries[i].key >= key
