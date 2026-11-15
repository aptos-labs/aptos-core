-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

/-!
# The driver's worker pool

A suite whose fixtures each need a whole process - a `lean` elaborating a
check, a child of this driver verifying a source - runs them several at a
time from one queue. The pool is shared: the number of workers and the
threads each of them plans for itself are one policy, and `runQueue` is the
only queue in the driver.

A suite also keeps a standing record of the fixtures that are known not to
pass, which it starts its queue with and which names what to rerun. The
record is read, merged and written here, once for every suite that keeps one.

A fixture's process runs under a timeout, so that one target that stops
making progress costs its own fixture rather than the run.
-/

namespace VerexE2ETests.Workers

/-- Memory a worker is planned with, in gigabytes: the largest peak of a Lean
fixture process is near two. -/
private def memoryPerWorkerGB : Nat := 3

/-- Half the hardware threads: the share of the machine a suite plans for,
leaving the other half to whatever else runs on it. -/
private def plannedThreads : Nat :=
  max 1 ((System.Platform.Internal.getHardwareConcurrency ()).toNat / 2)

/-- `MemAvailable` of `/proc/meminfo` in gigabytes, where that file is
readable. -/
private def availableMemoryGB : IO (Option Nat) := do
  let contents ← try pure (some (← IO.FS.readFile "/proc/meminfo")) catch _ => pure none
  let some contents := contents | return none
  for line in contents.splitOn "\n" do
    if line.startsWith "MemAvailable:" then
      let fields := (line.splitOn " ").filter (!·.isEmpty)
      if let some kilobytes := fields[1]?.bind (·.toNat?) then
        return some (kilobytes / 1048576)
  return none

/-- The number of workers a suite runs at once: `VEREX_E2E_JOBS` where it is
set, else half the hardware threads, bounded by one worker per
`memoryPerWorkerGB` of available memory. At least one. -/
def workerCount : IO Nat := do
  if let some value ← IO.getEnv "VEREX_E2E_JOBS" then
    match value.toNat? with
    | some count => if count > 0 then return count
    | none => pure ()
    throw <| IO.userError s!"VEREX_E2E_JOBS must be a positive number, not `{value}`"
  let mut workers := plannedThreads
  if let some available ← availableMemoryGB then
    workers := min workers (available / memoryPerWorkerGB)
  return max 1 workers

/-- The seconds a fixture's process may run where `VEREX_E2E_TIMEOUT` says
nothing: more than the longest fixture of a healthy tree takes. -/
private def defaultTimeout : Nat := 240

/-- How often the wait for a running fixture looks at whether it has ended,
in milliseconds. -/
private def pollInterval : UInt32 := 50

/-- The seconds a fixture's process may run: `VEREX_E2E_TIMEOUT` where it is
set, else `defaultTimeout`, and no timeout at all where it is `0`. -/
def timeoutSeconds : IO Nat := do
  let some value ← IO.getEnv "VEREX_E2E_TIMEOUT" | return defaultTimeout
  let some seconds := value.toNat?
    | throw <| IO.userError
        s!"VEREX_E2E_TIMEOUT must be a number of seconds, not `{value}`"
  return seconds

/-- What running a fixture's process came to. -/
inductive Run where
  /-- It ended on its own, with this exit code and this captured output. -/
  | ended (exitCode : UInt32) (output : String)
  /-- It was killed at the expiry of this many seconds. -/
  | timedOut (seconds : Nat)
  deriving Inhabited

/-- Run a command with its output captured, under `seconds` of runtime, or
without a limit when that is `0`. The driver enforces the limit itself, with
no tool of its own: the command runs in a session of its own, so that
`Child.kill` at its exit or at the deadline signals its whole process group and
a `lake` that started a `lean`, or a driver that started a child, goes with it.
Both pipes are read to their end in tasks of their own while the command runs,
so a command that prints more than a pipe holds never blocks on one, and the
wait polls instead of blocking past the deadline. -/
def runUnderTimeout (seconds : Nat) (args : IO.Process.SpawnArgs) : IO Run := do
  if seconds == 0 then
    let result ← IO.Process.output args
    return .ended result.exitCode (result.stdout ++ result.stderr)
  let child ← IO.Process.spawn { args with
    stdin := .null, stdout := .piped, stderr := .piped, setsid := true }
  let stdout ← IO.asTask child.stdout.readToEnd Task.Priority.dedicated
  let stderr ← IO.asTask child.stderr.readToEnd Task.Priority.dedicated
  let deadline := (← IO.monoMsNow) + seconds * 1000
  let mut ended : Option UInt32 := none
  repeat
    ended := ← child.tryWait
    if ended.isSome then break
    if (← IO.monoMsNow) ≥ deadline then break
    IO.sleep pollInterval
  let some exitCode := ended
    | child.kill
      discard child.wait
      -- The readers end as the pipes close; their outcome is not a result.
      discard <| IO.wait stdout
      discard <| IO.wait stderr
      return .timedOut seconds
  -- End whatever the fixture left in its session before waiting on the pipes a
  -- background descendant may still hold open; an already-empty group gives
  -- `ESRCH` (`errno` 3).
  try
    child.kill
  catch
  | .noSuchThing _ 3 _ => pure ()
  | error => throw error
  let out ← IO.ofExcept stdout.get
  let err ← IO.ofExcept stderr.get
  return .ended exitCode (out ++ err)

