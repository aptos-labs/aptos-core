-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Normalize

/-!
# Decoding literals

A runtime literal the contract reads through a twin's decoder — a
resource written as a nominal literal, a returned value through its codec
— decodes to the twin whose certificates the context proves.  The
evaluation runs the decoder's definitions on the literal, proves each
range certificate it leaves by the arithmetic of the context, and reduces
the rest: the twin and the proof of the decoding equation.
-/

namespace LeanerIR.Proofs.Denotation.RowSpec

open Lean Meta Elab Tactic

initialize registerTraceClass `leaner.decode

-- Re-encoding a decoded update must match the runtime's element update.
-- Only the decoder below reverses this direction, in its private inventory.
attribute [lir_data_norm] List.map_set

/-- The decoder's normalized pointwise identity does not traverse a
symbolic vector. Keep the `some` spelling, which remains after primitive
codec reduction, alongside the library's polymorphic `mapM_pure` law. -/
theorem decodeSomeList (values : List Native) : values.mapM some = some values := by
  induction values with
  | nil => rfl
  | cons value values ih =>
      simp only [List.mapM_cons, ih, bind, Option.bind_some, pure]

/-- Move an update back through its encoder, retaining a native vector
instead of traversing an erased list to reconstruct its elements. -/
theorem encodedList_set (encode : Native → RuntimeValue)
    (values : List Native) (index : Nat) (replacement : Native) :
    (values.map encode).set index (encode replacement) =
      (values.set index replacement).map encode := by
  induction values generalizing index with
  | nil => rfl
  | cons value values ih =>
    cases index with
    | zero => rfl
    | succ index => simp only [List.set, List.map_cons, ih]

/-- The reduced bound spelling used by the concrete Move vector decoder. -/
theorem decodedVectorBound (value : LeanerIR.SpecVector Native) :
    value.values.size < 18446744073709551616 := value.bounded

/-- The decoding closure of a twin: its own `decode?`, its fields', and the
runtime decoders they reach. -/
def decodeClosure (decode : Lean.Name) : MetaM (Array Lean.Name) :=
  unfoldClosureWith decode fun name =>
    name.getString! == "decode?" || name.getString! == "codec" ||
      (`LeanerIR.Proofs.Codec).isPrefixOf name ||
      [``LeanerIR.decodeInt?, ``LeanerIR.decodeBool?, ``LeanerIR.decodeString?,
        ``LeanerIR.decodeAddress?, ``LeanerIR.decodeSigner?, ``LeanerIR.decodeBytes?,
        ``LeanerIR.decodeUnit?].contains name

