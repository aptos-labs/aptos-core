-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.Shapes.Support

/-!
# Controls of the matrix's own checks

`expectOpen` and `expectResidual` are green only for the rejection they
state. Each control below is a use that must be refused: a cell that
verifies, a cell that fails with another diagnostic than the recorded one, a
target that does not exist, a verification that runs out of its budget, and
a failure that names a clause where a residual obligation is expected. The
last two commands are the uses that hold.
-/

namespace VerexSL.Tests.Shapes.SupportControls

set_option Elab.async false

verex module 0x42::shapes_support_controls where
  pragma verify = false

  fun holds(x : u64) -> u64 := x
  spec holds where
    aborts_if false
    ensures result == x

  fun fails(x : u64) -> u64 := x
  spec fails where
    aborts_if false
    ensures result == x + 1

  spec fun unfold(n : Int) : Int := if n <= 0 then 0 else 1 + unfold(n - 1)

  fun expensive(x : u64) -> u64 := x
  spec expensive where
    aborts_if false
    ensures unfold(40) == 40

/--
error: the open cell `«0x42».shapes_support_controls.holds` is established now: move it up to the positive controls
-/
#guard_msgs in
run_cmd do
  expectOpen `«0x42».shapes_support_controls.holds
    "the specification clause `ensures result == x` is not established"
    (← `(command| verify 0x42::shapes_support_controls::holds))

/--
error: the open cell `«0x42».shapes_support_controls.fails` fails otherwise than recorded: expected first
  the specification clause `aborts_if false` is not established
found [the specification clause `ensures result == x + 1` is not established]
-/
#guard_msgs in
run_cmd do
  expectOpen `«0x42».shapes_support_controls.fails
    "the specification clause `aborts_if false` is not established"
    (← `(command| verify 0x42::shapes_support_controls::fails))

/--
error: the attempt of `«0x42».shapes_support_controls.missing` reports an error that is not a verification failure of it: unknown function `missing` in `«0x42».shapes_support_controls`
-/
#guard_msgs in
run_cmd do
  expectOpen `«0x42».shapes_support_controls.missing
    "the specification clause `ensures result == x` is not established"
    (← `(command| verify 0x42::shapes_support_controls::missing))

/--
error: expected residual obligations of `«0x42».shapes_support_controls.fails` and no other report, found [the specification clause `ensures result == x + 1` is not established]
-/
#guard_msgs in
run_cmd do
  expectResidual `«0x42».shapes_support_controls.fails
    (← `(command| verify 0x42::shapes_support_controls::fails))

/--
error: the verification of `«0x42».shapes_support_controls.expensive` ran out of a budget
-/
#guard_msgs in
set_option verex.verifyHeartbeats 1 in
run_cmd do
  expectOpen `«0x42».shapes_support_controls.expensive
    "the specification clause `ensures unfold(40) == 40` is not established"
    (← `(command| verify 0x42::shapes_support_controls::expensive))

run_cmd do
  expectOpen `«0x42».shapes_support_controls.fails
    "the specification clause `ensures result == x + 1` is not established"
    (← `(command| verify 0x42::shapes_support_controls::fails))

end VerexSL.Tests.Shapes.SupportControls
