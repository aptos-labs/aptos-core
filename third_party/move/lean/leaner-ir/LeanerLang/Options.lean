-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean

/-- The heartbeat budget of a generated verification theorem, in the same
thousands-of-heartbeats units as `maxHeartbeats`. A target that does not
verify fails within it rather than searching without bound; the default
is the check driver's cap, which every checked target meets, and a target
that needs more raises it for itself. -/
register_option leaner.verifyHeartbeats : Nat := {
  defValue := 1500000
  descr := "heartbeat budget of the automatic verification of one function (thousands); \
    about a minute of elaboration, past which a proof is asked for"
}

register_option leaner.stageLog : String := {
  defValue := ""
  descr := "path of a file the verifier appends a timestamped line to at every stage of a \
    module and of a target, for timing a run whose messages arrive only at its end"
}

/-- Append a timestamped line to the stage log `leaner.stageLog` names, when it
names one: the messages of a command arrive only when it ends, so a run
that cannot be waited for is timed by its stages. -/
def stageLog [Monad m] [Lean.MonadOptions m] [MonadLiftT IO m] (message : String) : m Unit := do
  let path := leaner.stageLog.get (← Lean.getOptions)
  if path.isEmpty then return
  let now ← (IO.monoMsNow : IO Nat)
  (IO.FS.withFile path .append fun handle => handle.putStrLn s!"{now} {message}" : IO Unit)
