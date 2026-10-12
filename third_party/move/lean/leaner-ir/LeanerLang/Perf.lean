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

/-- The result of a measured verification attempt, including rejected targets.
The benchmark uses the error count to distinguish target rejections from
unrelated frontend or elaboration errors. -/
structure Outcome where
  target : String
  status : String
  errors : Nat
  deriving ToJson

initialize outcomes : IO.Ref (Array Outcome) ← IO.mkRef #[]

/-- Whether a sample counts the objects of its proof. The verification
benchmark turns it off: it compares time and heartbeats, and the count
walks every proof term. -/
initialize countingObjects : IO.Ref Bool ← IO.mkRef true

/-- Measure all artifacts produced by one stage. Stages sharing a target
are accumulated, so moving proof work into generated declarations cannot
make it disappear from the existing verification-cost gate. -/
def measureArtifacts [Monad m] [MonadLiftT BaseIO m] [MonadLiftT IO m] [MonadEnv m]
    (target : String) (names : Array Name) (elaborate : m Unit) : m Unit := do
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
  let mut objects := 0
  if ← (countingObjects.get : IO Bool) then
    for name in names do
      if let some value := (← getEnv).find? name |>.bind (·.value? (allowOpaque := true)) then
        objects := objects + (← (value.numObjs : IO Nat))
  let sample : Sample := {
      target
      heartbeats := stopHeartbeats - startHeartbeats
      objects
      elapsedMs := (stopTime - startTime) / 1000000 }
  (recorded.modify fun samples =>
    match samples.findIdx? (·.target == target) with
    | none => samples.push sample
    | some index => samples.modify index fun previous => {
        target
        heartbeats := previous.heartbeats + sample.heartbeats
        objects := previous.objects + sample.objects
        elapsedMs := previous.elapsedMs + sample.elapsedMs } : IO Unit)

/-- Measure one verification target while it elaborates. -/
def measure [Monad m] [MonadLiftT BaseIO m] [MonadLiftT IO m] [MonadEnv m]
    (target : String) (theoremName : Name) (elaborate : m Unit) : m Unit :=
  measureArtifacts target #[theoremName] elaborate

/-! ## Phases

A verification run reports the wall time it spends in each phase, as the
Move Prover reports its build, transformation, and solving times. Phases
nest, and each moment is charged to the innermost phase running, so the
phases add up to at most the run's total. -/

/-- A phase of a verification run. -/
inductive Phase where
  /-- Importing the Lean environment the rendering elaborates in. -/
  | load
  /-- From the Move or Rust sources to their validated unit. -/
  | frontend
  /-- Rendering the unit as LeanerLang. -/
  | render
  /-- Lowering the rendered modules to validated, linked units. -/
  | lowering
  /-- The definitions and kernel-checked certificates the proofs use: the
  unit, its semantics, compiled bodies, and contracts. -/
  | certification
  /-- The proofs, automatic and authored. -/
  | verification
  deriving BEq, Inhabited

def Phase.all : Array Phase :=
  #[.load, .frontend, .render, .lowering, .certification, .verification]

def Phase.name : Phase → String
  | .load => "load"
  | .frontend => "frontend"
  | .render => "render"
  | .lowering => "lowering"
  | .certification => "certification"
  | .verification => "verification"

private def Phase.index : Phase → Nat
  | .load => 0
  | .frontend => 1
  | .render => 2
  | .lowering => 3
  | .certification => 4
  | .verification => 5

/-- The nanoseconds and heartbeats charged to each phase, the phases
running, innermost first, and the clock, heartbeat count, and thread when
the innermost was last charged. Heartbeats count per thread, and Lean
elaborates commands on threads of its own, so an interval that ends on
another thread than it began is charged its time only. -/
structure PhaseClock where
  totals : Array Nat := Phase.all.map fun _ => 0
  heartbeats : Array Nat := Phase.all.map fun _ => 0
  running : List Phase := []
  since : Nat := 0
  sinceHeartbeats : Nat := 0
  sinceThread : UInt64 := 0
  deriving Inhabited

initialize phaseClock : IO.Ref PhaseClock ← IO.mkRef {}

/-- Charge the time and heartbeats since the last charge to the innermost
running phase. -/
private def PhaseClock.charge (clock : PhaseClock) (now beats : Nat) (thread : UInt64) :
    PhaseClock :=
  let clock := match clock.running with
    | phase :: _ =>
        let spent := if thread == clock.sinceThread then beats - clock.sinceHeartbeats else 0
        { clock with
          totals := clock.totals.modify phase.index (· + (now - clock.since))
          heartbeats := clock.heartbeats.modify phase.index (· + spent) }
    | [] => clock
  { clock with since := now, sinceHeartbeats := beats, sinceThread := thread }

/-- Run `action` in `phase`. -/
def withPhase [Monad m] [MonadLiftT BaseIO m] [MonadFinally m] (phase : Phase)
    (action : m α) : m α := do
  let entered ← (IO.monoNanosNow : BaseIO Nat)
  let enteredBeats ← (IO.getNumHeartbeats : BaseIO Nat)
  let enteredThread ← (IO.getTID : BaseIO UInt64)
  (phaseClock.modify fun clock =>
    let clock := clock.charge entered enteredBeats enteredThread
    { clock with running := phase :: clock.running } : BaseIO Unit)
  try action
  finally
    let left ← (IO.monoNanosNow : BaseIO Nat)
    let leftBeats ← (IO.getNumHeartbeats : BaseIO Nat)
    let leftThread ← (IO.getTID : BaseIO UInt64)
    (phaseClock.modify fun clock =>
      let clock := clock.charge left leftBeats leftThread
      { clock with running := clock.running.drop 1 } : BaseIO Unit)

/-- The nanoseconds and heartbeats charged to each phase. -/
def phaseTotals : BaseIO (Array (Phase × Nat × Nat)) := do
  let clock ← phaseClock.get
  return Phase.all.map fun phase =>
    (phase, clock.totals[phase.index]!, clock.heartbeats[phase.index]!)

/-- Seconds, to two decimals. -/
private def seconds (nanos : Nat) : String :=
  let centis := (nanos + 5000000) / 10000000
  let fraction := toString (centis % 100)
  s!"{centis / 100}.{if fraction.length < 2 then "0" ++ fraction else fraction}s"

/-- The time charged to each phase and a run's `total` nanoseconds, as the
Move Prover reports its own: `0.81s load, …, 5.30s verification, total
9.71s`. -/
def phaseSummary (total : Nat) : BaseIO String := do
  let clock ← phaseClock.get
  let phases := Phase.all.map fun phase => s!"{seconds clock.totals[phase.index]!} {phase.name}"
  return ", ".intercalate (phases.push s!"total {seconds total}").toList

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
