-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec unsorted where
  requires start <= end && end <= entries.length
  ensures ∀ (i in start .. result), entries[i].key < key
  ensures ∀ (i in result .. end), entries[i].key >= key
  loop 1 where
    invariant start <= low && low <= high && high <= end

spec strict where
  requires start <= end && end <= entries.length
  requires ∀ (i in start .. end; j in start .. end), i <= j ==> entries[i].key <= entries[j].key
  ensures ∀ (i in result .. end), entries[i].key > key
  loop 1 where
    invariant start <= low && low <= high && high <= end
    invariant ∀ (i in high .. end), entries[i].key >= key

spec past_end where
  requires start <= end
  requires ∀ (i in start .. end; j in start .. end), i <= j ==> entries[i].key <= entries[j].key
  ensures start <= result && result <= end
  loop 1 where
    invariant start <= low && low <= high && high <= end

spec wrong_invariant where
  requires start <= end && end <= entries.length
  requires ∀ (i in start .. end; j in start .. end), i <= j ==> entries[i].key <= entries[j].key
  ensures start <= result && result <= end
  loop 1 where
    invariant start <= low && low <= high && high <= end
    invariant ∀ (i in start .. low + 1), entries[i].key < key
