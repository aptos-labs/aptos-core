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

abbrev U8 := SpecInt (.bits 8) false

-- The dictionary the obligation states, not one instance synthesis finds: a
-- table's key carrier comes with its family's decidable equality.
example {Key : Type} (dictionary : DecidableEq Key) (key : Key) (fallback : U64) :
    SpecMap.ofEntries (fun value => value) SpecInt.val
        (SpecMap.writeBack (fun entry => (entry.1, SpecInt.ofInt fallback entry.2))
          (@SpecMap.update Key Int dictionary .sequence [] key 1)) =
      @SpecMap.update Key Int dictionary .sequence [] key 1 := by
  verex_written_back

-- Identity keys, identity values.
example {Key Value : Type} (dictionary : DecidableEq Key) (key : Key) (value : Value) :
    SpecMap.ofEntries (fun k => k) (fun v => v)
        (SpecMap.writeBack (fun entry => (entry.1, entry.2))
          (@SpecMap.update Key Value dictionary .sequence [] key value)) =
      @SpecMap.update Key Value dictionary .sequence [] key value := by
  verex_written_back

-- Integer keys, identity values: the key reads back at its stored value.
example {Value : Type} (key : U8) (fallback : U8) (value : Value) :
    SpecMap.ofEntries SpecInt.val (fun v => v)
        (SpecMap.writeBack (fun entry => (SpecInt.ofInt fallback entry.1, entry.2))
          (SpecMap.update .sequence [] key.val value)) =
      SpecMap.update .sequence [] key.val value := by
  verex_written_back

-- Identity keys, integer values, over a chain of updates and a removal.
example {Key : Type} (dictionary : DecidableEq Key) (first second : Key) (stored : U64)
    (fallback : U64) :
    SpecMap.ofEntries (fun value => value) SpecInt.val
        (SpecMap.writeBack (fun entry => (entry.1, SpecInt.ofInt fallback entry.2))
          (@SpecMap.remove Key Int dictionary .sequence
            (@SpecMap.update Key Int dictionary .sequence
              (@SpecMap.update Key Int dictionary .sequence [] first 1) second stored.val)
            first)) =
      @SpecMap.remove Key Int dictionary .sequence
        (@SpecMap.update Key Int dictionary .sequence
          (@SpecMap.update Key Int dictionary .sequence [] first 1) second stored.val)
        first := by
  verex_written_back

-- A value with no fit does not read back: the entries are not certified.
example {Key : Type} (dictionary : DecidableEq Key) (key : Key) (fallback : U64)
    (value : Int) : True := by
  fail_if_success
    have : SpecMap.ofEntries (fun read => read) SpecInt.val
          (SpecMap.writeBack (fun entry => (entry.1, SpecInt.ofInt fallback entry.2))
            (@SpecMap.update Key Int dictionary .sequence [] key value)) =
        @SpecMap.update Key Int dictionary .sequence [] key value := by
      verex_written_back
  trivial

-- A value outside the width does not read back.
example {Key : Type} (dictionary : DecidableEq Key) (key : Key) (fallback : U64) : True := by
  fail_if_success
    have : SpecMap.ofEntries (fun read => read) SpecInt.val
          (SpecMap.writeBack (fun entry => (entry.1, SpecInt.ofInt fallback entry.2))
            (@SpecMap.update Key Int dictionary .sequence [] key (2 ^ 64))) =
        @SpecMap.update Key Int dictionary .sequence [] key (2 ^ 64) := by
      verex_written_back
  trivial

-- An integer key with no fit does not read back.
example (key : Int) (fallback : U8) (value : Int) : True := by
  fail_if_success
    have : SpecMap.ofEntries SpecInt.val (fun read => read)
          (SpecMap.writeBack (fun entry => (SpecInt.ofInt fallback entry.1, entry.2))
            (SpecMap.update .sequence [] key value)) =
        SpecMap.update .sequence [] key value := by
      verex_written_back
  trivial

-- A value whose fit is not its own: the value the entry holds is named by a
-- hypothesis, and what it fits follows from the certified integer it equals.
example {Key : Type} (dictionary : DecidableEq Key) (key : Key) (fallback stored : U64)
    (named : Int) (holds : stored.val = named) :
    SpecMap.ofEntries (fun read => read) SpecInt.val
        (SpecMap.writeBack (fun entry => (entry.1, SpecInt.ofInt fallback entry.2))
          (@SpecMap.update Key Int dictionary .sequence [] key named)) =
      @SpecMap.update Key Int dictionary .sequence [] key named := by
  verex_written_back

-- A reader a family states, with the round trip the context holds: the entries
-- read back by that premise and by nothing else.
example {Key Native Held : Type} (dictionary : DecidableEq Key) (key : Key)
    (read : Native → Held) (unread : Held → Native) (value : Held)
    (back : ∀ held, read (unread held) = held) :
    SpecMap.ofEntries (fun key => key) read
        (SpecMap.writeBack (fun entry => (entry.1, unread entry.2))
          (@SpecMap.update Key Held dictionary .sequence [] key value)) =
      @SpecMap.update Key Held dictionary .sequence [] key value := by
  verex_written_back

