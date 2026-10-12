-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean

/-- The heartbeat budget of a generated verification theorem, in the same
thousands-of-heartbeats units as `maxHeartbeats`. A target that does not
verify fails within it rather than searching without bound; the default
is the check driver's cap, which every checked target meets, and a target
that needs more raises it for itself. -/
register_option verex.verifyHeartbeats : Nat := {
  defValue := 1500000
  descr := "heartbeat budget of the automatic verification of one function (thousands); \
    about a minute of elaboration, past which a proof is asked for"
}

/-- Skip the `verify` items: set, or unset in the language server, where
every change to an open file would run them again; `lake test` and the
command line check them. A `verify! f` item runs regardless; the code action
"Verify `f` here" marks one. -/
register_option verex.skip_verify : Bool := {
  defValue := false
  descr := "skip the `verify` items but the `verify!` ones; unset, skipped in the language \
    server only"
}

/-- Whether the `verify` items are skipped: `verex.skip_verify` where set;
unset, in the language server, which the command line tells apart by
`internal.cmdlineSnapshots`. A driver that elaborates in process
(`processCommands`) looks like the server and sets the option off itself. The
packages' `moreServerOptions` would set the option for the server, once Lake
passes them to library modules (leanprover/lean4#15626). -/
def skipVerify (options : Lean.Options) : Bool :=
  match options.find `verex.skip_verify with
  | some (.ofBool skip) => skip
  | _ => !Lean.internal.cmdlineSnapshots.get options

register_option verex.stageLog : String := {
  defValue := ""
  descr := "path of a file the verifier appends a timestamped line to at every stage of a \
    module and of a target, for timing a run whose messages arrive only at its end"
}

/-- Append a timestamped line to the stage log `verex.stageLog` names, when it
names one: the messages of a command arrive only when it ends, so a run
that cannot be waited for is timed by its stages. -/
def stageLog [Monad m] [Lean.MonadOptions m] [MonadLiftT IO m] (message : String) : m Unit := do
  let path := verex.stageLog.get (← Lean.getOptions)
  if path.isEmpty then return
  let now ← (IO.monoMsNow : IO Nat)
  (IO.FS.withFile path .append fun handle => handle.putStrLn s!"{now} {message}" : IO Unit)