/-- Check that `runUnderTimeout` ends the session's remaining processes when
the direct child exits, so a background descendant holding the pipes open
neither stalls the readers nor survives the run. -/
def testProcessGroupCleanup : IO Unit := do
  let started ← IO.monoMsNow
  let outcome ← runUnderTimeout 10 {
    cmd := "sh"
    args := #["-c", "sleep 30 &\necho $!\nexit 0"]
    setsid := true
    stdin := .null
    stdout := .piped
    stderr := .piped }
  let elapsed := (← IO.monoMsNow) - started
  let .ended exitCode output := outcome
    | throw <| IO.userError
        "expected `runUnderTimeout` to end when the direct child exited, not time out"
  unless exitCode == 0 do
    throw <| IO.userError s!"expected exit code 0, got {exitCode}"
  unless elapsed < 5000 do
    throw <| IO.userError
      s!"expected `runUnderTimeout` to return without waiting on `sleep 30`, took {elapsed}ms"
  let pidText := output.trimAscii.toString
  let some pid := pidText.toNat?
    | throw <| IO.userError s!"expected background pid on stdout, got `{output}`"
  if ← ("/proc" : System.FilePath).pathExists then
    let procEntry : System.FilePath := s!"/proc/{pid}"
    for _ in [:10] do
      unless ← procEntry.pathExists do break
      IO.sleep pollInterval
    if ← procEntry.pathExists then
      throw <| IO.userError s!"background process {pid} survived `runUnderTimeout`"
  else
    for _ in [:10] do
      if (← IO.Process.output { cmd := "kill", args := #["-0", toString pid] }).exitCode != 0 then
        break
      IO.sleep pollInterval
    if (← IO.Process.output { cmd := "kill", args := #["-0", toString pid] }).exitCode == 0 then
      throw <| IO.userError s!"background process {pid} survived `runUnderTimeout`"

/-- The threads one worker plans for itself, for the `verex.jobs` under which
it proves the targets of its fixture: the planned threads shared among the
workers, at least one. -/
def targetJobs (workers : Nat) : Nat :=
  max 1 (plannedThreads / max 1 workers)

/-- Run `work` on each of the `count` indices, `workers` of them at a time in
dedicated tasks from one queue that hands them out in the order `order` gives,
and return every result at its own index. `finished` is called with an index
and its result as that item ends, in the task that ran it. -/
def runQueue [Inhabited α] (count workers : Nat) (order : Array Nat)
    (work : Nat → IO α) (finished : Nat → α → IO Unit := fun _ _ => pure ()) :
    IO (Array α) := do
  let next ← IO.mkRef 0
  let workers := max 1 (min workers count)
  let tasks ← (List.range workers).toArray.mapM fun _ =>
    IO.asTask (prio := .dedicated) do
      let mut done : Array (Nat × α) := #[]
      repeat
        let position ← next.modifyGet fun position => (position, position + 1)
        let some index := order[position]? | break
        let result ← work index
        finished index result
        done := done.push (index, result)
      return done
  let mut results := Array.replicate count default
  for task in tasks do
    match ← IO.wait task with
    | .ok done => for (index, result) in done do results := results.set! index result
    | .error error => throw error
  return results

/-- The names a standing failure record holds, none where it does not exist. -/
def readFailures (record : System.FilePath) : IO (Array String) := do
  unless ← record.pathExists do return #[]
  let names := (← IO.FS.readFile record).trimAscii.toString.splitOn ","
  return (names.filter (!·.isEmpty)).toArray

/-- Write a standing failure record after a run of `ran`: the recorded names
the run did not run, in their order, and the names it failed. A name the run
passed is therefore gone from the record and a name it did not run stays, so
the record always holds what is known not to pass. The names are
comma-separated on one line, so that `VEREX_E2E_FILTER=$(cat <record>)` reruns
them, and the file is empty when nothing is known to fail. -/
def writeFailures (record : System.FilePath) (recorded ran failed : Array String) :
    IO Unit := do
  if let some parent := record.parent then
    IO.FS.createDirAll parent
  let names := recorded.filter (!ran.contains ·) ++ failed
  IO.FS.writeFile record <|
    if names.isEmpty then "" else ",".intercalate names.toList ++ "\n"

end VerexE2ETests.Workers
