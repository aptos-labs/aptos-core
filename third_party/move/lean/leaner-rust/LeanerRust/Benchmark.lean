-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

/-!
# Rust pipeline benchmark support

Set `LEANER_RUST_BENCHMARK=1` to emit machine-readable phase timings. During
`#guard_msgs` tests, also set `LEANER_RUST_BENCHMARK_OUTPUT` to a log path so
the diagnostics guard does not intentionally capture the benchmark stream:

    LEANER_RUST_BENCH|<phase>|<elapsed-nanoseconds>|<label>

The helper is deliberately inert during ordinary builds and tests. Keeping
the timing at the phase boundaries makes a warm test run distinguish exporter,
source, validation, and execution costs from Lean's own compilation time.
-/

namespace LeanerIR.Rust.Benchmark

private def cleanField (value : String) : String :=
  value.replace "|" "/" |>.replace "\n" " " |>.replace "\r" " "

def enabled : IO Bool := do
  match ← IO.getEnv "LEANER_RUST_BENCHMARK" with
  | none => pure false
  | some value => pure (value != "" && value != "0" && value != "false")

def record (phase label : String) (elapsedNanos : Nat) : IO Unit := do
  if ← enabled then
    let line := s!"LEANER_RUST_BENCH|{cleanField phase}|{elapsedNanos}|{cleanField label}\n"
    match ← IO.getEnv "LEANER_RUST_BENCHMARK_OUTPUT" with
    | some path =>
        let handle ← IO.FS.Handle.mk (System.FilePath.mk path) .append
        handle.putStr line
        handle.flush
    | none => IO.eprint line

def measure (phase label : String) (action : IO α) : IO α := do
  if !(← enabled) then
    return ← action
  let start ← IO.monoNanosNow
  try
    let result ← action
    record phase label ((← IO.monoNanosNow) - start)
    pure result
  catch error =>
    record (phase ++ ".failed") label ((← IO.monoNanosNow) - start)
    throw error

/-- Measure a pure `Except` phase. Pattern matching the result before recording
the endpoint forces the pure computation instead of timing only construction
of a suspended value. -/
def measureExcept (phase label : String) (action : Unit → Except ε α) :
    IO (Except ε α) := do
  if !(← enabled) then
    return action ()
  let start ← IO.monoNanosNow
  match action () with
  | .ok value =>
      record phase label ((← IO.monoNanosNow) - start)
      pure (.ok value)
  | .error error =>
      record phase label ((← IO.monoNanosNow) - start)
      pure (.error error)

end LeanerIR.Rust.Benchmark
