-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Tests.NominalQuotation

open Lean Meta Elab Command VerexSL VerexSL.Contract VerexIR.Proofs.Denote

-- Parameter replacement constructs the descriptor at its replaced explicit
-- arguments, including nested and phantom arguments. An unknown argument is
-- kept under substitution because it may itself contain parameters.
/--
error: nominal parameter replacement changed its type
-/
#guard_msgs in
run_cmd do
  let some unit := registeredUnit? (← getEnv) `«0x42».nominal_quotation
    | throwError "missing nominal quotation fixture"
  let name ← ensureUnitDefinition #["0x42", "nominal_quotation"] unit
  liftTermElabM <| withNominalUnit name do
    let procs ← Simp.SimprocsArray.add #[] ``reduceNominalParameterReplacement false
    let context ← Simp.mkContext { zeta := false, beta := false, iota := false, proj := false }
    let typeFunction ← mkArrow (mkConst ``Nat) (mkConst ``NTy)
    withLocalDeclD `types typeFunction fun types => do
      withLocalDeclD `unknown (mkConst ``NTy) fun unknown => do
        let unknownOperation := mkApp2 (mkConst ``NTy.substWith) types unknown
        let (unknownResult, _) ← simp unknownOperation context procs
        unless unknownResult.expr == unknownOperation do
          throwError "parameter replacement guessed an unknown type"
        let some schema0 := structNTy unit ⟨⟨0⟩, 0⟩ | throwError "missing fixture schema"
        let owner0 ← quoteNTy schema0
        let opaqueOwner := mkApp owner0.getAppFn unknown
        let operation := mkApp2 (mkConst ``NTy.substWith) types opaqueOwner
        let (result, _) ← simp operation context procs
        unless result.expr == operation do
          throwError "parameter replacement guessed an unknown argument"
      for index in [0, 1, 4, 5] do
        let some schema := structNTy unit ⟨⟨0⟩, index⟩ | throwError "missing fixture schema"
        let owner ← quoteNTy schema
        let replaced ← quoteResolvedNTy types schema
        let operation := mkApp2 (mkConst ``NTy.substWith) types owner
        let (result, _) ← simp operation context procs
        unless result.expr == replaced do
          throwError "parameter replacement did not retain its descriptor: {result.expr}"
        unless ← withDefault (isDefEq operation result.expr) do
          throwError "nominal parameter replacement changed its type"
      let some schema := structNTy unit ⟨⟨0⟩, 0⟩ | throwError "missing fixture schema"
      let some nested := structNTy unit ⟨⟨0⟩, 4⟩ | throwError "missing nested schema"
      let nested := schema.subst (.cons nested .nil)
      let nestedOwner ← quoteNTy nested
      let nestedOperation := mkApp2 (mkConst ``NTy.substWith) types nestedOwner
      let (nestedResult, _) ← simp nestedOperation context procs
      unless nestedResult.expr == (← quoteResolvedNTy types nested) &&
          (← withDefault (isDefEq nestedOperation nestedResult.expr)) do
        throwError "nested nominal arguments did not replace canonically"

-- Concrete named layouts compare by value; symbolic parameters and handle
-- comparisons retain their native comparison rather than acquiring this rule.
run_cmd do
  let some unit := registeredUnit? (← getEnv) `«0x42».nominal_quotation
    | throwError "missing nominal quotation fixture"
  let name ← ensureUnitDefinition #["0x42", "nominal_quotation"] unit
  liftTermElabM do
    let procs ← Simp.SimprocsArray.add #[] ``equalAsEquation false
    let context ← Simp.mkContext { zeta := false, beta := false, iota := false, proj := false }
    withNominalUnit name do
      for index in [0, 5] do
        let some schema := structNTy unit ⟨⟨0⟩, index⟩ | throwError "missing fixture schema"
        for concrete in [false, true] do
          let τ ← quoteNTy (if concrete then schema.subst (.cons .bool .nil) else schema)
          withLocalDeclD `family (mkConst ``Carriers) fun family => do
            let carrier := mkApp2 (mkConst ``NTy.carrier) family τ
            withLocalDeclD `left carrier fun left => do
              withLocalDeclD `right carrier fun right => do
                for handles in [false, true] do
                  let comparison := mkAppN (mkConst ``NTy.equal)
                    #[family, toExpr handles, τ, left, right]
                  let (result, _) ← simp comparison context procs
                  if concrete && !handles then
                    unless result.expr.isAppOf ``decide do
                      throwError "named concrete comparison did not become equality: {result.expr}"
                    let some proof := result.proof? | throwError "missing comparison proof"
                    discard <| inferType proof
                    unless (proof.find? (fun e => e == τ)).isSome do
                      throwError "comparison proof erased the descriptor"
                  else
                    unless result.expr == comparison do
                      throwError "unsafe comparison rule application: {result.expr}"

-- A structural field projection must preserve its owner's descriptor in the
-- recursive read. Otherwise `exists<Fee>` and `Fee[owner]` acquire different
-- resource key spellings after normalization.
verex module 0x42::nominal_resource_index where
  pragma verify = false
  struct Fee has Key where
    bps : u64
  fun read_fee(owner : Address) -> u64 := Fee[owner].bps
  fun bounded_fee(owner : Address) -> u64 := do
    let bps := if exists<Fee>(owner) && Fee[owner].bps <= 10000 then Fee[owner].bps else 500
    10000 - bps
  spec bounded_fee where
    aborts_if false
    ensures result <= 10000

run_cmd do
  let some unit := registeredUnit? (← getEnv) `«0x42».nominal_resource_index
    | throwError "missing nominal resource fixture"
  let name ← ensureUnitDefinition #["0x42", "nominal_resource_index"] unit
  liftTermElabM <| withNominalUnit name do
    let .ok compiled := compileModelFunction unit ⟨⟨0⟩, ⟨0⟩⟩
      | throwError "failed to compile nominal resource fixture"
    let some resource := structNTy unit ⟨⟨0⟩, 0⟩
      | throwError "missing nominal resource"
    let owner ← quoteNTy resource
    let quoted ← VerexSL.Verify.quoteTerm (mkConst name) compiled.body
    let row ← quoteRow compiled.frame
    let shape ← quoteShape compiled.result
    let bodyType ← quoteNTy compiled.result.bodyType
    let some extension ← getSimpExtension? `vir_denote
      | throwError "missing denotation rules"
    let rules ← extension.getTheorems
    let context ← Simp.mkContext { zeta := false, beta := true, iota := false, proj := false }
      #[rules]
    withLocalDeclD `family (mkConst ``Carriers) fun family =>
      withLocalDeclD `operations (mkApp3 (mkConst ``InterpreterOps) (mkConst name) family (mkConst ``Unit)) fun operations =>
        withLocalDeclD `locals (mkApp2 (mkConst ``HEnv) family row) fun locals => do
          let call := mkAppN (mkConst ``Term.denoteIn)
            #[mkConst name, family, mkConst ``Unit, operations, shape, row, bodyType, quoted, locals]
          let (normalized, _) ← simp call context
          let reads := normalized.expr.find? fun e => e.isAppOfArity ``InterpreterOps.read 8
          let some read := reads | throwError "nominal resource read was not exposed"
          unless read.getArg! 4 == owner do
            throwError "resource read lost its nominal type: {read.getArg! 4}, expected {owner}"

