import Move
import Tests.Common
open Move
open scoped Move Move.Compiler Move.Spec

move_module GlobalInv where
  struct Counter where
    value : U64
    deriving Key

  spec global where
    invariant Counter: 0 < this.value.toNat

-- The generated per-family body predicate reads as written.
example (c : GlobalInv.Counter) :
    GlobalInv.GlobalInvariant_Counter c ↔ 0 < c.value.toNat := Iff.rfl

-- And the family is registered.
open Lean Elab Command in
run_cmd do
  match Move.globalInvariant? (← getEnv) `GlobalInv.Counter with
  | some body => logInfo m!"registered: {body}"
  | none => throwError "not registered"
