-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.BigStep

/-!
# Fuelled structured-LIR interpreter

The interpreter consumes only `ExecutableUnit`, so unsupported constructs
have already been rejected by semantic preparation.  It evaluates the
structured expression arena directly: loops remain loops and calls resolve
validated declaration identities rather than lowering through a CFG or a
source-language runtime.
-/

namespace LeanerIR
namespace Interpreter

open Validation
open SemanticOperations

def runtimeLocation (namespaceId : NamespaceId) (loc : LocId) : RuntimeLocation :=
  { namespaceId, loc }

def located (namespaceId : NamespaceId) (loc : LocId) (value : α) : Located α :=
  { value, primary := runtimeLocation namespaceId loc }

def failAt (namespaceId : NamespaceId) (loc : LocId) (error : InterpreterError) :
    Except LocatedInterpreterError α :=
  .error (located namespaceId loc error)

namespace Internal

/-- Internal located result used by the executable evaluator and its proof. -/
structure ExprEvaluation where
  state : RuntimeState
  frame : RuntimeFrame
  control : LocatedControl

/-- Internal located function result used by the executable evaluator and its proof. -/
structure FunctionEvaluation where
  state : RuntimeState
  outcome : LocatedOutcome

/-- Internal result for executable operand-list evaluation. -/
inductive ValuesEvaluation where
  | values (state : RuntimeState) (frame : RuntimeFrame) (values : List RuntimeValue)
  | control (state : RuntimeState) (frame : RuntimeFrame) (control : LocatedControl)

/-- Internal result for executable statement-list evaluation. -/
inductive StatementsEvaluation where
  | done (state : RuntimeState) (frame : RuntimeFrame)
  | control (state : RuntimeState) (frame : RuntimeFrame) (control : LocatedControl)

/-- Lift a completed callee outcome back into its caller expression. -/
def callResult (namespaceId : NamespaceId) (loc : LocId)
    (lexical : Option Nat) (frame : RuntimeFrame)
    (inherited : Array (Nat × RuntimeValue))
    (result : FunctionEvaluation) : ExprEvaluation :=
  let here := runtimeLocation namespaceId loc
  let (frame, state) := applyPendingFrom inherited frame result.state
  match result.outcome.value with
  | .returned values => {
      state
      frame := registerReturnedLoan lexical values frame
      control := { value := .value (packResults values), primary := here } }
  | .threw kind arguments => {
      state
      frame
      control := {
        value := .throw_ kind arguments
        primary := result.outcome.primary
        callers := result.outcome.callers.push here } }