verify 0x42::nominal_resource_index::bounded_fee by
  on _ { }

-- The opener's equivalence of a row's fact emits the declaration's own
-- predicate of an encoding at the code it is stated of, unreplaced.
run_cmd do
  let some unit := registeredUnit? (← getEnv) `«0x42».nominal_quotation
    | throwError "missing nominal quotation fixture"
  let name ← ensureUnitDefinition #["0x42", "nominal_quotation"] unit
  liftTermElabM <| withNominalUnit name do
    for index in [0, 5] do
      let some schema := structNTy unit ⟨⟨0⟩, index⟩ | throwError "missing fixture schema"
      let owner ← quoteNTy (schema.subst (.cons .bool .nil))
      withLocalDeclD `family (mkConst ``Carriers) fun family =>
        withLocalDeclD `row (mkApp2 (mkConst ``NTy.carrier) family owner) fun row => do
          let view ← nativeTypeView owner
          let equivalence := if view.isAppOfArity ``NTy.struct 3 then ``NTy.holds_struct
            else ``NTy.holds_enum
          let opened := mkAppN (mkConst equivalence)
            (#[family] ++ view.getAppArgs ++ #[row])
          let stated ← inferType opened
          let some encoded := stated.find? (·.isAppOfArity ``NTy.encode 3)
            | throwError "the opener's equivalence emitted no encoding"
          unless encoded.getArg! 0 == family && encoded.getArg! 1 == view &&
              encoded.getArg! 2 == row do
            throwError "the opener's equivalence changed its encoding: {encoded}"
          unless ← isTypeCorrect opened do
            throwError "the opener's equivalence is ill typed"

-- Ordering selects a named declaration's structural branch before any split
-- of the generic parameter branch, and observes only its immediate row.
run_cmd do
  let some unit := registeredUnit? (← getEnv) `«0x42».nominal_quotation
    | throwError "missing nominal quotation fixture"
  let name ← ensureUnitDefinition #["0x42", "nominal_quotation"] unit
  liftTermElabM <| withNominalUnit name do
    let orderProcs ← Simp.SimprocsArray.add #[] ``nominalOrder false
    let context ← Simp.mkContext { zeta := false, beta := false, iota := false, proj := false }
    withLocalDeclD `family (mkConst ``Carriers) fun family =>
      withLocalDeclD `rank (mkConst ``VerexIR.ValueRanks) fun rank =>
        withLocalDeclD `unknown (mkConst ``NTy) fun unknown => do
          let unknownCarrier := mkApp2 (mkConst ``NTy.carrier) family unknown
          withLocalDeclD `left unknownCarrier fun left =>
            withLocalDeclD `right unknownCarrier fun right => do
              let observed := mkAppN (mkConst ``NTy.order) #[family, rank, unknown, left, right]
              let (result, _) ← simp observed context orderProcs
              unless result.expr == observed do
                throwError "nominal order unfolded an unknown type: {result.expr}"
    for index in [0, 5] do
      let some schema := structNTy unit ⟨⟨0⟩, index⟩ | throwError "missing fixture schema"
      let owner ← quoteNTy schema
      withLocalDeclD `family (mkConst ``Carriers) fun family =>
        withLocalDeclD `rank (mkConst ``VerexIR.ValueRanks) fun rank => do
          let carrier := mkApp2 (mkConst ``NTy.carrier) family owner
          withLocalDeclD `left carrier fun left =>
            withLocalDeclD `right carrier fun right => do
              let observed := mkAppN (mkConst ``NTy.order) #[family, rank, owner, left, right]
              let (result, _) ← simp observed context orderProcs
              unless result.expr.isAppOf ``HList.order ||
                  result.expr.isAppOf ``variantCarrier.order do
                throwError "order did not select its declaration: {result.expr}"
              let some proof := result.proof? | throwError "order has no equation"
              unless ← isTypeCorrect proof do throwError "order equation is ill typed"

-- Distinct declaration handles remain distinct when a symbolic frame
-- resolves their type parameters; their fields are irrelevant to dispatch.
run_cmd do
  let some unit := registeredUnit? (← getEnv) `«0x42».nominal_quotation
    | throwError "missing nominal quotation fixture"
  let name ← ensureUnitDefinition #["0x42", "nominal_quotation"] unit
  liftTermElabM <| withNominalUnit name do
    let some first := structNTy unit ⟨⟨0⟩, 0⟩ | throwError "missing first schema"
    let some second := structNTy unit ⟨⟨0⟩, 1⟩ | throwError "missing second schema"
    let first ← quoteNTy first
    let second ← quoteNTy second
    let procs ← Simp.SimprocsArray.add #[] ``nominalDeclarationEquality false
    let context ← Simp.mkContext { zeta := false, beta := false, iota := false, proj := false }
    withLocalDeclD `functions (mkApp (mkConst ``FunctionCarriers) (mkConst name)) fun functions =>
      withLocalDeclD `frame (mkApp2 (mkConst ``Skolems) (mkConst name) functions) fun frame => do
        let resolve := mkApp3 (mkConst ``Skolems.resolve) (mkConst name) functions frame
        let observed ← mkEq (mkApp resolve first) (mkApp resolve second)
        let (result, _) ← simp observed context procs
        unless result.expr.isFalse do
          throwError "resolved nominal resources did not retain distinct identities"
        let some proof := result.proof? | throwError "missing resource distinction proof"
        unless ← isTypeCorrect proof do throwError "resource distinction proof is ill typed"

