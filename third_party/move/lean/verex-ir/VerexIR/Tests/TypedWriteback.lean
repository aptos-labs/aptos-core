-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

open VerexIR VerexIR.Proofs.Denote

namespace VerexIR.Tests.TypedWriteback

abbrev U64 := SpecInt (.bits 64) false

example (raw : SpecEntries (U64 × U64)) (named : SpecMap Int Int)
    (read : SpecMap.ofEntries SpecInt.val SpecInt.val raw = named)
    (fallback : U64) (bounded : named.size < 2 ^ 64) :
    SpecMap.ofEntries SpecInt.val SpecInt.val
      (SpecMap.writeBack (fun entry =>
        (SpecInt.ofInt fallback entry.1, SpecInt.ofInt fallback entry.2)) named) = named := by
  verex_written_back

example (raw : SpecEntries (U64 × U64)) (key fallback : U64)
    (present : (SpecMap.ofEntries SpecInt.val SpecInt.val raw).hasKey key.val = true) :
    SpecMap.ofEntries SpecInt.val SpecInt.val
      (SpecMap.writeBack (fun entry =>
        (SpecInt.ofInt fallback entry.1, SpecInt.ofInt fallback entry.2))
        (SpecMap.update .sequence (SpecMap.ofEntries SpecInt.val SpecInt.val raw) key.val 7)) =
      SpecMap.update .sequence (SpecMap.ofEntries SpecInt.val SpecInt.val raw) key.val 7 := by
  verex_written_back

-- Without the defining read, a bounded map need not hold representable integers.
example (named : SpecMap Int Int) (fallback : U64) (bounded : named.size < 2 ^ 64) : True := by
  fail_if_success
    have : SpecMap.ofEntries SpecInt.val SpecInt.val
        (SpecMap.writeBack (fun entry =>
          (SpecInt.ofInt fallback entry.1, SpecInt.ofInt fallback entry.2)) named) = named := by
      verex_written_back
      assumption
  trivial

end VerexIR.Tests.TypedWriteback