mutual
  def evalFunction : Nat → ExecutableUnit → FunctionHandle → RuntimeState →
      Array RuntimeValue → Except LocatedInterpreterError FunctionEvaluation
    | 0, executable, handle, _, _ =>
        let loc := executable.unit.namespaces[handle.namespaceId.index]?
          |>.map (fun ns => ns.functions[handle.functionId.index]?.map (·.loc))
          |>.join |>.getD ⟨0⟩
        failAt handle.namespaceId loc .outOfFuel
    | fuel + 1, executable, handle, state, arguments => do
        let some ns := executable.unit.namespaces[handle.namespaceId.index]?
          | failAt handle.namespaceId ⟨0⟩ (.functionHasNoBody handle)
        let some declaration := ns.functions[handle.functionId.index]?
          | failAt handle.namespaceId ns.loc (.functionHasNoBody handle)
        if arguments.size != declaration.signature.parameters.size then
          failAt handle.namespaceId declaration.loc
            (.argumentArity declaration.signature.parameters.size arguments.size)
        let some frame := initialFrame? declaration arguments
          | failAt handle.namespaceId declaration.loc .unsupportedPreparedNode
        let .structured root := declaration.body
          | failAt handle.namespaceId declaration.loc (.functionHasNoBody handle)
        let evaluation ← evalExpr fuel executable handle.namespaceId frame state root
        match evaluation.control.value with
        | .value value =>
            let some values := unpackFallthrough declaration.signature.results.size value
              | failAt handle.namespaceId evaluation.control.primary.loc
                  (.resultArity declaration.signature.results.size 1)
            let outcome : Outcome := .returned values
            let finalState := finalizeFunctionState executable declaration.profile
              state evaluation.state evaluation.frame outcome
            return { state := finalState, outcome := {
              value := .returned values
              primary := evaluation.control.primary
              callers := evaluation.control.callers } }
        | .return_ values =>
            if values.size != declaration.signature.results.size then
              failAt handle.namespaceId evaluation.control.primary.loc
                (.resultArity declaration.signature.results.size values.size)
            let outcome : Outcome := .returned values
            let finalState := finalizeFunctionState executable declaration.profile
              state evaluation.state evaluation.frame outcome
            return { state := finalState, outcome := {
              value := .returned values
              primary := evaluation.control.primary
              callers := evaluation.control.callers } }
        | .throw_ kind arguments =>
            let outcome : Outcome := .threw kind arguments
            let finalState := finalizeFunctionState executable declaration.profile
              state evaluation.state evaluation.frame outcome
            return { state := finalState, outcome := {
              value := .threw kind arguments
              primary := evaluation.control.primary
              callers := evaluation.control.callers } }
        | .break_ .. | .continue_ .. =>
            failAt handle.namespaceId evaluation.control.primary.loc .escapedLoopControl

  def evalExpr : Nat → ExecutableUnit → NamespaceId → RuntimeFrame → RuntimeState →
      ExprId → Except LocatedInterpreterError ExprEvaluation
    | 0, executable, namespaceId, _, _, exprId =>
        let loc := executable.unit.namespaces[namespaceId.index]?
          |>.map (fun ns => ns.expressions[exprId.index]?.map (·.loc))
          |>.join |>.getD ⟨0⟩
        failAt namespaceId loc .outOfFuel
    | fuel + 1, executable, namespaceId, frame, state, exprId => do
        let some ns := executable.unit.namespaces[namespaceId.index]?
          | failAt namespaceId ⟨0⟩ .unsupportedPreparedNode
        let some expression := ns.expressions[exprId.index]?
          | failAt namespaceId ns.loc .unsupportedPreparedNode
        let here := runtimeLocation namespaceId expression.loc
        let normal (state : RuntimeState) (frame : RuntimeFrame) (value : RuntimeValue) :
            ExprEvaluation := { state, frame, control := { value := .value value, primary := here } }
        let propagate (evaluation : ExprEvaluation) : ExprEvaluation := evaluation
        match expression.kind with
        | .value value _ =>
            let some runtimeValue := constValue? value
              | failAt namespaceId expression.loc .unsupportedPreparedNode
            return normal state frame runtimeValue
        | .constant reference =>
            let some handle := resolveConstant? executable.unit namespaceId reference
              | failAt namespaceId expression.loc (.unknownConstant reference)
            let some targetNs := executable.unit.namespaces[handle.namespaceId.index]?
              | failAt namespaceId expression.loc .unsupportedPreparedNode
            let some declaration := targetNs.constants[handle.constantId]?
              | failAt namespaceId expression.loc .unsupportedPreparedNode
            let result ← evalExpr fuel executable handle.namespaceId
              { locals := #[] } state declaration.value
            match result.control.value with
            | .value value => return normal result.state frame value
            | _ => return { result with frame, control := result.control.pushCaller here }
        | .localVar localId =>
            let some value := readLocal? frame localId
              | failAt namespaceId expression.loc (.uninitializedLocal localId)
            return normal state frame value
        | .operation (.call (.function reference)) _ arguments _ =>
            let operands ← evalValues fuel executable namespaceId frame state arguments.toList
            match operands with
            | .control state frame control => return { state, frame, control }
            | .values state frame values =>
                let some handle := resolveFunction? executable.unit namespaceId reference
                  | failAt namespaceId expression.loc (.unknownFunction reference)
                let result ← match evalFunction fuel executable handle state values.toArray with
                  | .ok result => pure result
                  | .error error => throw (error.pushCaller here)
                return callResult namespaceId expression.loc
                  (certificateLoanId? executable.unit namespaceId exprId)
                  frame state.pending result
        | .operation (.call (.constructor reference variant)) _ arguments _ =>
            let operands ← evalValues fuel executable namespaceId frame state arguments.toList
            match operands with
            | .control state frame control => return { state, frame, control }
            | .values state frame values =>
                let some value := constructNominal? executable.unit namespaceId reference variant values.toArray
                  | failAt namespaceId expression.loc (.invalidConstructor reference variant)
                return normal state frame value
        | .operation (.call (.destructor reference variant)) _ arguments _ =>
            let operands ← evalValues fuel executable namespaceId frame state arguments.toList
            match operands with
            | .control state frame control => return { state, frame, control }
            | .values state frame [value] =>
                let some fields := destructNominal? executable.unit namespaceId reference variant value
                  | failAt namespaceId expression.loc (.invalidConstructor reference variant)
                return normal state frame (packResults fields)
            | .values _ _ _ => failAt namespaceId expression.loc (.invalidConstructor reference variant)
        | .operation (.call (.closure reference)) _ captures _ =>
            let operands ← evalValues fuel executable namespaceId frame state captures.toList
            match operands with
            | .control state frame control => return { state, frame, control }
            | .values state frame values =>
                let some handle := resolveFunction? executable.unit namespaceId reference
                  | failAt namespaceId expression.loc (.unknownFunction reference)
                return normal state frame (.closure handle values.toArray)
        | .operation (.call .invoke) _ arguments _ =>
            let operands ← evalValues fuel executable namespaceId frame state arguments.toList
            match operands with
            | .control state frame control => return { state, frame, control }
            | .values _ _ [] => failAt namespaceId expression.loc (.expectedClosure .unit)
            | .values state frame (callable :: arguments) =>
                let .closure handle captures := callable
                  | failAt namespaceId expression.loc (.expectedClosure callable)
                let result ← match evalFunction fuel executable handle state
                    (captures ++ arguments.toArray) with
                  | .ok result => pure result
                  | .error error => throw (error.pushCaller here)
                return callResult namespaceId expression.loc none
                  frame state.pending result
        | .operation (.profile operation _) _ arguments _ =>
            let operands ← evalValues fuel executable namespaceId frame state arguments.toList
            match operands with
            | .control state frame control => return { state, frame, control }
            | .values state frame values =>
                match evaluateProfileOperation? executable ns expression.typeId operation values.toArray with
                | some (.ok value) => return normal state frame value
                | some (.error (kind, thrown)) => return {
                    state, frame, control := located namespaceId expression.loc (.throw_ kind thrown) }
                | none => failAt namespaceId expression.loc (.invalidProfileOperation operation)
        | .operation (.primitive operation) _ arguments _ =>
            let operands ← evalValues fuel executable namespaceId frame state arguments.toList
            match operands with
            | .control state frame control => return { state, frame, control }
            | .values state frame values =>
                match evaluatePrimitiveOperation? ns expression.typeId operation values.toArray
                    executable.targetPointerWidth with
                | some (.ok value) => return normal state frame value
                | some (.error (kind, thrown)) => return {
                    state, frame, control := located namespaceId expression.loc (.throw_ kind thrown) }
                | none => failAt namespaceId expression.loc .unsupportedPreparedNode
        | .operation (.data operation) _ arguments _ =>
            let operands ← evalValues fuel executable namespaceId frame state arguments.toList
            match operands with
            | .control state frame control => return { state, frame, control }
            | .values state frame values =>
                match evaluateDataOperation? executable.unit ns.identity operation values.toArray with
                | some value => return normal state frame value
                | none => failAt namespaceId expression.loc (.invalidDataOperation operation)
        | .operation (.global kind) instantiations arguments _ =>
            let operands ← evalValues fuel executable namespaceId frame state arguments.toList
            match operands with
            | .control state frame control => return { state, frame, control }
            | .values state frame values =>
                match evaluateGlobalOperation? executable.unit ns expression.typeId exprId kind
                    instantiations values.toArray frame state with
                | some (.value frame state value) => return normal state frame value
                | some (.throw_ frame state kind thrown) => return {
                    state, frame, control := located namespaceId expression.loc (.throw_ kind thrown) }
                | none => failAt namespaceId expression.loc .unsupportedPreparedNode
        | .operation .assert _ arguments _ =>
            let operands ← evalValues fuel executable namespaceId frame state arguments.toList
            match operands with
            | .control state frame control => return { state, frame, control }
            | .values state frame [.bool true] => return normal state frame .unit
            | .values state frame [.bool false] => return {
                state, frame, control := located namespaceId expression.loc (.throw_ .abort #[]) }
            | .values _ _ [actual] => failAt namespaceId expression.loc (.expectedBoolean actual)
            | .values _ _ _ => failAt namespaceId expression.loc .unsupportedPreparedNode
        | .operation operation _ arguments _ =>
            let operands ← evalValues fuel executable namespaceId frame state arguments.toList
            match operands with
            | .control state frame control => return { state, frame, control }
            | .values state frame values =>
                let some (frame, state, value) := evaluatePlaceOperation? executable.unit ns
                    expression.typeId exprId operation values.toArray frame state
                  | failAt namespaceId expression.loc .unsupportedPreparedNode
                return normal state frame value
        | .block statements result =>
            let statementsResult ← evalStatements fuel executable namespaceId frame state statements.toList
            match statementsResult with
            | .control state frame control => return { state, frame, control }
            | .done state frame => match result with
              | none => return normal state frame .unit
              | some child =>
                let evaluation ← evalExpr fuel executable namespaceId frame state child
                match evaluation.control.value with
                | .value value => return normal evaluation.state evaluation.frame value
                | _ => return propagate evaluation
        | .letDecl pattern value body =>
            match value with
            | none =>
                let evaluation ← evalExpr fuel executable namespaceId frame state body
                match evaluation.control.value with
                | .value value => return normal evaluation.state evaluation.frame value
                | _ => return propagate evaluation
            | some initializer =>
                let initialized ← evalExpr fuel executable namespaceId frame state initializer
                match initialized.control.value with
                | .value value =>
                    let some bound := bindPattern executable.unit ns initialized.frame pattern value
                      | let patternLoc := ns.patterns[pattern.index]?.map (·.loc) |>.getD expression.loc
                        failAt namespaceId patternLoc .patternMismatch
                    let evaluation ← evalExpr fuel executable namespaceId bound initialized.state body
                    match evaluation.control.value with
                    | .value value => return normal evaluation.state evaluation.frame value
                    | _ => return propagate evaluation
                | _ => return propagate initialized
        | .ifElse condition thenBranch elseBranch =>
            let conditionResult ← evalExpr fuel executable namespaceId frame state condition
            match conditionResult.control.value with
            | .value (.bool true) =>
                let result ← evalExpr fuel executable namespaceId conditionResult.frame
                  conditionResult.state thenBranch
                match result.control.value with
                | .value value => return normal result.state result.frame value
                | _ => return propagate result
            | .value (.bool false) => match elseBranch with
                | none => return normal conditionResult.state conditionResult.frame .unit
                | some branch =>
                    let result ← evalExpr fuel executable namespaceId conditionResult.frame
                      conditionResult.state branch
                    match result.control.value with
                    | .value value => return normal result.state result.frame value
                    | _ => return propagate result
            | .value actual => failAt namespaceId expression.loc (.expectedBoolean actual)
            | _ => return propagate conditionResult
        | .match_ scrutinee arms =>
            let scrutineeResult ← evalExpr fuel executable namespaceId frame state scrutinee
            match scrutineeResult.control.value with
            | .value value =>
                let armResult ← evalArms fuel executable namespaceId ns expression.loc
                  scrutineeResult.frame scrutineeResult.state value arms.toList
                match armResult.control.value with
                | .value value => return normal armResult.state armResult.frame value
                | _ => return propagate armResult
            | _ => return propagate scrutineeResult
        | .loop _ body =>
            let bodyResult ← evalExpr fuel executable namespaceId frame state body
            match bodyResult.control.value with
            | .value _ | .continue_ 0 =>
                evalExpr fuel executable namespaceId bodyResult.frame bodyResult.state exprId
            | .break_ 0 value => return normal bodyResult.state bodyResult.frame (value.getD .unit)
            | .break_ (nest + 1) value => return {
                bodyResult with control := { bodyResult.control with value := .break_ nest value } }
            | .continue_ (nest + 1) => return {
                bodyResult with control := { bodyResult.control with value := .continue_ nest } }
            | .return_ _ | .throw_ .. => return propagate bodyResult
        | .break_ nest value => match value with
            | none => return { state, frame, control := located namespaceId expression.loc (.break_ nest none) }
            | some child =>
                let result ← evalExpr fuel executable namespaceId frame state child
                match result.control.value with
                | .value value => return { result with
                    control := located namespaceId expression.loc (.break_ nest (some value)) }
                | _ => return propagate result
        | .continue_ nest =>
            return { state, frame, control := located namespaceId expression.loc (.continue_ nest) }
        | .return_ values =>
            let result ← evalValues fuel executable namespaceId frame state values.toList
            match result with
            | .values state frame values =>
                return {
                  state := state
                  frame := frame
                  control := located namespaceId expression.loc (.return_ values.toArray) }
            | .control state frame control => return { state, frame, control }
        | .throw_ kind arguments =>
            let result ← evalValues fuel executable namespaceId frame state arguments.toList
            match result with
            | .values state frame values =>
                return {
                  state := state
                  frame := frame
                  control := located namespaceId expression.loc (.throw_ kind values.toArray) }
            | .control state frame control => return { state, frame, control }
        | .assign place value =>
            let result ← evalExpr fuel executable namespaceId frame state value
            match result.control.value with
            | .value value =>
                let some resolved := resolvePlace? executable.unit ns result.frame result.state place
                  | failAt namespaceId expression.loc (.invalidPlace place)
                let some (nextFrame, nextState) :=
                    writeRuntimePlace? result.frame result.state resolved value
                  | failAt namespaceId expression.loc (.invalidPlace place)
                return normal nextState nextFrame .unit
            | _ => return propagate result
        | .assignPattern pattern value =>
            let result ← evalExpr fuel executable namespaceId frame state value
            match result.control.value with
            | .value value =>
                let some nextFrame := bindPattern executable.unit ns result.frame pattern value
                  | let patternLoc := ns.patterns[pattern.index]?.map (·.loc) |>.getD expression.loc
                    failAt namespaceId patternLoc .patternMismatch
                return normal result.state nextFrame .unit
            | _ => return propagate result
        | .spec _ => return normal state frame .unit
        | .quantifier .. => failAt namespaceId expression.loc .unsupportedPreparedNode

  def evalValues : Nat → ExecutableUnit → NamespaceId → RuntimeFrame → RuntimeState →
      List ExprId → Except LocatedInterpreterError ValuesEvaluation
    | _, _, _, frame, state, [] => return .values state frame []
    | 0, executable, namespaceId, _, _, expression :: _ =>
        let loc := executable.unit.namespaces[namespaceId.index]?
          |>.map (fun ns => ns.expressions[expression.index]?.map (·.loc))
          |>.join |>.getD ⟨0⟩
        failAt namespaceId loc .outOfFuel
    | fuel + 1, executable, namespaceId, frame, state, expression :: expressions => do
        let head ← evalExpr fuel executable namespaceId frame state expression
        match head.control.value with
        | .value value =>
            let tail ← evalValues fuel executable namespaceId head.frame head.state expressions
            match tail with
            | .values state frame values => return .values state frame (value :: values)
            | .control state frame control => return .control state frame control
        | _ => return .control head.state head.frame head.control

  def evalStatements : Nat → ExecutableUnit → NamespaceId → RuntimeFrame → RuntimeState →
      List ExprId → Except LocatedInterpreterError StatementsEvaluation
    | _, _, _, frame, state, [] => return .done state frame
    | 0, executable, namespaceId, _, _, statement :: _ =>
        let loc := executable.unit.namespaces[namespaceId.index]?
          |>.map (fun ns => ns.expressions[statement.index]?.map (·.loc))
          |>.join |>.getD ⟨0⟩
        failAt namespaceId loc .outOfFuel
    | fuel + 1, executable, namespaceId, frame, state, statement :: statements => do
        let head ← evalExpr fuel executable namespaceId frame state statement
        match head.control.value with
        | .value _ => evalStatements fuel executable namespaceId head.frame head.state statements
        | _ => return .control head.state head.frame head.control

  def evalArms : Nat → ExecutableUnit → NamespaceId → ValidatedNamespace → LocId →
      RuntimeFrame → RuntimeState → RuntimeValue → List MatchArm →
      Except LocatedInterpreterError ExprEvaluation
    | _, _, namespaceId, _, ownerLoc, _, _, _, [] =>
        failAt namespaceId ownerLoc .nonExhaustiveMatch
    | 0, _, namespaceId, _, ownerLoc, _, _, _, _ :: _ =>
        failAt namespaceId ownerLoc .outOfFuel
    | fuel + 1, executable, namespaceId, ns, ownerLoc, frame, state, value, arm :: arms => do
        match bindPattern executable.unit ns frame arm.pattern value with
        | none => evalArms fuel executable namespaceId ns ownerLoc frame state value arms
        | some armFrame => match arm.guard with
          | none => evalExpr fuel executable namespaceId armFrame state arm.body
          | some guard =>
              let guardResult ← evalExpr fuel executable namespaceId armFrame state guard
              match guardResult.control.value with
              | .value (.bool true) =>
                  evalExpr fuel executable namespaceId guardResult.frame guardResult.state arm.body
              | .value (.bool false) =>
                  evalArms fuel executable namespaceId ns ownerLoc frame state value arms
              | .value actual => failAt namespaceId ownerLoc (.expectedBoolean actual)
              | _ => return guardResult
end

end Internal

/-- Execute a prepared structured-LIR function.  Fuel bounds recursive calls
and loop iterations; exhaustion is reported at the active LIR source point. -/
def run (executable : ExecutableUnit) (fuel : Nat) (function : FunctionHandle)
    (arguments : Array RuntimeValue) (state : RuntimeState := {}) :
    Except LocatedInterpreterError (RuntimeState × LocatedOutcome) := do
  let result ← Internal.evalFunction fuel executable function state arguments
  return (result.state, result.outcome)

end Interpreter
end LeanerIR
