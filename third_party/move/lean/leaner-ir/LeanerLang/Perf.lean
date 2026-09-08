-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean

/-!
# Verification cost measurement

Verification automation is only useful while it is fast, so every `verify`
records what its proof cost and a benchmark file compares those costs to a
checked-in baseline.

Two of the three recorded numbers are machine independent and gate the
comparison:

* `heartbeats` is the elaborator's own work counter — the same source and
  the same toolchain spend the same heartbeats — and measures **search**:
  how hard the tactics worked.
* `objects` counts the distinct subterms of the produced proof, sharing
  respected, and measures **term size**: how large the goals the tactics
  carried were.

Wall time is reported but never gates: it is the number a reader wants and
the number a machine cannot reproduce.

Separating the two gated numbers is what makes a regression actionable.
Term growth with flat search says the goals got bigger (the closing passes
walk more); search growth with flat terms says a rewrite stopped firing and
something is being re-derived.
-/

namespace LeanerLang.Perf

open Lean

/-- One measured verification target. -/
structure Sample where
  /-- `namespace::function` of the verified target. -/
  target : String
  /-- Elaborator work counter spent proving it. -/
  heartbeats : Nat
  /-- Distinct subterms of the proof, sharing respected. -/
  objects : Nat
  /-- Wall time in milliseconds.  Reported, never compared. -/
  elapsedMs : Nat
  deriving Repr, Inhabited, BEq

/-- Samples recorded while elaborating the current file.  A benchmark file
elaborates in one process, so a plain reference is the whole store. -/
initialize recorded : IO.Ref (Array Sample) ← IO.mkRef #[]

/-- Whether measurement is on.  Only the benchmark file turns it on, so an
ordinary `verify` pays nothing beyond the flag read. -/
initialize measuring : IO.Ref Bool ← IO.mkRef false

/-- Measure one verification target while it elaborates. -/
def measure [Monad m] [MonadLiftT BaseIO m] [MonadLiftT IO m] [MonadEnv m]
    (target : String) (theoremName : Name) (elaborate : m Unit) : m Unit := do
  unless ← (measuring.get : IO Bool) do
    elaborate
    return
  let startTime ← (IO.monoNanosNow : BaseIO Nat)
  let startHeartbeats ← (IO.getNumHeartbeats : BaseIO Nat)
  elaborate
  let stopHeartbeats ← (IO.getNumHeartbeats : BaseIO Nat)
  let stopTime ← (IO.monoNanosNow : BaseIO Nat)
  /- A theorem hands out its proof only to a caller that asks for opaque
  values; the proof term is exactly what this measures. -/
  let objects ← match (← getEnv).find? theoremName with
    | some info => match info.value? (allowOpaque := true) with
        | some value => (value.numObjs : IO Nat)
        | none => pure 0
    | none => pure 0
  (recorded.modify (·.push {
      target
      heartbeats := stopHeartbeats - startHeartbeats
      objects
      elapsedMs := (stopTime - startTime) / 1000000 }) : IO Unit)

/-- Render the recorded samples as the baseline text: one target per line,
sorted, carrying only the reproducible numbers. -/
def baselineText (samples : Array Sample) : String :=
  let sorted := samples.qsort (·.target < ·.target)
  sorted.foldl (init := "") fun text sample =>
    text ++ s!"{sample.target} {sample.heartbeats} {sample.objects}\n"

/-- Parse a baseline line back into its two gated numbers.  The numbers are
the last two fields, so a target name may carry spaces. -/
private def parseLine (line : String) : Option (String × Nat × Nat) := do
  let parts := line.splitOn " " |>.filter (!·.isEmpty)
  let objects ← parts[parts.length - 1]?
  let heartbeats ← parts[parts.length - 2]?
  let target := parts.take (parts.length - 2)
  if target.isEmpty then none else
  some (String.intercalate " " target, ← heartbeats.toNat?, ← objects.toNat?)

/-- The recorded baseline, by target. -/
def parseBaseline (text : String) : Array (String × Nat × Nat) :=
  text.splitOn "\n" |>.filterMap parseLine |>.toArray

/-- Growth of `current` over `previous`, in whole percent. -/
private def growthPercent (previous current : Nat) : Int :=
  if previous == 0 then (if current == 0 then 0 else 100)
  else (((current : Int) - (previous : Int)) * 100).tdiv (previous : Int)

/-- What a regression looks like, phrased so the next step is obvious.
Search and term size fail for different reasons and are fixed in different
places, so the report names which one moved. -/
private def diagnosis (heartbeatGrowth objectGrowth : Int) : String :=
  if objectGrowth ≥ heartbeatGrowth && objectGrowth > 0 then
    "term growth: the goals carried more, so the closing passes walk more \
     — look for a frame or state record that stopped being consumed"
  else if heartbeatGrowth > 0 then
    "search growth with flatter terms: a rewrite stopped firing and the \
     result is being re-derived — look for a closed equation whose shape \
     no longer matches"
  else "improvement"

/-- Compare the recorded samples with a baseline, or write a new one.
The tolerance is generous enough to absorb a shared-subterm count that
shifts with an unrelated library change and tight enough that a real
regression cannot hide behind it. -/
def report (samples : Array Sample) (baseline : Array (String × Nat × Nat))
    (tolerancePercent : Nat) : String × Bool :=
  let sorted := samples.qsort (·.target < ·.target)
  let (rows, failures) := sorted.foldl (init := (#[], #[])) fun (rows, failures) sample =>
    match baseline.find? (·.1 == sample.target) with
    | none =>
        (rows.push s!"  {sample.target}: new target — \
          {sample.heartbeats} heartbeats, {sample.objects} objects, \
          {sample.elapsedMs}ms", failures)
    | some (_, heartbeats, objects) =>
        let heartbeatGrowth := growthPercent heartbeats sample.heartbeats
        let objectGrowth := growthPercent objects sample.objects
        let row := s!"  {sample.target}: heartbeats {heartbeats} → \
          {sample.heartbeats} ({heartbeatGrowth}%), objects {objects} → \
          {sample.objects} ({objectGrowth}%), {sample.elapsedMs}ms"
        if heartbeatGrowth > tolerancePercent || objectGrowth > tolerancePercent then
          (rows.push row,
            failures.push s!"  {sample.target}: \
              {diagnosis heartbeatGrowth objectGrowth}")
        else (rows.push row, failures)
  let missing := baseline.filterMap fun (target, _, _) =>
    if sorted.any (·.target == target) then none
    else some s!"  {target}: no longer measured"
  let text := String.intercalate "\n"
    ((rows ++ missing).toList)
  if failures.isEmpty then (text, true)
  else
    (text ++ "\n\nregressed beyond " ++ toString tolerancePercent ++ "%:\n" ++
      String.intercalate "\n" failures.toList ++
      "\n\nFor a phase breakdown of one target, elaborate the benchmark with \
       the profiler, whose own counting inflates heartbeats and must never be \
       compared with the baseline:\n  \
       (cd leaner-ir && lake env lean -Dprofiler=true \
       LeanerLang/Tests/Performance.lean)\n\
       Accept a deliberate change with `lake build && UB=1 lake env lean \
       LeanerLang/Tests/Performance.lean`.  The build is part of the command: \
       `lake env lean` only sets the module path, so regenerating without it \
       records the cost of whatever imports happen to be built.", false)

end LeanerLang.Perf