/-- Prove a certificate by the arithmetic of the context. -/
def proveFits (proposition : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let proposition ← instantiateMVars proposition
  -- A certificate observes an already chosen value; it must not invent an
  -- unresolved loan identity while solving an arithmetic side condition.
  if proposition.hasMVar then return none
  if proposition.isAppOfArity ``LeanerIR.IntegerValueFits 3 then
    let value := proposition.getArg! 2
    if value.isAppOfArity ``LeanerIR.SpecInt.val 3 then
      let proof ← mkAppM ``LeanerIR.SpecInt.fits #[value.getArg! 2]
      if ← isDefEq (← inferType proof) proposition then return some proof
  /- Do not ask `assumption` to unfold every callee's Satisfies theorem
  while looking for a range proof. Their relations can be very large. -/
  for declaration in ← getLCtx do
    if declaration.isImplementationDetail then continue
    let type ← instantiateMVars declaration.type
    if type == proposition then return some (mkFVar declaration.fvarId)
  let goal ← mkFreshExprMVar proposition
  try
    let remaining ← Term.TermElabM.run' do
      Tactic.run goal.mvarId! do
        withoutRecover <| evalTactic (← `(tactic|
          first
          | omega
          | (simp (config := { failIfUnchanged := false })
              [LeanerIR.IntegerValueFits, LeanerIR.Ty.integerValueFits?,
               LeanerIR.Ty.integerBounds?] <;> omega)))
    if remaining.isEmpty then return some (← instantiateMVars goal) else return none
  catch _ => return none

/-- The certificates a decoding leaves as conditions. -/
partial def certificates (e : Lean.Expr) (acc : Array Lean.Expr) : Array Lean.Expr :=
  let acc := if e.isAppOfArity ``LeanerIR.IntegerValueFits 3 && !e.hasLooseBVars &&
      !acc.contains e then acc.push e else acc
  match e with
  | .app f a => certificates a (certificates f acc)
  | .lam _ t b _ | .forallE _ t b _ => certificates b (certificates t acc)
  | .letE _ t v b _ => certificates b (certificates v (certificates t acc))
  | .mdata _ b => certificates b acc
  | .proj _ _ b => certificates b acc
  | _ => acc

/-- A decoding evaluated: the decoder application (`decode?` of a twin,
or `Codec.decode?` of its codec) on a nominal literal reduces to `some
twin`, its certificates proved by the context.  Returns the twin and the
proof of the reduction. -/
def evaluateDecoding (roots : Array Lean.Name) (application : Lean.Expr) :
    MetaM (Option (Lean.Expr × Lean.Expr)) := do
  let mut theorems ← getSimpTheorems
  -- The decoder moves updates back to native values. The default rule
  -- moves maps through updates in the other direction; keep this reversal
  -- local so neither the decoder nor surrounding normalization can loop.
  theorems := theorems.eraseCore (.decl ``List.map_set)
  for name in [``Function.comp_def, ``encodedList_set, ``decodeSomeList, ``decodedVectorBound] do
    theorems ← theorems.addConst name
  for root in roots do
    for name in ← decodeClosure root do
      theorems ← theorems.addDeclToUnfold name
  let ctx ← Simp.mkContext (simpTheorems := #[theorems])
    (congrTheorems := ← getSimpCongrTheorems)
  let simprocs ← Simp.getSimprocs
  trace[leaner.decode] "first start: {← IO.getNumHeartbeats}"
  let (first, _) ← Meta.simp application ctx #[simprocs]
  trace[leaner.decode] "first done: {← IO.getNumHeartbeats}"
  let mut facts := theorems
  for certificate in certificates first.expr #[] do
    let some proof ← proveFits certificate | return none
    facts ← facts.add (.other (← mkFreshUserName `fits)) #[] proof
  let ctx ← Simp.mkContext (simpTheorems := #[facts])
    (congrTheorems := ← getSimpCongrTheorems)
  let (second, _) ← Meta.simp first.expr ctx #[simprocs]
  trace[leaner.decode] "second done: {← IO.getNumHeartbeats}"
  /- A closed range certificate can simplify while its theorem is indexed.
  Expose the range predicate only if the ordinary symbolic pass did not
  finish; unfolding it eagerly hides symbolic certificates from simp. -/
  let second ← if second.expr.isAppOfArity ``Option.some 2 then pure second else do
    let literalFacts ← facts.addDeclToUnfold ``LeanerIR.IntegerValueFits
    let ctx ← Simp.mkContext (simpTheorems := #[literalFacts])
      (congrTheorems := ← getSimpCongrTheorems)
    let (finished, _) ← Meta.simp second.expr ctx #[simprocs]
    trace[leaner.decode] "literal done: {← IO.getNumHeartbeats}"
    second.mkEqTrans finished
  unless second.expr.isAppOfArity ``Option.some 2 do
    trace[leaner.decode] "unfinished decoding: {second.expr}"
    return none
  let result ← first.mkEqTrans second
  let proof ← match result.proof? with
    | some proof => pure proof
    | none => mkEqRefl application
  /- The twin as the clauses read it: a codec's packing leaves a redex the
  closer's rows would not see through. -/
  let twin ← Core.betaReduce (second.expr.getArg! 1)
  return some (twin, proof)

/-- Decode a literal aggregate before its codec expands into a symbolic
`mapM` or field matcher. Only the value argument is normalized here; the
decoder evaluator uses its separate, bounded definition closure. -/
simproc ↓ [lir_data_norm] decodeAggregateLiteral (Codec.decode? _ _) := fun e => do
  unless e.isAppOfArity ``Codec.decode? 4 do return .continue
  let value ← Simp.simp (e.getArg! 3)
  unless value.expr.isAppOfArity ``RuntimeValue.vector 1 ||
      value.expr.isAppOfArity ``RuntimeValue.nominal 3 do return .continue
  let roots := (e.getArg! 2).getUsedConstants
  let application := mkApp e.appFn! value.expr
  let some (_, decoded) ← evaluateDecoding roots application | return .continue
  let equality ← mkCongrArg e.appFn! (← value.getProof)
  let proof ← mkEqTrans equality decoded
  let some (_, _, rhs) := (← inferType proof).eq? | return .continue
  return .done { expr := rhs, proof? := some proof }

/-- The twin a nominal literal decodes to. -/
def decodeLiteral (decode : Lean.Name) (value : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let some (twin, _) ← evaluateDecoding #[decode] (mkApp (mkConst decode) value) | return none
  return some twin


end LeanerIR.Proofs.Denotation.RowSpec
