-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Contract
import VerexSL.Modules
import VerexIR.Proofs.Denote.Agreement
import VerexIR.Proofs.Denote.FamilyTransport

/-!
# Verification: compiled functions, contracts, and loop invariants

The first part of `verify` (`VerexSL.Verify`): quotation of compiled
functions, the names of published artifacts, the typed contracts of a
target and its callees, and the conditions of its loops. It does not import
the solver, so an edit of the solver does not rebuild it.
-/

namespace VerexSL.Verify

open Lean Meta Elab Command
open VerexIR (FunctionHandle NamespaceId FunctionId)
open VerexIR.Validation (ValidatedUnit ValidatedNamespace)
open VerexIR.Proofs.Denote
open VerexSL.Contract

/-- Only the verifier can mark a completed artifact family for cache reuse.
The tag is serialized with its declaration for downstream modules. -/
initialize completedDenotations : TagDeclarationExtension ← mkTagDeclarationExtension

/-- A verification whose artifacts passed the audit: what a caller's proof
uses a callee's theorem by. A failed target keeps the theorem its proof
task committed, which proves nothing. -/
initialize auditedDenotations : TagDeclarationExtension ← mkTagDeclarationExtension

/-- The natives a verified function's theorems assume, keyed by its artifact
base: each native's handle and name. A caller assumes them as well. -/
initialize nativeDependencies :
    SimplePersistentEnvExtension (Name × Array (FunctionHandle × String))
      (NameMap (Array (FunctionHandle × String))) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := fun map (name, natives) => map.insert name natives
    addImportedFn := fun entries =>
      mkStateFromImportedEntries (fun map (name, natives) => map.insert name natives) {} entries }

/-- The functions whose in-body assumptions a verified function's theorems
assume to hold (`AssumptionsHold`), keyed by its artifact base: each
function's handle and name. A caller assumes them as well. -/
initialize assumptionDependencies :
    SimplePersistentEnvExtension (Name × Array (FunctionHandle × String))
      (NameMap (Array (FunctionHandle × String))) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := fun map (name, functions) => map.insert name functions
    addImportedFn := fun entries =>
      mkStateFromImportedEntries (fun map (name, functions) => map.insert name functions) {}
        entries }

/-- The assumed steps of lemmas a verified function's theorems take as
hypotheses (`L.lemmaTrusted_k`), keyed by its artifact base. A caller using
its theorems takes them as well. -/
initialize lemmaTrustDependencies :
    SimplePersistentEnvExtension (Name × Array Name) (NameMap (Array Name)) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := fun map (name, trusted) => map.insert name trusted
    addImportedFn := fun entries =>
      mkStateFromImportedEntries (fun map (name, trusted) => map.insert name trusted) {} entries }

deriving instance ToExpr for CheckedOp
deriving instance ToExpr for ModularOp
deriving instance ToExpr for CompareOp
deriving instance ToExpr for BitOp
deriving instance ToExpr for FunctionHandle
deriving instance ToExpr for VerexIR.TypeId
deriving instance ToExpr for VerexIR.LocId
deriving instance ToExpr for VerexIR.TypeUse
deriving instance ToExpr for VerexIR.Validation.Witness
deriving instance ToExpr for VerexIR.Validation.KeyMap
deriving instance ToExpr for VerexIR.Validation.IndexedArena
deriving instance ToExpr for VerexIR.LifetimeKind
deriving instance ToExpr for VerexIR.StructHandle

attribute [vir_denote_norm] VerexIR.moveArithmeticError
  VerexIR.SemanticOperations.resolveReturnedBorrows_empty
  VerexIR.SemanticOperations.resolveReturnedBorrows_integer

/-! ## Quotation of compiled functions -/