-- Without that premise the family's reader does not read back.
example {Key Native Held : Type} (dictionary : DecidableEq Key) (key : Key)
    (read : Native → Held) (unread : Held → Native) (value : Held) : True := by
  fail_if_success
    have : SpecMap.ofEntries (fun key => key) read
          (SpecMap.writeBack (fun entry => (entry.1, unread entry.2))
            (@SpecMap.update Key Held dictionary .sequence [] key value)) =
        @SpecMap.update Key Held dictionary .sequence [] key value := by
      verex_written_back
  trivial

-- A component the reader's law reduces without closing: the stored-integer law
-- rewrites the left side to `stored.val`, and the local equality closes what it
-- leaves. The cascade this replaced accepted the rewrite as success and left
-- `stored.val = named` behind.
example (fallback stored : U64) (named : Int) (holds : stored.val = named) :
    (SpecInt.ofInt fallback stored.val).val = named := by
  verex_written_component

/-! A lookup is not a computation: the bound of an updated map's size looks
the key's presence up among the hypotheses, and compares none of them by
unfolding. -/

-- A present key, stated: the size is the map's own.
example (map : SpecMap Int Int) (present : map.hasKey 5 = true) (bounded : map.size < 2 ^ 64) :
    (SpecMap.update .sequence map 5 7).size < 2 ^ 64 := by
  rw [SpecMap.size_update_sequence, if_pos (by verex_assumption)]
  exact bounded

-- Stated under a source-location marker, and with its sides exchanged.
example (map : SpecMap Int Int) (present : VerexIR.Proofs.Obligation "source" 0 0 (true = map.hasKey 5))
    (bounded : map.size < 2 ^ 64) : (SpecMap.update .sequence map 5 7).size < 2 ^ 64 := by
  rw [SpecMap.size_update_sequence, if_pos (by verex_assumption)]
  exact bounded

-- Through the step, where only the presence bounds the size: no validity
-- is stated, so neither looser bound of an update applies.
example (raw : SpecEntries (U64 × U64)) (key fallback : U64)
    (present : VerexIR.Proofs.Obligation "source" 0 0
      (true = (SpecMap.ofEntries SpecInt.val SpecInt.val raw).hasKey key.val)) :
    SpecMap.ofEntries SpecInt.val SpecInt.val
      (SpecMap.writeBack (fun entry =>
        (SpecInt.ofInt fallback entry.1, SpecInt.ofInt fallback entry.2))
        (SpecMap.update .sequence (SpecMap.ofEntries SpecInt.val SpecInt.val raw) key.val 7)) =
      SpecMap.update .sequence (SpecMap.ofEntries SpecInt.val SpecInt.val raw) key.val 7 := by
  verex_written_back

-- The same read without the presence is not bounded: a table may be full.
example (raw : SpecEntries (U64 × U64)) (key fallback : U64) : True := by
  fail_if_success
    have : SpecMap.ofEntries SpecInt.val SpecInt.val
        (SpecMap.writeBack (fun entry =>
          (SpecInt.ofInt fallback entry.1, SpecInt.ofInt fallback entry.2))
          (SpecMap.update .sequence (SpecMap.ofEntries SpecInt.val SpecInt.val raw) key.val 7)) =
        SpecMap.update .sequence (SpecMap.ofEntries SpecInt.val SpecInt.val raw) key.val 7 := by
      verex_written_back
      assumption
  trivial

-- A presence that only evaluation shows is not a stated one.
example : True := by
  fail_if_success
    have : SpecMap.hasKey ([(5, 7)] : SpecMap Int Int) 5 = true := by verex_assumption
  trivial

/-- A typed map written back and read again, as a leaf states it. -/
local macro "writtenBackRead(" key:term ", " value:term ", " map:term ")" : term =>
  `(SpecMap.ofEntries SpecInt.val SpecInt.val
      (SpecMap.writeBack (fun entry => (SpecInt.ofInt $key entry.1, SpecInt.ofInt $value entry.2))
        $map))

-- An absent key falls to the next arm, beside an equation between nested
-- written-back maps over a removal at a free key: a hypothesis with an
-- equation's shape, whose sides evaluate at length.
example (key fallbackKey : U8) (fallback : U64)
    (erased :
      writtenBackRead(fallbackKey, fallback, writtenBackRead(fallbackKey, fallback,
        writtenBackRead(fallbackKey, fallback, writtenBackRead(fallbackKey, fallback,
          SpecMap.remove .sequence
            (SpecMap.update .sequence ([(1, 2)] : SpecMap Int Int) 2 3) key.val)))) =
      writtenBackRead(fallbackKey, fallback, writtenBackRead(fallbackKey, fallback,
        writtenBackRead(fallbackKey, fallback,
          SpecMap.remove .sequence
            (SpecMap.update .sequence ([(1, 2)] : SpecMap Int Int) 2 3) key.val)))) :
    writtenBackRead(fallbackKey, fallback, SpecMap.update .sequence ([] : SpecMap Int Int) 1 2) =
      SpecMap.update .sequence ([] : SpecMap Int Int) 1 2 := by
  verex_written_back

end VerexIR.Tests.TypedWriteback
