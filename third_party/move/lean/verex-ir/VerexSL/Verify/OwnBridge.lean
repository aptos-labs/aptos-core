-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Contract
import VerexIR.Proofs.Denote.OwnBridge.Registry

/-!
# The bridge of a declaration's own invariant at an instance row

A unit installs a declaration's clauses at the frame a row of type arguments
induces over the base function values (`Contract.dataOwnTerms`). A caller
holds a value of the declaration at its own frame, at an instance row. The
bridge states the value's fact as the same source clauses read at the caller's
frame over the caller's value (`Contract.ownClausesAt`), with the facts of its
fields, as an equivalence (`NTy.holds_struct_bridge`, `NTy.holds_enum_bridge`).

Its one obligation is the agreement of the two readings across the move of
the value (`Skolems.reframeAtInstance`). It is built here along the installed
clause, one operator at a time, each step an instance of a law of
`VerexIR.Proofs.Denote.OwnBridge.Operators` or a congruence of an operator no
frame reads; the operator of the installed term selects the step. An operator
without a law ends the construction with the operator named, and the
declaration has no bridge at that row.

One step is not a plain law instance. A read with a fallback needs its index
bounded: the bound is asked of `omega`, once, over the caller-side guards in
scope (`boundFromGuards`), a decision procedure of linear arithmetic and no
search of the clause. Where it derives no bound, the read takes the one other
law there is for it, proved separately for a fallback that agrees across the
move (`Skolems.reframeAtInstance_vector_read_of_fallback`), which holds where
the element's carrier does not read the frame; where that law's premise does
not hold either, the construction ends. The two `<|>` in this module replace
a failure's message and select nothing. No step is retried and no other
alternative exists.

The kernel checks the result, with every conversion a law's instance needs at
the spelling the clauses use.

This module does not import the solver.
-/

namespace VerexSL.Verify.OwnBridge

open Lean Meta
open VerexIR.Validation (ValidatedUnit)
open VerexIR.Proofs.Denote
open VerexSL.Contract

/-- The frames a bridge is built between. -/
structure Frames where
  /-- What every law of the move takes first: the unit, its function values
  and base function values, their three agreements, the caller's frame, the
  instance row, and the inhabitability of the row's resolutions. -/
  leading : Array Lean.Expr
  /-- The caller-side guards in scope, which bound a read. -/
  guards : Array Lean.Expr := #[]

abbrev BuildM := ReaderT Frames MetaM

private def law (name : Name) (arguments : Array Lean.Expr) : BuildM Lean.Expr :=
  return mkAppN (mkConst name) ((← read).leading ++ arguments)

/-- A proof stated at the type it is used at: the conversion is the kernel's. -/
private def stated (proof left right : Lean.Expr) : MetaM Lean.Expr := do
  mkExpectedTypeHint proof (← mkEq left right)

private def spelled (e : Lean.Expr) : MetaM String :=
  return toString (((toString (← ppExpr e)).replace "\n" " ").take 160)

/-- The operator a term applies, for a report. -/
private def operatorOf (e : Lean.Expr) : String :=
  match e.getAppFn with
  | .const name _ => toString name
  | .fvar _ => "a bound bound"
  | .lam .. => "a function"
  | .forallE .. => "a binder"
  | .proj structure_ index _ => s!"the projection {structure_}.{index}"
  | _ => "a term"

/-- That a type of the declaration is over its parameters, which every move
at it takes: decided by evaluation. -/
private def bounded (τ : Lean.Expr) : BuildM Lean.Expr := do
  let move := mkAppN (mkConst ``Skolems.reframeAtInstance) ((← read).leading.push τ)
  let .forallE _ domain _ _ ← whnfR (← inferType move)
    | throwError "internal: the move at {τ} takes no bound"
  let_expr Eq _ decided _ := domain | throwError "internal: the move's bound is no equation"
  unless (← whnfD decided).isConstOf ``Bool.true do
    throwError "the type {← spelled τ} is not over the declaration's parameters"
  mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true)) domain