verex module 0x42::nominal_local_write where
  pragma verify = false
  struct BytePair has Drop where
    data : Vector<u8>
  fun first(byte : u8) -> BytePair := do
    let mut value := new BytePair { data := #[0, 0] }
    value.data[0] := byte
    value
  spec first where
    aborts_if false
    ensures result.data[0] == byte
    ensures result.data[1] == 0

verify 0x42::nominal_local_write::first by
  on _ { }

-- A vector field write retains one named Option codomain through its row equation.
verex module 0x42::nominal_vector_write where
  struct Bits has Copy, Drop where
    length : u64
    data : Vector<Bool>
  spec Bits where
    invariant length == data.length
  fun set(self : &mut Bits, index : u64) -> Unit := do
    self.data[index] := true
  spec set where
    requires index < self.length
    aborts_if false
    ensures self.data[index]
    ensures self.length == old(self).length
    ensures ∀ (k in 0 .. self.length), k != index ==> self.data[k] == old(self).data[k]

-- Quoted fields use named projections with inferred type arguments;
-- the row keeps its original nominal descriptor.
run_cmd do
  let some unit := registeredUnit? (← getEnv) `«0x42».nominal_quotation
    | throwError "missing nominal quotation fixture"
  let name ← ensureUnitDefinition #["0x42", "nominal_quotation"] unit
  liftTermElabM <| withNominalUnit name do
    let some schema := structNTy unit ⟨⟨0⟩, 1⟩ | throwError "missing phantom fixture"
    let owner ← quoteNTy (schema.subst (.cons .bool .nil))
    withLocalDeclD `family (mkConst ``Carriers) fun family => do
      let rowType := mkApp2 (mkConst ``NRow.cons) owner (mkConst ``NRow.nil)
      withLocalDeclD `values (mkApp2 (mkConst ``HList) family rowType) fun values => do
        let projected ← quotedRowProjections family values #[owner]
        unless projected == #[← mkAppM ``Prod.fst #[values]] do
          throwError "field does not match an inferred named projection"
        unless ← withDefault (isTypeCorrect projected[0]!) do
          throwError "field projection is ill typed"
