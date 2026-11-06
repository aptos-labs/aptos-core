-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Model.AbsInt
import Model.Abilities
import Model.StackUsage

/-!
# Locals safety

UNTRUSTED. Transcribes `move-bytecode-verifier/src/locals_safety/`: each
local is available, unavailable, or maybe available; parameters start
available and other locals unavailable; `CopyLoc`, `MoveLoc` and the borrows
need an available local; `StLoc` and `Ret` may not discard a value without
`drop`. The analysis runs on the abstract interpreter.
-/

namespace MoveBytecode.Model

inductive LocalState where
  | unavailable
  | maybeAvailable
  | available
  deriving DecidableEq, Repr, Inhabited

structure LocalsState where
  abilities : Array AbilitySet
  states : Array LocalState
  deriving Repr, Inhabited

/-- `AbstractState::new`. -/
def LocalsState.new (m : CompiledModule) (view : FunctionView) : VResult LocalsState := do
  let numArgs := view.parameters.length
  let n := numArgs + view.locals.length
  let states := (List.range n).map (fun i => if i < numArgs then LocalState.available
    else LocalState.unavailable)
  let abs ← (view.parameters ++ view.locals).mapM (Model.abilities m view.handle.typeParameters)
  pure ⟨abs.toArray, states.toArray⟩

def joinLocal : LocalState → LocalState → LocalState
  | .unavailable, .unavailable => .unavailable
  | .available, .available => .available
  | _, _ => .maybeAvailable

/-- `AbstractDomain::join`. -/
def LocalsState.join (a b : LocalsState) : VResult (LocalsState × Bool) :=
  if a.states.size != b.states.size || a.abilities.size != b.abilities.size then
    panicAt "join: mismatched local counts"
  else
    let joined := (a.states.toList.zip b.states.toList).map (fun (x, y) => joinLocal x y)
    if joined == a.states.toList then pure (a, false)
    else pure ({ a with states := joined.toArray }, true)

/-- `execute_inner`. -/
def LocalsState.execute (st : LocalsState) (instr : Bytecode) (_offset _blockEnd : Nat) :
    VResult LocalsState := do
  let stateAt (i : Nat) := lookup st.states i "local_states[idx]"
  let abilitiesAt (i : Nat) := lookup st.abilities i "all_local_abilities[idx]"
  match instr with
  | .stLoc i => do
    match ← stateAt i with
    | .maybeAvailable | .available =>
      if !(← abilitiesAt i).drop then reject "STLOC_UNSAFE_TO_DESTROY_ERROR"
      else pure { st with states := st.states.set! i .available }
    | .unavailable => pure { st with states := st.states.set! i .available }
  | .moveLoc i => do
    match ← stateAt i with
    | .maybeAvailable | .unavailable => reject "MOVELOC_UNAVAILABLE_ERROR"
    | .available => pure { st with states := st.states.set! i .unavailable }
  | .copyLoc i => do
    match ← stateAt i with
    | .maybeAvailable | .unavailable => reject "COPYLOC_UNAVAILABLE_ERROR"
    | .available => pure st
  | .mutBorrowLoc i | .immBorrowLoc i => do
    match ← stateAt i with
    | .maybeAvailable | .unavailable => reject "BORROWLOC_UNAVAILABLE_ERROR"
    | .available => pure st
  | .ret => do
    if st.states.size != st.abilities.size then panicAt "ret: mismatched local counts"
    for (s, a) in st.states.toList.zip st.abilities.toList do
      match s with
      | .maybeAvailable | .available =>
        if !a.drop then reject "UNSAFE_RET_UNUSED_VALUES_WITHOUT_DROP"
      | .unavailable => pure ()
    pure st
  | _ => pure st

def localsAnalysis : Analysis LocalsState :=
  { join := LocalsState.join, execute := LocalsState.execute }

/-- `locals_safety::verify`, returning the invariant map. Each local can
change at most twice along an ascending chain. -/
def verifyLocalsSafety (m : CompiledModule) (view : FunctionView) :
    VResult (List (Nat × LocalsState)) := do
  let initial ← LocalsState.new m view
  analyzeFunction localsAnalysis view.cfg view.code.code initial (2 * initial.states.size)

end MoveBytecode.Model
