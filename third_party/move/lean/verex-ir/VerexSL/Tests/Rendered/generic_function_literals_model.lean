-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.Rendered.Support

/-!
# `generic_function_literals_model` as a frontend renders it

The module below is the rendering of
`VerexE2ETests/SourceVerify/generic_function_literals_model.move` that the
source-verification driver writes to
`.verex/generic_function_literals_model.move.lean`, its `verex sources at`
command dropped; the diagnostics are those of
`generic_function_literals_model.exp`. Neither is edited here:
`scripts/prover-smoke-renderings.sh --write` writes this file again, and
without `--write` tells whether it is what the frontend renders.
-/

/-!
In a unit verified through its model, `aborts_of` and `ensures_of` of a literal of
a known target are what its contract states at the frame its type arguments induce,
whether a call of it goes through that contract or its body is inlined.
-/

/--
error: the specification clause `ensures global<Pending>(@0x1).n == 0` is not established
---
error: verex verification failed: the automatic verification of `other_argument` failed; provide a proof in `VerexE2ETests/SourceVerify/generic_function_literals_model.proof.lean`:
  verify other_argument by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_rendering in
verex module 0x1::generic_function_literals_model where
  pragma proof_file = "VerexE2ETests/SourceVerify/generic_function_literals_model.proof.lean"

  -- Copyright © Aptos Foundation
  -- SPDX-License-Identifier: Apache-2.0
  struct Pending has Key where
    n : u64

  struct Version has Drop, Store where
    major : u64

  /--
  A ghost field: the unit is verified through its model.
  -/
  struct Marked has Drop where
    v : u64

  spec Marked where
    ghost tag : Int

  fun check(s : u64) -> Unit :=
    assert!(s == 1, 1)

  spec check where
    pragma opaque
    aborts_if s != 1

  fun put {T has Drop, Store}(x : T) -> Unit := do
    let p := &mut Pending[@0x1]
    p.n := 1
    let _y := x

  spec put where
    pragma opaque
    aborts_if !exists<Pending>(@0x1)
    ensures global<Pending>(@0x1).n == 1
    modifies global<Pending>(@0x1)

  fun via(s : u64, x : u64) -> Unit := do
    check(s)
    put(new Version { major := x })

  spec via where
    pragma opaque
    aborts_if s != 1
    aborts_if aborts_of<function[Fn(Version) -> Unit](put::<Version>)>(
        new Version { major := x }
      )
    ensures ensures_of<function[Fn(Version) -> Unit](put::<Version>)>(
        new Version { major := x }
      )
    modifies global<Pending>(@0x1)

  /--
  A literal without type arguments.
  -/
  fun guarded(s : u64, x : u64) -> Unit := do
    check(s)
    put(new Version { major := x })

  spec guarded where
    pragma opaque
    aborts_if aborts_of<function[Fn(u64) -> Unit](check)>(s)
    aborts_if aborts_of<function[Fn(Version) -> Unit](put::<Version>)>(
        new Version { major := x }
      )
    ensures ensures_of<function[Fn(Version) -> Unit](put::<Version>)>(
        new Version { major := x }
      )
    modifies global<Pending>(@0x1)

  /--
  Targets a call inlines, their aborts stated exactly.
  -/
  fun positive {T has Drop}(v : u64, x : T) -> T := do
    assert!(v > 0, 1)
    x

  spec positive where
    aborts_if v == 0
    ensures result == x

  fun inlined_generic(v : u64, x : u64) -> u64 := do
    let Version { major := major } := positive(v, new Version { major := x })
    major

  spec inlined_generic where
    aborts_if aborts_of<function[Fn(u64, Version) -> Version](positive::<Version>)>(
        v, new Version { major := x }
      )
    ensures result == x

  fun nonzero(v : u64) -> u64 := do
    assert!(v > 0, 1)
    v

  spec nonzero where
    aborts_if v == 0
    ensures result == v

  fun inlined_literal(v : u64) -> u64 := nonzero(v)

  spec inlined_literal where
    aborts_if aborts_of<function[Fn(u64) -> u64](nonzero)>(v)
    ensures ensures_of<function[Fn(u64) -> u64](nonzero)>(v, result)

  /--
  Through the contract of `via`, the target's contract.
  -/
  fun caller(s : u64, x : u64) -> Unit := do
    via(s, x)

  spec caller where
    aborts_if s != 1
    aborts_if !exists<Pending>(@0x1)
    ensures global<Pending>(@0x1).n == 1
    modifies global<Pending>(@0x1), *

  fun caller_guarded(s : u64, x : u64) -> Unit := do
    guarded(s, x)

  spec caller_guarded where
    aborts_if s != 1
    aborts_if !exists<Pending>(@0x1)
    ensures global<Pending>(@0x1).n == 1
    modifies global<Pending>(@0x1), *

  fun other_argument(x : u64) -> Unit := do
    put(new Version { major := x })

  spec other_argument where
    pragma opaque
    aborts_if aborts_of<function[Fn(Version) -> Unit](put::<Version>)>(
        new Version { major := x }
      )
    ensures global<Pending>(@0x1).n == 0
    modifies global<Pending>(@0x1)

  -- error: the target's contract states 1
