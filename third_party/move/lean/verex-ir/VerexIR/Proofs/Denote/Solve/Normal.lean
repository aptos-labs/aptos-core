-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.VectorSizes
import VerexIR.Proofs.Denote.NominalQuote
import VerexIR.Proofs.Denote.GivenOwn
import VerexIR.Proofs.Denote.Utf8

/-!
# The solver: the normal form

The simprocs and lemmas of the normalization sets (`vir_denote_norm`,
`vir_denote_eval`): literal evaluation, canonical family spellings, and the
encodings' normal forms. The deciders read these sets by name, so this
module does not import them.
-/

namespace VerexIR.Proofs.Denote

open FamilyTransport (respellOperation? inducedCarriers? spelledNTy spelledNRow spelledNRows substitutionRow?
  rowEntry? substituteNTy substituteNRow substituteNRows)

open Lean Elab Tactic Meta

private theorem differentNominalDeclarations (left right : NTy)
    (leftHandle rightHandle : Option StructHandle)
    (leftObserved : left.handle? = leftHandle) (rightObserved : right.handle? = rightHandle)
    (distinct : leftHandle ≠ rightHandle) : left ≠ right := by
  intro same
  apply distinct
  rw [← leftObserved, ← rightObserved, same]

/-- A resource key observes a declaration's identity at its own frame. Resolving
its parameters preserves that identity, without exposing its field layout. -/
private def nominalResourceHandle? (type : Lean.Expr) : MetaM (Option (Lean.Expr × Lean.Expr)) := do
  let type := type.consumeMData
  let resolved := type.isAppOfArity ``Skolems.resolve 4
  let owner := if resolved then type.getArg! 3 else type
  unless isNominalType (← getEnv) owner do return none
  let view ← nativeTypeView owner
  let isStruct := view.isAppOfArity ``NTy.struct 3
  unless isStruct || view.isAppOfArity ``NTy.enum 5 do return none
  let theoremName := if resolved then
      if isStruct then ``NTy.handle?_resolve_struct else ``NTy.handle?_resolve_enum
    else if isStruct then ``NTy.handle?_struct else ``NTy.handle?_enum
  let arguments := if resolved then #[type.getArg! 2] ++ view.getAppArgs else view.getAppArgs
  let proof ← mkAppM theoremName arguments
  let statement ← inferType proof
  let some (_, _, handle) := statement.eq? | return none
  return some (handle, proof)

/-- Distinct declaration identities suffice to separate resource types.
Inspect their handles, never their field layouts. -/
simproc [vir_denote_norm] nominalDeclarationEquality ((_ : NTy) = _) := fun e => do
  let_expr Eq _ left right := e | return .continue
  let some (leftHandle, leftObserved) ← nominalResourceHandle? left | return .continue
  let some (rightHandle, rightObserved) ← nominalResourceHandle? right | return .continue
  if leftHandle == rightHandle then return .continue
  let condition ← mkAppM ``Ne #[leftHandle, rightHandle]
  let distinct ← withDefault (mkDecideProof condition)
  let proof ← mkAppM ``differentNominalDeclarations
    #[left, right, leftHandle, rightHandle, leftObserved, rightObserved, distinct]
  return .done { expr := mkConst ``False, proof? := some (← mkAppM ``eq_false #[proof]) }

/-- A certificate observation emits the encoding at the same nominal owner
it observed. The structural equation exposes the own conjunct, but its raw
layout is only a view of that owner. Preserve every other argument verbatim. -/
private def nominalCertificateOwner (rhs view owner : Lean.Expr) : Lean.Expr := Id.run do
  unless rhs.isAppOfArity ``And 2 do return rhs
  let own := rhs.getArg! 0
  unless own.isAppOfArity ``Carriers.own 4 do return rhs
  let encoded := own.getArg! 3
  unless encoded.isAppOfArity ``NTy.encode 3 && encoded.getArg! 1 == view do return rhs
  let encoded := mkAppN encoded.getAppFn (encoded.getAppArgs.set! 1 owner)
  let own := mkAppN own.getAppFn (own.getAppArgs.set! 3 encoded)
  return mkApp2 rhs.getAppFn own (rhs.getArg! 1)

/-- Observe a named declaration at an operation boundary. Only the selected
operation's existing equations may inspect the descriptor; the surrounding
carrier types and the declaration's nested fields keep their names. -/
private def nominalOperation (e type : Lean.Expr)
    (structEquations enumEquations : Array Name) :
    SimpM Simp.Step := do
  unless isNominalType (← getEnv) type do return .continue
  let view ← nativeTypeView type
  let equations := if view.isAppOfArity ``NTy.struct 3 then structEquations else enumEquations
  for equation in equations do
    let result? ← withNewMCtxDepth do
      let proof ← mkConstWithFreshMVarLevels equation
      let (arguments, _, statement) ← forallMetaTelescopeReducing (← inferType proof)
      let (lhs, rhs) := if let some (_, lhs, rhs) := statement.eq? then (lhs, rhs)
        else if statement.isAppOfArity ``Iff 2 then (statement.getArg! 0, statement.getArg! 1)
        else (statement, mkConst ``True)
      unless ← withDefault (isDefEq lhs e) do return none
      -- Equations selecting handle equality carry a structural Boolean
      -- condition. Resolve only reflexive equations after observing the
      -- declaration, without asking the solver to inspect its values.
      for argument in arguments do
        unless argument.isMVar && !(← argument.mvarId!.isAssigned) do continue
        let some (_, left, right) := (← inferType argument).eq? | continue
        if ← withDefault (isDefEq left right) then
          argument.mvarId!.assign (← mkEqRefl left)
      let proof ← instantiateMVars (mkAppN proof arguments)
      let rhs ← instantiateMVars rhs
      let rhs := nominalCertificateOwner rhs view type
      -- A field write returns the original owner's plain value. Its row
      -- equation must construct Option.map at that same named result type,
      -- rather than introducing a structural row spelling in the map alone.
      let rhs ← if e.isAppOf ``Proj.set? && rhs.isAppOfArity ``Option.map 4 then do
          let resultType ← whnfR (← inferType e)
          if resultType.isAppOfArity ``Option 1 then
            let canonical := mkAppN rhs.getAppFn
              (rhs.getAppArgs.set! 1 resultType.appArg!)
            if ← withDefault (isDefEq rhs canonical) then pure canonical else pure rhs
          else pure rhs
        else pure rhs
      if proof.hasExprMVar || rhs.hasExprMVar || rhs == e then return none
      -- Use the existing theorem directly. Building a simp theorem here
      -- would publish an auxiliary lemma on every observation of an Iff.
      let proof ← if statement.eq?.isSome then pure proof
        else if statement.isAppOfArity ``Iff 2 then mkAppM ``propext #[proof]
        else mkAppM ``eq_true #[proof]
      return some ({ expr := rhs, proof? := some proof } : Simp.Result)
    if let some result := result? then
      return .visit result
  return .continue

simproc [vir_denote_norm] nominalEquality
    (@NTy.equal ?carriers _ _ _ _) := fun e => do
  nominalOperation e (e.getArg! 2)
    #[``NTy.equal_struct_handle, ``NTy.equal_struct] #[``NTy.equal_enum]

/-- The program's equality observes the named declaration's structural
comparison, as a clause's does (`nominalEquality`): a struct's row, a
variant's variants. -/
simproc [vir_denote, vir_denote_norm] nominalEqb
    (@NTy.eqb ?carriers _ _ _) := fun e => do
  nominalOperation e (e.getArg! 1) #[``NTy.eqb_struct] #[``NTy.eqb_enum]

/-- The structural order observes one declaration layer through the definition's
existing equations; field types remain named. -/
simproc ↓ [vir_denote_norm] nominalOrder
    (@NTy.order ?carriers _ _ _ _) := fun e => do
  nominalOperation e (e.getArg! 2)
    #[``NTy.order.eq_11] #[``NTy.order.eq_12]

theorem NTy.viewed_struct [Carriers] (prophecies : Bool) (source : StructHandle)
    (arguments fields : NRow) (values : (NTy.struct source arguments fields).carrier) :
    NTy.viewed prophecies (.struct source arguments fields) values =
      HList.viewed prophecies fields values := rfl
theorem NTy.viewed_enum [Carriers] (prophecies : Bool) (source : StructHandle)
    (arguments : NRow) (names : List String) (rows : NRows) (distinct : names.Nodup)
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    NTy.viewed prophecies (.enum source arguments names rows distinct) value =
      variantViewed prophecies names rows value := rfl

/-- A clause's view of a named declaration's value reads its fields' views. -/
simproc [vir_denote_norm] nominalViewed
    (@NTy.viewed ?carriers _ _ _) := fun e => do
  nominalOperation e (e.getArg! 2) #[``NTy.viewed_struct] #[``NTy.viewed_enum]

theorem NTy.observe_struct [Carriers] (source : StructHandle)
    (arguments fields : NRow) (values : (NTy.struct source arguments fields).carrier) :
    NTy.observe (.struct source arguments fields) values =
      HList.observe fields values := rfl
theorem NTy.observe_enum [Carriers] (source : StructHandle)
    (arguments : NRow) (names : List String) (rows : NRows) (distinct : names.Nodup)
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    NTy.observe (.enum source arguments names rows distinct) value =
      variantObserve names rows value := rfl
theorem NTy.unerase_struct [Carriers] (source : StructHandle)
    (arguments fields : NRow) (values : (NTy.struct source arguments fields).erase.carrier) :
    NTy.unerase (.struct source arguments fields) values =
      HList.unerase fields values := rfl
theorem NTy.unerase_enum [Carriers] (source : StructHandle)
    (arguments : NRow) (names : List String) (rows : NRows) (distinct : names.Nodup)
    (value : (NTy.enum source arguments names rows distinct).erase.carrier) :
    NTy.unerase (.enum source arguments names rows distinct) value =
      variantUnerase names rows value := rfl

simproc [vir_denote_norm] nominalObserve
    (@NTy.observe ?carriers _ _) := fun e => do
  nominalOperation e (e.getArg! 1) #[``NTy.observe_struct] #[``NTy.observe_enum]

simproc [vir_denote_norm] nominalUnerase
    (@NTy.unerase ?carriers _ _) := fun e => do
  nominalOperation e (e.getArg! 1) #[``NTy.unerase_struct] #[``NTy.unerase_enum]

/-- Settling a value of a named declaration leaves it unchanged (`NTy.settle`). -/
dsimproc ↓ [vir_denote, vir_denote_norm] nominalSettle
    (@VerexIR.Proofs.Denote.NTy.settle ?carriers _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.NTy.settle _ τ value := e | return .continue
  unless isNominalType (← getEnv) τ do return .continue
  return .done value

/-- Resource dispatch observes only the identity of a named declaration: at
its own frame, and where a frame resolves it, which keeps the declaration
(`nominalResourceHandle?`).  The resolution of a type that is no named
declaration, a type parameter's among them, is not observed. -/
simproc [vir_denote_norm] nominalHandle (NTy.handle? _) := fun e => do
  let type := (e.getArg! 0).consumeMData
  unless type.isAppOfArity ``Skolems.resolve 4 do
    return ← nominalOperation e (e.getArg! 0) #[``NTy.handle?_struct] #[``NTy.handle?_enum]
  let some (handle, observed) ← nominalResourceHandle? type | return .continue
  -- The equation is stated at the declaration's view; the term names it.
  let proof ← withDefault (mkExpectedTypeHint observed (← mkEq e handle))
  return .done { expr := handle, proof? := some proof }

/-- The head field of a named struct's value moved to the frame of its type
arguments is that field moved there (`Skolems.reframe_struct_fst`): the
declaration is observed at the projection, and the field keeps its type's
name. The pattern also reaches the primitive projection of a pair, which is
no application of `Prod.fst` and carries no argument to read: declined, as
`reframeStructFst` declines it. -/
simproc [vir_denote_norm] nominalReframeHead (Prod.fst _) := fun e => do
  let e := e.consumeMData
  unless e.isAppOfArity ``Prod.fst 3 do return .continue
  let moved := e.appArg!.consumeMData
  unless moved.isAppOfArity ``Skolems.reframe 8 do return .continue
  nominalOperation e (moved.getArg! 5) #[``Skolems.reframe_struct_fst] #[]

/-- And the rest of its fields are the rest of the row moved there
(`Skolems.reframe_struct_snd`), whose own head the structural law reads. A
primitive projection is declined here as it is at the head. -/
simproc [vir_denote_norm] nominalReframeRest (Prod.snd _) := fun e => do
  let e := e.consumeMData
  unless e.isAppOfArity ``Prod.snd 3 do return .continue
  let moved := e.appArg!.consumeMData
  unless moved.isAppOfArity ``Skolems.reframe 8 do return .continue
  nominalOperation e (moved.getArg! 5) #[``Skolems.reframe_struct_snd] #[]

/-- A named declaration carries an invariant slot (`NTy.invariantFree`). -/
simproc [vir_denote, vir_denote_norm] reduceNominalInvariantFree
    (VerexIR.Proofs.Denote.NTy.invariantFree _) := fun e => do
  let_expr VerexIR.Proofs.Denote.NTy.invariantFree τ := e | return .continue
  unless isNominalType (← getEnv) τ do return .continue
  let view ← nativeTypeView τ
  unless view.isAppOfArity ``NTy.struct 3 || view.isAppOfArity ``NTy.enum 5 ||
      view.isAppOfArity ``NTy.nominal 3 do return .continue
  let falseExpr := mkConst ``Bool.false
  let proof ← mkExpectedTypeHint (← mkEqRefl falseExpr) (← mkEq e falseExpr)
  return .done { expr := falseExpr, proof? := some proof }

simproc [vir_denote, vir_denote_norm] nominalPlaceRead
    (@Proj.get? ?carriers _ _ _ _ _ _) := fun e => do
  nominalOperation e (e.getArg! 3) #[``Proj.get?_field] #[``Proj.get?_variant]

simproc [vir_denote, vir_denote_norm] nominalPlaceWrite
    (@Proj.set? ?carriers _ _ _ _ _ _ _) := fun e => do
  nominalOperation e (e.getArg! 3) #[``Proj.set?_field] #[``Proj.set?_variant]

/-- Updating a local owner retains its mutation certificate at the named
owner, while the selected projection follows the declaration's field rule. -/
simproc [vir_denote, vir_denote_norm] nominalOwnerPlaceWrite
    (@Proj.setOwner? ?carriers _ _ _ _ _ _ _ _ _) := fun e => do
  let owner := e.getArg! 5
  let step ← nominalOperation e owner
    #[``Proj.setOwner?_field] #[``Proj.setOwner?_variant]
  let .visit result := step | return step
  let view ← nativeTypeView owner
  let expr := result.expr.replace fun term => if term == view then some owner else none
  return .visit { result with expr }


/-- A declaration pattern observes its own layer through the established
binding equations. Its context and nested field patterns remain named. -/
simproc [vir_denote, vir_denote_norm] nominalPatternBinding
    (@Pat.bindValue ?carriers _ _ _ _ _) := fun e => do
  nominalOperation e (e.getArg! 2) #[``Pat.bindValue_struct] #[``Pat.bindValue_variant]

-- Rewrite primitive literals before unfolding the dependent carrier match.
-- These equations preserve the literal directly, avoiding repeated reductions
-- of the full native type recursion when a continuation is normalized.
@[vir_denote_norm ↓] theorem ofGround_unit [Carriers] (value : Unit) :
    NTy.unit.ofGround rfl value = value := rfl
@[vir_denote_norm ↓] theorem ofGround_bool [Carriers] (value : Bool) :
    NTy.bool.ofGround rfl value = value := rfl
@[vir_denote_norm ↓] theorem ofGround_int [Carriers] (width : Nat) (signed : Bool)
    (value : (NTy.int width signed).groundCarrier) :
    (NTy.int width signed).ofGround rfl value = value := rfl
@[vir_denote_norm ↓] theorem ofGround_num [Carriers] (value : NTy.num.groundCarrier) :
    NTy.num.ofGround rfl value = value := rfl
@[vir_denote_norm ↓] theorem ofGround_address [Carriers] (value : NTy.address.groundCarrier) :
    NTy.address.ofGround rfl value = value := rfl
@[vir_denote_norm ↓] theorem ofGround_signer [Carriers] (value : NTy.signer.groundCarrier) :
    NTy.signer.ofGround rfl value = value := rfl
@[vir_denote_norm ↓] theorem ofGround_string [Carriers] (value : String) :
    NTy.string.ofGround rfl value = value := rfl
@[vir_denote_norm ↓] theorem ofGround_bytes [Carriers] (value : NTy.bytes.groundCarrier) :
    NTy.bytes.ofGround rfl value = value := rfl
@[vir_denote_norm ↓] theorem ofGround_char [Carriers] (value : NTy.char.groundCarrier) :
    NTy.char.ofGround rfl value = value := rfl

-- The same constructor-first rule avoids traversing a dependent result match.
@[vir_denote_norm ↓] theorem ofBody_one {unit : VerexIR.Validation.ValidatedUnit}
    [FunctionCarriers unit] [Skolems unit] (type : NTy) (value : type.carrier) :
    ResultShape.ofBody (.one type) value = value := rfl
@[vir_denote_norm ↓] theorem ofBody_none {unit : VerexIR.Validation.ValidatedUnit}
    [FunctionCarriers unit] [Skolems unit] (value : NTy.unit.carrier) :
    ResultShape.ofBody .none value = () := rfl

-- Reduce constructed slice readings without unfolding symbolic loans, so their
-- shared extent remains available to the size normalization lemmas.
@[vir_denote_norm] theorem sliceOfArrays_extent {α : Type} {extent : Nat}
    (current final : SpecArray α extent) :
    (SpecSliceRef.ofArrays current final).extent = extent := rfl

-- Settlement compares the dependent arrays themselves, before a caller reads
-- their vector contents. Expose constructed projections at that boundary too.
@[vir_denote_norm] theorem sliceOfArrays_current {α : Type} {extent : Nat}
    (current final : SpecArray α extent) :
    (SpecSliceRef.ofArrays current final).current = current := rfl

@[vir_denote_norm] theorem sliceOfArrays_final {α : Type} {extent : Nat}
    (current final : SpecArray α extent) :
    (SpecSliceRef.ofArrays current final).final = final := rfl

@[vir_denote_norm] theorem sliceWithCurrent_extent {α : Type}
    (value : SpecSliceRef α) (current : SpecArray α value.extent) :
    (value.withCurrent current).extent = value.extent := rfl

@[vir_denote_norm] theorem sliceWithCurrent_current {α : Type}
    (value : SpecSliceRef α) (current : SpecArray α value.extent) :
    (value.withCurrent current).current = current := rfl

@[vir_denote_norm] theorem sliceWithCurrent_final {α : Type}
    (value : SpecSliceRef α) (current : SpecArray α value.extent) :
    (value.withCurrent current).final = value.final := rfl

@[vir_denote_norm] theorem sliceCurrent_vector {α : Type} (value : SpecSliceRef α) :
    value.current.vector = value.currentVector := rfl

@[vir_denote_norm] theorem sliceFinal_vector {α : Type} (value : SpecSliceRef α) :
    value.final.vector = value.finalVector := rfl

-- A loop frame keeps the settled prophecy. Its equality preserves both
-- the element contents and the extent already shared by the typed carrier.
@[vir_denote_norm ↓] theorem sliceFinalFrame_iff {α : Type} (left right : SpecSliceRef α) :
    SpecSliceRef.settled left.finalVector = SpecSliceRef.settled right.finalVector ↔
      left.extent = right.extent ∧ left.finalVector.values = right.finalVector.values := by
  constructor
  · intro same
    have vectors : left.finalVector = right.finalVector := by
      simpa using congrArg SpecSliceRef.finalVector same
    exact ⟨by simpa using congrArg (fun vector => vector.values.size) vectors,
      congrArg SpecVector.values vectors⟩
  · rintro ⟨_, same⟩
    exact congrArg SpecSliceRef.settled (SpecVector.ext same)

-- Keep whole-vector equality for opaque specification functions as well as
-- element-array equality for the indexed-write rules.
theorem sliceArray_eq_final {α : Type}
    (value : SpecSliceRef α) (current : SpecArray α value.extent) :
    current = value.final ↔ current.vector = value.finalVector ∧
      current.vector.values = value.finalVector.values :=
  ⟨fun same => ⟨congrArg SpecArray.vector same,
      congrArg (fun array => array.vector.values) same⟩,
    fun same => SpecArray.ext same.1⟩

-- Preserve whole-array prophecy substitution for constructed slice borrows;
-- expose the contents only when the final projection belongs to a symbolic slice.
simproc ↓ [vir_denote_norm] sliceSettlement (@Eq _ _ (@SpecSliceRef.final _ _)) := fun e => do
  let_expr Eq _ current final := e | return .continue
  let_expr SpecSliceRef.final _ reference := final | return .continue
  if reference.isAppOf ``SpecSliceRef.ofArrays || reference.isAppOf ``SpecSliceRef.withCurrent ||
      reference.isAppOf ``SpecSliceRef.settled || reference.isAppOf ``SpecSliceRef.map then
    return .continue
  let lemma ← mkAppM ``sliceArray_eq_final #[reference, current]
  let some (_, result) := (← inferType lemma).iff? | return .continue
  return .visit { expr := result, proof? := some (← mkAppM ``propext #[lemma]) }

@[vir_denote_norm] theorem sliceOfArrays_currentVector {α : Type} {extent : Nat}
    (current final : SpecArray α extent) :
    (SpecSliceRef.ofArrays current final).currentVector = current.vector := rfl

@[vir_denote_norm] theorem sliceOfArrays_finalVector {α : Type} {extent : Nat}
    (current final : SpecArray α extent) :
    (SpecSliceRef.ofArrays current final).finalVector = final.vector := rfl

@[vir_denote_norm] theorem sliceWithCurrent_currentVector {α : Type}
    (value : SpecSliceRef α) (current : SpecArray α value.extent) :
    (value.withCurrent current).currentVector = current.vector := rfl

attribute [vir_denote_norm] SpecSliceRef.withCurrent_finalVector
  SpecSliceRef.map_currentVector SpecSliceRef.map_finalVector SpecSliceRef.map_extent

-- Keep symbolic encodings opaque: only the proved validity relation is
-- rewritten, never the decoder's recursive implementation on unknown bytes.
attribute [vir_denote_norm ↓] Utf8.valid_iff Utf8.bounded_valid_iff
  Utf8.encoding_exists_iff
attribute [vir_denote_norm] Utf8.validate_stringBytes

/-- Evaluate at most 64 literal bytes. The computed answer selects a result;
reflexivity certifies it by kernel reduction, with no native decision axiom.
An unknown vector or byte stops before unfolding the UTF-8 validator. -/
simproc [vir_denote_norm] literalUtf8Validity
    (ByteArray.validateUTF8 (Utf8.toByteArray _)) := fun e => do
  let_expr ByteArray.validateUTF8 encoded := e | return .continue
  let_expr Utf8.toByteArray vector := encoded | return .continue
  let vector ← whnf vector
  let_expr VerexIR.SpecVector.mk _ values _ := vector | return .continue
  let some (_, elements) := values.arrayLit? | return .continue
  if elements.length > 64 then return .continue
  let mut bytes := ByteArray.empty
  for element in elements do
    let value ← whnf (← mkAppM ``VerexIR.SpecInt.val #[element])
    let some value := value.int? | return .continue
    unless 0 ≤ value && value < 256 do return .continue
    bytes := bytes.push (UInt8.ofNat value.toNat)
  let result := toExpr bytes.validateUTF8
  let proof ← mkExpectedTypeHint (← mkEqRefl e) (← mkEq e result)
  return .done { expr := result, proof? := some proof }

/-- A structural update of an array literal at literal positions: the
positions move, the elements are not inspected. -/
private def reduceArrayLiteral (array : Lean.Expr) (positions : Array Lean.Expr)
    (update : List Lean.Expr → Array Nat → Option (List Lean.Expr)) : MetaM Simp.DStep := do
  let some (type, elements) := array.arrayLit? | return .continue
  let some positions := positions.mapM (·.nat?) | return .continue
  let some updated := update elements positions | return .continue
  return .visit (← mkArrayLit type updated)

/-- An in-bounds write into an array literal. -/
dsimproc [vir_denote_norm] reduceSetIfInBounds (Array.setIfInBounds _ _ _) := fun e => do
  let_expr Array.setIfInBounds _ array index value := e | return .continue
  reduceArrayLiteral array #[index] fun elements positions =>
    let index := positions[0]!
    some (if index < elements.length then elements.set index value else elements)

/-- An in-bounds exchange in an array literal. -/
dsimproc [vir_denote_norm] reduceSwapIfInBounds (Array.swapIfInBounds _ _ _) := fun e => do
  let_expr Array.swapIfInBounds _ array left right := e | return .continue
  reduceArrayLiteral array #[left, right] fun elements positions =>
    match elements[positions[0]!]?, elements[positions[1]!]? with
    | some first, some second =>
        some ((elements.set positions[0]! second).set positions[1]! first)
    | _, _ => some elements

/-- A slice of an array literal, its end clamped to the size. -/
dsimproc [vir_denote_norm] reduceExtract (Array.extract _ _ _) := fun e => do
  let_expr Array.extract _ array start stop := e | return .continue
  reduceArrayLiteral array #[start, stop] fun elements positions =>
    some ((elements.take positions[1]!).drop positions[0]!)

/-- A range reversal of an array literal. -/
dsimproc [vir_denote_norm] reduceReverseSlice (VerexIR.Proofs.Denote.reverseSlice _ _ _) :=
  fun e => do
    let_expr VerexIR.Proofs.Denote.reverseSlice _ array left right := e | return .continue
    reduceArrayLiteral array #[left, right] fun elements positions =>
      let left := positions[0]!
      let right := positions[1]!
      if right ≤ elements.length then
        some (elements.take left ++ ((elements.take right).drop left).reverse ++
          elements.drop right)
      else some elements

/-- Normalize only a bind's action. Its continuation is the rest of the
program: the solver reaches it through a folded continuation when the action
completes, so the normalizer never traverses it while a head is processed.
An action that completed hands the bind back to the rewrite rules that
reduce it. -/
private def bindActionStep (e : Lean.Expr) : Simp.SimpM Simp.Step := do
  let arguments := e.getAppArgs
  if arguments.size < 2 then return .continue
  let action := arguments[arguments.size - 2]!
  let next := arguments[arguments.size - 1]!
  let result ← Simp.simp action
  let head := mkAppN e.getAppFn (arguments.extract 0 (arguments.size - 2))
  let expr := mkApp2 head result.expr next
  let proof? ← result.proof?.mapM fun proof => do mkCongrFun (← mkCongrArg head proof) next
  let completed := match result.expr.getAppFn with
    | .const name _ => name == ``VerexIR.Proofs.Spec.pure || name == ``VerexIR.Proofs.Spec.abort ||
        name == ``VerexIR.Proofs.Denote.abortAt
    | _ => false
  if completed then return .continue (some { expr, proof? })
  return .done { expr, proof? }

simproc ↓ [vir_denote_norm] bindAction (VerexIR.Proofs.Spec.bind _ _) := bindActionStep

simproc ↓ [vir_denote_norm] flowBindAction (@VerexIR.Proofs.Denote.Flow.bind ?carriers _ _ _ _ _ _ _) :=
  bindActionStep

/-- The callees whose body compiled at a row the verification published, by
the unit they belong to: each the call's handle and its type arguments, as
the certificate `compileFunctionAt unit handle (some θ) = .ok …` the walk is
given states them. Recorded where that certificate is collected
(`collectInstances`), so the rewrite below never outruns it. -/
-- The branch's own state: a proof elaborating in a task of the command that
-- collected the row reads what that command recorded, and a record made
-- anywhere else is discarded, which leaves the rewrite unavailable rather
-- than unfounded.
initialize compiledRows : EnvExtension (Std.HashMap Name (Array (Lean.Expr × Lean.Expr))) ←
  registerEnvExtension (pure {}) (asyncMode := .local)

/-- Record that the callee `handle` of the unit `unit` has a body compiled at
the type arguments `θ`, whose certificate a caller's proof is given. -/
def recordCompiledRow (unit : Name) (handle θ : Lean.Expr) : CoreM Unit :=
  modifyEnv fun env => compiledRows.modifyState env fun rows =>
    rows.insert unit ((rows.getD unit #[]).push (handle, θ))

/-- The rows published for the unit `unit` names, none where it is no
constant of a verified unit. -/
def compiledRowsOf (unit : Lean.Expr) : MetaM (Array (Lean.Expr × Lean.Expr)) := do
  let some name := unit.constName? | return #[]
  return (compiledRows.getState (← getEnv)).getD name #[]

/-- Whether `rows` holds the callee `handle` at `θ`: the handle and the row
are compared as the certificate states them. -/
def compiledRow? (rows : Array (Lean.Expr × Lean.Expr)) (handle θ : Lean.Expr) :
    MetaM Bool := do
  for (candidate, row) in rows do
    if ← withoutModifyingState (withNewMCtxDepth
        (isDefEq candidate handle <&&> isDefEq row θ)) then
      return true
  return false

/-- A generic call whose callee has a body compiled at the call's type
arguments: its action is the callee's meaning at that row
(`specializedMeaning_closedGeneric_eq_at`), from which the instance route
inlines the body (`inlineCompiled`, its substitution branch). This is the one
place the head a generic call denotes is read, and it reads it before the
head unfolds: the normalization unfolds a head it leaves standing, where the
field body's bind stood, so every other call reads as that bind, term for
term (`doc/plan/one-spelling.md`, D2).

The callee's meaning is read through the field of the meanings the call is
denoted with (`Meanings.generic`), which the normalization has not yet
reduced when the head is first visited; the equation holds of the reduced
spelling, and the rewrite is stated at the term as it stands. -/
simproc ↓ [vir_denote] specializedGenericAtRow
    (@VerexIR.Proofs.Denote.specializedMeaning ?unit ?functions ?skolems _ _ _ _ _) :=
  fun e => do
  let debug := verex.solverDebug.get (← getOptions)
  let skip (reason : String) : SimpM Simp.Step := do
    if debug then IO.println s!"    call at a row: no, {reason}"
    return .continue
  unless e.isAppOfArity ``VerexIR.Proofs.Denote.specializedMeaning 8 do return .continue
  let rows ← compiledRowsOf (e.getArg! 0)
  if rows.isEmpty then return ← skip "no body at a row of this unit"
  let given := (e.getArg! 6).consumeMData
  let meaning ←
    if given.isAppOfArity ``VerexIR.Proofs.Denote.closedGeneric 10 then pure given
    else whnfR given
  unless meaning.isAppOfArity ``VerexIR.Proofs.Denote.closedGeneric 10 do
    return ← skip s!"the meaning is no closed generic one: {meaning}"
  let θ := e.getArg! 3
  -- The call's type arguments are the head's and the callee's meaning's: one
  -- binder of the field body, so one expression.
  unless meaning.getArg! 7 == θ || (← isDefEq (meaning.getArg! 7) θ) do
    return ← skip "the head and the meaning read other type arguments"
  unless ← compiledRow? rows (meaning.getArg! 5) θ do
    return ← skip s!"no body of {meaning.getArg! 5} at {θ}"
  -- The rows the callee reads at the arguments are the caller's: the
  -- equation holds where no argument carries a reference, which the
  -- arguments decide.
  let row ← mkAppOptM ``Subtype.val #[none, none, some θ]
  let refFree := mkApp (mkConst ``VerexIR.Proofs.Denote.NRow.refFree) row
  -- The arguments are a literal row: evaluated here, as the kernel evaluates
  -- the proof of the side condition. The normalization reads its own
  -- transparency, at which the predicate does not unfold.
  unless (← withDefault (whnf refFree)).isConstOf ``Bool.true do
    return ← skip "a type argument carries a reference"
  let free ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
    (← mkEq refFree (mkConst ``Bool.true))
  let proof := mkAppN (mkConst ``VerexIR.Proofs.Denote.specializedMeaning_closedGeneric_eq_at)
    #[meaning.getArg! 0, meaning.getArg! 1, meaning.getArg! 2, meaning.getArg! 3,
      meaning.getArg! 4, meaning.getArg! 5, meaning.getArg! 6, θ, free, meaning.getArg! 8,
      meaning.getArg! 9, e.getArg! 7]
  let some (_, _, rewritten) := (← instantiateMVars (← inferType proof)).eq?
    | throwError "the equation of a call at its row does not state one"
  if debug then IO.println s!"    call at a row: {meaning.getArg! 5} at {θ}"
  return .visit { expr := rewritten, proof? := some (← mkExpectedTypeHint proof (← mkEq e rewritten)) }

/-- An equation between values under the family a call's type arguments
induce, stated as the caller reads them: the equation of their caller views
(`NTy.ofSkolem`), the view in which the caller holds a callee's results. -/
simproc ↓ [vir_denote_norm] instantiatedEqInCallerView (@Eq _ _ _) :=
  fun e => do
    let_expr Eq carrier left right := e | return .continue
    -- Optional observations retain their underlying memory view. Expanding
    -- a mapped option equation exposes the value equation below; transporting
    -- the whole option first would recreate the same equation under an
    -- existential witness and make normalization repeat indefinitely.
    let some (outer, θ, τ) ← (do
        match carrier.getAppFn.constName?, carrier.getAppArgs with
        | some ``VerexIR.Proofs.Denote.Carriers.carrier, #[family, index] =>
            let some (θ, outer) := inducedCarriers? family | pure none
            pure (some (outer, θ, mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.param) index))
        | some ``VerexIR.Proofs.Denote.NTy.carrier, #[family, τ] =>
            let some (θ, outer) := inducedCarriers? family | pure none
            pure (some (outer, θ, τ))
        | _, _ => pure none : MetaM (Option (Lean.Expr × Lean.Expr × Lean.Expr)))
      | return .continue
    -- Without type parameters the two views hold the same values.
    if (τ.find? (·.isAppOf ``VerexIR.Proofs.Denote.NTy.param)).isNone then return .continue
    let transport := mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.ofSkolem)
      #[outer, θ, τ]
    let restated ← (try
        let equation ← mkEq (mkApp transport left) (mkApp transport right)
        let injective := mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.ofSkolem_inj)
          #[outer, θ, τ, left, right]
        let proof ← mkAppM ``propext #[← mkAppM ``Iff.symm #[injective]]
        if ← isTypeCorrect proof then pure (some (equation, proof)) else pure none
      catch _ => pure none)
    -- A side spelled at another type would make an ill-typed rewrite: say
    -- so and leave the equation as stated, rather than restate it wrongly.
    let some (equation, proof) := restated
      | do
        trace[Elab.definition] "verex: an equation in the caller's view \
          was left as stated: {e}"
        return .continue
    return .visit { expr := equation, proof? := some proof }



/-- Whether a row is spelled to its end, so that a position past it is known
to be past it. -/
private partial def spelledSpine (row : Lean.Expr) : Bool :=
  row.isConstOf ``VerexIR.Proofs.Denote.NRow.nil ||
    (row.isAppOfArity ``VerexIR.Proofs.Denote.NRow.cons 2 && spelledSpine (row.getArg! 1))

/-- The row a substitution is by, as spelled. -/
private def spelledRow (row : Lean.Expr) : MetaM Lean.Expr := do
  if row.isAppOfArity ``Subtype.val 3 then
    let inner ← whnfR (row.getArg! 2)
    if let some spelled := substitutionRow? inner then return spelled
  return row

/-- A substitution by a literal row, evaluated by the given evaluator or,
failing it, by reduction. -/
private def reduceSubst (substitute : Lean.Expr → Lean.Expr → Option Lean.Expr)
    (e row subject : Lean.Expr) : MetaM Simp.DStep := do
  if let some substituted := substitute (← spelledRow row) subject then
    if substituted != e then return .visit substituted
  let reduced ← whnfR e
  if reduced == e then return .continue
  return .visit reduced

/-- A substitution by a literal row evaluates, the whole type at once, so no
substitution is left in an argument the normalizer does not visit. -/
dsimproc ↓ [vir_denote, vir_denote_norm] reduceParameterSubst
    (VerexIR.Proofs.Denote.NTy.subst _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.NTy.subst row τ := e | return .continue
  let env ← getEnv
  reduceSubst (fun row subject => substituteNTy row subject (some env)) e row τ

/-- The mark of a spelled type dropped (`NTy.unmark`): a marked type is its
referent's, and a type spelled by any other constructor, or a named
declaration, carries none. A type that is not spelled keeps the application:
the definition's own equations would be tried at every such term, with a side
condition that it is no mark. -/
dsimproc ↓ [vir_denote, vir_denote_norm] reduceUnmark
    (VerexIR.Proofs.Denote.NTy.unmark _) := fun e => do
  let_expr VerexIR.Proofs.Denote.NTy.unmark τ := e | return .continue
  let spelled := τ.consumeMData
  if spelled.isAppOfArity ``VerexIR.Proofs.Denote.NTy.observed 1 then
    return .visit (mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.unmark) spelled.appArg!)
  let env ← getEnv
  let .const head _ := spelled.getAppFn | return .continue
  if isNominalType env spelled then return .done τ
  match env.find? head with
  | some (.ctorInfo info) =>
      if info.induct == ``VerexIR.Proofs.Denote.NTy then return .done τ else return .continue
  | _ => return .continue

mutual
/-- The type a shared view of a spelled type holds (`NTy.erase`), evaluated. A
named declaration holding no mutable reference keeps its name, at its erased
arguments; one holding some is written out, as its quotation writes it
(`quoteNative`). A type that is not spelled keeps the application. -/
private partial def erasedType (τ : Lean.Expr) : MetaM Lean.Expr := do
  let τ := τ.consumeMData
  match_expr τ with
  | VerexIR.Proofs.Denote.NTy.ref referent => erasedType referent
  | VerexIR.Proofs.Denote.NTy.sliceRef element =>
      return mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.slice) (← erasedType element)
  | VerexIR.Proofs.Denote.NTy.tuple elements =>
      return mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.tuple) (← erasedRow elements)
  | VerexIR.Proofs.Denote.NTy.struct source arguments fields =>
      return mkApp3 (mkConst ``VerexIR.Proofs.Denote.NTy.struct) source
        (← erasedRow arguments) (← erasedRow fields)
  | VerexIR.Proofs.Denote.NTy.enum source arguments names rows distinct =>
      return mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.enum)
        #[source, ← erasedRow arguments, names, ← erasedRows rows, distinct]
  | VerexIR.Proofs.Denote.NTy.slice element =>
      return mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.slice) (← erasedType element)
  | VerexIR.Proofs.Denote.NTy.vector element =>
      return mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.vector) (← erasedType element)
  | VerexIR.Proofs.Denote.NTy.array element length =>
      return mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.array) (← erasedType element) length
  | VerexIR.Proofs.Denote.NTy.table key value =>
      return mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.table) (← erasedType key)
        (← erasedType value)
  | VerexIR.Proofs.Denote.NTy.ghost slot =>
      return mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.ghost) (← erasedType slot)
  | VerexIR.Proofs.Denote.NTy.observed referent =>
      return mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.observed) (← erasedType referent)
  | VerexIR.Proofs.Denote.NTy.unit => return τ
  | VerexIR.Proofs.Denote.NTy.bool => return τ
  | VerexIR.Proofs.Denote.NTy.int _ _ => return τ
  | VerexIR.Proofs.Denote.NTy.num => return τ
  | VerexIR.Proofs.Denote.NTy.address => return τ
  | VerexIR.Proofs.Denote.NTy.signer => return τ
  | VerexIR.Proofs.Denote.NTy.string => return τ
  | VerexIR.Proofs.Denote.NTy.bytes => return τ
  | VerexIR.Proofs.Denote.NTy.char => return τ
  | VerexIR.Proofs.Denote.NTy.param _ => return τ
  | VerexIR.Proofs.Denote.NTy.function _ _ _ => return τ
  | VerexIR.Proofs.Denote.NTy.nominal _ _ _ => return τ
  | _ =>
      unless isNominalType (← getEnv) τ do
        return mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.erase) τ
      let view ← nativeTypeView τ
      let erased ← erasedType view
      if erased == view then return τ
      let named := mkAppN τ.getAppFn (← τ.getAppArgs.mapM erasedType)
      if (← nativeTypeView named) == erased then return named
      return erased