mutual
/-- The literal of a native value. -/
partial def quoteCarrier : (τ : NTy) → τ.groundCarrier → MetaM Lean.Expr
  | .tuple elements, value => quoteRowValue elements value
  | .struct _ _ fields, value => quoteRowValue fields value
  | .enum _ _ names rows _, value => quoteVariantValue names rows value
  | .vector element, value => do
      let carrier := mkApp (mkConst ``NTy.groundCarrier) (← quoteNTy element)
      let values ← mkArrayLit carrier (← value.values.toList.mapM (quoteCarrier element))
      let bounded ← mkDecideProof (← mkAppM ``LT.lt
        #[← mkAppM ``Array.size #[values], mkNatLit (2 ^ 64)])
      mkAppM ``VerexIR.SpecVector.mk #[values, bounded]
  | .table key value, entries => do
      let pair := mkApp2 (mkConst ``Prod [.zero, .zero])
        (mkApp (mkConst ``NTy.groundCarrier) (← quoteNTy key))
        (mkApp (mkConst ``NTy.groundCarrier) (← quoteNTy value))
      let list ← mkListLit pair (← entries.entries.mapM fun entry => do
        mkAppM ``Prod.mk #[← quoteCarrier key entry.1, ← quoteCarrier value entry.2])
      let bounded ← mkDecideProof (← mkAppM ``LT.lt
        #[← mkAppM ``List.length #[list], mkNatLit (2 ^ 64)])
      mkAppM ``VerexIR.SpecEntries.mk #[list, bounded]
  | .ref referent, value => do
      mkAppM ``Prod.mk #[← quoteCarrier referent value.1, ← quoteCarrier referent value.2]
  | .ghost slot, value => quoteCarrier slot value
  | .int width signed, value => do
      let widthExpr := toExpr (VerexIR.IntWidth.bits width)
      let proposition ← mkAppM ``VerexIR.IntegerValueFits
        #[widthExpr, toExpr signed, toExpr value.val]
      let fits ← mkDecideProof proposition
      return mkAppN (mkConst ``VerexIR.SpecInt.mk)
        #[widthExpr, toExpr signed, toExpr value.val, fits]
  | .num, value => do
      let widthExpr := toExpr VerexIR.IntWidth.unbounded
      let proposition ← mkAppM ``VerexIR.IntegerValueFits
        #[widthExpr, toExpr true, toExpr value.val]
      let fits ← mkDecideProof proposition
      return mkAppN (mkConst ``VerexIR.SpecInt.mk)
        #[widthExpr, toExpr true, toExpr value.val, fits]
  | .bool, value => return toExpr value
  | .unit, _ => return mkConst ``Unit.unit
  | .address, value => return toExpr value
  | .signer, value => return toExpr value
  | .string, value => return toExpr value
  | .bytes, value => return toExpr value
  | .param _, _ => throwError "a literal of a type parameter"
  | .function _ _ _, _ => throwError "a literal of a function type"

partial def quoteRowValue : (row : NRow) → @HList Carriers.ground row → MetaM Lean.Expr
  | .nil, _ => return mkConst ``Unit.unit
  | .cons τ rest, value => do
      mkAppM ``Prod.mk #[← quoteCarrier τ value.1, ← quoteRowValue rest value.2]

partial def quoteVariantValue : (names : List String) → (rows : NRows) →
    @variantCarrier Carriers.ground names rows → MetaM Lean.Expr
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | _ :: names, .cons fields rest, value => do
      let ground := mkConst ``Carriers.ground
      let left := mkApp2 (mkConst ``HList) ground (← quoteRow fields)
      let right := mkAppN (mkConst ``variantCarrier) #[ground, toExpr names, ← quoteRows rest]
      match value with
      | .inl fields => mkAppOptM ``Sum.inl #[left, right, ← quoteRowValue _ fields]
      | .inr later => mkAppOptM ``Sum.inr #[left, right, ← quoteVariantValue names rest later]
end

def quotePlaceIndex : PlaceIndex → Lean.Expr
  | .literal index => mkApp (mkConst ``PlaceIndex.literal) (toExpr index)
  | .slot slot => mkApp (mkConst ``PlaceIndex.slot) (toExpr slot)
  | .fromEnd offset => mkApp (mkConst ``PlaceIndex.fromEnd) (toExpr offset)

def quoteProj : {τ σ : NTy} → Proj τ σ → MetaM Lean.Expr
  | τ, _, .nil => return mkApp (mkConst ``Proj.nil) (← quoteNTy τ)
  | .vector τ, σ, .index position rest =>
      return mkAppN (mkConst ``Proj.index)
        #[← quoteNTy τ, ← quoteNTy σ, quotePlaceIndex position, ← quoteProj rest]
  | .ref τ, σ, .deref rest =>
      return mkAppN (mkConst ``Proj.deref) #[← quoteNTy τ, ← quoteNTy σ, ← quoteProj rest]
  | .struct source arguments fields, τ, @Proj.field _ _ _ σ _ x rest =>
      return mkAppN (mkConst ``Proj.field) #[Lean.toExpr source, ← quoteRow arguments,
        ← quoteRow fields, ← quoteNTy σ, ← quoteNTy τ, ← quoteVar x, ← quoteProj rest]
  | .ghost τ, σ, .ghost rest =>
      return mkAppN (mkConst ``Proj.ghost) #[← quoteNTy τ, ← quoteNTy σ, ← quoteProj rest]
  | .enum source arguments names rows _, τ, @Proj.variant _ _ _ _ _ σ _ choices rest => do
      let namesExpr := toExpr names
      let distinct ← mkDecideProof (← mkAppM ``List.Nodup #[namesExpr])
      return mkAppN (mkConst ``Proj.variant) #[Lean.toExpr source, ← quoteRow arguments,
        namesExpr, ← quoteRows rows, distinct, ← quoteNTy σ, ← quoteNTy τ, ← quoteChoices choices,
        ← quoteProj rest]

def quoteVars {Γ : NRow} : {σs : NRow} → Vars Γ σs → MetaM Lean.Expr
  | _, .nil => return mkApp (mkConst ``Vars.nil) (← quoteRow Γ)
  | .cons σ σs, .cons target rest => do
      let varType := mkAppN (mkConst ``Var) #[← quoteRow Γ, ← quoteNTy σ]
      let target ← match target with
        | some x => mkAppOptM ``Option.some #[varType, ← quoteVar x]
        | none => pure (mkApp (mkConst ``Option.none [Level.zero]) varType)
      return mkAppN (mkConst ``Vars.cons)
        #[← quoteRow Γ, ← quoteNTy σ, ← quoteRow σs, target, ← quoteVars rest]

mutual
partial def quotePat {Γ : NRow} : {τ : NTy} → Pat Γ τ → MetaM Lean.Expr := fun {τ} pattern => do
  let context ← quoteRow Γ
  match τ, pattern with
  | τ, .wildcard => return mkAppN (mkConst ``Pat.wildcard) #[context, ← quoteNTy τ]
  | τ, .var x => return mkAppN (mkConst ``Pat.var) #[context, ← quoteNTy τ, ← quoteVar x]
  | τ, .literal value =>
      return mkAppN (mkConst ``Pat.literal) #[context, ← quoteNTy τ, ← quoteCarrier τ value]
  | .int width signed, .range lower upper inclusive =>
      return mkAppN (mkConst ``Pat.range) #[context, Lean.toExpr width, Lean.toExpr signed,
        Lean.toExpr lower, Lean.toExpr upper, Lean.toExpr inclusive]
  | .tuple σs, .tuple elements =>
      return mkAppN (mkConst ``Pat.tuple) #[context, ← quoteRow σs, ← quotePats elements]
  | .struct source arguments σs, .struct fields =>
      return mkAppN (mkConst ``Pat.struct)
        #[context, Lean.toExpr source, ← quoteRow arguments, ← quoteRow σs, ← quotePats fields]
  | .enum source arguments names rows _, @Pat.variant _ _ _ _ _ _ σs choice fields => do
      let namesExpr := Lean.toExpr names
      let distinct ← mkDecideProof (← mkAppM ``List.Nodup #[namesExpr])
      return mkAppN (mkConst ``Pat.variant) #[context, Lean.toExpr source, ← quoteRow arguments,
        namesExpr, ← quoteRows rows, distinct, ← quoteRow σs, ← quoteWhich choice,
        ← quotePats fields]

partial def quotePats {Γ : NRow} : {σs : NRow} → Pats Γ σs → MetaM Lean.Expr :=
  fun {σs} patterns => do
    let context ← quoteRow Γ
    match σs, patterns with
    | _, .nil => return mkApp (mkConst ``Pats.nil) context
    | .cons σ σs, .cons head tail =>
        return mkAppN (mkConst ``Pats.cons)
          #[context, ← quoteNTy σ, ← quoteRow σs, ← quotePat head, ← quotePats tail]
end

/-- A closed fact the kernel establishes by evaluating its statement, as an
auxiliary theorem: a constant, so that no normalization exposes the
`Eq.refl` behind it, whose inferred type would make every comparison with the
original evaluate the statement again in the elaborator. -/
def kernelFact (statement proof : Lean.Expr) : MetaM Lean.Expr :=
  mkAuxTheorem statement proof

/-- A proof that a closed boolean evaluates to `true`, by the kernel's
evaluation. -/
def kernelTrue (condition : Lean.Expr) : MetaM Lean.Expr := do
  kernelFact (← mkEq condition (mkConst ``Bool.true)) (← mkEqRefl (mkConst ``Bool.true))

/-- A proof that a closure target's supplied parameters and results are
shared where the given marks say, by the kernel's evaluation. -/
def closureSharingProof (unitExpr : Lean.Expr) (handle : FunctionHandle)
    (fullExpr capturedExpr suppliedExpr weaveExpr : Lean.Expr) (shared : Sharing) :
    MetaM Lean.Expr := do
  let read := mkAppN (mkConst ``closureShared) #[unitExpr, Lean.toExpr handle,
    mkAppN (mkConst ``Weave.mask) #[fullExpr, capturedExpr, suppliedExpr, weaveExpr]]
  let flags := Lean.toExpr shared
  kernelFact (← mkEq read flags) (← mkEqRefl flags)

/-- A proof that a closure target's own rows are the given ones, by the
kernel's evaluation. -/
def closureRowsProof (unitExpr : Lean.Expr) (handle : FunctionHandle)
    (fullExpr capturedExpr suppliedExpr weaveExpr resultsExpr : Lean.Expr) : MetaM Lean.Expr := do
  let read := mkAppN (mkConst ``closureRows?) #[unitExpr, Lean.toExpr handle,
    mkAppN (mkConst ``Weave.mask) #[fullExpr, capturedExpr, suppliedExpr, weaveExpr]]
  let rows ← mkAppM ``Option.some
    #[← mkAppM ``Prod.mk #[capturedExpr, ← mkAppM ``Prod.mk #[suppliedExpr, resultsExpr]]]
  kernelFact (← mkEq read rows) (← mkEqRefl rows)

mutual
partial def quoteTerm {unit : ValidatedUnit} (unitExpr : Lean.Expr) {ρ : ResultShape} {Γ : NRow} :
    {τ : NTy} → Term unit ρ Γ τ →
    MetaM Lean.Expr := fun {τ} term => do
  let shape ← quoteShape ρ
  let context ← quoteRow Γ
  let toExpr := fun (τ : NTy) => quoteNTy τ
  match τ, term with
  | τ, .lit value =>
      return mkAppN (mkConst ``Term.lit) #[unitExpr, shape, context, ← toExpr τ, ← quoteCarrier τ value]
  | τ, .var x => return mkAppN (mkConst ``Term.var) #[unitExpr, shape, context, ← toExpr τ, ← quoteVar x]
  | .int width signed, .checked site op failure left right =>
      return mkAppN (mkConst ``Term.checked) #[unitExpr, shape, context, Lean.toExpr width,
        Lean.toExpr signed, Lean.toExpr site, Lean.toExpr op, Lean.toExpr failure, ← quoteTerm unitExpr left, ← quoteTerm unitExpr right]
  | .int width signed, .modular op left right =>
      return mkAppN (mkConst ``Term.modular) #[unitExpr, shape, context, Lean.toExpr width,
        Lean.toExpr signed, Lean.toExpr op, ← quoteTerm unitExpr left, ← quoteTerm unitExpr right]
  | .bool, @Term.compare _ _ _ width signed op left right =>
      return mkAppN (mkConst ``Term.compare) #[unitExpr, shape, context, Lean.toExpr width,
        Lean.toExpr signed, Lean.toExpr op, ← quoteTerm unitExpr left, ← quoteTerm unitExpr right]
  | .bool, @Term.equal _ _ _ σ negated left right =>
      return mkAppN (mkConst ``Term.equal) #[unitExpr, shape, context, ← toExpr σ, Lean.toExpr negated,
        ← quoteTerm unitExpr left, ← quoteTerm unitExpr right]
  | .bool, @Term.equalErased _ _ _ σ negated left right =>
      return mkAppN (mkConst ``Term.equalErased) #[unitExpr, shape, context, ← toExpr σ,
        Lean.toExpr negated, ← quoteTerm unitExpr left, ← quoteTerm unitExpr right]
  | .bool, .not operand =>
      return mkAppN (mkConst ``Term.not) #[unitExpr, shape, context, ← quoteTerm unitExpr operand]
  | .bool, .logical conjunction left right =>
      return mkAppN (mkConst ``Term.logical) #[unitExpr, shape, context, Lean.toExpr conjunction,
        ← quoteTerm unitExpr left, ← quoteTerm unitExpr right]
  | .int width false, .bitwise op left right =>
      return mkAppN (mkConst ``Term.bitwise) #[unitExpr, shape, context, Lean.toExpr width, Lean.toExpr op,
        ← quoteTerm unitExpr left, ← quoteTerm unitExpr right]
  | .int width false, @Term.shift _ _ _ _ distanceWidth site left failure value distance =>
      return mkAppN (mkConst ``Term.shift) #[unitExpr, shape, context, Lean.toExpr width,
        Lean.toExpr distanceWidth, Lean.toExpr site, Lean.toExpr left, Lean.toExpr failure, ← quoteTerm unitExpr value,
        ← quoteTerm unitExpr distance]
  | .int width' signed', @Term.cast _ _ _ width signed _ _ site failure value =>
      return mkAppN (mkConst ``Term.cast) #[unitExpr, shape, context, Lean.toExpr width, Lean.toExpr signed,
        Lean.toExpr width', Lean.toExpr signed', Lean.toExpr site, Lean.toExpr failure, ← quoteTerm unitExpr value]
  | τ, .ite site condition thenBranch elseBranch =>
      return mkAppN (mkConst ``Term.ite) #[unitExpr, shape, context, ← toExpr τ, Lean.toExpr site,
        ← quoteTerm unitExpr condition,
        ← quoteTerm unitExpr thenBranch, ← quoteTerm unitExpr elseBranch]
  | τ, @Term.let_ _ _ _ σ _ x value body =>
      return mkAppN (mkConst ``Term.let_) #[unitExpr, shape, context, ← toExpr σ, ← toExpr τ, ← quoteVar x,
        ← quoteTerm unitExpr value, ← quoteTerm unitExpr body]
  | τ, @Term.drop _ _ _ σ _ value body =>
      return mkAppN (mkConst ``Term.drop) #[unitExpr, shape, context, ← toExpr σ, ← toExpr τ,
        ← quoteTerm unitExpr value, ← quoteTerm unitExpr body]
  | .unit, @Term.assign _ _ _ σ x value =>
      return mkAppN (mkConst ``Term.assign) #[unitExpr, shape, context, ← toExpr σ, ← quoteVar x,
        ← quoteTerm unitExpr value]
  | τ, .const value =>
      return mkAppN (mkConst ``Term.const) #[unitExpr, shape, context, ← toExpr τ, ← quoteTerm unitExpr value]
  | τ, .throw0 site kind =>
      return mkAppN (mkConst ``Term.throw0) #[unitExpr, shape, context, ← toExpr τ,
        Lean.toExpr site, Lean.toExpr kind]
  | τ, @Term.throw1 _ _ _ _ width signed site kind code =>
      return mkAppN (mkConst ``Term.throw1) #[unitExpr, shape, context, ← toExpr τ,
        Lean.toExpr width, Lean.toExpr signed, Lean.toExpr site, Lean.toExpr kind,
        ← quoteTerm unitExpr code]
  | τ, .return_ value =>
      return mkAppN (mkConst ``Term.return_) #[unitExpr, shape, context, ← toExpr τ, ← quoteTerm unitExpr value]
  | τ, .break_ nest =>
      return mkAppN (mkConst ``Term.break_) #[unitExpr, shape, context, ← toExpr τ, Lean.toExpr nest]
  | τ, .continue_ nest =>
      return mkAppN (mkConst ``Term.continue_) #[unitExpr, shape, context, ← toExpr τ, Lean.toExpr nest]
  | .unit, .loop site body =>
      return mkAppN (mkConst ``Term.loop) #[unitExpr, shape, context, Lean.toExpr site, ← quoteTerm unitExpr body]
  | .unit, .assume site =>
      return mkAppN (mkConst ``Term.assume) #[unitExpr, shape, context, Lean.toExpr site]
  | τ, .arbitrary =>
      return mkAppN (mkConst ``Term.arbitrary) #[unitExpr, shape, context, ← toExpr τ]
  | τ, .ghostValue site =>
      return mkAppN (mkConst ``Term.ghostValue) #[unitExpr, shape, context, ← toExpr τ, Lean.toExpr site]
  | τ, @Term.ghostInit _ _ _ _ σ site path value =>
      return mkAppN (mkConst ``Term.ghostInit) #[unitExpr, shape, context, ← toExpr τ, ← toExpr σ,
        Lean.toExpr site, ← quoteProj path, ← quoteTerm unitExpr value]
  | .ghost τ, .ghostSlot value =>
      return mkAppN (mkConst ``Term.ghostSlot) #[unitExpr, shape, context, ← toExpr τ, ← quoteTerm unitExpr value]
  | τ, .ghostRead slot =>
      return mkAppN (mkConst ``Term.ghostRead) #[unitExpr, shape, context, ← toExpr τ, ← quoteTerm unitExpr slot]
  | _, @Term.call _ _ _ σs site handle calleeShape arguments =>
      return mkAppN (mkConst ``Term.call) #[unitExpr, shape, context, ← quoteRow σs, Lean.toExpr site,
        Lean.toExpr handle, ← quoteShape calleeShape, ← quoteArgs unitExpr arguments]
  | _, @Term.callGeneric _ _ _ σs site handle typeArgs θ calleeShape arguments => do
      return mkAppN (mkConst ``Term.callGeneric) #[unitExpr, shape, context, ← quoteRow σs,
        Lean.toExpr site, Lean.toExpr handle, Lean.toExpr typeArgs, ← quoteTypeArgs θ, ← quoteShape calleeShape,
        ← quoteArgs unitExpr arguments]
  | _, @Term.closure _ _ _ full captured supplied handle weave results shared _ _ _ _ captures => do
      let (fullExpr, capturedExpr, suppliedExpr) :=
        (← quoteRow full, ← quoteRow captured, ← quoteRow supplied)
      let weaveExpr ← quoteWeave weave
      let resultsExpr ← quoteRow results
      let rowsProof ← closureRowsProof unitExpr handle fullExpr capturedExpr suppliedExpr
        weaveExpr resultsExpr
      let paramFree := fun row => mkApp (mkConst ``NRow.paramFree) row
      let closedProof ← kernelTrue (mkApp2 (mkConst ``and)
        (mkApp2 (mkConst ``and) (paramFree capturedExpr) (paramFree suppliedExpr))
        (paramFree resultsExpr))
      let sharingProof ← closureSharingProof unitExpr handle fullExpr capturedExpr suppliedExpr
        weaveExpr shared
      let faithfulProof ← kernelTrue (mkApp3 (mkConst ``closureFaithful) unitExpr
        (Lean.toExpr handle) (Lean.toExpr (#[] : Array (VerexIR.TypeId × VerexIR.TypeId))))
      return mkAppN (mkConst ``Term.closure) #[unitExpr, shape, context, fullExpr, capturedExpr,
        suppliedExpr, Lean.toExpr handle, weaveExpr, resultsExpr, Lean.toExpr shared, rowsProof,
        sharingProof, closedProof, faithfulProof, ← quoteArgs unitExpr captures]
  | _, @Term.closureGeneric _ _ _ full captured supplied handle weave typeArgs θ targetShape shared
      _ _ _ captures => do
      let (fullExpr, capturedExpr, suppliedExpr) :=
        (← quoteRow full, ← quoteRow captured, ← quoteRow supplied)
      let weaveExpr ← quoteWeave weave
      let shapeExpr ← quoteShape targetShape
      let rowsProof ← closureRowsProof unitExpr handle fullExpr capturedExpr suppliedExpr
        weaveExpr (mkApp (mkConst ``ResultShape.row) shapeExpr)
      let belowProof ← kernelTrue
        (mkApp2 (mkConst ``closureSignatureBelow) unitExpr (Lean.toExpr handle))
      let sharingProof ← closureSharingProof unitExpr handle fullExpr capturedExpr suppliedExpr
        weaveExpr shared
      return mkAppN (mkConst ``Term.closureGeneric) #[unitExpr, shape, context, fullExpr,
        capturedExpr, suppliedExpr, Lean.toExpr handle, weaveExpr, Lean.toExpr typeArgs,
        ← quoteTypeArgs θ, shapeExpr, Lean.toExpr shared, rowsProof, sharingProof, belowProof,
        ← quoteArgs unitExpr captures]
  | _, @Term.invoke _ _ _ σs shared site invokeShape function arguments =>
      return mkAppN (mkConst ``Term.invoke) #[unitExpr, shape, context, ← quoteRow σs,
        Lean.toExpr shared, Lean.toExpr site, ← quoteShape invokeShape, ← quoteTerm unitExpr function,
        ← quoteArgs unitExpr arguments]
  | .tuple σs, .tuple elements =>
      return mkAppN (mkConst ``Term.tuple) #[unitExpr, shape, context, ← quoteRow σs, ← quoteArgs unitExpr elements]
  | .struct source arguments σs, .pack _ _ fields =>
      return mkAppN (mkConst ``Term.pack)
        #[unitExpr, shape, context, ← quoteRow σs, Lean.toExpr source, ← quoteRow arguments,
          ← quoteArgs unitExpr fields]
  | .enum source arguments names rows _, @Term.variant _ _ _ σs _ _ _ _ _ choice fields => do
      let namesExpr := Lean.toExpr names
      let distinct ← mkDecideProof (← mkAppM ``List.Nodup #[namesExpr])
      return mkAppN (mkConst ``Term.variant) #[unitExpr, shape, context, ← quoteRow σs, namesExpr,
        ← quoteRows rows, Lean.toExpr source, ← quoteRow arguments, distinct,
        ← quoteWhich choice, ← quoteArgs unitExpr fields]
  | τ, @Term.field _ _ _ σs _ source arguments x value =>
      return mkAppN (mkConst ``Term.field) #[unitExpr, shape, context, ← quoteRow σs, ← toExpr τ,
        Lean.toExpr source, ← quoteRow arguments, ← quoteVar x, ← quoteTerm unitExpr value]
  | .bool, @Term.isVariant _ _ _ names rows source arguments _ tests value => do
      let namesExpr := Lean.toExpr names
      let distinct ← mkDecideProof (← mkAppM ``List.Nodup #[namesExpr])
      return mkAppN (mkConst ``Term.isVariant) #[unitExpr, shape, context, namesExpr,
        ← quoteRows rows, Lean.toExpr source, ← quoteRow arguments, distinct, Lean.toExpr tests,
        ← quoteTerm unitExpr value]
  | τ, @Term.payload _ _ _ _ names rows source arguments _ mismatch choices value => do
      let namesExpr := Lean.toExpr names
      let distinct ← mkDecideProof (← mkAppM ``List.Nodup #[namesExpr])
      return mkAppN (mkConst ``Term.payload) #[unitExpr, shape, context, ← toExpr τ, namesExpr,
        ← quoteRows rows, Lean.toExpr source, ← quoteRow arguments, distinct,
        Lean.toExpr mismatch, ← quoteChoices choices, ← quoteTerm unitExpr value]
  | τ, @Term.letRow _ _ _ σs _ targets value body =>
      return mkAppN (mkConst ``Term.letRow) #[unitExpr, shape, context, ← quoteRow σs, ← toExpr τ,
        ← quoteVars targets, ← quoteTerm unitExpr value, ← quoteTerm unitExpr body]
  | τ, @Term.letFields _ _ _ σs _ source arguments targets value body =>
      return mkAppN (mkConst ``Term.letFields) #[unitExpr, shape, context, ← quoteRow σs, ← toExpr τ,
        Lean.toExpr source, ← quoteRow arguments, ← quoteVars targets, ← quoteTerm unitExpr value,
        ← quoteTerm unitExpr body]
  | τ, @Term.caseOf _ _ _ σ _ scrutinee arms =>
      return mkAppN (mkConst ``Term.caseOf) #[unitExpr, shape, context, ← toExpr σ, ← toExpr τ,
        ← quoteTerm unitExpr scrutinee, ← quoteArms unitExpr arms]
  | τ, .deref value =>
      return mkAppN (mkConst ``Term.deref) #[unitExpr, shape, context, ← toExpr τ, ← quoteTerm unitExpr value]
  | .unit, @Term.mutate _ _ _ τ x value =>
      return mkAppN (mkConst ``Term.mutate) #[unitExpr, shape, context, ← toExpr τ, ← quoteVar x,
        ← quoteTerm unitExpr value]
  | σ, @Term.readPlace _ _ _ τ _ mismatch x path =>
      return mkAppN (mkConst ``Term.readPlace) #[unitExpr, shape, context, ← toExpr τ, ← toExpr σ,
        Lean.toExpr mismatch, ← quoteVar x, ← quoteProj path]
  | .unit, @Term.writePlace _ _ _ τ σ mismatch x path value =>
      return mkAppN (mkConst ``Term.writePlace) #[unitExpr, shape, context, ← toExpr τ, ← toExpr σ,
        Lean.toExpr mismatch, ← quoteVar x, ← quoteProj path, ← quoteTerm unitExpr value]
  | .ref σ, @Term.borrowPlace _ _ _ τ _ mismatch x path =>
      return mkAppN (mkConst ``Term.borrowPlace) #[unitExpr, shape, context, ← toExpr τ, ← toExpr σ,
        Lean.toExpr mismatch, ← quoteVar x, ← quoteProj path]
  | τ, .take x => return mkAppN (mkConst ``Term.take) #[unitExpr, shape, context, ← toExpr τ, ← quoteVar x]
  | .unit, @Term.resolve _ _ _ τ x =>
      return mkAppN (mkConst ``Term.resolve) #[unitExpr, shape, context, ← toExpr τ, ← quoteVar x]
  | .vector τ, .vectorLit count elements =>
      return mkAppN (mkConst ``Term.vectorLit) #[unitExpr, shape, context, ← toExpr τ, Lean.toExpr count,
        ← quoteArgs unitExpr elements]
  | .int 64 false, @Term.length _ _ _ τ vector =>
      return mkAppN (mkConst ``Term.length) #[unitExpr, shape, context, ← toExpr τ, ← quoteTerm unitExpr vector]
  | .address, .signerAddress signer =>
      return mkAppN (mkConst ``Term.signerAddress) #[unitExpr, shape, context, ← quoteTerm unitExpr signer]
  | τ, @Term.index _ _ _ _ width signed site vector position =>
      return mkAppN (mkConst ``Term.index) #[unitExpr, shape, context, ← toExpr τ, Lean.toExpr width,
        Lean.toExpr signed, Lean.toExpr site, ← quoteTerm unitExpr vector, ← quoteTerm unitExpr position]
  | .unit, @Term.checkIndex _ _ _ τ width signed site failure vector position =>
      return mkAppN (mkConst ``Term.checkIndex) #[unitExpr, shape, context, ← toExpr τ, Lean.toExpr width,
        Lean.toExpr signed, Lean.toExpr site, Lean.toExpr failure, ← quoteTerm unitExpr vector, ← quoteTerm unitExpr position]
  | .vector τ, .push vector element =>
      return mkAppN (mkConst ``Term.push) #[unitExpr, shape, context, ← toExpr τ, ← quoteTerm unitExpr vector,
        ← quoteTerm unitExpr element]
  | .vector τ, @Term.insert _ _ _ _ width signed site vector position element =>
      return mkAppN (mkConst ``Term.insert) #[unitExpr, shape, context, ← toExpr τ, Lean.toExpr width,
        Lean.toExpr signed, Lean.toExpr site, ← quoteTerm unitExpr vector, ← quoteTerm unitExpr position, ← quoteTerm unitExpr element]
  | .tuple (.cons τ (.cons (.vector _) .nil)), @Term.remove _ _ _ _ width signed site vector position =>
      return mkAppN (mkConst ``Term.remove) #[unitExpr, shape, context, ← toExpr τ, Lean.toExpr width,
        Lean.toExpr signed, Lean.toExpr site, ← quoteTerm unitExpr vector, ← quoteTerm unitExpr position]
  | .vector τ, @Term.swap _ _ _ _ width signed site vector left right =>
      return mkAppN (mkConst ``Term.swap) #[unitExpr, shape, context, ← toExpr τ, Lean.toExpr width,
        Lean.toExpr signed, Lean.toExpr site, ← quoteTerm unitExpr vector, ← quoteTerm unitExpr left, ← quoteTerm unitExpr right]
  | .vector τ, .concat left right =>
      return mkAppN (mkConst ``Term.concat) #[unitExpr, shape, context, ← toExpr τ, ← quoteTerm unitExpr left,
        ← quoteTerm unitExpr right]
  | .vector τ, @Term.slice _ _ _ _ width signed site vector start stop =>
      return mkAppN (mkConst ``Term.slice) #[unitExpr, shape, context, ← toExpr τ, Lean.toExpr width,
        Lean.toExpr signed, Lean.toExpr site, ← quoteTerm unitExpr vector, ← quoteTerm unitExpr start, ← quoteTerm unitExpr stop]
  | .vector τ, @Term.reverseSlice _ _ _ _ width signed site vector start stop =>
      return mkAppN (mkConst ``Term.reverseSlice) #[unitExpr, shape, context, ← toExpr τ, Lean.toExpr width,
        Lean.toExpr signed, Lean.toExpr site, ← quoteTerm unitExpr vector, ← quoteTerm unitExpr start, ← quoteTerm unitExpr stop]
  | .unit, @Term.destroyEmpty _ _ _ τ site vector =>
      return mkAppN (mkConst ``Term.destroyEmpty) #[unitExpr, shape, context, ← toExpr τ, Lean.toExpr site,
        ← quoteTerm unitExpr vector]
  | .bool, @Term.contains _ _ _ τ vector needle =>
      return mkAppN (mkConst ``Term.contains) #[unitExpr, shape, context, ← toExpr τ, ← quoteTerm unitExpr vector,
        ← quoteTerm unitExpr needle]
  | .tuple (.cons .bool (.cons (.int 64 false) .nil)), @Term.indexOf _ _ _ τ vector needle =>
      return mkAppN (mkConst ``Term.indexOf) #[unitExpr, shape, context, ← toExpr τ, ← quoteTerm unitExpr vector,
        ← quoteTerm unitExpr needle]
  | .int 8 true, @Term.order _ _ _ τ orders left right =>
      return mkAppN (mkConst ``Term.order) #[unitExpr, shape, context, ← toExpr τ,
        ← VerexSL.Quote.quoteValueOrders orders, ← quoteTerm unitExpr left,
        ← quoteTerm unitExpr right]
  | .unit, .assertion site =>
      return mkAppN (mkConst ``Term.assertion) #[unitExpr, shape, context, Lean.toExpr site]
  | .unit, .anchor site =>
      return mkAppN (mkConst ``Term.anchor) #[unitExpr, shape, context, Lean.toExpr site]
  | .unit, .mutationEnd site =>
      return mkAppN (mkConst ``Term.mutationEnd) #[unitExpr, shape, context, Lean.toExpr site]
  | .unit, .memoryWritten site =>
      return mkAppN (mkConst ``Term.memoryWritten) #[unitExpr, shape, context, Lean.toExpr site]
  | τ, .constructed site value =>
      return mkAppN (mkConst ``Term.constructed) #[unitExpr, shape, context, ← toExpr τ, Lean.toExpr site,
        ← quoteTerm unitExpr value]
  | τ, .seqAfter value effect =>
      return mkAppN (mkConst ``Term.seqAfter) #[unitExpr, shape, context, ← toExpr τ, ← quoteTerm unitExpr value,
        ← quoteTerm unitExpr effect]
  | τ, @Term.globalRead _ _ _ κ _ site arguments key =>
      return mkAppN (mkConst ``Term.globalRead) #[unitExpr, shape, context, ← toExpr κ, ← toExpr τ,
        Lean.toExpr site, ← quoteRow arguments, ← quoteTerm unitExpr key]
  | .bool, @Term.globalContains _ _ _ κ τ arguments key =>
      return mkAppN (mkConst ``Term.globalContains) #[unitExpr, shape, context, ← toExpr κ, ← toExpr τ,
        ← quoteRow arguments, ← quoteTerm unitExpr key]
  | .ref τ, @Term.globalBorrow _ _ _ κ _ site arguments key =>
      return mkAppN (mkConst ``Term.globalBorrow) #[unitExpr, shape, context, ← toExpr κ, ← toExpr τ,
        Lean.toExpr site, ← quoteRow arguments, ← quoteTerm unitExpr key]
  | .unit, @Term.globalPublish _ _ _ κ τ site arguments key value =>
      return mkAppN (mkConst ``Term.globalPublish) #[unitExpr, shape, context, ← toExpr κ, ← toExpr τ,
        Lean.toExpr site, ← quoteRow arguments, ← quoteTerm unitExpr key, ← quoteTerm unitExpr value]
  | τ, @Term.globalTake _ _ _ κ _ site arguments key =>
      return mkAppN (mkConst ``Term.globalTake) #[unitExpr, shape, context, ← toExpr κ, ← toExpr τ,
        Lean.toExpr site, ← quoteRow arguments, ← quoteTerm unitExpr key]
  | .unit, @Term.ghostStore _ _ _ κ τ site arguments key value =>
      return mkAppN (mkConst ``Term.ghostStore) #[unitExpr, shape, context, ← toExpr κ, ← toExpr τ,
        Lean.toExpr site, ← quoteRow arguments, ← quoteTerm unitExpr key, ← quoteTerm unitExpr value]