/-- The move of a caller's value at a type of the declaration. -/
private def moveOf (τ value : Lean.Expr) : BuildM Lean.Expr := do
  return mkAppN (mkConst ``Skolems.reframeAtInstance) ((← read).leading ++ #[τ, ← bounded τ, value])

/-- A term with one subterm exchanged, and the equation of the two from the
equation of the subterms. -/
private def congruentAt (term part part' same : Lean.Expr) : MetaM (Lean.Expr × Lean.Expr) := do
  if part == part' then return (term, ← mkEqRefl term)
  let motive ← withLocalDeclD `part (← inferType part) fun bound => do
    mkLambdaFVars #[bound]
      (term.replace fun inner => if inner == part then some bound else none)
  return ((motive.beta #[part']), ← mkCongrArg motive same)

/-- A value at the declaration's frame that is the move of a caller's value:
the declaration's type of it, the caller's value, and the equation. -/
structure Moved where
  type : Lean.Expr
  caller : Lean.Expr
  moved : Lean.Expr
  proof : Lean.Expr

private def different {α : Type} (what : String) (installed caller : Lean.Expr) : BuildM α := do
  throwError "the two readings differ in {what}: `{← spelled installed}` at the declaration's \
    frame, `{← spelled caller}` at the caller's"

private def headRow (row : Lean.Expr) : MetaM (Lean.Expr × Lean.Expr) := do
  let_expr NRow.cons head rest := ← whnfD row
    | throwError "internal: a field is read off the empty row {row}"
  return (head, rest)

/-- A bound of a read, decided by `omega` from the caller-side guards in scope:
one call, by refutation of the guards with the bound denied; none where it
derives no bound. -/
private def boundFromGuards (index size : Lean.Expr) : BuildM (Option Lean.Expr) := do
  let goal ← mkFreshExprMVar (← mkAppM ``LT.lt #[index, size])
  let guards := (← read).guards
  let outer ← getLCtx
  try
    -- The decision procedure refutes the guards with the bound denied.
    if let some refuted ← goal.mvarId!.falseOrByContra then
      refuted.withContext do
        let denied := (← getLCtx).foldl (init := #[]) fun found decl =>
          if outer.contains decl.fvarId then found else found.push decl.toExpr
        Lean.Elab.Tactic.Omega.omega (guards ++ denied).toList refuted
    return some (← instantiateMVars goal)
  catch _ => return none

/-- That a key reader identifies only equal keys, by the reader's form. -/
private def injective (reader : Lean.Expr) : MetaM Lean.Expr := do
  let statement ← mkAppM ``Function.Injective #[reader]
  if let .lam _ domain (.bvar 0) _ := reader then
    let proof ← withLocalDeclD `left domain fun left => withLocalDeclD `right domain fun right => do
      withLocalDeclD `same (← mkEq left right) fun same => mkLambdaFVars #[left, right, same] same
    return ← mkExpectedTypeHint proof statement
  if reader.isAppOfArity ``NTy.read 2 then
    return mkAppN (mkConst ``NativeMaps.NTy.read_injective) reader.getAppArgs
  if reader.isAppOfArity ``VerexIR.SpecInt.val 2 then
    return mkAppN (mkConst ``VerexIR.SpecInt.val_injective) reader.getAppArgs
  throwError "the key reader `{← spelled reader}` of a table's map has no injectivity law"

/-- A premise that holds by computation, stated as the law states it. -/
private def byComputation (premise : Lean.Expr) (outstanding : MessageData) : MetaM Lean.Expr :=
  forallTelescope premise fun binders body => do
    let_expr Eq _ left right := body | throwError "internal: the premise {premise} is no equation"
    unless ← withNewMCtxDepth (isDefEq left right) do throwError outstanding
    mkLambdaFVars binders (← mkExpectedTypeHint (← mkEqRefl left) body)

mutual

/-- The installed term `installed`, a value at the declaration's frame, as the
move of the caller's term `caller`. -/
partial def movedValue (installed caller : Lean.Expr) : BuildM Moved := do
  let installed := installed.consumeMData
  let caller := caller.consumeMData
  if installed.isAppOfArity ``Skolems.reframeAtInstance 12 then
    unless installed.appArg! == caller do different "the value read" installed caller
    return { type := installed.getArg! 9, caller, moved := installed, proof := ← mkEqRefl installed }
  let finish (type : Lean.Expr) (proof : Lean.Expr) : BuildM Moved := do
    let moved ← moveOf type caller
    return { type, caller, moved, proof := ← stated proof installed moved }
  -- A field read moves with the row it reads: the head field or the rest.
  let field (row row' : Lean.Expr) (head : Bool) : BuildM Moved := do
    let whole ← movedValue row row'
    let (type, proof) ← component whole head
    finish type (← mkEqTrans (← mkCongrArg installed.appFn! whole.proof) proof)
  match_expr installed with
  | Prod.fst _ _ row =>
      let_expr Prod.fst _ _ row' := caller | different "a field read" installed caller
      field row row' true
  | Prod.snd _ _ row =>
      let_expr Prod.snd _ _ row' := caller | different "a field read" installed caller
      field row row' false
  | Option.getD _ read fallback =>
      let_expr Option.getD _ read' fallback' := caller
        | different "a read with a fallback" installed caller
      let_expr getElem? _ _ _ _ _ elements index := read
        | throwError "a fallback of `{operatorOf read}`, which is no read of a vector, has no law"
      let_expr getElem? _ _ _ _ _ elements' index' := read'
        | different "a read with a fallback" installed caller
      let_expr VerexIR.SpecVector.values _ vector := elements
        | throwError "a read of `{operatorOf elements}`, which is no vector's elements, has no law"
      let_expr VerexIR.SpecVector.values _ vector' := elements'
        | different "a read with a fallback" installed caller
      let whole ← movedValue vector vector'
      let_expr NTy.vector element := ← whnfD whole.type
        | throwError "a read of a value of type {← spelled whole.type}, which is no vector, has no law"
      let vectorType := mkApp (mkConst ``NTy.vector) element
      let (atVector, first) ← congruentAt installed vector whole.moved whole.proof
      let (_, second) ← match ← frameFree index index' with
        | none => pure (atVector, ← mkEqRefl atVector)
        | some same => congruentAt atVector index index' same
      let size ← mkAppM ``Array.size #[elements']
      let proof ← match ← boundFromGuards index' size with
        | some within =>
            law ``Skolems.reframeAtInstance_vector_read
              #[element, ← bounded vectorType, vector', index', within, fallback, fallback']
        | none =>
            -- No bound was derived, so the read takes the law for
            -- fallbacks that agree across the move: they do where the
            -- element's carrier does not read the frame, and the
            -- construction ends where they do not.
            let moves ← (do
                let identity ← closedIdentity element fallback'
                unless ← withNewMCtxDepth (isDefEq fallback' fallback) do
                  throwError "the fallbacks differ"
                stated identity (← moveOf element fallback') fallback) <|>
              (do
                let element ← spelled element
                (throwError "a read with a fallback whose bound does not follow from the \
                  clause's own context, at the element type {element}: no law moves a fallback \
                  of a carrier that reads the frame" : BuildM Lean.Expr))
            law ``Skolems.reframeAtInstance_vector_read_of_fallback
              #[element, ← bounded vectorType, vector', index', fallback, fallback', moves]
      finish element (← mkEqTrans (← mkEqTrans first second) proof)
  | _ =>
      throwError "the operator `{operatorOf installed}` has no law over a value whose carrier \
        reads the frame: `{← spelled installed}`"

/-- The head or the rest of a moved row: the type of the part and the law's
instance at the row's own handle, arguments and fields. -/
partial def component (whole : Moved) (head : Bool) : BuildM (Lean.Expr × Lean.Expr) := do
  match_expr ← whnfD whole.type with
  | NTy.struct source arguments fields =>
      let (first, rest) ← headRow fields
      let row := mkApp3 (mkConst ``NTy.struct) source arguments (mkApp2 (mkConst ``NRow.cons) first rest)
      let leading := #[source, arguments, first, rest, ← bounded row, whole.caller]
      if head then return (first, ← law ``Skolems.reframeAtInstance_struct_fst leading)
      return (mkApp3 (mkConst ``NTy.struct) source arguments rest,
        ← law ``Skolems.reframeAtInstance_struct_snd leading)
  | NTy.tuple elements =>
      let (first, rest) ← headRow elements
      let row := mkApp (mkConst ``NTy.tuple) (mkApp2 (mkConst ``NRow.cons) first rest)
      let leading := #[first, rest, ← bounded row, whole.caller]
      if head then return (first, ← law ``Skolems.reframeAtInstance_tuple_fst leading)
      return (mkApp (mkConst ``NTy.tuple) rest, ← law ``Skolems.reframeAtInstance_tuple_snd leading)
  | _ =>
      throwError "a component of a value of type {← spelled whole.type}, which is neither a \
        struct's row nor a tuple, has no law"

/-- A caller's value of a type whose carrier and encoding do not read the
frame is its own move (`Skolems.reframeAtInstance_of_encode`): the equation of
the move and the value. -/
partial def closedIdentity (type value : Lean.Expr) : BuildM Lean.Expr := do
  unless (← whnfD (mkApp (mkConst ``NTy.paramFree) type)).isConstOf ``Bool.true do
    throwError "the type {← spelled type} reads the frame"
  let moved ← moveOf type value
  let source ← inferType value
  let target ← inferType moved
  unless ← withNewMCtxDepth (isDefEq source target) do
    throwError "the carrier of {← spelled type} is not the same at the two frames"
  let carriers ← mkExpectedTypeHint (← mkEqRefl source) (← mkEq source target)
  let applied ← law ``Skolems.reframeAtInstance_of_encode #[type, ← bounded type, value, carriers]
  let .forallE _ premise _ _ ← whnfR (← inferType applied)
    | throwError "internal: the identity of the move at {type} takes no encoding premise"
  let encoded ← byComputation premise
    m!"the encoding of {type} is not the same at the two frames"
  stated (mkApp applied encoded) moved value

/-- A comparison supplies the native type of both operands. An identical closed
operand at a frame-independent type is its own move. Other operands carry
their type in the reframe or projection from which they are read. -/
partial def movedOperand (type installed caller : Lean.Expr) : BuildM Moved := do
  let installed := installed.consumeMData
  let caller := caller.consumeMData
  if installed == caller && !caller.hasFVar && !caller.hasMVar && !caller.hasLooseBVars then
    let identity ← closedIdentity type caller
    let moved ← moveOf type caller
    return { type, caller, moved, proof := ← stated (← mkEqSymm identity) installed moved }
  movedValue installed caller

/-- The installed term, a value used where no law of the move applies, as the
caller's term: its type's carrier does not read the frame. -/
partial def closedValue (installed caller : Lean.Expr) : BuildM Lean.Expr := do
  let whole ← movedValue installed caller
  let identity ← (closedIdentity whole.type caller) <|> (do
    let type ← spelled whole.type
    let term ← spelled installed
    (throwError "a value of type {type}, whose carrier reads the frame, is read by an operator \
      that has no law: `{term}`" : BuildM Lean.Expr))
  stated (← mkEqTrans whole.proof identity) installed caller

/-- The equation of an installed term and the caller's, both of a type no
frame reads; `none` where they are the same term. -/
partial def frameFree (installed caller : Lean.Expr) : BuildM (Option Lean.Expr) := do
  let installed := installed.consumeMData.headBeta
  let caller := caller.consumeMData.headBeta
  if installed == caller then return none
  if let .forallE name domain body info := installed then
    let .forallE _ domain' body' _ := caller | different "a binder" installed caller
    if !body.hasLooseBVars && !body'.hasLooseBVars && (← isProp domain) then
      -- A guard: the consequent's agreement may use the caller's guard.
      let guard ← proved domain domain'
      let consequent ← withLocalDeclD `guard domain' fun hypothesis =>
        withReader (fun frames => { frames with guards := frames.guards.push hypothesis }) do
          mkLambdaFVars #[hypothesis] (← proved body body')
      return some (← mkAppM ``implies_congr_ctx #[guard, consequent])
    let left := Lean.Expr.lam name domain body info
    if domain == domain' then
      let pointwise ← withLocalDecl name info domain fun bound => do
        mkLambdaFVars #[bound] (← proved (body.instantiate1 bound) (body'.instantiate1 bound))
      return some (← mkAppOptM ``forall_congr
        #[some domain, some left, some (Lean.Expr.lam name domain body' info), some pointwise])
    -- A binder over a carrier ranges over the moved values of the caller's.
    let_expr NTy.carrier _ type := domain
      | throwError "a binder over `{← spelled domain}` has no law"
    let right := Lean.Expr.lam name domain' body' info
    let agree ← withLocalDecl name info domain' fun bound => do
      let same ← proved (body.instantiate1 (← moveOf type bound)) (body'.instantiate1 bound)
      mkLambdaFVars #[bound] (← mkAppM ``Iff.of_eq #[same])
    let equivalence ← law ``Skolems.forall_reframeAtInstance
      #[type, ← bounded type, left, right, agree]
    return some (← stated (mkApp3 (mkConst ``propext) installed caller equivalence) installed caller)
  if let .lam name domain body info := installed then
    let .lam _ domain' body' _ := caller | different "a function" installed caller
    unless domain == domain' do
      throwError "a function over `{← spelled domain}`, whose values read the frame, has no law"
    let pointwise ← withLocalDecl name info domain fun bound => do
      mkLambdaFVars #[bound] (← proved (body.instantiate1 bound) (body'.instantiate1 bound))
    return some (← mkFunExt pointwise)
  let arguments := installed.getAppArgs
  let arguments' := caller.getAppArgs
  let result (proof : Lean.Expr) : BuildM (Option Lean.Expr) :=
    return some (← stated proof installed caller)
  -- A field read, or a read with a fallback, over a domain the two sides
  -- spell differently is a closed value on both sides; over the same domain it
  -- is the congruence of the reads.
  if #[``Prod.fst, ``Prod.snd, ``Option.getD].any (installed.isAppOfArity · 3) &&
      installed.getArg! 0 != arguments'[0]! then
    return some (← closedValue installed caller)
  match_expr installed with
  | NTy.order _ rank type left right =>
      let_expr NTy.order _ rank' _ left' right' := caller | different "an order" installed caller
      unless rank == rank' do
        throwError "the two readings order by different ranks: `{← spelled rank}` at the \
          declaration's frame, `{← spelled rank'}` at the caller's"
      let first ← movedOperand type left left'
      let second ← movedOperand type right right'
      let moved ← mkCongr (← mkCongrArg (mkAppN installed.getAppFn (arguments.extract 0 3)) first.proof)
        second.proof
      result (← mkEqTrans moved (← law ``Skolems.order_reframeAtInstance
        #[type, ← bounded type, rank, left', right']))
  | NTy.equal _ handles type left right =>
      let_expr NTy.equal _ handles' _ left' right' := caller | different "an equality" installed caller
      unless handles == handles' && handles.isConstOf ``Bool.false do
        throwError "an equality that reads handle identity has no law"
      let first ← movedOperand type left left'
      let second ← movedOperand type right right'
      let moved ← mkCongr (← mkCongrArg (mkAppN installed.getAppFn (arguments.extract 0 3)) first.proof)
        second.proof
      result (← mkEqTrans moved (← law ``Skolems.equal_reframeAtInstance
        #[type, ← bounded type, left', right']))
  | Eq domain left right =>
      let_expr Eq domain' left' right' := caller | different "an equation" installed caller
      if domain == domain' then return ← congruence installed caller
      let_expr NTy.carrier _ type := domain
        | throwError "an equation at `{← spelled domain}` has no law"
      let first ← movedOperand type left left'
      let second ← movedOperand type right right'
      let moved ← mkCongr (← mkCongrArg (mkApp installed.getAppFn domain) first.proof) second.proof
      let equivalence ← law ``Skolems.reframeAtInstance_inj #[type, ← bounded type, left', right']
      let_expr Eq _ _ atMoved := ← inferType moved | throwError "internal"
      result (← mkEqTrans moved (mkApp3 (mkConst ``propext) atMoved caller equivalence))
  | Array.size _ elements =>
      let_expr Array.size _ elements' := caller | different "a size" installed caller
      let_expr VerexIR.SpecVector.values _ vector := elements | congruence installed caller
      let_expr VerexIR.SpecVector.values _ vector' := elements'
        | different "a size" installed caller
      if vector == vector' then return ← congruence installed caller
      let whole ← movedValue vector vector'
      let_expr NTy.vector element := ← whnfD whole.type
        | throwError "the size of a value of type {← spelled whole.type}, which is no vector, has no law"
      let vectorType := mkApp (mkConst ``NTy.vector) element
      let (_, moved) ← congruentAt installed vector whole.moved whole.proof
      result (← mkEqTrans moved (← law ``Skolems.reframeAtInstance_vector_size
        #[element, ← bounded vectorType, vector']))
  | VerexIR.SpecMap.Valid keys values discipline map =>
      let_expr VerexIR.SpecMap.Valid keys' values' discipline' map' := caller
        | different "a map's validity" installed caller
      let_expr VerexIR.SpecMap.ofEntries _ _ _ _ readKey readValue table := map
        | throwError "the validity of `{operatorOf map}`, which is no table's map, has no law"
      let_expr VerexIR.SpecMap.ofEntries _ _ _ _ readKey' readValue' table' := map'
        | different "a map's validity" installed caller
      let whole ← movedValue table table'
      let_expr NTy.table key value := ← whnfD whole.type
        | throwError "the map of a value of type {← spelled whole.type}, which is no table, has no law"
      let tableType := mkApp2 (mkConst ``NTy.table) key value
      let (atMoved, moved) ← congruentAt installed table whole.moved whole.proof
      let equivalence ← match_expr discipline with
        | VerexIR.SpecMap.Discipline.sequence _ =>
            unless discipline'.isAppOfArity ``VerexIR.SpecMap.Discipline.sequence 1 do
              different "a map's discipline" discipline discipline'
            law ``Skolems.valid_sequence_reframeAtInstance
              #[key, value, ← bounded tableType, table', keys, values, keys', values',
                readKey, readValue, readKey', readValue', ← injective readKey, ← injective readKey']
        | VerexIR.SpecMap.Discipline.ordered _ order =>
            let_expr VerexIR.SpecMap.Discipline.ordered _ order' := discipline'
              | different "a map's discipline" discipline discipline'
            -- The installed map is keyed by the native keys under their
            -- structural order, or by the family's typed read of the key
            -- type under the family's order of the typed keys.
            let (name, rank) ← match_expr order with
              | NTy.keyOrder _ rank _ =>
                  unless readKey matches .lam _ _ (.bvar 0) _ do
                    throwError "the key reader `{← spelled readKey}` of the installed ordered \
                      map has no law"
                  pure (``Skolems.valid_ordered_reframeAtInstance_keyOrder, rank)
              | NTy.readOrder _ rank _ =>
                  unless readKey.isAppOfArity ``NTy.read 2 do
                    throwError "the key reader `{← spelled readKey}` of the installed ordered \
                      map has no law"
                  pure (``Skolems.valid_ordered_reframeAtInstance_read, rank)
              | _ => throwError "the key order `{← spelled order}` of the installed ordered map \
                  has no law"
            let applied ← law name
              #[key, value, ← bounded tableType, table', values, keys', values', rank,
                readValue, readKey', readValue', order']
            let .forallE _ premise _ _ ← whnfR (← inferType applied)
              | throwError "internal: the ordered law takes no order premise"
            -- The caller's order of the keys it reads is its structural order
            -- of the keys: through the typed view by the family's coherence
            -- law (`NTy.readOrder_read_order`), and otherwise by computation.
            let reads ← if readKey'.isAppOfArity ``NTy.read 2 &&
                order'.isAppOfArity ``NTy.readOrder 3 then
              forallTelescope premise fun binders body => do
                let coherent := mkAppN (mkConst ``NTy.readOrder_read_order)
                  #[readKey'.getArg! 0, order'.getArg! 1, readKey'.getArg! 1, binders[0]!, binders[1]!]
                mkLambdaFVars binders (← mkExpectedTypeHint coherent body)
            else
              byComputation premise m!"an ordered table: the premise `{premise}`, that the \
                caller's order of the keys it reads is its structural order of the keys, does not \
                hold by computation"
            pure (mkApp applied reads)
        | _ => throwError "the discipline `{← spelled discipline}` of a table's map has no law"
      result (← mkEqTrans moved (mkApp3 (mkConst ``propext) atMoved caller equivalence))
  | Skolems.reframeAtInstance _ _ _ _ _ _ _ _ _ _ _ _ => return some (← closedValue installed caller)
  | _ => congruence installed caller

/-- The same operator at arguments that agree: an operator no frame reads
applied on both sides, its type and instance arguments converting and its
other arguments agreeing in turn. -/
partial def congruence (installed caller : Lean.Expr) : BuildM (Option Lean.Expr) := do
  let function := installed.getAppFn
  let arguments := installed.getAppArgs
  let arguments' := caller.getAppArgs
  unless function == caller.getAppFn && arguments.size == arguments'.size &&
      (function.isConst || function.isFVar) && !arguments.isEmpty do
    throwError "no law relates `{operatorOf installed}` at the declaration's frame to \
      `{operatorOf caller}` at the caller's: `{← spelled installed}` and `{← spelled caller}`"
  let mut applied := function
  let mut proof? : Option Lean.Expr := none
  for argument in arguments, argument' in arguments' do
    let same ← if argument == argument' then pure none else do
      let type ← whnf (← inferType argument)
      if (type.isSort && !type.isProp) || (← isProp type) || (← isClass? type).isSome then
        -- A type, a proof or an instance is the caller's by conversion.
        unless ← withNewMCtxDepth (isDefEq argument argument') do
          throwError "`{operatorOf installed}` is applied at `{← spelled argument}` at the \
            declaration's frame and at `{← spelled argument'}` at the caller's, which no law relates"
        pure none
      else
        -- An argument without agreement is reported under the operator it
        -- is an argument of.
        let same ← (try proved argument argument' catch error =>
          throwError "under `{operatorOf installed}`: {error.toMessageData}")
        pure (some same)
    proof? ← match proof?, same with
      | none, none => pure none
      | some proof, none => some <$> mkCongrFun proof argument
      | none, some same => some <$> mkCongrArg applied same
      | some proof, some same => some <$> mkCongr proof same
    applied := mkApp applied argument
  match proof? with
  | none => return some (← stated (← mkEqRefl installed) installed caller)
  | some proof => return some (← stated proof installed caller)

/-- The equation of the two terms, the reflexive one where they are the same. -/
partial def proved (installed caller : Lean.Expr) : BuildM Lean.Expr := do
  match ← frameFree installed caller with
  | some proof => pure proof
  | none => mkEqRefl installed.consumeMData.headBeta

end

/-- The constants of a caller's reading that name another frame than the
caller's, or a move: a right-hand side that names one is not in a leaf's form. -/
def foreignFrames : Array Name := #[
  ``Skolems.reframeAtInstance, ``Skolems.reframeAtBase, ``Skolems.reframe,
  ``Skolems.atArguments, ``Skolems.instantiate, ``Skolems.runtime, ``Carriers.runtime,
  ``Carriers.instantiate, ``Sorts.runtime, ``Sorts.instantiate, ``FunctionCarriers.runtime]

/-- The constants of a type a substitution or a resolution was not evaluated
in: a descriptor a leaf does not spell. -/
def unevaluatedTypes : Array Name := #[
  ``NTy.subst, ``NRow.subst, ``NRows.subst, ``NTy.unmark, ``NRow.getD, ``NRow.map,
  ``Skolems.resolve, ``NTy.substWith]

/-- Whether a key reader of a table's map is the typed view of the key's type
at the family `carriers`: an integer's value, the family's read of a type
parameter in constructor form (`NTy.read carriers (NTy.param i)`), or the
identity at a type that is neither an integer nor a type parameter. -/
def typedViewReader (carriers reader : Lean.Expr) : Bool :=
  let operations := mkApp (mkConst ``Carriers.toSorts) carriers
  reader.isAppOfArity ``VerexIR.SpecInt.val 2 ||
  (reader.isAppOfArity ``NTy.read 2 && reader.getArg! 0 == operations &&
    (reader.getArg! 1).isAppOfArity ``NTy.param 1) ||
  (match reader with
    | .lam _ domain (.bvar 0) _ =>
        domain.isAppOfArity ``NTy.carrier 2 && domain.getArg! 0 == operations &&
          !((domain.getArg! 1).isAppOf ``NTy.param || (domain.getArg! 1).isAppOf ``NTy.int ||
            (domain.getArg! 1).isConstOf ``NTy.num)
    | _ => false)

/-- What the form check reads a term against: the unit's generated
definitions, whose bodies are part of the term, and its base function values. -/
structure FormCheck where
  generated : Name → Bool
  base : Name

abbrev FormM := StateRefT (Std.HashSet (Lean.Expr × Lean.Expr)) MetaM

mutual
/-- Check every node of a term at the family `carriers`: no constant of
another frame, of a move, of the base function values or of an unevaluated
type; no family but `carriers`; every table's map keyed by the typed view at
`carriers`.  An application of a generated definition is checked as the
definition's body at the application's arguments, so that what a helper
states is checked exactly as if it stood inline, the frame it is applied at
included.  `expanding` are the definitions being read, none of which is read
inside itself. Binders are opened, so every node is checked closed. -/
partial def checkFormAt (context : FormCheck) (expanding : List Name)
    (carriers term : Lean.Expr) : FormM Unit := do
  if (← get).contains (carriers, term) then return
  modify (·.insert (carriers, term))
  match term with
  | .mdata _ inner | .proj _ _ inner => checkFormAt context expanding carriers inner
  | .lam name domain body info | .forallE name domain body info =>
      checkFormAt context expanding carriers domain
      withLocalDecl name info domain fun bound =>
        checkFormAt context expanding carriers (body.instantiate1 bound)
  | .letE _ type value body _ =>
      checkFormAt context expanding carriers type
      checkFormAt context expanding carriers value
      checkFormAt context expanding carriers (body.instantiate1 value)
  | .const .. | .app .. =>
      if term.isAppOfArity ``Skolems.toCarriers 3 && term != carriers then
        throwError "the caller's reading is not in a leaf's form: it names the family `{term}`"
      if term.isAppOfArity ``VerexIR.SpecMap.ofEntries 7 &&
          !typedViewReader carriers (term.getArg! 4) then
        throwError "the caller's reading is not in a leaf's form: the key reader \
          `{term.getArg! 4}` of a table's map is not the caller's typed view"
      let arguments := term.getAppArgs
      match term.getAppFn with
      | .const name levels =>
          if name == context.base || foreignFrames.contains name ||
              unevaluatedTypes.contains name then
            throwError "the caller's reading is not in a leaf's form: it names `{name}`"
          if context.generated name && !expanding.contains name then
            if let some info := (← getEnv).find? name then
              if let some value := info.value? then
                let body := (value.instantiateLevelParams info.levelParams levels).beta arguments
                try checkDefinitionAt context (name :: expanding) carriers body
                catch error =>
                  throwError "in the generated definition `{name}`: {error.toMessageData}"
      | function => checkFormAt context expanding carriers function
      for argument in arguments do checkFormAt context expanding carriers argument
  | _ => pure ()

/-- A generated definition's body at the arguments it is applied to.  The
binders its application leaves open are the definition's own: under one that
is a frame, the body is checked at that frame's family, not at the family of
the term that mentions the definition, which that frame is not. -/
partial def checkDefinitionAt (context : FormCheck) (expanding : List Name)
    (carriers body : Lean.Expr) : FormM Unit := do
  let .lam name domain inner info := body | checkFormAt context expanding carriers body
  checkFormAt context expanding carriers domain
  withLocalDecl name info domain fun bound => do
    let carriers := if domain.isAppOfArity ``Skolems 2 then
        mkApp3 (mkConst ``Skolems.toCarriers) (domain.getArg! 0) (domain.getArg! 1) bound
      else carriers
    checkDefinitionAt context expanding carriers (inner.instantiate1 bound)
end

/-- Check the form of a caller's reading, as a leaf holds it: it names one
frame, the caller's at the unit's function values (`carriers`), no move, and
no base function values (`base`); every type in it is evaluated; and the key
reader of every table's map is the caller's typed view of the key's type
(`typedViewReader`).  The unit's generated definitions the reading applies
(`generated`) are read through: each is checked as its body at the arguments
of the application, at any depth (`checkFormAt`).  Throws with the first
offending term. -/
def checkOutputForm (generated : Name → Bool) (base : Name) (carriers reading : Lean.Expr) :
    MetaM Unit :=
  (checkFormAt { generated, base } [] carriers reading).run' {}

/-- What a bridge is built of: the theorem's statement and proof, the key the
solver reads it by, and the number of clauses. -/
structure Built where
  statement : Lean.Expr
  proof : Lean.Expr
  source : Lean.Expr
  row : Lean.Expr
  /-- The declaration's parameter row and the instance type the statement is
  at, both closed: the application descriptor the registry records. -/
  params : Lean.Expr
  instanceType : Lean.Expr
  clauses : Nat

/-- The clause the construction is in, for a report. -/
structure Stopped where
  source : Lean.Expr
  row : Lean.Expr
  clause : String
  reason : String

/-- Build the bridge of the declaration `owner` of a unit at the instance row
`row`. `unitName`, `functionsName` and `baseName` are the unit's constant, its
function values and its base function values; `installed` is the constant of
the declaration's installed clauses (`<unit>.dataOwn.case{i}`). -/
def build (unit : ValidatedUnit) (unitName functionsName baseName installed : Name)
    (generated : Name → Bool) (owner : VerexIR.StructHandle) (row : NRow) :
    MetaM (Except Stopped Built) := do
  let some declared := structNTy unit owner
    | throwError "internal: a declaration with a data invariant has no native type"
  let unitExpr := mkConst unitName
  let functions := mkConst functionsName
  let base := mkConst baseName
  let declaredType ← quoteNTy declared
  let instanceType ← quoteNTy (declared.subst row)
  let view ← nativeTypeView instanceType
  let stopped (clause reason : String) : Stopped :=
    { source := view.getArg! 0, row := view.getArg! 1, clause, reason }
  let params ← quoteRow row
  let field (name : Name) (carriers : Lean.Expr) : Lean.Expr := mkApp2 (mkConst name) unitExpr carriers
  let baseCarrier := field ``FunctionCarriers.carrier base
  let baseFunctions := field ``FunctionCarriers.functions base
  let baseNominals := field ``FunctionCarriers.nominals base
  let sameCarrier ← mkExpectedTypeHint (← mkEqRefl baseCarrier)
    (← mkEq (field ``FunctionCarriers.carrier functions) baseCarrier)
  let sameFunctions ← mkExpectedTypeHint (← mkHEqRefl baseFunctions)
    (← mkHEq (field ``FunctionCarriers.functions functions) baseFunctions)
  let sameNominals ← mkExpectedTypeHint (← mkEqRefl baseNominals)
    (← mkEq (field ``FunctionCarriers.nominals functions) baseNominals)
  let installedInfo ← getConstInfoDefn installed
  withLocalDeclD `Θ (mkApp2 (mkConst ``Skolems) unitExpr functions) fun frame => do
  let carriers := mkApp3 (mkConst ``Skolems.toCarriers) unitExpr functions frame
  let resolutions := mkApp2 (mkConst ``NRow.map)
    (mkApp3 (mkConst ``Skolems.resolve) unitExpr functions frame) params
  let inhabitableType ← mkEq (mkApp (mkConst ``NRow.inhabitable) resolutions) (mkConst ``Bool.true)
  withLocalDeclD `inhabitable inhabitableType fun inhabitable => do
  withLocalDeclD `value (carrierType carriers instanceType) fun value => do
    let frames : Frames := { leading :=
      #[unitExpr, functions, base, sameCarrier, sameFunctions, sameNominals, frame, params, inhabitable] }
    let clauses ← ownClausesAt unit owner row frame value
    if clauses.isEmpty then
      return .error (stopped "" "the declaration states no clause at this row")
    -- The installed clauses at the row's resolutions over the moved value.
    let predicate ← withLocalDeclD `row (mkConst ``NRow) fun bound => do
      mkLambdaFVars #[bound]
        (← mkEq (mkApp (mkConst ``NRow.inhabitable) bound) (mkConst ``Bool.true))
    let typeArguments ← mkAppOptM ``Subtype.mk
      #[some (mkConst ``NRow), some predicate, some resolutions, some inhabitable]
    let moved ← (moveOf declaredType value).run frames
    let installedReading := (installedInfo.value.beta #[typeArguments, moved])
    -- The agreement, clause by clause: both readings are conjunctions of the
    -- same marked clauses in the same order.
    let reading ← conjunction (clauses.map fun (term, range) => markObligation range term)
    let rec parts (conjunction : Lean.Expr) (count : Nat) : List Lean.Expr :=
      match count, conjunction with
      | count + 2, .app (.app (.const ``And _) left) right => parts left (count + 1) ++ [right]
      | _, whole => [whole]
    let installedParts := parts installedReading clauses.size
    unless installedParts.length == clauses.size do
      return .error (stopped "" "the installed clauses and the caller's differ in number")
    let mut agreements : Array Lean.Expr := #[]
    for installedClause in installedParts, (term, range) in clauses do
      let callerClause := markObligation range term
      let agreed : Except String Lean.Expr ← (try
          Except.ok <$> (proved installedClause callerClause).run frames
        catch error => pure (Except.error (← error.toMessageData.toString)))
      match agreed with
      | .ok proof => agreements := agreements.push proof
      | .error reason =>
          return .error (stopped s!"{range.file}:{range.startByte}-{range.endByte}" reason)
    let mut agreement := agreements[0]!
    for next in agreements.extract 1 agreements.size do
      agreement ← mkCongr (← mkCongrArg (mkConst ``And) agreement) next
    let agree ← mkAppM ``Iff.of_eq #[← stated agreement installedReading reading]
    let caller ← mkLambdaFVars #[value] reading
    -- The declaration's own row is its parameters, which the row substitutes
    -- to itself.
    let bridged : Except String (Lean.Expr × Lean.Expr) ← (try
      let declaredView ← whnfD declaredType
      let arguments := declaredView.getArg! 1
      let substituted := mkApp2 (mkConst ``NRow.subst) params arguments
      unless ← withNewMCtxDepth (isDefEq substituted params) do
        throwError "the row is not what the declaration's arguments substitute to: an entry \
          carries a reference mark"
      let ownRow ← mkExpectedTypeHint (← mkEqRefl params) (← mkEq substituted params)
      let installedOwn ← withLocalDeclD `arguments (mkConst ``NRow) fun arguments' =>
        withLocalDeclD `encoded (mkConst ``VerexIR.RuntimeValue) fun encoded => do
          let left := mkAppN (mkConst ``FunctionCarriers.own)
            #[unitExpr, functions, declaredView.getArg! 0, arguments', encoded]
          let right := mkAppN (mkConst ``OwnAtArguments)
            #[unitExpr, base, declaredView, mkConst installed, arguments', encoded]
          unless ← withNewMCtxDepth (isDefEq left right) do
            throwError "the unit's function values do not install `{installed}` as the \
              declaration's own invariant"
          mkLambdaFVars #[arguments', encoded]
            (← mkExpectedTypeHint (← mkEqRefl right) (← mkEq left right))
      let (name, fields) ←
        if declaredView.isAppOfArity ``NTy.struct 3 && view.isAppOfArity ``NTy.struct 3 then
          pure (``NTy.holds_struct_bridge,
            mkApp3 (mkConst ``HList.Holds) carriers (view.getArg! 2) value)
        else if declaredView.isAppOfArity ``NTy.enum 5 && view.isAppOfArity ``NTy.enum 5 then
          pure (``NTy.holds_enum_bridge,
            mkApp4 (mkConst ``variantCarrier.Holds) carriers (view.getArg! 2) (view.getArg! 3) value)
        else throwError "the declaration's type {declaredType} is neither a struct nor an enum"
      let proof ← (do
        let over ← bounded declaredView
        law name (declaredView.getAppArgs ++ #[over, ownRow, mkConst installed, installedOwn,
          caller, value, agree])).run frames
      let statement ← mkAppM ``Iff
        #[mkApp3 (mkConst ``NTy.Holds) carriers instanceType value, ← mkAppM ``And #[reading, fields]]
      let proof ← mkExpectedTypeHint proof statement
      Meta.check proof
      checkOutputForm generated baseName carriers reading
      pure (Except.ok (statement, proof))
    catch error => pure (Except.error (← error.toMessageData.toString)))
    match bridged with
    | .error reason => return .error (stopped "" reason)
    | .ok (statement, proof) =>
        let binders := #[frame, inhabitable, value]
        return .ok {
          statement := ← mkForallFVars binders statement
          proof := ← mkLambdaFVars binders proof
          source := view.getArg! 0, row := view.getArg! 1, params, instanceType
          clauses := clauses.size }

end VerexSL.Verify.OwnBridge
