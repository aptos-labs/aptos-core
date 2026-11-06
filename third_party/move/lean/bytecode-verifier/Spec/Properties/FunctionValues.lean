-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Properties.Certificates
import Spec.Properties.References

/-!
# Function values D5

TRUSTED. D5 (plan §7): "Captured values are never references. A closure's
`copy`, `drop` and `store` abilities are justified by its target and every
captured value. `CallClosure` supplies exactly the remaining parameters,
conforming to their types, and receives conforming results. Only closures
over storable functions reach global storage."

It is stated over executions (`Reachable`), as D1 is, for every admissible
initial state: it needs no identities, only the closures present in each
state and the step each `CallClosure` takes.

- **D5a captures.** Every captured value of every closure, at any depth of
  any value of a state, holds no reference: "it is not possible to _capture_
  reference values" (book, `functions.md`, "Lambda Expressions and
  Closures").
- **D5b abilities.** Every closure is well formed (`Value.wellFormed`): it
  records the type of a closure over its target with its mask, the remaining
  parameters and the target's results, and its captured values conform to the
  captured parameters' types. Its abilities are justified by its target, at
  most `copy`, `drop`, and `store` for a storable target
  (`Program.closureAbilities`), and by every captured value, at most each
  one's abilities: "the function underlying a closure ... is `copy+drop`
  (and not `store`). This is intersected with the abilities of all the
  captured context variables" (book, `functions.md`).
- **D5c calls.** Every `CallClosure` that continues enters the closure's
  target, from its first instruction, with exactly its parameters: the
  captured values and the provided arguments mixed by the mask
  (`maskCompose`), conforming to the target's parameter types. It receives
  conforming results: the target's results are the closure's (D5b), and the
  target's `Ret` returns values conforming to them (D1).
- **D5d storage.** Every closure in global storage, during the execution and
  in the storage a returning execution commits, targets a storable function,
  `public` or `persistent` and not merely `entry` (`Program.storable`): "a
  _persistent_ function is required to build a storable function value"
  (book, `functions.md`, "Denoting Function Values").
-/

namespace MoveBytecode

/-- Whether a value is a closure. -/
def Value.isClosure : Value → Bool
  | .closure .. => true
  | .bool _ => false
  | .int _ _ => false
  | .address _ => false
  | .signer _ => false
  | .vector _ _ => false
  | .struct _ _ => false
  | .variant _ _ _ => false
  | .ref _ => false
  | .table .. => false

namespace FunctionValues

/-- **D5a**, **D5b** for one node: a closure captures no reference, is well
formed, and has no ability its target or a captured value lacks. -/
def closureOk (P : Program) : Value → Bool
  | v@(.closure f _ captured _ _ a) =>
    captured.all Value.referenceFree && v.wellFormed P &&
      a.subset (P.closureAbilities f) && captured.all fun c => a.subset (c.abilities P)
  | .bool _ => true
  | .int _ _ => true
  | .address _ => true
  | .signer _ => true
  | .vector _ _ => true
  | .struct _ _ => true
  | .variant _ _ _ => true
  | .ref _ => true
  | .table .. => true

/-- D5a and D5b for every node of every value of `vs`. -/
def valuesOk (P : Program) (vs : List Value) : Bool :=
  vs.all fun v => v.nodes.all fun (_, w) => closureOk P w

/-- **D5d** for global storage `st`: every closure in it, at any depth,
targets a storable function. -/
def storedOk (P : Program) (st : GlobalStorage) : Bool :=
  st.all fun r => r.value.nodes.all fun (_, w) =>
    match w with
    | .closure f .. => P.storable f
    | _ => true

/-- D5a, D5b and D5d in state `s`: its locals, operand stacks, held values and
global storage. -/
def stateOk (P : Program) (s : State) : Bool :=
  valuesOk P (s.frames.flatMap fun fr => fr.locals.toList.filterMap id ++ fr.stack) &&
    valuesOk P s.held && valuesOk P (s.storage.map (·.value)) && storedOk P s.storage

/-- D5a, D5b and D5d after a step: in the next state, or in the returned values
and the committed global storage. -/
def outcomeOk (P : Program) : Outcome → Bool
  | .running s => stateOk P s
  | .returned vs st => valuesOk P vs && valuesOk P (st.map (·.value)) && storedOk P st
  | .failed _ => true
  | .wrong _ => true

/-- **D5c** for a step from `s` with outcome `o`: when the current instruction
is `CallClosure` and the step continues, the closure on top of the segment and
the remaining arguments below it, the new frame runs the closure's target from
its first instruction, and its first locals are the captured values and the
provided arguments in parameter order, conforming to the target's parameter
types. -/
def callEntersOk (P : Program) (s : State) (o : Outcome) : Bool :=
  match s.frames with
  | [] => true
  | fr :: _ =>
    if let some (.callClosure _) := currentInstr P fr then
      match fr.stack, o with
      | .closure f mask captured ps _ _ :: rest, .running s' =>
        match s'.frames, P.paramTypes f with
        | callee :: _, some tys =>
          let args := maskCompose mask captured (rest.take ps.length).reverse
          callee.function == f && callee.pc == 0 && conformsAll P args tys &&
            callee.locals.toList.take args.length == args.map some
        | _, _ => false
      | _, .running _ => false
      | _, .returned _ _ => false
      | _, .failed _ => true
      | _, .wrong _ => true
    else true

end FunctionValues

/-- **D5.** In every state reachable from `s₀`, every closure captures no
reference and is well formed, with abilities justified by its target and its
captured values, and every closure in global storage targets a storable
function; the same holds after every step, the committed storage of a
returning step included; and every `CallClosure` that continues enters its
target with exactly its parameters, conforming. -/
def FunctionValuesSafe (P : Program) (N : NativeRel) (s₀ : State) : Prop :=
  ∀ s, Reachable P N s₀ s → FunctionValues.stateOk P s = true ∧
    ∀ o, StepRel P N s o →
      FunctionValues.outcomeOk P o = true ∧ FunctionValues.callEntersOk P s o = true

end MoveBytecode