partial def quoteArgs {unit : ValidatedUnit} (unitExpr : Lean.Expr) {ρ : ResultShape} {Γ : NRow} :
    {σs : NRow} → Args unit ρ Γ σs →
    MetaM Lean.Expr := fun {σs} arguments => do
  let shape ← quoteShape ρ
  let context ← quoteRow Γ
  match σs, arguments with
  | _, .nil => return mkAppN (mkConst ``Args.nil) #[unitExpr, shape, context]
  | .cons σ σs, .cons head tail =>
      return mkAppN (mkConst ``Args.cons) #[unitExpr, shape, context, ← quoteNTy σ, ← quoteRow σs,
        ← quoteTerm unitExpr head, ← quoteArgs unitExpr tail]

partial def quoteArms {unit : ValidatedUnit} (unitExpr : Lean.Expr) {ρ : ResultShape} {Γ : NRow}
    {σ τ : NTy} : Arms unit ρ Γ σ τ →
    MetaM Lean.Expr := fun arms => do
  let prefix_ := #[unitExpr, ← quoteShape ρ, ← quoteRow Γ, ← quoteNTy σ, ← quoteNTy τ]
  match arms with
  | .nil mismatch => return mkAppN (mkConst ``Arms.nil) (prefix_.push (Lean.toExpr mismatch))
  | .cons arm pattern body rest =>
      return mkAppN (mkConst ``Arms.cons)
        (prefix_ ++ #[Lean.toExpr arm, ← quotePat pattern, ← quoteTerm unitExpr body,
          ← quoteArms unitExpr rest])
  | .guarded arm pattern guard body rest =>
      return mkAppN (mkConst ``Arms.guarded) (prefix_ ++
        #[Lean.toExpr arm, ← quotePat pattern, ← quoteTerm unitExpr guard, ← quoteTerm unitExpr body, ← quoteArms unitExpr rest])
end

mutual
/-- Fold over the calls a term makes, in evaluation order: each callee
with the type arguments of the call, and their native row where it has
them; and over its sites a path's decisions name, each by its kind (`abort`
for a throw, `op` for an operation that may abort, `if`, `loop`,
`invoke`, `match`). -/
partial def _root_.VerexIR.Proofs.Denote.Term.foldCalls {α : Type}
    (visit : FunctionHandle → Array VerexIR.TypeUse → Option TypeArgs → α → α)
    (atSite : String → Nat → α → α) {unit : ValidatedUnit}
    {ρ : ResultShape} :
    {Γ : NRow} → {τ : NTy} → Term unit ρ Γ τ → α → α
  | _, _, .checked site _ _ left right, found =>
      right.foldCalls visit atSite (left.foldCalls visit atSite (atSite "op" site found))
  | _, _, .modular _ left right, found => right.foldCalls visit atSite (left.foldCalls visit atSite found)
  | _, _, .compare _ left right, found => right.foldCalls visit atSite (left.foldCalls visit atSite found)
  | _, _, .equal _ left right, found => right.foldCalls visit atSite (left.foldCalls visit atSite found)
  | _, _, .equalErased _ left right, found => right.foldCalls visit atSite (left.foldCalls visit atSite found)
  | _, _, .not operand, found => operand.foldCalls visit atSite found
  | _, _, .logical _ left right, found => right.foldCalls visit atSite (left.foldCalls visit atSite found)
  | _, _, .bitwise _ left right, found => right.foldCalls visit atSite (left.foldCalls visit atSite found)
  | _, _, .shift site _ _ value distance, found =>
      distance.foldCalls visit atSite (value.foldCalls visit atSite (atSite "op" site found))
  | _, _, .cast site _ value, found => value.foldCalls visit atSite (atSite "op" site found)
  | _, _, .ite site condition thenBranch elseBranch, found =>
      let found := match site with
        | some site => atSite "if" site found
        | none => found
      elseBranch.foldCalls visit atSite (thenBranch.foldCalls visit atSite
        (condition.foldCalls visit atSite found))
  | _, _, .let_ _ value body, found => body.foldCalls visit atSite (value.foldCalls visit atSite found)
  | _, _, .drop value body, found => body.foldCalls visit atSite (value.foldCalls visit atSite found)
  | _, _, .assign _ value, found => value.foldCalls visit atSite found
  | _, _, .const value, found => value.foldCalls visit atSite found
  | _, _, .throw0 site _, found => atSite "abort" site found
  | _, _, .throw1 site _ code, found => code.foldCalls visit atSite (atSite "abort" site found)
  | _, _, .return_ value, found => value.foldCalls visit atSite found
  | _, _, .loop site body, found => body.foldCalls visit atSite (atSite "loop" site found)
  | _, _, .assume _, found => found
  | _, _, .ghostRead slot, found => slot.foldCalls visit atSite found
  | _, _, .ghostSlot value, found => value.foldCalls visit atSite found
  | _, _, .ghostInit _ _ value, found => value.foldCalls visit atSite found
  | _, _, .call _ handle _ arguments, found => arguments.foldCalls visit atSite (visit handle #[] none found)
  | _, _, .callGeneric _ handle typeArgs θ _ arguments, found =>
      arguments.foldCalls visit atSite (visit handle typeArgs (some θ) found)
  -- An invocation of a closure runs its target, whose theorem a proof uses.
  | _, _, .closure handle _ _ _ _ _ _ _ captures, found =>
      captures.foldCalls visit atSite (visit handle #[] none found)
  | _, _, .closureGeneric handle _ typeArgs θ _ _ _ _ _ captures, found =>
      captures.foldCalls visit atSite (visit handle typeArgs (some θ) found)
  | _, _, .invoke site _ function arguments, found =>
      arguments.foldCalls visit atSite (function.foldCalls visit atSite (atSite "invoke" site found))
  | _, _, .tuple elements, found => elements.foldCalls visit atSite found
  | _, _, .pack _ _ fields, found => fields.foldCalls visit atSite found
  | _, _, .variant _ _ _ _ fields, found => fields.foldCalls visit atSite found
  | _, _, .field _ value, found => value.foldCalls visit atSite found
  | _, _, .isVariant _ value, found => value.foldCalls visit atSite found
  | _, _, .payload _ _ value, found => value.foldCalls visit atSite found
  | _, _, .letRow _ value body, found => body.foldCalls visit atSite (value.foldCalls visit atSite found)
  | _, _, .letFields _ value body, found => body.foldCalls visit atSite (value.foldCalls visit atSite found)
  | _, _, .caseOf scrutinee arms, found => arms.foldCalls visit atSite (scrutinee.foldCalls visit atSite found)
  | _, _, .deref value, found => value.foldCalls visit atSite found
  | _, _, .mutate _ value, found => value.foldCalls visit atSite found
  | _, _, .writePlace _ _ _ value, found => value.foldCalls visit atSite found
  | _, _, .vectorLit _ elements, found => elements.foldCalls visit atSite found
  | _, _, .length vector, found => vector.foldCalls visit atSite found
  | _, _, .signerAddress signer, found => signer.foldCalls visit atSite found
  | _, _, .index site vector position, found =>
      position.foldCalls visit atSite (vector.foldCalls visit atSite (atSite "op" site found))
  | _, _, .checkIndex site _ vector position, found =>
      position.foldCalls visit atSite (vector.foldCalls visit atSite (atSite "op" site found))
  | _, _, .push vector element, found => element.foldCalls visit atSite (vector.foldCalls visit atSite found)
  | _, _, .insert site vector position element, found =>
      element.foldCalls visit atSite (position.foldCalls visit atSite
        (vector.foldCalls visit atSite (atSite "op" site found)))
  | _, _, .remove site vector position, found =>
      position.foldCalls visit atSite (vector.foldCalls visit atSite (atSite "op" site found))
  | _, _, .swap site vector left right, found =>
      right.foldCalls visit atSite (left.foldCalls visit atSite
        (vector.foldCalls visit atSite (atSite "op" site found)))
  | _, _, .concat left right, found => right.foldCalls visit atSite (left.foldCalls visit atSite found)
  | _, _, .slice site vector start stop, found =>
      stop.foldCalls visit atSite (start.foldCalls visit atSite
        (vector.foldCalls visit atSite (atSite "op" site found)))
  | _, _, .reverseSlice site vector start stop, found =>
      stop.foldCalls visit atSite (start.foldCalls visit atSite
        (vector.foldCalls visit atSite (atSite "op" site found)))
  | _, _, .destroyEmpty site vector, found => vector.foldCalls visit atSite (atSite "op" site found)
  | _, _, .contains vector needle, found => needle.foldCalls visit atSite (vector.foldCalls visit atSite found)
  | _, _, .indexOf vector needle, found => needle.foldCalls visit atSite (vector.foldCalls visit atSite found)
  | _, _, .order _ left right, found => right.foldCalls visit atSite (left.foldCalls visit atSite found)
  | _, _, .seqAfter value effect, found => effect.foldCalls visit atSite (value.foldCalls visit atSite found)
  | _, _, .assertion _, found => found
  | _, _, .anchor _, found => found
  | _, _, .mutationEnd _, found => found
  | _, _, .memoryWritten _, found => found
  | _, _, .constructed _ value, found => value.foldCalls visit atSite found
  | _, _, .globalRead site _ key, found => key.foldCalls visit atSite (atSite "op" site found)
  | _, _, .globalContains _ _ key, found => key.foldCalls visit atSite found
  | _, _, .globalBorrow site _ key, found => key.foldCalls visit atSite (atSite "op" site found)
  | _, _, .globalPublish site _ key value, found =>
      value.foldCalls visit atSite (key.foldCalls visit atSite (atSite "op" site found))
  | _, _, .globalTake site _ key, found => key.foldCalls visit atSite (atSite "op" site found)
  | _, _, .ghostStore _ _ key value, found => value.foldCalls visit atSite (key.foldCalls visit atSite found)
  | _, _, _, found => found

partial def _root_.VerexIR.Proofs.Denote.Args.foldCalls {α : Type}
    (visit : FunctionHandle → Array VerexIR.TypeUse → Option TypeArgs → α → α)
    (atSite : String → Nat → α → α) {unit : ValidatedUnit}
    {ρ : ResultShape} :
    {Γ σs : NRow} → Args unit ρ Γ σs → α → α
  | _, _, .nil, found => found
  | _, _, .cons head tail, found => tail.foldCalls visit atSite (head.foldCalls visit atSite found)

partial def _root_.VerexIR.Proofs.Denote.Arms.foldCalls {α : Type}
    (visit : FunctionHandle → Array VerexIR.TypeUse → Option TypeArgs → α → α)
    (atSite : String → Nat → α → α) {unit : ValidatedUnit}
    {ρ : ResultShape} :
    {Γ : NRow} → {σ τ : NTy} → Arms unit ρ Γ σ τ → α → α
  | _, _, _, .nil _, found => found
  | _, _, _, .cons arm _ body rest, found =>
      rest.foldCalls visit atSite (body.foldCalls visit atSite (armSite arm found))
  | _, _, _, .guarded arm _ guard body rest, found =>
      rest.foldCalls visit atSite (body.foldCalls visit atSite
        (guard.foldCalls visit atSite (armSite arm found)))
where
  /-- The source `match` an arm belongs to. -/
  armSite (arm : Option (Nat × String)) (found : α) : α :=
    match arm with
    | some (site, _) => atSite "match" site found
    | none => found
end

/-- The callees a term calls, in order of first occurrence. -/
def _root_.VerexIR.Proofs.Denote.Term.callees {unit : ValidatedUnit} {ρ : ResultShape} {Γ : NRow}
    {τ : NTy}
    (term : Term unit ρ Γ τ) (found : Array FunctionHandle) : Array FunctionHandle :=
  term.foldCalls (fun handle _ _ found => if found.contains handle then found else found.push handle)
    (fun _ _ found => found)
    found

/-- The mutable parameters of a compiled function as a literal. -/
def quoteMutables {Γ : NRow} : Mutables Γ → MetaM Lean.Expr
  | .nil => return mkApp (mkConst ``Mutables.nil) (← quoteRow Γ)
  | @Mutables.cons _ τ x rest =>
      return mkAppN (mkConst ``Mutables.cons)
        #[← quoteRow Γ, ← quoteNTy τ, ← quoteVar x, ← quoteMutables rest]

def quoteFunction {unit : ValidatedUnit} (unitExpr : Lean.Expr) (f : Function unit)
    (body mutables : Lean.Expr) : MetaM Lean.Expr :=
  return mkAppN (mkConst ``Function.mk)
    #[unitExpr, ← quoteRow f.params, ← quoteRow f.locals, ← quoteShape f.result, body, mutables]

/-! ## Names -/

def pathName (segments : Array String) : Name :=
  segments.foldl (fun name segment => Name.str name segment) .anonymous

def rootIdent (name : Name) : Ident := mkIdent (rootNamespace ++ name)

def compiledName (segments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName segments) function) "compiled"

def compiledEqName (segments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName segments) function) "compiled_eq"

def typedSemanticsVerifiedName (segments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName segments) function) "typedSemanticsVerified"

def verifiedName (segments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName segments) function) "verified"

/-! ## Contracts

The authored contract is translated once over runtime rows
(`buildContract`); its typed form is over the compiler's argument row and
result shape, and its public form is the runtime-row view of that. -/

/-- Name of a module's table of declared preconditions, which `requires_of`
reads. -/
def requiresTableName (segments : Array String) : Name :=
  Name.str (pathName segments) "requiresTable"

/-- Name of a unit's predicate of the data invariants of its stored
resources. -/
def storedInvariantName (segments : Array String) : Name :=
  Name.str (pathName segments) "storedInvariant"

/-- Name of a unit's theorem that its resource types correspond to their
keys' types. -/
def resourcesTypedName (segments : Array String) : Name :=
  Name.str (pathName segments) "resourcesTyped"

def typesAgreeName (segments : Array String) : Name :=
  Name.str (pathName segments) "typesAgree"

/-- The parameter row and result shape of a declaration, when every type
is native.  This is the signature the compiler will produce, decided from
the declaration alone so a contract can exist for an unverified function. -/
def nativeSignature? (unit : ValidatedUnit) (namespaceId : NamespaceId)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) :
    Option (NRow × ResultShape) := do
  let params ← declaration.signature.parameters.toList.mapM fun parameter =>
    ntyOf unit namespaceId parameter.typeUse.typeId
  let result ← match declaration.signature.results.toList with
    | [] => some ResultShape.none
    | [result] => (ntyOf unit namespaceId result.typeId).map ResultShape.one
    | _ => none
  some (NRow.ofList params, result)

def addAbbrev (name : Name) (value : Lean.Expr) : TermElabM Unit := do
  let value ← instantiateMVars value
  let type ← instantiateMVars (← inferType value)
  addDecl (.defnDecl { name, levelParams := [], type, value, hints := .abbrev, safety := .safe })
  setReducibilityStatus name .reducible
  enableRealizationsForConst name

/-- Run `k` under the skolem family every generated contract, invariant,
and twin bridge abstracts over. -/
private def withSkolems {n : Type → Type} [MonadControlT MetaM n] [Monad n] {α : Type}
    (k : Lean.Expr → n α) : n α :=
  withLocalDecl `unit .implicit (mkConst ``ValidatedUnit) fun unit =>
    withLocalDecl `Θ .instImplicit (mkApp (mkConst ``Skolems) unit) k

