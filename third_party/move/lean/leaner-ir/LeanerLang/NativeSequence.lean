-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Typed
import LeanerLang.Perf
import LeanerLang.NativeRegistry
import LeanerLang.NativeExpression
import LeanerLang.NativeComparison
import LeanerLang.NativeAggregate
import LeanerLang.NativeCallValue
import LeanerLang.NativeEffects
import LeanerLang.NativeVector
import LeanerLang.NativeCondition
import LeanerLang.NativeFlowBody
import LeanerLang.NativeObservedBody
import LeanerLang.NativeNestedBody
import LeanerIR.Proofs.NativeConditionAgreement
import LeanerIR.Proofs.NativeStatementAgreement
import LeanerIR.Proofs.ScalarAgreement
import LeanerIR.Proofs.Certify

/-! Typed locals and conditionals. Native computations use `Spec.bind`; local
slot updates occur only in the separately checked execution agreement. -/

namespace LeanerLang.NativeSequence

open Lean Meta Elab Command
open LeanerIR.Proofs.Denotation

set_option quotPrecheck false

private def rooted (name : Name) : Ident := mkIdent (rootNamespace ++ name)

private partial def index? (expression : Lean.Expr) : Option Nat :=
  (expression.nat? <|> expression.rawNatLit?).orElse fun _ =>
    if expression.getAppNumArgs == 1 then index? (expression.getArg! 0) else none