private partial def erasedRow (row : Lean.Expr) : MetaM Lean.Expr := do
  let row := row.consumeMData
  match_expr row with
  | VerexIR.Proofs.Denote.NRow.nil => return row
  | VerexIR.Proofs.Denote.NRow.cons head tail =>
      return mkApp2 (mkConst ``VerexIR.Proofs.Denote.NRow.cons) (← erasedType head)
        (← erasedRow tail)
  | _ => return mkApp (mkConst ``VerexIR.Proofs.Denote.NRow.erase) row

private partial def erasedRows (rows : Lean.Expr) : MetaM Lean.Expr := do
  let rows := rows.consumeMData
  match_expr rows with
  | VerexIR.Proofs.Denote.NRows.nil => return rows
  | VerexIR.Proofs.Denote.NRows.cons fields tail =>
      return mkApp2 (mkConst ``VerexIR.Proofs.Denote.NRows.cons) (← erasedRow fields)
        (← erasedRows tail)
  | _ => return mkApp (mkConst ``VerexIR.Proofs.Denote.NRows.erase) rows
end

/-- A spelled type's shared view, evaluated (`erasedType`): the definition is
reducible, so its application is not indexed by its own equations, and a named
declaration's is not reached by them at all. -/
dsimproc ↓ [vir_denote, vir_denote_norm] reduceErase
    (VerexIR.Proofs.Denote.NTy.erase _) := fun e => do
  let_expr VerexIR.Proofs.Denote.NTy.erase τ := e | return .continue
  let erased ← erasedType τ
  if erased == e.consumeMData then return .continue
  return .done erased

dsimproc ↓ [vir_denote, vir_denote_norm] reduceEraseRow
    (VerexIR.Proofs.Denote.NRow.erase _) := fun e => do
  let_expr VerexIR.Proofs.Denote.NRow.erase row := e | return .continue
  let erased ← erasedRow row
  if erased == e.consumeMData then return .continue
  return .done erased

dsimproc ↓ [vir_denote, vir_denote_norm] reduceEraseRows
    (VerexIR.Proofs.Denote.NRows.erase _) := fun e => do
  let_expr VerexIR.Proofs.Denote.NRows.erase rows := e | return .continue
  let erased ← erasedRows rows
  if erased == e.consumeMData then return .continue
  return .done erased

/-- A frame's parameter replacement constructs the same named declaration
at its replaced arguments. Only the explicit arguments are traversed; the
field layout stays behind the descriptor. Unknown argument syntax retains
its substitution rather than being assumed independent of parameters. A
parameter stands for its entry with the mark dropped (`NTy.substWith`), so
the replacement spells that drop; `NTy.unmark` of a spelled entry evaluates
(`reduceUnmark`), and of a frame's reading is itself
(`Skolems.unmark_type`). -/
dsimproc ↓ [vir_denote, vir_denote_norm] reduceNominalParameterReplacement
    (NTy.substWith _ _) := fun e => do
  let_expr NTy.substWith types owner := e | return .continue
  let env ← getEnv
  unless isNominalType env owner && spelledNTy owner (some env) do return .continue
  let replaced := owner.replace fun term =>
    if term.isAppOfArity ``NTy.param 1 then
      some (mkApp (mkConst ``NTy.unmark) (mkApp types term.appArg!).headBeta)
    else none
  return .visit replaced