/-- Run `k` over a unit, an executable unit of it, and a frame at it: the
binders of an artifact that reads the executable. -/
def withExecutableSkolems {n : Type → Type} [MonadControlT MetaM n] [Monad n] {α : Type}
    (unit : ValidatedUnit) (k : Lean.Expr → Lean.Expr → n α) : n α :=
  withLocalDecl `unit .implicit (mkConst ``ValidatedUnit) fun unitExpr =>
    -- A unit verified through its model reads it from a model
    -- (`ModelUnit`, `doc/designs/ghost-fields.md`).
    withLocalDeclD `executable (Contract.unitReaderType unit unitExpr) fun executable =>
      withLocalDecl `Θ .instImplicit (mkApp (mkConst ``Skolems) unitExpr) (k executable)

/-- The binders of a frame: its unit and itself. -/
def frameBinders (skolems : Lean.Expr) : MetaM (Array Lean.Expr) := do
  return #[← Contract.frameUnit skolems, skolems]

/-- The binders of an artifact over an executable unit and a frame: the unit,
the executable, and the frame. -/
def executableBinders (executable skolems : Lean.Expr) : MetaM (Array Lean.Expr) := do
  return #[← Contract.frameUnit skolems, executable, skolems]

/-- The quoted native signature of a function, which its contract is
stated over. -/
def quoteNativeSignature (skolems : Lean.Expr) (params : NRow) (result : ResultShape) :
    MetaM NativeSignature := do
  let argumentTypes ← params.toList.toArray.mapM quoteNTy
  let (resultType, resultComponentTypes) ← match result with
    | .none => pure (none, #[])
    | .one τ => do
        let components ← match τ with
          | .tuple elements => elements.toList.toArray.mapM quoteNTy
          | _ => pure #[]
        pure (some (← quoteNTy τ), components)
  return { skolems, params := ← quoteRow params, argumentTypes, shape := ← quoteShape result,
           resultType, resultComponentTypes }

/-- Whether a result carries a mutable reference: a caller reasons about such
a callee through its denotation, since the value view of its contract
cannot relate a later write through the result to the lender. -/
def _root_.VerexIR.Proofs.Denote.ResultShape.returnsReference : ResultShape → Bool
  | .one (.ref _) => true
  | .one (.tuple elements) => elements.toList.any fun | .ref _ => true | _ => false
  | _ => false

/-- Whether a callee's contract stands for it at a call: always, unless it
returns a mutable reference without its contract stating the reference's
final value, when its body stands for it instead. -/
def contractStandsFor (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Bool :=
  !(nativeSignature? unit namespaceId declaration).any (·.2.returnsReference) ||
    hasFinalContract ns declaration || (Contract.mapRoleOf? unit namespaceId ns declaration).isSome

/-- The heartbeat budget a function sets for its own verification, in the
units of `verex.verifyHeartbeats`: `pragma heartbeats = N` counts thousands
of them. -/
def heartbeatBudget? (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) :
    Option Nat :=
  declaration.pragmas.findSome? fun
    | .assign "heartbeats" (.constant (.integer value)) _ =>
        if 0 < value then some (value.toNat * 1000) else none
    | _ => none

/-- The heartbeat budget a function sets for each leaf of its automatic
verification, in the units of `verex.leafHeartbeats`: `pragma
leaf_heartbeats = N` counts thousands of them. -/
def leafHeartbeatBudget? (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) :
    Option Nat :=
  declaration.pragmas.findSome? fun
    | .assign "leaf_heartbeats" (.constant (.integer value)) _ =>
        if 0 < value then some value.toNat else none
    | _ => none

/-- Whether a function's specification or its module sets `pragma verify =
manual`: an authored proof, `verify f by …`, establishes it. -/
def requiresAuthoredProof (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) :
    Bool :=
  declaration.pragmas.any fun
    | .assign "verify" (.name none "manual") _ => true
    | _ => false

/-- Whether a function is specified `pragma opaque`. -/
def isOpaque (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Bool :=
  declaration.pragmas.any fun
    | .assign "opaque" (.constant (.bool true)) _ => true
    | _ => false

/-- Whether a function selects bit-vector decisions, `pragma bv`: the
decision applies to every unsigned value of a leaf, not only the
parameters or results the pragma names. -/
def selectsBitVectors (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Bool :=
  declaration.pragmas.any fun
    | .assign name _ _ => name == "bv" || name == "bv_ret"
    | _ => false

/-- The functions an expression calls directly. -/
private partial def expressionCallees (unit : ValidatedUnit) (namespaceId : NamespaceId)
    (id : VerexIR.ExprId) (found : Array FunctionHandle) : Array FunctionHandle :=
  match unit.namespaces[namespaceId.index]?.bind (·.expressions[id.index]?) with
  | none => found
  | some expression =>
      let found := match expression.kind with
        | .operation (.call (.function reference)) _ _ _ =>
            match VerexIR.SemanticOperations.resolveFunction? unit namespaceId reference with
            | some callee => if found.contains callee then found else found.push callee
            | none => found
        | _ => found
      (VerexIR.Validation.expressionChildren expression.kind).foldl
        (fun found child => expressionCallees unit namespaceId child found) found

/-- The functions a function's body calls directly. -/
def bodyCallees (unit : ValidatedUnit) (handle : FunctionHandle) : Array FunctionHandle :=
  match unit.namespaces[handle.namespaceId.index]?.bind (·.functions[handle.functionId.index]?) with
  | some { body := .structured root, .. } => expressionCallees unit handle.namespaceId root #[]
  | _ => #[]

/-- Whether a function's body calls a function with a precondition, which
it owes at the call. -/
def callsRequiring (unit : ValidatedUnit) (handle : FunctionHandle) : Bool :=
  (bodyCallees unit handle).any fun callee =>
    match unit.namespaces[callee.namespaceId.index]?.bind (·.functions[callee.functionId.index]?) with
    | some declaration => declaration.contract.conditions.any (·.kind == .requires)
    | none => false

/-- Whether a function is on a cycle of calls: a call of it reaches it again. -/
private def onCallCycle (unit : ValidatedUnit) (handle : FunctionHandle) : Bool := Id.run do
  let mut reached : Array FunctionHandle := #[]
  let mut worklist := bodyCallees unit handle
  while let some next := worklist.back? do
    worklist := worklist.pop
    if next == handle then return true
    if reached.contains next then continue
    reached := reached.push next
    worklist := worklist ++ bodyCallees unit next
  return false

/-- Whether a call is reasoned about through the callee's contract rather
than its body, as the Move Prover does: a native's always, a function's only
when it is `opaque`, or on a cycle of calls, whose body no inlining can
exhaust. Every other callee is inlined, specified or not. A contract stands
for a callee only when it states a returned mutable reference's final value. -/
def usedThroughContract (unit : ValidatedUnit) (callee : FunctionHandle) (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Bool :=
  -- An intrinsic map role stands for the map model, as a native does.
  (Contract.mapRoleOf? unit callee.namespaceId ns declaration).isSome ||
  declaration.contract.loc.isSome && contractStandsFor unit callee.namespaceId ns declaration &&
    (declaration.body == .absent || isOpaque declaration || onCallCycle unit callee ||
      -- A body reading an intrinsic map's representation has no denotation.
      (Contract.representationRead? unit ns declaration).isSome)

/-- Whether a function has type parameters: it is proved over every skolem
family and type instantiation. -/
def isGeneric (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Bool :=
  declaration.signature.generics.any (·.kind == .typeArg)

/-- The typed contract of a function over its native arguments, abstracted
over the skolem family and, for a generic function, its type
instantiation. -/
def typedContractOf (unit : ValidatedUnit) (namespaceIndex : Nat) (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (params : NRow) (result : ResultShape)
    (nativeModel : Option NativeModel := none) (stored : Contract.StoredInvariants := {}) :
    MetaM Lean.Expr :=
  -- A contract is over a unit and a frame at it; one stating a behavioral
  -- predicate over an executable unit of it as well.
  let readsUnit := Contract.contractReadsUnit unit ⟨namespaceIndex⟩ ns declaration
  let overFrame (k : Array Lean.Expr → Option Lean.Expr → Lean.Expr → MetaM Lean.Expr) :=
    if readsUnit then withExecutableSkolems unit fun executable skolems => do
      k (← executableBinders executable skolems) (some executable) skolems
    else withSkolems fun skolems => do k (← frameBinders skolems) none skolems
  overFrame fun binders executable skolems => do
    let signature ← quoteNativeSignature skolems params result
    let types ← Contract.frameTypes skolems
    let build (typeInstantiation requiresTable : Option Lean.Expr) :=
      buildContract unit ⟨namespaceIndex⟩ ns declaration signature
        (types := types) (typeInstantiation := typeInstantiation)
        (nativeModel := nativeModel) (executable := executable) (requiresTable := requiresTable)
        (stored := stored)
    -- One stating `requires_of` takes the table of declared preconditions
    -- at the unit, last.
    let close (binders : Array Lean.Expr) (typeInstantiation : Option Lean.Expr) :
        MetaM Lean.Expr := do
      if let some executable := executable then
        if Contract.contractReadsRequires unit ⟨namespaceIndex⟩ declaration then
          let tableType := mkApp (mkConst ``VerexIR.Proofs.RequiresTable)
            (← Contract.executableUnit executable)
          return ← withLocalDeclD `requiresTable tableType fun table => do
            mkLambdaFVars (binders.push table) (← build typeInstantiation (some table))
      mkLambdaFVars binders (← build typeInstantiation none)
    if isGeneric declaration then
      let instantiationType ← mkAppM ``Array
        #[← mkAppM ``Prod #[mkConst ``VerexIR.TypeId, mkConst ``VerexIR.TypeId]]
      withLocalDeclD `typeInstantiation instantiationType fun typeInstantiation =>
        close (binders.push typeInstantiation) (some typeInstantiation)
    else close binders none


/-- The projections of a contract's conditions, in `Contract.mk`'s order. -/
private def contractConditions : Array Name :=
  #[``VerexIR.Proofs.Contract.requires, ``VerexIR.Proofs.Contract.assumes,
    ``VerexIR.Proofs.Contract.ensures, ``VerexIR.Proofs.Contract.aborts,
    ``VerexIR.Proofs.Contract.mayAbort, ``VerexIR.Proofs.Contract.mustAbort,
    ``VerexIR.Proofs.Contract.frame]

/-- The contract `generic`, specialized at type arguments `θ` of a
caller's frame (`doc/designs/native-verification.md`, "Specialized
contracts"): the generic contract at the frame `θ` induces, read in the
caller's view (`Contract.ofSkolem`), its conditions in the transport normal
form (`vir_transport`), the specification functions they apply transported
to the caller's frame (`ensureDefinitionTransport`); a call normalizes them
further at its arguments. Declared with its equivalence to the generic
contract read so, which the normalization proves, once per contract and
`θ`. The equivalence's name. -/
def ensureSpecialization (unit : ValidatedUnit) (generic : Name) (θ : TypeArgs) :
    TermElabM Name := do
  let specialized := Contract.specializedName generic θ.1
  let equivalence := specialized ++ `equivalent
  if (← getEnv).contains equivalence then return equivalence
  let some value := ((← getEnv).find? generic).bind (·.value?)
    | throwError m!"internal: the contract `{generic}` has no definition"
  -- The definitions the generic contract applies, transported across the
  -- frame `θ` induces.
  let mut transports := #[]
  for definition in (← definitionsIn value).1 do
    if let some transport ← ensureDefinitionTransport unit definition θ then
      unless transports.contains transport do transports := transports.push transport
  let lemmas ← (#[generic] ++ transports).mapM fun name =>
    `(Lean.Parser.Tactic.simpLemma| $(mkIdent (rootNamespace ++ name)):ident)
  let simpSyntax ← `(tactic| simp only [VerexIR.Proofs.Contract.ofSkolem, $lemmas,*,
    vir_transport])
  let context ← IO.mkRef none
  let holder ← mkFreshExprMVar (mkConst ``True)
  discard <| Tactic.run holder.mvarId! do
    let { ctx, simprocs, .. } ← Tactic.mkSimpContext simpSyntax (eraseLocal := false)
    context.set (some (← Simp.mkContext { ctx.config with failIfUnchanged := false }
      ctx.simpTheorems ctx.congrTheorems, simprocs))
    holder.mvarId!.assign (mkConst ``True.intro)
  let some (simpContext, simprocs) ← context.get | throwError "internal: no simp context"
  let θExpr ← quoteTypeArgs θ
  -- The generic contract read in the caller's view at the frame `θ` induces.
  let readAt (binders : Array Lean.Expr) : MetaM Lean.Expr := do
    let some frameIndex ← binders.findIdxM? fun binder => do
        return (← inferType binder).isAppOfArity ``Skolems 1
      | throwError m!"the contract `{generic}` is not over a frame"
    let frame := binders[frameIndex]!
    let unitExpr := (← inferType frame).appArg!
    let induced := mkApp3 (mkConst ``Skolems.instantiate) unitExpr θExpr frame
    mkAppOptM ``VerexIR.Proofs.Contract.ofSkolem
      #[unitExpr, frame, θExpr, none, none, mkAppN (mkConst generic) (binders.set! frameIndex induced)]
  let contractType ← inferType (mkConst generic)
  -- Each condition normalized over its arguments, with its equation.
  let (contract, equations) ← forallTelescope contractType fun binders _ => do
    let read ← readAt binders
    let mut conditions := #[]
    let mut equations := #[]
    for projection in contractConditions do
      let applied ← mkAppM projection #[read]
      let (condition, equation) ← forallTelescope (← inferType applied) fun arguments _ => do
        -- The condition alone, its contract unfolded and projected, so that
        -- the normal form visits no other.
        let reading ← whnfR (← deltaExpand (mkAppN applied arguments) fun name =>
          name == ``VerexIR.Proofs.Contract.ofSkolem || name == generic)
        let (result, _) ← Simp.main reading simpContext
          (methods := Simp.mkDefaultMethodsCore simprocs)
        let equation ← match result.proof? with
          | some proof => pure proof
          | none => mkEqRefl reading
        -- The types and defaults the normal form leaves at the induced
        -- family, in types and instances, at the caller's.
        let respelled ← FamilyTransport.respellInducedFamily result.expr
        let equation ← if respelled == result.expr then pure equation else
          mkEqTrans equation
            (← mkExpectedTypeHint (← mkEqRefl result.expr) (← mkEq result.expr respelled))
        return (← mkLambdaFVars arguments respelled, ← mkLambdaFVars arguments equation)
      conditions := conditions.push (← instantiateMVars condition)
      equations := equations.push (← mkLambdaFVars binders (← instantiateMVars equation))
    let readType ← whnfR (← inferType read)
    let contract := mkAppN (mkConst ``VerexIR.Proofs.Contract.mk) (readType.getAppArgs ++ conditions)
    return (← mkLambdaFVars binders contract, equations)
  addAbbrev specialized contract
  -- A condition of the specialized contract is its normal form: the
  -- equation read backwards.
  let (statement, proof) ← forallTelescope contractType fun binders _ => do
    let read ← readAt binders
    let specializedAt := mkAppN (mkConst specialized) binders
    let mut fields := #[]
    for projection in contractConditions, equation in equations do
      let field ← forallTelescope (← inferType (← mkAppM projection #[read])) fun arguments _ => do
        let left := mkAppN (← mkAppM projection #[specializedAt]) arguments
        let right := mkAppN (← mkAppM projection #[read]) arguments
        let backwards ← mkEqSymm (mkAppN (equation.beta binders) arguments)
        let iff ← mkExpectedTypeHint (← mkAppM ``Iff.of_eq #[backwards]) (← mkAppM ``Iff #[left, right])
        mkLambdaFVars arguments iff
      fields := fields.push field
    let equivalent ← mkAppOptM ``VerexIR.Proofs.Contract.Equivalent
      #[none, none, none, none, specializedAt, read]
    let proof := mkAppN (mkConst ``VerexIR.Proofs.Contract.Equivalent.mk)
      ((← whnfR (← inferType read)).getAppArgs ++ #[specializedAt, read] ++ fields)
    return (← mkForallFVars binders equivalent, ← mkLambdaFVars binders proof)
  addDecl (.thmDecl { name := equivalence, levelParams := [], type := statement, value := proof })
  return equivalence

/-- The runtime form of a typed contract: at the public family and the empty
type instantiation, or, for a generic function, at the frame type arguments
`θ` induce over the public family and a type instantiation, read at the
arguments' types. -/
def publicContractOf (params : NRow) (result : ResultShape) (generic : Bool) (typed : Lean.Expr)
    (readsUnit : Bool := false) (readsRequires : Bool := false) : MetaM Lean.Expr := do
  let row ← quoteRow params
  let shape ← quoteShape result
  -- The runtime form relates runtime globals to typed memory by the unit's
  -- resource types: it always takes the unit and an executable unit of it.
  withLocalDecl `unit .implicit (mkConst ``ValidatedUnit) fun unitExpr =>
  withLocalDeclD `executable (Contract.executableType unitExpr) fun executable => do
    let runtime := mkApp (mkConst ``Skolems.runtime) unitExpr
    let atFrame (frame : Lean.Expr) := if readsUnit then mkApp3 typed unitExpr executable frame
      else mkApp2 typed unitExpr frame
    let close (binders : Array Lean.Expr) (contract : Lean.Expr → MetaM Lean.Expr) :
        MetaM Lean.Expr :=
      if readsUnit && readsRequires then
        withLocalDeclD `requiresTable (mkApp (mkConst ``VerexIR.Proofs.RequiresTable) unitExpr)
          fun table => do
            mkLambdaFVars (#[unitExpr, executable] ++ binders.push table) (← contract table)
      else do
        mkLambdaFVars (#[unitExpr, executable] ++ binders) (← contract (mkConst ``Unit.unit))
    let applyTable (typed : Lean.Expr) (table : Lean.Expr) : Lean.Expr :=
      if readsUnit && readsRequires then mkApp typed table else typed
    if generic then
      let instantiationType ← mkAppM ``Array
        #[← mkAppM ``Prod #[mkConst ``VerexIR.TypeId, mkConst ``VerexIR.TypeId]]
      withLocalDeclD `θ (mkConst ``TypeArgs) fun θ =>
      withLocalDeclD `typeInstantiation instantiationType fun instantiation => do
        let frame := mkApp3 (mkConst ``Skolems.instantiate) unitExpr θ runtime
        let arguments ← mkAppM ``Subtype.val #[θ]
        close #[θ, instantiation] fun table => do
          let typed := (applyTable (mkApp (atFrame frame) instantiation) table).headBeta
          let read ← mkAppOptM ``VerexIR.Proofs.Contract.ofSkolem
            #[some unitExpr, some runtime, some θ, some row, some shape, some typed]
          mkAppOptM ``VerexIR.Proofs.Contract.prophetic
            #[some unitExpr, some runtime, some (mkApp2 (mkConst ``NRow.subst) arguments row),
              some (mkApp2 (mkConst ``ResultShape.subst) arguments shape), some read]
    else
      close #[] fun table => do
        let typed := (applyTable (atFrame runtime) table).headBeta
        mkAppOptM ``VerexIR.Proofs.Contract.prophetic
          #[some unitExpr, some runtime, some row, some shape, some typed]

/-- The declarations whose stored invariants a unit's predicate states, and
those whose invariants it does not carry, keyed by the predicate's name.
Recorded once, with the predicate, for every module that builds contracts
over the unit. -/
private initialize storedDeclarations :
    SimplePersistentEnvExtension
      (Name × Array VerexIR.StructHandle × Array (VerexIR.StructHandle × String))
      (NameMap (Array VerexIR.StructHandle × Array (VerexIR.StructHandle × String))) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := fun map (name, declarations) => map.insert name declarations
    addImportedFn := fun entries =>
      mkStateFromImportedEntries (fun map (name, declarations) => map.insert name declarations)
        {} entries }

/-- Define once per unit the predicate of the data invariants of its stored
resources (`Contract.storedInvariantTerm`), which the solver's normalization
unfolds where it is applied, and record the declarations it covers. -/
def ensureStoredInvariant (segments : Array String) (unit : ValidatedUnit)
    : CommandElabM Contract.StoredInvariants := do
  let name := storedInvariantName segments
  if let some (carriers, unsupported) := (storedDeclarations.getState (← getEnv)).find? name then
    let predicate := if (← getEnv).contains name then some (mkConst name) else none
    return { predicate, carriers, unsupported }
  let stored ← liftTermElabM (Contract.storedInvariantTerm unit)
  modifyEnv fun env => storedDeclarations.addEntry env (name, stored.carriers, stored.unsupported)
  if stored.cases.isEmpty then return stored
  liftTermElabM do
    -- Over the unit memory is typed at, then the executable unit where an
    -- invariant reads it.
    let readsUnit := Contract.storedInvariantReadsUnit unit
    let leadingCount := if readsUnit then 2 else 1
    -- A constant and its equation, which the solver's normalization unfolds
    -- where the constant is applied to all its arguments.
    let define (constant : Name) (value : Lean.Expr) (arity : Nat) : TermElabM Unit := do
      let type ← inferType value
      addDecl (.defnDecl {
        name := constant, levelParams := [], type, value, hints := .regular 0, safety := .safe })
      let statement ← forallBoundedTelescope type arity fun binders _ => do
        mkForallFVars binders (← mkEq (mkAppN (mkConst constant) binders) (value.beta binders))
      let proof ← forallTelescope statement fun binders equality => do
        mkLambdaFVars binders (← mkEqRefl equality.appArg!)
      let equation := constant ++ `eq
      addDecl (.thmDecl { name := equation, levelParams := [], type := statement, value := proof })
      let attr ← `(attr| vir_denote_norm)
      Lean.Elab.Term.applyAttributes equation
        #[{ name := `vir_denote_norm, stx := attr, kind := .global }]
    -- Each declaration's invariant behind a constant of its own, over an
    -- instance's type arguments and a value of it, selected by the
    -- declaration before it is applied: a value of another resource type
    -- never unfolds it.
    let mut constants : Array Name := #[]
    for case in stored.cases, index in [0:stored.cases.size] do
      let constant := name ++ Name.mkSimple s!"case{index}"
      define constant case (leadingCount + 2)
      constants := constants.push constant
    let predicate ← withLocalDeclD `unit (mkConst ``ValidatedUnit) fun unitExpr =>
      withLocalDeclD `executable (Contract.unitReaderType unit unitExpr) fun executable =>
      withLocalDeclD `resource (mkConst ``ResourceType) fun resource =>
        withLocalDeclD `value (mkApp2 (mkConst ``ResourceType.carrier) unitExpr resource)
          fun held => do
          let leading := if readsUnit then #[unitExpr, executable] else #[unitExpr]
          let handle? := mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.handle?)
            (mkApp (mkConst ``VerexIR.Proofs.Denote.ResourceType.type) resource)
          let mut selected := mkConst ``True
          for ((owner, σ), constant) in ((stored.carriers.zip stored.types).zip constants).reverse do
            let some arguments := σ.nominalArguments?
              | throwError "internal: a stored declaration's native type is not nominal"
            let arguments ← quoteRow arguments
            let test ← mkEq handle? (← mkAppM ``Option.some #[toExpr owner])
            let case := mkAppN (mkConst ``VerexIR.Proofs.Denote.StoredCase)
              #[unitExpr, ← quoteNTy σ, arguments, mkAppN (mkConst constant) leading, resource, held]
            selected ← mkAppM ``ite #[test, case, selected]
          mkLambdaFVars leading (← mkLambdaFVars #[resource, held] selected)
    define name predicate (leadingCount + 2)
  return { stored with predicate := some (mkConst name) }

/-- Define the typed and public contracts of a function, once: the contract
over its native arguments, and its runtime form. -/
def ensureContracts (segments : Array String) (function : String) (unit : ValidatedUnit)
    (namespaceIndex : Nat) (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (params : NRow) (result : ResultShape) (nativeModel : Option NativeModel := none) :
    CommandElabM Unit := do
  let typedName := typedContractName segments function
  let publicName := contractName segments function
  -- A unit verified through its model has no runtime contract
  -- (`doc/designs/ghost-fields.md`).
  let model := declaresGhosts unit
  if (← getEnv).contains (if model then typedName else publicName) then
    let some info := (← getEnv).find? typedName
      | throwError m!"`{function}` has a contract this unit did not generate"
    unless info.type.getUsedConstants.contains ``HList do
      throwError m!"`{function}` has a contract this unit did not generate"
    return
  let stored ← ensureStoredInvariant segments unit
  liftTermElabM do
    -- A body is proved against its implementation; a native, which has
    -- none, is only ever seen by its callers.
    let view := if declaration.body == .absent then ContractView.interface
      else .implementation
    addAbbrev typedName (← typedContractOf unit namespaceIndex ns (view.of declaration)
      params result nativeModel stored)
    unless model do
      addAbbrev publicName
        (← publicContractOf params result (isGeneric declaration) (mkConst typedName)
          (Contract.contractReadsUnit unit ⟨namespaceIndex⟩ ns (view.of declaration))
          (Contract.contractReadsRequires unit ⟨namespaceIndex⟩ (view.of declaration)))

def countErrors (log : MessageLog) : Nat :=
  log.reportedPlusUnreported.toList.filter (·.severity == .error) |>.length

/-- The proof file a namespace's `proof_file` pragma names: where the
authored proofs of its functions live. -/
def proofFile? (ns : ValidatedNamespace) : Option String :=
  ns.pragmas.findSome? fun
    | .assign "proof_file" (.constant (.string path)) _ => some path
    | _ => none

/-- The proof a failed automatic verification asks for: the template of an
authored proof, one alternative per leaf the solver left undecided, the
others to cheap deciders only; `verex_prepare` shows a leaf in source form. -/
def proofRequest (ns : ValidatedNamespace) (function : String) (leaves : Array String) :
    MessageData :=
  let place := match proofFile? ns with
    | some path => m!"in `{path}`"
    | none => m!"in the module"
  let alternatives := leaves.foldl (init := m!"") fun text leaf =>
    text ++ m!"\n    | {leaf} => verex_prepare; sorry"
  m!"provide a proof {place}:\n  verify {function} by lean =>\n    verex_obligations\n    \
    verex_on{alternatives}\n    | _ => verex_decide\n  (`verex_prepare` states a leaf in source \
    form; `trace_state` after it shows the goal)"

/-- Generate the contract a function's callers see of it, where that
differs from the one its body is proved against: they assume it. -/
def ensureInterfaceContract (segments : Array String) (function : String) (unit : ValidatedUnit)
    (namespaceIndex : Nat) (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (params : NRow) (result : ResultShape) : CommandElabM Unit := do
  let name := typedInterfaceContractName segments function
  if (← getEnv).contains name then return
  let stored ← ensureStoredInvariant segments unit
  liftTermElabM do
    let contract ← typedContractOf unit namespaceIndex ns
      (ContractView.interface.of declaration) params result (stored := stored)
    addAbbrev name contract

/-! ## Loop invariants

Each loop of a compiled body gets an invariant over the locals: the
authored `invariant` clauses, over the slots they read (asserted defined), and the
automatic frame tying every immutable local to its entry value. -/

/-- The slots of a local row, as projections of a row value. -/
private def slotProjections (row : Lean.Expr) (count : Nat) : MetaM (Array Lean.Expr) := do
  let mut rest := row
  let mut slots := #[]
  for _ in [:count] do
    slots := slots.push (← mkAppM ``Prod.fst #[rest])
    rest ← mkAppM ``Prod.snd #[rest]
  return slots

/-- The clause-level value of a native slot binder, when a clause can read
it: an integer's value, any other value itself, a reference's current value. -/
private def logicalValue (τ : NTy) (value : Lean.Expr) : MetaM (Option Lean.Expr) := do
  match τ with
  | .int _ _ | .num => return some (← mkAppM ``VerexIR.SpecInt.val #[value])
  | .bool | .address | .signer | .string | .param _ => return some value
  | .unit | .bytes => return none
  | .tuple _ | .struct _ _ _ | .enum _ _ _ _ _ | .vector _ | .table _ _ | .function _ _ _ =>
      return some value
  | .ref referent => logicalValue referent (← mkAppM ``Prod.fst #[value])
  | .ghost slot => logicalValue slot value

/-- The locals a pattern binds. -/
private partial def patternLocals (ns : ValidatedNamespace) (pattern : VerexIR.PatternId) :
    Array Nat :=
  match ns.patterns[pattern.index]? with
  | some { kind := .variable localId, .. } => #[localId.index]
  | some { kind := .tuple children, .. }
  | some { kind := .constructor _ _ _ children, .. } => children.flatMap (patternLocals ns)
  | _ => #[]

/-- The locals an expression tree reads from its context: those it binds
itself, by a quantifier, a `let`, or a match arm, are its own. -/
partial def referencedLocals (ns : ValidatedNamespace) (id : VerexIR.ExprId)
    (found : Array Nat := #[]) (bound : Array Nat := #[]) : Array Nat :=
  match ns.expressions[id.index]? with
  | none => found
  | some expression =>
      let found := match expression.kind with
        | .localVar localId =>
            if found.contains localId.index || bound.contains localId.index then found
            else found.push localId.index
        | _ => found
      let bound := match expression.kind with
        | .quantifier _ binders _ _ _ => binders.foldl (init := bound) fun bound binder =>
            bound ++ patternLocals ns binder.pattern
        | .letDecl pattern _ _ => bound ++ patternLocals ns pattern
        | .match_ _ arms => arms.foldl (init := bound) fun bound arm =>
            bound ++ patternLocals ns arm.pattern
        | _ => bound
      (VerexIR.Validation.expressionChildren expression.kind).foldl
        (fun found child => referencedLocals ns child found bound) found

/-- The locals a clause reads at its own state: those outside `old(..)`,
which reads another state. -/
private partial def currentLocals (ns : ValidatedNamespace) (id : VerexIR.ExprId)
    (found : Array Nat := #[]) (bound : Array Nat := #[]) : Array Nat :=
  match ns.expressions[id.index]? with
  | none => found
  | some { kind := .operation (.specification .old) .., .. } => found
  | some expression =>
      let found := match expression.kind with
        | .localVar localId =>
            if found.contains localId.index || bound.contains localId.index then found
            else found.push localId.index
        | _ => found
      let bound := match expression.kind with
        | .quantifier _ binders _ _ _ => binders.foldl (init := bound) fun bound binder =>
            bound ++ patternLocals ns binder.pattern
        | .letDecl pattern _ _ => bound ++ patternLocals ns pattern
        | .match_ _ arms => arms.foldl (init := bound) fun bound arm =>
            bound ++ patternLocals ns arm.pattern
        | _ => bound
      (VerexIR.Validation.expressionChildren expression.kind).foldl
        (fun found child => currentLocals ns child found bound) found

/-- The locals read under `old(..)` within a clause: in a loop invariant,
their values at the function's start. -/
partial def oldReferencedLocals (ns : ValidatedNamespace) (id : VerexIR.ExprId)
    (found : Array Nat := #[]) : Array Nat :=
  match ns.expressions[id.index]? with
  | none => found
  | some expression => match expression.kind with
      | .operation (.specification .old) _ arguments _ =>
          arguments.foldl (fun found argument => referencedLocals ns argument found) found
      | kind => (VerexIR.Validation.expressionChildren kind).foldl
          (fun found child => oldReferencedLocals ns child found) found

/-- Whether an expression reads the function's start state: a behavioral
predicate without a state label reads the invocation from it. -/
partial def readsEntryState (ns : ValidatedNamespace) (id : VerexIR.ExprId) : Bool :=
  match ns.expressions[id.index]? with
  | none => false
  | some expression => match expression.kind with
      | .operation (.specification (.behavior _ range)) _ arguments _ =>
          range.pre.isNone || arguments.any (readsEntryState ns)
      | kind => (VerexIR.Validation.expressionChildren kind).any (readsEntryState ns)

/-- The root local of a place, and whether the place goes through a
dereference of it. -/
private partial def placeRootLocal? (ns : ValidatedNamespace) (place : VerexIR.PlaceId)
    (fuel : Nat := ns.places.size) : Option (Nat × Bool) :=
  match fuel, ns.places[place.index]? with
  | 0, _ | _, none => none
  | _ + 1, some (.localVar localId) => some (localId.index, false)
  | fuel + 1, some (.deref base) =>
      (placeRootLocal? ns base fuel).map fun (root, _) => (root, true)
  | fuel + 1, some (.field base _ _) | fuel + 1, some (.ghostField base _ _)
  | fuel + 1, some (.index base _)
  | fuel + 1, some (.subslice base ..) | fuel + 1, some (.downcast base _) =>
      placeRootLocal? ns base fuel

/-- The locals a loop body can change, each with whether the body only
writes through it as a reference: those it binds, assigns, or lends
mutably, since a lender holds its borrow's prophecy from the borrow on.  A local
written through only keeps its prophecy; every other local keeps its entry
value through the loop. -/
private partial def loopBodyLocals (ns : ValidatedNamespace) (id : VerexIR.ExprId)
    (found : Array (Nat × Bool) := #[]) : Array (Nat × Bool) :=
  match ns.expressions[id.index]? with
  | none => found
  | some expression =>
      let rec bound (pattern : VerexIR.PatternId) : Array (Nat × Bool) :=
        match ns.patterns[pattern.index]? with
        | some { kind := .variable localId, .. } => #[(localId.index, false)]
        | some { kind := .tuple children, .. }
        | some { kind := .constructor _ _ _ children, .. } => children.flatMap bound
        | _ => #[]
      let direct : Array (Nat × Bool) := match expression.kind with
        | .letDecl pattern _ _ => bound pattern
        | .assignPattern pattern _ => bound pattern
        | .assign place _ => (placeRootLocal? ns place).toArray
        | .operation (.borrow .mutable place) _ _ _ => (placeRootLocal? ns place).toArray
        | .operation (.reference .mutate) _ arguments _ =>
            match (arguments[0]?.bind fun target => ns.expressions[target.index]?).map (·.kind) with
            | some (VerexIR.ExprKind.localVar localId) => #[(localId.index, true)]
            | _ => #[]
        -- An `update` writes the ghost field its target place names.
        | .spec block => block.conditions.flatMap fun condition =>
            if condition.kind != .update then #[] else
            match (condition.auxiliary.find? (·.1 == "updateTarget")).bind fun (_, target) =>
                (ns.expressions[target.index]?).map (·.kind) with
            | some (.operation (.read place) _ _ _) => (placeRootLocal? ns place).toArray
            | _ => #[]
        | _ => #[]
      let found := direct.foldl (init := found) fun found (slot, through) =>
        match found.findIdx? (·.1 == slot) with
        | some index => found.set! index (slot, found[index]!.2 && through)
        | none => found.push (slot, through)
      (VerexIR.Validation.expressionChildren expression.kind).foldl
        (fun found child => loopBodyLocals ns child found) found

/-- Whether a specification block writes a spec variable's memory: an
`update` of a field of `global<G>(a)` (`doc/designs/ghost-fields.md`). -/
private def updatesMemory (ns : ValidatedNamespace) (block : VerexIR.SpecBlock) : Bool :=
  block.conditions.any fun condition =>
    condition.kind == .update && (condition.auxiliary.find? (·.1 == "updateTarget")).any
      fun (_, target) =>
        let kindAt (id : VerexIR.ExprId) : Option VerexIR.ExprKind :=
          (ns.expressions[id.index]?).map (·.kind)
        match kindAt target with
        | some (.operation (.data (.select _ _)) _ #[base] _) =>
            (kindAt base).any fun
              | .operation (.specification (.global none)) _ _ _ => true
              | _ => false
        | _ => false

mutual
/-- Whether evaluating an expression may change the global store: through a
global operation, or through a call to a function that may. An invocation
of a `kept` local, a function-typed parameter the body does not reassign,
keeps the store (`KeepsMemoryAt`); building a closure runs nothing, and a
specification block changes only a spec variable's memory, in the model. -/
private partial def mayTouchStore (unit : ValidatedUnit) (namespaceId : NamespaceId)
    (id : VerexIR.ExprId) (visiting : Array FunctionHandle) (kept : Array Nat := #[]) : Bool :=
  match unit.namespaces[namespaceId.index]? with
  | none => true
  | some ns =>
  match ns.expressions[id.index]? with
  | none => true
  | some { kind := .spec block, .. } => updatesMemory ns block
  | some expression =>
      let direct := match expression.kind with
        | .operation (.global _) _ _ _ => true
        | .operation (.call (.function reference)) _ _ _ =>
            match VerexIR.SemanticOperations.resolveFunction? unit namespaceId reference with
            | some callee => calleeMayTouchStore unit callee visiting
            | none => true
        | .operation (.call (.constructor ..)) _ _ _
        | .operation (.call (.destructor ..)) _ _ _
        | .operation (.call (.closure _ _)) _ _ _ => false
        | .operation (.call .invoke) _ arguments _ =>
            match (arguments[0]?.bind fun callee => ns.expressions[callee.index]?).map (·.kind) with
            | some (VerexIR.ExprKind.localVar localId) => !kept.contains localId.index
            | _ => true
        | .operation (.call (.extension ..)) _ _ _ => true
        | _ => false
      direct || (VerexIR.Validation.expressionChildren expression.kind).any
        fun child => mayTouchStore unit namespaceId child visiting kept

/-- Whether a call may change the global store. A callee used through its
contract changes it only where its frame permits; every other callee is
inlined, so its body decides. A callee already on the path adds nothing its
body has not. -/
private partial def calleeMayTouchStore (unit : ValidatedUnit) (callee : FunctionHandle)
    (visiting : Array FunctionHandle) : Bool :=
  if visiting.contains callee then false else
  match unit.namespaces[callee.namespaceId.index]?.bind fun ns =>
      (ns, ·) <$> ns.functions[callee.functionId.index]? with
  | none => true
  | some (ns, declaration) =>
      if usedThroughContract unit callee ns declaration then
        declaration.contract.modifiesAll || !declaration.contract.modifies.isEmpty
      else match declaration.body with
        | .structured root => mayTouchStore unit callee.namespaceId root (visiting.push callee)
        | _ => true
end

/-- The in-body specification blocks of a body stating a condition of a
kind `accept` admits, with their expressions. -/
def bodySpecifications (ns : ValidatedNamespace) (root : VerexIR.ExprId)
    (accept : VerexIR.ConditionKind → Bool) : Array (VerexIR.ExprId × VerexIR.SpecBlock) :=
  Id.run do
    let mut found := #[]
    let mut pending := #[root]
    let mut visited : Std.HashSet Nat := {}
    while let some id := pending.back? do
      pending := pending.pop
      if visited.contains id.index then continue
      visited := visited.insert id.index
      let some expression := ns.expressions[id.index]? | continue
      if let .spec block := expression.kind then
        if block.conditions.any (accept ·.kind) then found := found.push (id, block)
      pending := pending ++ VerexIR.Validation.expressionChildren expression.kind
    return found

/-- The names of the leaves owing a function's clauses, by the clauses' source
ranges: each clause by its kind, numbered in source order among the clauses
of that kind (`ensures_2`, `aborts_if_1`, `invariant_1`, `modifies_1`). -/
def clauseNames (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) :
    Array (VerexIR.Proofs.ObligationRange × String) := Id.run do
  let kindName : VerexIR.ConditionKind → Option String
    | .ensures => some "ensures" | .abortsIf => some "aborts_if"
    | .abortsWith => some "aborts_with" | .succeedsIf => some "succeeds_if"
    | .emits => some "emits" | .decreases => some "decreases"
    | .loopInvariant => some "invariant" | .assertion => some "assert"
    | .apply => some "apply" | .split => some "split"
    | _ => none
  let mut clauses : Array (String × VerexIR.Proofs.ObligationRange) := #[]
  for condition in declaration.contract.conditions do
    if let some kind := kindName condition.kind then
      clauses := clauses.push (kind, Contract.locRange unit condition.loc)
  for target in declaration.contract.modifies do
    if let some expression := ns.expressions[target.index]? then
      clauses := clauses.push ("modifies", Contract.locRange unit expression.loc)
  if let .structured root := declaration.body then
    for (_, block) in bodySpecifications ns root fun _ => true do
      for condition in block.conditions do
        if let some kind := kindName condition.kind then
          clauses := clauses.push (kind, Contract.locRange unit condition.loc)
  let ordered := clauses.qsort fun (_, a) (_, b) =>
    a.file < b.file || (a.file == b.file && a.startByte < b.startByte)
  let mut counts : Std.HashMap String Nat := {}
  let mut names := #[]
  for (kind, range) in ordered do
    if range.startByte ≥ range.endByte || names.any (·.1 == range) then continue
    let index := counts.getD kind 0 + 1
    counts := counts.insert kind index
    names := names.push (range, s!"{kind}_{index}")
  return names

/-- A function's preconditions, each by its number among them in source
order: what a key names a callee's precondition by (`requires 1 of g`). -/
def preconditionNumbers (unit : ValidatedUnit)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) :
    Array (VerexIR.Proofs.ObligationRange × Nat) :=
  let ranges := (declaration.contract.conditions.filter (·.kind == .requires)).map
    fun condition => Contract.locRange unit condition.loc
  let ordered := ranges.qsort fun a b =>
    a.file < b.file || (a.file == b.file && a.startByte < b.startByte)
  ordered.zipIdx.map fun (range, index) => (range, index + 1)

/-- The calls of a body, outside its specifications, in source order: each
call's site, the callee's name, and the call's number among the calls of
the callee and among all calls (`doc/plan/proof-language.md`, "Obligation
addressing"). -/
def callSites (unit : ValidatedUnit) (ns : ValidatedNamespace) (namespaceIndex : Nat)
    (root : VerexIR.ExprId) : Array (Nat × String × Nat × Nat) := Id.run do
  let mut found : Array (VerexIR.Proofs.ObligationRange × Nat × String) := #[]
  let mut pending := #[root]
  let mut visited : Std.HashSet Nat := {}
  while let some id := pending.back? do
    pending := pending.pop
    if visited.contains id.index then continue
    visited := visited.insert id.index
    let some expression := ns.expressions[id.index]? | continue
    if expression.kind matches .spec _ then continue
    if let .operation (.call (.function reference)) _ _ _ := expression.kind then
      if let some handle := VerexIR.SemanticOperations.resolveFunction? unit ⟨namespaceIndex⟩ reference then
        let name := ((unit.namespaces[handle.namespaceId.index]?).bind fun calleeNs =>
          (calleeNs.functions[handle.functionId.index]?).bind fun callee =>
            (calleeNs.tables.names[callee.name.index]?).map (·.name)).getD "callee"
        found := found.push (Contract.locRange unit expression.loc, id.index, name)
    pending := pending ++ VerexIR.Validation.expressionChildren expression.kind
  let ordered := found.qsort fun (a, i, _) (b, j, _) =>
    a.startByte < b.startByte || (a.startByte == b.startByte && i < j)
  let mut counts : Std.HashMap String Nat := {}
  let mut sites := #[]
  for (_, index, name) in ordered, overall in [1:ordered.size + 1] do
    let number := counts.getD name 0 + 1
    counts := counts.insert name number
    sites := sites.push (loopSite namespaceIndex index, name, number, overall)
  return sites

/-- The sites of a body a path's decisions name, by kind in source order:
each site, its kind, and its number among the sites of the kind
(`doc/plan/proof-language.md`, "Obligation addressing"). The kinds are the
compiled body's (`Term.foldCalls`): an `abort` is a throw, of an `abort` or
an `assert!`; an `op` is an operation that may abort; an `if` is a
conditional the compiler takes as one of the source's decisions; a
`loop` is a loop, and the conditional heading a `while` loop is its
`loopHeader`, numbered as the loop; a `match` is one, and the conditional
testing an arm of one an import pass lowered (`ExprOrigin.matchArm`) its
`matchArm`, labeled by the arm's variant, with the untested last arm's
variant as its `matchElse`. -/
def bodySites (unit : ValidatedUnit) (ns : ValidatedNamespace) (namespaceIndex : Nat)
    (root : VerexIR.ExprId) (kinds : Std.HashMap Nat String) :
    Array (Nat × String × Nat × String) := Id.run do
  let kindOf (id : VerexIR.ExprId) := (ns.expressions[id.index]?).map (·.kind)
  let isBreak (id : VerexIR.ExprId) := (kindOf id).any (· matches .break_ ..)
  -- Each site with its kind and source range; a lowered `match`'s arms as
  -- the match, the arm's variant and the untested last arm's.
  let mut found : Array (String × VerexIR.Proofs.ObligationRange × Nat) := #[]
  let mut arms : Array (VerexIR.Proofs.ObligationRange × Nat × String × Option String) := #[]
  let mut headers : Array (Nat × Nat) := #[]
  let mut pending := #[root]
  let mut visited : Std.HashSet Nat := {}
  while let some id := pending.back? do
    pending := pending.pop
    if visited.contains id.index then continue
    visited := visited.insert id.index
    let some expression := ns.expressions[id.index]? | continue
    if expression.kind matches .spec _ then continue
    let range := Contract.locRange unit expression.loc
    match kinds[loopSite namespaceIndex id.index]?, expression.origin with
    | some "if", some (.matchArm site variant last) =>
        arms := arms.push (Contract.locRange unit site, id.index, variant, last)
    | some "abort", some (.matchMismatch _) => pure ()
    | some kind, _ => found := found.push (kind, range, id.index)
    | none, _ => pure ()
    if let .loop _ body := expression.kind then
      if let some (.ifElse _ _ (some elseBranch)) := kindOf body then
        if isBreak elseBranch then headers := headers.push (id.index, body.index)
    pending := pending ++ VerexIR.Validation.expressionChildren expression.kind
  -- A loop's heading is no source conditional.
  found := found.filter fun (kind, _, index) => kind != "if" || !headers.any (·.2 == index)
  -- A lowered `match` is one site, at its source range.
  for (range, _, _, _) in arms do
    unless found.any (fun (kind, other, _) => kind == "match" && other == range) do
      found := found.push ("match", range, 0)
  let ordered := found.qsort fun (_, a, i) (_, b, j) =>
    a.startByte < b.startByte || (a.startByte == b.startByte && i < j)
  let mut counts : Std.HashMap String Nat := {}
  let mut sites := #[]
  let mut numbered : Array (VerexIR.Proofs.ObligationRange × Nat) := #[]
  for (kind, range, index) in ordered do
    let number := counts.getD kind 0 + 1
    counts := counts.insert kind number
    if kind == "match" then numbered := numbered.push (range, number)
    -- A lowered match has no site of its own, only its arms'.
    unless kind == "match" && index == 0 do
      sites := sites.push (loopSite namespaceIndex index, kind, number, "")
  for (range, index, variant, last) in arms do
    if let some (_, number) := numbered.find? (·.1 == range) then
      sites := sites.push (loopSite namespaceIndex index, "matchArm", number, variant)
      if let some last := last then
        sites := sites.push (loopSite namespaceIndex index, "matchElse", number, last)
  for (loopIndex, header) in headers do
    if let some (_, _, number, _) := sites.find? (·.1 == loopSite namespaceIndex loopIndex) then
      sites := sites.push (loopSite namespaceIndex header, "loopHeader", number, "")
  return sites

/-- The types a clause reads the locals of a function at: a reference local
at its referent. -/
private def clauseLocalTypes (unit : ValidatedUnit)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) :
    MetaM (Array VerexIR.Ty) :=
  declaration.locals.mapM fun localDecl => do
    let some ty := unit.tables.types[localDecl.type.typeId.index]?
      | throwError "a local has an unknown type"
    match ty with
    | .reference reference =>
        let some referent := unit.tables.types[reference.referent.index]?
          | throwError "a reference local has an unknown referent type"
        pure referent
    | _ => pure ty

/-- Abstract a predicate over the executable unit when its clauses read it. -/
private def overExecutable (outer : Lean.Expr) (readsUnit : Bool)
    (k : Option Lean.Expr → TermElabM Lean.Expr) : TermElabM Lean.Expr :=
  k (if readsUnit then some outer else none)

/-- Global memory at an executable's unit. -/
private def memoryAt (executable : Lean.Expr) : MetaM Lean.Expr := do
  return mkApp (mkConst ``VerexIR.Proofs.Denote.Memory) (← Contract.executableUnit executable)

/-- The function-typed parameters without a frame that a body does not
reassign: they keep memory. -/
private def loopKeptParameters (ns : ValidatedNamespace) (root : VerexIR.ExprId)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) (params : NRow) :
    Array Nat :=
  let parameterTypes := params.toList
  let written := loopBodyLocals ns root
  (List.range parameterTypes.length).toArray.filter fun index =>
    parameterTypes[index]? matches some (.function ..) && !written.any (·.1 == index) &&
      !declaration.contract.parameterFrames.any (·.parameter.index == index)

/-- Whether an unannotated loop's body runs once: the loop keeps its entry
exactly. -/
private def loopRunsOnce (ns : ValidatedNamespace) (site : VerexIR.ExprId)
    (block : VerexIR.SpecBlock) : Bool :=
  let runsOnce := match ns.expressions[site.index]? with
    | some { kind := .loop _ body, .. } => VerexIR.Validation.loopBodyRunsOnce ns body
    | _ => false
  runsOnce && !block.conditions.any (·.kind == .loopInvariant)

/-- Whether a loop of a function keeps its stored resources' invariants from
the function's start (`loopInvariants`): one that may touch the store. -/
def loopsKeepStoredInvariants (unit : ValidatedUnit) (namespaceId : NamespaceId)
    (ns : ValidatedNamespace) (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (params : NRow) : Bool :=
  match declaration.body with
  | .structured root =>
      let kept := loopKeptParameters ns root declaration params
      (loopSpecifications ns root).any fun (site, block) =>
        !loopRunsOnce ns site block && mayTouchStore unit namespaceId site #[] kept
  | _ => false

/-- The invariant of each loop of a function, keyed by the loop's site:
over the function's starting locals and state (what `old` reads), the loop's
entry locals and state, and the current locals and state; first over the
executable unit where a clause states a behavioral predicate, which the
returned flag marks. -/
def loopInvariants (unit : ValidatedUnit) (outer : Lean.Expr) (namespaceId : NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (params : NRow) (row : NRow) (types : Option Lean.Expr)
    (storedInvariant : Option Lean.Expr) :
    TermElabM (Array (Nat × Lean.Expr × Bool)) := do
  let .structured root := declaration.body | return #[]
  let envType ← mkAppM ``HEnv #[← quoteRow row]
  let argumentsType ← mkAppM ``HList #[← quoteRow params]
  let parameterTypes := params.toList
  let row := row.toList
  let localTypes ← clauseLocalTypes unit declaration
  let mut invariants := #[]
  let kept := loopKeptParameters ns root declaration params
  for (site, block) in loopSpecifications ns root do
    -- An unannotated loop whose body runs once keeps its entry exactly.
    if loopRunsOnce ns site block then
      let stateType ← memoryAt outer
      let invariant ← overExecutable outer false fun _ =>
        withLocalDeclD `start argumentsType fun start =>
        withLocalDeclD `startState stateType fun startState =>
        withLocalDeclD `entry envType fun entry =>
        withLocalDeclD `initial stateType fun initial =>
        withLocalDeclD `env envType fun env =>
        withLocalDeclD `state stateType fun state => do
          mkLambdaFVars #[start, startState, entry, initial, env, state]
            (← mkAppM ``And #[← mkEq env entry, ← mkEq state initial])
      invariants := invariants.push (loopSite namespaceId.index site.index, invariant, false)
      continue
    let changed := loopBodyLocals ns site
    let storeTouched := mayTouchStore unit namespaceId site #[] kept
    let stateType ← memoryAt outer
    let storedReadsUnit := storeTouched && storedInvariant.isSome &&
      Contract.storedInvariantReadsUnit unit
    let readsUnit := storedReadsUnit || block.conditions.any fun condition =>
      condition.kind == .loopInvariant &&
        Contract.conditionReadsBehavior unit namespaceId condition.expression
    let invariant ← overExecutable outer readsUnit fun executable =>
      withLocalDeclD `start argumentsType fun start =>
      withLocalDeclD `startState stateType fun startState =>
      withLocalDeclD `entry envType fun entry =>
      withLocalDeclD `initial stateType fun initial =>
      withLocalDeclD `env envType fun env =>
      withLocalDeclD `state stateType fun state => do
        let entrySlots ← slotProjections entry row.length
        let startSlots ← slotProjections start parameterTypes.length
        let slots ← slotProjections env row.length
        let mut frame := #[]
        for (localDecl, index) in declaration.locals.zipIdx do
          if h : index < row.length then
            match changed.find? (·.1 == localDecl.id.index), row[index] with
            | none, _ => frame := frame.push (← mkEq slots[index]! entrySlots[index]!)
            | some (_, true), .ref referent =>
                let carrier ← mkAppM ``NTy.carrier #[← quoteNTy referent]
                let prophecyOf (slot : Lean.Expr) : MetaM Lean.Expr := do
                  mkAppM ``Option.map
                    #[mkAppN (mkConst ``Prod.snd [.zero, .zero]) #[carrier, carrier], slot]
                frame := frame.push
                  (← mkEq (← prophecyOf slots[index]!) (← prophecyOf entrySlots[index]!))
            | _, _ => pure ()
        -- A local the body modifies whose type declares data invariants
        -- keeps them across the iterations, as a clause would state.
        let header := (loopHeaderLocals ns root site
          ((Array.range parameterTypes.length).map (⟨·⟩))).getD #[]
        -- A slot carrying a data invariant, at its native type, a
        -- reference's referent's.
        let dataInvariantSlots := changed.filterMap fun (slot, _) =>
          if header.any (·.index == slot) &&
              localTypes[slot]?.any (Contract.hasDataInvariant unit ·) then
            row[slot]?.map fun
              | .ref referent => (slot, referent)
              | τ => (slot, τ)
          else none
        let referenced := block.conditions.foldl (fun found condition =>
          if condition.kind == .loopInvariant then currentLocals ns condition.expression found
          else found) (dataInvariantSlots.map (·.1))
        let oldReferenced := block.conditions.foldl (fun found condition =>
          if condition.kind == .loopInvariant then oldReferencedLocals ns condition.expression found
          else found) #[]
        let rec authoredOver (index : Nat) (binders entryBinders : Array (Option Lean.Expr)) :
            TermElabM Lean.Expr := do
          if h : index < row.length then
            let τ := row[index]
            let name := match declaration.locals[index]? with
              | some localDecl => Name.mkSimple localDecl.name
              | none => Name.mkSimple s!"slot{index}"
            let carrier ← mkAppM ``NTy.carrier #[← quoteNTy τ]
            -- A slot a clause reads under `old` is asserted defined at the
            -- loop's entry, where its value is read.
            let withEntry (binders : Array (Option Lean.Expr)) : TermElabM Lean.Expr := do
              if oldReferenced.contains index then
                withLocalDeclD (name.appendAfter "_entry") carrier fun entryBinder => do
                  let inner ← authoredOver (index + 1) binders (entryBinders.push (some entryBinder))
                  mkAppM ``Option.elim
                    #[entrySlots[index]!, mkConst ``False, ← mkLambdaFVars #[entryBinder] inner]
              else authoredOver (index + 1) binders (entryBinders.push none)
            if referenced.contains index then
              -- A slot a clause reads is asserted defined, so that an
              -- iteration knows its value before the body reads it; the
              -- binder carries the local's name into an authored proof.
              withLocalDeclD name carrier fun binder => do
                let inner ← withEntry (binders.push (some binder))
                mkAppM ``Option.elim
                  #[slots[index]!, mkConst ``False, ← mkLambdaFVars #[binder] inner]
            else withEntry (binders.push none)
          else
            let mut locals : Array (Option Lean.Expr) := #[]
            for (τ, binder) in row.toArray.zip binders do
              locals := locals.push (← match binder with
                | some binder => logicalValue τ binder
                | none => pure none)
            let mut entryLocals : Array (Option Lean.Expr) := #[]
            for (τ, binder) in row.toArray.zip entryBinders do
              entryLocals := entryLocals.push (← match binder with
                | some binder => logicalValue τ binder
                | none => pure none)
            -- A parameter read under `old` is its argument at the function's
            -- start; a local has no such value and is read at the loop's
            -- entry.
            let mut oldLocals : Array (Option Lean.Expr) := #[]
            for index in [0:row.length] do
              if oldReferenced.contains index then
                match parameterTypes[index]? with
                | some τ => oldLocals := oldLocals.push (← logicalValue τ startSlots[index]!)
                | none => oldLocals := oldLocals.push entryLocals[index]!
              else oldLocals := oldLocals.push none
            translateLoopInvariants unit namespaceId ns block locals localTypes types
              (declaration.locals.map (·.name)) oldLocals startState
              dataInvariantSlots entryLocals (some initial) (some state) executable
        let authored ← authoredOver 0 #[] #[]
        -- The store the body leaves alone keeps its entry value; one it
        -- changes keeps the data invariants of its stored resources, where
        -- they held at the function's start.
        let mut stateFrame : Array Lean.Expr := #[]
        if storeTouched then
          if let some invariant := storedInvariant then
            let invariant ← Contract.storedInvariantAt unit invariant executable state
            stateFrame := stateFrame.push (← mkArrow
              (← mkAppM ``VerexIR.Proofs.Denote.MemoryInvariants #[invariant, startState])
              (← mkAppM ``VerexIR.Proofs.Denote.MemoryInvariants #[invariant, state]))
        else
          stateFrame := stateFrame.push (← mkEq state initial)
        let conjunction ← ((frame.push authored) ++ stateFrame).foldrM (init := mkConst ``True)
          fun clause rest => mkAppM ``And #[clause, rest]
        mkLambdaFVars #[start, startState, entry, initial, env, state] conjunction
    invariants := invariants.push (loopSite namespaceId.index site.index, invariant, readsUnit)
  return invariants

/-- The labels of the state anchors an expression tree reads. -/
private partial def anchorLabels (ns : ValidatedNamespace) (id : VerexIR.ExprId)
    (found : Array Nat := #[]) : Array Nat :=
  match ns.expressions[id.index]? with
  | none => found
  | some expression =>
      let found := match expression.kind with
        | .operation (.specification (.withStateAnchor label)) _ _ _ =>
            if found.contains label then found else found.push label
        | _ => found
      (VerexIR.Validation.expressionChildren expression.kind).foldl
        (fun found child => anchorLabels ns child found) found

/-- The locals read under `old(..)` within the state anchor `anchor`, or,
for `none`, outside every anchor; `within` is the anchor around `id`. -/
private partial def oldLocalsAt (ns : ValidatedNamespace) (id : VerexIR.ExprId)
    (anchor : Option Nat) (within : Option Nat := none) (found : Array Nat := #[]) :
    Array Nat :=
  match ns.expressions[id.index]? with
  | none => found
  | some expression => match expression.kind with
      | .operation (.specification (.withStateAnchor label)) _ arguments _ =>
          arguments.foldl (fun found argument => oldLocalsAt ns argument anchor (some label) found)
            found
      | .operation (.specification .old) _ arguments _ =>
          if within == anchor then
            arguments.foldl (fun found argument => referencedLocals ns argument found) found
          else found
      | kind => (VerexIR.Validation.expressionChildren kind).foldl
          (fun found child => oldLocalsAt ns child anchor within found) found

/-- The site of the specification block saving the state anchor `label`. -/
private def anchorSite? (namespaceId : NamespaceId) (ns : ValidatedNamespace)
    (root : VerexIR.ExprId) (label : Nat) : Option Nat :=
  let saves (condition : VerexIR.Condition) : Bool :=
    condition.kind matches .assumption &&
      match ns.expressions[condition.expression.index]? with
      | some { kind := .operation (.specification (.saveStateAnchor saved)) _ _ _, .. } =>
          saved == label
      | _ => false
  (bodySpecifications ns root (· matches .assumption)).findSome? fun (id, block) =>
    if block.conditions.any saves then some (loopSite namespaceId.index id.index) else none

/-- Assert the slots `referenced` of a row defined, each value bound under
its local's name with `suffix`, and continue with the clause-level values
of the bound slots. -/
private def overDefined (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (row : List NTy) (slots : Array Lean.Expr) (referenced : Array Nat) (suffix : String)
    (k : Array (Option Lean.Expr) → TermElabM Lean.Expr) : TermElabM Lean.Expr :=
  let rec go (index : Nat) (values : Array (Option Lean.Expr)) : TermElabM Lean.Expr := do
    if h : index < row.length then
      if referenced.contains index then
        let name := match declaration.locals[index]? with
          | some localDecl => Name.mkSimple (localDecl.name ++ suffix)
          | none => Name.mkSimple s!"slot{index}{suffix}"
        let carrier ← mkAppM ``NTy.carrier #[← quoteNTy row[index]]
        withLocalDeclD name carrier fun binder => do
          let inner ← go (index + 1) (values.push (← logicalValue row[index] binder))
          mkAppM ``Option.elim #[slots[index]!, mkConst ``False, ← mkLambdaFVars #[binder] inner]
      else go (index + 1) (values.push none)
    else k values
  go 0 #[]

/-- The condition of each in-body assertion of a function, keyed by its
site: over the function's starting arguments and state (what `old` reads),
the locals and state each state anchor it reads recorded (an `AnchorOf` its
site), and the current locals and state; first over the executable unit
where a clause states a behavioral predicate, which the returned flag
marks. A slot a clause reads is asserted defined. -/
def assertionConditions (unit : ValidatedUnit) (outer : Lean.Expr) (namespaceId : NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (params : NRow) (row : NRow) (types : Option Lean.Expr)
    (ownProof : Bool := true) :
    TermElabM (Array (Nat × Lean.Expr × Bool)) := do
  let .structured root := declaration.body | return #[]
  let rowType ← quoteRow row
  let envType ← mkAppM ``HEnv #[rowType]
  let argumentsType ← mkAppM ``HList #[← quoteRow params]
  let stateType ← memoryAt outer
  let parameterTypes := params.toList
  let row := row.toList
  let localTypes ← clauseLocalTypes unit declaration
  let mut conditions := #[]
  for (site, block) in bodySpecifications ns root (· matches .assertion | .apply | .split) do
    -- A step of an inlined callee's proof states nothing in its caller.
    let asserted := if block.isProof && !ownProof then #[]
      else block.conditions.filter (·.kind matches .assertion | .apply | .split)
    let readsUnit := asserted.any fun condition =>
      Contract.conditionReadsBehavior unit namespaceId condition.expression
    if asserted.any (fun condition =>
        (oldLocalsAt ns condition.expression none).any (· ≥ parameterTypes.length)) then
      throwError "an in-body assertion reads a local under `old`, which only a parameter has"
    let labels := asserted.foldl
      (fun found condition => anchorLabels ns condition.expression found) #[]
    let anchors ← labels.mapM fun label => do
      let some anchorSite := anchorSite? namespaceId ns root label
        | throwError m!"an in-body assertion reads the state anchor {label}, which its \
            function does not save"
      pure (label, ← mkAppM ``AnchorOf #[Lean.toExpr anchorSite, rowType])
    let first := fun (e : Lean.Expr) =>
      mkApp3 (mkConst ``Prod.fst [.zero, .zero]) envType stateType e
    let second := fun (e : Lean.Expr) =>
      mkApp3 (mkConst ``Prod.snd [.zero, .zero]) envType stateType e
    let condition ← overExecutable outer readsUnit fun executable =>
      withLocalDeclD `start argumentsType fun start =>
      withLocalDeclD `startState stateType fun startState => do
        let rec withAnchors (index : Nat) (bound : Array Lean.Expr) : TermElabM Lean.Expr := do
          if h : index < anchors.size then
            let (label, anchorType) := anchors[index]
            withLocalDeclD (Name.mkSimple s!"anchor{label}") anchorType fun anchor =>
              withAnchors (index + 1) (bound.push anchor)
          else
            withLocalDeclD `env envType fun env =>
            withLocalDeclD `state stateType fun state => do
              let startSlots ← slotProjections start parameterTypes.length
              -- Each assertion is its own clause, the slots it reads asserted
              -- defined inside its marker.
              let clauses ← asserted.mapM fun asserted => do
                let oldReferenced := oldLocalsAt ns asserted.expression none
                let oldLocals ← (Array.range row.length).mapM fun index =>
                  match parameterTypes[index]? with
                  | some τ => if oldReferenced.contains index then
                      logicalValue τ startSlots[index]! else pure none
                  | none => pure none
                let rec overAnchors (index : Nat)
                    (labeled : Array (Nat × Array (Option Lean.Expr) × Lean.Expr)) :
                    TermElabM Lean.Expr := do
                  if h : index < anchors.size then
                    let (label, _) := anchors[index]
                    let anchor := bound[index]!
                    overDefined declaration row (← slotProjections (first anchor) row.length)
                      (oldLocalsAt ns asserted.expression (some label)) s!"_at{label}"
                      fun anchorLocals =>
                        overAnchors (index + 1) (labeled.push (label, anchorLocals, second anchor))
                  else
                    overDefined declaration row (← slotProjections env row.length)
                      (currentLocals ns asserted.expression) "" fun locals =>
                        translateLoopInvariants unit namespaceId ns
                          { block with conditions := #[asserted] } locals localTypes
                          types (declaration.locals.map (·.name)) oldLocals startState
                          (state := some state) (executable := executable)
                          (kind := asserted.kind) (labeledAnchors := labeled)
                return Contract.markCondition unit asserted (← overAnchors 0 #[])
              let conjunction ← clauses.foldrM (init := mkConst ``True)
                fun clause rest => mkAppM ``And #[clause, rest]
              mkLambdaFVars (bound ++ #[env, state]) conjunction
        mkLambdaFVars #[start, startState] (← withAnchors 0 #[])
    conditions := conditions.push (loopSite namespaceId.index site.index, condition, readsUnit)
  return conditions

/-- The precondition an inlined callee's caller owes where the callee
starts, as the Move Prover asserts it at the call: its `requires` clauses
over its arguments and the memory at its start
(`Contract.startPrecondition`), if it states any. -/
def preconditionCondition (unit : ValidatedUnit) (outer : Lean.Expr) (namespaceId : NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (params : NRow) (types : Option Lean.Expr) :
    TermElabM (Option (Lean.Expr × Bool)) := do
  let required := (ContractView.interface.of declaration).contract.conditions.filter
    fun condition => condition.kind == .requires &&
      !Contract.conditionReadsRequires unit namespaceId condition.expression
  if required.isEmpty then return none
  let readsUnit := required.any fun condition =>
    Contract.conditionReadsBehavior unit namespaceId condition.expression
  let argumentsType ← mkAppM ``HList #[← quoteRow params]
  let stateType ← memoryAt outer
  let parameterTypes := params.toList
  let localTypes ← clauseLocalTypes unit declaration
  let condition ← overExecutable outer readsUnit fun executable =>
    withLocalDeclD `start argumentsType fun start =>
    withLocalDeclD `startState stateType fun startState => do
      let startSlots ← slotProjections start parameterTypes.length
      let locals ← (Array.range declaration.locals.size).mapM fun index =>
        match parameterTypes[index]? with
        | some τ => logicalValue τ startSlots[index]!
        | none => pure none
      mkLambdaFVars #[start, startState] (← Contract.startPrecondition unit namespaceId ns
        declaration locals localTypes types startState executable)
  return some (condition, readsUnit)

/-- Whether a condition of a specification block marks the state or a
derivation instead of stating an assumption. -/
private def isMarker (ns : ValidatedNamespace) (condition : VerexIR.Condition) : Bool :=
  (ns.expressions[condition.expression.index]?).any fun expression =>
    expression.kind matches .operation (.specification (.saveStateAnchor _)) _ _ _ ||
      expression.kind matches .operation (.specification (.foldsCaptureAnchor _)) _ _ _ ||
      expression.kind matches .operation (.specification .inlineCallSummary) _ _ _

/-- The in-body assumptions of a body: each block stating one, with the
conditions it assumes. -/
private def bodyAssumptions (ns : ValidatedNamespace) (root : VerexIR.ExprId) :
    Array (VerexIR.ExprId × VerexIR.SpecBlock) :=
  (bodySpecifications ns root (· matches .assumption)).filterMap fun (id, block) =>
    let assumed := block.conditions.filter fun condition =>
      condition.kind matches .assumption && !isMarker ns condition
    if assumed.isEmpty then none else some (id, { block with conditions := assumed })

/-- Whether a function's body states an in-body assumption. -/
def statesAssumptions (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Bool :=
  match declaration.body with
  | .structured root => !(bodyAssumptions ns root).isEmpty
  | _ => false

/-- The conditions of a function's in-body assumptions as the meanings read
them (`Meanings.assumption`): over the executable unit, the function's
starting arguments and state (what `old` reads), and a site, the condition
of the assumption there over the locals and state. A slot a condition reads
is assumed defined. -/
def assumptionTable (unit : ValidatedUnit) (outer : Lean.Expr) (namespaceId : NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (params : NRow) (row : NRow) (types : Option Lean.Expr)
    : TermElabM Lean.Expr := do
  let .structured root := declaration.body | throwError "an in-body assumption needs a body"
  let rowType ← quoteRow row
  let envType ← mkAppM ``HEnv #[rowType]
  let argumentsType ← mkAppM ``HList #[← quoteRow params]
  let stateType ← memoryAt outer
  let parameterTypes := params.toList
  let rowTypes := row.toList
  let localTypes ← clauseLocalTypes unit declaration
  let entryType ← withLocalDeclD `Γ (mkConst ``NRow) fun context => do
    mkLambdaFVars #[context]
      (← mkArrow (← mkAppM ``HEnv #[context]) (← mkArrow stateType (mkSort .zero)))
  let sigmaType ← mkAppOptM ``Sigma #[mkConst ``NRow, entryType]
  withLocalDeclD `start argumentsType fun start =>
  withLocalDeclD `startState stateType fun startState =>
  withLocalDeclD `site (mkConst ``Nat) fun site => do
    let mut table ← mkAppOptM ``Option.none #[sigmaType]
    for (id, block) in (bodyAssumptions ns root).reverse do
      if block.conditions.any (fun condition =>
          (oldLocalsAt ns condition.expression none).any (· ≥ parameterTypes.length)) then
        throwError "an in-body assumption reads a local under `old`, which only a parameter has"
      unless (block.conditions.foldl
          (fun found condition => anchorLabels ns condition.expression found) #[]).isEmpty do
        throwError "an in-body assumption reads a state anchor, which the reading of its \
          function under its assumptions does not record"
      let condition ← withLocalDeclD `env envType fun env =>
        withLocalDeclD `state stateType fun state => do
          let startSlots ← slotProjections start parameterTypes.length
          let clauses ← block.conditions.mapM fun assumed => do
            let oldReferenced := oldLocalsAt ns assumed.expression none
            let oldLocals ← (Array.range rowTypes.length).mapM fun index =>
              match parameterTypes[index]? with
              | some τ => if oldReferenced.contains index then
                  logicalValue τ startSlots[index]! else pure none
              | none => pure none
            overDefined declaration rowTypes (← slotProjections env rowTypes.length)
              (currentLocals ns assumed.expression) "" fun locals =>
                translateLoopInvariants unit namespaceId ns { block with conditions := #[assumed] }
                  locals localTypes types (declaration.locals.map (·.name)) oldLocals
                  startState (state := some state) (executable := some outer)
                  (kind := .assumption)
          let conjunction ← clauses.foldrM (init := mkConst ``True)
            fun clause rest => mkAppM ``And #[clause, rest]
          mkLambdaFVars #[env, state] conjunction
      let entry ← mkAppOptM ``Sigma.mk #[mkConst ``NRow, entryType, rowType, condition]
      table ← mkAppM ``ite #[← mkEq site (toExpr (loopSite namespaceId.index id.index)),
        ← mkAppM ``Option.some #[entry], table]
    mkLambdaFVars #[start, startState, site] table

/-- The expressions of a body, each once. -/
def bodyExpressionIds (ns : ValidatedNamespace) (root : VerexIR.ExprId) :
    Array VerexIR.ExprId := Id.run do
  let mut found := #[]
  let mut pending := #[root]
  let mut visited : Std.HashSet Nat := {}
  while let some id := pending.back? do
    pending := pending.pop
    if visited.contains id.index then continue
    visited := visited.insert id.index
    let some expression := ns.expressions[id.index]? | continue
    found := found.push id
    pending := pending ++ VerexIR.Validation.expressionChildren expression.kind
  return found

/-- An optional value of `resultType` over a row of locals, each local it
reads bound to its value: none where one is undefined (`overDefined`, for a
value). -/
private def valueOverDefined (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (row : List NTy) (slots : Array Lean.Expr) (referenced : Array Nat) (resultType : Lean.Expr)
    (k : Array (Option Lean.Expr) → TermElabM Lean.Expr) : TermElabM Lean.Expr :=
  let rec go (index : Nat) (values : Array (Option Lean.Expr)) : TermElabM Lean.Expr := do
    if h : index < row.length then
      if referenced.contains index then
        let name := match declaration.locals[index]? with
          | some localDecl => Name.mkSimple localDecl.name
          | none => Name.mkSimple s!"slot{index}"
        let carrier ← mkAppM ``NTy.carrier #[← quoteNTy row[index]]
        withLocalDeclD name carrier fun binder => do
          let inner ← go (index + 1) (values.push (← logicalValue row[index] binder))
          mkAppOptM ``Option.bind #[some carrier, some resultType, some slots[index]!,
            some (← mkLambdaFVars #[binder] inner)]
      else go (index + 1) (values.push none)
    else k values
  go 0 #[]

/-- The sites of a model's ghost writes, each with its update's source
range, where a value out of the field's domain is reported. -/
def ghostWriteSites (unit : ValidatedUnit)
    (owners : Array (FunctionHandle × ValidatedNamespace ×
      VerexIR.FunctionDecl VerexIR.Validation.FunctionBody × Function unit)) :
    Array (Nat × VerexIR.Proofs.ObligationRange) := Id.run do
  let mut sites := #[]
  for (owner, ns, declaration, _) in owners do
    let .structured root := declaration.body | continue
    for id in bodyExpressionIds ns root do
      let some { kind := .spec block, .. } := ns.expressions[id.index]? | continue
      for condition in block.conditions do
        if condition.kind == .update then
          sites := sites.push (loopSite owner.namespaceId.index condition.expression.index,
            Contract.locRange unit condition.loc)
  return sites

/-- The ghost tables of a model's bodies (`Meanings.ghost`,
`Meanings.ghostInit`) over the executable unit: each `update` the value its
right side denotes over its function's locals and state; each pack of a value
with initialized ghost fields, keyed by the field, the initializer's value over
the packed value (`doc/designs/ghost-fields.md`). A value the slot's type does not
decode is none: the write takes any value. -/
def ghostTables (unit : ValidatedUnit) (outer : Lean.Expr)
    (owners : Array (FunctionHandle × ValidatedNamespace ×
      VerexIR.FunctionDecl VerexIR.Validation.FunctionBody × Function unit))
    (types : Option Lean.Expr) :
    TermElabM (Lean.Expr × Lean.Expr) := do
  let stateType ← memoryAt outer
  let ghostFamily ← withLocalDeclD `Γ (mkConst ``NRow) fun context => do
    let inner ← withLocalDeclD `τ (mkConst ``NTy) fun τ => do
      mkLambdaFVars #[τ] (← mkArrow (← mkAppM ``HEnv #[context])
        (← mkArrow stateType (← mkAppM ``Option #[← mkAppM ``NTy.carrier #[τ]])))
    mkLambdaFVars #[context] (← mkAppOptM ``Sigma #[mkConst ``NTy, inner])
  let ghostEntryType ← mkAppOptM ``Sigma #[mkConst ``NRow, ghostFamily]
  let initFamily ← withLocalDeclD `τ (mkConst ``NTy) fun τ => do
    let inner ← withLocalDeclD `σ (mkConst ``NTy) fun σ => do
      mkLambdaFVars #[σ] (← mkArrow (← mkAppM ``NTy.carrier #[τ])
        (← mkArrow stateType (← mkAppM ``Option #[← mkAppM ``NTy.carrier #[σ]])))
    mkLambdaFVars #[τ] (← mkAppOptM ``Sigma #[mkConst ``NTy, inner])
  let initEntryType ← mkAppOptM ``Sigma #[mkConst ``NTy, initFamily]
  withLocalDeclD `site (mkConst ``Nat) fun site => do
    let mut ghosts ← mkAppOptM ``Option.none #[ghostEntryType]
    let mut inits ← mkAppOptM ``Option.none #[initEntryType]
    for (owner, ns, declaration, compiled) in owners do
      let .structured root := declaration.body | continue
      let some view := compileNamespaceAt? unit owner.namespaceId | continue
      let row := compiled.params ++ compiled.locals
      let rowTypes := row.toList
      let localTypes ← clauseLocalTypes unit declaration
      for id in bodyExpressionIds ns root do
        let some expression := ns.expressions[id.index]? | continue
        match expression.kind with
        | .spec block =>
            for condition in block.conditions do
              unless condition.kind == .update do continue
              let some (_, target) := condition.auxiliary.find? (·.1 == "updateTarget") | continue
              -- A ghost field's type, or a spec variable's: its memory's field.
              let τ? : Option NTy := match ns.expressions[target.index]? with
                | some { kind := .operation (.read place) _ _ _, .. } =>
                    match compilePlace unit row view view.fuel place with
                    | .ok ⟨_, τ, _, _⟩ => some τ
                    | .error _ => none
                | some { kind := .operation (.data (.select _ _)) _ #[base] _, .. } =>
                    let baseKind : Option VerexIR.ExprKind :=
                      (ns.expressions[base.index]?).map (·.kind)
                    match baseKind with
                    | some (.operation (.specification (.global none)) #[.typeArg resource] _ _) =>
                        match view.resource unit owner.namespaceId resource.typeId with
                        | some (.struct _ _ (.cons τ .nil), _) => some τ
                        | _ => none
                    | _ => none
                | _ => none
              let some τ := τ? | continue
              let τE ← quoteNTy τ
              let carrier ← mkAppM ``NTy.carrier #[τE]
              let value ← withLocalDeclD `env (← mkAppM ``HEnv #[← quoteRow row]) fun env =>
                withLocalDeclD `state stateType fun state => do
                  let body ← valueOverDefined declaration rowTypes
                    (← slotProjections env rowTypes.length) (currentLocals ns condition.expression)
                    carrier fun locals => do
                      let written ← Contract.translateUpdateValue unit owner.namespaceId ns
                        condition.expression locals localTypes types
                        (declaration.locals.map (·.name)) state (some outer) τ
                      mkAppM ``Option.some #[written]
                  mkLambdaFVars #[env, state] body
              let entry ← mkAppOptM ``Sigma.mk #[mkConst ``NRow, ghostFamily, ← quoteRow row,
                ← mkAppOptM ``Sigma.mk #[mkConst ``NTy, ghostFamily.bindingBody!.instantiate1
                  (← quoteRow row) |>.appArg!, τE, value]]
              ghosts ← mkAppM ``ite #[← mkEq site
                (toExpr (loopSite owner.namespaceId.index condition.expression.index)),
                ← mkAppM ``Option.some #[entry], ghosts]
        | .operation (.call (.constructor reference variant)) _ _ _ =>
            let some handle := VerexIR.SemanticOperations.resolveStruct? unit owner.namespaceId
                reference | continue
            let ghostDecls := VerexIR.SemanticOperations.nominalGhosts unit handle
            let some τ := view.nty unit owner.namespaceId expression.typeId | continue
            let some ty := unit.tables.types[expression.typeId.index]? | continue
            let fields : Option NRow := match τ, variant with
              | .struct _ _ fields, none => some fields
              | .enum _ _ names rows _, some name => match Which.ofName names rows name with
                  | some ⟨row, _⟩ => some row
                  | none => none
              | _, _ => none
            let some fields := fields | continue
            let declaredCount := fields.toList.length - ghostDecls.size
            for (ghost, k) in ghostDecls.zipIdx do
              let some initializer := ghost.initializer | continue
              let some (.ghost σ) := fields.toList[declaredCount + k]? | continue
              let τE ← quoteNTy τ
              let σE ← quoteNTy σ
              let value ← withLocalDeclD `packed (← mkAppM ``NTy.carrier #[τE]) fun packed =>
                withLocalDeclD `state stateType fun state => do
                  let initial ← Contract.translateGhostInitializer unit types ty variant
                    initializer τ σ packed state (some outer)
                  mkLambdaFVars #[packed, state] (← mkAppM ``Option.some #[initial])
              let entry ← mkAppOptM ``Sigma.mk #[mkConst ``NTy, initFamily, τE,
                ← mkAppOptM ``Sigma.mk #[mkConst ``NTy, initFamily.bindingBody!.instantiate1 τE
                  |>.appArg!, σE, value]]
              inits ← mkAppM ``ite #[← mkEq site
                (toExpr (ghostInitSite owner.namespaceId.index id.index k)),
                ← mkAppM ``Option.some #[entry], inits]
        | _ => pure ()
    return (← mkLambdaFVars #[site] ghosts, ← mkLambdaFVars #[site] inits)

/-- The data invariant owed where a mutation of a local whose type carries
one ends: where a loan of the local dies, keyed by the loan, and after a
write into the local, keyed by the write (`Term.mutationEnd`). Each is over
the function's starting arguments and state and the current locals and
state; first over the executable unit where a data invariant states a
behavioral predicate, which the returned flag marks. -/
def mutationConditions (unit : ValidatedUnit) (outer : Lean.Expr) (handle : FunctionHandle)
    (view : CompileNamespace)
    (ns : ValidatedNamespace) (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (params : NRow) (row : NRow) (types : Option Lean.Expr)
    : TermElabM (Array (Nat × Lean.Expr × Bool)) := do
  let .structured root := declaration.body | return #[]
  let namespaceId := handle.namespaceId
  let rowType ← quoteRow row
  let row := row.toList
  let carries (slot : VerexIR.LocalId) : Bool := (row[slot.index]?).any view.carriesInvariant
  let mut sites : Array (Nat × Nat) := #[]
  for fact in loanFactsOf unit handle do
    if let some lender := view.lender? fact then
      if carries lender then
        sites := sites.push (loopSite namespaceId.index fact.expression.index, lender.index)
  for id in bodyExpressionIds ns root do
    if let some { kind := .assign place _, .. } := ns.expressions[id.index]? then
      unless view.place? place matches some (.localVar _) do
        if let some owner := view.ownedRoot? view.placeFuel place then
          if carries owner then sites := sites.push (loopSite namespaceId.index id.index, owner.index)
  if sites.isEmpty then return #[]
  let envType ← mkAppM ``HEnv #[rowType]
  let argumentsType ← mkAppM ``HList #[← quoteRow params]
  let stateType ← memoryAt outer
  let localTypes ← clauseLocalTypes unit declaration
  let readsUnit := Contract.storedInvariantReadsUnit unit
  let block : VerexIR.SpecBlock := { loc := ⟨0⟩ }
  let mut conditions := #[]
  let mut seen : Array Nat := #[]
  for (site, slot) in sites do
    if seen.contains site then continue
    seen := seen.push site
    let condition ← overExecutable outer readsUnit fun executable =>
      withLocalDeclD `start argumentsType fun start =>
      withLocalDeclD `startState stateType fun startState =>
      withLocalDeclD `env envType fun env =>
      withLocalDeclD `state stateType fun state => do
        let owed ← overDefined declaration row (← slotProjections env row.length) #[slot] ""
          fun locals => translateLoopInvariants unit namespaceId ns block locals localTypes
            types (declaration.locals.map (·.name)) #[] startState
            ((row[slot]?.map fun | .ref referent => (slot, referent) | τ => (slot, τ)).toArray)
            (state := some state) (executable := executable) (kind := .assertion)
        mkLambdaFVars #[start, startState, env, state] owed
    conditions := conditions.push (site, condition, readsUnit)
  return conditions

/-- The data invariant owed of each value a function constructs whose type
carries one, keyed by the construction (`Term.constructed`): over the
function's starting arguments and state, the value, and the state; first
over the executable unit where a data invariant states a behavioral
predicate, which the returned flag marks. -/
def constructionConditions (unit : ValidatedUnit) (outer : Lean.Expr) (handle : FunctionHandle)
    (view : CompileNamespace)
    (ns : ValidatedNamespace) (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (params : NRow) (types : Option Lean.Expr) :
    TermElabM (Array (Nat × Lean.Expr × Bool)) := do
  let .structured root := declaration.body | return #[]
  let namespaceId := handle.namespaceId
  let argumentsType ← mkAppM ``HList #[← quoteRow params]
  let stateType ← memoryAt outer
  let readsUnit := Contract.storedInvariantReadsUnit unit
  let mut conditions := #[]
  for id in bodyExpressionIds ns root do
    let some expression := ns.expressions[id.index]? | continue
    unless expression.kind matches .operation (.call (.constructor _ _)) _ _ _ do continue
    let .ok (some τ) := typeOf? unit view namespaceId expression.typeId | continue
    unless view.carriesInvariant τ do continue
    let some ty := ns.tables.types[expression.typeId.index]? | continue
    let condition ← overExecutable outer readsUnit fun executable =>
      withLocalDeclD `start argumentsType fun start =>
      withLocalDeclD `startState stateType fun startState => do
        let carrier ← mkAppM ``NTy.carrier #[← quoteNTy τ]
        withLocalDeclD `value carrier fun value =>
        withLocalDeclD `state stateType fun state => do
          let owed ← Contract.valueDataInvariants unit namespaceId ns types ty τ
            value state executable
          mkLambdaFVars #[start, startState, value, state] owed
    conditions := conditions.push (loopSite namespaceId.index id.index, condition, readsUnit)
  return conditions

/-- The namespace invariants a function owes where a write of global memory
ends (`Term.memoryWritten`), keyed by the write: after a publication or a
take, and where a mutable borrow of a resource dies. Each is over the
function's starting arguments and state, the anchor the write saved before
it, and the locals and state after it; first over the executable unit where
an invariant states a behavioral predicate, which the returned flag marks. -/
def memoryConditions (unit : ValidatedUnit) (outer : Lean.Expr) (handle : FunctionHandle)
    (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (params : NRow) (row : NRow) (types : Option Lean.Expr)
    : TermElabM (Array (Nat × Lean.Expr × Bool)) := do
  let .structured root := declaration.body | return #[]
  let namespaceId := handle.namespaceId
  unless unit.namespaces.any (!·.invariants.isEmpty) do return #[]
  let readsUnit := unit.namespaces.toList.zipIdx.any fun (invariantNs, index) =>
    invariantNs.invariants.any fun invariant =>
      Contract.conditionReadsBehavior unit ⟨index⟩ invariant.condition.expression
  let rowType ← quoteRow row
  let envType ← mkAppM ``HEnv #[rowType]
  let argumentsType ← mkAppM ``HList #[← quoteRow params]
  let stateType ← memoryAt outer
  let mut conditions := #[]
  for id in bodyExpressionIds ns root do
    let some expression := ns.expressions[id.index]? | continue
    let .operation (.global kind) instantiations _ _ := expression.kind | continue
    unless kind matches .publish | .take | .borrow .mutable do continue
    let some (VerexIR.GenericArgument.typeArg resource) := instantiations[0]? | continue
    let some ty := unit.tables.types[resource.typeId.index]? | continue
    let some (written, _, _) := Contract.nominalDeclaration? unit ty | continue
    let site := loopSite namespaceId.index id.index
    let anchorType ← mkAppM ``AnchorOf #[Lean.toExpr site, rowType]
    let condition ← overExecutable outer readsUnit fun executable =>
      withLocalDeclD `start argumentsType fun start =>
      withLocalDeclD `startState stateType fun startState =>
      withLocalDeclD `anchor anchorType fun anchor =>
      withLocalDeclD `env envType fun env =>
      withLocalDeclD `state stateType fun state => do
        let before := mkApp3 (mkConst ``Prod.snd [.zero, .zero]) envType stateType anchor
        let owed ← Contract.memoryWriteInvariants unit namespaceId ns declaration types
          written state before executable (writtenType := some resource.typeId)
        mkLambdaFVars #[start, startState, anchor, env, state] owed
    conditions := conditions.push (site, condition, readsUnit)
  -- A spec variable's write is a write of its memory.
  for id in bodyExpressionIds ns root do
    let some { kind := .spec block, .. } := ns.expressions[id.index]? | continue
    for update in block.conditions do
      unless update.kind == .update do continue
      let some (_, target) := update.auxiliary.find? (·.1 == "updateTarget") | continue
      let some { kind := .operation (.data (.select _ _)) _ #[base] _, .. } :=
          ns.expressions[target.index]? | continue
      let baseKind : Option VerexIR.ExprKind := (ns.expressions[base.index]?).map (·.kind)
      let some (.operation (.specification (.global none)) #[.typeArg resource] _ _) := baseKind
        | continue
      let some ty := unit.tables.types[resource.typeId.index]? | continue
      let some (written, _, _) := Contract.nominalDeclaration? unit ty | continue
      let site := loopSite namespaceId.index update.expression.index
      let anchorType ← mkAppM ``AnchorOf #[Lean.toExpr site, rowType]
      let condition ← overExecutable outer readsUnit fun executable =>
        withLocalDeclD `start argumentsType fun start =>
        withLocalDeclD `startState stateType fun startState =>
        withLocalDeclD `anchor anchorType fun anchor =>
        withLocalDeclD `env envType fun env =>
        withLocalDeclD `state stateType fun state => do
          let before := mkApp3 (mkConst ``Prod.snd [.zero, .zero]) envType stateType anchor
          let owed ← Contract.memoryWriteInvariants unit namespaceId ns declaration types
            written state before executable (writtenType := some resource.typeId)
          mkLambdaFVars #[start, startState, anchor, env, state] owed
      conditions := conditions.push (site, condition, readsUnit)
  return conditions

/-- The invariants a function owes where a call of a callee that leaves its
`[suspendable]` invariants to its callers returns
(`Contract.delegatingCallees`), keyed by the call: over the function's
starting arguments and state and the memory before and after the call;
first over the executable unit where an invariant states a behavioral
predicate, which the returned flag marks. -/
def afterCallConditions (unit : ValidatedUnit) (outer : Lean.Expr) (handle : FunctionHandle)
    (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) (params : NRow)
    (types : Option Lean.Expr) :
    TermElabM (Array (Nat × Lean.Expr × Bool)) := do
  let .structured root := declaration.body | return #[]
  let callees := Contract.delegatingCallees unit handle.namespaceId declaration
  if callees.isEmpty then return #[]
  let readsUnit := unit.namespaces.toList.zipIdx.any fun (invariantNs, index) =>
    invariantNs.invariants.any fun invariant =>
      Contract.conditionReadsBehavior unit ⟨index⟩ invariant.condition.expression
  let argumentsType ← mkAppM ``HList #[← quoteRow params]
  let stateType ← memoryAt outer
  let mut conditions := #[]
  for id in bodyExpressionIds ns root do
    let some expression := ns.expressions[id.index]? | continue
    let .operation (.call (.function reference)) _ _ _ := expression.kind | continue
    let some functionId := unit.resolution.function? reference.name | continue
    let some (_, written) := callees.find? (·.1 == { namespaceId := reference.namespaceId, functionId })
      | continue
    let condition ← overExecutable outer readsUnit fun executable =>
      withLocalDeclD `start argumentsType fun start =>
      withLocalDeclD `startState stateType fun startState =>
      withLocalDeclD `before stateType fun before =>
      withLocalDeclD `after stateType fun after => do
        let owed ← Contract.callWriteInvariants unit handle.namespaceId ns declaration types
          written before after executable
        mkLambdaFVars #[start, startState, before, after] owed
    conditions := conditions.push (loopSite handle.namespaceId.index id.index, condition, readsUnit)
  return conditions

end VerexSL.Verify