private structure Emitted where
  computation : Term
  verify : TSyntax `tactic
  preserves : TSyntax `tactic
  agreement : TSyntax `tactic
  usesCall : Bool := false

private partial def controlDepth (body : Lean.Expr) : Nat :=
  if body.isAppOfArity ``blockResult 2 || body.isAppOfArity ``statementsCons 2 then
    2 + controlDepth (body.getArg! 0) + controlDepth (body.getArg! 1)
  else if body.isAppOfArity ``nativeAssignLocal 2 then
    1 + controlDepth (body.getArg! 1)
  else if body.isAppOfArity ``blockUnit 1 then
    1 + controlDepth (body.getArg! 0)
  else if body.isAppOfArity ``nativeLoop 2 then
    4 + controlDepth (body.getArg! 1)
  else if body.isAppOfArity ``letNativeValue 3 then
    1 + controlDepth (body.getArg! 1) + controlDepth (body.getArg! 2)
  else if body.isAppOfArity ``nativeBranch 3 && (body.getArg! 2).isAppOf ``Option.some then
    1 + max (controlDepth (body.getArg! 1)) (controlDepth ((body.getArg! 2).getArg! 1))
  else 1

/-- Hoist a common local assignment out of a branch. No continuation is
copied into its arms; the agreement uses the corresponding denotation laws. -/
private partial def assignment? (expression : Lean.Expr) : Option Lean.Expr := do
  if expression.isAppOfArity ``nativeAssignLocal 2 then return expression
  if expression.isAppOfArity ``blockUnit 1 then
    let statements := expression.getArg! 0
    if statements.isAppOfArity ``statementsCons 2 &&
        (statements.getArg! 1).isConstOf ``statementsNil then
      return ← assignment? (statements.getArg! 0)
  if expression.isAppOfArity ``nativeBranch 3 &&
      (expression.getArg! 2).isAppOf ``Option.some then
    let yes ← assignment? (expression.getArg! 1)
    let no ← assignment? ((expression.getArg! 2).getArg! 1)
    guard (yes.getArg! 0 == no.getArg! 0)
    return mkApp2 (mkConst ``nativeAssignLocal) (yes.getArg! 0)
      (mkApp3 (mkConst ``nativeBranch) (expression.getArg! 0) (yes.getArg! 1)
        (mkApp2 (mkConst ``Option.some [Level.zero]) (mkConst ``ExprDenotation) (no.getArg! 1)))
  none

private partial def valueDepth (body : Lean.Expr) : Nat := Id.run do
  if !NativeAggregate.supported body && !NativeCopy.supported body then return 1
  let mut operands := body.getArg! 1
  let mut depth := 0
  while operands.isAppOfArity ``valuesCons 2 do
    depth := max depth (valueDepth (operands.getArg! 0))
    operands := operands.getArg! 1
  return depth + 1

def generate? (segments : Array String) (function : String)
    (generated : Denotation.Generated) (artifacts : Typed.Artifacts)
    (twins : Array SpecTypes.TwinInfo)
    (rawContract typedContract : Name) : CommandElabM Bool := do
  let some bodyValue := (← getEnv).find? generated.body |>.bind (·.value?) | return false
  let body := bodyValue.bindingBody!
  let ownedRep : Typed.ValueRep → Bool
    | .bool | .unit | .twin .. | .vector .. => true
    | _ => false
  let ownedCall := body.isAppOfArity ``nativeCall 4 &&
    (artifacts.signature.results.isEmpty ||
      (body.getUsedConstants.contains ``NominalConstructor.evaluate?) ||
      artifacts.signature.arguments.any (fun value => ownedRep value.rep) ||
      artifacts.signature.arguments.any (fun value => value.kind == .shared) ||
      artifacts.signature.results.any (fun value => ownedRep value.rep))
  let ownedRead := body.isAppOfArity ``localVar 1 &&
    artifacts.signature.results.any (fun result => match result.rep with
      | .bool | .unit | .twin .. | .vector .. => true | _ => false)
  let scalarLiteral := body.isAppOfArity ``LeanerIR.Proofs.Denotation.value 1 &&
    [``LeanerIR.RuntimeValue.bool, ``LeanerIR.RuntimeValue.integer,
      ``LeanerIR.RuntimeValue.address, ``LeanerIR.RuntimeValue.string,
      ``LeanerIR.RuntimeValue.signer, ``LeanerIR.RuntimeValue.bytes,
      ``LeanerIR.RuntimeValue.unit].contains
        (body.getArg! 0).getAppFn.constName!
  unless body.isAppOfArity ``letNativeValue 3 || body.isAppOfArity ``nativeBranch 3 ||
      body.isAppOfArity ``blockResult 2 || body.isAppOfArity ``blockUnit 1 ||
      body.isAppOfArity ``nativeThrow 2 ||
      NativeExpression.nested body || NativeExpression.division body || NativeExpression.bitwise body ||
      NativeExpression.multiplication body ||
      NativeExpression.shift body || NativeCopy.supported body ||
      NativeVector.isLength body ||
      NativeComparison.supported body || NativeCondition.supported body ||
      NativeAggregate.supported body || NativeAggregate.isVariantTest body ||
      NativeAggregate.isProjection body || ownedCall || ownedRead || scalarLiteral do return false
  let signature := artifacts.signature
  unless signature.typeParameterCount == 0 && signature.results.size ≤ 1 do
    throwError "native local sequencing requires a monomorphic owned result"
  let resultRep := if signature.results.isEmpty then .unit else signature.results[0]!.rep
  let integerRep : Typed.ValueRep := match resultRep with
    | .int .. => resultRep
    | _ => match signature.locals.find? (fun slot => match slot.rep with
        | .int .. => true | _ => false) with
      | some slot => slot.rep
      | none => .bool
  let integerShape : Option (Nat × Bool) := match integerRep with
    | .int (.bits width) signed => some (width, signed)
    | _ => none
  let hasShift := body.getUsedConstants.any fun name =>
    [``PrimitiveLocationOperation.checkedShiftLeft, ``PrimitiveLocationOperation.checkedShiftRight].contains name
  let hasIndex := body.getUsedConstants.contains ``nativeIndexedLocalBorrowOperation
  let rec vectorElementRep : Typed.ValueRep → Bool
    | .int (.bits width) _ => width > 0
    | .bool | .address | .string | .signer | .bytes | .twin _ #[] => true
    | .vector element _ => vectorElementRep element
    | _ => false
  unless (match integerRep with | .bool => true | .int (.bits width) _ => width > 0 | _ => false) &&
      signature.results.all (·.kind == .plain) && signature.locals.all (fun slot =>
      (slot.kind == .plain || slot.kind == .shared) && match slot.rep with
        | .int (.bits width) signed => width > 0 &&
            (hasShift || hasIndex || integerShape == some (width, signed))
        | .bool | .unit => true
        | .twin _ #[] => true
        | .vector element _ => vectorElementRep element
        | _ => false) do
    throwError "native local sequencing requires owned scalar, monomorphic nominal, or vector locals"
  let moduleName := segments.foldl Name.str .anonymous
  let base := (← getCurrNamespace) ++ Name.str moduleName function
  let loopInfos := (NativeLoopInfo.entries.getState (← getEnv)).find? base |>.getD #[]
  let args := mkIdent `args
  let nativeType ← integerRep.typeSyntax (mkIdent `Carrier)
  let resultType ← resultRep.typeSyntax (mkIdent `Carrier)
  let arithmeticContext : CommandElabM (Term × Term) := do
    let some (width, signed) := integerShape
      | throwError "native arithmetic requires an integer representation"
    return (← ``(LeanerIR.IntWidth.bits $(Syntax.mkNatLit width)), quote signed)
  let mut slots : Array (Option Term) := #[]
  for index in [:signature.locals.size] do
    if index < signature.arguments.size then
      slots := slots.push (some (← ``($(rooted (artifacts.argumentsType ++
        signature.arguments[index]!.name)) $args)))
    else slots := slots.push none
  let initialLoanLocations ← if signature.arguments.any (fun slot =>
      match slot.rep with | .twin .. | .vector .. => true | _ => false) then do
      let values ← signature.arguments.mapIdxM fun index argument => do
        let codec ← argument.rep.codecSyntax (mkIdent `codecs)
        let some (some value) := slots[index]? | throwError "missing native argument"
        ``(($codec).encode $value)
      ``(LeanerIR.SemanticOperations.parameterLoanLocations #[$values,*])
    else ``(#[])
  let frameWithGhosts (ghosts : Array (Option Term)) (slots : Array (Option Term)) : CommandElabM Term := do
    let values ← slots.zipIdx.mapM fun (slot, index) => match slot with
      | none => match ghosts[index]? |>.join with
        | none => ``(none)
        | some value => match signature.locals[index]!.rep with
          | .int .. => ``(Option.map LeanerIR.RuntimeValue.integer
              (Option.map LeanerIR.SpecInt.val $value))
          | .bool => ``(Option.map LeanerIR.RuntimeValue.bool $value)
          | .unit => ``(Option.map (fun _ : Unit => LeanerIR.RuntimeValue.unit) $value)
          | _ => throwError "native dead-local agreement requires a scalar representation"
      | some value => match signature.locals[index]!.rep with
        | .bool => ``(some (LeanerIR.RuntimeValue.bool $value))
        | .unit => ``(some LeanerIR.RuntimeValue.unit)
        | .twin name #[] => ``(some ($(rooted (name ++ `erase)) $value))
        | .vector .. => do
          let codec ← signature.locals[index]!.rep.codecSyntax (mkIdent `codecs)
          ``(some (($codec).encode $value))
        | _ => ``(some (LeanerIR.RuntimeValue.integer ($value).val))
    ``(({ locals := #[$values,*], loanLocations := $initialLoanLocations } : LeanerIR.RuntimeFrame))
  let frame := frameWithGhosts #[]
  let borrowFreeWithGhosts (ghosts : Array (Option Term)) (slots : Array (Option Term)) : CommandElabM (TSyntax `tactic) := do
    unless ghosts.isEmpty do
      let mut proof ← `(tactic| exact LeanerIR.Proofs.ComputationAgreement.CellsBorrowFree.nil)
      for index in (List.range slots.size).reverse do
        let cellProof ← match slots[index]! with
          | some value => match signature.locals[index]!.rep with
            | .int .. => `(tactic| exact LeanerIR.Proofs.ComputationAgreement.integer_cell (some ($value).val))
            | .bool => `(tactic| exact LeanerIR.Proofs.ComputationAgreement.bool_cell (some $value))
            | .unit => `(tactic| exact LeanerIR.Proofs.ComputationAgreement.unit_cell (some ()))
            | _ => `(tactic| simp [LeanerIR.Proofs.ComputationAgreement.BorrowFreeCell,
                LeanerIR.SemanticOperations.outermostBorrows, LeanerIR.SemanticOperations.collectPruned,
                LeanerIR.SemanticOperations.borrowEntry?])
          | none => match ghosts[index]? |>.join with
            | none => `(tactic| exact True.intro)
            | some value => do
              let (value, rule) ← match signature.locals[index]!.rep with
                | .int .. => pure (← ``(Option.map LeanerIR.SpecInt.val $value),
                    rooted ``LeanerIR.Proofs.ComputationAgreement.integer_cell)
                | .bool => pure (value, rooted ``LeanerIR.Proofs.ComputationAgreement.bool_cell)
                | .unit => pure (value, rooted ``LeanerIR.Proofs.ComputationAgreement.unit_cell)
                | _ => throwError "native dead-local agreement requires a scalar representation"
              `(tactic| exact $rule $value)
        proof ← `(tactic|
          (apply LeanerIR.Proofs.ComputationAgreement.CellsBorrowFree.cons
           · $cellProof:tactic
           · $proof:tactic))
      return ← `(tactic| (apply LeanerIR.Proofs.ComputationAgreement.borrowFreeCells; $proof:tactic))
    if signature.locals.any (fun slot => match slot.rep with | .twin .. | .vector .. => true | _ => false) then
      let mut facts : Array (TSyntax `tactic) := #[]
      for (slot, index) in slots.zipIdx do
        if let some value := slot then
          let rep := signature.locals[index]!.rep
          let encoded ← match rep with
            | .twin name #[] => ``($(rooted (name ++ `erase)) $value)
            | .bool => ``(LeanerIR.RuntimeValue.bool $value)
            | .unit => ``(LeanerIR.RuntimeValue.unit)
            | .vector .. => do
                let codec ← rep.codecSyntax (mkIdent `codecs)
                ``(($codec).encode $value)
            | _ => ``(LeanerIR.RuntimeValue.integer ($value).val)
          let proof ← if let .twin name #[] := rep then
            `(tactic| exact
              LeanerIR.SemanticOperations.Plain.outermostBorrows_eq_empty
                ($(rooted (name ++ `plain_erase)) $value))
            else do
              let codecs ← twins.mapM fun twin =>
                `(Lean.Parser.Tactic.simpLemma| $(rooted (twin.twin ++ `codec)):term)
              `(tactic|
                (apply LeanerIR.SemanticOperations.Plain.outermostBorrows_eq_empty
                 simp [LeanerIR.Proofs.Codec.boundedVector_encode, LeanerIR.Proofs.Codec.vector_encode,
                   LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.bool,
                   LeanerIR.Proofs.Codec.address, LeanerIR.Proofs.Codec.string,
                   LeanerIR.Proofs.Codec.signer, LeanerIR.Proofs.Codec.bytes,
                   leaner_plain, $codecs,*]))
          facts := facts.push (← `(tactic| have $(mkIdent (Name.mkSimple s!"plain_{index}")) :
            LeanerIR.SemanticOperations.outermostBorrows $encoded = #[] := by
              $proof:tactic))
      return ← `(tactic|
        ($facts:tactic*
         simp_all [LeanerIR.SemanticOperations.frameBorrows] <;> assumption))
    if signature.locals.any (fun slot => match slot.rep with | .bool | .unit => true | _ => false) then
      return ← `(tactic| simp [LeanerIR.SemanticOperations.frameBorrows,
        LeanerIR.SemanticOperations.outermostBorrows, LeanerIR.SemanticOperations.collectPruned,
        LeanerIR.SemanticOperations.borrowEntry?])
    let values ← slots.mapM fun slot => match slot with
      | none => ``(none)
      | some value => ``(some ($value).val)
    `(tactic| exact LeanerIR.Proofs.ComputationAgreement.integerLocals_borrowFree [$values,*])
  let operand (slots : Array (Option Term)) (known : Array NativeAggregate.Known) :
      Lean.Expr → CommandElabM (Term × TSyntax `tactic) := NativeCopy.emitPure fun expression => do
    if NativeAggregate.isProjection expression then
      let (rep, value, proof) ← NativeAggregate.read twins signature.locals slots known expression
      unless (match rep with | .int .. => true | _ => false) do
        throwError "native arithmetic projection is not an integer"
      return (← ``(($value).val), proof)
    if expression.isAppOfArity ``LeanerIR.Proofs.Denotation.value 1 &&
        (expression.getArg! 0).isAppOfArity ``LeanerIR.RuntimeValue.integer 1 then
      let value ← liftTermElabM <| PrettyPrinter.delab ((expression.getArg! 0).getArg! 0)
      return (value, ← `(tactic| exact LeanerIR.Proofs.ComputationAgreement.literal _ _))
    unless expression.isAppOfArity ``localVar 1 do
      throwError "native local arithmetic currently requires local reads or integer literals"
    let some index := index? (expression.getArg! 0) | throwError "invalid native local index"
    let some (some value) := slots[index]? | throwError "native read of an unavailable local"
    unless (match signature.locals[index]!.rep with | .int .. => true | _ => false) do
      throwError "native integer operand has a non-integer local type"
    return (← ``(($value).val), ← `(tactic| exact LeanerIR.Proofs.ComputationAgreement.read _ _ _ rfl))
  let booleanOperand (slots : Array (Option Term)) (known : Array NativeAggregate.Known)
      : Lean.Expr → CommandElabM (Term × TSyntax `tactic) := NativeCopy.emitPure fun expression => do
    if expression.isAppOfArity ``localVar 1 then
      let some index := index? (expression.getArg! 0) | throwError "invalid native Boolean local"
      let some (some value) := slots[index]? | throwError "native Boolean local is unavailable"
      unless signature.locals[index]!.rep == .bool do
        throwError "native Boolean operand has a non-Boolean local type"
      return (value, ← `(tactic| exact LeanerIR.Proofs.ComputationAgreement.read _ _ _ rfl))
    if expression.isAppOfArity ``LeanerIR.Proofs.Denotation.value 1 &&
        (expression.getArg! 0).isAppOfArity ``LeanerIR.RuntimeValue.bool 1 then
      let value ← liftTermElabM <| PrettyPrinter.delab ((expression.getArg! 0).getArg! 0)
      return (value, ← `(tactic| exact LeanerIR.Proofs.ComputationAgreement.literal _ _))
    if NativeAggregate.isProjection expression then
      let (rep, value, proof) ← NativeAggregate.read twins signature.locals slots known expression
      unless rep == .bool do throwError "native Boolean projection has a non-Boolean type"
      return (value, proof)
    throwError "native Boolean operand requires a local read, literal, or projection"
  let condition (slots : Array (Option Term)) (known : Array NativeAggregate.Known) :
      Lean.Expr → CommandElabM (Term × TSyntax `tactic) := NativeCopy.emitPure fun expression => do
    if NativeAggregate.isProjection expression then
      let (rep, value, proof) ← NativeAggregate.read twins signature.locals slots known expression
      unless rep == .bool do throwError "native condition projection is not Boolean"
      return (value, proof)
    if NativeAggregate.isVariantTest expression then
      let owner ← NativeAggregate.unaryOperand expression
      let (rep, value, proof) ← NativeAggregate.read twins signature.locals slots known owner
      if let some choice := known.find? (·.expression == owner) then
        let .twin name #[] := rep | throwError "native variant requires an enum twin"
        let selected := quote ((← NativeAggregate.variants expression).contains choice.variant)
        let agreement ← `(tactic|
          (apply LeanerIR.Proofs.ComputationAgreement.nativeOperation_value
             (value := LeanerIR.RuntimeValue.bool $selected)
           · apply LeanerIR.Proofs.ComputationAgreement.cons
             · $proof:tactic
             · exact LeanerIR.Proofs.ComputationAgreement.nil _
           · intro state
             simp_all [$(rooted (name ++ `codec)):term, $(rooted (name ++ `erase)):term,
               LeanerIR.Proofs.Denotation.NominalVariantTest.evaluate?,
               LeanerIR.Proofs.Denotation.liftConstructorEvaluator,
               LeanerIR.SemanticOperations.testNominalVariants?]))
        return (selected, agreement)
      return ← NativeAggregate.emitTest twins rep value proof expression
    if expression.isAppOfArity ``localVar 1 then
      let some index := index? (expression.getArg! 0) | throwError "invalid native condition local"
      let some (some value) := slots[index]? | throwError "native condition local is unavailable"
      unless (match signature.locals[index]!.rep with | .bool => true | _ => false) do
        throwError "native condition requires a Boolean local"
      return (value, ← `(tactic| exact LeanerIR.Proofs.ComputationAgreement.read _ _ _ rfl))
    if expression.isAppOfArity ``LeanerIR.Proofs.Denotation.value 1 &&
        (expression.getArg! 0).isAppOfArity ``LeanerIR.RuntimeValue.bool 1 then
      let value ← liftTermElabM <| PrettyPrinter.delab ((expression.getArg! 0).getArg! 0)
      return (value, ← `(tactic| exact LeanerIR.Proofs.ComputationAgreement.literal _ _))
    let booleanComparison ← if NativeComparison.isEquality expression then do
      let operands := expression.getArg! 1
      unless operands.isAppOfArity ``valuesCons 2 do
        throwError "native equality requires operands"
      let first ← NativeCopy.unwrap (operands.getArg! 0)
      if first.isAppOfArity ``localVar 1 then
        let some index := index? (first.getArg! 0) | throwError "invalid native equality local"
        let some slot := signature.locals[index]? | throwError "native equality local is unavailable"
        pure (slot.rep == .bool)
      else if NativeAggregate.isProjection first then
        let (rep, _, _) ← NativeAggregate.read twins signature.locals slots known first
        pure (rep == .bool)
      else pure (first.isAppOfArity ``LeanerIR.Proofs.Denotation.value 1 &&
        (first.getArg! 0).isAppOfArity ``LeanerIR.RuntimeValue.bool 1)
    else pure false
    if booleanComparison then
      NativeComparison.emit (booleanOperand slots known) expression true
    else NativeComparison.emit (operand slots known) expression
  let splitVariant (slots : Array (Option Term)) (expression : Lean.Expr)
      (proof : TSyntax `tactic) : CommandElabM (TSyntax `tactic) := do
    unless NativeAggregate.isVariantTest expression do return proof
    let some index := index? (((expression.getArg! 1).getArg! 0).getArg! 0)
      | throwError "invalid native enum local"
    let some (some value) := slots[index]? | throwError "native enum local is unavailable"
    let .twin name #[] := signature.locals[index]!.rep
      | throwError "native enum split requires a monomorphic twin"
    return ← `(tactic|
      (cases $(mkIdent `variantEquation):ident : $value:term <;>
        simp_all (config := { failIfUnchanged := false }) only
          [$(mkIdent `variantEquation):term, $(rooted (name ++ `codec)):term,
            $(rooted (name ++ `erase)):term, Bool.false_eq_true, Bool.true_eq_false,
            not_true_eq_false] <;> $proof:tactic))
  let rec pureValue (fuel : Nat) (slots : Array (Option Term)) (known : Array NativeAggregate.Known) (rep : Typed.ValueRep)
      (expression : Lean.Expr) : CommandElabM (Term × TSyntax `tactic) := do
    let fuel + 1 := fuel | throwError "native value exceeds its expression depth"
    if NativeCopy.supported expression then
      let (value, proof) ← pureValue fuel slots known rep (← NativeCopy.operand expression)
      return (value, ← NativeCopy.returns proof)
    if let some index := NativeCopy.localBorrow? expression then
      let some (some value) := slots[index]? | throwError "native shared borrow local is unavailable"
      unless signature.locals[index]!.rep == rep do throwError "native shared borrow type mismatch"
      return (value, ← NativeCopy.borrowReturns index slots.size)
    if NativeAggregate.isProjection expression then
      let (actualRep, value, proof) ← NativeAggregate.read twins signature.locals slots known expression
      unless actualRep == rep do throwError "native payload type mismatch"
      return (value, proof)
    if expression.isAppOfArity ``localVar 1 then
      let some index := index? (expression.getArg! 0) | throwError "invalid native value local"
      let some (some value) := slots[index]? | throwError "native value local is unavailable"
      unless signature.locals[index]!.rep == rep do throwError "native value local type mismatch"
      return (value, ← `(tactic| exact LeanerIR.Proofs.ComputationAgreement.read _ _ _ rfl))
    if NativeAggregate.supported expression then
      return ← NativeAggregate.emit twins (pureValue fuel slots known) rep expression
    if rep == .unit && expression.isConstOf ``nativeSpec && !loopInfos.isEmpty then
      return (← ``(()), ← `(tactic| exact LeanerIR.Proofs.ComputationAgreement.literal _ _))
    if rep == .bool then return ← condition slots known expression
    if rep == .unit && expression.isAppOfArity ``LeanerIR.Proofs.Denotation.value 1 &&
        (expression.getArg! 0).isConstOf ``LeanerIR.RuntimeValue.unit then
      return (← ``(()), ← `(tactic| exact LeanerIR.Proofs.ComputationAgreement.literal _ _))
    if expression.isAppOfArity ``LeanerIR.Proofs.Denotation.value 1 &&
        (expression.getArg! 0).isAppOfArity ``LeanerIR.RuntimeValue.integer 1 then
      let literal ← liftTermElabM <| PrettyPrinter.delab ((expression.getArg! 0).getArg! 0)
      let ty ← rep.typeSyntax (mkIdent `Carrier)
      return (← ``((⟨$literal, by decide⟩ : $ty)),
        ← `(tactic| exact LeanerIR.Proofs.ComputationAgreement.literal _ _))
    if expression.isAppOfArity ``LeanerIR.Proofs.Denotation.value 1 then
      let constructor := match rep with
        | .address => ``LeanerIR.RuntimeValue.address
        | .string => ``LeanerIR.RuntimeValue.string
        | .signer => ``LeanerIR.RuntimeValue.signer
        | .bytes => ``LeanerIR.RuntimeValue.bytes
        | _ => .anonymous
      if constructor != .anonymous && (expression.getArg! 0).isAppOfArity constructor 1 then
        let literal ← liftTermElabM <| PrettyPrinter.delab ((expression.getArg! 0).getArg! 0)
        return (literal, ← `(tactic| exact LeanerIR.Proofs.ComputationAgreement.literal _ _))
    throwError "native pure value requires a local read, literal, or nominal constructor"
  let vectorLength (slots : Array (Option Term)) (known : Array NativeAggregate.Known)
      (expected : Typed.ValueRep) (expression : Lean.Expr) (resultName : Option Ident := none) :
      CommandElabM NativeOperands.Value := do
    unless expected == .int (.bits 64) false do
      throwError "native Move vector length requires a u64 result"
    let source ← NativeAggregate.unaryOperand expression
    let metadata ← NativeCopy.unwrap source
    let rep ← if metadata.isAppOfArity ``localVar 1 then do
        let some index := index? (metadata.getArg! 0) | throwError "invalid native vector local"
        let some localInfo := signature.locals[index]? | throwError "native vector local is out of range"
        pure localInfo.rep
      else if metadata.isAppOfArity ``nativeCall 4 then do
        let callee ← NativeCallValue.resolve metadata
        let #[result] := callee.artifacts.signature.results
          | throwError "native vector length requires one callee result"
        pure result.rep
      else if NativeAggregate.isProjection metadata then do
        let (rep, _, _) ← NativeAggregate.read twins signature.locals slots known metadata
        pure rep
      else throwError "native vector length requires resolved local, field, or callee type metadata"
    let argument ← NativeEffects.emit twins slots
      (fun rep value => pureValue (valueDepth value) slots known rep value) rep source
    NativeVector.emitLength rep argument resultName
  let checked (slots : Array (Option Term)) (known : Array NativeAggregate.Known) (expression : Lean.Expr) :
      CommandElabM (Term × Term × Term × TSyntax `tactic × TSyntax `tactic × TSyntax `tactic) := do
    let (widthTerm, signedTerm) ← arithmeticContext
    unless expression.isAppOfArity ``nativePrimitiveOperation 2 do
      throwError "native local sequencing currently requires checked arithmetic steps"
    let operation := expression.getArg! 0
    let operator := operation.getAppFn.constName!
    unless [``PrimitiveLocationOperation.checkedAdd,
        ``PrimitiveLocationOperation.checkedSubtract].contains operator do
      throwError "native local sequencing currently supports checked addition and subtraction"
    let operands := expression.getArg! 1
    unless operands.isAppOfArity ``valuesCons 2 &&
        (operands.getArg! 1).isAppOfArity ``valuesCons 2 &&
        ((operands.getArg! 1).getArg! 1).isConstOf ``valuesNil do
      throwError "native local arithmetic requires two operands"
    let (left, leftProof) ← operand slots known (operands.getArg! 0)
    let (right, rightProof) ← operand slots known ((operands.getArg! 1).getArg! 0)
    let value ← if operator == ``PrimitiveLocationOperation.checkedAdd then ``($left + $right)
      else ``($left - $right)
    let kind ← liftTermElabM <| PrettyPrinter.delab (operation.getArg! 0)
    let computation ← ``(LeanerIR.Proofs.NativeArithmetic.checkedInteger $widthTerm $signedTerm
      (LeanerIR.Proofs.NativeArithmetic.runtimeFailure $kind) $value)
    let operandsProof ← `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.cons
       · $leftProof:tactic
       · apply LeanerIR.Proofs.ComputationAgreement.cons
         · $rightProof:tactic
         · exact LeanerIR.Proofs.ComputationAgreement.nil _))
    let successRule := rooted <| if operator == ``PrimitiveLocationOperation.checkedAdd then
      ``LeanerIR.Proofs.ComputationAgreement.evaluate_checkedAdd_fits else
      ``LeanerIR.Proofs.ComputationAgreement.evaluate_checkedSubtract_fits
    let failureRule := rooted <| if operator == ``PrimitiveLocationOperation.checkedAdd then
      ``LeanerIR.Proofs.ComputationAgreement.evaluate_checkedAdd_overflow else
      ``LeanerIR.Proofs.ComputationAgreement.evaluate_checkedSubtract_overflow
    let success ← `(tactic|
      (intro $(mkIdent `fits):ident $(mkIdent `state):ident
       exact $successRule rfl $(mkIdent `fits)))
    let failure ← `(tactic|
      (intro $(mkIdent `overflow):ident $(mkIdent `state):ident
       exact $failureRule rfl $(mkIdent `overflow)))
    return (computation, kind, value, operandsProof, success, failure)
  let rec emit (fuel : Nat) (ghosts : Array (Option Term)) (slots : Array (Option Term))
      (known : Array NativeAggregate.Known) (expression : Lean.Expr) :
      CommandElabM Emitted := do
    let fuel + 1 := fuel | throwError "native local sequence exceeds its structural depth"
    let frame := frameWithGhosts ghosts
    let borrowFree := borrowFreeWithGhosts ghosts
    if expression.isAppOfArity ``blockUnit 1 then
      let next ← emit fuel ghosts slots known (mkApp2 (mkConst ``blockResult) (expression.getArg! 0)
        (mkApp (mkConst ``LeanerIR.Proofs.Denotation.value) (mkConst ``LeanerIR.RuntimeValue.unit)))
      let agreement ← `(tactic|
        (apply LeanerIR.Proofs.Spec.Equiv.trans
          (LeanerIR.Proofs.ComputationAgreement.fromFrame_unit _ _ _ _)
         $(next.agreement):tactic))
      return { next with agreement }
    if expression.isAppOfArity ``blockResult 2 then
      let statements := expression.getArg! 0
      let result := expression.getArg! 1
      if statements.isConstOf ``statementsNil then
        let next ← emit fuel ghosts slots known result
        let agreement ← `(tactic|
          (apply LeanerIR.Proofs.Spec.Equiv.trans
            (LeanerIR.Proofs.ComputationAgreement.fromFrame_congr _ _
              (LeanerIR.Proofs.ComputationAgreement.block_nil _ _))
           $(next.agreement):tactic))
        return { next with agreement }
      if statements.isAppOfArity ``statementsCons 2 then
        let head := statements.getArg! 0
        if head.getUsedConstants.contains ``nativeReturn then
          let indices := slots.zipIdx.filterMap fun (value, index) =>
            if value.isSome then some ({ index } : LeanerIR.LocalId) else none
          let info : NativeLoopInfo.Loop := {
            site := ⟨0⟩, slots := indices
            representations := indices.map (fun slot => signature.locals[slot.index]!.rep)
            predicate := .anonymous }
          let type ← NativeFlowBody.headerType info
          let mut dead := (NativeLocalView.boundLocals head).filter (fun slot => !indices.contains slot)
          for (value, index) in ghosts.zipIdx do
            if value.isSome && !dead.contains ⟨index⟩ then dead := dead.push ⟨index⟩
          for slot in dead do
            unless signature.locals[slot.index]!.kind == .plain &&
                (match signature.locals[slot.index]!.rep with | .int .. | .bool | .unit => true | _ => false) do
              throwError "native return dead-local agreement requires owned scalar locals"
          let view : NativeLocalView.View := { locals := signature.locals, dead, frame := frameWithGhosts }
          let returns : NativeNestedBody.Returns := {
            rep := resultRep, codec := rooted artifacts.resultsCodec
            borrowFree := borrowFreeWithGhosts }
          let scalar := fun currentSlots rep value name => NativeCondition.emitChoice twins currentSlots
            (fun rep value => pureValue (valueDepth value) currentSlots #[] rep value) integerRep rep value name
          let first ← NativeNestedBody.emit (some returns) (4 * controlDepth head + 8)
            loopInfos args [] info view ghosts slots scalar head
          let joined := mkIdent (Name.mkSimple s!"return_join_{fuel}")
          let returnedDead := mkIdent (Name.mkSimple s!"return_dead_{fuel}")
          let nextGhosts ← view.unpackGhosts returnedDead
          let nextSlots ← view.slots info joined
          let next ← emit fuel nextGhosts nextSlots #[]
            (mkApp2 (mkConst ``blockResult) (statements.getArg! 1) result)
          let proof ← first.verifyWith fun kind value => do
            match kind with
            | .normal => `(tactic|
                (let $joined : $type := $value
                 change LeanerIR.Proofs.wp $(next.computation) _ _ _
                 simp (config := { failIfUnchanged := false }) only [$joined:term, Prod.fst, Prod.snd]
                 $(next.verify):tactic))
            | .return_ => `(tactic|
                (change LeanerIR.Proofs.wp (LeanerIR.Proofs.Spec.pure ($value : $resultType)) _ _ _
                 rw [LeanerIR.Proofs.wp_pure]
                 leaner_native_data <;> leaner_certified_close!))
            | _ => throwError "native loop control escapes the function boundary"
          return {
            computation := ← ``(LeanerIR.Proofs.NativeReturnFlow.finish $(first.computation)
              (fun $joined : $type => $(next.computation)))
            verify := ← `(tactic|
              (rw [LeanerIR.Proofs.NativeReturnFlow.finish, LeanerIR.Proofs.wp_bind]
               $proof:tactic))
            preserves := ← `(tactic|
              (apply LeanerIR.Proofs.StatePreserving.bind
               · $(first.preserves):tactic
               · intro flow
                 cases flow with
                 | normal $joined:ident => $(next.preserves):tactic
                 | continue_ _ => exact LeanerIR.Proofs.StatePreserving.pure _
                 | break_ _ => exact LeanerIR.Proofs.StatePreserving.pure _))
            agreement := ← `(tactic|
              (apply LeanerIR.Proofs.ComputationAgreement.fromFrame_returning_discard
                 (frames := $(← view.frames info))
               · $(first.agreement):tactic
               · intro value; rfl
               · intro $joined:ident observed_frame
                 rintro ⟨$returnedDead:ident, sameFrame⟩
                 subst observed_frame
                 $(next.agreement):tactic))
            usesCall := next.usesCall || head.getUsedConstants.contains ``nativeCall }
        if head.isAppOfArity ``nativeLoop 2 then
          let some site := index? (head.getArg! 0) | throwError "invalid native loop site"
          let some info := loopInfos.find? (·.site.index == site)
            | throwError "native loop has no typed invariant"
          let type ← NativeFlowBody.headerType info
          let packed ← NativeFlowBody.pack info slots
          let bodyLocals := (NativeLocalView.boundLocals (head.getArg! 1)).filter
            (fun slot => !info.slots.contains slot)
          let nested := (head.getArg! 1).getUsedConstants.contains ``nativeLoop
          let observed := nested || !bodyLocals.isEmpty
          let mut dead := bodyLocals
          for (value, index) in ghosts.zipIdx do
            if value.isSome && !dead.contains ⟨index⟩ then dead := dead.push ⟨index⟩
          for slot in dead do
            unless signature.locals[slot.index]!.kind == .plain &&
                (match signature.locals[slot.index]!.rep with | .int .. | .bool | .unit => true | _ => false) do
              throwError "native dead-local agreement requires owned scalar locals"
          let view : NativeLocalView.View := { locals := signature.locals, dead, frame := frameWithGhosts }
          let current := mkIdent (Name.mkSimple s!"loop_locals_{fuel}")
          let exited := mkIdent (Name.mkSimple s!"loop_exit_{fuel}")
          let currentDead := mkIdent (Name.mkSimple s!"loop_dead_{fuel}")
          let exitDead := mkIdent (Name.mkSimple s!"loop_exit_dead_{fuel}")
          let currentGhosts ← if observed then view.unpackGhosts ⟨currentDead.raw⟩ else pure ghosts
          let exitGhosts ← if observed then view.unpackGhosts ⟨exitDead.raw⟩ else pure ghosts
          let currentSlots ← NativeFlowBody.unpack info slots ⟨current.raw⟩
          let exitSlots ← NativeFlowBody.unpack info slots ⟨exited.raw⟩
          let currentFrame ← frame currentSlots
          let scalar := fun currentSlots rep value name => NativeCondition.emitChoice twins currentSlots
              (fun rep value => pureValue (valueDepth value) currentSlots #[] rep value)
              integerRep rep value name
          let step ← if nested then
              NativeNestedBody.emitRoot (4 * controlDepth (head.getArg! 1) + 8) loopInfos args info
                view currentGhosts currentSlots scalar (head.getArg! 1)
            else if observed then
              NativeObservedBody.emit (4 * controlDepth (head.getArg! 1) + 8) info info
                view currentGhosts currentSlots scalar (head.getArg! 1)
            else
              NativeFlowBody.emit (4 * controlDepth (head.getArg! 1) + 8) info
                signature.locals currentSlots frame scalar (head.getArg! 1)
          let next ← emit fuel exitGhosts exitSlots #[]
            (mkApp2 (mkConst ``blockResult) (statements.getArg! 1) result)
          let invariant ← ``($(rooted info.predicate) $args $packed)
          let iteration ← ``(fun $current : $type => LeanerIR.Proofs.NativeFlow.iteration $(step.computation))
          let computation ← ``(LeanerIR.Proofs.NativeLoop.run $iteration $packed $invariant)
          let recursive := mkIdent (Name.mkSimple s!"loop_recursive_{fuel}")
          let hypothesis := mkIdent (Name.mkSimple s!"loop_hypothesis_{fuel}")
          let holds := mkIdent (Name.mkSimple s!"loop_invariant_{fuel}")
          let mut bounds : Array (TSyntax `tactic) := #[]
          for (slot, rep) in info.slots.zip info.representations do
            if let .int _ signed := rep then
              let some (some value) := currentSlots[slot.index]? | throwError "missing typed loop local"
              let rule := rooted (if signed then ``LeanerIR.IntegerValueFits.signed_bounds
                else ``LeanerIR.IntegerValueFits.unsigned_bounds)
              bounds := bounds.push (← `(tactic|
                (have loopBounds := $rule (LeanerIR.SpecInt.fits $value)
                 leaner_cases loopBounds)))
          let stepProof ← step.verifyWith fun kind values => do
            match kind with
            | .normal | .continue_ => `(tactic|
                (change LeanerIR.Proofs.wp ($recursive $values) _ _ _
                 apply $hypothesis
                 simp (config := { failIfUnchanged := false }) only
                   [$(rooted info.predicate):term, Prod.fst, Prod.snd]
                 leaner_certified_close!))
            | .break_ => `(tactic|
                (change LeanerIR.Proofs.wp (LeanerIR.Proofs.Spec.pure $values) _ _ _
                 rw [LeanerIR.Proofs.wp_pure]
                 let $exited : $type := $values
                 change LeanerIR.Proofs.wp $(next.computation) _ _ _
                 simp (config := { failIfUnchanged := false }) only [$exited:term, Prod.fst, Prod.snd]
                 $(next.verify):tactic))
          let mut iterationProof ← `(tactic|
            (rw [LeanerIR.Proofs.NativeLoop.body, LeanerIR.Proofs.wp_bind,
               LeanerIR.Proofs.NativeFlow.wp_iteration]
             $stepProof:tactic))
          for bound in bounds.reverse do
            iterationProof ← `(tactic| ($bound:tactic; $iterationProof:tactic))
          let verify ← `(tactic|
            (rw [LeanerIR.Proofs.wp_bind]
             apply LeanerIR.Proofs.wp_withInvariant_fix_frame
             · simp (config := { failIfUnchanged := false }) only
                 [$(rooted info.predicate):term, Prod.fst, Prod.snd]
               leaner_certified_close!
             · intro $recursive:ident $hypothesis:ident $current:ident $holds:ident
               simp (config := { failIfUnchanged := false }) only
                 [$(rooted info.predicate):term, Prod.fst, Prod.snd] at $holds:ident
               leaner_cases $holds:ident
               $iterationProof:tactic))
          let preserves ← `(tactic|
            (apply LeanerIR.Proofs.StatePreserving.bind
             · apply LeanerIR.Proofs.NativeLoop.preserves
               intro $current:ident
               apply LeanerIR.Proofs.NativeFlow.iteration_preserves
               $(step.preserves):tactic
             · intro $exited:ident; $(next.preserves):tactic))
          let agreement ← if observed then `(tactic|
            (apply LeanerIR.Proofs.ComputationAgreement.fromFrame_observed_discard
               (frames := $(← view.frames info)) (first := $computation)
             · apply LeanerIR.Proofs.ComputationAgreement.observed_loop
               · exact ⟨$(← view.packGhosts ghosts slots), rfl⟩
               · intro $current:ident observed_frame
                 rintro ⟨$currentDead:ident, sameFrame⟩
                 subst observed_frame
                 $(step.agreement):tactic
             · intro $exited:ident observed_frame
               rintro ⟨$exitDead:ident, sameFrame⟩
               subst observed_frame
               $(next.agreement):tactic))
            else `(tactic|
            (apply LeanerIR.Proofs.ComputationAgreement.fromFrame_loop
               (entry := fun $current : $type => $currentFrame)
               (exit := fun $current : $type => $currentFrame)
               (iteration := $iteration) (locals := $packed) (invariant := $invariant)
             · apply LeanerIR.Proofs.ComputationAgreement.loopIteration_of_controlled
               intro $current:ident
               $(step.agreement):tactic
             · intro $exited:ident; $(next.agreement):tactic))
          return ⟨← ``(LeanerIR.Proofs.Spec.bind $computation
              (fun $exited : $type => $(next.computation))), verify, preserves, agreement,
            next.usesCall || head.getUsedConstants.contains ``nativeCall⟩
        let some assignment := assignment? (statements.getArg! 0)
          | do
            let head := statements.getArg! 0
            let rep ← if head.isAppOfArity ``nativeCall 4 then do
                let callee ← NativeCallValue.resolve head
                let results := callee.artifacts.signature.results
                if results.isEmpty then pure Typed.ValueRep.unit
                else if results.size == 1 then pure results[0]!.rep
                else throwError "native discarded call requires a single result or Unit"
              else pure Typed.ValueRep.unit
            let type ← rep.typeSyntax (mkIdent `Carrier)
            let codec ← rep.codecSyntax (mkIdent `codecs)
            let first ← NativeCondition.emitChoice twins slots
              (fun rep value => pureValue (valueDepth value) slots known rep value) integerRep rep head
            let next ← emit fuel ghosts slots known
              (mkApp2 (mkConst ``blockResult) (statements.getArg! 1) result)
            let verify ← first.verifyWith next.verify
            return ⟨← ``(LeanerIR.Proofs.Spec.bind $(first.computation)
                (fun _ : $type => $(next.computation))),
              ← `(tactic| (rw [LeanerIR.Proofs.wp_bind]; $verify:tactic)),
              ← `(tactic|
                (apply LeanerIR.Proofs.StatePreserving.bind
                 · $(first.preserves):tactic
                 · intro ignored; $(next.preserves):tactic)),
              ← `(tactic|
                (apply LeanerIR.Proofs.ComputationAgreement.fromFrame_discard
                   (encode := fun value : $type => ($codec).encode value)
                 · $(first.agreement):tactic
                 · $(first.preserves):tactic
                 · $(next.agreement):tactic)),
              next.usesCall || head.getUsedConstants.contains ``nativeCall⟩
        let binder := mkApp2 (mkConst ``LeanerIR.SemanticOperations.NativePatternBinder.mk)
          (mkNatLit 1) (mkApp (mkConst ``LeanerIR.SemanticOperations.NativePattern.variable)
            (assignment.getArg! 0))
        let rebound := mkApp3 (mkConst ``letNativeValue) binder (assignment.getArg! 1)
          (mkApp2 (mkConst ``blockResult) (statements.getArg! 1) result)
        let next ← emit fuel ghosts slots known rebound
        let agreement ← `(tactic|
          (apply LeanerIR.Proofs.Spec.Equiv.trans
            (LeanerIR.Proofs.ComputationAgreement.fromFrame_congr _ _
              (LeanerIR.Proofs.ComputationAgreement.block_assign_head _ _ _ _ _
                (by simp (config := { failIfUnchanged := false }) only
                  [LeanerIR.Proofs.ComputationAgreement.unit_single_assign,
                  LeanerIR.Proofs.ComputationAgreement.branch_assign] <;> rfl) _))
           $(next.agreement):tactic))
        return { next with agreement }
      throwError "native statement sequencing requires a supported assignment"
    if expression.isAppOfArity ``nativeThrow 2 ||
        (resultRep == .unit && (expression.isAppOfArity ``nativeBranch 3 ||
          expression.isAppOfArity ``LeanerIR.Proofs.Denotation.value 1)) then
      let scalar ← NativeCondition.emitChoice twins slots
        (fun rep value => pureValue (valueDepth value) slots known rep value) integerRep resultRep expression
      let agreement ← `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.fromFrame_scalar
         · $(scalar.agreement):tactic
         · intro result; rfl
         · $(← borrowFree slots):tactic))
      return ⟨scalar.computation, ← scalar.verifyWith (← `(tactic| leaner_certified_close!)),
        scalar.preserves, agreement, expression.getUsedConstants.contains ``nativeCall⟩
    if NativeVector.isLength expression then
      let scalar ← vectorLength slots known resultRep expression
      let agreement ← `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.fromFrame_scalar
         · $(scalar.agreement):tactic
         · intro result; rfl
         · $(← borrowFree slots):tactic))
      return ⟨scalar.computation, ← scalar.verifyWith (← `(tactic| leaner_certified_close!)),
        scalar.preserves, agreement, expression.getUsedConstants.contains ``nativeCall⟩
    if (NativeAggregate.supported expression && NativeEffects.required expression) ||
        (resultRep == .bool && NativeCondition.supported expression &&
          !expression.isAppOfArity ``nativeBranch 3) then
      let scalar ← if NativeCondition.supported expression then
          NativeCondition.emit twins slots
            (fun rep value => pureValue (valueDepth value) slots known rep value) integerRep expression
        else
          NativeEffects.emit twins slots
            (fun rep value => pureValue (valueDepth value) slots known rep value) resultRep expression
      let agreement ← `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.fromFrame_scalar
         · $(scalar.agreement):tactic
         · intro result; rfl
         · $(← borrowFree slots):tactic))
      return ⟨scalar.computation, ← scalar.verifyWith
          (← `(tactic| (leaner_native_data <;> leaner_certified_close!))),
        scalar.preserves, agreement, expression.getUsedConstants.contains ``nativeCall⟩
    if expression.isAppOfArity ``nativeCall 4 then
      let call ← NativeCallValue.emit twins resultRep
        (fun rep operand => pureValue (valueDepth operand) slots known rep operand) expression
      let agreement ← `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.fromFrame_scalar
         · $(call.agreement):tactic
         · intro result; rfl
         · $(← borrowFree slots):tactic))
      return ⟨call.computation, call.verify, call.preserves, agreement, true⟩
    if NativeAggregate.supported expression || NativeAggregate.isProjection expression ||
        NativeCopy.supported expression ||
        expression.isAppOfArity ``LeanerIR.Proofs.Denotation.value 1 then
      let (value, evaluates) ← pureValue (valueDepth expression) slots known resultRep expression
      let codec ← resultRep.codecSyntax (mkIdent `codecs)
      let agreement ← `(tactic|
        (apply LeanerIR.Proofs.Spec.Equiv.trans ?_
          (LeanerIR.Proofs.encodeSpec_pure _ $value).symm
         apply LeanerIR.Proofs.ComputationAgreement.fromFrame_returns
           (value := ($codec).encode $value)
         · $evaluates:tactic
         · rfl
         · $(← borrowFree slots):tactic))
      return ⟨← ``(LeanerIR.Proofs.Spec.pure $value),
        ← `(tactic| (rw [LeanerIR.Proofs.wp_pure]; leaner_certified_close!)),
        ← `(tactic| exact LeanerIR.Proofs.StatePreserving.pure _), agreement, false⟩
    if NativeComparison.supported expression || NativeAggregate.isVariantTest expression then
      let (value, evaluates) ← condition slots known expression
      let agreement ← `(tactic|
        (apply LeanerIR.Proofs.Spec.Equiv.trans ?_
          (LeanerIR.Proofs.encodeSpec_pure _ ($value : Bool)).symm
         apply LeanerIR.Proofs.ComputationAgreement.fromFrame_returns
           (value := LeanerIR.RuntimeValue.bool $value)
         · $evaluates:tactic
         · rfl
         · $(← borrowFree slots):tactic))
      return ⟨← ``(LeanerIR.Proofs.Spec.pure $value),
        ← splitVariant slots expression
          (← `(tactic| (rw [LeanerIR.Proofs.wp_pure]; leaner_certified_close!))),
        ← `(tactic| exact LeanerIR.Proofs.StatePreserving.pure _), agreement, false⟩
    if expression.isAppOfArity ``nativeBranch 3 then
      if NativeAggregate.isVariantTest (expression.getArg! 0) then
        let testExpression := expression.getArg! 0
        let owner ← NativeAggregate.unaryOperand testExpression
        let (rep, ownerValue, _) ← NativeAggregate.read twins signature.locals slots known owner
        let .twin name #[] := rep | throwError "native matching requires a monomorphic enum"
        let some info := twins.find? (·.twin == name) | throwError "missing native enum twin"
        let variants ← NativeAggregate.variants testExpression
        unless (expression.getArg! 2).isAppOf ``Option.some do
          throwError "native enum match requires an else result"
        let arm (choice : NativeAggregate.Known) : CommandElabM Emitted := do
          let selected := variants.contains choice.variant
          let nextKnown := known.push choice
          let next ← emit fuel ghosts slots nextKnown
            (if selected then expression.getArg! 1 else (expression.getArg! 2).getArg! 1)
          let (_, evaluates) ← condition slots nextKnown testExpression
          let agreement ← `(tactic|
            (apply LeanerIR.Proofs.Spec.Equiv.trans
              (LeanerIR.Proofs.ComputationAgreement.fromFrame_branch_select _ _ _ _ _ _
                $(quote selected) (by $evaluates:tactic))
             $(next.agreement):tactic))
          return { next with agreement }
        if let some choice := known.find? (·.expression == owner) then
          return ← arm choice
        let equation := mkIdent (Name.mkSimple s!"variant_{fuel}")
        let mut terms : Array (TSyntax ``Lean.Parser.Term.matchAlt) := #[]
        let mut verifies : Array (TSyntax `tactic) := #[]
        let mut preserves : Array (TSyntax `tactic) := #[]
        let mut agreements : Array (TSyntax `tactic) := #[]
        let mut usesCall := false
        for variant in info.variants do
          let fields := variant.fields.mapIdx fun index _ =>
            mkIdent (Name.mkSimple s!"payload_{fuel}_{index}")
          let fieldTerms : Array Term := fields.map fun field => ⟨field.raw⟩
          let fieldBinders ← fields.mapM fun field => `(binderIdent| $field:ident)
          let mut fieldBounds : Array (TSyntax `tactic) := #[]
          for ((_, rep), field) in variant.fields.zip fields do
            if let .int _ signed := rep then
              let bounds := rooted (if signed then
                ``LeanerIR.IntegerValueFits.signed_bounds else
                ``LeanerIR.IntegerValueFits.unsigned_bounds)
              fieldBounds := fieldBounds.push (← `(tactic|
                (have $(mkIdent `payloadBounds):ident := $bounds
                   (LeanerIR.SpecInt.fits $field:term)
                 leaner_cases $(mkIdent `payloadBounds):ident)))
          let constructor := rooted (name ++ Name.mkSimple variant.name)
          let value ← ``($constructor $fieldTerms*)
          let next ← arm ⟨owner, value, variant.name, fieldTerms⟩
          usesCall := usesCall || next.usesCall
          terms := terms.push (← `(Lean.Parser.Term.matchAltExpr|
            | $value:term => $(next.computation)))
          let caseName := mkIdent (Name.mkSimple variant.name)
          let mut verify := next.verify
          for bound in fieldBounds.reverse do
            verify ← `(tactic| ($bound:tactic; $verify:tactic))
          verifies := verifies.push (← `(tactic| case $caseName:ident $fieldBinders* =>
            simp_all (config := { failIfUnchanged := false }) only
              [$equation:term, $(rooted (name ++ `codec)):term, $(rooted (name ++ `erase)):term]
            leaner_native_hypotheses <;> $verify:tactic))
          preserves := preserves.push (← `(tactic| case $caseName:ident $fieldBinders* =>
            simp (config := { failIfUnchanged := false }) only [$equation:term]
            $(next.preserves):tactic))
          agreements := agreements.push (← `(tactic| case $caseName:ident $fieldBinders* =>
            simp (config := { failIfUnchanged := false }) only [$equation:term]
            $(next.agreement):tactic))
        return ⟨← `(term| (match $ownerValue:term with $terms:matchAlt*)),
          ← `(tactic| (cases $equation:ident : $ownerValue:term; $verifies:tactic*)),
          ← `(tactic| (cases $equation:ident : $ownerValue:term; $preserves:tactic*)),
          ← `(tactic| (cases $equation:ident : $ownerValue:term; $agreements:tactic*)), usesCall⟩
      unless (expression.getArg! 2).isAppOf ``Option.some do
        throwError "native scalar branch requires an else result"
      let yes ← emit fuel ghosts slots known (expression.getArg! 1)
      let no ← emit fuel ghosts slots known ((expression.getArg! 2).getArg! 1)
      let verify ← `(tactic|
        (rw [LeanerIR.Proofs.wp_branch]
         constructor
         · intro $(mkIdent `branchTrue):ident
           leaner_native_guard $(mkIdent `branchTrue):ident <;>
             $(← splitVariant slots (expression.getArg! 0) yes.verify):tactic
         · intro $(mkIdent `branchFalse):ident
           leaner_native_guard $(mkIdent `branchFalse):ident <;>
             $(← splitVariant slots (expression.getArg! 0) no.verify):tactic))
      if NativeCondition.supported (expression.getArg! 0) then
        let test ← NativeCondition.emit twins slots
          (fun rep value => pureValue (valueDepth value) slots known rep value)
          integerRep (expression.getArg! 0)
        let branchProof ← test.verifyWith verify
        let preserves ← `(tactic|
          (apply LeanerIR.Proofs.StatePreserving.bind
           · $(test.preserves):tactic
           · intro condition
             apply LeanerIR.Proofs.StatePreserving.branch condition
             · $(yes.preserves):tactic
             · $(no.preserves):tactic))
        let agreement ← `(tactic|
          (apply LeanerIR.Proofs.ComputationAgreement.fromFrame_branch_scalar
           · $(test.agreement):tactic
           · $(test.preserves):tactic
           · $(yes.agreement):tactic
           · $(no.agreement):tactic))
        return ⟨← ``(LeanerIR.Proofs.Spec.bind $(test.computation)
          (fun condition : Bool => if condition then $(yes.computation) else $(no.computation))),
          ← `(tactic| (rw [LeanerIR.Proofs.wp_bind]; $branchProof:tactic)),
          preserves, agreement, yes.usesCall || no.usesCall⟩
      let (test, conditionProof) ← condition slots known (expression.getArg! 0)
      let preserves ← `(tactic|
        (apply LeanerIR.Proofs.StatePreserving.branch $test
         · $(yes.preserves):tactic
         · $(no.preserves):tactic))
      let agreement ← `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.fromFrame_branch (test := $test)
         · $conditionProof:tactic
         · $(yes.agreement):tactic
         · $(no.agreement):tactic))
      return ⟨← ``(if $test then $(yes.computation) else $(no.computation)),
        verify, preserves, agreement, yes.usesCall || no.usesCall⟩
    if expression.isAppOfArity ``letNativeValue 3 then
      let initializer := expression.getArg! 1
      if initializer.isAppOfArity ``letNativeValue 3 && !NativeIndex.supported initializer then
        let associated := mkApp3 (mkConst ``letNativeValue) (initializer.getArg! 0)
          (initializer.getArg! 1)
          (mkApp3 (mkConst ``letNativeValue) (expression.getArg! 0)
            (initializer.getArg! 2) (expression.getArg! 2))
        let next ← emit fuel ghosts slots known associated
        let agreement ← `(tactic|
          (apply LeanerIR.Proofs.Spec.Equiv.trans
            (LeanerIR.Proofs.ComputationAgreement.fromFrame_congr _ _
              (LeanerIR.Proofs.ComputationAgreement.let_assoc _ _ _ _ _ _))
           $(next.agreement):tactic))
        return { next with agreement }
      let binder := expression.getArg! 0
      let pattern := binder.getArg! 1
      if !pattern.isAppOfArity ``LeanerIR.SemanticOperations.NativePattern.variable 1 then
        let (rep, value, evaluates) ← NativeAggregate.read twins signature.locals slots known
          (expression.getArg! 1)
        let boundSlots ← NativeAggregate.bindPattern twins signature.locals slots pattern rep value
        let next ← emit fuel ghosts boundSlots known (expression.getArg! 2)
        let codec ← rep.codecSyntax (mkIdent `codecs)
        let bound ← frame boundSlots
        let agreement ← `(tactic|
          (apply LeanerIR.Proofs.Spec.Equiv.trans
            (LeanerIR.Proofs.ComputationAgreement.fromFrame_congr _ _
              (LeanerIR.Proofs.ComputationAgreement.let_returns _ _
                (bound := $bound) (value := ($codec).encode $value) (by $evaluates:tactic) rfl))
           $(next.agreement):tactic))
        return { next with agreement }
      let some index := index? (pattern.getArg! 0) | throwError "invalid native binder index"
      unless index < slots.size do throwError "native binder lies outside the local row"
      -- Propagate a pure local alias directly. It needs a slot update only
      -- in execution agreement, not another native let or arithmetic atom.
      if initializer.isAppOfArity ``localVar 1 then
        let rep := signature.locals[index]!.rep
        let (value, evaluates) ← pureValue (valueDepth initializer) slots known rep initializer
        let encoded ← match rep with
          | .int .. => ``(LeanerIR.RuntimeValue.integer ($value).val)
          | .bool => ``(LeanerIR.RuntimeValue.bool $value)
          | .unit => ``(LeanerIR.RuntimeValue.unit)
          | .twin name #[] => ``($(rooted (name ++ `erase)) $value)
          | _ => do
              let codec ← rep.codecSyntax (mkIdent `codecs)
              ``(($codec).encode $value)
        let nextKnown := known.filter fun fact => (fact.expression.find? fun node =>
          node.isAppOfArity ``localVar 1 && index? (node.getArg! 0) == some index).isNone
        let nextSlots := slots.set! index (some value)
        let bound ← frame nextSlots
        let next ← emit fuel ghosts nextSlots nextKnown (expression.getArg! 2)
        let agreement ← `(tactic|
          (apply LeanerIR.Proofs.Spec.Equiv.trans
            (LeanerIR.Proofs.ComputationAgreement.fromFrame_congr _ _
              (LeanerIR.Proofs.ComputationAgreement.let_returns _ _
                (bound := $bound) (value := $encoded) (by $evaluates:tactic) rfl))
           $(next.agreement):tactic))
        return { next with agreement }
      let binderName := mkIdent (Name.str .anonymous
        (if slots[index]!.isSome then s!"local_{index}_write_{fuel}" else s!"local_{index}"))
      let nextSlots := slots.set! index (some (⟨binderName.raw⟩ : Term))
      let bound ← frame nextSlots
      -- A rebinding invalidates only variant facts that observe that slot.
      -- The initializer still uses the old facts and old local value.
      let nextKnown := known.filter fun fact => (fact.expression.find? fun node =>
        node.isAppOfArity ``localVar 1 && index? (node.getArg! 0) == some index).isNone
      let next ← emit fuel ghosts nextSlots nextKnown (expression.getArg! 2)
      let initializer := expression.getArg! 1
      if initializer.isAppOfArity ``nativeCall 4 ||
          initializer.isAppOfArity ``nativeBranch 3 ||
          NativeIndex.supported initializer ||
          NativeVector.isLength initializer ||
          (NativeAggregate.supported initializer && NativeEffects.required initializer) ||
          (signature.locals[index]!.rep == .bool && NativeCondition.supported initializer) then
        let rep := signature.locals[index]!.rep
        let ty ← rep.typeSyntax (mkIdent `Carrier)
        let codec ← rep.codecSyntax (mkIdent `codecs)
        let call ← if initializer.isAppOfArity ``nativeCall 4 then
            NativeCallValue.emit twins rep
              (fun rep value => pureValue (valueDepth value) slots known rep value)
              initializer (some (binderName, next.verify))
          else do
            let scalar ← if NativeVector.isLength initializer then
                vectorLength slots known rep initializer (some binderName)
              else if NativeCondition.supported initializer then
                NativeCondition.emitChoice twins slots
                  (fun rep value => pureValue (valueDepth value) slots known rep value)
                  integerRep rep initializer (some binderName)
              else
                NativeEffects.emit twins slots
                  (fun rep value => pureValue (valueDepth value) slots known rep value) rep initializer
                  (some binderName)
            let verify ← scalar.verifyWith next.verify
            pure (⟨scalar.computation, verify, scalar.preserves, scalar.agreement⟩ :
              NativeCallValue.Emitted)
        let agreement ← `(tactic|
          (apply LeanerIR.Proofs.ComputationAgreement.fromFrame_let_scalar
             (encode := fun value : $ty => ($codec).encode value)
             (bound := fun $binderName : $ty => $bound)
           · $(call.agreement):tactic
           · $(call.preserves):tactic
           · intro $binderName:ident; rfl
           · intro $binderName:ident
             $(next.agreement):tactic))
        let preserves ← `(tactic|
          (apply LeanerIR.Proofs.StatePreserving.bind
           · $(call.preserves):tactic
           · intro $binderName:ident
             $(next.preserves):tactic))
        return ⟨← ``(LeanerIR.Proofs.Spec.bind $(call.computation)
          (fun $binderName : $ty => $(next.computation))),
          ← `(tactic| (rw [LeanerIR.Proofs.wp_bind]; $(call.verify):tactic)),
          preserves, agreement, next.usesCall || initializer.getUsedConstants.contains ``nativeCall⟩
      if NativeCopy.supported initializer || (NativeCopy.localBorrow? initializer).isSome ||
          initializer.isAppOfArity ``localVar 1 ||
          initializer.isAppOfArity ``LeanerIR.Proofs.Denotation.value 1 ||
          (match signature.locals[index]!.rep with | .bool | .unit | .twin .. | .vector .. => true | _ => false) then
        let rep := signature.locals[index]!.rep
        let ty ← rep.typeSyntax (mkIdent `Carrier)
        let (value, evaluates) ← pureValue (valueDepth initializer) slots known rep initializer
        let encoded ← match rep with
          | .int .. => ``(LeanerIR.RuntimeValue.integer ($value).val)
          | .bool => ``(LeanerIR.RuntimeValue.bool $value)
          | .unit => ``(LeanerIR.RuntimeValue.unit)
          | .twin name #[] => ``($(rooted (name ++ `erase)) $value)
          | .vector .. => do
            let codec ← rep.codecSyntax (mkIdent `codecs)
            ``(($codec).encode $value)
          | _ => throwError "unsupported native pure local representation"
        let agreement ← `(tactic|
          (let $binderName : $ty := $value
           apply LeanerIR.Proofs.Spec.Equiv.trans
            (LeanerIR.Proofs.ComputationAgreement.fromFrame_congr _ _
              (LeanerIR.Proofs.ComputationAgreement.let_returns _ _
                (bound := $bound) (value := $encoded) (by $evaluates:tactic) rfl))
           $(next.agreement):tactic))
        return ⟨← ``(let $binderName : $ty := $value; $(next.computation)),
          ← `(tactic| (let $binderName : $ty := $value; $(next.verify):tactic)),
          ← `(tactic| (let $binderName : $ty := $value; $(next.preserves):tactic)),
          agreement, next.usesCall⟩
      if NativeExpression.nested initializer || NativeExpression.division initializer ||
          NativeExpression.multiplication initializer ||
          NativeExpression.bitwise initializer || NativeExpression.shift initializer ||
          (signature.locals[index]!.rep != integerRep &&
            initializer.isAppOfArity ``nativePrimitiveOperation 2) then
        let .int (.bits width) signed := signature.locals[index]!.rep
          | throwError "native arithmetic requires a fixed-width integer local"
        let nativeType ← signature.locals[index]!.rep.typeSyntax (mkIdent `Carrier)
        let first ← NativeExpression.emit ⟨nativeType, width, signed, slots, #[]⟩ initializer
          (if next.usesCall || NativeExpression.shift initializer then some binderName else none)
        let verifyFirst ← first.verifyWith next.verify
        let verify ← `(tactic| (rw [LeanerIR.Proofs.wp_bind]; $verifyFirst:tactic))
        let preserves ← `(tactic|
          (apply LeanerIR.Proofs.StatePreserving.bind
           · $(first.preserves):tactic
           · intro $binderName:ident
             $(next.preserves):tactic))
        let agreement ← `(tactic|
          (apply LeanerIR.Proofs.ComputationAgreement.fromFrame_let_scalar
             (encode := fun value : $nativeType => LeanerIR.RuntimeValue.integer value.val)
             (bound := fun $binderName : $nativeType => $bound)
           · $(first.agreement):tactic
           · $(first.preserves):tactic
           · intro $binderName:ident; rfl
           · intro $binderName:ident
             $(next.agreement):tactic))
        return ⟨← ``(LeanerIR.Proofs.Spec.bind $(first.computation)
          (fun $binderName : $nativeType => $(next.computation))), verify, preserves, agreement, next.usesCall⟩
      let (first, kind, value, operandsProof, success, failure) ← checked slots known initializer
      let computation ← ``(LeanerIR.Proofs.Spec.bind $first (fun $binderName : $nativeType => $(next.computation)))
      -- Name arithmetic results used by later calls, keeping their value
      -- equations explicit in the modular proof context.
      let verify ← if next.usesCall then `(tactic|
        (rw [LeanerIR.Proofs.wp_bind, LeanerIR.Proofs.NativeArithmetic.wp_checkedInteger_value]
         constructor
         · intro $binderName:ident $(mkIdent `valueEquation):ident
           $(next.verify):tactic
         · intro $(mkIdent `overflow):ident
           simp [LeanerIR.IntegerValueFits, LeanerIR.Ty.integerValueFits?, LeanerIR.Ty.integerBounds?]
             at $(mkIdent `overflow):ident
           all_goals simp (config := { failIfUnchanged := false }) only
             [LeanerIR.Proofs.NativeArithmetic.runtimeFailure, Prod.mk.injEq,
               Array.mk.injEq, List.cons.injEq, LeanerIR.RuntimeValue.integer.injEq,
               and_true, true_and] <;> leaner_certified_close!))
      else `(tactic|
        (rw [LeanerIR.Proofs.wp_bind, LeanerIR.Proofs.NativeArithmetic.wp_checkedInteger]
         constructor
         · intro $(mkIdent (Name.str .anonymous s!"fits_{fuel}")):ident
           simp [LeanerIR.IntegerValueFits, LeanerIR.Ty.integerValueFits?, LeanerIR.Ty.integerBounds?]
             at $(mkIdent (Name.str .anonymous s!"fits_{fuel}")):ident
           leaner_cases $(mkIdent (Name.str .anonymous s!"fits_{fuel}")):ident
           $(next.verify):tactic
         · intro $(mkIdent `overflow):ident
           simp [LeanerIR.IntegerValueFits, LeanerIR.Ty.integerValueFits?, LeanerIR.Ty.integerBounds?]
             at $(mkIdent `overflow):ident
           all_goals simp (config := { failIfUnchanged := false }) only
             [LeanerIR.Proofs.NativeArithmetic.runtimeFailure, Prod.mk.injEq,
               Array.mk.injEq, List.cons.injEq, LeanerIR.RuntimeValue.integer.injEq,
               and_true, true_and] <;> leaner_certified_close!))
      let preserves ← `(tactic|
        (apply LeanerIR.Proofs.StatePreserving.bind
         · exact LeanerIR.Proofs.NativeArithmetic.checkedInteger_preserves _ _ _ _
         · intro $binderName:ident
           $(next.preserves):tactic))
      let agreement ← `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.fromFrame_let_checkedInteger
           (bound := fun $binderName : $nativeType => $bound) (kind := $kind) (value := $value)
         · intro $(mkIdent `fits):ident
           apply LeanerIR.Proofs.ComputationAgreement.operation_value
           · $operandsProof:tactic
           · revert $(mkIdent `fits):ident
             $success:tactic
         · intro $(mkIdent `overflow):ident
           apply LeanerIR.Proofs.ComputationAgreement.operation_abort
           · $operandsProof:tactic
           · revert $(mkIdent `overflow):ident
             $failure:tactic
         · intro $binderName:ident
           rfl
         · intro $binderName:ident
           $(next.agreement):tactic))
      return ⟨computation, verify, preserves, agreement, next.usesCall⟩
    else if expression.isAppOfArity ``localVar 1 then
      let some index := index? (expression.getArg! 0) | throwError "invalid native result local"
      let some (some value) := slots[index]? | throwError "native result local is unavailable"
      let agreement ← `(tactic|
        (apply LeanerIR.Proofs.Spec.Equiv.trans ?_
          (LeanerIR.Proofs.encodeSpec_pure _ $value).symm
         apply LeanerIR.Proofs.ComputationAgreement.fromFrame_returns
         · exact LeanerIR.Proofs.ComputationAgreement.read _ _ _ rfl
         · rfl
         · $(← borrowFree slots):tactic))
      return ⟨← ``(LeanerIR.Proofs.Spec.pure $value),
        ← `(tactic| (rw [LeanerIR.Proofs.wp_pure]; leaner_certified_close!)),
        ← `(tactic| exact LeanerIR.Proofs.StatePreserving.pure _), agreement, false⟩
    else
      if NativeExpression.nested expression || NativeExpression.division expression ||
          NativeExpression.multiplication expression ||
          NativeExpression.bitwise expression || NativeExpression.shift expression then
        let some (width, signed) := integerShape | throwError "native arithmetic requires an integer representation"
        let scalar ← NativeExpression.emit ⟨nativeType, width, signed, slots, #[]⟩ expression
        let agreement ← `(tactic|
          (apply LeanerIR.Proofs.ComputationAgreement.fromFrame_scalar
           · $(scalar.agreement):tactic
           · intro result; rfl
           · $(← borrowFree slots):tactic))
        return ⟨scalar.computation, ← scalar.verifyWith (← `(tactic| leaner_certified_close!)),
          scalar.preserves, agreement, false⟩
      let (computation, _, _, operandsProof, success, failure) ← checked slots known expression
      let agreement ← `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.fromFrame_checkedInteger
         · $operandsProof:tactic
         · $success:tactic
         · $failure:tactic
         · intro $(mkIdent `result):ident; rfl
         · $(← borrowFree slots):tactic))
      let verify ← `(tactic|
        (apply (LeanerIR.Proofs.NativeArithmetic.wp_checkedInteger _ _ _ _ _ _ _).mpr
         constructor
         · intro $(mkIdent `fits):ident
           simp [LeanerIR.IntegerValueFits, LeanerIR.Ty.integerValueFits?, LeanerIR.Ty.integerBounds?]
             at $(mkIdent `fits):ident
           leaner_cases $(mkIdent `fits):ident
           leaner_certified_close!
         · intro $(mkIdent `overflow):ident
           simp [LeanerIR.IntegerValueFits, LeanerIR.Ty.integerValueFits?, LeanerIR.Ty.integerBounds?]
             at $(mkIdent `overflow):ident
           all_goals
             leaner_native_guard $(mkIdent `overflow):ident
             simp (config := { failIfUnchanged := false }) only [Int.reduceToNat]
               at $(mkIdent `overflow):ident
             first
             | (exfalso; omega)
             | leaner_certified_close!))
      return ⟨computation, verify,
        ← `(tactic| exact LeanerIR.Proofs.NativeArithmetic.checkedInteger_preserves _ _ _ _), agreement, false⟩
  let computation := rooted (base ++ `computation)
  let summary := rooted (base ++ `nativeSummary)
  let verified := rooted (base ++ `computationVerified)
  let preserves := rooted (base ++ `computationState)
  let represents := rooted (base ++ `computationRepresents)
  let names := #[`computation, `nativeSummary, `computationVerified,
    `computationState, `computationRepresents].map (base ++ ·)
  Perf.measureArtifacts s!"{moduleName}::{function} typed" names do
    let emitted ← emit (controlDepth body) #[] slots #[] body
    profileitM Exception s!"native computation {base}" (← getOptions) <| elabCommand (← `(def $computation ($args : $(rooted artifacts.argumentsType)) :
        LeanerIR.Proofs.Spec LeanerIR.RuntimeState LeanerIR.Proofs.Failure $resultType :=
      $(emitted.computation)))
    profileitM Exception s!"native VC {base}" (← getOptions) <| elabCommand (← `(theorem $summary : LeanerIR.Proofs.Satisfies $computation
        (LeanerIR.Proofs.Contract.withStateFrame $(rooted typedContract)) := by
      apply LeanerIR.Proofs.satisfies_of_wp
      intro $args:ident $(mkIdent `initial):ident $(mkIdent `permitted):ident
      simp only [$computation:term,
        LeanerIR.Proofs.Contract.withStateFrame, $(rooted typedContract):term,
        LeanerIR.Proofs.Contract.typed, $(rooted rawContract):term,
        $(rooted artifacts.argumentsCodec):term, $(rooted artifacts.resultsCodec):term]
          at $(mkIdent `permitted):ident ⊢
      leaner_cases $(mkIdent `permitted):ident
      $(emitted.verify):tactic))
    elabCommand (← `(theorem $verified : LeanerIR.Proofs.Satisfies $computation
        $(rooted typedContract) := LeanerIR.Proofs.satisfies_of_stateFrame $summary))
    profileitM Exception s!"native preservation {base}" (← getOptions) <| elabCommand (← `(theorem $preserves ($args : $(rooted artifacts.argumentsType)) :
        LeanerIR.Proofs.StatePreserving ($computation $args) := by
      change LeanerIR.Proofs.StatePreserving $(emitted.computation)
      $(emitted.preserves):tactic))
    profileitM Exception s!"native agreement {base}" (← getOptions) <| elabCommand (← `(theorem $represents ($(mkIdent `executable) : LeanerIR.Validation.ExecutableUnit) :
        LeanerIR.Proofs.Represents $(rooted artifacts.argumentsCodec)
          $(rooted artifacts.resultsCodec) $computation
          ($(rooted generated.denotation) $(mkIdent `executable)) := by
      intro $args:ident
      simp only [$computation:term]
      apply LeanerIR.Proofs.Spec.Equiv.trans
        (LeanerIR.Proofs.ComputationAgreement.function_fromFrame _ _ _ _ _ $(← frame slots) (by rfl))
      $(emitted.agreement):tactic))
  return true

end LeanerLang.NativeSequence