/-- The substitution of a literal row evaluates (`reduceParameterSubst`). -/
dsimproc ↓ [vir_denote, vir_denote_norm] reduceRowSubst
    (VerexIR.Proofs.Denote.NRow.subst _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.NRow.subst row fields := e | return .continue
  let env ← getEnv
  reduceSubst (fun row subject => substituteNRow row subject (some env)) e row fields

/-- The substitution of literal variant rows evaluates (`reduceParameterSubst`). -/
dsimproc ↓ [vir_denote, vir_denote_norm] reduceRowsSubst
    (VerexIR.Proofs.Denote.NRows.subst _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.NRows.subst row rows := e | return .continue
  let env ← getEnv
  reduceSubst (fun row subject => substituteNRows row subject (some env)) e row rows

/-- A transport between types that agree, such as one by a generic site's
compile-time evidence (`Term.callGeneric`, `Term.closureGeneric`): the value
itself. -/
dsimproc ↓ [vir_denote, vir_denote_norm] resolvedCast (cast _ _) := fun e => do
  let_expr cast α β _ value := e | return .continue
  unless ← withTransparency .default (isDefEq α β) do return .continue
  return .visit value

/-- The typed reads of a type at a family (`NTy.read` and its carrier,
instances, write, read-back, and view). -/
private def typedReads : List Name :=
  [``VerexIR.Proofs.Denote.NTy.readCarrier, ``VerexIR.Proofs.Denote.NTy.read,
   ``VerexIR.Proofs.Denote.NTy.readDecEq, ``VerexIR.Proofs.Denote.NTy.readInhabited]

private def typedReadSpelling (head : Name) (family : Lean.Expr)
    (τ : Lean.Expr) (rest : Array Lean.Expr) :
    MetaM (Option Lean.Expr) := do
  let τ := τ.consumeMData
  let atFamily (τ : Lean.Expr) : MetaM Lean.Expr := do
    if head != ``VerexIR.Proofs.Denote.NTy.readInhabited then
      return mkAppN (mkConst head) (#[family, τ] ++ rest)
    let some held := rest[0]? | return mkAppN (mkConst head) (#[family, τ] ++ rest)
    let values := rest.extract 1 rest.size
    let inhabitable := mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.inhabitable) τ
    let held ← if ← withDefault (isDefEq inhabitable (mkConst ``Bool.true)) then
        mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
          (← mkEq inhabitable (mkConst ``Bool.true))
      else pure held
    return mkAppN (mkConst head) (#[family, τ, held] ++ values)
  if τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.param 1 then return some (← atFamily τ)
  let int := τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.int 2
  let num := τ.isConstOf ``VerexIR.Proofs.Denote.NTy.num
  unless int || num || isNominalType (← getEnv) τ ||
      τ.getAppFn.constName?.any (·.getPrefix == ``VerexIR.Proofs.Denote.NTy) do
    return none
  let integer := int || num
  let intType := mkConst ``Int
  let carrier := mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.carrier) family τ
  let inhabited? : Option (Lean.Expr × Array Lean.Expr) := do
    let proof ← rest[0]?
    pure (proof, rest.extract 1 rest.size)
  let canonicalProof (proof : Lean.Expr) : MetaM Lean.Expr := do
    let inhabitable := mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.inhabitable) τ
    if ← withDefault (isDefEq inhabitable (mkConst ``Bool.true)) then
      mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
        (← mkEq inhabitable (mkConst ``Bool.true))
    else pure proof
  let nativeInhabited (proof : Lean.Expr) : MetaM Lean.Expr := do
    let proof ← canonicalProof proof
    let fallback := mkApp3 (mkConst ``VerexIR.Proofs.Denote.NTy.inhabitantOf) family τ proof
    pure (mkApp3 (mkConst ``VerexIR.Proofs.Denote.nativeInhabited) family τ fallback)
  if head == ``VerexIR.Proofs.Denote.NTy.readCarrier then
    return some (mkAppN (if integer then intType else carrier) rest)
  if head == ``VerexIR.Proofs.Denote.NTy.read then
    let read := if int then
        mkApp2 (mkConst ``VerexIR.SpecInt.val)
          (mkApp (mkConst ``VerexIR.IntWidth.bits) (τ.getArg! 0)) (τ.getArg! 1)
      else if num then
        mkApp2 (mkConst ``VerexIR.SpecInt.val) (mkConst ``VerexIR.IntWidth.unbounded)
          (mkConst ``Bool.true)
      else .lam `value carrier (.bvar 0) .default
    return some (mkAppN read rest).headBeta
  if head == ``VerexIR.Proofs.Denote.NTy.readDecEq then
    let inst ← if integer then synthInstance (← mkAppM ``DecidableEq #[intType])
      else pure (mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.decEq) family τ)
    return some (mkAppN inst rest)
  if head == ``VerexIR.Proofs.Denote.NTy.readInhabited then
    let some (proof, rest) := inhabited? | return none
    let inst ← if integer then synthInstance (← mkAppM ``Inhabited #[intType])
      else nativeInhabited proof
    return some (mkAppN inst rest)
  return some (← atFamily τ)

/-- Evaluate a typed read from its original native type. Constructor equations
can unfold a nominal descriptor while selecting their identity branch; retain
that descriptor in the identity's binder instead. -/
dsimproc ↓ [vir_denote, vir_denote_norm] typedReadValue
    (@VerexIR.Proofs.Denote.NTy.read ?carriers _) := fun e => do
  let args := e.getAppArgs
  let some family := args[0]? | return .continue
  let some τ := args[1]? | return .continue
  let env ← getEnv
  let τ ← NativeMaps.nativeTypeHead τ
  unless isNominalType env τ ||
      τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.struct 3 ||
      τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.enum 5 do return .continue
  let some read ← typedReadSpelling ``VerexIR.Proofs.Denote.NTy.read family τ
      (args.extract 2 args.size) | return .continue
  if read == e then return .continue
  return .visit read

/-- Evaluate a typed value's native value (`NTy.unread`) from its original
native type: at a named declaration the value itself, the inverse of the
identity `typedReadValue` reads it by. -/
dsimproc ↓ [vir_denote, vir_denote_norm] typedUnreadValue
    (@VerexIR.Proofs.Denote.NTy.unread ?carriers _ _) := fun e => do
  let args := e.getAppArgs
  let some family := args[0]? | return .continue
  let some τ := args[1]? | return .continue
  unless args.size ≥ 3 do return .continue
  let env ← getEnv
  let τ ← NativeMaps.nativeTypeHead τ
  unless isNominalType env τ ||
      τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.struct 3 ||
      τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.enum 5 do return .continue
  let carrier := mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.carrier) family τ
  let unread : Lean.Expr := .lam `value carrier (.bvar 0) .default
  return .visit (mkAppN unread (args.extract 3 args.size)).headBeta

/-- A result shape as the constructor it reduces to, with that
constructor's arguments: the shape a function's result type names is the
generated `<fn>.shape` abbreviation, which reduction unfolds and matching
does not. -/
private def resultShape? (shape : Lean.Expr) : MetaM (Option (Name × Array Lean.Expr)) := do
  if shape.hasLooseBVars then return none
  let reduced ← whnfD shape
  let some constructor := reduced.getAppFn.constName? | return none
  unless constructor == ``VerexIR.Proofs.Denote.ResultShape.none ||
      constructor == ``VerexIR.Proofs.Denote.ResultShape.one do
    return none
  return some (constructor, reduced.getAppArgs)

/-- The domain a call binds its result at, from the result type its meaning
states, the carrier of a result shape at a family: `Unit` without a result,
and with one the carrier of its native type, at the family the call is made
at. A callee's type arguments are substituted into that type, those of the
shape and, where the family is one type arguments induce, the family's, so
the domain is at the family below them. None where the type is not the
carrier of a shape that reduces to a constructor. -/
def callResultDomain? (result : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let result := result.consumeMData
  unless result.isAppOfArity ``VerexIR.Proofs.Denote.ResultShape.carrier 2 do return none
  let env ← getEnv
  let shape := (result.getArg! 1).consumeMData
  -- A substitution by a spelled row into a shape is evaluated on the shape's
  -- type; any other shape is read as the constructor it reduces to.
  let substituted? ← if shape.isAppOfArity ``VerexIR.Proofs.Denote.ResultShape.subst 2 then do
      let row ← spelledRow (shape.getArg! 0)
      if spelledSpine row then
        match ← resultShape? (shape.getArg! 1) with
        | some (constructor, #[τ]) =>
            if constructor == ``VerexIR.Proofs.Denote.ResultShape.one then
              pure ((substituteNTy row τ (some env)).map fun τ' =>
                (``VerexIR.Proofs.Denote.ResultShape.one, #[τ']))
            else pure none
        | some (constructor, #[]) => pure (some (constructor, #[]))
        | _ => pure none
      else pure none
    else pure none
  let some (constructor, arguments) ← match substituted? with
      | some resolved => pure (some resolved)
      | none => resultShape? shape
    | return none
  if constructor == ``VerexIR.Proofs.Denote.ResultShape.none then return some (mkConst ``Unit)
  let #[τ] := arguments | return none
  let mut family := result.getArg! 0
  let mut τ := τ
  repeat
    let some (θ, outer) := inducedCarriers? family | break
    let some row := substitutionRow? θ | break
    let some τ' := substituteNTy row τ (some env) | break
    family := outer
    τ := τ'
  return some (mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.carrier) family τ)

/-- Literal scalar arguments of a map construction, without unfolding named
values or maps. These are the closed constructions the typed map evaluator
can materialize at a call's specialization boundary. -/
private def primitiveMapLiteral (type value : Lean.Expr) : Bool :=
  let value := value.consumeMData
  if type.isConstOf ``Int then
    value.int?.isSome ||
      ((value.isAppOfArity ``Int.ofNat 1 || value.isAppOfArity ``Int.negSucc 1) &&
        value.appArg!.isRawNatLit)
  else if type.isConstOf ``Nat then value.nat?.isSome || value.isRawNatLit
  else if type.isConstOf ``Bool then value.isConstOf ``Bool.true || value.isConstOf ``Bool.false
  else type.isConstOf ``String && value.isStringLit

private def literalMapConstruction (e : Lean.Expr) : Bool := Id.run do
  unless e.isAppOfArity ``VerexIR.SpecMap.updateAll 7 &&
      !e.hasFVar && !e.hasMVar && !e.hasLooseBVars do return false
  let args := e.getAppArgs
  unless [``Int, ``Nat, ``Bool, ``String].any args[0]!.isConstOf do return false
  let discipline := args[3]!
  unless discipline.isAppOfArity ``VerexIR.SpecMap.Discipline.sequence 1 ||
      (discipline.isAppOfArity ``VerexIR.SpecMap.Discipline.ordered 2 &&
        discipline.appArg!.isConstOf ``VerexIR.SpecMap.intOrder) do return false
  let some (_, entries) := args[4]!.listLit? | return false
  let some (_, keys) := args[5]!.listLit? | return false
  let some (_, values) := args[6]!.listLit? | return false
  return keys.all (primitiveMapLiteral args[0]!) &&
    values.all (primitiveMapLiteral args[1]!) &&
    entries.all fun entry => entry.isAppOfArity ``Prod.mk 4 &&
      primitiveMapLiteral args[0]! (entry.getArg! 2) &&
      primitiveMapLiteral args[1]! (entry.getArg! 3)

/-- Upgrade a base function-carrier family (`<pkg>.functionCarriersBase`) to the
full unit function-carrier family (`<pkg>.functionCarriers`) on own-independent
carrier and value operations once `<pkg>.functionCarriers` is registered. -/
private def upgradeBaseFamily? (env : Environment) (family : Lean.Expr) : Option Lean.Expr := do
  let f := family.consumeMData
  guard (f.isAppOfArity ``VerexIR.Proofs.Denote.Skolems.toCarriers 3)
  let unit := f.getArg! 0
  let base := f.getArg! 1
  let frame := (f.getArg! 2).consumeMData
  let .const baseName levels := base | none
  let .str pkg "functionCarriersBase" := baseName | none
  let fullName := Name.str pkg "functionCarriers"
  guard (env.contains fullName)
  let full := mkConst fullName levels
  if frame.isAppOfArity ``VerexIR.Proofs.Denote.Skolems.runtime 2 && frame.getArg! 1 == base then
    some (mkApp3 f.getAppFn unit full (mkApp2 frame.getAppFn unit full))
  else if frame.isAppOfArity ``VerexIR.Proofs.Denote.Skolems.atArguments 4 &&
      frame.getArg! 1 == base then
    some (mkApp3 f.getAppFn unit full
      (mkAppN frame.getAppFn #[unit, full, frame.getArg! 2, frame.getArg! 3]))
  else
    none

private def hasBaseFamilyConstant (env : Environment) (e : Lean.Expr) : Bool :=
  (e.find? fun t => match t with
    | .const (.str pkg "functionCarriersBase") _ => env.contains (Name.str pkg "functionCarriers")
    | _ => false).isSome

/-- The arguments a rebuilding branch keeps stand over the arguments it
changed, so the types expected of them changed with them. Check that
boundary where it is crossed: from `boundary` on, each argument's own type
converts with the type the rebuilt application expects of it. The check
inspects the two types, not the value or proof they describe. -/
private def keptArgumentsConvert (fn : Lean.Expr) (args : Array Lean.Expr)
    (boundary : Nat) : MetaM Bool := do
  if boundary ≥ args.size then return true
  let functionType ← inferType fn
  -- The prefix is instantiated once and one binder peeled per step, through
  -- `instantiateForall`, which unfolds a body that is no `forallE` as the
  -- restart from the whole type did.
  let mut bound ← instantiateForall functionType (args.extract 0 boundary)
  for index in [boundary:args.size] do
    let .forallE _ expected _ _ := bound | return false
    if expected.hasLooseBVars then return false
    let actual ← inferType args[index]!
    unless expected == actual || (← withNewMCtxDepth (isDefEq actual expected)) do return false
    if index + 1 < args.size then bound ← instantiateForall bound #[args[index]!]
  return true

private def isCarrierFamilyHead (name : Name) : Bool :=
  name == ``VerexIR.Proofs.Denote.Skolems.toCarriers ||
  name == ``VerexIR.Proofs.Denote.Carriers.instantiate ||
  name == ``VerexIR.Proofs.Denote.Skolems.instantiate ||
  name == `VerexIR.Proofs.Denote.GivenFrame.toCarriers

/-- A candidate outer spelling, including implicit type arguments ordinary
rewriting does not reach. -/
private def canonicalFamilies (e : Lean.Expr) : MetaM Lean.Expr := do
  let env ← getEnv
  let substituteNTy := fun row τ => FamilyTransport.substituteNTy row τ (some env)
  let substituteNRow := fun row fields => FamilyTransport.substituteNRow row fields (some env)
  let substituteNRows := fun row rows => FamilyTransport.substituteNRows row rows (some env)
  let instantiated := (e.find? fun t => t.isConstOf ``VerexIR.Proofs.Denote.Skolems.instantiate ||
      t.isConstOf ``VerexIR.Proofs.Denote.Carriers.instantiate ||
      t.isConstOf ``VerexIR.Proofs.Denote.NTy.subst || t.isConstOf ``VerexIR.Proofs.Denote.NRow.subst ||
      t.isConstOf ``VerexIR.Proofs.Denote.NRows.subst ||
      t.isConstOf ``VerexIR.Proofs.Denote.ResultShape.subst ||
      t.constName?.any typedReads.contains).isSome
  let mapInputs := (e.find? fun t =>
    t.isAppOfArity ``VerexIR.SpecMap.ofEntries 7 &&
      ((t.getArg! 6).isAppOfArity ``Prod.fst 3 ||
        (t.getArg! 6).isAppOfArity ``Prod.snd 3)).isSome
  let literalMaps := (e.find? literalMapConstruction).isSome
  let givenOperations := (e.find? fun t => (givenOperationProjection? t).isSome).isSome
  let baseFamilies := hasBaseFamilyConstant env e
  unless instantiated || mapInputs || literalMaps || givenOperations || baseFamilies do
    return e
  let ownFreeBaseHeads : Array Name := #[
    ``VerexIR.Proofs.Denote.NTy.carrier,
    ``VerexIR.Proofs.Denote.HList,
    ``VerexIR.Proofs.Denote.variantCarrier,
    ``VerexIR.Proofs.Denote.NTy.plainFallback,
    ``VerexIR.Proofs.Denote.NTy.readCarrier,
    ``VerexIR.Proofs.Denote.NTy.read,
    ``VerexIR.Proofs.Denote.NTy.readDecEq,
    ``VerexIR.Proofs.Denote.NTy.readInhabited,
    ``VerexIR.Proofs.Denote.NTy.readOrder
  ]
  -- This is a syntactic pass. Keep binders closed instead of rebuilding their
  -- telescopes; only closed expressions may reach a meta-level reduction.
  Core.transform e (post := fun t => do
    let args := t.getAppArgs
    let some head := t.getAppFn.constName? | return .continue
    if baseFamilies && !t.hasLooseBVars && ownFreeBaseHeads.contains head then
      if let some family := args[0]? then
        if let some upgraded := upgradeBaseFamily? env family then
          if head == ``VerexIR.Proofs.Denote.NTy.plainFallback && args.size == 3 then
            let τ := args[1]!
            let inhabitable := mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.inhabitable) τ
            let held ← if ← withDefault (isDefEq inhabitable (mkConst ``Bool.true)) then
                mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
                  (← mkEq inhabitable (mkConst ``Bool.true))
              else pure args[2]!
            return .done (mkApp3 t.getAppFn upgraded τ held)
          else
            return .done (mkAppN t.getAppFn (#[upgraded] ++ args.extract 1 args.size))
    if let some original := givenOperationProjection? t then return .done original
    if literalMapConstruction t then
      if let some literal ← (try NativeMaps.groundLiteral? t catch _ => pure none) then
        if literal != t then return .done literal
    if head == ``VerexIR.SpecMap.ofEntries then
      if let some canonical ← NativeMaps.canonicalMapOfEntries? t then
        return .done canonical
    let induced (family : Lean.Expr) : Option (Lean.Expr × Lean.Expr) := do
      let (θ, outer) ← inducedCarriers? family
      (substitutionRow? θ).map (outer, ·)
    unless instantiated do return .continue
    if head == ``VerexIR.Proofs.Denote.NTy.subst || head == ``VerexIR.Proofs.Denote.NRow.subst ||
        head == ``VerexIR.Proofs.Denote.NRows.subst then
      let #[θ, operand] := args | return .continue
      let row ← if θ.hasLooseBVars then pure θ else spelledRow θ
      unless spelledSpine row do return .continue
      let evaluated := if head == ``VerexIR.Proofs.Denote.NTy.subst then substituteNTy row operand
        else if head == ``VerexIR.Proofs.Denote.NRow.subst then substituteNRow row operand
        else substituteNRows row operand
      return match evaluated with
        | some evaluated => .done evaluated
        | none => .continue
    if head == ``VerexIR.Proofs.Denote.ResultShape.subst then
      let #[θ, shape] := args | return .continue
      let row ← if θ.hasLooseBVars then pure θ else spelledRow θ
      unless spelledSpine row do return .continue
      let some (constructor, shapeArgs) := (← resultShape? shape) | return .continue
      if constructor == ``VerexIR.Proofs.Denote.ResultShape.none then
        return .done (mkConst ``VerexIR.Proofs.Denote.ResultShape.none)
      let #[τ] := shapeArgs | return .continue
      return match substituteNTy row τ with
        | some τ' => .done (mkApp (mkConst ``VerexIR.Proofs.Denote.ResultShape.one) τ')
        | none => .continue
    if head == ``VerexIR.Proofs.Denote.ResultShape.carrier then
      let #[family, shape] := args | return .continue
      let some (constructor, shapeArgs) := (← resultShape? shape) | return .continue
      if constructor == ``VerexIR.Proofs.Denote.ResultShape.none then
        return .done (mkConst ``Unit)
      let #[τ] := shapeArgs | return .continue
      return .done (mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.carrier) family τ)
    if typedReads.contains head then
      let some family := args[0]? | return .continue
      let some τ := args[1]? | return .continue
      let rest := args.extract 2 args.size
      let τ₀ := τ.consumeMData
      if head == ``VerexIR.Proofs.Denote.NTy.readInhabited && (induced family).isSome &&
          !(τ₀.isAppOfArity ``VerexIR.Proofs.Denote.NTy.param 1 ||
            τ₀.isAppOfArity ``VerexIR.Proofs.Denote.NTy.int 2 ||
            τ₀.isConstOf ``VerexIR.Proofs.Denote.NTy.num) then
        match ← typedReadSpelling head family τ rest with
        | some t' => return .done t'
        | none => return .continue
      let (family, τ) ← match induced family with
        | some (outer, row) =>
            let some τ' := substituteNTy row τ | return .continue
            pure (outer, τ')
        | none => pure (family, τ)
      match ← typedReadSpelling head family τ rest with
      | some t' => return .done t'
      | none => return .continue
    -- The index past the arguments a branch changes: the arguments from
    -- there on are kept over a changed type or row, so that is the
    -- dependency boundary the rebuilding has to check.
    let rebuilt : Option (Lean.Expr × Nat) := do
      let family ← args[0]?
      let (outer, row) ← induced family
      if head == ``VerexIR.Proofs.Denote.NTy.carrier ||
          head == ``VerexIR.Proofs.Denote.NTy.deepHolds ||
          head == ``VerexIR.Proofs.Denote.NTy.Holds then
        let τ ← args[1]?
        let τ' ← substituteNTy row τ
        pure (mkAppN t.getAppFn (#[outer, τ'] ++ args.extract 2 args.size), 2)
      else if head == ``VerexIR.Proofs.Denote.HList ||
          head == ``VerexIR.Proofs.Denote.HList.Holds then
        let fields ← args[1]?
        let fields' ← substituteNRow row fields
        pure (mkAppN t.getAppFn (#[outer, fields'] ++ args.extract 2 args.size), 2)
      else if head == ``VerexIR.Proofs.Denote.variantCarrier ||
          head == ``VerexIR.Proofs.Denote.variantCarrier.Holds then
        let names ← args[1]?
        let rows ← args[2]?
        let rows' ← substituteNRows row rows
        pure (mkAppN t.getAppFn (#[outer, names, rows'] ++ args.extract 3 args.size), 3)
      else none
    match rebuilt with
    | some (t', boundary) =>
        if ← keptArgumentsConvert t'.getAppFn t'.getAppArgs boundary then return .done t'
        else return .continue
    | none =>
    -- Any other native operation at an induced family: at the outer family,
    -- each type and index it takes substituted.
    if t.hasLooseBVars then return .continue
    match ← respellOperation? t with
    | some t' => return .done t'
    | none => return .continue)

/-- Whether a term and its respelling are definitionally equal, by `isDefEq` on
the smallest applications they differ in, each distinct pair once per target. A
family converts only under the application it instantiates. On the whole term,
`isDefEq` fails on that argument at every enclosing application and unfolds it
instead, which on a term of shared reads costs the term's unshared size. -/
private partial def convertsAtSites (left right : Lean.Expr) : MetaM Bool := do
  if left == right then return true
  let conversions := (← targetState).familyConversions
  if let some known := (← conversions.get)[(left, right)]? then return known
  let result ← match left, right with
    | .mdata _ inner, _ => convertsAtSites inner right
    | _, .mdata _ inner => convertsAtSites left inner
    | .lam name type body info, .lam _ type' body' _
    | .forallE name type body info, .forallE _ type' body' _ =>
      if ← convertsAtSites type type' <&&> withLocalDecl name info type fun local_ =>
          convertsAtSites (body.instantiate1 local_) (body'.instantiate1 local_) then
        pure true
      else isDefEq left right
    | .app .., .app .. =>
      let (arguments, arguments') := (left.getAppArgs, right.getAppArgs)
      let sameHead := left.getAppFn == right.getAppFn && arguments.size == arguments'.size
      if ← pure sameHead <&&> arguments.size.allM fun index _ =>
          convertsAtSites arguments[index]! arguments'[index]! then
        pure true
      else if left.getAppFn.constName?.any isCarrierFamilyHead &&
          right.getAppFn.constName?.any isCarrierFamilyHead then
        pure false
      -- What a primitive projection erases is the structure's type
      -- parameters: the application converts even where one of them does
      -- not, so each parameter is converted at its own site, and the
      -- application itself then carries the owner through the projection. A
      -- constructor erases nothing and reduces nothing away, so every
      -- argument of one is converted at its own site.
      else if ← pure sameHead <&&> (do
          let some name := left.getAppFn.constName? | pure false
          pure (((← getEnv).find? name).any (· matches .ctorInfo _))) then
        arguments.size.allM fun index _ => isDefEq arguments[index]! arguments'[index]!
      else if sameHead then
        -- The head's projection info is read once: a projection erases its
        -- structure's parameters, so those are converted at their own sites
        -- and the projection itself carries the owner.
        match left.getAppFn.constName? with
        | none => isDefEq left right
        | some name =>
          match ← getProjectionFnInfo? name with
          | none => isDefEq left right
          | some info =>
            if ← (min info.numParams arguments.size).allM fun index _ =>
                isDefEq arguments[index]! arguments'[index]! then
              isDefEq left right
            else
              pure false
      else isDefEq left right
    | _, _ => isDefEq left right
  unless left.hasMVar || right.hasMVar do conversions.modify (·.insert (left, right) result)
  return result

/-- The canonical spelling of an expression's families, where it converts.
An expression the search left as it was is not searched again. Conversion uses
default transparency so a caller restricting simp transparency cannot suppress
a valid conversion of nominal types.

The input is typed and every branch rebuilds a site by a type-preserving
construction, so the respelling is well typed without a second global check:
a changed type, index or operation prefix converts at the application where
it stands (`respellOperation?` checks the prefix through its last changed
type or index; the carrier, `Holds`, `HList` and variant branches check the
arguments they keep over a changed one with `keptArgumentsConvert`), while
the fallback, read and result-shape branches rest on the construction they
perform, which changes no dependency boundary. The conversion of a whole
term is not that check either — a projection converts even where a type
parameter it erases does not — so `convertsAtSites` converts a projection's
parameters, and every argument of a constructor, at its own site. -/
def canonicalFamilyResult (e : Lean.Expr) : MetaM Simp.Result := withDefault do
  if (← (← targetState).canonicalForms.get).contains e then return { expr := e }
  -- A pass takes off one level of induced families: a callee's callee at
  -- generic types is two levels below the caller.
  let mut canonical ← canonicalFamilies e
  if canonical == e then
    unless e.hasMVar do (← targetState).canonicalForms.modify (·.insert e)
    return { expr := e }
  unless ← convertsAtSites e canonical do return { expr := e }
  repeat
    let next ← canonicalFamilies canonical
    if next == canonical then break
    unless ← convertsAtSites canonical next do break
    canonical := next
  return { expr := canonical }

/-- An order projection carries its parameter sort and codec as implicit
arguments, and its fields' plain sorts below the arguments. Give these
projections their common Given spelling without evaluating the comparison. -/
dsimproc ↓ [vir_denote, vir_denote_norm] givenOrderProjection
    (@TypedOrder.order _ _ _ _ _ _) := fun e => withDefault do
  if e.hasLooseBVars then return .continue
  unless (e.find? fun expression => (givenOperationProjection? expression).isSome).isSome do
    return .continue
  let canonical ← Core.transform e (post := fun expression => do
    if let some original := givenOperationProjection? expression then return .done original
    return .continue)
  unless ← isDefEq e canonical do return .continue
  return .visit canonical

@[inherit_doc assertCertifiedClauses]
elab_rules : tactic
  | `(tactic| verex_certified_clauses) =>
      assertCertifiedClauses (fun statement => return (← canonicalFamilyResult statement).expr)

@[inherit_doc assertObservedElementClauses]
elab_rules : tactic
  | `(tactic| verex_observed_elements) =>
      assertObservedElementClauses (fun statement => return (← canonicalFamilyResult statement).expr)

/-- Make the family spellings of the goals canonical, their targets too
where `target` holds (`canonicalFamilies`). -/
def canonicalFamiliesOf (target : Bool) (select : FVarId → Bool := fun _ => true) : TacticM Unit := do
  let goals ← getGoals
  let mut result := #[]
  for goal in goals do
    if ← goal.isAssigned then continue
    let mut goal := goal
    if target then
      let type ← goal.withContext (instantiateMVars (← goal.getType))
      let canonical ← goal.withContext (canonicalFamilyResult type)
      if canonical.expr != type then
        goal ← goal.withContext (applySimpResultToTarget goal type canonical)
    for decl in ← goal.withContext getLCtx do
      if decl.isImplementationDetail || !select decl.fvarId then continue
      let type ← goal.withContext (instantiateMVars decl.type)
      let canonical ← goal.withContext (canonicalFamilyResult type)
      if canonical.expr != type then
        let some (_, next) ← goal.withContext
            (applySimpResultToLocalDecl goal decl.fvarId canonical false) | continue
        goal := next
    result := result.push goal
  setGoals result.toList

elab_rules : tactic
  | `(tactic| verex_canonical_families) => canonicalFamiliesOf true

elab_rules : tactic
  | `(tactic| verex_canonical_hypotheses) => canonicalFamiliesOf false

elab_rules : tactic
  | `(tactic| verex_canonical_hypotheses [$hypotheses,*]) => focusGoal do
      let selected ← withMainContext do
        hypotheses.getElems.mapM fun hypothesis => do
          let .fvar id ← instantiateMVars (← Lean.Elab.Term.elabTerm hypothesis none)
            | throwError "expected a local hypothesis"
          return id
      canonicalFamiliesOf false selected.contains

/-- The outer frame, the type arguments, and their spelled row of a frame a
call's type arguments induce. -/
private def inducedFrame? (frame : Lean.Expr) : Option (Lean.Expr × Lean.Expr × Lean.Expr) := do
  let frame := frame.consumeMData
  guard (frame.isAppOfArity ``VerexIR.Proofs.Denote.Skolems.instantiate 4)
  let θ := frame.getArg! 2
  pure (θ, frame.getArg! 3, ← substitutionRow? θ)

/-- A literal failure, as compiled code throws one (a pattern mismatch), in
the form the clause naming its abort code reads (`Failure.ofInts`). -/
dsimproc [vir_denote] literalFailure (Prod.mk _ _) := fun e => do
  let_expr Prod.mk kindType _ kind payload := e | return .continue
  unless kindType.isConstOf ``VerexIR.ThrowKind do return .continue
  let some (elementType, _) := payload.arrayLit? | return .continue
  unless elementType.isConstOf ``Int do return .continue
  return .done (mkApp2 (mkConst ``VerexIR.Proofs.Failure.ofInts) kind payload)

/-- A value carried from the runtime family to an induced frame, as the
outer frame carries it at the type's substitution, evaluated
(`Skolems.ofRuntime_instantiate`): it then meets the outer frame's
transports, also where the value's type needs the substitution evaluated. -/
dsimproc ↓ [vir_denote, vir_denote_norm] ofRuntimeOuterFrame
    (@VerexIR.Proofs.Denote.Skolems.ofRuntime _ ?functions ?frame _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.Skolems.ofRuntime unit functions frame τ value := e
    | return .continue
  let some (θ, outer, row) := inducedFrame? frame | return .continue
  let some substituted := substituteNTy row τ (some (← getEnv)) | return .continue
  let carriers := mkApp3 (mkConst ``VerexIR.Proofs.Denote.Skolems.toCarriers) unit functions outer
  return .visit (mkApp4 (mkConst ``VerexIR.Proofs.Denote.NTy.toSkolem) carriers θ τ
    (mkApp5 e.getAppFn unit functions outer substituted value))

/-- A value carried from an induced frame to the runtime family, as the
outer frame carries it (`Skolems.toRuntime_instantiate`, `ofRuntimeOuterFrame`). -/
dsimproc ↓ [vir_denote, vir_denote_norm] toRuntimeOuterFrame
    (@VerexIR.Proofs.Denote.Skolems.toRuntime _ ?functions ?frame _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.Skolems.toRuntime unit functions frame τ value := e
    | return .continue
  let some (θ, outer, row) := inducedFrame? frame | return .continue
  let some substituted := substituteNTy row τ (some (← getEnv)) | return .continue
  let carriers := mkApp3 (mkConst ``VerexIR.Proofs.Denote.Skolems.toCarriers) unit functions outer
  let view := mkApp4 (mkConst ``VerexIR.Proofs.Denote.NTy.ofSkolem) carriers θ τ value
  return .visit (mkApp5 e.getAppFn unit functions outer substituted view)

/-- Whether a term rebuilds a value from its components: the value itself,
a certified integer from its value, or a pair of its first component and a
rebuild of its second, down to the unit a row ends with. -/
private partial def rebuilds (term value : Lean.Expr) : Bool :=
  term == value ||
    (term.isAppOfArity ``VerexIR.SpecInt.mk 4 &&
      (let first := term.getArg! 2
       first.isAppOfArity ``VerexIR.SpecInt.val 3 && first.appArg! == value)) ||
    (term.isAppOfArity ``Prod.mk 4 &&
      (let first := term.getArg! 2
       first.isAppOfArity ``Prod.fst 3 && first.getArg! 2 == value &&
         (let rest := term.getArg! 3
          let tail := mkApp3 (mkConst ``Prod.snd first.getAppFn.constLevels!) (first.getArg! 0)
            (first.getArg! 1) value
          rebuilds rest tail ||
            ((rest.isConstOf ``Unit.unit || rest.isAppOfArity ``PUnit.unit 0) &&
              let rowType := first.getArg! 1
              rowType.isConstOf ``Unit || rowType.isAppOfArity ``PUnit 0 ||
                (rowType.isAppOfArity ``VerexIR.Proofs.Denote.HList 2 &&
                  (rowType.getArg! 1).isConstOf ``VerexIR.Proofs.Denote.NRow.nil)))))

simproc [vir_denote, vir_denote_norm] mapIdentity (Array.map _ _) := fun e => do
  let_expr Array.map element _ function array := e | return .continue
  let .lam name domain body info := function | return .continue
  unless rebuilds body (.bvar 0) do return .continue
  unless ← withTransparency .default (isDefEq (← inferType e) (← inferType array)) do
    return .continue
  let levels := [e.getAppFn.constLevels!.head!]
  let identity := mkApp (mkConst ``Array.map_id'' levels) element
  let proof ← withLocalDecl name info domain fun x => do
    let reflexive ← mkEqRefl x
    let pointwise ← mkLambdaFVars #[x] reflexive
    return mkApp3 identity function pointwise array
  let proof ← mkExpectedTypeHint proof (← mkEq e array)
  return .done { expr := array, proof? := some proof }

private def valueTransport? (function : Lean.Expr) : Option (Lean.Expr × Lean.Expr × Lean.Expr) :=
  let function := function.consumeMData
  let body := match function with
    | .lam _ _ body _ =>
        if body.isAppOfArity ``VerexIR.Proofs.Denote.NTy.toSkolem 4 && body.appArg! == Lean.mkBVar 0 then
          some body.appFn!
        else none
    | _ => some function
  match body with
  | some transport =>
      if transport.isAppOfArity ``VerexIR.Proofs.Denote.NTy.toSkolem 3 then
        some (transport.getArg! 0, transport.getArg! 1, transport.getArg! 2)
      else none
  | none => none

/-- Values of the caller carried into a callee's view element by element are
the caller's values (`FamilyTransport.toSkolemIdentity`). -/
simproc [vir_denote, vir_denote_norm] mapTransportIdentity (Array.map _ _) := fun e => do
  let_expr Array.map element _ function array := e | return .continue
  let some (outer, θ, τ) := valueTransport? function | return .continue
  let some identity ← (try some <$> FamilyTransport.toSkolemIdentity outer θ τ element
      catch _ => pure none) | return .continue
  let pointwise ← withLocalDeclD `value element fun value => do
    mkLambdaFVars #[value] (← mkAppM ``congrFun #[identity, value])
  let proof := mkApp4 (mkConst ``Array.map_id'' [e.getAppFn.constLevels!.head!]) element function
    pointwise array
  let proof ← mkExpectedTypeHint proof (← mkEq e array)
  return .visit { expr := array, proof? := some proof }

/-- An optional caller's value carried into a callee's view is the caller's
value (`mapTransportIdentity`). -/
simproc [vir_denote, vir_denote_norm] optionTransportIdentity (Option.map _ _) := fun e => do
  let_expr Option.map element _ function option := e | return .continue
  let some (outer, θ, τ) := valueTransport? function | return .continue
  let some identity ← (try some <$> FamilyTransport.toSkolemIdentity outer θ τ element
      catch _ => pure none) | return .continue
  let mapped ← withLocalDeclD `function (← inferType function) fun g => do
    mkLambdaFVars #[g] (← mkAppM ``Option.map #[g, option])
  let congruence ← mkAppM ``congrArg #[mapped, identity]
  let proof ← mkEqTrans congruence (← mkAppOptM ``Option.map_id' #[element, option])
  let proof ← mkExpectedTypeHint proof (← mkEq e option)
  return .visit { expr := option, proof? := some proof }

private theorem differentResourceTypes {left right : ResourceType}
    (distinct : left.type ≠ right.type) : left ≠ right :=
  fun same => distinct (congrArg ResourceType.type same)

private theorem differentResourceArguments (left right : ResourceType)
    (leftArgs rightArgs : NRow)
    (leftObserved : left.arguments = leftArgs) (rightObserved : right.arguments = rightArgs)
    (distinct : leftArgs ≠ rightArgs) : left ≠ right := by
  intro same
  apply distinct
  rw [← leftObserved, ← rightObserved, same]

private theorem argumentWith_paramFree (types : Nat → NTy) (τ : NTy)
    (free : τ.paramFree = true) : τ.argumentWith types = τ := by
  cases τ with
  | param _ => simp [NTy.paramFree] at free
  | observed referent =>
      simp only [NTy.paramFree] at free
      exact congrArg NTy.observed (NTy.substWith_paramFree types referent free)
  | _ => exact NTy.substWith_paramFree types _ free

private theorem map_argumentWith_paramFree (types : Nat → NTy) :
    (row : NRow) → row.paramFree = true → row.map (NTy.argumentWith types) = row
  | .nil, _ => rfl
  | .cons τ rest, free => by
      simp only [NRow.paramFree, Bool.and_eq_true] at free
      simp only [NRow.map, argumentWith_paramFree types τ free.1,
        map_argumentWith_paramFree types rest free.2]

private theorem skolemResource_arguments_paramFree {unit : Validation.ValidatedUnit}
    [FunctionCarriers unit] [Θ : Skolems unit] (τ : NTy) (args : NRow)
    (free : args.paramFree = true) :
    (Skolems.resource τ args).arguments = args :=
  map_argumentWith_paramFree Θ.argument args free

@[inline] private def fastDefEq (a b : Lean.Expr) : MetaM Bool :=
  if a == b then pure true else withDefault (isDefEq a b)

private abbrev decEqStorageKey : DecidableEq VerexIR.StorageKey := inferInstance

private theorem option_structHandle_ne_of_indices {n m s t : Nat}
    (differ : (Nat.beq n m && Nat.beq s t) = false) :
    (some (VerexIR.StructHandle.mk ⟨n⟩ s) : Option VerexIR.StructHandle) ≠
      some (VerexIR.StructHandle.mk ⟨m⟩ t) := by
  intro same
  injection same with same
  cases same
  simp [Nat.beq_refl] at differ

private def optionStructHandleIndices? (h : Lean.Expr) :
    MetaM (Option (Lean.Expr × Lean.Expr × Nat × Nat)) := do
  let h := h.consumeMData
  unless h.isAppOfArity ``Option.some 2 do return none
  let sh := (h.getArg! 1).consumeMData
  unless sh.isAppOfArity ``VerexIR.StructHandle.mk 2 do return none
  let ns := (sh.getArg! 0).consumeMData
  unless ns.isAppOfArity ``VerexIR.NamespaceId.mk 1 do return none
  let nExpr := ns.appArg!
  let sExpr := sh.getArg! 1
  let some n ← getNatValue? nExpr | return none
  let some s ← getNatValue? sExpr | return none
  return some (nExpr, sExpr, n, s)

/-- Observe a declaration's `StructHandle` (nominal descriptor or structural `.struct`/`.enum`),
including under `Skolems.resolve`, without exposing field layouts. -/
private def anyResourceHandle? (type : Lean.Expr) : MetaM (Option (Lean.Expr × Lean.Expr)) := do
  let type := type.consumeMData
  let resolved := type.isAppOfArity ``Skolems.resolve 4
  let owner := (if resolved then type.getArg! 3 else type).consumeMData
  let env ← getEnv
  let view ← if isNominalType env owner then nativeTypeView owner else pure owner
  let isStruct := view.isAppOfArity ``NTy.struct 3
  let isEnum := view.isAppOfArity ``NTy.enum 5
  unless isStruct || isEnum do return none
  let theoremName := if resolved then
      if isStruct then ``NTy.handle?_resolve_struct else ``NTy.handle?_resolve_enum
    else if isStruct then ``NTy.handle?_struct else ``NTy.handle?_enum
  let arguments :=
    if resolved then #[type.getArg! 0, type.getArg! 1, type.getArg! 2] ++ view.getAppArgs
    else view.getAppArgs
  let proof := mkAppN (mkConst theoremName) arguments
  let handle := mkApp2 (mkConst ``Option.some [Level.zero]) (mkConst ``VerexIR.StructHandle) (view.getArg! 0)
  return some (handle, proof)

/-- Safely prove a closed `Decidable` proposition only when `decide p` reduces to `true`. -/
private def decideClosedProp? (prop : Lean.Expr) : MetaM (Option Lean.Expr) := do
  if prop.hasFVar || prop.hasMVar || prop.hasLooseBVars then return none
  try withDefault do
    let decInst ← synthInstance (mkApp (mkConst ``Decidable) prop)
    let b := mkApp2 (mkConst ``decide) prop decInst
    unless (← whnf b).isConstOf ``Bool.true do return none
    return some (mkApp3 (mkConst ``of_decide_eq_true) prop decInst
      (← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true)) (← mkEq b (mkConst ``Bool.true))))
  catch _ => return none

private partial def closedParamFreeRow (row : Lean.Expr) : Bool :=
  !row.hasFVar && !row.hasMVar && !row.hasLooseBVars &&
    (row.find? fun t =>
      t.isAppOf ``NTy.param || t.isAppOf ``Skolems.argument || t.isAppOf ``Skolems.resolve).isNone

/-- Decide `left ≠ right` for two `ResourceType`s without unfolding field layouts and declining
cleanly when resources differ only on unresolved type parameters. -/
def distinctResourceProof? (left right : Lean.Expr) : MetaM (Option Lean.Expr) := do
  if left == right then return none
  if left.isConstOf ``TableMemory.allocationResource && right.isAppOfArity ``Skolems.resource 5 then
    return some (mkAppN (mkConst ``Skolems.allocationResource_ne_resource) right.getAppArgs)
  if right.isConstOf ``TableMemory.allocationResource && left.isAppOfArity ``Skolems.resource 5 then
    return some (mkAppN (mkConst ``Skolems.resource_ne_allocationResource) left.getAppArgs)
  let directTypeOf? (res : Lean.Expr) : Option Lean.Expr :=
    if res.isAppOfArity ``Skolems.resource 5 then
      some (mkApp4 (mkConst ``Skolems.resolve) (res.getArg! 0) (res.getArg! 1) (res.getArg! 2)
        (res.getArg! 3))
    else if res.isAppOfArity ``ResourceType.mk 3 then some (res.getArg! 0)
    else none
  let leftType ← match directTypeOf? left with
    | some t => pure t
    | none => withReducible (whnf (mkApp (mkConst ``ResourceType.type) left))
  let rightType ← match directTypeOf? right with
    | some t => pure t
    | none => withReducible (whnf (mkApp (mkConst ``ResourceType.type) right))
  let lh? ← anyResourceHandle? leftType
  let rh? ← anyResourceHandle? rightType
  if let (some (lh, lp), some (rh, rp)) := (lh?, rh?) then
    if lh != rh then
      if let (some (n, s, nv, sv), some (m, t, mv, tv)) :=
          (← optionStructHandleIndices? lh, ← optionStructHandleIndices? rh) then
        if nv != mv || sv != tv then
          let distinct := mkApp5 (mkConst ``option_structHandle_ne_of_indices) n m s t
            (mkApp2 (mkConst ``Eq.refl [levelOne]) (mkConst ``Bool) (mkConst ``Bool.false))
          let tne := mkAppN (mkConst ``differentNominalDeclarations)
            #[leftType, rightType, lh, rh, lp, rp, distinct]
          return some (mkApp3 (mkConst ``differentResourceTypes) left right tne)
      if let some distinct ← decideClosedProp? (← mkAppM ``Ne #[lh, rh]) then
        let tne ← mkAppM ``differentNominalDeclarations
          #[leftType, rightType, lh, rh, lp, rp, distinct]
        return some (← mkAppOptM ``differentResourceTypes #[some left, some right, some tne])
  for decl in ← getLCtx do
    if decl.isAuxDecl then continue
    let ty := (← instantiateMVars decl.type).consumeMData
    let pair? : Option (Lean.Expr × Lean.Expr) :=
      if ty.isAppOfArity ``Ne 3 then some (ty.getArg! 1, ty.getArg! 2)
      else if ty.isAppOfArity ``Not 1 && ty.appArg!.isAppOfArity ``Eq 3 then
        some (ty.appArg!.getArg! 1, ty.appArg!.getArg! 2)
      else none
    if let some (l, r) := pair? then
      if ← (fastDefEq l left <&&> fastDefEq r right) then
        return some (← mkExpectedTypeHint decl.toExpr (← mkAppM ``Ne #[left, right]))
      if ← (fastDefEq l right <&&> fastDefEq r left) then
        return some (← mkAppM ``Ne.symm #[decl.toExpr])
  let directArgsOf? (res : Lean.Expr) : MetaM (Option (Lean.Expr × Lean.Expr)) := do
    if res.isAppOfArity ``Skolems.resource 5 then
      let args := res.getArg! 4
      if args.isAppOfArity ``NRow.nil 0 then
        let freeProof := mkApp2 (mkConst ``Eq.refl [levelOne]) (mkConst ``Bool) (mkConst ``Bool.true)
        return some (args, mkAppN (mkConst ``skolemResource_arguments_paramFree)
          (res.getAppArgs.push freeProof))
      if closedParamFreeRow args then
        if let some freeProof ← decideClosedProp?
            (← mkEq (mkApp (mkConst ``NRow.paramFree) args) (mkConst ``Bool.true)) then
          return some (args, mkAppN (mkConst ``skolemResource_arguments_paramFree)
            (res.getAppArgs.push freeProof))
      return none
    else if res.isAppOfArity ``ResourceType.mk 3 then
      let args := res.getArg! 1
      return some (args, ← mkEqRefl args)
    else return none
  let (leftArgs, leftEq) ← match ← directArgsOf? left with
    | some pair => pure pair
    | none => do
        let a ← withReducible (whnf (mkApp (mkConst ``ResourceType.arguments) left))
        pure (a, ← mkEqRefl a)
  let (rightArgs, rightEq) ← match ← directArgsOf? right with
    | some pair => pure pair
    | none => do
        let a ← withReducible (whnf (mkApp (mkConst ``ResourceType.arguments) right))
        pure (a, ← mkEqRefl a)
  if leftArgs != rightArgs && closedParamFreeRow leftArgs && closedParamFreeRow rightArgs then
    if let some ane ← decideClosedProp? (← mkAppM ``Ne #[leftArgs, rightArgs]) then
      return some (mkAppN (mkConst ``differentResourceArguments)
        #[left, right, leftArgs, rightArgs, leftEq, rightEq, ane])
  return none

/-- Decide `otherKey ≠ key` from `LCtx` or literal `StorageKey` inequality. -/
def distinctStorageKeyProof? (otherKey key : Lean.Expr) : MetaM (Option Lean.Expr) := do
  if otherKey == key then return none
  for decl in ← getLCtx do
    if decl.isAuxDecl then continue
    let ty := (← instantiateMVars decl.type).consumeMData
    let pair? : Option (Lean.Expr × Lean.Expr) :=
      if ty.isAppOfArity ``Ne 3 then some (ty.getArg! 1, ty.getArg! 2)
      else if ty.isAppOfArity ``Not 1 && ty.appArg!.isAppOfArity ``Eq 3 then
        some (ty.appArg!.getArg! 1, ty.appArg!.getArg! 2)
      else none
    if let some (l, r) := pair? then
      if l == otherKey && r == key then
        return some (← mkExpectedTypeHint decl.toExpr (← mkAppM ``Ne #[otherKey, key]))
      if l == key && r == otherKey then
        return some (← mkAppM ``Ne.symm #[decl.toExpr])
      if otherKey.isAppOfArity ``VerexIR.StorageKey.address 1 &&
          key.isAppOfArity ``VerexIR.StorageKey.address 1 then
        let (aOther, aKey) := (otherKey.appArg!, key.appArg!)
        if l == aOther && r == aKey then
          let p ← withLocalDeclD `eq (← mkEq otherKey key) fun eq => do
            let inj ← mkAppM ``VerexIR.StorageKey.address.inj #[eq]
            mkLambdaFVars #[eq] (mkApp decl.toExpr inj)
          return some (← mkExpectedTypeHint p (← mkAppM ``Ne #[otherKey, key]))
        if l == aKey && r == aOther then
          let p ← withLocalDeclD `eq (← mkEq otherKey key) fun eq => do
            let inj ← mkAppM ``VerexIR.StorageKey.address.inj #[eq]
            mkLambdaFVars #[eq] (mkApp decl.toExpr (← mkEqSymm inj))
          return some (← mkExpectedTypeHint p (← mkAppM ``Ne #[otherKey, key]))
  if !otherKey.hasFVar && !otherKey.hasMVar && !key.hasFVar && !key.hasMVar then
    if let some p ← decideClosedProp? (← mkAppM ``Ne #[otherKey, key]) then
      return some p
  return none

private theorem HList.holds_of_invariantFree [Carriers] (row : NRow)
    (free : row.invariantFree = true) (values : HList row) :
    HList.Holds row values :=
  (HList.holds_iff row values).mpr
    (HList.deepHolds_of_invariantFree row free values)

@[simp, vir_denote, vir_denote_norm]
theorem ofArrays_extent {α : Type} {extent : Nat} (current final : SpecArray α extent) :
    (SpecSliceRef.ofArrays current final).extent = extent := rfl

@[simp, vir_denote, vir_denote_norm]
theorem withCurrent_extent {α : Type} (value : SpecSliceRef α)
    (current : SpecArray α value.extent) :
    (value.withCurrent current).extent = value.extent := rfl

mutual
private partial def closeStructuralHoldsSubgoal (g : MVarId) : MetaM Bool := withDefault <| g.withContext do
  let t := (← instantiateMVars (← g.getType)).consumeMData
  if t.isAppOfArity ``NTy.Holds 3 then
    if let some pf ← holdsProof? (t.getArg! 0) (t.getArg! 1) (t.getArg! 2) then
      g.assign pf
      return true
    return false
  if t.isAppOfArity ``Carriers.own 4 then
    let (family, source, arguments, encoded) := (t.getArg! 0, t.getArg! 1, t.getArg! 2, t.getArg! 3)
    if let .app (.const ``GivenFrame.toCarriers _)
        (.app (.app (.const ``GivenFrame.withDeclarations _) given) declarations) := family.consumeMData then
      let selected := mkApp2 declarations source arguments
      let reduced ← (try withDefault (whnf selected) catch _ => pure selected)
      if reduced.isAppOfArity ``Option.none 1 then
        let absent ← mkExpectedTypeHint (← mkEqRefl reduced) (← mkEq selected reduced)
        let proof := mkAppN (mkConst ``GivenFrame.withDeclarations_own_none)
          #[given, declarations, source, arguments, absent, encoded]
        g.assign (← mkExpectedTypeHint proof t)
        return true
    let reduced ← (try withDefault (whnf t) catch _ => pure t)
    if reduced.isTrue then
      g.assign (← mkExpectedTypeHint (mkConst ``True.intro) t)
      return true
    return false
  if t.isAppOfArity ``HList.Holds 3 then
    let (c', row, vals) := (t.getArg! 0, t.getArg! 1, t.getArg! 2)
    if spelledNRow row (some (← getEnv)) then
      let rf := mkApp (mkConst ``NRow.invariantFree) row
      if (← (try withDefault (whnf rf) catch _ => pure rf)).isConstOf ``Bool.true then
        let fp ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true)) (← mkEq rf (mkConst ``Bool.true))
        g.assign (mkApp4 (mkConst ``HList.holds_of_invariantFree) c' row fp vals)
        return true
  if t.isAppOfArity ``variantCarrier.Holds 4 then
    let (c', names, rows, vals) := (t.getArg! 0, t.getArg! 1, t.getArg! 2, t.getArg! 3)
    if spelledNRows rows (some (← getEnv)) then
      let rf := mkApp (mkConst ``NRows.invariantFree) rows
      if (← (try withDefault (whnf rf) catch _ => pure rf)).isConstOf ``Bool.true then
        let fp ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true)) (← mkEq rf (mkConst ``Bool.true))
        g.assign (mkAppN (mkConst ``variantCarrier.holds_of_invariantFree) #[c', names, rows, fp, vals])
        return true
  let r ← (try whnfD t catch _ => pure t)
  if r.isTrue then
    g.assign (← mkExpectedTypeHint (mkConst ``True.intro) t)
    return true
  if r.isAppOfArity ``And 2 then
    let stated ← if r == t then pure g else g.change r
    let parts ← (try stated.apply (mkConst ``And.intro) catch _ => pure [g])
    if parts == [g] then return false
    return ← parts.allM closeStructuralHoldsSubgoal
  if r.isAppOfArity ``NTy.Holds 3 then
    if let some pf ← holdsProof? (r.getArg! 0) (r.getArg! 1) (r.getArg! 2) then
      let stated ← if r == t then pure g else g.change r
      stated.assign pf
      return true
  return false

private partial def holdsProof? (carriers τ v : Lean.Expr) : MetaM (Option Lean.Expr) := do
  try
    let carrierTy := mkApp2 (mkConst ``NTy.carrier) carriers τ
    let vTyped ← withDefault do
      if (← inferType v) == carrierTy then pure v else mkExpectedTypeHint v carrierTy
    let expected := mkApp3 (mkConst ``NTy.Holds) carriers τ vTyped
    if ← isInvariantFreeNTy τ then
      let cf := mkApp (mkConst ``NTy.invariantFree) τ
      let freeProof ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
        (← mkEq cf (mkConst ``Bool.true))
      return some (mkApp4 (mkConst ``NTy.holds_of_invariantFree) carriers τ freeProof vTyped)
    if let some h ← withDefault (findHoldsHypothesis? expected) then
      return some (← withDefault (mkExpectedTypeHint h expected))
    for decl in ← getLCtx do
      if decl.isAuxDecl then continue
      let ty := (← instantiateMVars decl.type).consumeMData
      if let some (_, lhs, rhs) := ty.eq? then
        let lhs := lhs.consumeMData
        let rhs := rhs.consumeMData
        if lhs.isAppOfArity ``Skolems.plainAt 7 && rhs.isAppOfArity ``Option.some 2 then
          if ← withDefault (isDefEq (rhs.getArg! 1) v <&&> isDefEq (lhs.getArg! 3) τ) then
            let args := lhs.getAppArgs
            let readCarriers := mkApp3 (mkConst ``Skolems.toCarriers) args[0]! args[1]! args[2]!
            if ← withDefault (isDefEq readCarriers carriers) then
              if let some h ← (try withDefault do
                  let readCarrierTy := mkApp2 (mkConst ``NTy.carrier) readCarriers args[3]!
                  let vRead ← mkExpectedTypeHint (rhs.getArg! 1) readCarrierTy
                  let eqExpected ← mkEq lhs (mkApp2 (mkConst ``Option.some [Level.zero]) readCarrierTy vRead)
                  let eqTyped ← mkExpectedTypeHint decl.toExpr eqExpected
                  let h := mkAppN (mkConst ``Skolems.holds_plainAt) (args ++ #[vRead, eqTyped])
                  if ← isDefEq (← inferType h) expected then
                    pure (some (← mkExpectedTypeHint h expected))
                  else pure none
                catch _ => pure none) then
                return some h
        else if lhs.isAppOfArity ``InterpreterOps.read 8 && rhs.isAppOfArity ``Option.some 2 then
          if ← withDefault (isDefEq (rhs.getArg! 1) v <&&> isDefEq (lhs.getArg! 4) τ <&&>
              isDefEq (lhs.getArg! 1) carriers) then
            let args := lhs.getAppArgs
            if let some h ← (try withDefault do
                let readCarrierTy := mkApp2 (mkConst ``NTy.carrier) args[1]! args[4]!
                let vRead ← mkExpectedTypeHint (rhs.getArg! 1) readCarrierTy
                let eqExpected ← mkEq lhs (mkApp2 (mkConst ``Option.some [Level.zero]) readCarrierTy vRead)
                let eqTyped ← mkExpectedTypeHint decl.toExpr eqExpected
                let h := mkAppN (mkConst ``InterpreterOps.read_holds) (args ++ #[vRead, eqTyped])
                if ← isDefEq (← inferType h) expected then
                  pure (some (← mkExpectedTypeHint h expected))
                else pure none
              catch _ => pure none) then
              return some h
    if !τ.hasMVar then
      let view ← (try withDefault (nativeTypeView τ) catch _ => pure τ)
      let view ← if view.getAppFn.isConst &&
          !(view.isAppOfArity ``NTy.struct 3 || view.isAppOfArity ``NTy.enum 5) then
          (try withDefault (whnfD view) catch _ => pure view)
        else pure view
      let equiv? : Option Name :=
        if view.isAppOfArity ``NTy.struct 3 then some ``NTy.holds_struct
        else if view.isAppOfArity ``NTy.enum 5 then some ``NTy.holds_enum
        else none
      if let some equivName := equiv? then
        let declared := isNominalType (← getEnv) τ
        let saved ← saveState
        let closed? ← try withDefault do
          let goalMVar ← mkFreshExprMVar expected
          let viewCarrierTy := mkApp2 (mkConst ``NTy.carrier) carriers view
          let vForView ←
            if (← inferType vTyped) == viewCarrierTy then pure vTyped
            else mkExpectedTypeHint vTyped viewCarrierTy
          let equiv := mkAppN (mkConst equivName) (#[carriers] ++ view.getAppArgs ++ #[vForView])
          let subs ← goalMVar.mvarId!.apply (← mkAppM ``Iff.mpr #[equiv])
          let subs ← subs.mapM fun sub => do
            if !declared || view == τ then pure sub
            else
              try
                sub.change ((← instantiateMVars (← sub.getType)).replace
                  fun part => if part == view then some τ else none)
              catch _ => pure sub
          if ← subs.allM closeStructuralHoldsSubgoal then
            let pf ← instantiateMVars goalMVar
            if !pf.hasMVar then pure (some pf) else do saved.restore; pure none
          else do saved.restore; pure none
        catch _ => saved.restore; pure none
        if let some pf := closed? then return some pf
    return none
  catch _ => return none
end

/-- Plain storage read at an induced frame, evaluated at the outer frame (`Skolems.plainAt_instantiate`). -/
simproc ↓ [vir_denote, vir_denote_norm] plainAtOuterFrame
    (@VerexIR.Proofs.Denote.Skolems.plainAt _ ?functions ?frame _ _ _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.Skolems.plainAt unit functions frame τ arguments key memory := e
    | return .continue
  let some (θ, outer, row) := inducedFrame? frame | return .continue
  let env ← getEnv
  let some substituted := substituteNTy row τ (some env) | return .continue
  let some substitutedArgs := substituteNRow row arguments (some env) | return .continue
  let outerCarriers := mkApp3 (mkConst ``VerexIR.Proofs.Denote.Skolems.toCarriers) unit functions outer
  let innerRead := mkAppN (mkConst ``VerexIR.Proofs.Denote.Skolems.plainAt)
    #[unit, functions, outer, substituted, substitutedArgs, key, memory]
  let transport := mkApp3 (mkConst ``VerexIR.Proofs.Denote.NTy.toSkolem)
    outerCarriers θ τ
  let rhs ← mkAppM ``Option.map #[transport, innerRead]
  let rawProof := mkAppN (mkConst ``VerexIR.Proofs.Denote.Skolems.plainAt_instantiate)
    #[unit, functions, θ, outer, τ, arguments, key, memory]
  let targetEq ← mkEq e rhs
  unless ← withDefault (isDefEq (← inferType rawProof) targetEq) do return .continue
  return .visit { expr := rhs, proof? := some (← mkExpectedTypeHint rawProof targetEq) }

/-- Plain storage write at an induced frame, evaluated at the outer frame (`Skolems.writePlain_instantiate`). -/
simproc ↓ [vir_denote, vir_denote_norm] writePlainOuterFrame
    (@VerexIR.Proofs.Denote.Skolems.writePlain _ ?functions ?frame _ _ _ _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.Skolems.writePlain unit functions frame τ arguments key value memory := e
    | return .continue
  let some (θ, outer, row) := inducedFrame? frame | return .continue
  let env ← getEnv
  let some substituted := substituteNTy row τ (some env) | return .continue
  let some substitutedArgs := substituteNRow row arguments (some env) | return .continue
  let outerCarriers := mkApp3 (mkConst ``VerexIR.Proofs.Denote.Skolems.toCarriers) unit functions outer
  let transport := mkApp3 (mkConst ``VerexIR.Proofs.Denote.NTy.ofSkolem)
    outerCarriers θ τ
  let mappedValue ← mkAppM ``Option.map #[transport, value]
  let rhs := mkAppN (mkConst ``VerexIR.Proofs.Denote.Skolems.writePlain)
    #[unit, functions, outer, substituted, substitutedArgs, key, mappedValue, memory]
  let rawProof := mkAppN (mkConst ``VerexIR.Proofs.Denote.Skolems.writePlain_instantiate)
    #[unit, functions, θ, outer, τ, arguments, key, value, memory]
  let targetEq ← mkEq e rhs
  unless ← withDefault (isDefEq (← inferType rawProof) targetEq) do return .continue
  return .visit { expr := rhs, proof? := some (← mkExpectedTypeHint rawProof targetEq) }

/-- Collapse `Option.map (NTy.toSkolem θ τ) opt` / `Option.map (NTy.ofSkolem θ τ) opt`
when source and target carrier types convert. -/
simproc [vir_denote, vir_denote_norm] optionPlainTransportIdentity (Option.map _ _) := fun e => do
  let_expr Option.map element resultType function option := e | return .continue
  let fnBare := function.consumeMData
  let (heqName, args?) : Name × Option (Array Lean.Expr) :=
    if fnBare.isAppOfArity ``NTy.toSkolem 3 then
      (``NTy.toSkolem_heq, some fnBare.getAppArgs)
    else if fnBare.isAppOfArity ``NTy.ofSkolem 3 then
      (``NTy.ofSkolem_heq, some fnBare.getAppArgs)
    else (.anonymous, none)
  let some args := args? | return .continue
  unless ← withDefault (isDefEq element resultType) do return .continue
  let some proof ← (try withDefault do
      let identity ← withLocalDeclD `x element fun x => do
        let fx := mkApp function x
        let heq ← mkAppOptM heqName (args.map some ++ #[some x])
        let eq ← mkAppOptM ``eq_of_heq #[some element, some fx, some x, some heq]
        mkLambdaFVars #[x] (← mkExpectedTypeHint eq (← mkEq fx x))
      let funEq ← mkAppM ``funext #[identity]
      let mapped ← withLocalDeclD `g (← inferType function) fun g => do
        mkLambdaFVars #[g]
          (← mkAppOptM ``Option.map #[some element, some element, some g, some option])
      let congr ← mkAppM ``congrArg #[mapped, funEq]
      let idEq ← mkAppOptM ``Option.map_id' #[some element, some option]
      let total ← mkEqTrans congr idEq
      pure (some (← mkExpectedTypeHint total (← mkEq e option)))
    catch _ => pure none) | return .continue
  return .visit { expr := option, proof? := some proof }

/-- Read-over-write simproc for `Skolems.plainAt` over `Skolems.writePlain` (all 4 fast paths). -/
simproc [vir_denote, vir_denote_norm] reducePlainAtWritePlain
    (@VerexIR.Proofs.Denote.Skolems.plainAt _ ?functions ?frame _ _ _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.Skolems.plainAt unit functions frame σ others otherKey mem := e
    | return .continue
  let_expr VerexIR.Proofs.Denote.Skolems.writePlain wUnit wFunctions wFrame τ arguments key value innerMem := mem
    | return .continue
  unless unit == wUnit && functions == wFunctions do return .continue
  let readRes := mkApp5 (mkConst ``Skolems.resource) unit functions frame σ others
  let writeRes := mkApp5 (mkConst ``Skolems.resource) unit functions wFrame τ arguments
  if frame == wFrame then
    if let some distinct ← distinctResourceProof? readRes writeRes then
      let proof := mkAppN (mkConst ``Skolems.plainAt_writePlain_other)
        #[unit, functions, frame, τ, σ, arguments, others, key, otherKey, innerMem, value, distinct]
      let rhs := mkAppN (mkConst ``Skolems.plainAt)
        #[unit, functions, frame, σ, others, otherKey, innerMem]
      let rCarriers := mkApp3 (mkConst ``Skolems.toCarriers) unit functions frame
      let rOptCarrierTy := mkApp (mkConst ``Option [Level.zero]) (mkApp2 (mkConst ``NTy.carrier) rCarriers σ)
      let targetEq := mkApp3 (mkConst ``Eq [Level.one]) rOptCarrierTy e rhs
      return .visit { expr := rhs, proof? := some (mkApp2 (mkConst ``id [Level.zero]) targetEq proof) }
    if let some ne ← distinctStorageKeyProof? otherKey key then
      let proof := mkAppN (mkConst ``Skolems.plainAt_writePlain_ne_key)
        #[unit, functions, frame, τ, σ, arguments, others, key, otherKey, innerMem, value, ne]
      let rhs := mkAppN (mkConst ``Skolems.plainAt)
        #[unit, functions, frame, σ, others, otherKey, innerMem]
      let rCarriers := mkApp3 (mkConst ``Skolems.toCarriers) unit functions frame
      let rOptCarrierTy := mkApp (mkConst ``Option [Level.zero]) (mkApp2 (mkConst ``NTy.carrier) rCarriers σ)
      let targetEq := mkApp3 (mkConst ``Eq [Level.one]) rOptCarrierTy e rhs
      return .visit { expr := rhs, proof? := some (mkApp2 (mkConst ``id [Level.zero]) targetEq proof) }
  let exactSameRes := frame == wFrame && σ == τ && others == arguments
  unless ← (if exactSameRes then pure true
      else fastDefEq frame wFrame <&&> fastDefEq σ τ <&&> fastDefEq others arguments) do
    return .continue
  let carriers := mkApp3 (mkConst ``Skolems.toCarriers) unit functions wFrame
  let carrierTy := mkApp2 (mkConst ``NTy.carrier) carriers τ
  let optCarrierTy := mkApp (mkConst ``Option [Level.zero]) carrierTy
  if value.consumeMData.isAppOfArity ``Option.none 1 then
    if otherKey == key then
      let proof := mkAppN (mkConst ``Skolems.plainAt_writePlain_none_same)
        #[unit, functions, wFrame, τ, arguments, key, innerMem]
      let targetEq := mkApp3 (mkConst ``Eq [Level.one]) optCarrierTy e value
      if exactSameRes then
        return .visit { expr := value, proof? := some (mkApp2 (mkConst ``id [Level.zero]) targetEq proof) }
      return .visit { expr := value, proof? := some (← mkExpectedTypeHint proof (← mkEq e value)) }
    if let some ne ← distinctStorageKeyProof? otherKey key then
      let proof := mkAppN (mkConst ``Skolems.plainAt_writePlain_ne)
        #[unit, functions, wFrame, τ, arguments, key, otherKey, value, innerMem, ne]
      let rhs := mkAppN (mkConst ``Skolems.plainAt)
        #[unit, functions, frame, σ, others, otherKey, innerMem]
      let targetEq := mkApp3 (mkConst ``Eq [Level.one]) optCarrierTy e rhs
      if exactSameRes then
        return .visit { expr := rhs, proof? := some (mkApp2 (mkConst ``id [Level.zero]) targetEq proof) }
      return .visit { expr := rhs, proof? := some (← mkExpectedTypeHint proof (← mkEq e rhs)) }
    let proof := mkAppN (mkConst ``Skolems.plainAt_writePlain_none)
      #[unit, functions, wFrame, τ, arguments, key, otherKey, innerMem]
    let cond := mkApp3 (mkConst ``Eq [Level.one]) (mkConst ``VerexIR.StorageKey) otherKey key
    let dec := mkApp2 (mkConst ``decEqStorageKey) otherKey key
    let fallback := mkAppN (mkConst ``Skolems.plainAt)
      #[unit, functions, wFrame, τ, arguments, otherKey, innerMem]
    let rhs := mkAppN (mkConst ``ite [Level.one]) #[optCarrierTy, cond, dec, value, fallback]
    let targetEq := mkApp3 (mkConst ``Eq [Level.one]) optCarrierTy e rhs
    if exactSameRes then
      return .visit { expr := rhs, proof? := some (mkApp2 (mkConst ``id [Level.zero]) targetEq proof) }
    return .visit { expr := rhs, proof? := some (← mkExpectedTypeHint proof (← mkEq e rhs)) }
  if let some ne ← distinctStorageKeyProof? otherKey key then
    let proof := mkAppN (mkConst ``Skolems.plainAt_writePlain_ne)
      #[unit, functions, wFrame, τ, arguments, key, otherKey, value, innerMem, ne]
    let rhs := mkAppN (mkConst ``Skolems.plainAt)
      #[unit, functions, frame, σ, others, otherKey, innerMem]
    let targetEq := mkApp3 (mkConst ``Eq [Level.one]) optCarrierTy e rhs
    if exactSameRes then
      return .visit { expr := rhs, proof? := some (mkApp2 (mkConst ``id [Level.zero]) targetEq proof) }
    return .visit { expr := rhs, proof? := some (← mkExpectedTypeHint proof (← mkEq e rhs)) }
  let vBare := value.consumeMData
  unless vBare.isAppOfArity ``Option.some 2 do return .continue
  let v := vBare.getArg! 1
  let some holds ← holdsProof? carriers τ v | return .continue
  let holdsExpected := mkAppN (mkConst ``Option.elim [Level.zero, Level.one])
    #[carrierTy, mkSort Level.zero, value, mkConst ``True, mkApp2 (mkConst ``NTy.Holds) carriers τ]
  let holdsTyped := mkApp2 (mkConst ``id [Level.zero]) holdsExpected holds
  if otherKey == key then
    let proof := mkAppN (mkConst ``Skolems.plainAt_writePlain_same)
      #[unit, functions, wFrame, τ, arguments, key, innerMem, value, holdsTyped]
    let targetEq := mkApp3 (mkConst ``Eq [Level.one]) optCarrierTy e value
    if exactSameRes then
      return .visit { expr := value, proof? := some (mkApp2 (mkConst ``id [Level.zero]) targetEq proof) }
    let targetEq ← mkEq e value
    unless ← withDefault (isDefEq (← inferType proof) targetEq) do return .continue
    return .visit { expr := value, proof? := some (← mkExpectedTypeHint proof targetEq) }
  let proof := mkAppN (mkConst ``Skolems.plainAt_writePlain)
    #[unit, functions, wFrame, τ, arguments, key, otherKey, innerMem, value, holdsTyped]
  let cond := mkApp3 (mkConst ``Eq [Level.one]) (mkConst ``VerexIR.StorageKey) otherKey key
  let dec := mkApp2 (mkConst ``decEqStorageKey) otherKey key
  let fallback := mkAppN (mkConst ``Skolems.plainAt)
    #[unit, functions, wFrame, τ, arguments, otherKey, innerMem]
  let rhs := mkAppN (mkConst ``ite [Level.one]) #[optCarrierTy, cond, dec, value, fallback]
  let targetEq := mkApp3 (mkConst ``Eq [Level.one]) optCarrierTy e rhs
  if exactSameRes then
    return .visit { expr := rhs, proof? := some (mkApp2 (mkConst ``id [Level.zero]) targetEq proof) }
  let targetEq ← mkEq e rhs
  unless ← withDefault (isDefEq (← inferType proof) targetEq) do return .continue
  return .visit { expr := rhs, proof? := some (← mkExpectedTypeHint proof targetEq) }

/-- Collapse consecutive writes to the same resource and key (`Skolems.writePlain_writePlain_same`). -/
simproc [vir_denote, vir_denote_norm] reduceWritePlainWritePlain
    (@VerexIR.Proofs.Denote.Skolems.writePlain _ ?functions ?frame _ _ _ _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.Skolems.writePlain unit functions frame τ arguments key v2 mem := e
    | return .continue
  let_expr VerexIR.Proofs.Denote.Skolems.writePlain iUnit iFunctions iFrame iτ iArgs iKey v1 innerMem := mem
    | return .continue
  unless unit == iUnit && functions == iFunctions && key == iKey do return .continue
  unless ← withDefault (isDefEq frame iFrame <&&> isDefEq τ iτ <&&> isDefEq arguments iArgs) do
    return .continue
  let rhs := mkAppN (mkConst ``Skolems.writePlain)
    #[unit, functions, frame, τ, arguments, key, v2, innerMem]
  if v2.consumeMData.isAppOfArity ``Option.none 1 then
    let proof := mkAppN (mkConst ``Skolems.writePlain_writePlain_none_same)
      #[unit, functions, frame, τ, arguments, key, v1, innerMem]
    let targetEq ← mkEq e rhs
    unless ← withDefault (isDefEq (← inferType proof) targetEq) do return .continue
    return .visit { expr := rhs, proof? := some (← mkExpectedTypeHint proof targetEq) }
  let v2Bare := v2.consumeMData
  unless v2Bare.isAppOfArity ``Option.some 2 do return .continue
  let carriers := mkApp3 (mkConst ``Skolems.toCarriers) unit functions frame
  let some holds2 ← holdsProof? carriers τ (v2Bare.getArg! 1) | return .continue
  let carrierTy := mkApp2 (mkConst ``NTy.carrier) carriers τ
  let optCarrierTy := mkApp (mkConst ``Option [Level.zero]) carrierTy
  let v1Typed ← withDefault do
    if (← inferType v1) == optCarrierTy then pure v1 else mkExpectedTypeHint v1 optCarrierTy
  let v2Typed ← withDefault do
    if (← inferType v2) == optCarrierTy then pure v2 else mkExpectedTypeHint v2 optCarrierTy
  let holdsExpected := mkAppN (mkConst ``Option.elim [Level.zero, Level.one])
    #[carrierTy, mkSort Level.zero, v2Typed, mkConst ``True, mkApp2 (mkConst ``NTy.Holds) carriers τ]
  let holds2Typed ← mkExpectedTypeHint holds2 holdsExpected
  let proof := mkAppN (mkConst ``Skolems.writePlain_writePlain_same)
    #[unit, functions, frame, τ, arguments, key, v1Typed, v2Typed, innerMem, holds2Typed]
  let targetEq ← mkEq e rhs
  unless ← withDefault (isDefEq (← inferType proof) targetEq) do return .continue
  return .visit { expr := rhs, proof? := some (← mkExpectedTypeHint proof targetEq) }

private theorem Skolems.writePlain_apply_of_eq {unit : Validation.ValidatedUnit}
    [FunctionCarriers unit] [Θ : Skolems unit] (τ : NTy) (arguments : NRow) {r : ResourceType}
    (key otherKey : StorageKey) (value : Option τ.carrier) (memory : Memory unit)
    (same : r = Skolems.resource τ arguments) :
    Skolems.writePlain τ arguments key value memory r otherKey =
      if otherKey = key then
        Skolems.writePlain τ arguments key value memory r key
      else memory r otherKey := by
  by_cases h : otherKey = key
  · subst h; rw [if_pos rfl]
  · rw [if_neg h, Skolems.writePlain_other_key_of_eq τ arguments key otherKey value memory same h]

/-- Reduce raw-memory applications `Skolems.writePlain τ args k v m r k'` (e.g. `slotFrame` after
`Skolems.resource` normalization) when `r ≠ Skolems.resource τ args`, `k' ≠ k`, or `k'` is undecided. -/
simproc [vir_denote, vir_denote_norm] reduceWritePlainApply
    ((@VerexIR.Proofs.Denote.Skolems.writePlain _ ?functions ?frame _ _ _ _ _) _ _) := fun e => do
  unless e.getAppNumArgs == 10 do return .continue
  unless e.getAppFn.isConstOf ``Skolems.writePlain do return .continue
  let args := e.getAppArgs
  let (unit, functions, frame, τ, arguments, key, value, memory, otherRes, otherKey) :=
    (args[0]!, args[1]!, args[2]!, args[3]!, args[4]!, args[5]!, args[6]!, args[7]!, args[8]!, args[9]!)
  let writeRes := mkApp5 (mkConst ``Skolems.resource) unit functions frame τ arguments
  if let some distinct ← distinctResourceProof? otherRes writeRes then
    let proof := mkAppN (mkConst ``Skolems.writePlain_other_resource_apply)
      #[unit, functions, frame, τ, arguments, otherRes, key, otherKey, value, memory, distinct]
    let rhs := mkApp2 memory otherRes otherKey
    return .visit { expr := rhs, proof? := some (← mkExpectedTypeHint proof (← mkEq e rhs)) }
  if let some ne ← distinctStorageKeyProof? otherKey key then
    let proof := mkAppN (mkConst ``Skolems.writePlain_ne_key)
      #[unit, functions, frame, τ, arguments, otherRes, key, otherKey, value, memory, ne]
    let rhs := mkApp2 memory otherRes otherKey
    return .visit { expr := rhs, proof? := some (← mkExpectedTypeHint proof (← mkEq e rhs)) }
  if ← withDefault (isDefEq otherRes writeRes) then
    let same ← mkExpectedTypeHint (← mkEqRefl otherRes) (← mkEq otherRes writeRes)
    if otherKey != key then
      let cond ← mkEq otherKey key
      let dec ← synthInstance (mkApp (mkConst ``Decidable) cond)
      let atKey := mkAppN (mkConst ``Skolems.writePlain)
        #[unit, functions, frame, τ, arguments, key, value, memory, otherRes, key]
      let fallback := mkApp2 memory otherRes otherKey
      let rhs := mkAppN (mkConst ``ite [Level.one]) #[← inferType fallback, cond, dec, atKey, fallback]
      let proof := mkAppN (mkConst ``Skolems.writePlain_apply_of_eq)
        #[unit, functions, frame, τ, arguments, otherRes, key, otherKey, value, memory, same]
      let targetEq ← mkEq e rhs
      if ← withDefault (isDefEq (← inferType proof) targetEq) then
        return .visit { expr := rhs, proof? := some (← mkExpectedTypeHint proof targetEq) }
  return .continue

theorem Skolems.isSome_writePlain_none_same_of_eq {unit : Validation.ValidatedUnit}
    [FunctionCarriers unit] {Θ : Skolems unit} (τ : NTy) (arguments : NRow)
    (key : StorageKey) (memory : Memory unit) {r : ResourceType} {k : StorageKey}
    (sameRes : r = Skolems.resource (Θ := Θ) τ arguments) (sameKey : k = key) :
    ((Skolems.writePlain (Θ := Θ) τ arguments key none memory) r k).isSome = false := by
  subst sameRes; cases sameKey
  rw [Skolems.writePlain_of_holds τ arguments key none memory trivial, Memory.set_same, if_pos rfl]
  rfl

theorem Skolems.isSome_writePlain_some_same_of_eq {unit : Validation.ValidatedUnit}
    [FunctionCarriers unit] {Θ : Skolems unit} (τ : NTy) (arguments : NRow)
    (key : StorageKey) (value : @NTy.carrier Θ.toCarriers τ) (memory : Memory unit)
    (holds : @NTy.Holds Θ.toCarriers τ value) {r : ResourceType} {k : StorageKey}
    (sameRes : r = Skolems.resource (Θ := Θ) τ arguments) (sameKey : k = key) :
    ((Skolems.writePlain (Θ := Θ) τ arguments key (some value) memory) r k).isSome = true := by
  subst sameRes; cases sameKey
  rw [Skolems.writePlain_of_holds τ arguments key (some value) memory holds, Memory.set_same, if_pos rfl]
  rfl

/-- Reduce `Option.isSome (Skolems.writePlain τ args k v m r k')` when `r` and `k'` convert to
the written resource and key (`Skolems.isSome_writePlain_none_same_of_eq`,
`Skolems.isSome_writePlain_some_same_of_eq`). -/
simproc [vir_denote, vir_denote_norm] reduceIsSomeWritePlainApply
    (Option.isSome ((@VerexIR.Proofs.Denote.Skolems.writePlain _ ?functions ?frame _ _ _ _ _) _ _)) := fun e => do
  let_expr Option.isSome _ slot := e | return .continue
  let slot := slot.consumeMData
  unless slot.getAppNumArgs == 10 && slot.getAppFn.isConstOf ``Skolems.writePlain do
    return .continue
  let args := slot.getAppArgs
  let (unit, functions, frame, τ, arguments, key, value, memory, otherRes, otherKey) :=
    (args[0]!, args[1]!, args[2]!, args[3]!, args[4]!, args[5]!, args[6]!, args[7]!, args[8]!, args[9]!)
  let writeRes := mkApp5 (mkConst ``Skolems.resource) unit functions frame τ arguments
  unless ← withDefault (isDefEq otherRes writeRes <&&> isDefEq otherKey key) do
    return .continue
  let sameRes ← mkExpectedTypeHint (← mkEqRefl otherRes) (← mkEq otherRes writeRes)
  let sameKey ← mkExpectedTypeHint (← mkEqRefl otherKey) (← mkEq otherKey key)
  let vBare := value.consumeMData
  if vBare.isAppOfArity ``Option.none 1 then
    let proof := mkAppN (mkConst ``Skolems.isSome_writePlain_none_same_of_eq)
      #[unit, functions, frame, τ, arguments, key, memory, otherRes, otherKey, sameRes, sameKey]
    let rhs := mkConst ``Bool.false
    let targetEq ← mkEq e rhs
    unless ← withDefault (isDefEq (← inferType proof) targetEq) do return .continue
    return .done { expr := rhs, proof? := some (← mkExpectedTypeHint proof targetEq) }
  if vBare.isAppOfArity ``Option.some 2 then
    let v := vBare.getArg! 1
    let carriers := mkApp3 (mkConst ``Skolems.toCarriers) unit functions frame
    let some holds ← holdsProof? carriers τ v | return .continue
    let carrierTy := mkApp2 (mkConst ``NTy.carrier) carriers τ
    let vTyped ← withDefault do
      if (← inferType v) == carrierTy then pure v else mkExpectedTypeHint v carrierTy
    let holdsExpected := mkApp3 (mkConst ``NTy.Holds) carriers τ vTyped
    let holdsTyped ← withDefault do
      if (← inferType holds) == holdsExpected then pure holds else mkExpectedTypeHint holds holdsExpected
    let proof := mkAppN (mkConst ``Skolems.isSome_writePlain_some_same_of_eq)
      #[unit, functions, frame, τ, arguments, key, vTyped, memory, holdsTyped, otherRes, otherKey, sameRes, sameKey]
    let rhs := mkConst ``Bool.true
    let targetEq ← mkEq e rhs
    unless ← withDefault (isDefEq (← inferType proof) targetEq) do return .continue
    return .done { expr := rhs, proof? := some (← mkExpectedTypeHint proof targetEq) }
  return .continue

@[vir_denote, vir_denote_norm]
theorem option_some_bind {α β : Type} (a : α) (f : α → Option β) :
    (some a).bind f = f a := rfl

@[vir_denote, vir_denote_norm]
theorem option_none_bind {α β : Type} (f : α → Option β) :
    (none : Option α).bind f = none := rfl

@[simp, vir_denote, vir_denote_norm]
theorem ite_bind {α β : Type} {condition : Prop} [Decidable condition]
    (left right : Option α) (f : α → Option β) :
    (if condition then left else right).bind f =
      if condition then left.bind f else right.bind f := by
  split <;> rfl

@[vir_denote, vir_denote_norm]
theorem prod_snd_punit {α : Type} (p : α × PUnit) : p.snd = () := rfl

/-- A callee's value of a type without type parameters, read in the
caller's view, is the value itself: both views hold the same values
(`FamilyTransport.pointwiseIdentity`). The transport's head, its family and
type arguments, and the types it reads at. -/
private def paramFreeTransportStep (head : Name) (typeArguments : Nat) : Simp.Simproc := fun e => do
  unless e.isAppOfArity head (3 + typeArguments) do return .continue
  let arguments := e.getAppArgs
  let types := arguments.extract 2 (2 + typeArguments)
  if types.any (fun type => (type.find? (·.isAppOf ``VerexIR.Proofs.Denote.NTy.param)).isSome) then
    return .continue
  let value := arguments.back!
  let transport := fun (held : Lean.Expr) => mkAppN e.getAppFn (arguments.pop.push held)
  let some identity ← (try some <$> FamilyTransport.pointwiseIdentity (← inferType value) transport
      catch _ => pure none) | return .continue
  let proof ← mkAppM ``congrFun #[identity, value]
  return .visit { expr := value, proof? := some (← mkExpectedTypeHint proof (← mkEq e value)) }

simproc [vir_denote_norm] paramFreeValueTransport
    (@VerexIR.Proofs.Denote.NTy.ofSkolem ?carriers _ _ _) :=
  paramFreeTransportStep ``VerexIR.Proofs.Denote.NTy.ofSkolem 1
simproc [vir_denote_norm] paramFreeVariantTransport
    (@VerexIR.Proofs.Denote.variantCarrier.ofSkolem ?carriers _ _ _ _) :=
  paramFreeTransportStep ``VerexIR.Proofs.Denote.variantCarrier.ofSkolem 2
simproc [vir_denote_norm] paramFreeRowTransport
    (@VerexIR.Proofs.Denote.HList.ofSkolem ?carriers _ _ _) :=
  paramFreeTransportStep ``VerexIR.Proofs.Denote.HList.ofSkolem 1

simproc ↓ [vir_denote, vir_denote_norm] variantNameTransport
    (@VerexIR.Proofs.Denote.variantName ?carriers _ _ _) :=
  fun e => do
    let_expr VerexIR.Proofs.Denote.variantName carriers _ _ value := e | return .continue
    let_expr VerexIR.Proofs.Denote.Skolems.toCarriers unit functions outer := carriers.consumeMData
      | return .continue
    let_expr VerexIR.Proofs.Denote.variantCarrier.ofSkolem _ θ names rows v := value
      | return .continue
    let proof := mkAppN (mkConst ``VerexIR.Proofs.Denote.variantName_ofSkolem)
      #[unit, functions, outer, θ, names, rows, v]
    let some (_, _, name) := (← inferType proof).eq? | return .continue
    return .visit { expr := name, proof? := some proof }

private def variantOuterFamily (transport : Name) (e : Lean.Expr) : SimpM Simp.Step := do
  let e := e.consumeMData
  let (unit, functions, skolems, names, rows, value, carried) ← match e.getAppArgs with
    | #[unit, functions, skolems, names, rows, value] =>
        pure (unit, functions, skolems, names, rows, value, false)
    | #[carriers, names, rows, value] =>
        let_expr VerexIR.Proofs.Denote.Skolems.toCarriers unit functions skolems :=
            carriers.consumeMData
          | return .continue
        pure (unit, functions, skolems, names, rows, value, true)
    | _ => return .continue
  let_expr VerexIR.Proofs.Denote.Skolems.instantiate _ _ θ outer := skolems.consumeMData
    | return .continue
  let some row := substitutionRow? θ | return .continue
  let some substituted := substituteNRows row rows (some (← getEnv)) | return .continue
  let some (result, equation) ← (try withTransparency .default do
      let transported := mkAppN (mkConst transport) #[unit, functions, outer, θ, names, rows, value]
      let some (_, atOuter, _) := (← inferType transported).eq? | return none
      let ofSkolem := atOuter.appArg!
      let_expr VerexIR.Proofs.Denote.variantCarrier.ofSkolem carriers _ _ _ _ := ofSkolem
        | return none
      let carrier ← inferType ofSkolem
      unless ← isDefEq carrier (← inferType value) do return none
      let heq ← mkAppOptM ``VerexIR.Proofs.Denote.variantCarrier.ofSkolem_heq
        #[some carriers, some θ, some names, some rows, some value]
      let identity ← mkAppOptM ``eq_of_heq #[some carrier, some ofSkolem, some value, some heq]
      let observe ← withLocalDeclD `value carrier fun value =>
        mkLambdaFVars #[value] (mkApp atOuter.appFn! value)
      let proof ← mkEqTrans (← mkEqSymm transported) (← mkCongrArg observe identity)
      let result := if carried then
          mkAppN e.getAppFn
            #[mkApp3 (mkConst ``VerexIR.Proofs.Denote.Skolems.toCarriers) unit functions outer,
              names, substituted, value]
        else mkAppN e.getAppFn #[unit, functions, outer, names, substituted, value]
      pure (some (result, ← mkExpectedTypeHint proof (← mkEq e result)))
    catch _ => pure none)
    | return .continue
  return .visit { expr := result, proof? := some equation }

simproc ↓ [vir_denote_norm] variantNameOuterFamily
    (@VerexIR.Proofs.Denote.variantName ?carriers _ _ _) :=
  variantOuterFamily ``VerexIR.Proofs.Denote.variantName_ofSkolem

private partial def containsStructQuoted (τ : Lean.Expr) : MetaM Bool := do
  let τView ← nativeTypeView τ
  if τView.isAppOf ``NTy.struct then return true
  τView.getAppArgs.anyM containsStructQuoted

simproc ↓ [vir_denote_norm] equalOuterFamily
    (@VerexIR.Proofs.Denote.NTy.equal ?carriers _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.NTy.equal family handles τ left right := e | return .continue
  let some (θ, outer) := inducedCarriers? family.consumeMData | return .continue
  let structFree := !(← containsStructQuoted τ)
  unless handles.isConstOf ``Bool.false || structFree do return .continue
  let some row := substitutionRow? θ | return .continue
  let some substituted := substituteNTy row τ (some (← getEnv)) | return .continue
  let some (value, equation) ← (try withTransparency .default do
      let atFalse := mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.equal)
        #[family, mkConst ``Bool.false, τ, left, right]
      let first ← mkExpectedTypeHint (← mkEqRefl e) (← mkEq e atFalse)
      let transported ← mkAppOptM ``VerexIR.Proofs.Denote.NTy.equal_ofSkolem
        #[some outer, some θ, some τ, some left, some right]
      let view (value : Lean.Expr) : MetaM Lean.Expr := do
        mkAppOptM ``VerexIR.Proofs.Denote.NTy.ofSkolem
          #[some outer, some θ, some τ, some value]
      let value := mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.equal)
        #[outer, mkConst ``Bool.false, substituted, ← view left, ← view right]
      let proof ← mkEqTrans first transported
      pure (some (value, ← mkExpectedTypeHint proof (← mkEq e value)))
    catch _ => pure none)
    | return .continue
  return .visit { expr := value, proof? := some equation }

mutual
/-- Native equality at a type that compares a value by every part, directly:
none of its parts is a type parameter's or a function value's, compared by an
erasure, or a ghost slot, which equality ignores. -/
def NTy.comparesByValue : NTy → Bool
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char => true
  | .tuple row | .struct _ _ row => NRow.comparesByValue row
  | .enum _ _ _ rows _ => NRows.comparesByValue rows
  | .vector element | .slice element | .array element _ => element.comparesByValue
  | .table key value => key.comparesByValue && value.comparesByValue
  | .ref referent => referent.comparesByValue
  | .sliceRef _ | .ghost _ | .param _ | .function _ _ _ | .nominal _ _ _
  | .observed _ => false

def NRow.comparesByValue : NRow → Bool
  | .nil => true
  | .cons τ rest => τ.comparesByValue && NRow.comparesByValue rest

def NRows.comparesByValue : NRows → Bool
  | .nil => true
  | .cons row rest => NRow.comparesByValue row && NRows.comparesByValue rest
end

section EqualAsEquation
variable [Carriers]

private theorem listEqual_iff_eq {α : Type} (equal : α → α → Bool)
    (same : ∀ a b, equal a b = true ↔ a = b) (left right : List α) :
    listEqual equal left right = true ↔ left = right := by
  simpa using NativeMaps.listEqual_iff_map equal id (by simpa using same) left right

mutual
theorem NTy.equal_iff_eq : (τ : NTy) → τ.comparesByValue = true → (left right : τ.carrier) →
    (NTy.equal false τ left right = true ↔ left = right)
  | .unit, _, _, _ => by simp [NTy.equal]
  | .bool, _, _, _ => by simp [NTy.equal]
  | .int _ _, _, left, right => by simp [NTy.equal, SpecInt.eq_iff_val left right]
  | .num, _, left, right => by simp [NTy.equal, SpecInt.eq_iff_val left right]
  | .address, _, _, _ => by simp [NTy.equal]
  | .signer, _, _, _ => by simp [NTy.equal]
  | .string, _, _, _ => by simp [NTy.equal]
  | .bytes, _, _, _ => by simp [NTy.equal]
  | .char, _, _, _ => by simp [NTy.equal]
  | .tuple row, plain, left, right => by
      simp only [NTy.equal]
      exact HList.equal_iff_eq row (by simpa [NTy.comparesByValue] using plain) left right
  | .struct _ _ row, plain, left, right => by
      simp only [NTy.equal, Bool.false_and, Bool.false_eq_true, ↓reduceIte]
      exact HList.equal_iff_eq row (by simpa [NTy.comparesByValue] using plain) left right
  | .enum _ _ names rows _, plain, left, right => by
      simp only [NTy.equal]
      exact variantCarrier.equal_iff_eq names rows (by simpa [NTy.comparesByValue] using plain)
        left right
  | .vector element, plain, left, right | .slice element, plain, left, right => by
      simp only [NTy.equal]
      rw [listEqual_iff_eq _
        (NTy.equal_iff_eq element (by simpa [NTy.comparesByValue] using plain))]
      exact ⟨fun same => SpecVector.ext (Array.toList_inj.mp same),
        fun same => congrArg (·.values.toList) same⟩
  | .array element _, plain, left, right => by
      simp only [NTy.equal]
      rw [listEqual_iff_eq _
        (NTy.equal_iff_eq element (by simpa [NTy.comparesByValue] using plain))]
      exact ⟨fun same => SpecArray.ext (SpecVector.ext (Array.toList_inj.mp same)),
        fun same => congrArg (·.vector.values.toList) same⟩
  | .table key value, plain, left, right => by
      have plains : key.comparesByValue = true ∧ value.comparesByValue = true := by
        simpa [NTy.comparesByValue] using plain
      simp only [NTy.equal]
      rw [listEqual_iff_eq _ fun a b => by
        simp only [Bool.and_eq_true, NTy.equal_iff_eq key plains.1,
          NTy.equal_iff_eq value plains.2, Prod.ext_iff]]
      exact ⟨fun same => SpecEntries.ext same, fun same => congrArg (·.entries) same⟩
  | .ref referent, plain, left, right => by
      have plain : referent.comparesByValue = true := by simpa [NTy.comparesByValue] using plain
      simp only [NTy.equal, Bool.and_eq_true, NTy.equal_iff_eq referent plain, Prod.ext_iff]
  | .sliceRef _, plain, _, _ | .ghost _, plain, _, _ | .param _, plain, _, _ | .function _ _ _, plain, _, _
  | .nominal _ _ _, plain, _, _ | .observed _, plain, _, _ => by
      simp [NTy.comparesByValue] at plain

/-- Rows comparing by value compare as the equation of the rows. -/
theorem HList.equal_iff_eq : (row : NRow) → row.comparesByValue = true →
    (left right : HList row) → (HList.equal false row left right = true ↔ left = right)
  | .nil, _, _, _ => by simp only [HList.equal]; exact ⟨fun _ => rfl, fun _ => trivial⟩
  | .cons τ rest, plain, left, right => by
      have plains : τ.comparesByValue = true ∧ rest.comparesByValue = true := by
        simpa [NRow.comparesByValue] using plain
      simp only [HList.equal, Bool.and_eq_true, NTy.equal_iff_eq τ plains.1,
        HList.equal_iff_eq rest plains.2]
      exact ⟨fun same => Prod.ext same.1 same.2, fun same => same ▸ ⟨rfl, rfl⟩⟩

/-- Variants comparing by value compare as the equation of the variants. -/
theorem variantCarrier.equal_iff_eq : (names : List String) → (rows : NRows) →
    rows.comparesByValue = true → (left right : variantCarrier names rows) →
    (variantCarrier.equal false names rows left right = true ↔ left = right)
  | _, .nil, _, left, _ => nomatch left
  | [], .cons _ _, _, left, _ => nomatch left
  | _ :: _, .cons fields rest, plain, .inl left, .inl right => by
      have plains : fields.comparesByValue = true ∧ rest.comparesByValue = true := by
        simpa [NRows.comparesByValue] using plain
      simp only [variantCarrier.equal]
      exact (HList.equal_iff_eq fields plains.1 left right).trans
        ⟨congrArg Sum.inl, Sum.inl.inj⟩
  | _ :: _, .cons _ _, _, .inl _, .inr _ => by simp [variantCarrier.equal]
  | _ :: _, .cons _ _, _, .inr _, .inl _ => by simp [variantCarrier.equal]
  | _ :: names, .cons fields rest, plain, .inr left, .inr right => by
      have plains : fields.comparesByValue = true ∧ rest.comparesByValue = true := by
        simpa [NRows.comparesByValue] using plain
      simp only [variantCarrier.equal]
      exact (variantCarrier.equal_iff_eq names rest plains.2 left right).trans
        ⟨congrArg Sum.inr, Sum.inr.inj⟩
end

/-- `NTy.equal_iff_eq` as a Boolean, in the form of native equality at a scalar. -/
theorem NTy.equal_eq_decide (τ : NTy) (plain : τ.comparesByValue = true) (left right : τ.carrier) :
    NTy.equal false τ left right = @decide (left = right) (Classical.propDecidable _) := by
  rw [Bool.eq_iff_iff, NTy.equal_iff_eq τ plain]
  simp only [decide_eq_true_eq]

end EqualAsEquation

/-- Whether a quoted type compares by value, read off its constructors; a type
not literal to its leaves does not count. -/
private partial def comparesByValueQuoted (τ : Lean.Expr) : MetaM Bool := do
  let τ ← nativeTypeView τ
  match τ.getAppFn.constName?, τ.getAppArgs with
  | some ``NTy.unit, _ | some ``NTy.bool, _ | some ``NTy.int, _ | some ``NTy.num, _
  | some ``NTy.address, _ | some ``NTy.signer, _ | some ``NTy.string, _ | some ``NTy.bytes, _ => pure true
  | some ``NTy.tuple, #[elements] => rowCompares elements
  | some ``NTy.struct, #[_, _, fields] => rowCompares fields
  | some ``NTy.enum, #[_, _, _, rows, _] => rowsCompare rows
  | some ``NTy.vector, #[element] => comparesByValueQuoted element
  | some ``NTy.table, #[key, value] => comparesByValueQuoted key <&&> comparesByValueQuoted value
  | some ``NTy.ref, #[referent] => comparesByValueQuoted referent
  | _, _ => pure false
where
  rowCompares (row : Lean.Expr) : MetaM Bool := do
    let row := row.consumeMData
    match row.getAppFn.constName?, row.getAppArgs with
    | some ``NRow.nil, _ => pure true
    | some ``NRow.cons, #[τ, rest] => comparesByValueQuoted τ <&&> rowCompares rest
    | _, _ => pure false
  rowsCompare (rows : Lean.Expr) : MetaM Bool := do
    let rows := rows.consumeMData
    match rows.getAppFn.constName?, rows.getAppArgs with
    | some ``NRows.nil, _ => pure true
    | some ``NRows.cons, #[row, rest] => rowCompares row <&&> rowsCompare rest
    | _, _ => pure false

/-- Native equality between values, one of them not built, at a compound type
comparing by value: the equation of the values, which rewrites where a
comparison of an opaque value by its parts is stuck. Built values compare part
by part (`HList.equal_cons`, `variantCarrier.equal_inr_inr`). -/
simproc ↓ [vir_denote_norm] equalAsEquation
    (@VerexIR.Proofs.Denote.NTy.equal ?carriers _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.NTy.equal family handles τ left right := e | return .continue
  unless handles.isConstOf ``Bool.false do return .continue
  if (inducedCarriers? family.consumeMData).isSome then return .continue
  let τView ← nativeTypeView τ
  if τView.getAppFn.constName?.any (· matches ``NTy.unit | ``NTy.bool | ``NTy.int | ``NTy.num
      | ``NTy.address | ``NTy.signer | ``NTy.string | ``NTy.bytes) then return .continue
  unless ← comparesByValueQuoted τ do return .continue
  let built (value : Lean.Expr) : MetaM Bool :=
    return (← isConstructorApp? value.consumeMData).isSome
  if ← (built left <&&> built right) then return .continue
  let plain ← mkExpectedTypeHint (mkApp2 (mkConst ``Eq.refl [1]) (mkConst ``Bool) (mkConst ``Bool.true))
    (← mkEq (mkApp (mkConst ``NTy.comparesByValue) τ) (mkConst ``Bool.true))
  let proof := mkAppN (mkConst ``NTy.equal_eq_decide) #[family, τ, plain, left, right]
  let some (_, _, value) := (← inferType proof).eq? | return .continue
  return .visit { expr := value, proof? := some proof }

/-- The name at a position of a literal list of names. -/
private partial def literalNameAt? (list : Lean.Expr) (position : Nat) : Option String :=
  let list := list.consumeMData
  if list.isAppOfArity ``List.cons 3 then
    if position == 0 then
      match (list.getArg! 1).consumeMData with
      | .lit (.strVal name) => some name
      | _ => none
    else literalNameAt? (list.getArg! 2) (position - 1)
  else none

/-- The variant a constructed enum value holds, read off its injections
(`variantName_inl`, `variantName_inr`) definitionally: the decided
inhabitation of an induced family's type arguments defeats simp's matching
of the lemmas at a callee's instance. -/
simproc [vir_denote, vir_denote_norm] variantNameConstructor
    (@VerexIR.Proofs.Denote.variantName ?carriers _ _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.variantName _ names _ value := e | return .continue
  let mut position := 0
  let mut current := value.consumeMData
  while current.isAppOfArity ``Sum.inr 3 do
    position := position + 1
    current := (current.getArg! 2).consumeMData
  unless current.isAppOfArity ``Sum.inl 3 do return .continue
  let some name := literalNameAt? names position | return .continue
  return .visit { expr := mkStrLit name }

/-- The row a native row expression denotes, in constructor form. -/
private def rowConstructor (row : Lean.Expr) : MetaM Lean.Expr := do
  let row := row.consumeMData
  if row.isAppOfArity ``VerexIR.Proofs.Denote.NRow.cons 2 || row.isConstOf ``VerexIR.Proofs.Denote.NRow.nil then
    return row
  whnfD row

/-- The rows a native rows expression denotes, in constructor form. -/
private def rowsConstructor (rows : Lean.Expr) : MetaM Lean.Expr := do
  let rows := rows.consumeMData
  if rows.isAppOfArity ``VerexIR.Proofs.Denote.NRows.cons 2 || rows.isConstOf ``VerexIR.Proofs.Denote.NRows.nil then
    return rows
  whnfD rows

/-- The components of a value of a product carrier: a pair's, or its
projections. -/
private def pairComponents (value : Lean.Expr) : MetaM (Lean.Expr × Lean.Expr) := do
  let value := value.consumeMData
  if value.isAppOfArity ``Prod.mk 4 then return (value.getArg! 2, value.getArg! 3)
  return (← mkAppM ``Prod.fst #[value], ← mkAppM ``Prod.snd #[value])

/-- The injections a constructed enum value is made of: its variant's
position and payload. -/
private def injections? (value : Lean.Expr) : MetaM (Option (Nat × Lean.Expr)) := do
  let mut position := 0
  let mut current := value.consumeMData
  while current.isAppOfArity ``Sum.inr 3 do
    position := position + 1
    current := (current.getArg! 2).consumeMData
  unless current.isAppOfArity ``Sum.inl 3 do return none
  return some (position, current.getArg! 2)

/-- The names and rows after the first `count` variants, with the row at
that position. -/
private def variantAt? (names rows : Lean.Expr) (count : Nat) :
    MetaM (Option (Lean.Expr × Lean.Expr × Lean.Expr)) := do
  let mut names := names.consumeMData
  let mut rows ← rowsConstructor rows
  for _ in [:count] do
    unless names.isAppOfArity ``List.cons 3 do return none
    let_expr VerexIR.Proofs.Denote.NRows.cons _ rest := rows | return none
    names := (names.getArg! 2).consumeMData
    rows ← rowsConstructor rest
  unless names.isAppOfArity ``List.cons 3 do return none
  let_expr VerexIR.Proofs.Denote.NRows.cons fields _ := rows | return none
  return some (names, rows, fields)

/-- The family a transport's type arguments induce over the family it carries. -/
private def inducedFamily (outer θ : Lean.Expr) : Lean.Expr :=
  mkApp2 (mkConst ``VerexIR.Proofs.Denote.Carriers.instantiate) θ outer

/-- A constructed enum value transported to a callee's family, injection by
injection (`variantCarrier.toSkolem_inl`, `variantCarrier.toSkolem_inr`)
definitionally. -/
dsimproc [vir_denote, vir_denote_norm] variantToSkolemConstructor
    (@VerexIR.Proofs.Denote.variantCarrier.toSkolem ?outer _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.variantCarrier.toSkolem outer θ names rows value := e
    | return .continue
  let some (position, payload) ← injections? value | return .continue
  let inner := inducedFamily outer θ
  let mut levels : Array (Lean.Expr × Lean.Expr × Lean.Expr) := #[]
  for i in [:position + 1] do
    let some level ← variantAt? names rows i | return .continue
    levels := levels.push level
  let some (_, _, fields) := levels[position]? | return .continue
  let mut built := mkAppN (mkConst ``VerexIR.Proofs.Denote.HList.toSkolem)
    #[outer, θ, fields, payload]
  for i in (List.range (position + 1)).reverse do
    let some (names, rows, fields) := levels[i]? | return .continue
    let_expr VerexIR.Proofs.Denote.NRows.cons _ rest := rows | return .continue
    let left := mkApp2 (mkConst ``VerexIR.Proofs.Denote.HList) inner fields
    let right := mkApp3 (mkConst ``VerexIR.Proofs.Denote.variantCarrier) inner
      (names.getArg! 2) rest
    built := if i == position then
      mkApp3 (mkConst ``Sum.inl [Level.zero, Level.zero]) left right built
    else mkApp3 (mkConst ``Sum.inr [Level.zero, Level.zero]) left right built
  return .visit built

dsimproc [vir_denote, vir_denote_norm] hlistToSkolemConstructor
    (@VerexIR.Proofs.Denote.HList.toSkolem ?outer _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.HList.toSkolem outer θ row values := e
    | return .continue
  let row ← rowConstructor row
  if row.isConstOf ``VerexIR.Proofs.Denote.NRow.nil then
    return .done (mkConst ``Unit.unit)
  let_expr VerexIR.Proofs.Denote.NRow.cons τ rest := row | return .continue
  let inner := inducedFamily outer θ
  let (head, tail) ← pairComponents values
  let headType := mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.carrier) inner τ
  let tailType := mkApp2 (mkConst ``VerexIR.Proofs.Denote.HList) inner rest
  return .visit (mkApp4 (mkConst ``Prod.mk [Level.zero, Level.zero]) headType tailType
    (mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.toSkolem) #[outer, θ, τ, head])
    (mkAppN (mkConst ``VerexIR.Proofs.Denote.HList.toSkolem)
      #[outer, θ, rest, tail]))

private def transportIdentity (identity : Name) (arguments : Array Lean.Expr) (value e : Lean.Expr) :
    SimpM Simp.Step := do
  let some equation ← (try withTransparency .default do
      let type ← inferType e
      unless ← isDefEq type (← inferType value) do return none
      let heq ← mkAppOptM identity (arguments.map some ++ #[some value])
      let proof ← mkAppOptM ``eq_of_heq #[some type, some e, some value, some heq]
      pure (some (← mkExpectedTypeHint proof (← mkEq e value)))
    catch _ => pure none)
    | return .continue
  return .visit { expr := value, proof? := some equation }

simproc ↓ [vir_denote, vir_denote_norm] ofSkolemIdentity
    (@VerexIR.Proofs.Denote.NTy.ofSkolem ?outer _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.NTy.ofSkolem outer θ τ value := e | return .continue
  transportIdentity ``VerexIR.Proofs.Denote.NTy.ofSkolem_heq #[outer, θ, τ] value e
simproc ↓ [vir_denote, vir_denote_norm] toSkolemIdentity
    (@VerexIR.Proofs.Denote.NTy.toSkolem ?outer _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.NTy.toSkolem outer θ τ value := e | return .continue
  transportIdentity ``VerexIR.Proofs.Denote.NTy.toSkolem_heq #[outer, θ, τ] value e
simproc ↓ [vir_denote, vir_denote_norm] hlistOfSkolemIdentity
    (@VerexIR.Proofs.Denote.HList.ofSkolem ?outer _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.HList.ofSkolem outer θ row values := e | return .continue
  transportIdentity ``VerexIR.Proofs.Denote.HList.ofSkolem_heq #[outer, θ, row] values e
simproc ↓ [vir_denote, vir_denote_norm] hlistToSkolemIdentity
    (@VerexIR.Proofs.Denote.HList.toSkolem ?outer _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.HList.toSkolem outer θ row values := e | return .continue
  transportIdentity ``VerexIR.Proofs.Denote.HList.toSkolem_heq #[outer, θ, row] values e
simproc ↓ [vir_denote, vir_denote_norm] variantOfSkolemIdentity
    (@VerexIR.Proofs.Denote.variantCarrier.ofSkolem ?outer _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.variantCarrier.ofSkolem outer θ names rows value := e
    | return .continue
  transportIdentity ``VerexIR.Proofs.Denote.variantCarrier.ofSkolem_heq #[outer, θ, names, rows] value e
simproc ↓ [vir_denote, vir_denote_norm] variantToSkolemIdentity
    (@VerexIR.Proofs.Denote.variantCarrier.toSkolem ?outer _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.variantCarrier.toSkolem outer θ names rows value := e
    | return .continue
  transportIdentity ``VerexIR.Proofs.Denote.variantCarrier.toSkolem_heq #[outer, θ, names, rows] value e

private def extractPlainFallback? (e : Lean.Expr) : Option (Lean.Expr × Lean.Expr × Lean.Expr) :=
  let e := e.consumeMData
  if e.isAppOfArity ``VerexIR.Proofs.Denote.NTy.plainFallback 3 then
    some (e.getArg! 0, e.getArg! 1, e.getArg! 2)
  else if e.isAppOfArity ``VerexIR.Proofs.Denote.plainFallback 2 then
    let ne := (e.getArg! 1).consumeMData
    if ne.isAppOfArity ``VerexIR.Proofs.Denote.NTy.nonempty_of_inhabitable 3 then
      some (ne.getArg! 0, ne.getArg! 1, ne.getArg! 2)
    else none
  else none

private def extractFallback? (e : Lean.Expr) : Option (Bool × Lean.Expr × Lean.Expr × Lean.Expr) :=
  let e := e.consumeMData
  if let some (family, τ, inhabitable) := extractPlainFallback? e then
    some (true, family, τ, inhabitable)
  else if e.isAppOfArity ``VerexIR.Proofs.Denote.NTy.inhabitantOf 3 then
    some (false, e.getArg! 0, e.getArg! 1, e.getArg! 2)
  else none

simproc ↓ [vir_denote_norm] inhabitantOuterFamily
    (@VerexIR.Proofs.Denote.NTy.inhabitantOf ?family _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.NTy.inhabitantOf family τ inhabitable := e
    | return .continue
  if e.hasLooseBVars then return .continue
  let some (θ, outer) := inducedCarriers? family.consumeMData | return .continue
  let some row := substitutionRow? θ | return .continue
  let some substituted := substituteNTy row τ (some (← getEnv)) | return .continue
  let some (value, equation) ← (try withTransparency .default do
      let inhabitable' := mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.inhabitable) substituted
      unless (← whnf inhabitable').isConstOf ``Bool.true do return none
      let decided ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
        (← mkEq inhabitable' (mkConst ``Bool.true))
      let value := mkApp3 (mkConst ``VerexIR.Proofs.Denote.NTy.inhabitantOf) outer
        substituted decided
      let type ← inferType e
      unless ← isDefEq type (← inferType value) do return none
      let heq ← mkAppOptM
        ``VerexIR.Proofs.Denote.FamilyTransport.inhabitantOf_instantiate_heq
        #[some outer, some θ, some τ, some inhabitable, some decided]
      let proof ← mkAppOptM ``eq_of_heq #[some type, some e, some value, some heq]
      pure (some (value, ← mkExpectedTypeHint proof (← mkEq e value)))
    catch _ => pure none)
    | return .continue
  return .visit { expr := value, proof? := some equation }

simproc ↓ [vir_denote_norm] plainFallbackOuterFamily
    (@VerexIR.Proofs.Denote.NTy.plainFallback ?family _ _) := fun e => do
  let e := e.consumeMData
  let some (family, τ, inhabitable) := extractPlainFallback? e | return .continue
  if e.hasLooseBVars then return .continue
  let some (θ, outer) := inducedCarriers? family.consumeMData | return .continue
  let some row := substitutionRow? θ | return .continue
  let some substituted := substituteNTy row τ (some (← getEnv)) | return .continue
  let some (value, equation) ← (try withTransparency .default do
      let inhabitable' := mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.inhabitable) substituted
      unless (← whnf inhabitable').isConstOf ``Bool.true do return none
      let decided ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
        (← mkEq inhabitable' (mkConst ``Bool.true))
      let value := mkApp3 (mkConst ``VerexIR.Proofs.Denote.NTy.plainFallback) outer
        substituted decided
      let type ← inferType e
      unless ← isDefEq type (← inferType value) do return none
      let heq ← mkAppOptM
        ``VerexIR.Proofs.Denote.NTy.plainFallback_instantiate_heq
        #[some outer, some θ, some τ, some inhabitable]
      let proof ← mkAppOptM ``eq_of_heq #[some type, some e, some value, some heq]
      pure (some (value, ← mkExpectedTypeHint proof (← mkEq e value)))
    catch _ => pure none)
    | return .continue
  return .visit { expr := value, proof? := some equation }

private def ownView? (carrier : Lean.Expr) : MetaM (Option NativeMaps.TypedView) := do
  let_expr VerexIR.Proofs.Denote.NTy.carrier carriers τ := carrier.consumeMData | return none
  let some view ← NativeMaps.typedView? carriers τ | return none
  unless view.carrier == carrier do return none
  return some view

private def mapTypes? (map : Lean.Expr) : MetaM (Option (Lean.Expr × Lean.Expr)) := do
  let type ← instantiateMVars (← inferType map)
  if let .app (.app (.const ``VerexIR.SpecMap _) K) V := type.consumeMData then return some (K, V)
  let_expr List pair := ← whnfR type | return none
  let_expr Prod K V := pair.consumeMData | return none
  return some (K, V)

private def ownInstance? (inhabited : Lean.Expr) (view : NativeMaps.TypedView) :
    MetaM (Option (Option Lean.Expr)) := do
  let induced? : Option (Bool × Lean.Expr × Lean.Expr × Lean.Expr) := do
    let_expr VerexIR.Proofs.Denote.nativeInhabited _ _ fallback := inhabited.consumeMData | none
    let (isPlain, family, τ, inhabitable) ← extractFallback? fallback
    let _ ← inducedCarriers? family.consumeMData
    some (isPlain, family, τ, inhabitable)
  try withTransparency .default do
    let some (isPlain, family, τ, inhabitable) := induced?
      | if ← withReducible (isDefEq inhabited view.inhabited) then return some none else return none
    let_expr VerexIR.Proofs.Denote.nativeInhabited outer substituted own := view.inhabited.consumeMData
      | return none
    let some (ownPlain, ownFamily, _, _) := extractFallback? own | return none
    unless isPlain == ownPlain do return none
    let fallback := inhabited.consumeMData.appArg!
    let mut family := family
    let mut τ := τ
    let mut inhabitable := inhabitable
    let mut chain : Option Lean.Expr := none
    repeat
      let some (θ, below) := inducedCarriers? family.consumeMData | return none
      let some row := substitutionRow? θ | return none
      let some next := substituteNTy row τ (some (← getEnv)) | return none
      let decided ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
        (← mkEq (mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.inhabitable) next)
          (mkConst ``Bool.true))
      let heq ← if isPlain then
          mkAppOptM ``VerexIR.Proofs.Denote.NTy.plainFallback_instantiate_heq
            #[some below, some θ, some τ, some inhabitable]
        else
          mkAppOptM ``VerexIR.Proofs.Denote.FamilyTransport.inhabitantOf_instantiate_heq
            #[some below, some θ, some τ, some inhabitable, some decided]
      chain ← match chain with
        | some known => pure (some (← mkHEqTrans known heq))
        | none => pure (some heq)
      family := below
      τ := next
      inhabitable := decided
      if below == ownFamily then break
    let some equal := chain | return none
    let carrier ← inferType own
    let defaults ← mkAppOptM ``eq_of_heq #[some carrier, some fallback, some own, some equal]
    let instanceOf ← withLocalDeclD `fallback carrier fun fallback =>
      mkLambdaFVars #[fallback]
        (mkApp3 (mkConst ``VerexIR.Proofs.Denote.nativeInhabited) outer substituted fallback)
    let proof ← mkCongrArg instanceOf defaults
    return some (some (← mkExpectedTypeHint proof (← mkEq inhabited view.inhabited)))
  catch _ => return none

private def ownReadStep (e : Lean.Expr) (typePosition instancePosition : Nat) (own : Lean.Expr)
    (view : Unit → MetaM (Option NativeMaps.TypedView)) : SimpM Simp.Step := do
  let args := e.getAppArgs
  let inhabited := args[instancePosition]!
  let induced := match inhabited.consumeMData with
    | .app (.app (.app (.const ``VerexIR.Proofs.Denote.nativeInhabited _) _) _) fallback =>
        (extractFallback? fallback).any fun (_, family, _, _) => (inducedCarriers? family.consumeMData).isSome
    | _ => false
  if args[typePosition]! == own && !induced then return .continue
  let some view ← view () | return .continue
  if args[typePosition]! == own && inhabited == view.inhabited then return .continue
  let some proof? ← ownInstance? inhabited view | return .continue
  let value := mkAppN e.getAppFn
    ((args.set! typePosition own).set! instancePosition view.inhabited)
  let some instanceEq := proof? | return .visit { expr := value }
  let some equation ← (try withTransparency .default do
      let read ← withLocalDeclD `inhabited (← mkAppM ``Inhabited #[own]) fun inhabited =>
        mkLambdaFVars #[inhabited]
          (mkAppN e.getAppFn ((args.set! typePosition own).set! instancePosition inhabited))
      let proof ← mkCongrArg read instanceEq
      pure (some (← mkExpectedTypeHint proof (← mkEq e value)))
    catch _ => pure none)
    | return .continue
  return .visit { expr := value, proof? := some equation }

/-- A typed map's value read at the map's own value type and default: a
contract names the value type and its default at the family its call
induces, while the map, named by the caller, is at the caller's; the two
spellings are one type, and the defaults one value (`ownInstance?`). -/
simproc [vir_denote_norm] valueAtOwnInstance (VerexIR.SpecMap.valueAt _ _) :=
  fun e => do
  let e := e.consumeMData
  let_expr VerexIR.SpecMap.valueAt _ _ _ _ map _ := e | return .continue
  let some (_, V) ← mapTypes? map | return .continue
  ownReadStep e 1 3 V fun _ => ownView? V

/-- A typed map's key read at the map's own key type and default
(`valueAtOwnInstance`). -/
simproc [vir_denote_norm] keyAtOwnInstance (VerexIR.SpecMap.keyAt _ _) :=
  fun e => do
  let e := e.consumeMData
  let_expr VerexIR.SpecMap.keyAt _ _ _ map _ := e | return .continue
  let some (K, _) ← mapTypes? map | return .continue
  ownReadStep e 0 2 K fun _ => ownView? K

/-- The component a variable selects from a row of values, by the
variable's constructors (`Var.select`) definitionally. -/
dsimproc [vir_denote, vir_denote_norm] varSelectConstructor
    (@VerexIR.Proofs.Denote.Var.select ?inst _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.Var.select _ _ _ selector values := e | return .continue
  let mut selector := selector.consumeMData
  let mut values := values
  while selector.isAppOfArity ``VerexIR.Proofs.Denote.Var.there 4 do
    selector := (selector.getArg! 3).consumeMData
    values := (← pairComponents values).2
  unless selector.isAppOfArity ``VerexIR.Proofs.Denote.Var.here 2 do return .continue
  return .visit (← pairComponents values).1

/-- A choice's projection of a constructed enum value, by the choice's and
the value's constructors (`Which.project?`) definitionally: the payload
where the injections agree, nothing otherwise. -/
dsimproc [vir_denote, vir_denote_norm] whichProjectConstructor
    (@VerexIR.Proofs.Denote.Which.project? ?carriers _ _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.Which.project? _ _ _ _ choice value := e
    | return .continue
  let mut depth := 0
  let mut choice := choice.consumeMData
  while choice.isAppOfArity ``VerexIR.Proofs.Denote.Which.there 6 do
    depth := depth + 1
    choice := (choice.getArg! 5).consumeMData
  unless choice.isAppOfArity ``VerexIR.Proofs.Denote.Which.here 4 do return .continue
  let some (position, payload) ← injections? value | return .continue
  -- The row's type as the projection states it, so that the lemmas on
  -- the option match the context's spelling.
  let_expr Option row := (← instantiateMVars (← inferType e)) | return .continue
  if position == depth then
    return .visit (mkApp2 (mkConst ``Option.some [Level.zero]) row payload)
  return .visit (mkApp (mkConst ``Option.none [Level.zero]) row)

/-- Close `goal`, an equation between two projections of one variant value
`value`, case by case on the value: at a constructor both sides reduce. -/
private partial def projectionsAgree (goal : MVarId) (value : FVarId) : MetaM Bool := do
  try goal.refl; return true catch _ => pure ()
  -- A variant carrier is a sum by its definition, which `cases` does not unfold.
  let goal ← goal.withContext do
    goal.replaceLocalDeclDefEq value (← whnfD (← value.getType))
  let some subgoals ← (some <$> goal.cases value) <|> pure none | return false
  for subgoal in subgoals do
    if ← (do subgoal.mvarId.refl; pure true) <|> pure false then continue
    -- The value inside the case: the rest of the variants, where the value is
    -- not the first, or the plain side of a certificate, which stands between
    -- an enum's value and its variants (`NTy.carrier`) and which the
    -- recursion looks past to the sum.  A certificate itself is a field of
    -- the case too, and is not what the projections read.
    let mut inside := none
    for field in subgoal.fields do
      unless field.isFVar do continue
      let shape ← subgoal.mvarId.withContext do whnfD (← inferType field)
      if shape.isAppOfArity ``Sum 2 then
        inside := some field.fvarId!
        break
    let some value := inside | return false
    unless ← projectionsAgree subgoal.mvarId value do return false
  return true

open Lean Meta Simp in
/-- A choice's projection at a frame type arguments induce, at the outer
frame with the types substituted: convertible where the two frames' carriers
unfold to the same type, and equal case by case on the value otherwise, since
the rows a choice carries are data, which no transparency respells. The
projection itself reduces on the choice alone, reading its value with
`Sum.elim`. A step proof's leaf reads a callee's results so
(`verex_step_obligation`); the automatic normal form keeps the callee's
spelling. -/
simproc_decl whichProjectFamily
    (@VerexIR.Proofs.Denote.Which.project? ?carriers _ _ _ _ _) := fun e => do
  let e := e.consumeMData
  unless e.isAppOfArity ``VerexIR.Proofs.Denote.Which.project? 6 && !e.hasLooseBVars do
    return .continue
  let some (respelled, _) ← FamilyTransport.respelledOperation? e | return .continue
  let valueType ← inferType e.appArg!
  let statement ← withLocalDeclD `value valueType fun value => do
    mkForallFVars #[value] (← mkEq (mkApp e.appFn! value) (mkApp respelled.appFn! value))
  let proof ← mkFreshExprMVar statement
  let (value, goal) ← proof.mvarId!.intro1
  -- The carriers of the two frames agree by unfolding their definitions.
  unless ← withTransparency .default (projectionsAgree goal value) do return .continue
  return .visit { expr := respelled, proof? := some (mkApp (← instantiateMVars proof) e.appArg!) }

open Lean Meta Simp in
/-- A choice selection at a frame type arguments induce, at the outer
frame with the types substituted: established as a variant projection is
(`whichProjectFamily`), by conversion where the two frames' carriers unfold
to the same type and case by case on the value otherwise, since a position
witness's rows are data, which no transparency respells. -/
simproc ↓ [vir_denote, vir_denote_norm] choicesSelectFamily
    (@VerexIR.Proofs.Denote.Choices.select? ?carriers _ _ _ _ _) :=
    fun e => do
  let e := e.consumeMData
  unless e.isAppOfArity ``VerexIR.Proofs.Denote.Choices.select? 6 &&
      !e.hasLooseBVars do
    return .continue
  let some (respelled, _) ← FamilyTransport.respelledOperation? e | return .continue
  let valueType ← inferType e.appArg!
  let statement ← withLocalDeclD `value valueType fun value => do
    mkForallFVars #[value] (← mkEq (mkApp e.appFn! value) (mkApp respelled.appFn! value))
  let proof ← mkFreshExprMVar statement
  let (value, goal) ← proof.mvarId!.intro1
  unless ← withTransparency .default (projectionsAgree goal value) do return .continue
  return .visit { expr := respelled, proof? := some (mkApp (← instantiateMVars proof) e.appArg!) }

/-- A choice's projection of a constructed enum value, mapped: the mapped
payload, or nothing, typed as the map spells its result. A contract spells
the map at the caller's family and the projection at the callee's, which
no lemma on the option matches across. -/
dsimproc ↓ [vir_denote, vir_denote_norm] optionMapProjectConstructor
    (Option.map _ (@VerexIR.Proofs.Denote.Which.project? ?carriers _ _ _ _ _)) := fun e => do
  let e := e.consumeMData
  let_expr Option.map _ β function projection := e | return .continue
  let_expr VerexIR.Proofs.Denote.Which.project? _ _ _ _ choice value := projection
    | return .continue
  let mut depth := 0
  let mut choice := choice.consumeMData
  while choice.isAppOfArity ``VerexIR.Proofs.Denote.Which.there 6 do
    depth := depth + 1
    choice := (choice.getArg! 5).consumeMData
  unless choice.isAppOfArity ``VerexIR.Proofs.Denote.Which.here 4 do return .continue
  let some (position, payload) ← injections? value | return .continue
  if position == depth then
    return .visit (mkApp2 (mkConst ``Option.some [Level.zero]) β (mkApp function payload).headBeta)
  return .visit (mkApp (mkConst ``Option.none [Level.zero]) β)

/-- An option's map and default at a constructor, definitionally: where the
option's element type is spelled at one family and the map's at another,
the lemmas do not match. -/
dsimproc [vir_denote, vir_denote_norm] optionMapSomeConstructor (Option.map _ (some _)) := fun e => do
  let e := e.consumeMData
  let_expr Option.map _ β function option := e | return .continue
  let_expr Option.some _ value := option.consumeMData | return .continue
  return .visit (mkApp2 (mkConst ``Option.some [Level.zero]) β (mkApp function value).headBeta)
dsimproc [vir_denote, vir_denote_norm] optionGetDSomeConstructor (Option.getD (some _) _) := fun e => do
  let e := e.consumeMData
  let_expr Option.getD _ option _ := e | return .continue
  let_expr Option.some _ value := option.consumeMData | return .continue
  return .visit value
dsimproc [vir_denote, vir_denote_norm] optionGetDNoneConstructor (Option.getD none _) := fun e => do
  let e := e.consumeMData
  let_expr Option.getD _ option fallback := e | return .continue
  unless option.consumeMData.isAppOfArity ``Option.none 1 do return .continue
  return .visit fallback
dsimproc [vir_denote, vir_denote_norm] optionBindSomeConstructor (Option.bind (some _) _) := fun e => do
  let e := e.consumeMData
  let_expr Option.bind _ _ option function := e | return .continue
  let_expr Option.some _ value := option.consumeMData | return .continue
  return .visit (mkApp function value).headBeta
dsimproc [vir_denote, vir_denote_norm] optionBindNoneConstructor (Option.bind none _) := fun e => do
  let e := e.consumeMData
  let_expr Option.bind _ β option _ := e | return .continue
  unless option.consumeMData.isAppOfArity ``Option.none 1 do return .continue
  return .visit (mkApp (mkConst ``Option.none [Level.zero]) β)
dsimproc [vir_denote, vir_denote_norm] optionIsSomeSomeConstructor (Option.isSome (some _)) := fun e => do
  let e := e.consumeMData
  let_expr Option.isSome _ option := e | return .continue
  let_expr Option.some _ _ := option.consumeMData | return .continue
  return .visit (mkConst ``Bool.true)
dsimproc [vir_denote, vir_denote_norm] optionIsSomeNoneConstructor (Option.isSome none) := fun e => do
  let e := e.consumeMData
  let_expr Option.isSome _ option := e | return .continue
  unless option.consumeMData.isAppOfArity ``Option.none 1 do return .continue
  return .visit (mkConst ``Bool.false)

/-- Two options compared, at whatever family each side spells its element type:
`some = some` compares payloads (`Option.some.injEq`) and `some`/`none` refutes
(`Option.some_ne_none`), even when each side spells `NTy.carrier` at a distinct
`Carriers` instance. -/
simproc [vir_denote, vir_denote_norm] optionInjectionFamilies (@Eq _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr Eq _ lhs rhs := e | return .continue
  let lhs := lhs.consumeMData
  let rhs := rhs.consumeMData
  let left := if lhs.isAppOfArity ``Option.some 2 then some true
    else if lhs.isAppOfArity ``Option.none 1 then some false else none
  let right := if rhs.isAppOfArity ``Option.some 2 then some true
    else if rhs.isAppOfArity ``Option.none 1 then some false else none
  let (some leftSome, some rightSome) := (left, right) | return .continue
  if !leftSome && !rightSome && lhs == rhs then return .continue
  let α := lhs.getArg! 0
  let some equation ← (try withTransparency .default do
      match leftSome, rightSome with
      | true, true =>
        let proof ← mkAppOptM ``Option.some.injEq #[some α, some (lhs.getArg! 1), some (rhs.getArg! 1)]
        let some (_, _, inner) := (← inferType proof).eq? | return none
        pure (some (inner, ← mkExpectedTypeHint proof (← mkEq e inner)))
      | true, false =>
        let neProof ← mkAppOptM ``Option.some_ne_none #[some α, some (lhs.getArg! 1)]
        let proof ← mkAppM ``eq_false #[neProof]
        pure (some (mkConst ``False, ← mkExpectedTypeHint proof (← mkEq e (mkConst ``False))))
      | false, true =>
        let neProof ← mkAppM ``Ne.symm #[← mkAppOptM ``Option.some_ne_none #[some (rhs.getArg! 0), some (rhs.getArg! 1)]]
        let proof ← mkAppM ``eq_false #[neProof]
        pure (some (mkConst ``False, ← mkExpectedTypeHint proof (← mkEq e (mkConst ``False))))
      | false, false =>
        let eqProof ← mkExpectedTypeHint (← mkEqRefl lhs) e
        let proof ← mkAppM ``eq_true #[eqProof]
        pure (some (mkConst ``True, ← mkExpectedTypeHint proof (← mkEq e (mkConst ``True))))
    catch _ => pure none)
    | return .continue
  return .visit { expr := equation.1, proof? := some equation.2 }

/-- Two injections compared, at whatever family each side spells its sum:
the same injection compares its payloads, different ones refute
(`Sum.inr.injEq`, `Sum.inl_ne_inr`), which the lemmas state at one spelling.
Keyed on every equation: a pattern naming the injections would fix the
equation's type to a sum, where a contract states it at the enum's carrier. -/
simproc [vir_denote, vir_denote_norm] sumInjectionFamilies (@Eq _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr Eq _ lhs rhs := e | return .continue
  let lhs := lhs.consumeMData
  let rhs := rhs.consumeMData
  let left := if lhs.isAppOfArity ``Sum.inl 3 then some true
    else if lhs.isAppOfArity ``Sum.inr 3 then some false else none
  let right := if rhs.isAppOfArity ``Sum.inl 3 then some true
    else if rhs.isAppOfArity ``Sum.inr 3 then some false else none
  let (some leftInl, some rightInl) := (left, right) | return .continue
  let arguments := #[some (lhs.getArg! 0), some (lhs.getArg! 1), some (lhs.getArg! 2), some (rhs.getArg! 2)]
  let some equation ← (try withTransparency .default do
      if leftInl == rightInl then
        let proof ← mkAppOptM (if leftInl then ``Sum.inl.injEq else ``Sum.inr.injEq) arguments
        let some (_, _, inner) := (← inferType proof).eq? | return none
        pure (some (inner, ← mkExpectedTypeHint proof (← mkEq e inner)))
      else
        let proof ← mkAppM ``eq_false
          #[← mkAppOptM (if leftInl then ``Sum.inl_ne_inr else ``Sum.inr_ne_inl) arguments]
        pure (some (mkConst ``False, ← mkExpectedTypeHint proof (← mkEq e (mkConst ``False))))
    catch _ => pure none)
    | return .continue
  return .visit { expr := equation.1, proof? := some equation.2 }

/-- The components of a side of an equation at `carrier`, a product: a pair's,
or its reads, as the normal form spells them. -/
private def rowComponents (carrier side : Lean.Expr) : Lean.Expr × Lean.Expr :=
  if side.isAppOfArity ``Prod.mk 4 then (side.getArg! 2, side.getArg! 3)
  else
    let levels := carrier.getAppFn.constLevels!
    let (α, β) := (carrier.getArg! 0, carrier.getArg! 1)
    (mkApp3 (mkConst ``Prod.fst levels) α β side, mkApp3 (mkConst ``Prod.snd levels) α β side)

/-- What a type spells of a row of values: the unit, or the product it
unfolds to. -/
private inductive RowSpelling where
  | unit
  | pair (carrier : Lean.Expr)

/-- The row a type spells, read from the spelling alone: `Prod α β`, `Unit`,
`HList row` and `NTy.carrier _ (.tuple row)`. The row needs no more than
reducible unfolding; any other type is not a row here. -/
private def rowSpelling? (spelled : Lean.Expr) : MetaM (Option RowSpelling) := do
  if spelled.isAppOfArity ``Prod 2 then return some (.pair spelled)
  if spelled.isConstOf ``Unit || spelled.isAppOf ``PUnit then return some .unit
  let carrierView ← if spelled.isAppOfArity ``NTy.carrier 2 then
      nativeTypeView (spelled.getArg! 1) else pure spelled
  if spelled.isAppOfArity ``NTy.carrier 2 && carrierView.isAppOfArity ``NTy.ref 1 then
    let carriers := spelled.getArg! 0
    let component := mkApp2 (mkConst ``NTy.carrier) carriers (carrierView.getArg! 0)
    return some (.pair (mkApp2 (mkConst ``Prod [levelZero, levelZero]) component component))
  let row? : Option (Lean.Expr × Lean.Expr) :=
    if spelled.isAppOfArity ``HList 2 then some (spelled.getArg! 0, spelled.getArg! 1)
    else if spelled.isAppOfArity ``NTy.carrier 2 && carrierView.isAppOfArity ``NTy.tuple 1 then
      some (spelled.getArg! 0, carrierView.getArg! 0)
    else if spelled.isAppOfArity ``NTy.carrier 2 && carrierView.isAppOfArity ``NTy.struct 3 then
      some (spelled.getArg! 0, carrierView.getArg! 2)
    else none
  let some (carriers, row) := row? | return none
  let row ← withReducible (whnf row)
  if row.isConstOf ``NRow.nil then return some .unit
  unless row.isAppOfArity ``NRow.cons 2 do return none
  let head := mkApp2 (mkConst ``NTy.carrier) carriers (row.getArg! 0)
  let tail := mkApp2 (mkConst ``HList) carriers (row.getArg! 1)
  return some (.pair (mkApp2 (mkConst ``Prod [levelZero, levelZero]) head tail))

simproc [vir_denote_norm] certifiedByConstruction (@Eq _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr Eq type lhs rhs := e | return .continue
  let lhs := lhs.consumeMData
  let rhs := rhs.consumeMData
  let constructed (side : Lean.Expr) : Bool :=
    side.isAppOfArity ``VerexIR.SpecInt.mk 4
  let type ← whnfR type
  unless type.isAppOfArity ``VerexIR.SpecInt 2 do return .continue
  let substitutable (side other : Lean.Expr) := match side with
    | .fvar id => !other.containsFVar id
    | _ => false
  if substitutable lhs rhs || substitutable rhs lhs then return .continue
  unless constructed lhs != constructed rhs do return .continue
  let equivalence ←
    (try pure (some (← mkAppM ``VerexIR.SpecInt.eq_iff_val #[lhs, rhs])) catch _ => pure none)
  let some equivalence := equivalence | return .continue
  let reduced ← (try
      let statement ← whnfR (← inferType equivalence)
      let_expr Iff stated values := statement | pure none
      if !(← isDefEq stated e) then pure none
      else
        let proof ← mkAppM ``propext #[equivalence]
        if ← isTypeCorrect proof then pure (some (values, proof)) else pure none
    catch _ => pure none)
  let some (values, proof) := reduced | return .continue
  return .visit { expr := values, proof? := some proof }

simproc [vir_denote_norm] pairInjectionRows (@Eq _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr Eq type lhs rhs := e | return .continue
  let lhs := lhs.consumeMData
  let rhs := rhs.consumeMData
  let observation (side : Lean.Expr) : Bool :=
    side.isAppOf ``VerexIR.Proofs.ResultOf || side.isAppOf ``VerexIR.Proofs.StateOf
  if observation lhs || observation rhs then return .continue
  let spelled := type.consumeMData
  let (leftPair, rightPair) := (lhs.isAppOfArity ``Prod.mk 4, rhs.isAppOfArity ``Prod.mk 4)
  if leftPair && rightPair && spelled.isAppOfArity ``Prod 2 then return .continue
  let carrierView ← if spelled.isAppOfArity ``NTy.carrier 2 then
      nativeTypeView (spelled.getArg! 1) else pure spelled
  let tupleRow := spelled.isAppOf ``HList ||
    (spelled.isAppOfArity ``NTy.carrier 2 && carrierView.isAppOf ``NTy.tuple)
  let unitSide := lhs.isConstOf ``Unit.unit || lhs.isConstOf ``PUnit.unit ||
    rhs.isConstOf ``Unit.unit || rhs.isConstOf ``PUnit.unit
  unless leftPair || rightPair || tupleRow || unitSide do return .continue
  let substitutable (side other : Lean.Expr) : Bool := match side with
    | .fvar x => !other.containsFVar x
    | _ => false
  if substitutable lhs rhs || substitutable rhs lhs then return .continue
  match ← rowSpelling? spelled with
  | none => return .continue
  | some .unit =>
    let same ← mkExpectedTypeHint (← mkEqRefl lhs) e
    return .visit { expr := mkConst ``True, proof? := some (mkApp2 (mkConst ``eq_true) e same) }
  | some (.pair carrier) =>
    let (leftFirst, leftSecond) := rowComponents carrier lhs
    let (rightFirst, rightSecond) := rowComponents carrier rhs
    let components ← mkAppM ``And #[← mkEq leftFirst rightFirst, ← mkEq leftSecond rightSecond]
    let split := mkApp4 (mkConst ``Prod.ext_iff carrier.getAppFn.constLevels!)
      (carrier.getArg! 0) (carrier.getArg! 1) lhs rhs
    return .visit { expr := components, proof? := some (mkApp3 (mkConst ``propext) e components split) }

/-- The value and position a pair's component reads, as a read of a value. -/
private def pairRead? (component : Lean.Expr) : Option (Lean.Expr × Bool) :=
  let component := component.consumeMData
  if component.isAppOfArity ``Prod.fst 3 then some (component.getArg! 2, true)
  else if component.isAppOfArity ``Prod.snd 3 then some (component.getArg! 2, false)
  else match component with
    | .proj ``Prod index value => some (value, index == 0)
    | _ => none

/-- Whether a type's spelling is a row's end: a row with no entry, at
whatever layer the type spells it (`rowSpelling?`) — `Unit`, the empty row
of certified values, the empty plain row.  The one reader of row spellings
answers this, so that a layer it reads is a row's end for every rule that
ends a row through this. -/
private def spelledRowEnd (type : Lean.Expr) : MetaM Bool := do
  match ← rowSpelling? (← whnfR type) with
  | some .unit => return true
  | _ => return false

/-- Whether a type's spelling is a row of one entry: the row the type spells
has an entry (`rowSpelling?`) and the rest of it is the row's end. -/
private def spelledSingleton (type : Lean.Expr) : MetaM Bool := do
  match ← rowSpelling? (← whnfR type) with
  | some (.pair carrier) => spelledRowEnd (carrier.getArg! 1)
  | _ => return false

/-- A row of the reads of one value is that value, up to its unit end
(`(e.1, e.2)`, `(e.1, ())` where the spelling of `e`'s type shows a unit second
component): the form the components of an equation leave where a value is rebuilt
from them. Definitional, by structure eta. -/
dsimproc [vir_denote_norm] rowOfReads (Prod.mk _ _) := fun e => do
  let e := e.consumeMData
  unless e.isAppOfArity ``Prod.mk 4 do return .continue
  let some (value, true) := pairRead? (e.getArg! 2) | return .continue
  -- Eta stays within the row's layer. Forgetting a certificate-free
  -- singleton can leave `(row.1, ())` at the plain row; replacing it with
  -- the certified row would reintroduce its implicit component types.
  let some (.pair carrier) ← rowSpelling? (← inferType value) | return .continue
  unless ← withReducible (isDefEq carrier (← inferType e)) do return .continue
  let second := (e.getArg! 3).consumeMData
  if let some (other, false) := pairRead? second then
    return if other == value then .visit value else .continue
  unless second.isConstOf ``Unit.unit || second.isConstOf ``PUnit.unit do return .continue
  unless ← spelledSingleton (← inferType value) do return .continue
  return .visit value

open Lean Meta in
/-- A declaration's data invariant of a value where the declaration states
none: `True`.  The unit's own clauses are a field of its function values
(`FunctionCarriers.own`), so a pack of a declaration without an invariant
leaves a cut over the trivial predicate, which closes by unfolding the
field alone.  Reduced here, the cut closes in the normal form and no leaf
is spent on it; a declaration that does state an invariant keeps its
clauses, which the step that reports the obligation reads
(`doc/designs/data-invariants.md`). -/
dsimproc [vir_denote, vir_denote_norm] ownWithoutInvariant
    (@VerexIR.Proofs.Denote.FunctionCarriers.own _ ?functions _ _ _) := fun e => do
  unless e.isAppOfArity ``VerexIR.Proofs.Denote.FunctionCarriers.own 5 do return .continue
  -- The field alone, not the application: a unit that states an invariant
  -- keeps a predicate here, and reducing the application would unfold its
  -- clauses at every occurrence for nothing.
  let field := mkAppN e.getAppFn (e.getAppArgs.extract 0 2)
  let reduced ← (try withDefault (whnf field) catch _ => pure field)
  let mut body := reduced
  for _ in [0:3] do
    let .lam _ _ next _ := body | return .continue
    body := next
  unless body.isConstOf ``True do return .continue
  return .done (mkConst ``True)

/-- Selecting an absent declaration in an installed Given frame owes no
own clause. Inspect only the declaration table at this handle and argument
row; a present or unknown declaration retains its predicate and encoding. -/
simproc [vir_denote, vir_denote_norm] givenOwnWithoutInvariant
    (@Carriers.own ?carriers _ _ _) := fun e => do
  let_expr Carriers.own family source arguments encoded := e | return .continue
  let_expr GivenFrame.toCarriers installed := family | return .continue
  let_expr GivenFrame.withDeclarations given declarations := installed | return .continue
  let selected := mkApp2 declarations source arguments
  let reduced ← (try withDefault (whnf selected) catch _ => pure selected)
  unless reduced.isAppOfArity ``Option.none 1 do return .continue
  let absent ← mkExpectedTypeHint (← mkEqRefl reduced) (← mkEq selected reduced)
  let proof := mkAppN (mkConst ``GivenFrame.withDeclarations_own_none)
    #[given, declarations, source, arguments, absent, encoded]
  return .done { expr := mkConst ``True, proof? := some (← mkAppM ``eq_true #[proof]) }

open Lean Meta in
/-- A variant declaration's data invariants of a value whose variants carry
no data invariants: `True` (`variantCarrier.holds_of_invariantFree`,
`NRows.invariantFree`). -/
simproc [vir_denote_norm] variantWithoutCertificates
    (@VerexIR.Proofs.Denote.variantCarrier.Holds ?carriers _ _ _) := fun e => do
  let e := e.consumeMData
  unless e.isAppOfArity ``VerexIR.Proofs.Denote.variantCarrier.Holds 4 do return .continue
  let parts := e.getAppArgs
  unless spelledNRows parts[2]! (some (← getEnv)) do return .continue
  let free := mkApp (mkConst ``VerexIR.Proofs.Denote.NRows.invariantFree) parts[2]!
  unless (← (try withDefault (whnf free) catch _ => pure free)).isConstOf ``Bool.true do
    return .continue
  let decided ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
    (← mkEq free (mkConst ``Bool.true))
  let holds := mkAppN
    (mkConst ``VerexIR.Proofs.Denote.variantCarrier.holds_of_invariantFree)
    #[parts[0]!, parts[1]!, parts[2]!, decided, parts[3]!]
  return .done { expr := mkConst ``True, proof? := some (← mkAppM ``eq_true #[holds]) }

simproc [vir_denote_norm] holdsWithoutCertificates
    (@VerexIR.Proofs.Denote.NTy.Holds ?carriers _ _) := fun e => do
  let e := e.consumeMData
  unless e.isAppOfArity ``VerexIR.Proofs.Denote.NTy.Holds 3 do return .continue
  let parts := e.getAppArgs
  unless spelledNTy parts[1]! (some (← getEnv)) do return .continue
  let free := mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.invariantFree) parts[1]!
  unless (← (try withDefault (whnf free) catch _ => pure free)).isConstOf ``Bool.true do
    return .continue
  let decided ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
    (← mkEq free (mkConst ``Bool.true))
  let holds := mkAppN
    (mkConst ``VerexIR.Proofs.Denote.NTy.holds_of_invariantFree)
    #[parts[0]!, parts[1]!, decided, parts[2]!]
  return .done { expr := mkConst ``True, proof? := some (← mkAppM ``eq_true #[holds]) }

attribute [vir_denote_norm] VerexIR.Proofs.Denote.NTy.invariantFree
  VerexIR.Proofs.Denote.NRow.invariantFree VerexIR.Proofs.Denote.NRows.invariantFree

attribute [vir_denote_norm] Sum.inl.injEq Sum.inr.injEq reduceCtorEq

attribute [vir_denote_norm] VerexIR.Proofs.Denote.variantName_inl
  VerexIR.Proofs.Denote.variantName_inr VerexIR.Proofs.Denote.ghostValueOf_same
  VerexIR.Proofs.Denote.ghostInitOf_same Weave.supplying_cons Weave.supplying_nil NRow.length
attribute [vir_denote_norm] VerexIR.Proofs.wp_choose VerexIR.Proofs.wp_assume
  VerexIR.Proofs.wp_certifyState VerexIR.Proofs.Denote.wp_prophecyOf
attribute [vir_denote_norm] VerexIR.Proofs.wp_certified
  VerexIR.Proofs.Denote.wp_checking VerexIR.Proofs.Denote.ResultShape.Holds
attribute [vir_denote_norm] VerexIR.Proofs.Denote.GhostInits.initialize_nil
  VerexIR.Proofs.Denote.GhostInits.initialize_cons
  VerexIR.Proofs.Denote.Lender.resolve_slot VerexIR.Proofs.Denote.Lender.resolve_place
  VerexIR.Proofs.Denote.Lender.resolve_candidate VerexIR.Proofs.Denote.Lender.resolve_referent
  VerexIR.Proofs.Denote.Lender.resolve_equation VerexIR.Proofs.Denote.Lender.resolve_seq
  VerexIR.Proofs.Denote.SliceLender.resolve_slot VerexIR.Proofs.Denote.SliceLender.resolve_array
  VerexIR.Proofs.Denote.SliceLender.resolve_vector VerexIR.Proofs.Denote.SliceLender.resolve_slice
  VerexIR.Proofs.Denote.SliceLender.resolve_candidateArray
  VerexIR.Proofs.Denote.SliceLender.resolve_candidateVector
  VerexIR.Proofs.Denote.SliceLender.resolve_candidateSlice
  VerexIR.Proofs.Denote.SliceLender.resolve_referent
  VerexIR.Proofs.Denote.SliceLender.resolve_equation
  VerexIR.Proofs.Denote.SliceLender.resolve_seq
  VerexIR.Proofs.Denote.checkingAt_unit VerexIR.Proofs.Denote.checkingAt_bool
  VerexIR.Proofs.Denote.checkingAt_int VerexIR.Proofs.Denote.checkingAt_num
  VerexIR.Proofs.Denote.checkingAt_address VerexIR.Proofs.Denote.checkingAt_signer
  VerexIR.Proofs.Denote.checkingAt_string VerexIR.Proofs.Denote.checkingAt_bytes
  VerexIR.Proofs.Denote.checkingAt_char VerexIR.Proofs.Denote.checkingAt_function
attribute [vir_denote_norm] VerexIR.Proofs.Denote.Proj.hasDeref
  VerexIR.Proofs.Denote.Proj.hasDeref_nil VerexIR.Proofs.Denote.Proj.hasDeref_deref
attribute [vir_denote_norm] VerexIR.Proofs.Denote.Skolems.reframe_struct_fst
  VerexIR.Proofs.Denote.Skolems.reframe_struct_snd

section ReframeAtBase
variable {unit : Validation.ValidatedUnit} {functions base : FunctionCarriers unit}
  (sameCarrier : functions.carrier = base.carrier)
  (sameFunctions : HEq functions.functions base.functions)
  (sameNominals : functions.nominals = base.nominals)
  (Θ : @Skolems unit functions) {params : NRow} (own : params.isParamsFrom 0 = true)
  (inhabitable : (params.map Θ.resolve).inhabitable = true)

@[vir_denote_norm] theorem Skolems.reframeAtBase_inj {τ : NTy}
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    (left right : @NTy.carrier Θ.toCarriers τ) :
    Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable bounded left =
      Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable bounded right ↔
      left = right := by
  constructor
  · intro same
    apply @NTy.encode_injective Θ.toCarriers τ
    rw [← Skolems.encode_reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable bounded left,
      ← Skolems.encode_reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable bounded right,
      same]
  · intro same; rw [same]

@[vir_denote_norm] theorem Skolems.equal_reframeAtBase {τ : NTy}
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    (left right : @NTy.carrier Θ.toCarriers τ) :
    @NTy.equal (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers false τ
        (Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable bounded left)
        (Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable bounded right) =
      @NTy.equal Θ.toCarriers false τ left right := by
  rw [@NTy.equal_codec (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers,
    @NTy.equal_codec Θ.toCarriers]
  simp only [NTy.codec_encode, Skolems.encode_reframeAtBase]
  rfl

@[vir_denote_norm] theorem Skolems.order_reframeAtBase {τ : NTy}
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    (rank : ValueRanks) (left right : @NTy.carrier Θ.toCarriers τ) :
    @NTy.order (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers rank τ
        (Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable bounded left)
        (Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable bounded right) =
      @NTy.order Θ.toCarriers rank τ left right := by
  rw [← @NTy.order_codec (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers,
    ← @NTy.order_codec Θ.toCarriers]
  simp only [NTy.codec_encode, Skolems.encode_reframeAtBase]

theorem Skolems.reframeAtBase_of_encode {τ : NTy} (bounded) (value : @NTy.carrier Θ.toCarriers τ)
    (carriers : @NTy.carrier Θ.toCarriers τ =
      @NTy.carrier (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers τ)
    (encoded : ∀ value,
      @NTy.encode (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers τ
        (cast carriers value) = @NTy.encode Θ.toCarriers τ value) :
    Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable bounded value =
      cast carriers value := by
  apply @NTy.encode_injective (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers τ
  rw [Skolems.encode_reframeAtBase, encoded]

@[vir_denote_norm] theorem Skolems.reframeAtBase_unit (bounded) (value : Unit) :
    Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable (τ := .unit) bounded value = value :=
  Skolems.reframeAtBase_of_encode sameCarrier sameFunctions sameNominals Θ own inhabitable bounded value rfl fun _ => rfl
@[vir_denote_norm] theorem Skolems.reframeAtBase_bool (bounded) (value : Bool) :
    Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable (τ := .bool) bounded value = value :=
  Skolems.reframeAtBase_of_encode sameCarrier sameFunctions sameNominals Θ own inhabitable bounded value rfl fun _ => rfl
@[vir_denote_norm] theorem Skolems.reframeAtBase_int {width : Nat} {signed : Bool} (bounded)
    (value : SpecInt (.bits width) signed) :
    Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable (τ := .int width signed) bounded value = value :=
  Skolems.reframeAtBase_of_encode sameCarrier sameFunctions sameNominals Θ own inhabitable bounded value rfl fun _ => rfl
@[vir_denote_norm] theorem Skolems.reframeAtBase_num (bounded) (value : SpecInt .unbounded true) :
    Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable (τ := .num) bounded value = value :=
  Skolems.reframeAtBase_of_encode sameCarrier sameFunctions sameNominals Θ own inhabitable bounded value rfl fun _ => rfl
@[vir_denote_norm] theorem Skolems.reframeAtBase_address (bounded) (value : String) :
    Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable (τ := .address) bounded value = value :=
  Skolems.reframeAtBase_of_encode sameCarrier sameFunctions sameNominals Θ own inhabitable bounded value rfl fun _ => rfl
@[vir_denote_norm] theorem Skolems.reframeAtBase_signer (bounded) (value : String) :
    Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable (τ := .signer) bounded value = value :=
  Skolems.reframeAtBase_of_encode sameCarrier sameFunctions sameNominals Θ own inhabitable bounded value rfl fun _ => rfl
@[vir_denote_norm] theorem Skolems.reframeAtBase_string (bounded) (value : String) :
    Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable (τ := .string) bounded value = value :=
  Skolems.reframeAtBase_of_encode sameCarrier sameFunctions sameNominals Θ own inhabitable bounded value rfl fun _ => rfl
@[vir_denote_norm] theorem Skolems.reframeAtBase_bytes (bounded) (value : Array UInt8) :
    Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable (τ := .bytes) bounded value = value :=
  Skolems.reframeAtBase_of_encode sameCarrier sameFunctions sameNominals Θ own inhabitable bounded value rfl fun _ => rfl
@[vir_denote_norm] theorem Skolems.reframeAtBase_char (bounded) (value : SpecChar) :
    Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable (τ := .char) bounded value = value :=
  Skolems.reframeAtBase_of_encode sameCarrier sameFunctions sameNominals Θ own inhabitable bounded value rfl fun _ => rfl

@[vir_denote_norm] theorem Skolems.reframeAtBase_struct_fst {source : StructHandle}
    {arguments : NRow} {τ : NTy} {rest : NRow} (bounded)
    (value : @NTy.carrier Θ.toCarriers (.struct source arguments (.cons τ rest))) :
    (Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable bounded value).1 =
      Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable
        (paramsHead (paramsFields bounded)) value.1 := by
  apply @NTy.encode_injective (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers τ
  rw [(encodeFields _ _ (Skolems.encode_reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable bounded value)).1,
    Skolems.encode_reframeAtBase]

@[vir_denote_norm] theorem Skolems.reframeAtBase_struct_snd {source : StructHandle}
    {arguments : NRow} {τ : NTy} {rest : NRow} (bounded)
    (value : @NTy.carrier Θ.toCarriers (.struct source arguments (.cons τ rest))) :
    (Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable bounded value).2 =
      Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable
        (τ := .struct source arguments rest)
        (paramsStruct (Bool.and_eq_true _ _ |>.mp bounded).1 (paramsTail (paramsFields bounded))) value.2 := by
  apply @NTy.encode_injective (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers
    (.struct source arguments rest)
  rw [(encodeFields _ _ (Skolems.encode_reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable bounded value)).2,
    Skolems.encode_reframeAtBase]

simproc [vir_denote_norm] reframeStructFst (Prod.fst _) := fun e => do
  let e := e.consumeMData
  unless e.isAppOfArity ``Prod.fst 3 do return .continue
  let value := (e.getArg! 2).consumeMData
  let proof? : Option Lean.Expr :=
    if value.isAppOfArity ``VerexIR.Proofs.Denote.Skolems.reframeAtBase 13 then
      let args := value.getAppArgs
      let σ := args[10]!.consumeMData
      if σ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.struct 3 &&
          (σ.getArg! 2).consumeMData.isAppOfArity ``VerexIR.Proofs.Denote.NRow.cons 2 then
        let fields := (σ.getArg! 2).consumeMData
        some (mkAppN (mkConst ``VerexIR.Proofs.Denote.Skolems.reframeAtBase_struct_fst)
          #[args[0]!, args[1]!, args[2]!, args[3]!, args[4]!, args[5]!, args[6]!, args[7]!,
            args[8]!, args[9]!, σ.getArg! 0, σ.getArg! 1, fields.getArg! 0, fields.getArg! 1,
            args[11]!, args[12]!])
      else none
    else none
  let some rawProof := proof? | return .continue
  let some (rhs, proof) ← (try withNewMCtxDepth <| withDefault do
      let some (_, _, rhs) := (← inferType rawProof).eq? | return none
      let proof ← mkExpectedTypeHint rawProof (← mkEq e rhs)
      return some (rhs, proof)
    catch _ => pure none) | return .continue
  return .done { expr := rhs, proof? := some proof }

@[vir_denote_norm] theorem Skolems.reframeAtBase_tuple_fst {τ : NTy} {rest : NRow} (bounded)
    (value : @NTy.carrier Θ.toCarriers (.tuple (.cons τ rest))) :
    (Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable
        (τ := .tuple (.cons τ rest)) bounded value).1 =
      Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable
        (τ := τ) (paramsHead bounded) value.1 := by
  apply @NTy.encode_injective (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers τ
  rw [(encodeElements _ _ (Skolems.encode_reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable bounded value)).1,
    Skolems.encode_reframeAtBase]

@[vir_denote_norm] theorem Skolems.reframeAtBase_tuple_snd {τ : NTy} {rest : NRow} (bounded)
    (value : @NTy.carrier Θ.toCarriers (.tuple (.cons τ rest))) :
    (Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable
        (τ := .tuple (.cons τ rest)) bounded value).2 =
      Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable
        (τ := .tuple rest) (paramsTail bounded) value.2 := by
  apply @NTy.encode_injective (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers (.tuple rest)
  rw [(encodeElements _ _ (Skolems.encode_reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable bounded value)).2,
    Skolems.encode_reframeAtBase]

private theorem pVariant {f : Nat → Bool} {source : StructHandle}
    {arguments fields : NRow} {name : String} {names : List String} {rest : NRows}
    {distinct : (name :: names).Nodup}
    (bounded : (NTy.enum source arguments (name :: names) (.cons fields rest) distinct).paramsAll f = true) :
    fields.paramsAll f = true := by
  simp only [NTy.paramsAll, NRows.paramsAll, Bool.and_eq_true] at bounded
  exact bounded.2.1

@[vir_denote_norm] theorem Skolems.reframeAtBase_enum_inl {source : StructHandle}
    {arguments fields : NRow} {name : String} {names : List String} {rest : NRows}
    {distinct : (name :: names).Nodup} (bounded)
    (value : @HList Θ.toCarriers fields) :
    Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable
        (τ := .enum source arguments (name :: names) (.cons fields rest) distinct) bounded (.inl value) =
      .inl (Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable
        (τ := .tuple fields) (pVariant bounded) value) := by
  apply @NTy.encode_injective (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers
    (.enum source arguments (name :: names) (.cons fields rest) distinct)
  have row := Skolems.encode_reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable
    (τ := .tuple fields) (pVariant bounded) value
  simp only [NTy.encode_tuple, RuntimeValue.tuple.injEq] at row
  simp only [Skolems.encode_reframeAtBase, NTy.encode_enum_inl, row]

@[vir_denote_norm] theorem Skolems.reframeAtBase_enum_inr {source : StructHandle}
    {arguments fields : NRow} {name : String} {names : List String} {rest : NRows}
    {distinct : (name :: names).Nodup} (bounded)
    (value : @variantCarrier Θ.toCarriers names rest) :
    Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable
        (τ := .enum source arguments (name :: names) (.cons fields rest) distinct) bounded (.inr value) =
      .inr (Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable
        (τ := .enum source arguments names rest (List.nodup_cons.mp distinct).2) (paramsVariants bounded) value) := by
  apply @NTy.encode_injective (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers
    (.enum source arguments (name :: names) (.cons fields rest) distinct)
  simp only [Skolems.encode_reframeAtBase, NTy.encode_enum_inr]

@[vir_denote_norm] theorem Skolems.reframeAtBase_vector_values {element : NTy} (bounded)
    (vector : @NTy.carrier Θ.toCarriers (.vector element)) :
    (Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable
        (τ := .vector element) bounded vector).values =
      vector.values.map (Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable
        (τ := element) (by simpa only [NTy.paramsAll] using bounded)) := by
  let Φ := (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers
  have encoded := Skolems.encode_reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable bounded vector
  rw [@NTy.encode_vector Φ, @NTy.encode_vector Θ.toCarriers] at encoded
  simp only [RuntimeValue.vector.injEq] at encoded
  apply Array.map_inj_right (f := @NTy.encode Φ element)
    (fun _ _ same => @NTy.encode_injective Φ element _ _ same) |>.mp
  rw [Array.map_map]
  refine encoded.trans ?_
  apply Array.map_congr_left
  intro value _
  exact (Skolems.encode_reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable _ value).symm

@[vir_denote_norm] theorem Skolems.reframeAtBase_ghost {slot : NTy} (bounded)
    (value : @NTy.carrier Θ.toCarriers (.ghost slot)) :
    Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable
        (τ := .ghost slot) bounded value =
      Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable
        (τ := slot) (by simpa only [NTy.paramsAll] using bounded) value := by
  let Φ := (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers
  apply @NTy.encode_injective Φ (.ghost slot)
  rw [Skolems.encode_reframeAtBase, @NTy.encode_ghost Φ,
    Skolems.encode_reframeAtBase, @NTy.encode_ghost Θ.toCarriers]

private theorem pTableKey {allowed : Nat → Bool} {key value : NTy}
    (bounded : (NTy.table key value).paramsAll allowed = true) : key.paramsAll allowed = true := by
  simp only [NTy.paramsAll, Bool.and_eq_true] at bounded; exact bounded.1

private theorem pTableValue {allowed : Nat → Bool} {key value : NTy}
    (bounded : (NTy.table key value).paramsAll allowed = true) : value.paramsAll allowed = true := by
  simp only [NTy.paramsAll, Bool.and_eq_true] at bounded; exact bounded.2

/-- A table's entries move with it. -/
theorem Skolems.reframeAtBase_table_entries {key value : NTy} (bounded)
    (table : @NTy.carrier Θ.toCarriers (.table key value)) :
    (Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable
        (τ := .table key value) bounded table).entries =
      table.entries.map fun entry =>
        (Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable
           (pTableKey bounded) entry.1,
         Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable
           (pTableValue bounded) entry.2) := by
  let Φ := (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers
  have encoded := Skolems.encode_reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable bounded table
  rw [@NTy.encode_table Φ, @NTy.encode_table Θ.toCarriers] at encoded
  simp only [RuntimeValue.vector.injEq] at encoded
  have encoded := List.toArray_inj encoded
  rw [show (table.entries.map fun entry =>
        RuntimeValue.tuple #[@NTy.encode Θ.toCarriers key entry.1, @NTy.encode Θ.toCarriers value entry.2]) =
      (table.entries.map fun entry =>
        (Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable (pTableKey bounded) entry.1,
         Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable (pTableValue bounded) entry.2)).map
        (fun entry => RuntimeValue.tuple #[@NTy.encode Φ key entry.1, @NTy.encode Φ value entry.2]) by
      simp only [List.map_map, Function.comp_def, Φ, Skolems.encode_reframeAtBase]] at encoded
  have injective : ∀ a b : @NTy.carrier Φ key × @NTy.carrier Φ value,
      RuntimeValue.tuple #[@NTy.encode Φ key a.1, @NTy.encode Φ value a.2] =
      RuntimeValue.tuple #[@NTy.encode Φ key b.1, @NTy.encode Φ value b.2] → a = b := by
    intro a b same
    simp only [RuntimeValue.tuple.injEq, Array.mk.injEq, List.cons.injEq, and_true] at same
    exact Prod.ext (@NTy.encode_injective Φ key _ _ same.1) (@NTy.encode_injective Φ value _ _ same.2)
  exact List.map_inj_right (fun a b same => injective a b same) |>.mp encoded

/-- The validity of a sequence map, under injective reads of its keys, is that
of the keys: it does not see their move to the base frame of the type arguments. -/
theorem Skolems.valid_sequence_reframeAtBase {key value : NTy} (bounded)
    (table : @NTy.carrier Θ.toCarriers (.table key value))
    {K V K' V' : Type}
    (readKey : @NTy.carrier (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers key → K)
    (readValue : @NTy.carrier (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers value → V)
    (readKey' : @NTy.carrier Θ.toCarriers key → K')
    (readValue' : @NTy.carrier Θ.toCarriers value → V')
    (injective : Function.Injective readKey) (injective' : Function.Injective readKey') :
    SpecMap.Valid .sequence
        (SpecMap.ofEntries readKey readValue
          (Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable
            (τ := .table key value) bounded table)) ↔
      SpecMap.Valid .sequence (SpecMap.ofEntries readKey' readValue' table) := by
  simp only [SpecMap.Valid, SpecMap.Distinct, SpecMap.ofEntries, Skolems.reframeAtBase_table_entries,
    List.map_map, List.Nodup, List.pairwise_map, Function.comp_def]
  have reads : ∀ a b,
      readKey (Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable (pTableKey bounded) a) ≠
      readKey (Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable (pTableKey bounded) b) ↔
      readKey' a ≠ readKey' b := by
    intro a b
    refine not_congr ⟨fun same => ?_, fun same => by rw [injective' same]⟩
    rw [(Skolems.reframeAtBase_inj sameCarrier sameFunctions sameNominals Θ own inhabitable _ a b).mp (injective same)]
  simp only [reads]

end ReframeAtBase

open Lean Meta in
/-- A value of a type without parameters or function values, whose carrier and
encoding do not read the frame, stays itself (`Skolems.reframeAtBase_of_encode`). -/
simproc ↓ [vir_denote, vir_denote_norm] reframeAtBaseClosed
    (@Skolems.reframeAtBase _ _ _ _ _ _ _ _ _ _ _ _ _) := fun e => do
  let #[unit, functions, base, sameCarrier, sameFunctions, sameNominals,
      frame, params, own, inhabitable, τ, bounded, value] := e.getAppArgs
    | return .continue
  withTransparency .default do
    unless (← whnf (mkApp (mkConst ``NTy.paramFree) τ)).isConstOf ``true do return .continue
    let source ← inferType value
    unless ← isDefEq source (← inferType e) do return .continue
    let carriers ← mkEqRefl source
    let identity := mkAppN (mkConst ``Skolems.reframeAtBase_of_encode)
      #[unit, functions, base, sameCarrier, sameFunctions, sameNominals,
        frame, params, own, inhabitable, τ, bounded, value, carriers]
    let .forallE _ encodedType _ _ ← whnf (← inferType identity) | return .continue
    let some encoded ← forallBoundedTelescope encodedType (some 1) fun bound statement => do
        let_expr Eq _ left right := statement | return none
        unless ← isDefEq left right do return none
        return some (← mkLambdaFVars bound (← mkEqRefl right))
      | return .continue
    let proof ← mkExpectedTypeHint (mkApp identity encoded) (← mkEq e value)
    return .visit { expr := value, proof? := some proof }

attribute [vir_denote_norm] VerexIR.Proofs.Denote.Weave.compose
-- A value a function states, before the equation is taken apart.
attribute [vir_denote_norm ↓] exists_apply_eq_apply

/-- A conjunction stated true, as a lemma's Boolean equation gives it where
its other side is decided: each conjunct true. -/
@[vir_denote_norm] theorem true_eq_and (a b : Bool) : (true = (a && b)) = (a = true ∧ b = true) := by
  cases a <;> cases b <;> decide

attribute [vir_denote_norm] decide_false decide_true Bool.decide_eq_true Bool.decide_eq_false
-- A branch on a condition neither arm reads, as `omega` splits it.
attribute [vir_denote_norm] dite_eq_ite
/-- The sequence of integers literal elements hold, if each element is
one. -/
private def literalIntegers? (elements : Lean.Expr) (value? : Lean.Expr → Option Lean.Expr) :
    MetaM (Option Lean.Expr) := do
  let some (_, elements) := elements.listLit? | return none
  let some values := elements.mapM value? | return none
  return some (← mkListLit (mkConst ``Int) values)

/-- `[a, …][k]?.getD 0`, the read of a literal sequence of integers. -/
private def literalIntegerRead (values index : Lean.Expr) : MetaM Lean.Expr := do
  mkAppM ``Option.getD #[← mkAppM ``GetElem?.getElem? #[values, index], mkIntLit 0]

/-- The program's read of a literal sequence of certified integers at an
arbitrary position (`[⟨a, _⟩, …][k].val`), as the read of the sequence of
their values, `[a, …][k]?.getD 0`. -/
simproc [vir_denote_norm] literalCertifiedRead (VerexIR.SpecInt.val _) := fun e => do
  if e.hasLooseBVars || !e.isAppOfArity ``VerexIR.SpecInt.val 3 then return .continue
  let element := e.getArg! 2
  unless element.isAppOfArity ``GetElem.getElem 8 do return .continue
  let some values ← literalIntegers? (element.getArg! 5) fun element =>
      if element.isAppOfArity ``VerexIR.SpecInt.mk 4 then some (element.getArg! 2) else none
    | return .continue
  let index := element.getArg! 6
  let rhs ← literalIntegerRead values index
  -- The read's bound may state the length as a literal.
  let proof := mkAppN (mkConst ``val_getElem_eq_getD_map_val)
    #[e.getArg! 0, e.getArg! 1, element.getArg! 5, index, element.getArg! 7]
  return .done { expr := rhs, proof? := some (← mkExpectedTypeHint proof (← mkEq e rhs)) }

/-- Optional and certified reads of the same integer literals must share a
normal form. The fallback is kept as the read states it, whatever it is,
including at out-of-bounds positions. -/
simproc [vir_denote_norm] literalOptionalCertifiedRead (VerexIR.SpecInt.val _) := fun e => do
  if e.hasLooseBVars || !e.isAppOfArity ``VerexIR.SpecInt.val 3 then return .continue
  let element := e.getArg! 2
  unless element.isAppOfArity ``Option.getD 3 do return .continue
  let lookup := element.getArg! 1
  unless lookup.isAppOfArity ``GetElem?.getElem? 7 do return .continue
  let some values ← literalIntegers? (lookup.getArg! 5) fun element =>
      if element.isAppOfArity ``VerexIR.SpecInt.mk 4 then some (element.getArg! 2) else none
    | return .continue
  let fallback := element.getArg! 2
  let rhs ← mkAppM ``Option.getD #[
    ← mkAppM ``GetElem?.getElem? #[values, lookup.getArg! 6], ← mkAppM ``VerexIR.SpecInt.val #[fallback]]
  let proof ← mkAppM ``val_getD_getElem?_map_val #[lookup.getArg! 5, lookup.getArg! 6, fallback]
  return .done { expr := rhs, proof? := some (← mkExpectedTypeHint proof (← mkEq e rhs)) }

attribute [vir_denote_norm] Option.bind_fun_some

/-- A lookup followed by a total step resolves nothing exactly where the lookup
does: a place the check before it resolves is refuted by that check's fact. -/
theorem option_bind_some_eq_none_iff {α β : Type} (o : Option α) (f : α → β) :
    (o.bind fun a => some (f a)) = none ↔ o = none := by
  cases o <;> simp

attribute [vir_denote_norm] option_bind_some_eq_none_iff

/-- A quantifier over a range with literal bounds is its instances, one per
position, when the range is short (at most 64 positions): the first position
is split off until the range is empty, each step decided by the kernel. -/
simproc [vir_denote_norm] expandLiteralRange (∀ _ : Int, _ ≤ _ → _ < _ → _) :=
  fun e => do
    let .forallE binder (.const ``Int []) body _ := e | return .continue
    let .forallE _ lowerBound rest _ := body | return .continue
    let .forallE _ upperBound predicate _ := rest | return .continue
    -- The premises' proofs are unused by the clause.
    if predicate.hasLooseBVar 0 || predicate.hasLooseBVar 1 then return .continue
    let_expr LE.le _ _ low lowVariable := lowerBound | return .continue
    let_expr LT.lt _ _ highVariable high := upperBound | return .continue
    unless lowVariable == .bvar 0 && highVariable == .bvar 1 do return .continue
    let some lowValue := low.int? | return .continue
    let some highValue := high.int? | return .continue
    unless lowValue < highValue && highValue ≤ lowValue + 64 do return .continue
    let intType : Lean.Expr := .const ``Int []
    -- The clause with the range's variable free at `#0`, and as a predicate.
    let clause := predicate.lowerLooseBVars 2 2
    let clauseAt (position : Lean.Expr) : Lean.Expr := clause.instantiate1 position
    let asPredicate : Lean.Expr := .lam binder intType clause .default
    -- From the last position back: `(∀ i, k ≤ i → i < high → P i) ↔ conjunction`.
    let mut conjunction : Lean.Expr := mkConst ``True
    let mut proof ← mkAppOptM ``VerexIR.Proofs.Denote.forall_int_range_empty
      #[asPredicate, high, high, ← mkDecideProof (← mkAppM ``LE.le #[high, high])]
    let mut position := highValue
    while position > lowValue do
      position := position - 1
      let here : Lean.Expr := toExpr position
      let short ← mkAppM ``LE.le #[high, ← mkAppM ``HAdd.hAdd #[here, (toExpr (64 : Int) : Lean.Expr)]]
      let split ← mkAppOptM ``VerexIR.Proofs.Denote.forall_int_range_split
        #[asPredicate, here, high, ← mkDecideProof (← mkAppM ``LT.lt #[here, high]),
          ← mkDecideProof short]
      let restCongruent ← mkAppM ``and_congr #[← mkAppM ``Iff.refl #[clauseAt here], proof]
      proof ← mkAppM ``Iff.trans #[split, restCongruent]
      conjunction := mkAnd (clauseAt here) conjunction
    return .visit { expr := conjunction, proof? := some (← mkAppM ``propext #[proof]) }
-- Typed memory: a slot read after a write, a frame's resource types at the
-- runtime frame, and the values it carries as they are.
attribute [vir_denote, vir_denote_norm] VerexIR.Proofs.Denote.Memory.set_same
  VerexIR.Proofs.Denote.ite_some_some VerexIR.Proofs.Denote.isSome_ite
  Bool.ite_eq_true_distrib if_false_left VerexIR.Proofs.Denote.ite_true_or
-- A bitwise operation of a value with itself or with zero.
attribute [vir_denote_norm] VerexIR.Proofs.IntegerArithmetic.bitwiseAnd_self
  VerexIR.Proofs.IntegerArithmetic.bitwiseOr_self Nat.and_self Nat.or_self Nat.xor_self
  VerexIR.Proofs.IntegerArithmetic.bitwiseAnd_zero VerexIR.Proofs.IntegerArithmetic.zero_bitwiseAnd
  VerexIR.Proofs.IntegerArithmetic.bitwiseOr_zero VerexIR.Proofs.IntegerArithmetic.zero_bitwiseOr
  VerexIR.Proofs.IntegerArithmetic.bitwiseXor_zero VerexIR.Proofs.IntegerArithmetic.zero_bitwiseXor
  Nat.and_zero Nat.zero_and Nat.or_zero Nat.zero_or Nat.xor_zero Nat.zero_xor
-- A specification's bitwise operation on nonnegative operands, as the runtime computes it.
attribute [vir_denote_norm] VerexIR.Proofs.IntegerArithmetic.bitwiseOr_nonnegative
  VerexIR.Proofs.IntegerArithmetic.bitwiseXor_nonnegative
-- An empty vector, by its size.
attribute [vir_denote_norm] Array.isEmpty_iff_size_eq_zero
  VerexIR.Proofs.Denote.array_isEmpty_eq_false
-- The data invariant a stored value's resource type selects.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.NTy.handle?_struct
  VerexIR.Proofs.Denote.NTy.handle?_enum VerexIR.Proofs.Denote.NTy.handle?_resolve_struct
  VerexIR.Proofs.Denote.NTy.handle?_resolve_enum
  VerexIR.Proofs.Denote.Memory.set_other VerexIR.Proofs.Denote.Memory.set_other_resource
  VerexIR.Proofs.Denote.Skolems.writePlain_ne_key
  VerexIR.Proofs.Denote.Skolems.plainAt_writePlain_ne_key
  VerexIR.Proofs.Denote.Skolems.writePlain_other_key
  VerexIR.Proofs.Denote.Skolems.writePlain_other_resource
  VerexIR.Proofs.Denote.Skolems.resource VerexIR.Proofs.Denote.NRow.map
  VerexIR.Proofs.Denote.ResourceType.mk.injEq
  VerexIR.Proofs.Denote.NTy.struct.injEq VerexIR.Proofs.Denote.NTy.enum.injEq
  VerexIR.Proofs.Denote.NRow.cons.injEq VerexIR.StructHandle.mk.injEq
  VerexIR.Proofs.Denote.NTy.int.injEq VerexIR.Proofs.Denote.NTy.vector.injEq
  VerexIR.StorageKey.address.injEq VerexIR.Proofs.Denote.NTy.storageKey_address
  VerexIR.Proofs.Denote.NTy.storageKey_signer VerexIR.Proofs.Denote.NTy.storageKey_int
  VerexIR.Proofs.Denote.NTy.storageKey_bool VerexIR.Proofs.Denote.NTy.storageKey_string
-- A literal instantiation keys a family as the runtime does.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.instantiatedTypeId_cons

-- The function value naming a target spells its weave as a closure literal does.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.Weave.supplying_nil
  VerexIR.Proofs.Denote.Weave.supplying_cons
  VerexIR.Proofs.Denote.instantiatedTypeId_nil VerexIR.TypeId.mk.injEq
-- A type parameter's value defaults under an induced family to its
-- argument's value.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.Carriers.default_instantiate

theorem NTy.holds_instantiate_heq {outer : Carriers} (θ : TypeArgs) (τ : NTy)
    (value : @NTy.carrier (Carriers.instantiate θ outer) τ)
    (value' : @NTy.carrier outer (τ.subst θ.1))
    (h : HEq value value') :
    @NTy.Holds (Carriers.instantiate θ outer) τ value =
      @NTy.Holds outer (τ.subst θ.1) value' := by
  rw [← @NTy.holds_ofSkolem_iff outer θ τ value]
  congr 1
  exact eq_of_heq ((@NTy.ofSkolem_heq outer θ τ value).trans h)

simproc ↓ [vir_denote, vir_denote_norm] ntyHoldsOuterFamily
    (@VerexIR.Proofs.Denote.NTy.Holds ?family _ _) := fun e => do
  let e := e.consumeMData
  unless e.isAppOfArity ``VerexIR.Proofs.Denote.NTy.Holds 3 do return .continue
  if e.hasLooseBVars then return .continue
  let (family, τ, value) := (e.getArg! 0, e.getArg! 1, e.getArg! 2)
  let some (θ, outer) := inducedCarriers? family.consumeMData | return .continue
  let some row := substitutionRow? θ | return .continue
  let some substituted := substituteNTy row τ (some (← getEnv))
    | return .continue
  let some (rhs, equation) ← (try withTransparency .default do
      let expectedCarrier := mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.carrier) outer substituted
      unless ← isDefEq (← inferType value) expectedCarrier do return none
      let rhs := mkApp3 (mkConst ``VerexIR.Proofs.Denote.NTy.Holds) outer substituted value
      let heq ← mkHEqRefl value
      let proof ← mkAppOptM ``VerexIR.Proofs.Denote.NTy.holds_instantiate_heq
        #[some outer, some θ, some τ, some value, some value, some heq]
      pure (some (rhs, ← mkExpectedTypeHint proof (← mkEq e rhs)))
    catch _ => pure none)
    | return .continue
  return .visit { expr := rhs, proof? := some equation }

/-- A check at the carriers a call's type arguments induce is the check at
the outer carriers, of the type substituted and of the same value: the fact
it owes and the fact what follows it sees are one proposition
(`NTy.holds_instantiate_heq`). -/
theorem checkingAt_instantiate_heq {State : Type} {outer : Carriers} (site : Nat)
    (kind : CertifyKind) (θ : TypeArgs) (τ : NTy)
    (value : @NTy.carrier (Carriers.instantiate θ outer) τ)
    (value' : @NTy.carrier outer (τ.subst θ.1)) (h : HEq value value') :
    @checkingAt (Carriers.instantiate θ outer) State site kind τ value =
      @checkingAt outer State site kind (τ.subst θ.1) value' := by
  have same := NTy.holds_instantiate_heq θ τ value value' h
  unfold checkingAt checking Interpretation.checking
  dsimp only
  rw [same]

/-- The check a term is, at the carriers a call's type arguments induce, as
the check at the outer carriers of the type substituted and of the same value
(`checkingAt_instantiate_heq`): the check there and the equation of the two.
None where the term is no such check, the substitution is not evaluated, or
the value's carrier is not the substituted type's.

The attempt is a transaction: the carrier comparison and the construction of
the equation run on a saved state, which is restored wherever the attempt
yields none, also where a step of it throws, so a declined attempt leaves no
assignment. An interruption or an exhausted budget is no decline: it is
thrown on as it is. -/
private def checkAtOuterFamily? (e : Lean.Expr) : MetaM (Option (Lean.Expr × Lean.Expr)) := do
  let e := e.consumeMData
  unless e.isAppOfArity ``VerexIR.Proofs.Denote.checkingAt 6 do return none
  if e.hasLooseBVars then return none
  let arguments := e.getAppArgs
  let (family, state, site, kind, τ, value) :=
    (arguments[0]!, arguments[1]!, arguments[2]!, arguments[3]!, arguments[4]!, arguments[5]!)
  let some (θ, outer) := inducedCarriers? family.consumeMData | return none
  let some row := substitutionRow? θ | return none
  let some substituted := substituteNTy row τ (some (← getEnv)) | return none
  let saved ← saveState
  try
    let found ← withTransparency .default do
      let expectedCarrier := mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.carrier) outer substituted
      unless ← isDefEq (← inferType value) expectedCarrier do return none
      let rhs := mkAppN (mkConst ``VerexIR.Proofs.Denote.checkingAt)
        #[outer, state, site, kind, substituted, value]
      let heq ← mkHEqRefl value
      let proof ← mkAppOptM ``VerexIR.Proofs.Denote.checkingAt_instantiate_heq
        #[some state, some outer, some site, some kind, some θ, some τ, some value, some value,
          some heq]
      pure (some (rhs, ← mkExpectedTypeHint proof (← mkEq e rhs)))
    if found.isNone then saved.restore
    return found
  catch failure =>
    saved.restore
    if failure.isRuntime || failure.isInterrupt then throw failure
    return none

/-- A check of a generic callee's body that the walk inlines on the generic
route (`closedGeneric`: a callee whose conditions are not carried at a row)
is emitted at the carriers the call's type arguments induce. It is stated
here at the outer carriers, where the body's steps are exposed, so the step
that cuts at it makes the fact it owes and the fact its continuation takes
at the one spelling the caller's context states its facts in
(`checkAtOuterFamily?`). -/
simproc ↓ [vir_denote, vir_denote_norm] checkingAtOuterFamily
    (@VerexIR.Proofs.Denote.checkingAt ?family _ _ _ _ _) := fun e => do
  let some (rhs, equation) ← checkAtOuterFamily? e | return .continue
  return .visit { expr := rhs, proof? := some equation }

-- The type arguments of an opaque specification function, at an
-- instantiated family the caller's, at the public family themselves.
-- Before `resolve_instantiate`, which a parameter's type also matches.
attribute [vir_denote_norm high] VerexIR.Proofs.Denote.Skolems.type_instantiate
attribute [vir_denote_norm] VerexIR.Proofs.Denote.Skolems.type_runtime VerexIR.Proofs.Denote.NTy.substWith
  VerexIR.Proofs.Denote.NRow.substWith VerexIR.Proofs.Denote.NRows.substWith
-- A resource type at an induced frame is its substitution, evaluated, so
-- that a callee's slot and the caller's are one term. A parameter's entry is
-- read with its mark dropped, which evaluates with it (`reduceUnmark`).
attribute [vir_denote, vir_denote_norm] VerexIR.Proofs.Denote.NTy.subst
  VerexIR.Proofs.Denote.NRow.subst VerexIR.Proofs.Denote.NRows.subst
  VerexIR.Proofs.Denote.NRow.getD
attribute [vir_denote_norm high] VerexIR.Proofs.Denote.NTy.substWith_type_runtime
-- A callee's value in the caller's view reduces constructor by constructor.
-- A transported entry list of scalar keys and values is the list itself.
attribute [vir_denote_norm] Prod.eta VerexIR.SpecVector.eta List.map_id' List.map_id

-- A vector's mapped element conversion is still unapplied when the
-- normalizer visits it. Its identity must simplify before vector equality
-- can substitute the caller's value into the search facts.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.NTy.ofSkolem_unit_fun
  VerexIR.Proofs.Denote.NTy.ofSkolem_bool_fun VerexIR.Proofs.Denote.NTy.ofSkolem_int_fun
  VerexIR.Proofs.Denote.NTy.ofSkolem_num_fun VerexIR.Proofs.Denote.NTy.ofSkolem_address_fun
  VerexIR.Proofs.Denote.NTy.ofSkolem_signer_fun VerexIR.Proofs.Denote.NTy.ofSkolem_string_fun
  VerexIR.Proofs.Denote.NTy.ofSkolem_bytes_fun VerexIR.Proofs.Denote.NTy.ofSkolem_param_fun

attribute [vir_denote_norm] VerexIR.Proofs.Denote.NTy.ofSkolem_unit
  VerexIR.Proofs.Denote.NTy.ofSkolem_bool VerexIR.Proofs.Denote.NTy.ofSkolem_int
  VerexIR.Proofs.Denote.NTy.ofSkolem_address VerexIR.Proofs.Denote.NTy.ofSkolem_signer
  VerexIR.Proofs.Denote.NTy.ofSkolem_string VerexIR.Proofs.Denote.NTy.ofSkolem_bytes
  VerexIR.Proofs.Denote.NTy.ofSkolem_param VerexIR.Proofs.Denote.NTy.ofSkolem_tuple
  VerexIR.Proofs.Denote.NTy.ofSkolem_struct VerexIR.Proofs.Denote.NTy.ofSkolem_enum
  VerexIR.Proofs.Denote.NTy.ofSkolem_vector VerexIR.Proofs.Denote.NTy.ofSkolem_ref
  VerexIR.Proofs.Denote.NTy.ofSkolem_ghost
  VerexIR.Proofs.Denote.HList.ofSkolem_nil VerexIR.Proofs.Denote.HList.ofSkolem_cons
  VerexIR.Proofs.Denote.variantCarrier.ofSkolem_inl
  VerexIR.Proofs.Denote.variantCarrier.ofSkolem_inr
  VerexIR.Proofs.Denote.ResultShape.ofSkolem_none VerexIR.Proofs.Denote.ResultShape.ofSkolem_one
-- A function calling itself, or a member of a cycle of calls: the calls
-- routed to the cycle are `self`, every other call its callee's meaning.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.routeMeaning_self
  VerexIR.Proofs.Denote.routeMeaning_other VerexIR.Proofs.Denote.routeGeneric_self
  VerexIR.Proofs.Denote.routeGeneric_other VerexIR.FunctionHandle.mk.injEq
  VerexIR.NamespaceId.mk.injEq VerexIR.FunctionId.mk.injEq

/-- Handles differ where their literal indices do: a check the kernel
evaluates, in place of rewriting the handles' equality. -/
theorem handle_ne_of_indices {n m f g : Nat} (differ : (Nat.beq n m && Nat.beq f g) = false) :
    (VerexIR.FunctionHandle.mk ⟨n⟩ ⟨f⟩) ≠ (VerexIR.FunctionHandle.mk ⟨m⟩ ⟨g⟩) := by
  intro same
  simp only [VerexIR.FunctionHandle.mk.injEq, VerexIR.NamespaceId.mk.injEq,
    VerexIR.FunctionId.mk.injEq] at same
  simp [same.1, same.2, Nat.beq_refl] at differ

/-- The indices of a function handle, where they are literals. -/
private def handleIndices? (handle : Lean.Expr) :
    MetaM (Option (Lean.Expr × Lean.Expr × Nat × Nat)) := do
  let handle := handle.consumeMData
  unless handle.isAppOfArity ``VerexIR.FunctionHandle.mk 2 do return none
  let spaceId := (handle.getArg! 0).consumeMData
  let functionId := (handle.getArg! 1).consumeMData
  unless spaceId.isAppOfArity ``VerexIR.NamespaceId.mk 1 &&
      functionId.isAppOfArity ``VerexIR.FunctionId.mk 1 do return none
  let some space ← getNatValue? spaceId.appArg! | return none
  let some function ← getNatValue? functionId.appArg! | return none
  return some (spaceId.appArg!, functionId.appArg!, space, function)

/-- A proof that `callee` is not the handle of literal indices `entry`. -/
private def handlesDiffer (callee entry : Lean.Expr × Lean.Expr × Nat × Nat) : Lean.Expr :=
  let (n, f, _, _) := callee
  let (m, g, _, _) := entry
  mkApp5 (mkConst ``handle_ne_of_indices) n m f g
    (mkApp2 (mkConst ``Eq.refl [levelOne]) (mkConst ``Bool) (mkConst ``Bool.false))

/-- Resolve a call along its route chain in one step: every entry of another
literal handle is passed by `other`, a lemma application with a check the
kernel evaluates, and the entry of the call's own signature is applied by
`self`. `entries` is the number of the chain's arguments up to its `rest`,
`callArguments` the number of the call's. The chain's arguments are not
visited, so the cost of a call is its position in the chain. -/
private def routeCall (route other self : Name) (entries callArguments : Nat)
    (selfArguments : Array Lean.Expr → Array Lean.Expr → Array Lean.Expr)
    (otherArguments : Array Lean.Expr → Array Lean.Expr → Lean.Expr → Array Lean.Expr)
    (e : Lean.Expr) : SimpM Simp.Step := do
  let arity := 3 + entries + callArguments
  unless e.isAppOfArity route arity do return .continue
  let levels := e.getAppFn.constLevels!
  let start := e.getAppArgs
  let call := start.extract (3 + entries) arity
  let some callee ← handleIndices? call[0]! | return .continue
  let σs := call[callArguments - 3]!
  let shape := call[callArguments - 2]!
  let type ← inferType e
  let level ← getLevel type
  let trans (first second : Lean.Expr) (left middle right : Lean.Expr) : Lean.Expr :=
    mkApp6 (mkConst ``Eq.trans [level]) type left middle right first second
  let mut current := e
  let mut proof? : Option Lean.Expr := none
  repeat
    unless current.isAppOfArity route arity do break
    let arguments := current.getAppArgs
    let entry := arguments.extract 3 (3 + entries)
    let some literal ← handleIndices? entry[0]! | break
    let prefixArguments := arguments.extract 0 3
    if literal.2.2.1 == callee.2.2.1 && literal.2.2.2 == callee.2.2.2 then
      -- The call's own entry, at its own signature.
      unless ← withReducible (isDefEq σs entry[1]! <&&> isDefEq shape entry[2]!) do break
      let step := mkAppN (mkConst self levels) (prefixArguments ++ selfArguments entry call)
      let some (_, _, result) := (← inferType step).eq? | break
      let proof := match proof? with
        | some earlier => trans earlier step e current result
        | none => step
      return .visit { expr := result, proof? := some proof }
    let rest := entry.back!
    let next := mkAppN rest call
    let step := mkAppN (mkConst other levels) (prefixArguments ++
      otherArguments entry call (handlesDiffer callee literal))
    proof? := some <| match proof? with
      | some earlier => trans earlier step e current next
      | none => step
    current := next
  match proof? with
  | some proof => return .visit { expr := current, proof? := some proof }
  | none => return .continue

/-- `routeMeaning handle π ρ self rest callee σs shape args`. -/
simproc ↓ [vir_denote_norm] resolveCalleeRoute
    (@VerexIR.Proofs.Denote.routeMeaning _ ?functions ?skolems _ _ _ _ _ _ _ _ _) :=
  routeCall ``VerexIR.Proofs.Denote.routeMeaning ``VerexIR.Proofs.Denote.routeMeaning_other
    ``VerexIR.Proofs.Denote.routeMeaning_self 5 4
    (fun entry call => entry ++ #[call[3]!])
    (fun entry call differ => #[entry[0]!, call[0]!, entry[1]!, call[1]!, entry[2]!, call[2]!,
      entry[3]!, entry[4]!, call[3]!, differ])

/-- `routeGeneric handle π ρ self frame rest callee typeArgs θ σs shape args`. -/
simproc ↓ [vir_denote_norm] resolveGenericRoute
    (@VerexIR.Proofs.Denote.routeGeneric _ ?functions ?skolems _ _ _ _ _ _ _ _ _ _ _ _) :=
  routeCall ``VerexIR.Proofs.Denote.routeGeneric ``VerexIR.Proofs.Denote.routeGeneric_other
    ``VerexIR.Proofs.Denote.routeGeneric_self 6 6
    (fun entry call => entry ++ #[call[1]!, call[2]!, call[5]!])
    (fun entry call differ => #[entry[0]!, call[0]!, entry[1]!, call[3]!, entry[2]!, call[4]!,
      entry[3]!, entry[4]!, entry[5]!, call[1]!, call[2]!, call[5]!, differ])

/-- The certified locals the context defines (`x.val = e`), with their
definitions. -/
private def namedValues : MetaM (Array (Lean.LocalDecl × Lean.Expr × Lean.Expr)) := do
  (← getLCtx).foldlM (init := #[]) fun found decl => do
    if decl.isImplementationDetail then return found
    let some (_, lhs, rhs) := (← instantiateMVars decl.type).eq? | return found
    unless lhs.isAppOfArity ``VerexIR.SpecInt.val 3 && lhs.appArg!.isFVar do return found
    if rhs.containsFVar lhs.appArg!.fvarId! then return found
    return found.push (decl, lhs.appArg!, rhs)

/-- A term with every named value it reads replaced by its definition, read
the same way: as the clause that cut it states the value. -/
private partial def expandNamed (named : Array (Lean.LocalDecl × Lean.Expr × Lean.Expr))
    (fuel : Nat) (e : Lean.Expr) : Lean.Expr :=
  if fuel == 0 then e else
  e.replace fun sub =>
    if sub.isAppOfArity ``VerexIR.SpecInt.val 3 then
      (named.find? (·.2.1 == sub.appArg!)).map fun (_, _, d) => expandNamed named (fuel - 1) d
    else none

/-- A certified local of `type` the context defines as `value` (`x.val = e`),
directly or through the locals `e` names, with the proof of its definition. -/
private def namedLocal? (type value : Lean.Expr) : MetaM (Option (Lean.Expr × Lean.Expr)) := do
  if value.hasLooseBVars then return none
  let named ← namedValues
  let expandedValue := expandNamed named named.size value
  for (decl, certified, rhs) in named do
    unless ← isDefEq (← inferType certified) type do continue
    if rhs == value then return some (certified, decl.toExpr)
    unless expandNamed named named.size rhs == expandedValue do continue
    -- The definition and the value, each read through the other names'
    -- definitions, are one term.
    let mut theorems : SimpTheorems := {}
    for (other, _, _) in named do
      unless other.fvarId == decl.fvarId do
        theorems ← theorems.add (.fvar other.fvarId) #[] other.toExpr
    let ctx ← Simp.mkContext (simpTheorems := #[theorems])
    let methods := Simp.mkDefaultMethodsCore #[]
    let (fromDefinition, _) ← Simp.main rhs ctx (methods := methods)
    let (fromValue, _) ← Simp.main value ctx (methods := methods)
    unless fromDefinition.expr == fromValue.expr do continue
    return some (certified, ← mkEqTrans (← mkEqTrans decl.toExpr (← fromDefinition.getProof))
      (← mkEqSymm (← fromValue.getProof)))
  return none

/-- A specification integer cut to a width (`SpecInt.ofInt`) of a value a
certified local has by the context: that local. -/
simproc ↓ [vir_denote_norm] cutNamedValue (VerexIR.SpecInt.ofInt _ _) := fun e => do
  let_expr VerexIR.SpecInt.ofInt _ _ default value := e | return .continue
  let some (certified, definition) ← namedLocal? (← inferType e) value | return .continue
  let proof ← mkAppM ``VerexIR.SpecInt.ofInt_eq_of_val #[default, certified, definition]
  return .done { expr := certified, proof? := some proof }

/-- A certified integer a clause defines by a value (`∃ x, x.val = e ∧ p x`)
a certified local has by the context: that local. -/
simproc ↓ [vir_denote_norm] existsNamedValue (∃ _ : VerexIR.SpecInt _ _, _) := fun e => do
  let_expr Exists domain body := e | return .continue
  let .lam name _ (.app (.app (.const ``And _) definition) rest) info := body | return .continue
  let some (_, lhs, value) := definition.eq? | return .continue
  unless lhs.isAppOfArity ``VerexIR.SpecInt.val 3 && lhs.appArg! == .bvar 0 do return .continue
  let some (certified, named) ← namedLocal? domain value | return .continue
  let proof ← mkAppOptM ``VerexIR.Proofs.Denote.SpecInt.exists_val_eq
    #[none, none, none, some (.lam name domain rest info), certified, named]
  return .visit { expr := rest.instantiate1 certified, proof? := some (← mkPropExt proof) }

/-- A certified integer defined by its value and read only by its value is the
value, its range a conjunct: no cut is left to read back. -/
theorem SpecInt.exists_val_read_unsigned (n : Nat) {value : Int} {p : Int → Prop} :
    (∃ c : VerexIR.SpecInt (.bits (n + 1)) false, c.val = value ∧ p c.val) ↔
      (0 ≤ value ∧ value ≤ 2 ^ (n + 1) - 1) ∧ p value :=
  ⟨fun ⟨c, defined, held⟩ =>
    ⟨(IntegerValueFits_unsigned_succ n value).mp (defined ▸ c.fits), defined ▸ held⟩,
    fun ⟨bounds, held⟩ =>
      ⟨⟨value, (IntegerValueFits_unsigned_succ n value).mpr bounds⟩, rfl, held⟩⟩

theorem SpecInt.exists_val_read_signed (n : Nat) {value : Int} {p : Int → Prop} :
    (∃ c : VerexIR.SpecInt (.bits (n + 1)) true, c.val = value ∧ p c.val) ↔
      (-(2 : Int) ^ n ≤ value ∧ value ≤ 2 ^ n - 1) ∧ p value :=
  ⟨fun ⟨c, defined, held⟩ =>
    ⟨(IntegerValueFits_signed_succ n value).mp (defined ▸ c.fits), defined ▸ held⟩,
    fun ⟨bounds, held⟩ =>
      ⟨⟨value, (IntegerValueFits_signed_succ n value).mpr bounds⟩, rfl, held⟩⟩

open Lean Meta Simp in
/-- One spelling of a defined value: `∃ c, c.val = v ∧ p c` at a fixed width is
the range of `v` and `p` at the value `v` cut to the width, wherever it occurs;
where `p` reads `c` only by its value, `p` at `v` itself. -/
simproc [vir_denote_norm] existsDefinedValue (∃ _ : VerexIR.SpecInt _ _, _) := fun e => do
  let_expr Exists domain body := e | return .continue
  let .lam name _ (.app (.app (.const ``And _) definition) rest) info := body | return .continue
  let some (_, lhs, value) := definition.eq? | return .continue
  unless lhs.isAppOfArity ``VerexIR.SpecInt.val 3 && lhs.appArg! == .bvar 0 do return .continue
  if value.hasLooseBVars then return .continue
  let_expr VerexIR.SpecInt width signed := domain | return .continue
  let_expr VerexIR.IntWidth.bits bits := width | return .continue
  let some bits := bits.nat? | return .continue
  let .succ n := bits | return .continue
  let signed? := signed.isConstOf ``Bool.true
  -- `c`'s reads by its value abstracted; `c` left is another read.
  let (theorem_, predicate) ← withLocalDeclD name domain fun c => do
    let held := rest.instantiate1 c
    withLocalDeclD name (mkConst ``Int) fun read => do
      let reading := held.replace fun sub =>
        if sub.isAppOfArity ``VerexIR.SpecInt.val 3 && sub.appArg! == c then some read else none
      if reading.containsFVar c.fvarId! then
        return (if signed? then ``SpecInt.exists_val_cut_signed else ``SpecInt.exists_val_cut_unsigned,
          Expr.lam name domain rest info)
      return (if signed? then ``SpecInt.exists_val_read_signed else ``SpecInt.exists_val_read_unsigned,
        ← mkLambdaFVars #[read] reading)
  let iff ← mkAppOptM theorem_ #[mkNatLit n, some value, some predicate]
  let some (_, rhs) := (← instantiateMVars (← inferType iff)).iff? | return .continue
  return .visit { expr := rhs.headBeta, proof? := some (← mkPropExt iff) }

/-- Whether an expression states a value a clause defines (`bindCuts`): a
certified integer by its value, or the check of a literal function value. -/
private def statesDefinedValue (e : Lean.Expr) : Bool :=
  (e.find? fun sub => (sub.isAppOfArity ``Exists 2 &&
    (sub.getArg! 0).isAppOfArity ``VerexIR.SpecInt 2) ||
      sub.isAppOfArity ``closureTypingCheck 7).isSome

open Lean Meta Simp in
/-- The check of a literal function value a leaf's clause binds, where the
literal is a closure site's: the site's generated check (`siteCheckProof?`). -/
simproc ↓ [vir_denote_norm] existsClosureChecked (@Exists _ _) := fun e => do
  let_expr Exists domain body := e | return .continue
  let some proof ← siteCheckProof? domain | return .continue
  let iff := mkApp3 (mkConst ``exists_prop_of_true) domain body proof
  return .visit { expr := body.beta #[proof], proof? := some (← mkPropExt iff) }

open Lean Meta Simp in
/-- A behavioral predicate of a closure site's constructor capturing values,
as of the function value naming its target at the woven arguments
(`ensuresOf_closureOf_woven`, `abortsOf_closureOf_woven`): the named value
a literal of the target capturing nothing, typed by the site's generated
`named` theorem. -/
private def wovenBehavior (rule : Name) (e : Lean.Expr) : SimpM Step := withDefault do
  unless e.getAppNumArgs > 7 do return .continue
  let function := (e.getArg! 7).consumeMData
  let .const constructor _ := function.getAppFn | return .continue
  let some entry := siteEquationsAt? (← getEnv) constructor | return .continue
  let some namedTyping := entry.named | return .continue
  let some literal ← literalValue? function | return .continue
  unless (literal.captured.consumeMData).isAppOfArity ``NRow.cons 2 do return .continue
  let typing ← unfoldLiterals (← inferType (mkConst namedTyping))
  let typed ← mkExpectedTypeHint (mkConst namedTyping) typing
  unless typing.isAppOfArity ``SiteTyped 10 do return .continue
  let namedArguments := (typing.getAppArgs.extract 0 9).push (mkConst ``NRow.nil)
    |>.push (mkConst ``Unit.unit) |>.push typed
  let named := mkAppN (mkConst ``Skolems.construct) namedArguments
  let naming := mkAppN (mkConst ``Skolems.closure_construct) namedArguments
  -- The reference-freedom premise `free` decided; no other premise is open.
  let premise (name : Name) (condition : Lean.Expr) : MetaM (Option Lean.Expr) := do
    if name == `free then decidedFrameFree? condition else return none
  let some (rhs, proof) ← rewriteByEquationCore? rule e premise
      #[(`closed, literal.closed), (`named, named), (`naming, naming)]
    | return .continue
  return .visit { expr := rhs, proof? := some proof }

open Lean Meta Simp in
/-- `ensures_of` of a literal function value capturing values, as of the
function value naming its target (`wovenBehavior`). -/
simproc [vir_denote_norm] wovenEnsuresOf
    (@VerexIR.Proofs.EnsuresOf _ ?functions ?frame _ _ _ _ _ _ _ _ _) :=
  wovenBehavior ``VerexIR.Proofs.ensuresOf_closureOf_woven

open Lean Meta Simp in
/-- `aborts_of` of a literal function value capturing values, as of the
function value naming its target (`wovenBehavior`). -/
simproc [vir_denote_norm] wovenAbortsOf
    (@VerexIR.Proofs.AbortsOf _ ?functions ?frame _ _ _ _ _ _ _) :=
  wovenBehavior ``VerexIR.Proofs.abortsOf_closureOf_woven

/-- The values a leaf's clauses define (`bindCuts`): an integer by its value
(`∃ x, x.val = e ∧ p x`) as a certified local of that value where the path
names one (`existsNamedValue`), otherwise as the value cut to the width with
its range a conjunct (`SpecInt.exists_val_cut_unsigned`); a literal function
value's check, a closure site's, by its generated check (`existsClosureChecked`). At the
leaf, after the path has named its locals; the normal form keeps a clause's
as stated. -/
elab "verex_defined_values" : tactic => focusGoal do
  let goal ← getMainGoal
  goal.withContext do
  let mut stating := #[]
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    if statesDefinedValue (← instantiateMVars decl.type) then stating := stating.push decl.fvarId
  let target := statesDefinedValue (← instantiateMVars (← goal.getType))
  if stating.isEmpty && !target then return
  let mut theorems : SimpTheorems := {}
  for name in [``SpecInt.exists_val, ``SpecInt.exists_unbounded, ``SpecInt.exists_val_unsigned,
      ``SpecInt.exists_val_signed, ``SpecInt.exists_val_unbounded] do
    theorems ← theorems.addConst name
  let simprocs ← ({} : Simprocs).add ``existsNamedValue (post := false)
  let simprocs ← simprocs.add ``existsDefinedValue (post := true)
  let simprocs ← simprocs.add ``existsClosureChecked (post := false)
  let ctx ← Simp.mkContext (config := { failIfUnchanged := false }) (simpTheorems := #[theorems])
    (congrTheorems := ← getSimpCongrTheorems)
  match ← simpGoal goal ctx #[simprocs] (simplifyTarget := target) (fvarIdsToSimp := stating) with
  | (none, _) => replaceMainGoal []
  | (some (_, next), _) => replaceMainGoal [next]

-- A certified integer cut to its own width is itself; a cut compared reads
-- by its value.
attribute [vir_denote_norm] VerexIR.SpecInt.ofInt_val VerexIR.SpecInt.ofInt_eq_iff
  VerexIR.SpecInt.eq_ofInt_iff
-- A vector read at a wider element type (`coerceNative`): its values mapped,
-- a literal's element by element.
attribute [vir_denote_norm] VerexIR.SpecVector.map List.map_toArray List.map_cons
  List.map_nil
-- The value of a bitwise operation at its operator.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.BitOp.eval_and VerexIR.Proofs.Denote.BitOp.eval_or
  VerexIR.Proofs.Denote.BitOp.eval_xor
-- The structural order, as the ordering it computes.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.NTy.compareResult_val VerexIR.orderValue_lt_zero
  VerexIR.orderValue_eq_zero VerexIR.zero_lt_orderValue VerexIR.orderValue_le_zero
  VerexIR.zero_le_orderValue VerexIR.orderValue_eq_neg_one VerexIR.orderValue_eq_one
  VerexIR.Proofs.Denote.NTy.order VerexIR.Proofs.Denote.HList.order
  VerexIR.Proofs.Denote.variantCarrier.order
  Int.compare_eq_lt Int.compare_eq_gt
  Int.compare_eq_eq List.toList_toArray VerexIR.SpecMap.intOrder_compare
-- A vector construction whose representability is decided, and the sizes
-- of in-bounds insertion and removal. Whether an insertion or removal is
-- in bounds is an arithmetic side condition, decided at a leaf by omega
-- (`verex_lookups_after_writes`), not by the normalization, whose
-- default discharger would run it over the whole set.
attribute [vir_denote_norm] Option.dite_none_right_eq_some
  Array.size_insertIdx Array.size_eraseIdx
theorem self_eq_extract_iff {α : Type} {as : Array α} {i j : Nat} :
    as = as.extract i j ↔ as.size = 0 ∨ i = 0 ∧ as.size ≤ j := by
  rw [eq_comm]; exact Array.extract_eq_self_iff

-- A slice, read by its size and elements; a vector's by its array's.
attribute [vir_denote_norm] VerexIR.SpecVector.values_extract
attribute [vir_denote_norm] Array.extract_size Array.size_extract Array.getElem?_extract
  Array.extract_eq_self_iff self_eq_extract_iff Array.extract_size_left
-- An operation on a mapped array, as the mapped result.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.setIfInBounds_map
  VerexIR.Proofs.Denote.extract_map VerexIR.Proofs.Denote.push_map
  VerexIR.Proofs.Denote.append_map
-- A range reversal, read by its size and elements.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.size_reverseSlice
  VerexIR.Proofs.Denote.getElem?_reverseSlice VerexIR.Proofs.Denote.reverseSlice_reverseSlice
attribute [vir_denote_norm] VerexIR.Proofs.Denote.findIndex?_eq_none_iff
  VerexIR.Proofs.Denote.findIndex?_isSome_iff VerexIR.Proofs.Denote.forall_imp_ne_iff
  VerexIR.Proofs.Denote.forall_imp_ne_iff' VerexIR.Proofs.Denote.exists_some_val_eq_iff
  VerexIR.Proofs.Denote.forall_some_val_ne_iff VerexIR.Proofs.Denote.findIndex?_zero
  VerexIR.Proofs.Denote.findIndex?_succ VerexIR.Proofs.Denote.NTy.eqb_int
  VerexIR.Proofs.Denote.NTy.eqb_bool VerexIR.Proofs.Denote.NTy.eqb_address
  VerexIR.Proofs.Denote.NTy.eqb_unit VerexIR.Proofs.Denote.NTy.eqb_param
-- A wrapping shift whose value cannot reach the modulus, and the truncating
-- remainder of a nonnegative shift, the runtime's.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.shiftLeft_emod_of_fits
  VerexIR.Proofs.Denote.shiftLeft_tmod_of_fits VerexIR.Proofs.Denote.shiftLeft_tmod_of_nonneg
/-- A proof of `0 ≤ e` through sums, products and quotients of terms the
normalization's discharger shows nonnegative. -/
private partial def nonnegative? (e : Lean.Expr) : Simp.SimpM (Option Lean.Expr) := do
  let parts (lemma : Name) (left right : Lean.Expr) : Simp.SimpM (Option Lean.Expr) := do
    let some first ← nonnegative? left | return none
    let some second ← nonnegative? right | return none
    return some (mkApp4 (mkConst lemma) left right first second)
  let whole : Simp.SimpM (Option Lean.Expr) := do
    (← Simp.getMethods).discharge? (mkIntLE (mkIntLit 0) e)
  let structural ← match_expr e with
    | HAdd.hAdd _ _ _ _ left right => parts ``Int.add_nonneg left right
    | HMul.hMul _ _ _ _ left right => parts ``Int.mul_nonneg left right
    | HDiv.hDiv _ _ _ _ left right => parts ``Int.ediv_nonneg left right
    | Nat.cast _ _ n => return some (mkApp (mkConst ``Int.natCast_nonneg) n)
    -- A certified unsigned value by its bounds, not by a search.
    | VerexIR.SpecInt.val width signed value =>
        if signed.isConstOf ``Bool.false && width.isAppOfArity ``VerexIR.IntWidth.bits 1 then
          return some (← mkAppM ``And.left #[← mkAppM ``VerexIR.SpecInt.unsigned_bounds #[value]])
        else whole
    | Int.shiftLeft value distance => do
        let some first ← nonnegative? value | whole
        return some (mkApp3 (mkConst ``VerexIR.Proofs.Denote.shiftLeft_nonneg) value distance first)
    | _ => whole
  if structural.isSome || !(e.isAppOfArity ``HAdd.hAdd 6 || e.isAppOfArity ``HMul.hMul 6 ||
      e.isAppOfArity ``HDiv.hDiv 6) then
    return structural
  -- A sum, product or quotient whose sign a fact states as a whole.
  whole

/-- One spelling of a quotient (`doc/designs/proof-language.md`, "One spelling of
a quotient"): a truncating quotient or remainder of a nonnegative dividend is
the Euclidean one, `/` and `%` of unbounded integers; `Int.tdiv` and
`Int.tmod` stay only where the dividend's sign is open. -/
simproc [vir_denote_norm] tdivOfNonnegative (Int.tdiv _ _) := fun e => do
  let_expr Int.tdiv dividend divisor := e | return .continue
  let some nonnegative ← nonnegative? dividend | return .continue
  let quotient := mkIntDiv dividend divisor
  let proof := mkApp3 (mkConst ``Int.tdiv_eq_ediv_of_nonneg) dividend divisor nonnegative
  return .visit { expr := quotient, proof? := some proof }

simproc [vir_denote_norm] tmodOfNonnegative (Int.tmod _ _) := fun e => do
  let_expr Int.tmod dividend divisor := e | return .continue
  let some nonnegative ← nonnegative? dividend | return .continue
  let remainder := mkIntMod dividend divisor
  let proof := mkApp3 (mkConst ``Int.tmod_eq_emod_of_nonneg) dividend divisor nonnegative
  return .visit { expr := remainder, proof? := some proof }

-- A concatenation's size, linear in its parts'.
attribute [vir_denote_norm] Array.size_append

-- The identities of zero and one, which a substitution or a case split
-- leaves (`r - 0`, `r * 0`): the arithmetic in its simplest spelling.
attribute [vir_denote_norm] Int.add_zero Int.zero_add Int.sub_zero Int.mul_zero Int.zero_mul
  Int.mul_one Int.one_mul Int.zero_ediv Int.ediv_zero Int.ediv_one Int.zero_emod Int.emod_one

/-! Membership (`doc/plan/proof-language.md`, "The deciders' own theory of
membership"): the leaf's deciders read `e ∈ v` by its positions
(`mem_iff_exists_int_index`); the step normal form keeps it an atom with its own algebra,
and reads a search for an element as that atom (`VerexSL.Verify`'s step simp context). -/

@[vir_denote_norm] theorem mem_swapIfInBounds {α : Type} (xs : Array α) (i j : Nat) (a : α) :
    a ∈ xs.swapIfInBounds i j ↔ a ∈ xs := by
  unfold Array.swapIfInBounds
  split
  · split
    · exact (Array.swap_perm ‹_› ‹_›).mem_iff
    · rfl
  · rfl

/-- A statement or its negation holds, whatever spelling it has. -/
@[vir_denote_norm] theorem or_not_self_iff (p : Prop) : (p ∨ ¬p) ↔ True :=
  iff_true_intro (Classical.em p)

@[vir_denote_norm] theorem not_or_self_iff (p : Prop) : (¬p ∨ p) ↔ True :=
  iff_true_intro ((Classical.em p).symm)

/-- An integer read at its own width is itself: the identity coercion of a
vector's elements, and of a needle, which membership reads. -/
@[vir_denote_norm] theorem SpecInt.mk_val_eq {width : IntWidth} {signed : Bool}
    (x : SpecInt width signed) (fits : IntegerValueFits width signed x.val) :
    (⟨x.val, fits⟩ : SpecInt width signed) = x := rfl

@[vir_denote_norm] theorem Array.map_mk_val {width : IntWidth} {signed : Bool}
    (xs : Array (SpecInt width signed)) (fits : ∀ x : SpecInt width signed,
      IntegerValueFits width signed x.val) :
    xs.map (fun x => (⟨x.val, fits x⟩ : SpecInt width signed)) = xs := by
  have : (fun x => (⟨x.val, fits x⟩ : SpecInt width signed)) = id := rfl
  rw [this, Array.map_id]

/-- A search for an element is membership, by its value for an integer, or over a
vector's positions whatever its reads fall back to: the spellings the step normal
form reads as the atom (`doc/plan/proof-language.md`, "The deciders' own theory of membership"). -/
theorem exists_mem_val_eq_iff {width : IntWidth} {signed : Bool}
    (xs : Array (SpecInt width signed)) (e : SpecInt width signed) :
    (∃ x, x ∈ xs ∧ x.val = e.val) ↔ e ∈ xs :=
  ⟨fun ⟨_, mem, same⟩ => SpecInt.ext same ▸ mem, fun mem => ⟨e, mem, rfl⟩⟩

theorem forall_mem_val_ne_iff {width : IntWidth} {signed : Bool}
    (xs : Array (SpecInt width signed)) (e : SpecInt width signed) :
    (∀ x, x ∈ xs → ¬x.val = e.val) ↔ ¬e ∈ xs :=
  ⟨fun all mem => all e mem rfl, fun absent _ mem same => absent (SpecInt.ext same ▸ mem)⟩

theorem exists_position_some_iff {α : Type} (xs : Array α) (e : α) :
    (∃ i : Int, 0 ≤ i ∧ i < xs.size ∧ xs[i.toNat]? = some e) ↔ e ∈ xs :=
  (mem_iff_exists_int_index xs e).symm

theorem forall_position_some_ne_iff {α : Type} (xs : Array α) (e : α) :
    (∀ i : Int, 0 ≤ i → i < xs.size → ¬xs[i.toNat]? = some e) ↔ ¬e ∈ xs := by
  rw [mem_iff_exists_int_index]
  exact ⟨fun all ⟨i, low, high, read⟩ => all i low high read,
    fun absent i low high read => absent ⟨i, low, high, read⟩⟩

theorem exists_position_eq_iff {α : Type} (xs : Array α) (d e : α) :
    (∃ i : Int, 0 ≤ i ∧ i < xs.size ∧ xs[i.toNat]?.getD d = e) ↔ e ∈ xs := by
  rw [mem_iff_exists_int_index]
  refine exists_congr fun i => ⟨fun ⟨low, high, read⟩ => ⟨low, high, ?_⟩,
    fun ⟨low, high, found⟩ => ⟨low, high, by rw [found, Option.getD_some]⟩⟩
  rw [Array.getElem?_eq_getElem (by omega : i.toNat < xs.size), Option.getD_some] at read
  rw [Array.getElem?_eq_getElem (by omega : i.toNat < xs.size), read]

theorem forall_position_ne_iff {α : Type} (xs : Array α) (d e : α) :
    (∀ i : Int, 0 ≤ i → i < xs.size → ¬xs[i.toNat]?.getD d = e) ↔ ¬e ∈ xs := by
  rw [← exists_position_eq_iff xs d e]
  exact ⟨fun all ⟨i, low, high, read⟩ => all i low high read,
    fun absent i low high read => absent ⟨i, low, high, read⟩⟩

attribute [vir_denote_norm] Array.mem_push Array.mem_append Array.mem_singleton List.mem_toArray
  Array.mem_map

open Lean Meta Simp in
/-- `∀ x, p x ∧ q x` as `(∀ x, p x) ∧ (∀ x, q x)`, `forall_and`, its binder's name
kept: a clause's binder stays in a proof's scope. -/
def forallAndKeepingName : Simproc := fun e => do
  let .forallE name domain body info := e | return .continue
  let body' := body.consumeMData
  unless body'.isAppOfArity ``And 2 do return .continue
  let (left, right) := (body'.getArg! 0, body'.getArg! 1)
  let iff ← mkAppOptM ``forall_and #[domain, some (.lam name domain left info), some (.lam name domain right info)]
  let result := mkAnd (.forallE name domain left info) (.forallE name domain right info)
  return .visit { expr := result, proof? := some (← mkPropExt iff) }

open Lean Meta Simp in
simproc [vir_denote_norm] forallAndDependent (∀ x : ?α, ?p x) := forallAndKeepingName

open Lean Meta Simp in
simproc [vir_denote_norm] forallAndArrow (_ → _) := forallAndKeepingName

open Lean Meta Simp in
/-- A literal position: the normal form evaluates it wherever a substitution leaves
one. -/
simproc [vir_denote_norm] literalToNat (Int.toNat _) := fun e => do
  let some value := e.appArg!.int? | return .continue
  return .done { expr := toExpr value.toNat }

/-- A step of a nest: a vector concatenated to what precedes it, or an element
appended to it. -/
inductive SpecVector.Step (α : Type) where
  | vector (appended : VerexIR.SpecVector α)
  | element (appended : α)

/-- The elements a step adds. -/
def SpecVector.Step.values {α : Type} : SpecVector.Step α → Array α
  | .vector appended => appended.values
  | .element appended => #[appended]

/-- A vector grown by a step. -/
def SpecVector.Step.grow {α : Type} (base : VerexIR.SpecVector α) :
    SpecVector.Step α → VerexIR.SpecVector α
  | .vector appended => base.append appended
  | .element appended => base.push appended

/-- The lengths of a nest from `start`, as integers, in the order of its steps:
`start + len(b) + 1 + …`. -/
def SpecVector.lengthsFrom {α : Type} (start : Int) (rest : List (SpecVector.Step α)) : Int :=
  rest.foldl (fun sum step => sum + (step.values.size : Int)) start

private theorem SpecVector.le_lengthsFrom {α : Type} (rest : List (SpecVector.Step α)) (start : Int) :
    start ≤ SpecVector.lengthsFrom start rest := by
  induction rest generalizing start with
  | nil => exact Int.le_refl _
  | cons step rest induction =>
      have := induction (start + (step.values.size : Int))
      simp only [SpecVector.lengthsFrom, List.foldl_cons] at this ⊢
      omega

/-- A nest of concatenations and appends is one concatenation under one premise
on its steps' lengths: the depth-one lemmas, step by step. -/
theorem SpecVector.values_nest {α : Type} (rest : List (SpecVector.Step α))
    (first : VerexIR.SpecVector α)
    (bounded : SpecVector.lengthsFrom (first.values.size : Int) rest ≤ 18446744073709551615) :
    (rest.foldl SpecVector.Step.grow first).values =
      rest.foldl (fun values step => values ++ step.values) first.values := by
  induction rest generalizing first with
  | nil => rfl
  | cons step rest induction =>
      have below : (first.values.size : Int) + (step.values.size : Int) ≤
          SpecVector.lengthsFrom (first.values.size : Int) (step :: rest) :=
        SpecVector.le_lengthsFrom rest _
      have joined : (SpecVector.Step.grow first step).values = first.values ++ step.values := by
        cases step with
        | vector appended =>
            exact VerexIR.SpecVector.values_append (by
              simp only [SpecVector.Step.values] at below
              omega)
        | element appended =>
            have one : (SpecVector.Step.values (.element appended)).size = 1 := rfl
            simp only [SpecVector.Step.grow, SpecVector.Step.values]
            rw [VerexIR.SpecVector.values_push appended (by omega), Array.append_singleton]
      have size : (((SpecVector.Step.grow first step).values.size : Nat) : Int) =
          (first.values.size : Int) + (step.values.size : Int) := by
        rw [joined, Array.size_append]
        omega
      have grown := induction (SpecVector.Step.grow first step) (by
        rw [size]
        exact bounded)
      simp only [List.foldl_cons]
      rw [grown, joined]

open Lean Meta Simp in
/-- The values of a left nest of `SpecVector.append` and `SpecVector.push` are the
concatenation of its steps' under one premise, the sum of the steps' lengths as
integers (a literal vector's as a literal, an appended element's as one), in
their order, at most `MAX_U64`. A proof states it
(`len(a) + 1 + len(b) + … <= MAX_U64`), matched by spelling in the normal form;
`omega` proves it at a leaf. The nest is read whole or not at all: where the
premise is not discharged the term stays, as this rule cannot flatten the whole
nest without the total bound; other rules may still apply where their own
premises hold. A single append of an element is no nest and is left to its own rule
(`SpecVector.values_push`), whose premise is that append's room alone. -/
private def valuesOfNestSteps (e : Lean.Expr) : SimpM Simp.Step := do
  let_expr VerexIR.SpecVector.values α nest := e | return .continue
  let mut first := nest
  -- The steps in their order, each a vector (`true`) or an element.
  let mut rest : List (Bool × Lean.Expr) := []
  repeat
    if first.isAppOfArity ``VerexIR.SpecVector.append 3 then
      rest := (true, first.appArg!) :: rest
      first := first.getArg! 1
    else if first.isAppOfArity ``VerexIR.SpecVector.push 3 then
      rest := (false, first.appArg!) :: rest
      first := first.getArg! 1
    else break
  if rest.all (!·.1) && rest.length < 2 then return .continue
  let int := mkConst ``Int
  -- A literal vector's length is its literal, which `omega` reads.
  let length (vector : Lean.Expr) : MetaM Lean.Expr := do
    let vector := vector.consumeMData
    if vector.isAppOfArity ``VerexIR.SpecVector.mk 3 then
      let values := (vector.getArg! 1).consumeMData
      if values.isAppOfArity ``List.toArray 2 then
        if let some (_, elements) := values.appArg!.listLit? then
          return toExpr (Int.ofNat elements.length)
    mkAppOptM ``Nat.cast #[int, none, ← mkAppM ``Array.size #[← mkAppM ``VerexIR.SpecVector.values #[vector]]]
  let sum ← rest.foldlM (init := ← length first) fun sum (isVector, added) => do
    mkAppM ``HAdd.hAdd #[sum, ← if isVector then length added else pure (toExpr (1 : Int))]
  let steps ← mkListLit (mkApp (mkConst ``SpecVector.Step) α) (rest.map fun (isVector, added) =>
    mkApp2 (mkConst (if isVector then ``SpecVector.Step.vector else ``SpecVector.Step.element)) α added)
  let values ← rest.foldlM (init := ← mkAppM ``VerexIR.SpecVector.values #[first])
    fun values (isVector, added) => do
      mkAppM ``HAppend.hAppend #[values, ← if isVector then mkAppM ``VerexIR.SpecVector.values #[added]
        else mkArrayLit α [added]]
  let premise ← mkAppM ``LE.le #[sum, toExpr (18446744073709551615 : Int)]
  let some bounded ← (← getMethods).discharge? premise | return .continue
  let proof := mkApp4 (mkConst ``SpecVector.values_nest) α steps first bounded
  let statement ← mkEq e values
  return .visit { expr := values, proof? := some (← mkExpectedTypeHint proof statement) }

simproc [vir_denote_norm] valuesOfNest
    (VerexIR.SpecVector.values (VerexIR.SpecVector.append _ _)) := valuesOfNestSteps

simproc [vir_denote_norm] valuesOfPushNest
    (VerexIR.SpecVector.values (VerexIR.SpecVector.push _ _)) := valuesOfNestSteps

-- A resolved prophecy brings an operation's result into a leaf.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.wrapInt_unsigned
  VerexIR.Proofs.Denote.wrapInt_signed VerexIR.Proofs.Denote.ModularOp.run_val
  VerexIR.Proofs.Denote.BitOp.run_val
-- Rust's primitives: a flag as the exact result's range, a character's validity
-- as its code point's.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.ModularOp.overflows_unsigned_succ
  VerexIR.Proofs.Denote.ModularOp.overflows_signed_succ VerexIR.Proofs.Denote.ModularOp.eval_add
  VerexIR.Proofs.Denote.ModularOp.eval_subtract VerexIR.Proofs.Denote.ModularOp.eval_multiply
  VerexIR.Proofs.Denote.isUnicodeScalar_eq_true
/-- What `pop` leaves after `push`: the vector pushed to. -/
theorem SpecVector_extract_push_size {α : Type} (vector : VerexIR.SpecVector α) (element : α)
    (length : Nat) (room : vector.values.size + 1 < 2 ^ 64) (whole : length = vector.values.size) :
    (vector.push element).extract 0 length = vector := by
  subst whole
  apply VerexIR.SpecVector.ext
  simp only [VerexIR.SpecVector.values_extract, VerexIR.SpecVector.values_push element room]
  apply Array.ext
  · simp
  · intro position _ inner
    simp only [Array.getElem_extract, Nat.zero_add]
    exact Array.getElem_push_lt inner

attribute [vir_denote_norm] SpecVector_extract_push_size

theorem spec_assume_true {σ ε : Type} :
    (VerexIR.Proofs.Spec.assume True : VerexIR.Proofs.Spec σ ε Unit) =
      VerexIR.Proofs.Spec.pure () := by
  apply VerexIR.Proofs.Spec.extensionality
  · funext initial result final
    exact propext ⟨fun ⟨_, hr, hf⟩ => ⟨hr, hf⟩, fun ⟨hr, hf⟩ => ⟨trivial, hr, hf⟩⟩
  · funext _ _; rfl
  · funext _; rfl

attribute [vir_denote, vir_denote_norm] spec_assume_true

attribute [vir_denote_norm] VerexIR.Proofs.wp_pure
  VerexIR.Proofs.wp_abort VerexIR.Proofs.wp_given VerexIR.Proofs.Spec.pure_bind
  VerexIR.Proofs.Spec.abort_bind VerexIR.Proofs.Denote.wp_abortAt VerexIR.Proofs.Denote.wp_opAt
  VerexIR.Proofs.Denote.wp_opAfterAt VerexIR.Proofs.Denote.AfterAt_true
  VerexIR.Proofs.Denote.opAt_pure VerexIR.Proofs.Denote.opAt_abort
  VerexIR.Proofs.Denote.wp_branchAt VerexIR.Proofs.Denote.branchIn_some
  VerexIR.Proofs.Denote.branchIn_none VerexIR.Proofs.Denote.AbortsAt_true VerexIR.Proofs.Denote.Taken_true
  VerexIR.Proofs.Denote.wp_armAt VerexIR.Proofs.Denote.armIn_some VerexIR.Proofs.Denote.armIn_none
  VerexIR.Proofs.Denote.wp_dispatchAt VerexIR.Proofs.Denote.Dispatched_true
  VerexIR.Proofs.Denote.Matched_true
  VerexIR.Proofs.Denote.abortAt_bind VerexIR.Proofs.Denote.Flow.bind_abortAt
  VerexIR.Proofs.Denote.ResultShape.bodyType Bool.not_eq_true decide_eq_true_eq Bool.and_eq_true
  Bool.or_eq_true and_assoc exists_and_left exists_and_right exists_eq_left exists_eq_left'
  exists_true_left
  and_true true_and and_imp forall_eq forall_eq' Prod.mk.injEq exists_eq exists_eq'
  and_false false_and not_false_eq_true not_true_eq_false true_implies false_implies implies_true
  true_or or_true false_or or_false
  imp_self eq_self_iff_true ite_true ite_false Bool.false_eq_true Bool.true_eq_false
  Bool.not_eq_false decide_eq_false_iff_not Bool.and_eq_false_imp Bool.not_true Bool.not_false
  Bool.not_not Bool.not_eq_false' Bool.not_eq_true' Option.some.injEq Option.elim_some Option.elim_none
  List.getElem?_toArray List.getElem?_cons_zero
  List.getElem?_cons_succ Option.getD_some
  List.cons.injEq List.nil_eq
  beq_self_eq_true VerexIR.Proofs.Denote.Array.push_eq_push_iff
  Nat.lt_succ_self Nat.lt_add_one Nat.le_refl
  Option.map_some Option.map_none
  VerexIR.Proofs.Denote.nat_eq_add_succ_iff
  VerexIR.Proofs.Denote.nat_add_succ_eq_iff VerexIR.Proofs.Denote.nat_eq_succ_iff
  VerexIR.Proofs.Denote.nat_succ_eq_iff
  Option.bind_some Option.bind_none
  Option.getD_none
  VerexIR.SemanticOperations.instantiatedTypeId_empty
  Nat.add_assoc VerexIR.Proofs.Denote.nat_self_eq_add_iff
  VerexIR.Proofs.Denote.nat_add_eq_self_iff
  VerexIR.SpecVector.ofArray?_eq VerexIR.SpecVector.values_set VerexIR.SpecVector.values_mk
  Array.toList_map
  VerexIR.Proofs.Denote.exists_mem_map_iff Array.mem_toList_iff
  exists_eq_right exists_eq_right' VerexIR.Proofs.Denote.exists_range_eq_iff
  VerexIR.Proofs.Denote.mem_iff_exists_int_index
  VerexIR.Proofs.Denote.not_exists_range_iff VerexIR.Proofs.Denote.not_forall_range_iff
  VerexIR.Proofs.Denote.forall_exists_range_iff
  Option.isSome_eq_false_iff Option.isNone_iff_eq_none true_iff false_iff iff_true iff_false
  VerexIR.Proofs.Denote.not_eq_none_iff_isSome
  Int.toNat_natCast Array.toArray_toList List.toList_toArray Array.length_toList Array.size_set!
  Array.size_push Array.size_map VerexIR.Proofs.Denote.toArray_inj_iff
  VerexIR.SpecVector.values_push Array.append_singleton
  -- A field update a clause states (`nativeFieldUpdate`).
  VerexIR.Proofs.Denote.Var.update_here VerexIR.Proofs.Denote.Var.update_there
  List.map_toArray List.map_cons List.map_nil List.push_toArray
  List.size_toArray List.length_cons List.length_nil List.getElem?_eq_some_iff
  List.getElem?_eq_none_iff Array.getElem?_eq_none_iff Array.size_setIfInBounds Array.getElem?_map
  -- The transport of a value at the identity instantiation.
  Option.map_id' Array.map_id' Int.sub_add_cancel
  Option.map_map Function.comp_def Array.set!_eq_setIfInBounds List.setIfInBounds_toArray
  List.set_cons_zero List.set_cons_succ List.insertIdxIfInBounds_toArray List.insertIdx_zero
  List.insertIdx_succ_cons List.eraseIdxIfInBounds_toArray List.eraseIdx_cons_zero
  List.eraseIdx_cons_succ List.toList_toArray Option.bind_eq_some_iff Option.map_eq_some_iff
  -- A literal vector updated at a position not known literally, and a
  -- literal vector transported elementwise.
  List.length_insertIdx List.getElem?_insertIdx List.length_eraseIdx List.getElem?_eraseIdx
  List.length_map List.getElem?_map List.getElem_cons_zero List.getElem_cons_succ
  List.getElem_toArray
  List.getElem?_nil List.getElem?_cons_zero List.getElem?_cons_succ
  Int.toNat_natCast VerexIR.Proofs.Denote.vectorLength_val
  VerexIR.Proofs.Denote.NTy.eqb_vector_decide
  VerexIR.Proofs.Denote.NTy.eqb_table_decide
  VerexIR.Proofs.Denote.NTy.refFree VerexIR.Proofs.Denote.NRow.refFree
  VerexIR.Proofs.Denote.NRows.refFree VerexIR.Proofs.Denote.NTy.ghostFree
  VerexIR.Proofs.Denote.NRow.ghostFree VerexIR.Proofs.Denote.NRows.ghostFree Option.isSome_some Option.isSome_none Option.isSome_map
  VerexIR.GlobalKey.mk.injEq VerexIR.StorageKey.address.injEq Option.map_eq_none_iff
  VerexIR.StructHandle.mk.injEq VerexIR.Proofs.Contract.typed
  List.toArray_eq_iff exists_prop exists_const Int.not_lt Int.not_le ne_eq Decidable.not_not
  Bool.true_or Bool.false_or Bool.or_true Bool.or_false

private partial def isLiteralList (e : Lean.Expr) : Bool :=
  let e := e.consumeMData
  if e.isAppOfArity ``List.nil 1 then true
  else if e.isAppOfArity ``List.cons 3 then isLiteralList (e.getArg! 2)
  else false

/-- A read from a constant list needs no enumeration of the index. -/
theorem getElem_replicated {α : Type u} (value : α) (n i : Nat) (h : i < n) :
    (List.replicate n value)[i]'(by simpa only [List.length_replicate] using h) = value := by
  apply List.getElem_replicate

/-- Literal vectors often contain equal values while their index comes from
a callee. Recognize the constant list and use its lookup lemma, without case
search over the index. Definitional equality also ignores range certificates
on otherwise equal bounded values. -/
simproc [vir_denote_norm] uniformListRead ((_ : List _)[_]'_) := fun e => do
  unless e.isAppOfArity ``GetElem.getElem 8 do return .continue
  let some (type, elements) := (e.getArg! 5).listLit? | return .continue
  let some value := elements.head? | return .continue
  if elements.any (·.hasExprMVar) then return .continue
  -- A skolem-family conversion may expose literal elements before the
  -- list's type is reduced. Leave those reads to the ordinary rewrites.
  unless ← withReducible (isDefEq type (← inferType value)) do return .continue
  unless ← elements.allM fun element => withReducible (isDefEq value element) do
    return .continue
  -- Keep the bound's existing type. The application builder does not
  -- unfold List.length when checking a bound against a numeric literal.
  let proof ← mkAppOptM ``getElem_replicated
    #[type, value, ← mkAppM ``List.length #[e.getArg! 5], e.getArg! 6, e.getArg! 7]
  return .visit { expr := value, proof? := some proof }

/-- Read a swapped literal vector before invoking the leaf solver. Expanding
reads of symbolic arrays introduces conditionals which obstruct their authored
array proofs, so those arrays retain the swap operation. The rewrite itself
uses the standard, kernel-checked array lookup theorem. -/
simproc [vir_denote_norm] literalSwapRead (((_ : Array _).swapIfInBounds _ _)[_]'_) := fun e => do
  unless e.isAppOfArity ``GetElem.getElem 8 do return .continue
  let swapped := e.getArg! 5
  unless swapped.isAppOfArity ``Array.swapIfInBounds 4 do return .continue
  let xs := (swapped.getArg! 1).consumeMData
  unless (xs.isAppOfArity ``List.toArray 2 || xs.isAppOfArity ``Array.mk 2) &&
      isLiteralList (xs.getArg! 1) do return .continue
  let rules ← mkSimpTheoremFromConst ``Array.getElem_swapIfInBounds
  if let some result ← Simp.tryTheorem? e rules[0]! then return .visit result
  return .continue


-- Variant names are literals, decided where they are compared.
attribute [vir_denote_norm] String.reduceBEq String.reduceEq String.reduceBNe String.reduceNe
-- A value transported into a callee's view and back is the value, also
-- element by element.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.NTy.ofSkolem_toSkolem
  VerexIR.Proofs.Denote.NTy.toSkolem_ofSkolem VerexIR.Proofs.Denote.HList.ofSkolem_toSkolem
  VerexIR.Proofs.Denote.HList.toSkolem_ofSkolem
  VerexIR.Proofs.Denote.variantCarrier.ofSkolem_toSkolem
  VerexIR.Proofs.Denote.variantCarrier.toSkolem_ofSkolem
  Array.map_map List.map_map
-- The order of booleans is their comparison, computed on literals.
@[vir_denote_norm] theorem compare_false_false : compare false false = Ordering.eq := rfl
@[vir_denote_norm] theorem compare_false_true : compare false true = Ordering.lt := rfl
@[vir_denote_norm] theorem compare_true_false : compare true false = Ordering.gt := rfl
@[vir_denote_norm] theorem compare_true_true : compare true true = Ordering.eq := rfl
-- A position a pinned integer gives is a literal, and so are the
-- comparisons of the lengths it computes; literal keys compare equal or
-- not, as a map's precondition over literal vectors states them.
attribute [vir_denote_norm] Int.reduceToNat Nat.reduceLeDiff Nat.reduceLT Int.reduceLT
  Int.reduceLE Int.reduceEq Int.reduceNe

/-- Fold literal shifts before collecting arithmetic bounds. Leaving `1 << k`
symbolic makes each use of that constant acquire separate quotient/remainder
facts. As with literal powers, respect Lean's evaluation threshold. Symbolic
operands keep their bit-vector representation and existing arithmetic lemmas. -/
private def shiftLiteralStep (operation : Int → Nat → Int) : Simp.DSimproc := fun e => do
  let args := e.getAppArgs
  let (some value, some distance) := (args[0]?.bind Lean.Expr.int?, args[1]?.bind Lean.Expr.nat?)
    | return .continue
  unless ← checkExponent distance (warning := false) do return .continue
  return .done (mkIntLit (operation value distance))

dsimproc_decl reduceIntShiftLeft (Int.shiftLeft _ _) := shiftLiteralStep Int.shiftLeft
dsimproc_decl reduceIntShiftRight (Int.shiftRight _ _) := shiftLiteralStep Int.shiftRight
attribute [vir_denote, vir_denote_norm] reduceIntShiftLeft reduceIntShiftRight

/-- A shift right of a multiple of the shift's power of two is the quotient's
multiple, exactly. -/
theorem shiftRight_mul_pow {multiplier quotient : Int} (value : Int) (distance : Nat)
    (isMultiple : multiplier = quotient * 2 ^ distance) :
    Int.shiftRight (multiplier * value) distance = quotient * value := by
  subst isMultiple
  rw [shiftRight_eq_div, Int.mul_right_comm,
    Int.mul_ediv_cancel _ (Int.ne_of_gt (Int.pow_pos (by decide)))]

/-- A shift right of the shift's power of two times a value is that value:
`(16 * x) >> 4 = x`. -/
theorem shiftRight_pow_mul {multiplier : Int} (value : Int) (distance : Nat)
    (isPower : multiplier = 2 ^ distance) : Int.shiftRight (multiplier * value) distance = value := by
  rw [shiftRight_mul_pow value distance (quotient := 1) (by rw [isPower, Int.one_mul]), Int.one_mul]

/-- A shift right of a literal multiple of the shift's power of two, as the
quotient's multiple: `(16 * x) >> 4` is `x`, syntactically, where a product
with `x` is then one atom to the deciders. The kernel decides the literal
equation. -/
simproc_decl reduceShiftRightMul (Int.shiftRight (HMul.hMul _ _) _) := fun e => do
  let args := e.getAppArgs
  let product := args[0]!
  unless product.isAppOfArity ``HMul.hMul 6 do return .continue
  let (some multiplier, some distance) := (product.getArg! 4 |>.int?, args[1]?.bind Lean.Expr.nat?)
    | return .continue
  unless ← checkExponent distance (warning := false) do return .continue
  let power : Int := 2 ^ distance
  unless multiplier % power == 0 do return .continue
  let quotient := multiplier / power
  let value := product.getArg! 5
  let powerExpr ← mkAppM ``HPow.hPow #[mkIntLit 2, mkNatLit distance]
  if quotient == 1 then
    let some isPower ← decidedFrameFree? (← mkEq (product.getArg! 4) powerExpr)
      | return .continue
    let proof ← mkAppM ``shiftRight_pow_mul #[value, mkNatLit distance, isPower]
    return .done { expr := value, proof? := some proof }
  let some isMultiple ← decidedFrameFree?
      (← mkEq (product.getArg! 4) (← mkAppM ``HMul.hMul #[mkIntLit quotient, powerExpr]))
    | return .continue
  let proof ← mkAppM ``shiftRight_mul_pow #[value, mkNatLit distance, isMultiple]
  return .done { expr := ← mkAppM ``HMul.hMul #[mkIntLit quotient, value], proof? := some proof }

attribute [vir_denote, vir_denote_norm] reduceShiftRightMul

/-- A bitwise operation on integer literals, evaluated: a value a program
computes from literals, as `3 & 7`, is a literal to the observations of
what holds it. The kernel decides the equation by evaluation. -/
private def bitwiseLiteralStep (operation : Int → Int → Int) : Lean.Meta.Simp.Simproc := fun e => do
  let args := e.getAppArgs
  let (some left, some right) := (args[0]?.bind Lean.Expr.int?, args[1]?.bind Lean.Expr.int?)
    | return .continue
  let result := mkIntLit (operation left right)
  let proof ← mkExpectedTypeHint (← mkEqRefl e) (← mkEq e result)
  return .done { expr := result, proof? := some proof }

simproc_decl reduceBitwiseAnd (VerexIR.Proofs.IntegerArithmetic.bitwiseAnd _ _) :=
  bitwiseLiteralStep VerexIR.Proofs.IntegerArithmetic.bitwiseAnd
simproc_decl reduceBitwiseOr (VerexIR.Proofs.IntegerArithmetic.bitwiseOr _ _) :=
  bitwiseLiteralStep VerexIR.Proofs.IntegerArithmetic.bitwiseOr
simproc_decl reduceBitwiseXor (VerexIR.Proofs.IntegerArithmetic.bitwiseXor _ _) :=
  bitwiseLiteralStep VerexIR.Proofs.IntegerArithmetic.bitwiseXor
/-- A lifted bitwise operation (`BitOp.eval`) on integer literals, evaluated
as its operation. -/
simproc_decl reduceBitOpEval (VerexIR.Proofs.Denote.BitOp.eval _ _ _) := fun e => do
  let args := e.getAppArgs
  let some op := args[0]? | return .continue
  let operation : Option VerexIR.Proofs.Denote.BitOp :=
    if op.isConstOf ``VerexIR.Proofs.Denote.BitOp.and then some .and
    else if op.isConstOf ``VerexIR.Proofs.Denote.BitOp.or then some .or
    else if op.isConstOf ``VerexIR.Proofs.Denote.BitOp.xor then some .xor
    else none
  let some operation := operation | return .continue
  let (some left, some right) := (args[1]?.bind Lean.Expr.int?, args[2]?.bind Lean.Expr.int?)
    | return .continue
  let result := mkIntLit (operation.eval left right)
  let proof ← mkExpectedTypeHint (← mkEqRefl e) (← mkEq e result)
  return .done { expr := result, proof? := some proof }
attribute [vir_denote_norm, vir_denote_eval] reduceBitwiseAnd reduceBitwiseOr reduceBitwiseXor
  reduceBitOpEval
-- Concatenated vector literals, as a specification builds a key with
-- `concat(vec(1), vec(2))`, are the literal of the concatenation.
attribute [vir_denote_norm] List.append_toArray List.cons_append List.nil_append

/-- An element appended to a vector built from an array is the vector of the
grown array, where the grown array is within a vector's length. -/
theorem SpecVector.push_mk {α : Type} (values : Array α) (bounded : values.size < 2 ^ 64)
    (element : α) (grown : Array α) (same : values.push element = grown)
    (within : grown.size < 2 ^ 64) :
    (VerexIR.SpecVector.mk values bounded).push element = ⟨grown, within⟩ := by
  subst same
  have room : values.size + 1 < 2 ^ 64 := by simpa only [Array.size_push] using within
  apply VerexIR.SpecVector.ext
  rw [VerexIR.SpecVector.values_push element room]

open Lean Meta Simp in
/-- An element appended to a literal vector is the literal with that element
last, as a clause's concatenation of two literals is the literal of their
elements. The vector is one built from a list literal, of whatever elements.
The grown length is decided on the literal before anything is built: a
literal with no room for the element stays, as the append leaves such a
vector as it is. -/
simproc [vir_denote_norm] pushOfLiteral
    (VerexIR.SpecVector.push (VerexIR.SpecVector.mk _ _) _) := fun e => do
  let_expr VerexIR.SpecVector.push α vector element := e | return .continue
  let_expr VerexIR.SpecVector.mk _ values bounded := vector | return .continue
  let some (_, elements) := values.arrayLit? | return .continue
  unless elements.length + 1 < 2 ^ 64 do return .continue
  let grown ← mkArrayLit α (elements ++ [element])
  let within ← VerexIR.Proofs.Denote.decidedLemma (← mkAppM ``LT.lt
    #[← mkAppM ``Array.size #[grown], mkNatLit (2 ^ 64)])
  let same ← mkExpectedTypeHint (← mkEqRefl grown)
    (← mkEq (← mkAppM ``Array.push #[values, element]) grown)
  let proof := mkAppN (mkConst ``SpecVector.push_mk)
    #[α, values, bounded, element, grown, same, within]
  return .visit { expr := mkApp3 (mkConst ``VerexIR.SpecVector.mk) α grown within, proof? := some proof }

/-- The order of two integer literals, as the structural order of literal
values compares their components. -/
simproc [vir_denote_eval] reduceIntCompare (@compare Int _ _ _) := fun e => do
  let_expr compare _ _ left right := e | return .continue
  let (some a, some b) := (left.int?, right.int?) | return .continue
  let result := match Ord.compare a b with
    | .lt => mkConst ``Ordering.lt
    | .eq => mkConst ``Ordering.eq
    | .gt => mkConst ``Ordering.gt
  let proof ← mkExpectedTypeHint (← mkEqRefl e) (← mkEq e result)
  return .done { expr := result, proof? := some proof }

/-- A literal certified at a width it fits, as a specification cuts an integer
into a fixed-width position (`SpecInt.ofInt`). -/
simproc [vir_denote_eval] reduceSpecIntOfInt (VerexIR.SpecInt.ofInt _ _) := fun e => do
  let_expr VerexIR.SpecInt.ofInt width signed default value := e | return .continue
  let some literal := value.int? | return .continue
  let some widthValue := (match_expr width.consumeMData with
      | VerexIR.IntWidth.bits bits => (bits.nat? <|> bits.rawNatLit?).map VerexIR.IntWidth.bits
      | VerexIR.IntWidth.unbounded => some VerexIR.IntWidth.unbounded
      | _ => none) | return .continue
  let some signedValue := (if signed.isConstOf ``Bool.true then some true
      else if signed.isConstOf ``Bool.false then some false else none) | return .continue
  unless VerexIR.Ty.integerValueFits? (.integer widthValue signedValue) literal == some true do
    return .continue
  let fits ← VerexIR.Proofs.Denote.decidedLemma
    (← mkAppM ``VerexIR.IntegerValueFits #[width, signed, value])
  let certified := mkAppN (mkConst ``VerexIR.SpecInt.mk) #[width, signed, value, fits]
  let proof ← mkAppOptM ``VerexIR.SpecInt.ofInt_eq_of_val
    #[width, signed, default, certified, value, ← mkEqRefl value]
  return .done { expr := certified, proof? := some proof }

-- The neighbours of a key in a map literal, entry by entry, and its keys
-- and values.
attribute [vir_denote_eval] VerexIR.SpecMap.nextKey?_nil VerexIR.SpecMap.nextKey?_cons
  VerexIR.SpecMap.prevKey?_nil VerexIR.SpecMap.prevKey?_cons Option.some_or Option.none_or
  VerexIR.SpecMap.keys_nil VerexIR.SpecMap.keys_cons VerexIR.SpecMap.values_nil
  VerexIR.SpecMap.values_cons

/-- The unit value spelled `()` (`Unit.unit`), as contracts build it, where
reduction leaves its constructor (`PUnit.unit`), so that a value read in two
places reaches the deciders as one term. -/
dsimproc ↓ [vir_denote_norm, vir_transport] unitSpelling (Unit.unit) := fun e => do
  unless e.isConstOf ``PUnit.unit && e.constLevels! == [levelOne] do return .continue
  return .done (mkConst ``Unit.unit)

/-- A map's order of a constructed type's typed keys, as a contract at the
type spells it: integers by `SpecMap.intOrder`, any other by the structural
order (`NTy.keyOrder`); a type parameter's stays its family's. The order is
reducible, so a simp key reduces it away: the orders are respelled where a
clause states them, in a discipline and in a map's neighbours. -/
def reducedOrderArguments : Simp.DSimproc := fun e => do
  let arguments := e.getAppArgs
  let mut changed := false
  let mut respelled := arguments
  for i in [:arguments.size] do
    let argument := arguments[i]!
    let_expr NTy.readOrder _ _ τ := argument | continue
    let τView ← nativeTypeView τ
    let .const name _ := τView.getAppFn | continue
    unless (← getEnv).isConstructor name && name != ``NTy.param do continue
    let reduced ← whnfR argument
    unless reduced.isConstOf ``VerexIR.SpecMap.intOrder || reduced.isAppOf ``NTy.keyOrder do continue
    respelled := respelled.set! i reduced
    changed := true
  unless changed do return .continue
  return .done (mkAppN e.getAppFn respelled)

dsimproc ↓ [vir_denote_norm, vir_transport] orderedReducedOrder
    (VerexIR.SpecMap.Discipline.ordered _) := reducedOrderArguments
dsimproc ↓ [vir_denote_norm, vir_transport] nextKeyReducedOrder
    (VerexIR.SpecMap.nextKey? _ _ _) := reducedOrderArguments
dsimproc ↓ [vir_denote_norm, vir_transport] prevKeyReducedOrder
    (VerexIR.SpecMap.prevKey? _ _ _) := reducedOrderArguments

/-- Integer keys compare as integers, at every rank. -/
@[vir_denote_eval] theorem NTy.readOrder_compare_int [Carriers] (rank : VerexIR.ValueRanks)
    (width : Nat) (signed : Bool) (left right : Int) :
    (NTy.readOrder rank (.int width signed)).compare left right = compare left right := rfl

-- The integer order itself, as the order of a type argument's integer keys
-- reads at the caller.
attribute [vir_denote_eval] VerexIR.SpecMap.intOrder_compare

-- The lexicographic order of literal lists, as the structural order of
-- literal vectors unfolds to.
attribute [vir_denote_eval] List.compareLex_nil_nil List.compareLex_nil_cons
  List.compareLex_cons_nil List.compareLex_cons_cons Ordering.lt_then Ordering.gt_then
  Ordering.eq_then

/-- A comparison of two orderings is decided: what the navigation of a
literal map (`nextKey?`, `prevKey?`) unfolds to at its entries. -/
theorem Ordering.beq_eq_decide (a b : Ordering) : (a == b) = decide (a = b) := rfl

attribute [vir_denote_eval] Ordering.beq_eq_decide decide_true decide_false

attribute [vir_denote_eval] reduceCtorEq Nat.reduceAdd Nat.reduceSub Nat.reduceDiv Nat.reducePow
  Nat.reduceLT Nat.reduceEqDiff Int.reduceAdd Int.reduceSub Int.reducePow Int.reduceLT Int.reduceLE
  Int.reduceEq Int.reduceNe
  Int.reduceNatCast' Int.reduceToNat String.reduceBEq String.reduceEq String.reduceBNe
  String.reduceNe dite_true dite_false

/-- An induced frame's carriers, as the family its arguments give over the
caller's: a contract hands a specification function the frame's carriers
unapplied, where the transport of an operation reads them applied. -/
theorem Skolems.toCarriers_instantiate {unit : Validation.ValidatedUnit}
    [FunctionCarriers unit] (θ : TypeArgs) (Θ : Skolems unit) :
    (Skolems.instantiate θ Θ).toCarriers = Carriers.instantiate θ Θ.toCarriers := rfl

/-- The same frame's types, as a function: each parameter's reading
(`Skolems.type_instantiate`) where a contract hands the types unapplied, as
it does to a recursive specification function. -/
theorem Skolems.types_instantiate {unit : Validation.ValidatedUnit}
    [FunctionCarriers unit] (θ : TypeArgs) (Θ : Skolems unit) :
    (Skolems.instantiate θ Θ).type =
      fun index => ((NTy.param index).subst θ.1).substWith Θ.type :=
  funext (Skolems.type_instantiate θ Θ)

-- The transport normal form (`vir_transport`): values, operations and
-- contracts at a call's induced frame read at the caller's.
attribute [vir_transport] Skolems.toCarriers_instantiate Skolems.types_instantiate
  instantiatedEqInCallerView mapTransportIdentity optionTransportIdentity
  paramFreeValueTransport paramFreeVariantTransport paramFreeRowTransport variantNameTransport
  variantNameOuterFamily equalOuterFamily variantToSkolemConstructor hlistToSkolemConstructor
  ofSkolemIdentity
  toSkolemIdentity hlistOfSkolemIdentity hlistToSkolemIdentity variantOfSkolemIdentity
  variantToSkolemIdentity inhabitantOuterFamily plainFallbackOuterFamily
  NativeMaps.NTy.read_param_instantiate
  NativeMaps.NTy.unread_param_instantiate
  NativeMaps.ofEntriesTransport
  Carriers.orders_instantiate Carriers.erasures_instantiate
  NTy.ofSkolem_toSkolem
  NTy.toSkolem_unit NTy.toSkolem_bool NTy.toSkolem_int NTy.toSkolem_num NTy.toSkolem_param
  NTy.toSkolem_tuple NTy.toSkolem_struct NTy.toSkolem_enum NTy.toSkolem_ref NTy.toSkolem_ghost
  HList.toSkolem_nil HList.toSkolem_fst HList.toSkolem_snd HList.toSkolem_mk
  variantCarrier.toSkolem_inl variantCarrier.toSkolem_inr ResultShape.toSkolem
/-- A quantifier over certified integers, spelled as a clause quantifies over
an integer type (`Clauses`, a type's domain): over the integers the type
admits, its native value certified by the membership
(`SpecInt.forall_mk`). A specialized contract quantifies so at a type
argument's integers as a direct contract does. -/
simproc ↓ [vir_transport] forallCertified (∀ _ : _, _) := fun e => do
  let .forallE name domain body _ := e | return .continue
  if ← isProp domain then return .continue
  unless ← isProp e do return .continue
  -- The domain in the caller's spelling, as a frame a contract is
  -- specialized at induces it; the key cannot read it so.
  let domain' ← whnfD (← FamilyTransport.respellInducedFamily domain)
  let_expr VerexIR.SpecInt width signed := domain' | return .continue
  let Q := Lean.mkLambda name .default domain' body
  let unbounded := (← whnfR width).isConstOf ``VerexIR.IntWidth.unbounded
  let proof ← if unbounded then
      mkAppM ``VerexIR.Proofs.Denote.SpecInt.forall_mk_unbounded #[Q]
    else mkAppOptM ``VerexIR.Proofs.Denote.SpecInt.forall_mk #[width, signed, Q]
  let some (_, rhs) := (← whnfR (← inferType proof)).iff? | return .continue
  let expr ← Core.betaReduce rhs
  let proof ← mkExpectedTypeHint (← mkPropExt proof) (← mkEq e expr)
  return .visit { expr, proof? := some proof }

/-- Whether a closed row holds a reference, decided: the type arguments of a
transport (`InvokeMeaning.Coherent`). -/
dsimproc [vir_transport] reduceRowRefFree (NRow.refFree _) := fun e => do
  if e.hasFVar || e.hasMVar then return .continue
  let reduced ← withDefault (whnf e)
  if reduced.isConstOf ``Bool.true || reduced.isConstOf ``Bool.false then return .done reduced
  return .continue

open Lean Meta Simp in
/-- A behavioral predicate at the frame a call's type arguments induce, at
the outer frame (`rewriteInducedFrame`): by the executable's coherence or a
model's. -/
private def behaviorInducedFrame (frameIndex : Nat) (rules : List Name) : Simproc := fun e => do
  unless e.getAppNumArgs > frameIndex &&
      (e.getArg! frameIndex).consumeMData.isAppOf ``Skolems.instantiate do
    return .continue
  for rule in rules do
    let step ← rewriteInducedFrame rule e
    unless step matches .continue .. do return step
  return .continue

simproc [vir_transport] ensuresOfInducedFrame (@VerexIR.Proofs.EnsuresOf _ ?functions ?frame _ _ _ _ _ _ _ _ _) :=
  behaviorInducedFrame 2 [``VerexIR.Proofs.closureMeaning_ensuresOf_instantiate,
    ``VerexIR.Proofs.ModelUnit.ensuresOf_instantiate]
simproc [vir_transport] abortsOfInducedFrame (@VerexIR.Proofs.AbortsOf _ ?functions ?frame _ _ _ _ _ _ _) :=
  behaviorInducedFrame 2 [``VerexIR.Proofs.closureMeaning_abortsOf_instantiate,
    ``VerexIR.Proofs.ModelUnit.abortsOf_instantiate]
simproc [vir_transport] resultOfInducedFrame
    (@VerexIR.Proofs.ResultOf _ ?functions ?frame _ _ _ _ _ _ _ _) :=
  behaviorInducedFrame 2 [``VerexIR.Proofs.closureMeaning_resultOf_instantiate,
    ``VerexIR.Proofs.ModelUnit.resultOf_instantiate]
simproc [vir_transport] stateOfInducedFrame (@VerexIR.Proofs.StateOf _ ?functions ?frame _ _ _ _ _ _ _) :=
  behaviorInducedFrame 2 [``VerexIR.Proofs.closureMeaning_stateOf_instantiate,
    ``VerexIR.Proofs.ModelUnit.stateOf_instantiate]
simproc [vir_transport] requiresOfInducedFrame
    (@VerexIR.Proofs.RequiresOf _ ?functions ?frame _ _ _ _ _ _ _) :=
  behaviorInducedFrame 2 [``VerexIR.Proofs.requiresOf_instantiate]

-- Behavioral predicates at the outer frame (above), the substitutions they
-- read evaluated, and the type arguments' freedom from references decided.
attribute [vir_transport] reduceParameterSubst reduceRowSubst reduceRowsSubst ResultShape.subst
  ResultShape.ofSkolem ResultShape.functionOfSkolem

-- Complete an action before traversing the continuation it selects.
attribute [vir_denote ↓] VerexIR.Proofs.Denote.Flow.bind_pure_value
  VerexIR.Proofs.Denote.Flow.bind_pure_return VerexIR.Proofs.Denote.Flow.bind_pure_break
  VerexIR.Proofs.Denote.Flow.bind_pure_continue VerexIR.Proofs.Denote.Flow.bind_abort
  VerexIR.Proofs.Denote.Flow.bind_abortAt VerexIR.Proofs.Spec.pure_bind

open Lean Meta Simp in
/-- A completed action's `wp` before its continuation is traversed:
`wp_pure` and `wp_abort` at a `wp` itself. Not as rules taken before the
arguments: a path's markers (`Taken`, `Matched`, `Dispatched`, `AfterAt`,
`AbortsAt`) are reducible, so such a rule matches a marked `wp` at the
marker and rewrites the marker away with it, and the path loses its
decision or point. -/
def completedAction (e : Lean.Expr) : SimpM Step := do
  unless e.isAppOfArity ``VerexIR.Proofs.wp 7 do return .continue
  let action := (e.getArg! 3).consumeMData
  let ensures := e.getArg! 4
  let aborts := e.getArg! 5
  let state := e.getArg! 6
  let (rule, result) ←
    if action.isAppOfArity ``VerexIR.Proofs.Spec.pure 4 then
      let value := action.getArg! 3
      pure (← mkAppM ``VerexIR.Proofs.wp_pure #[value, state, ensures, aborts],
        (mkApp2 ensures value state).headBeta)
    else if action.isAppOfArity ``VerexIR.Proofs.Spec.abort 4 then
      let error := action.getArg! 3
      pure (← mkAppM ``VerexIR.Proofs.wp_abort #[error, state, ensures, aborts],
        (mkApp aborts error).headBeta)
    else return .continue
  return .visit { expr := result, proof? := some (← mkPropExt rule) }

simproc ↓ [vir_denote] completedPure (VerexIR.Proofs.wp (VerexIR.Proofs.Spec.pure _) _ _ _) :=
  completedAction
simproc ↓ [vir_denote] completedAbort (VerexIR.Proofs.wp (VerexIR.Proofs.Spec.abort _) _ _ _) :=
  completedAction

end VerexIR.Proofs.Denote
