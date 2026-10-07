-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Contract
import LeanerLang.Modules
import LeanerIR.Proofs.Denote.Close

/-!
# Verification through the denotation

`verify f` compiles `f`'s validated body to its typed term
(`LeanerIR.Proofs.Denote.compileFunction`), certifies that compilation by
`rfl`, proves the authored contract over the term's denotation by one
normalization with the `lir_denote` inventory and the leaf closer, and
transports the result to the big-step meaning through the agreement
theorem.  Per target it publishes:

- `f.compiled`, the term, and `f.compiled_eq`, the kernel-checked
  certificate that the compiler produced it;
- `f.typedVerified`, the contract over the denotation;
- `f.verified`, the same contract of the function's big-step meaning.

Nothing here is proved about agreement per target, and nothing selects a
route: a construct the compiler does not carry is a reported error.
-/

namespace LeanerLang.Verify

open Lean Meta Elab Command
open LeanerIR (RuntimeValue FunctionHandle NamespaceId FunctionId)
open LeanerIR.Validation (ValidatedUnit ValidatedNamespace)
open LeanerIR.Proofs.Denote
open LeanerLang.Contract

/-- Only the verifier can mark a completed artifact family for cache reuse.
The tag is serialized with its declaration for downstream modules. -/
private initialize completedDenotations : TagDeclarationExtension ← mkTagDeclarationExtension

/-- The natives a verified function's theorems assume, keyed by its artifact
base: each native's handle and name. A caller assumes them as well. -/
private initialize nativeDependencies :
    SimplePersistentEnvExtension (Name × Array (FunctionHandle × String))
      (NameMap (Array (FunctionHandle × String))) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := fun map (name, natives) => map.insert name natives
    addImportedFn := fun entries =>
      mkStateFromImportedEntries (fun map (name, natives) => map.insert name natives) {} entries }

/-- The functions whose in-body assumptions a verified function's theorems
assume to hold (`AssumptionsHold`), keyed by its artifact base: each
function's handle and name. A caller assumes them as well. -/
private initialize assumptionDependencies :
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
private initialize lemmaTrustDependencies :
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
deriving instance ToExpr for LeanerIR.TypeId
deriving instance ToExpr for LeanerIR.LocId
deriving instance ToExpr for LeanerIR.TypeUse
deriving instance ToExpr for LeanerIR.Validation.Witness
deriving instance ToExpr for LeanerIR.Validation.KeyMap
deriving instance ToExpr for LeanerIR.Validation.IndexedArena
deriving instance ToExpr for LeanerIR.LifetimeKind
deriving instance ToExpr for LeanerIR.StructHandle

attribute [lir_denote_norm] LeanerLang.Contract.lengthVector_vector
  LeanerLang.Contract.elementsVector_vector
  LeanerLang.Contract.containsVector_vector
  LeanerLang.Contract.updateVector_vector LeanerLang.Contract.pushVector_vector
  LeanerLang.Contract.concatVector_vector LeanerLang.Contract.sliceVector_vector

/-- A variant test of an encoded enum value tests the variant it holds. -/
theorem LeanerLang.Contract.testVariants_encode_enum {unit : LeanerIR.Validation.ValidatedUnit}
    [LeanerIR.Proofs.Denote.Skolems unit]
    (source : LeanerIR.StructHandle) (arguments : LeanerIR.Proofs.Denote.NRow)
    (names : List String) (rows : LeanerIR.Proofs.Denote.NRows) (distinct : names.Nodup)
    (value : LeanerIR.Proofs.Denote.variantCarrier names rows)
    (owner : LeanerIR.StructHandle) (variants : Array String) :
    LeanerLang.Contract.testVariants
        (LeanerIR.Proofs.Denote.NTy.encode (.enum source arguments names rows distinct) value)
        owner variants =
      (source == owner && LeanerLang.Contract.variantMember
        (LeanerIR.Proofs.Denote.variantName names rows value) variants.toList) := by
  obtain ⟨fields, encoded⟩ :=
    LeanerIR.Proofs.Denote.NTy.encode_enum_nominal source arguments names rows distinct value
  rw [encoded]; rfl

/-- A variant field selected of an encoded enum value is the field of the
variant it holds. -/
theorem LeanerLang.Contract.selectVariantField_encode_enum {unit : LeanerIR.Validation.ValidatedUnit}
    [LeanerIR.Proofs.Denote.Skolems unit]
    (source : LeanerIR.StructHandle) (arguments : LeanerIR.Proofs.Denote.NRow)
    (names : List String) (rows : LeanerIR.Proofs.Denote.NRows) (distinct : names.Nodup)
    (value : LeanerIR.Proofs.Denote.variantCarrier names rows)
    (owner : LeanerIR.StructHandle) (variants : Array (String × Nat)) :
    LeanerLang.Contract.selectVariantField
        (LeanerIR.Proofs.Denote.NTy.encode (.enum source arguments names rows distinct) value)
        owner variants =
      if source == owner then
        match LeanerLang.Contract.variantIndex
            (LeanerIR.Proofs.Denote.variantName names rows value) variants.toList with
        | some index =>
            (LeanerIR.Proofs.Denote.variantPayload names rows value)[index]?.getD .unit
        | none => .unit
      else .unit := by
  rw [LeanerIR.Proofs.Denote.NTy.encode_enum_payload]; rfl

attribute [lir_denote_norm] LeanerIR.moveArithmeticError LeanerLang.Contract.abortCodeMatches
attribute [lir_denote_norm] LeanerIR.Proofs.Denote.DataInvariant.withCollections
  LeanerIR.Proofs.Denote.DataInvariant.collection LeanerIR.Proofs.Denote.DataInvariant.elements
  LeanerIR.Proofs.Denote.DataInvariant.value LeanerIR.Proofs.Denote.DataInvariant.row
  LeanerIR.Proofs.Denote.DataInvariant.variant
attribute [lir_denote_norm] LeanerLang.Contract.updateNominalField
  LeanerLang.Contract.updateFieldIndex
  LeanerLang.Contract.testVariants_encode_enum
  LeanerLang.Contract.selectVariantField_encode_enum
  LeanerLang.Contract.testVariants_nominal_self
  LeanerLang.Contract.testVariants_nominal LeanerLang.Contract.variantMember
  LeanerLang.Contract.selectVariantField_nominal
  LeanerLang.Contract.selectVariantField_nominal_none LeanerLang.Contract.variantIndex
  LeanerIR.SemanticOperations.resolveReturnedBorrows_empty
  LeanerIR.SemanticOperations.resolveReturnedBorrows_integer

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
      mkAppM ``LeanerIR.SpecVector.mk #[values, bounded]
  | .ref referent, value => do
      mkAppM ``Prod.mk #[← quoteCarrier referent value.1, ← quoteCarrier referent value.2]
  | .int width signed, value => do
      let widthExpr := toExpr (LeanerIR.IntWidth.bits width)
      let proposition ← mkAppM ``LeanerIR.IntegerValueFits
        #[widthExpr, toExpr signed, toExpr value.val]
      let fits ← mkDecideProof proposition
      return mkAppN (mkConst ``LeanerIR.SpecInt.mk)
        #[widthExpr, toExpr signed, toExpr value.val, fits]
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

def quoteVar : {Γ : NRow} → {τ : NTy} → Var Γ τ → MetaM Lean.Expr
  | .cons τ Γ, _, .here => return mkAppN (mkConst ``Var.here) #[← quoteRow Γ, ← quoteNTy τ]
  | .cons σ Γ, τ, .there rest =>
      return mkAppN (mkConst ``Var.there)
        #[← quoteRow Γ, ← quoteNTy σ, ← quoteNTy τ, ← quoteVar rest]

def quotePlaceIndex : PlaceIndex → Lean.Expr
  | .literal index => mkApp (mkConst ``PlaceIndex.literal) (toExpr index)
  | .slot slot => mkApp (mkConst ``PlaceIndex.slot) (toExpr slot)
  | .fromEnd offset => mkApp (mkConst ``PlaceIndex.fromEnd) (toExpr offset)

def quoteWhich : {names : List String} → {rows : NRows} → {σs : NRow} → Which names rows σs →
    MetaM Lean.Expr
  | name :: names, .cons fields rest, _, .here =>
      return mkAppN (mkConst ``Which.here)
        #[toExpr name, toExpr names, ← quoteRow fields, ← quoteRows rest]
  | name :: names, .cons fields rest, σs, .there later =>
      return mkAppN (mkConst ``Which.there) #[toExpr name, toExpr names, ← quoteRow fields,
        ← quoteRow σs, ← quoteRows rest, ← quoteWhich later]

def quoteChoices {names : List String} {rows : NRows} {τ : NTy} :
    Choices names rows τ → MetaM Lean.Expr
  | .nil =>
      return mkAppN (mkConst ``Choices.nil) #[toExpr names, ← quoteRows rows, ← quoteNTy τ]
  | @Choices.cons _ _ _ σs choice x rest =>
      return mkAppN (mkConst ``Choices.cons) #[toExpr names, ← quoteRows rows, ← quoteNTy τ,
        ← quoteRow σs, ← quoteWhich choice, ← quoteVar x, ← quoteChoices rest]

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
  | .enum source arguments names rows _, τ, @Proj.variant _ _ _ _ _ σ _ choices rest => do
      let namesExpr := toExpr names
      let distinct ← mkDecideProof (← mkAppM ``List.Nodup #[namesExpr])
      return mkAppN (mkConst ``Proj.variant) #[Lean.toExpr source, ← quoteRow arguments,
        namesExpr, ← quoteRows rows, distinct, ← quoteNTy σ, ← quoteNTy τ, ← quoteChoices choices,
        ← quoteProj rest]

/-- The literal of type arguments, with their inhabitability decided. -/
def quoteTypeArgs (θ : TypeArgs) : MetaM Lean.Expr := do
  let row ← quoteRow θ.1
  let inhabitable (row : Lean.Expr) :=
    mkApp3 (mkConst ``Eq [1]) (mkConst ``Bool) (mkApp (mkConst ``NRow.inhabitable) row)
      (mkConst ``Bool.true)
  let predicate := Lean.mkLambda `row .default (mkConst ``NRow) (inhabitable (.bvar 0))
  return mkAppN (mkConst ``Subtype.mk [1])
    #[mkConst ``NRow, predicate, row, ← mkDecideProof (inhabitable row)]

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

/-- A proof that a closure target's supplied parameters are shared where the
given flags say, by the kernel's evaluation. -/
def closureSharingProof (unitExpr : Lean.Expr) (handle : FunctionHandle)
    (fullExpr capturedExpr suppliedExpr weaveExpr : Lean.Expr) (shared : List Bool) :
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
  | .int width signed, .checked op failure left right =>
      return mkAppN (mkConst ``Term.checked) #[unitExpr, shape, context, Lean.toExpr width,
        Lean.toExpr signed, Lean.toExpr op, Lean.toExpr failure, ← quoteTerm unitExpr left, ← quoteTerm unitExpr right]
  | .int width signed, .modular op left right =>
      return mkAppN (mkConst ``Term.modular) #[unitExpr, shape, context, Lean.toExpr width,
        Lean.toExpr signed, Lean.toExpr op, ← quoteTerm unitExpr left, ← quoteTerm unitExpr right]
  | .bool, @Term.compare _ _ _ width signed op left right =>
      return mkAppN (mkConst ``Term.compare) #[unitExpr, shape, context, Lean.toExpr width,
        Lean.toExpr signed, Lean.toExpr op, ← quoteTerm unitExpr left, ← quoteTerm unitExpr right]
  | .bool, @Term.equal _ _ _ σ negated left right =>
      return mkAppN (mkConst ``Term.equal) #[unitExpr, shape, context, ← toExpr σ, Lean.toExpr negated,
        ← quoteTerm unitExpr left, ← quoteTerm unitExpr right]
  | .bool, .not operand =>
      return mkAppN (mkConst ``Term.not) #[unitExpr, shape, context, ← quoteTerm unitExpr operand]
  | .bool, .logical conjunction left right =>
      return mkAppN (mkConst ``Term.logical) #[unitExpr, shape, context, Lean.toExpr conjunction,
        ← quoteTerm unitExpr left, ← quoteTerm unitExpr right]
  | .int width false, .bitwise op left right =>
      return mkAppN (mkConst ``Term.bitwise) #[unitExpr, shape, context, Lean.toExpr width, Lean.toExpr op,
        ← quoteTerm unitExpr left, ← quoteTerm unitExpr right]
  | .int width false, @Term.shift _ _ _ _ distanceWidth left failure value distance =>
      return mkAppN (mkConst ``Term.shift) #[unitExpr, shape, context, Lean.toExpr width,
        Lean.toExpr distanceWidth, Lean.toExpr left, Lean.toExpr failure, ← quoteTerm unitExpr value,
        ← quoteTerm unitExpr distance]
  | .int width' signed', @Term.cast _ _ _ width signed _ _ failure value =>
      return mkAppN (mkConst ``Term.cast) #[unitExpr, shape, context, Lean.toExpr width, Lean.toExpr signed,
        Lean.toExpr width', Lean.toExpr signed', Lean.toExpr failure, ← quoteTerm unitExpr value]
  | τ, .ite condition thenBranch elseBranch =>
      return mkAppN (mkConst ``Term.ite) #[unitExpr, shape, context, ← toExpr τ, ← quoteTerm unitExpr condition,
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
  | τ, .throw0 kind =>
      return mkAppN (mkConst ``Term.throw0) #[unitExpr, shape, context, ← toExpr τ, Lean.toExpr kind]
  | τ, @Term.throw1 _ _ _ σ _ kind code =>
      return mkAppN (mkConst ``Term.throw1) #[unitExpr, shape, context, ← toExpr σ, ← toExpr τ,
        Lean.toExpr kind, ← quoteTerm unitExpr code]
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
        (Lean.toExpr handle) (Lean.toExpr (#[] : Array (LeanerIR.TypeId × LeanerIR.TypeId))))
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
  | _, @Term.invoke _ _ _ σs shared invokeShape function arguments =>
      return mkAppN (mkConst ``Term.invoke) #[unitExpr, shape, context, ← quoteRow σs,
        Lean.toExpr shared, ← quoteShape invokeShape, ← quoteTerm unitExpr function,
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
  | τ, @Term.index _ _ _ _ width signed vector position =>
      return mkAppN (mkConst ``Term.index) #[unitExpr, shape, context, ← toExpr τ, Lean.toExpr width,
        Lean.toExpr signed, ← quoteTerm unitExpr vector, ← quoteTerm unitExpr position]
  | .unit, @Term.checkIndex _ _ _ τ width signed failure vector position =>
      return mkAppN (mkConst ``Term.checkIndex) #[unitExpr, shape, context, ← toExpr τ, Lean.toExpr width,
        Lean.toExpr signed, Lean.toExpr failure, ← quoteTerm unitExpr vector, ← quoteTerm unitExpr position]
  | .vector τ, .push vector element =>
      return mkAppN (mkConst ``Term.push) #[unitExpr, shape, context, ← toExpr τ, ← quoteTerm unitExpr vector,
        ← quoteTerm unitExpr element]
  | .vector τ, @Term.insert _ _ _ _ width signed vector position element =>
      return mkAppN (mkConst ``Term.insert) #[unitExpr, shape, context, ← toExpr τ, Lean.toExpr width,
        Lean.toExpr signed, ← quoteTerm unitExpr vector, ← quoteTerm unitExpr position, ← quoteTerm unitExpr element]
  | .tuple (.cons τ (.cons (.vector _) .nil)), @Term.remove _ _ _ _ width signed vector position =>
      return mkAppN (mkConst ``Term.remove) #[unitExpr, shape, context, ← toExpr τ, Lean.toExpr width,
        Lean.toExpr signed, ← quoteTerm unitExpr vector, ← quoteTerm unitExpr position]
  | .vector τ, @Term.swap _ _ _ _ width signed vector left right =>
      return mkAppN (mkConst ``Term.swap) #[unitExpr, shape, context, ← toExpr τ, Lean.toExpr width,
        Lean.toExpr signed, ← quoteTerm unitExpr vector, ← quoteTerm unitExpr left, ← quoteTerm unitExpr right]
  | .vector τ, .concat left right =>
      return mkAppN (mkConst ``Term.concat) #[unitExpr, shape, context, ← toExpr τ, ← quoteTerm unitExpr left,
        ← quoteTerm unitExpr right]
  | .vector τ, @Term.slice _ _ _ _ width signed vector start stop =>
      return mkAppN (mkConst ``Term.slice) #[unitExpr, shape, context, ← toExpr τ, Lean.toExpr width,
        Lean.toExpr signed, ← quoteTerm unitExpr vector, ← quoteTerm unitExpr start, ← quoteTerm unitExpr stop]
  | .vector τ, @Term.reverseSlice _ _ _ _ width signed vector start stop =>
      return mkAppN (mkConst ``Term.reverseSlice) #[unitExpr, shape, context, ← toExpr τ, Lean.toExpr width,
        Lean.toExpr signed, ← quoteTerm unitExpr vector, ← quoteTerm unitExpr start, ← quoteTerm unitExpr stop]
  | .unit, @Term.destroyEmpty _ _ _ τ vector =>
      return mkAppN (mkConst ``Term.destroyEmpty) #[unitExpr, shape, context, ← toExpr τ, ← quoteTerm unitExpr vector]
  | .bool, @Term.contains _ _ _ τ vector needle =>
      return mkAppN (mkConst ``Term.contains) #[unitExpr, shape, context, ← toExpr τ, ← quoteTerm unitExpr vector,
        ← quoteTerm unitExpr needle]
  | .tuple (.cons .bool (.cons (.int 64 false) .nil)), @Term.indexOf _ _ _ τ vector needle =>
      return mkAppN (mkConst ``Term.indexOf) #[unitExpr, shape, context, ← toExpr τ, ← quoteTerm unitExpr vector,
        ← quoteTerm unitExpr needle]
  | .int 8 true, @Term.order _ _ _ τ orders left right =>
      return mkAppN (mkConst ``Term.order) #[unitExpr, shape, context, ← toExpr τ, Lean.toExpr orders,
        ← quoteTerm unitExpr left, ← quoteTerm unitExpr right]
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
  | τ, @Term.globalRead _ _ _ κ _ arguments key =>
      return mkAppN (mkConst ``Term.globalRead) #[unitExpr, shape, context, ← toExpr κ, ← toExpr τ,
        ← quoteRow arguments, ← quoteTerm unitExpr key]
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
  | .cons pattern body rest =>
      return mkAppN (mkConst ``Arms.cons)
        (prefix_ ++ #[← quotePat pattern, ← quoteTerm unitExpr body, ← quoteArms unitExpr rest])
  | .guarded pattern guard body rest =>
      return mkAppN (mkConst ``Arms.guarded) (prefix_ ++
        #[← quotePat pattern, ← quoteTerm unitExpr guard, ← quoteTerm unitExpr body, ← quoteArms unitExpr rest])
end

mutual
/-- Fold over the calls a term makes, in evaluation order: each callee
with the type arguments of the call. -/
partial def _root_.LeanerIR.Proofs.Denote.Term.foldCalls {α : Type}
    (visit : FunctionHandle → Array LeanerIR.TypeUse → α → α) {unit : ValidatedUnit}
    {ρ : ResultShape} :
    {Γ : NRow} → {τ : NTy} → Term unit ρ Γ τ → α → α
  | _, _, .checked _ _ left right, found => right.foldCalls visit (left.foldCalls visit found)
  | _, _, .modular _ left right, found => right.foldCalls visit (left.foldCalls visit found)
  | _, _, .compare _ left right, found => right.foldCalls visit (left.foldCalls visit found)
  | _, _, .equal _ left right, found => right.foldCalls visit (left.foldCalls visit found)
  | _, _, .not operand, found => operand.foldCalls visit found
  | _, _, .logical _ left right, found => right.foldCalls visit (left.foldCalls visit found)
  | _, _, .bitwise _ left right, found => right.foldCalls visit (left.foldCalls visit found)
  | _, _, .shift _ _ value distance, found => distance.foldCalls visit (value.foldCalls visit found)
  | _, _, .cast _ value, found => value.foldCalls visit found
  | _, _, .ite condition thenBranch elseBranch, found =>
      elseBranch.foldCalls visit (thenBranch.foldCalls visit (condition.foldCalls visit found))
  | _, _, .let_ _ value body, found => body.foldCalls visit (value.foldCalls visit found)
  | _, _, .drop value body, found => body.foldCalls visit (value.foldCalls visit found)
  | _, _, .assign _ value, found => value.foldCalls visit found
  | _, _, .const value, found => value.foldCalls visit found
  | _, _, .throw1 _ code, found => code.foldCalls visit found
  | _, _, .return_ value, found => value.foldCalls visit found
  | _, _, .loop _ body, found => body.foldCalls visit found
  | _, _, .assume _, found => found
  | _, _, .call _ handle _ arguments, found => arguments.foldCalls visit (visit handle #[] found)
  | _, _, .callGeneric _ handle typeArgs _ _ arguments, found =>
      arguments.foldCalls visit (visit handle typeArgs found)
  -- An invocation of a closure runs its target, whose theorem a proof uses.
  | _, _, .closure handle _ _ _ _ _ _ _ captures, found =>
      captures.foldCalls visit (visit handle #[] found)
  | _, _, .closureGeneric handle _ typeArgs _ _ _ _ _ _ captures, found =>
      captures.foldCalls visit (visit handle typeArgs found)
  | _, _, .invoke _ function arguments, found =>
      arguments.foldCalls visit (function.foldCalls visit found)
  | _, _, .tuple elements, found => elements.foldCalls visit found
  | _, _, .pack _ _ fields, found => fields.foldCalls visit found
  | _, _, .variant _ _ _ _ fields, found => fields.foldCalls visit found
  | _, _, .field _ value, found => value.foldCalls visit found
  | _, _, .isVariant _ value, found => value.foldCalls visit found
  | _, _, .payload _ _ value, found => value.foldCalls visit found
  | _, _, .letRow _ value body, found => body.foldCalls visit (value.foldCalls visit found)
  | _, _, .letFields _ value body, found => body.foldCalls visit (value.foldCalls visit found)
  | _, _, .caseOf scrutinee arms, found => arms.foldCalls visit (scrutinee.foldCalls visit found)
  | _, _, .deref value, found => value.foldCalls visit found
  | _, _, .mutate _ value, found => value.foldCalls visit found
  | _, _, .writePlace _ _ _ value, found => value.foldCalls visit found
  | _, _, .vectorLit _ elements, found => elements.foldCalls visit found
  | _, _, .length vector, found => vector.foldCalls visit found
  | _, _, .signerAddress signer, found => signer.foldCalls visit found
  | _, _, .index vector position, found => position.foldCalls visit (vector.foldCalls visit found)
  | _, _, .checkIndex _ vector position, found => position.foldCalls visit (vector.foldCalls visit found)
  | _, _, .push vector element, found => element.foldCalls visit (vector.foldCalls visit found)
  | _, _, .insert vector position element, found =>
      element.foldCalls visit (position.foldCalls visit (vector.foldCalls visit found))
  | _, _, .remove vector position, found => position.foldCalls visit (vector.foldCalls visit found)
  | _, _, .swap vector left right, found => right.foldCalls visit (left.foldCalls visit (vector.foldCalls visit found))
  | _, _, .concat left right, found => right.foldCalls visit (left.foldCalls visit found)
  | _, _, .slice vector start stop, found => stop.foldCalls visit (start.foldCalls visit (vector.foldCalls visit found))
  | _, _, .reverseSlice vector start stop, found =>
      stop.foldCalls visit (start.foldCalls visit (vector.foldCalls visit found))
  | _, _, .destroyEmpty vector, found => vector.foldCalls visit found
  | _, _, .contains vector needle, found => needle.foldCalls visit (vector.foldCalls visit found)
  | _, _, .indexOf vector needle, found => needle.foldCalls visit (vector.foldCalls visit found)
  | _, _, .order _ left right, found => right.foldCalls visit (left.foldCalls visit found)
  | _, _, .seqAfter value effect, found => effect.foldCalls visit (value.foldCalls visit found)
  | _, _, .assertion _, found => found
  | _, _, .anchor _, found => found
  | _, _, .mutationEnd _, found => found
  | _, _, .memoryWritten _, found => found
  | _, _, .constructed _ value, found => value.foldCalls visit found
  | _, _, .globalRead _ key, found => key.foldCalls visit found
  | _, _, .globalContains _ _ key, found => key.foldCalls visit found
  | _, _, .globalBorrow _ _ key, found => key.foldCalls visit found
  | _, _, .globalPublish _ _ key value, found => value.foldCalls visit (key.foldCalls visit found)
  | _, _, .globalTake _ _ key, found => key.foldCalls visit found
  | _, _, _, found => found

partial def _root_.LeanerIR.Proofs.Denote.Args.foldCalls {α : Type}
    (visit : FunctionHandle → Array LeanerIR.TypeUse → α → α) {unit : ValidatedUnit}
    {ρ : ResultShape} :
    {Γ σs : NRow} → Args unit ρ Γ σs → α → α
  | _, _, .nil, found => found
  | _, _, .cons head tail, found => tail.foldCalls visit (head.foldCalls visit found)

partial def _root_.LeanerIR.Proofs.Denote.Arms.foldCalls {α : Type}
    (visit : FunctionHandle → Array LeanerIR.TypeUse → α → α) {unit : ValidatedUnit}
    {ρ : ResultShape} :
    {Γ : NRow} → {σ τ : NTy} → Arms unit ρ Γ σ τ → α → α
  | _, _, _, .nil _, found => found
  | _, _, _, .cons _ body rest, found => rest.foldCalls visit (body.foldCalls visit found)
  | _, _, _, .guarded _ guard body rest, found =>
      rest.foldCalls visit (body.foldCalls visit (guard.foldCalls visit found))
end

/-- The callees a term calls, in order of first occurrence. -/
def _root_.LeanerIR.Proofs.Denote.Term.callees {unit : ValidatedUnit} {ρ : ResultShape} {Γ : NRow}
    {τ : NTy}
    (term : Term unit ρ Γ τ) (found : Array FunctionHandle) : Array FunctionHandle :=
  term.foldCalls (fun handle _ found => if found.contains handle then found else found.push handle)
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

private def pathName (segments : Array String) : Name :=
  segments.foldl (fun name segment => Name.str name segment) .anonymous

private def rootIdent (name : Name) : Ident := mkIdent (rootNamespace ++ name)

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
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) :
    Option (NRow × ResultShape) := do
  let params ← declaration.signature.parameters.toList.mapM fun parameter =>
    ntyOf unit namespaceId parameter.typeUse.typeId
  let result ← match declaration.signature.results.toList with
    | [] => some ResultShape.none
    | [result] => (ntyOf unit namespaceId result.typeId).map ResultShape.one
    | _ => none
  some (NRow.ofList params, result)

private def addAbbrev (name : Name) (value : Lean.Expr) : TermElabM Unit := do
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
private def withExecutableSkolems {n : Type → Type} [MonadControlT MetaM n] [Monad n] {α : Type}
    (k : Lean.Expr → Lean.Expr → n α) : n α :=
  withLocalDecl `unit .implicit (mkConst ``ValidatedUnit) fun unit =>
    withLocalDeclD `executable (Contract.executableType unit) fun executable =>
      withLocalDecl `Θ .instImplicit (mkApp (mkConst ``Skolems) unit) (k executable)

/-- The binders of a frame: its unit and itself. -/
private def frameBinders (skolems : Lean.Expr) : MetaM (Array Lean.Expr) := do
  return #[← Contract.frameUnit skolems, skolems]

/-- The binders of an artifact over an executable unit and a frame: the unit,
the executable, and the frame. -/
private def executableBinders (executable skolems : Lean.Expr) : MetaM (Array Lean.Expr) := do
  return #[← Contract.frameUnit skolems, executable, skolems]

/-- The native row a twin's fields form, as projections of a twin value,
built at the row type so that the row lemmas match it; a nested twin
field is its own row. -/
private partial def twinFields (twins : Array SpecTypes.TwinInfo) (info : SpecTypes.TwinInfo)
    (row : NRow) (value : Lean.Expr) : MetaM Lean.Expr := do
  let mut components : Array (NTy × Lean.Expr) := #[]
  let mut rest := row
  for (name, rep) in info.fields do
    let .cons τ tail := rest | throwError "internal: a twin's row is shorter than its fields"
    rest := tail
    let projection ← mkAppM (info.twin ++ Name.mkSimple name) #[value]
    let component ← match rep, τ with
      | .nominal twin _, .struct _ _ innerRow =>
          match twins.find? (·.twin == twin) with
          | some inner => twinFields twins inner innerRow projection
          | none => pure projection
      | .nominal twin _, .enum .. => mkAppM (twin ++ `native) #[projection]
      -- A vector of nested twins is viewed element by element.
      | .vector (.nominal twin _) bounded, .vector (.struct _ _ innerRow) =>
          match twins.find? (·.twin == twin) with
          | some inner => do
              -- `SpecVector α` or `Array α`: the element type is the argument.
              let elementType := (← whnfR (← inferType projection)).appArg!
              let element ← withLocalDeclD `element elementType fun element => do
                mkLambdaFVars #[element] (← twinFields twins inner innerRow element)
              mkAppM (if bounded then ``LeanerIR.SpecVector.map else ``Array.map)
                #[element, projection]
          | none => pure projection
      | .vector (.nominal twin _) bounded, .vector (.enum ..) =>
          -- A non-generic enum twin's elements, through its native view.
          match twins.find? (·.twin == twin) with
          | some inner =>
              if inner.typeParameterCount != 0 then pure projection else do
              let elementType := (← whnfR (← inferType projection)).appArg!
              let element ← withLocalDeclD `element elementType fun element => do
                mkLambdaFVars #[element] (← mkAppM (twin ++ `native) #[element])
              mkAppM (if bounded then ``LeanerIR.SpecVector.map else ``Array.map)
                #[element, projection]
          | none => pure projection
      | _, _ => pure projection
    components := components.push (τ, component)
  let mut tuple := mkConst ``Unit.unit
  let mut tailRow : NRow := .nil
  for (τ, component) in components.reverse do
    tuple ← mkAppOptM ``Prod.mk #[some (← mkAppM ``NTy.carrier #[← quoteNTy τ]),
      some (← mkAppM ``HList #[← quoteRow tailRow]), some component, some tuple]
    tailRow := .cons τ tailRow
  return tuple

/-- One literal shape a twin's erasure can take: binderList for its scalar
fields, the runtime literal they erase to, the twin value they build, the
range checkList its integers need, and the decoderNames the proof unfolds. A
struct has one shape per combination of the variants its enum-typed
fields may hold. -/
private structure LiteralShape where
  binderList : Array (Ident × Term) := #[]
  eraseTerms : Array Term := #[]
  componentTerms : Array Term := #[]
  checkList : Array (Ident × Term) := #[]
  decoderNames : Array Ident := #[]

private def LiteralShape.append (left right : LiteralShape) : LiteralShape :=
  { binderList := left.binderList ++ right.binderList, eraseTerms := left.eraseTerms ++ right.eraseTerms,
    componentTerms := left.componentTerms ++ right.componentTerms, checkList := left.checkList ++ right.checkList,
    decoderNames := left.decoderNames ++ right.decoderNames }

mutual
/-- The literal shapes of a field list, as the product of its fields'
shapes. `none` when a field has a representation the bridge does not
carry. -/
private partial def fieldShapes (twins : Array SpecTypes.TwinInfo)
    (fields : Array (String × SpecTypes.FieldRep)) (base : String) :
    CommandElabM (Option (Array LiteralShape)) := do
  let mut shapes : Array LiteralShape := #[{}]
  for (name, rep) in fields do
    let binder := mkIdent (Name.mkSimple s!"{base}_{name}")
    let fieldShapes : Array LiteralShape ← match rep with
      | .int (.bits width) signed =>
          let fits := mkIdent (Name.mkSimple s!"{base}_{name}_fits")
          let intType ← `(term| Int)
          let erasure ← `(term| LeanerIR.RuntimeValue.integer $binder)
          let component ← `(term| ⟨$binder, $fits⟩)
          let check ← `(term| LeanerIR.IntegerValueFits
            (LeanerIR.IntWidth.bits $(Syntax.mkNumLit (toString width))) $(quote signed) $binder)
          let binderList := #[(binder, intType)]
          let eraseTerms := #[erasure]
          let componentTerms := #[component]
          let checkList := #[(fits, check)]
          pure #[{ binderList, eraseTerms, componentTerms, checkList }]
      | .bool =>
          let boolType ← `(term| Bool)
          let erasure ← `(term| LeanerIR.RuntimeValue.bool $binder)
          let binderList := #[(binder, boolType)]
          let eraseTerms := #[erasure]
          let componentTerms := #[binder]
          pure #[{ binderList, eraseTerms, componentTerms }]
      | .address =>
          let stringType ← `(term| String)
          let erasure ← `(term| LeanerIR.RuntimeValue.address $binder)
          let binderList := #[(binder, stringType)]
          let eraseTerms := #[erasure]
          let componentTerms := #[binder]
          pure #[{ binderList, eraseTerms, componentTerms }]
      | .nominal twin arguments =>
          let some inner := twins.find? (·.twin == twin) | return none
          let some innerShapes ← twinShapes twins inner s!"{base}_{name}" arguments
            | return none
          pure innerShapes
      | _ => return none
    shapes := shapes.flatMap fun shape => fieldShapes.map shape.append
  return some shapes

/-- The literal shapes of a twin value at the representations of its type
`arguments`: one per struct, one per variant of an enum, each carrying the
literal and the twin value as one component. -/
private partial def twinShapes (twins : Array SpecTypes.TwinInfo) (info : SpecTypes.TwinInfo)
    (base : String) (arguments : Array SpecTypes.FieldRep := #[]) :
    CommandElabM (Option (Array LiteralShape)) := do
  let handle := (Syntax.mkNumLit (toString info.namespaceIndex),
    Syntax.mkNumLit (toString info.structIndex))
  let decoder := rootIdent (info.twin ++ `decode?)
  let instantiate := fun (fields : Array (String × SpecTypes.FieldRep)) =>
    fields.map fun (name, rep) => (name, rep.instantiate arguments)
  let twinType ← SpecTypes.FieldRep.typeSyntax #[] (.nominal info.twin arguments)
  if info.variants.isEmpty then
    let some shapes ← fieldShapes twins (instantiate info.fields) base | return none
    shapes.mapM fun shape => do
      let literal ← `(term| LeanerIR.RuntimeValue.nominal ⟨⟨$(handle.1)⟩, $(handle.2)⟩ none
        #[$(shape.eraseTerms),*])
      let component ← `(term| (⟨$(shape.componentTerms),*⟩ : $twinType))
      let decoderNames := #[decoder] ++ shape.decoderNames
      let eraseTerms := #[literal]
      let componentTerms := #[component]
      pure { shape with eraseTerms, componentTerms, decoderNames }
  else
    let mut shapes : Array LiteralShape := #[]
    for variant in info.variants do
      let some variantShapes ←
          fieldShapes twins (instantiate variant.fields) s!"{base}_{variant.name}"
        | return none
      let constructor := rootIdent (info.twin ++ Name.mkSimple variant.name)
      for shape in variantShapes do
        let literal ← `(term| LeanerIR.RuntimeValue.nominal ⟨⟨$(handle.1)⟩, $(handle.2)⟩
          (some $(quote variant.name)) #[$(shape.eraseTerms),*])
        let component ← if shape.componentTerms.isEmpty then pure (constructor : Term)
          else `(term| $constructor:ident $(shape.componentTerms)*)
        let decoderNames := #[decoder] ++ shape.decoderNames
        let eraseTerms := #[literal]
        let componentTerms := #[component]
        shapes := shapes.push { shape with eraseTerms, componentTerms, decoderNames }
    return some shapes
end

/-- The decode bridge of one struct twin: decoding each literal its erasure
can unfold to is the twin of the fields, under the integer range checkList. -/
private def ensureDecodeBridge (twins : Array SpecTypes.TwinInfo) (info : SpecTypes.TwinInfo) :
    CommandElabM Unit := do
  let name := info.twin ++ `decode?_fields
  if (← getEnv).contains name then return
  let some shapes ← twinShapes twins info "field" | return
  for shape in shapes, index in [0:shapes.size] do
    let name := if index == 0 then name else name.appendIndexAfter index
    let some literal := shape.eraseTerms[0]? | continue
    let some value := shape.componentTerms[0]? | continue
    let rhs ← shape.checkList.foldrM (init := ← `(term| some $value)) fun (fits, check) rest =>
      `(term| if $fits:ident : $check then $rest else none)
    let binderSyntax ← shape.binderList.mapM fun (binder, type) =>
      `(bracketedBinder| ($binder:ident : $type))
    let lemmas ← shape.decoderNames.mapM fun decoder => `(Lean.Parser.Tactic.simpLemma| $decoder:ident)
    elabCommand (← `(set_option Elab.async false in
      @[lir_denote_norm] theorem $(rootIdent name):ident $binderSyntax* :
        $(rootIdent (info.twin ++ `decode?)) $literal = $rhs := by
      simp only [$lemmas,*, LeanerIR.decodeInt?, LeanerIR.Proofs.Codec.specInt]
      repeat' (first | rfl | (split <;> simp_all))))

/-- The twins a field representation holds, through vectors. -/
private partial def heldTwins (twins : Array SpecTypes.TwinInfo) :
    SpecTypes.FieldRep → Array SpecTypes.TwinInfo
  | .nominal twin _ => (twins.find? (·.twin == twin)).toArray
  | .vector element _ => heldTwins twins element
  | _ => #[]

/-- The syntax of a twin field's native carrier from a bound variable of
the field's twin type: a nested struct expands to its row, a nested enum
takes its native view, and a vector maps its elements. -/
private partial def nativeFieldSyntax (twins : Array SpecTypes.TwinInfo)
    (rep : SpecTypes.FieldRep) (variable_ : Term) : TermElabM Term := do
  match rep with
  | .nominal twin _ =>
      match twins.find? (·.twin == twin) with
      | some inner =>
          if inner.variants.isEmpty then
            let mut tuple ← `(term| ())
            for (name, fieldRep) in inner.fields.reverse do
              let projection ← `(term| $(mkIdent (inner.twin ++ Name.mkSimple name)) $variable_)
              let component ← nativeFieldSyntax twins fieldRep projection
              tuple ← `(term| ($component, $tuple))
            pure tuple
          else `(term| $(mkIdent (inner.twin ++ `native)) $variable_)
      | none => pure variable_
  | .vector element bounded =>
      if (heldTwins twins element).isEmpty then pure variable_ else
      let elementVariable := mkIdent `element
      let converted ← nativeFieldSyntax twins element elementVariable
      if bounded then
        `(term| LeanerIR.SpecVector.map (fun $elementVariable:ident => $converted) $variable_)
      else `(term| Array.map (fun $elementVariable:ident => $converted) $variable_)
  | _ => pure variable_

/-- The native view of an enum twin, `Twin.native : Twin → variantCarrier`,
and the lemma that its erasure is the native encoding of that view. -/
private partial def ensureEnumNative (unit : ValidatedUnit) (twins : Array SpecTypes.TwinInfo)
    (info : SpecTypes.TwinInfo) : TermElabM Unit := do
  let nativeName := info.twin ++ `native
  if (← getEnv).contains nativeName then return
  -- A variant's enum-typed field bridges through the inner twin's view.
  let innerEnums := info.variants.flatMap fun variant => variant.fields.flatMap fun (_, rep) =>
    (heldTwins twins rep).filter fun inner =>
      !inner.variants.isEmpty && inner.typeParameterCount == 0
  for inner in innerEnums do
    ensureEnumNative unit twins inner
  let handle : LeanerIR.StructHandle := ⟨⟨info.namespaceIndex⟩, info.structIndex⟩
  let some τ := structNTy unit handle | return
  let τE ← quoteNTy τ
  let twinType := mkConst info.twin
  let mut arms : Array (TSyntax ``Lean.Parser.Term.matchAlt) := #[]
  for variant in info.variants, index in [0:info.variants.size] do
    let constructor := mkIdent (info.twin ++ Name.mkSimple variant.name)
    let variables := variant.fields.mapIdx fun i _ => mkIdent (Name.mkSimple s!"field{i}")
    let pattern ← if variables.isEmpty then pure (constructor : Term)
      else `(term| $constructor:ident $variables*)
    let mut tuple ← `(term| ())
    for (_, rep) in variant.fields.reverse, variable_ in variables.reverse do
      let component ← nativeFieldSyntax twins rep variable_
      tuple ← `(term| ($component, $tuple))
    let mut injected ← `(term| Sum.inl $tuple)
    for _ in [0:index] do
      injected ← `(term| Sum.inr $injected)
    arms := arms.push (← `(Lean.Parser.Term.matchAltExpr| | $pattern:term => $injected))
  let value := mkIdent `value
  let body ← `(term| fun ($value:ident : $(mkIdent info.twin)) =>
    match $value:ident with $arms:matchAlt*)
  let definition ← withSkolems fun skolems => do
    let carrier ← mkAppM ``NTy.carrier #[τE]
    let definition ← Lean.Elab.Term.elabTermEnsuringType body (some (← mkArrow twinType carrier))
    Lean.Elab.Term.synthesizeSyntheticMVarsNoPostponing
    mkLambdaFVars (← frameBinders skolems) (← instantiateMVars definition)
  addAbbrev nativeName definition
  let normAttr ← `(attr| lir_denote_norm)
  Lean.Elab.Term.applyAttributes nativeName #[{ name := `lir_denote_norm, stx := normAttr, kind := .global }]
  let eraseName := info.twin ++ `erase_eq_encode
  let statement ← withSkolems fun skolems => withLocalDeclD `value twinType fun value => do
    let lhs ← mkAppM (info.twin ++ `erase) #[value]
    let rhs ← mkAppM ``NTy.encode #[τE, mkApp3 (mkConst nativeName) (← Contract.frameUnit skolems) skolems value]
    mkForallFVars ((← frameBinders skolems).push value) (← mkEq lhs rhs)
  let lemmaNames := info.variants.map (fun variant =>
      info.twin ++ Name.mkSimple s!"erase_{variant.name}") ++
    innerEnums.map (·.twin ++ `erase_eq_encode) ++
    innerEnums.map (·.twin ++ `codec_encode_eq) ++
    #[``NTy.encode_enum_inl, ``NTy.encode_enum_inr, ``HList.encode_cons, ``HList.encode_nil,
      ``LeanerIR.Proofs.Codec.boundedVector_encode, ``NTy.encode_vector,
      ``LeanerIR.SpecVector.map_values, ``Array.map_map]
  let lemmas ← lemmaNames.mapM fun name => `(Lean.Parser.Tactic.simpLemma| $(mkIdent name):ident)
  let proofSyntax ← `(term| fun {unit : ValidatedUnit} [Skolems unit] ($value:ident : $(mkIdent info.twin)) => by
    cases $value:ident <;> (try simp only [$lemmas,*]) <;> rfl)
  let proof ← Lean.Elab.Term.elabTermEnsuringType proofSyntax (some statement)
  Lean.Elab.Term.synthesizeSyntheticMVarsNoPostponing
  let proof ← instantiateMVars proof
  addDecl (.thmDecl { name := eraseName, levelParams := [], type := statement, value := proof })
  -- The same bridge between the codecs, for the element maps of vectors.
  let codecName := info.twin ++ `codec_encode_eq
  let (statement, proof) ← withSkolems fun skolems => do
    let lhs ← mkAppM ``LeanerIR.Proofs.Codec.encode #[mkConst (info.twin ++ `codec)]
    let rhs ← mkAppM ``Function.comp
      #[← mkAppM ``LeanerIR.Proofs.Codec.encode #[← mkAppM ``NTy.codec #[τE]],
        mkApp2 (mkConst nativeName) (← Contract.frameUnit skolems) skolems]
    let pointwise ← withLocalDeclD `value twinType fun value => do
      let erased := mkApp3 (mkConst eraseName) (← Contract.frameUnit skolems) skolems value
      let encoded ← mkAppM ``NTy.codec_encode #[τE, mkApp3 (mkConst nativeName) (← Contract.frameUnit skolems) skolems value]
      mkLambdaFVars #[value] (← mkEqTrans erased (← mkEqSymm encoded))
    pure (← mkForallFVars (← frameBinders skolems) (← mkEq lhs rhs),
      ← mkLambdaFVars (← frameBinders skolems) (← mkAppM ``funext #[pointwise]))
  addDecl (.thmDecl { name := codecName, levelParams := [], type := statement, value := proof })

/-- Close an equation whose sides the kernel identifies: the elaborator's
defeq check does not unfold the codecs a twin's encoding goes through, and
the declaration's kernel check decides it. -/
elab "leaner_twin_kernel_rfl" : tactic => do
  let goal ← Lean.Elab.Tactic.getMainGoal
  let target ← instantiateMVars (← goal.getType)
  let some (_, lhs, _) := target.eq? | throwError "the bridge goal is not an equation"
  goal.assign (← mkEqRefl lhs)
  Lean.Elab.Tactic.replaceMainGoal []

/-- A twin at the arguments one use applies: a non-generic twin at none, a
generic one at its own parameters (the skolem family) or at represented
types without parameters. -/
private structure Spelling where
  info : SpecTypes.TwinInfo
  arguments : Array SpecTypes.FieldRep := #[]

/-- A generic twin at its own parameters, in order. -/
private def Spelling.isSkolem (spelling : Spelling) : Bool :=
  spelling.info.typeParameterCount != 0 &&
    spelling.arguments == (Array.range spelling.info.typeParameterCount).map .parameter

/-- A spelling a bridge is stated at: non-generic, the skolem family, or
concrete arguments. -/
private def Spelling.bridged (spelling : Spelling) : Bool :=
  spelling.info.typeParameterCount == 0 || spelling.isSkolem ||
    spelling.arguments.all (·.parameterCount == 0)

/-- The root of a spelling's bridge lemmas: the twin's own, else a name
keyed by the arguments. -/
private def Spelling.root (spelling : Spelling) : Name :=
  if spelling.info.typeParameterCount == 0 || spelling.isSkolem then spelling.info.twin
  else spelling.info.twin ++ Name.mkSimple s!"at_{hash (reprStr spelling.arguments)}"

/-- The native type a represented field denotes. -/
private partial def fieldNTy? (unit : ValidatedUnit) (twins : Array SpecTypes.TwinInfo) :
    SpecTypes.FieldRep → Option NTy
  | .int (.bits width) signed => some (.int width signed)
  | .int .. => none
  | .bool => some .bool
  | .string => some .string
  | .address => some .address
  | .signer => some .signer
  | .bytes => some .bytes
  | .unit => some .unit
  | .vector element _ => (fieldNTy? unit twins element).map .vector
  | .parameter index => some (.param index)
  | .nominal twin arguments => do
      let info ← twins.find? (·.twin == twin)
      let τ ← structNTy unit ⟨⟨info.namespaceIndex⟩, info.structIndex⟩
      let arguments ← arguments.mapM (fieldNTy? unit twins)
      return τ.subst (NRow.ofList arguments.toList)

/-- The nested spellings of a spelling's fields: a nominal field's, and a
vector's elements'. -/
private def Spelling.nested (spelling : Spelling) (twins : Array SpecTypes.TwinInfo) :
    Array (Spelling × Bool) :=
  spelling.info.fields.filterMap fun (_, rep) =>
    match rep.instantiate spelling.arguments with
    | .nominal twin arguments =>
        (twins.find? (·.twin == twin)).map fun inner => (⟨inner, arguments⟩, false)
    | .vector (.nominal twin arguments) _ =>
        (twins.find? (·.twin == twin)).map fun inner => (⟨inner, arguments⟩, true)
    | _ => none

/-- Whether a spelling's bridge rewrites: an enum is bridged through its
native view, a vector of nested twins through the composition of two maps,
and a struct through the bridges of the fields that rewrite. -/
private partial def Spelling.rewrites (spelling : Spelling) (twins : Array SpecTypes.TwinInfo) :
    Bool :=
  !spelling.info.variants.isEmpty ||
    (spelling.nested twins).any fun (nested, vector) => vector || nested.rewrites twins

/-- The bridge between a struct twin at one spelling and its native type:
the twin's erasure is the native encoding of the field view
(`erase_eq_encode`), and so is its codec's encoding (`codec_encode_eq`),
which a vector of it maps. Nested spellings are bridged first; a bridge
that rewrites uses theirs. -/
private partial def ensureSpellingBridge (unit : ValidatedUnit) (twins : Array SpecTypes.TwinInfo)
    (spelling : Spelling) : TermElabM Unit := do
  let info := spelling.info
  unless info.variants.isEmpty do
    if info.typeParameterCount == 0 then ensureEnumNative unit twins info
    return
  unless spelling.bridged do return
  let root := spelling.root
  let eraseName := root ++ `erase_eq_encode
  if (← getEnv).contains eraseName then return
  let handle : LeanerIR.StructHandle := ⟨⟨info.namespaceIndex⟩, info.structIndex⟩
  let some generic := structNTy unit handle | return
  let some argumentTypes := spelling.arguments.mapM (fieldNTy? unit twins) | return
  let τ := if info.typeParameterCount == 0 || spelling.isSkolem then generic
    else generic.subst (NRow.ofList argumentTypes.toList)
  let .struct _ _ row := τ | return
  let mut lemmas : Array Name := #[]
  for (nested, vector) in spelling.nested twins do
    unless nested.bridged do continue
    ensureSpellingBridge unit twins nested
    let nestedRoot := if nested.info.variants.isEmpty then nested.root else nested.info.twin
    if vector then lemmas := lemmas.push (nestedRoot ++ `codec_encode_eq)
    else if nested.rewrites twins then lemmas := lemmas.push (nestedRoot ++ `erase_eq_encode)
  let τE ← quoteNTy τ
  let applied (suffix : Name) (skolems : Lean.Expr) : MetaM Lean.Expr := do
    if info.typeParameterCount == 0 then return mkConst (info.twin ++ suffix)
    let carriers ← Contract.frameCarriers skolems
    let codecs ← if spelling.isSkolem then
        pure ((Array.range info.typeParameterCount).map fun index =>
          mkApp2 (mkConst ``Carriers.codec) carriers (toExpr index))
      else spelling.arguments.mapM (·.codec none)
    mkAppM (info.twin ++ suffix) codecs
  -- The erasure at the family, its argument type, and the field view.
  let viewAt (skolems : Lean.Expr) : MetaM (Lean.Expr × Lean.Expr × Lean.Expr) := do
    let eraseFn ← applied `erase skolems
    let twinType := (← whnfD (← inferType eraseFn)).bindingDomain!
    let view ← withLocalDeclD `value twinType fun value => do
      mkLambdaFVars #[value] (← twinFields twins info row value)
    return (eraseFn, twinType, view)
  let (statement, reflexive, codecStatement) ← withSkolems fun skolems => do
    let (eraseFn, twinType, view) ← viewAt skolems
    let (statement, reflexive) ← withLocalDeclD `value twinType fun value => do
      let lhs := mkApp eraseFn value
      let rhs ← mkAppM ``NTy.encode #[τE, view.beta #[value]]
      pure (← mkForallFVars ((← frameBinders skolems).push value) (← mkEq lhs rhs),
        ← mkLambdaFVars ((← frameBinders skolems).push value) (← mkEqRefl lhs))
    let codecStatement ← mkForallFVars (← frameBinders skolems) (← mkEq
      (← mkAppM ``LeanerIR.Proofs.Codec.encode #[← applied `codec skolems])
      (← mkAppM ``Function.comp
        #[← mkAppM ``LeanerIR.Proofs.Codec.encode #[← mkAppM ``NTy.codec #[τE]], view]))
    pure (statement, reflexive, codecStatement)
  -- Outside the skolem context, where no second family shadows the
  -- statement's.
  let proof ← if spelling.rewrites twins then do
      let rules ← (lemmas.push (info.twin ++ `erase)).mapM fun name =>
        `(Lean.Parser.Tactic.simpLemma| $(mkIdent name):ident)
      let proofSyntax ← `(term| by
        intros
        simp only [$rules,*, LeanerIR.Proofs.Codec.boundedVector_encode,
          LeanerIR.Proofs.Denote.NTy.encode_struct, LeanerIR.Proofs.Denote.NTy.encode_vector,
          LeanerIR.Proofs.Denote.HList.encode_cons, LeanerIR.Proofs.Denote.HList.encode_nil,
          LeanerIR.SpecVector.map_values]
        all_goals repeat erw [Array.map_map]
        all_goals leaner_twin_kernel_rfl)
      let proof ← Lean.Elab.Term.elabTermEnsuringType proofSyntax (some statement)
      Lean.Elab.Term.synthesizeSyntheticMVarsNoPostponing
      instantiateMVars proof
    else pure reflexive
  addDecl (.thmDecl { name := eraseName, levelParams := [], type := statement, value := proof })
  let attr ← `(attr| lir_denote_norm)
  Lean.Elab.Term.applyAttributes eraseName #[{ name := `lir_denote_norm, stx := attr, kind := .global }]
  -- The codec's encoding, pointwise the erasure's.
  let codecProof ← withSkolems fun skolems => do
    let (_, twinType, view) ← viewAt skolems
    let pointwise ← withLocalDeclD `value twinType fun value => do
      let erased := mkApp3 (mkConst eraseName) (← Contract.frameUnit skolems) skolems value
      let encoded ← mkAppM ``NTy.codec_encode #[τE, view.beta #[value]]
      mkLambdaFVars #[value] (← mkEqTrans erased (← mkEqSymm encoded))
    mkLambdaFVars (← frameBinders skolems) (← mkAppM ``funext #[pointwise])
  let codecName := root ++ `codec_encode_eq
  addDecl (.thmDecl
    { name := codecName, levelParams := [], type := codecStatement, value := codecProof })

/-- The bridges of the unit's twins: every non-generic twin, and every
generic one at the skolem family, where its type parameters are the
family's carriers. -/
private def ensureTwinBridges (unit : ValidatedUnit) (twins : Array SpecTypes.TwinInfo) :
    TermElabM Unit := do
  for info in twins do
    let arguments := (Array.range info.typeParameterCount).map .parameter
    let bridged := (← getEnv).contains (info.twin ++ `erase_eq_encode)
    ensureSpellingBridge unit twins ⟨info, arguments⟩
    if !bridged && info.variants.isEmpty && info.typeParameterCount != 0 then
      let decodeAttr ← `(attr| lir_denote_norm)
      Lean.Elab.Term.applyAttributes (info.twin ++ `decode?)
        #[{ name := `lir_denote_norm, stx := decodeAttr, kind := .global }]

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
def _root_.LeanerIR.Proofs.Denote.ResultShape.returnsReference : ResultShape → Bool
  | .one (.ref _) => true
  | .one (.tuple elements) => elements.toList.any fun | .ref _ => true | _ => false
  | _ => false

/-- Whether a callee's contract stands for it at a call: always, unless it
returns a mutable reference without its contract stating the reference's
final value, when its body stands for it instead. -/
def contractStandsFor (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : Bool :=
  !(nativeSignature? unit namespaceId declaration).any (·.2.returnsReference) ||
    hasFinalContract ns declaration || (Contract.mapRoleOf? unit namespaceId ns declaration).isSome

/-- The heartbeat budget a function sets for its own verification, in the
units of `leaner.verifyHeartbeats`: `pragma heartbeats = N` counts thousands
of them. -/
def heartbeatBudget? (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) :
    Option Nat :=
  declaration.pragmas.findSome? fun
    | .assign "heartbeats" (.constant (.integer value)) _ =>
        if 0 < value then some (value.toNat * 1000) else none
    | _ => none

/-- Whether a function's specification or its module sets `pragma verify =
manual`: an authored proof, `verify f by …`, establishes it. -/
def requiresAuthoredProof (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) :
    Bool :=
  declaration.pragmas.any fun
    | .assign "verify" (.name none "manual") _ => true
    | _ => false

/-- Whether a function is specified `pragma opaque`. -/
def isOpaque (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : Bool :=
  declaration.pragmas.any fun
    | .assign "opaque" (.constant (.bool true)) _ => true
    | _ => false

/-- Whether a function selects bit-vector decisions, `pragma bv`: the
decision applies to every unsigned value of a leaf, not only the
parameters or results the pragma names. -/
def selectsBitVectors (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : Bool :=
  declaration.pragmas.any fun
    | .assign name _ _ => name == "bv" || name == "bv_ret"
    | _ => false

/-- The functions an expression calls directly. -/
private partial def expressionCallees (unit : ValidatedUnit) (namespaceId : NamespaceId)
    (id : LeanerIR.ExprId) (found : Array FunctionHandle) : Array FunctionHandle :=
  match unit.namespaces[namespaceId.index]?.bind (·.expressions[id.index]?) with
  | none => found
  | some expression =>
      let found := match expression.kind with
        | .operation (.call (.function reference)) _ _ _ =>
            match LeanerIR.SemanticOperations.resolveFunction? unit namespaceId reference with
            | some callee => if found.contains callee then found else found.push callee
            | none => found
        | _ => found
      (LeanerIR.Validation.expressionChildren expression.kind).foldl
        (fun found child => expressionCallees unit namespaceId child found) found

/-- The functions a function's body calls directly. -/
private def bodyCallees (unit : ValidatedUnit) (handle : FunctionHandle) : Array FunctionHandle :=
  match unit.namespaces[handle.namespaceId.index]?.bind (·.functions[handle.functionId.index]?) with
  | some { body := .structured root, .. } => expressionCallees unit handle.namespaceId root #[]
  | _ => #[]

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
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : Bool :=
  -- An intrinsic map role stands for the map model, as a native does.
  (Contract.mapRoleOf? unit callee.namespaceId ns declaration).isSome ||
  declaration.contract.loc.isSome && contractStandsFor unit callee.namespaceId ns declaration &&
    (declaration.body == .absent || isOpaque declaration || onCallCycle unit callee)

/-- Whether a function has type parameters: it is proved over every skolem
family and type instantiation. -/
def isGeneric (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : Bool :=
  declaration.signature.generics.any (·.kind == .typeArg)

/-- The typed contract of a function over its native arguments, abstracted
over the skolem family and, for a generic function, its type
instantiation. -/
def typedContractOf (unit : ValidatedUnit) (namespaceIndex : Nat) (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (params : NRow) (result : ResultShape) (twins : Array SpecTypes.TwinInfo)
    (nativeModel : Option NativeModel := none) (stored : Contract.StoredInvariants := {}) :
    MetaM Lean.Expr :=
  -- A contract is over a unit and a frame at it; one stating a behavioral
  -- predicate over an executable unit of it as well.
  let readsUnit := Contract.contractReadsUnit unit ⟨namespaceIndex⟩ ns declaration
  let overFrame (k : Array Lean.Expr → Option Lean.Expr → Lean.Expr → MetaM Lean.Expr) :=
    if readsUnit then withExecutableSkolems fun executable skolems => do
      k (← executableBinders executable skolems) (some executable) skolems
    else withSkolems fun skolems => do k (← frameBinders skolems) none skolems
  overFrame fun binders executable skolems => do
    let signature ← quoteNativeSignature skolems params result
    let carriers ← Contract.frameCarriers skolems
    let types ← Contract.frameTypes skolems
    let build (typeInstantiation requiresTable : Option Lean.Expr) :=
      buildContract unit ⟨namespaceIndex⟩ ns declaration signature twins
        (carrier := mkApp (mkConst ``Carriers.carrier) carriers)
        (codecs := mkApp (mkConst ``Carriers.codec) carriers)
        (types := types) (typeInstantiation := typeInstantiation)
        (nativeModel := nativeModel) (executable := executable) (requiresTable := requiresTable)
        (stored := stored)
    -- One stating `requires_of` takes the table of declared preconditions
    -- at the unit, last.
    let close (binders : Array Lean.Expr) (typeInstantiation : Option Lean.Expr) :
        MetaM Lean.Expr := do
      if let some executable := executable then
        if Contract.contractReadsRequires unit ⟨namespaceIndex⟩ declaration then
          let tableType := mkApp (mkConst ``LeanerIR.Proofs.RequiresTable)
            (← Contract.executableUnit executable)
          return ← withLocalDeclD `requiresTable tableType fun table => do
            mkLambdaFVars (binders.push table) (← build typeInstantiation (some table))
      mkLambdaFVars binders (← build typeInstantiation none)
    if isGeneric declaration then
      let instantiationType ← mkAppM ``Array
        #[← mkAppM ``Prod #[mkConst ``LeanerIR.TypeId, mkConst ``LeanerIR.TypeId]]
      withLocalDeclD `typeInstantiation instantiationType fun typeInstantiation =>
        close (binders.push typeInstantiation) (some typeInstantiation)
    else close binders none


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
        withLocalDeclD `requiresTable (mkApp (mkConst ``LeanerIR.Proofs.RequiresTable) unitExpr)
          fun table => do
            mkLambdaFVars (#[unitExpr, executable] ++ binders.push table) (← contract table)
      else do
        mkLambdaFVars (#[unitExpr, executable] ++ binders) (← contract (mkConst ``Unit.unit))
    let applyTable (typed : Lean.Expr) (table : Lean.Expr) : Lean.Expr :=
      if readsUnit && readsRequires then mkApp typed table else typed
    if generic then
      let instantiationType ← mkAppM ``Array
        #[← mkAppM ``Prod #[mkConst ``LeanerIR.TypeId, mkConst ``LeanerIR.TypeId]]
      withLocalDeclD `θ (mkConst ``TypeArgs) fun θ =>
      withLocalDeclD `typeInstantiation instantiationType fun instantiation => do
        let frame := mkApp3 (mkConst ``Skolems.instantiate) unitExpr θ runtime
        let arguments ← mkAppM ``Subtype.val #[θ]
        close #[θ, instantiation] fun table => do
          let typed := (applyTable (mkApp (atFrame frame) instantiation) table).headBeta
          let read ← mkAppOptM ``LeanerIR.Proofs.Contract.ofSkolem
            #[some unitExpr, some runtime, some θ, some row, some shape, some typed]
          mkAppOptM ``LeanerIR.Proofs.Contract.prophetic
            #[some unitExpr, some runtime, some (mkApp2 (mkConst ``NRow.subst) arguments row),
              some (mkApp2 (mkConst ``ResultShape.subst) arguments shape), some read]
    else
      close #[] fun table => do
        let typed := (applyTable (atFrame runtime) table).headBeta
        mkAppOptM ``LeanerIR.Proofs.Contract.prophetic
          #[some unitExpr, some runtime, some row, some shape, some typed]

/-- The declarations whose stored invariants a unit's predicate states, and
those whose invariants it does not carry, keyed by the predicate's name.
Recorded once, with the predicate, for every module that builds contracts
over the unit. -/
private initialize storedDeclarations :
    SimplePersistentEnvExtension
      (Name × Array LeanerIR.StructHandle × Array (LeanerIR.StructHandle × String) ×
        Bool × Array (LeanerIR.StructHandle × String))
      (NameMap (Array LeanerIR.StructHandle × Array (LeanerIR.StructHandle × String) ×
        Bool × Array (LeanerIR.StructHandle × String))) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := fun map (name, declarations) => map.insert name declarations
    addImportedFn := fun entries =>
      mkStateFromImportedEntries (fun map (name, declarations) => map.insert name declarations)
        {} entries }

/-- Define once per unit the predicate of the data invariants of its stored
resources (`Contract.storedInvariantTerm`), which the closer's normalization
unfolds where it is applied, and record the declarations it covers. -/
def ensureStoredInvariant (segments : Array String) (unit : ValidatedUnit)
    (twins : Array SpecTypes.TwinInfo) : CommandElabM Contract.StoredInvariants := do
  let name := storedInvariantName segments
  if let some (carriers, unsupported, hasCollections, collectionUnsupported) :=
      (storedDeclarations.getState (← getEnv)).find? name then
    let predicate := if (← getEnv).contains name then some (mkConst name) else none
    return { predicate, carriers, unsupported, hasCollections, collectionUnsupported }
  let stored ← liftTermElabM (Contract.storedInvariantTerm unit twins)
  modifyEnv fun env => storedDeclarations.addEntry env
    (name, stored.carriers, stored.unsupported, stored.hasCollections, stored.collectionUnsupported)
  if stored.cases.isEmpty && stored.collectionCases.isEmpty then return stored
  liftTermElabM do
    -- Over the unit memory is typed at, then the executable unit where an
    -- invariant reads it.
    let readsUnit := Contract.storedInvariantReadsUnit unit
    let valueProp ← mkArrow (mkConst ``RuntimeValue) (mkSort .zero)
    let withUnit (type : Lean.Expr) : MetaM Lean.Expr :=
      withLocalDeclD `unit (mkConst ``ValidatedUnit) fun unitExpr => do
        if readsUnit then
          mkForallFVars #[unitExpr] (← mkArrow (Contract.executableType unitExpr) type)
        else mkForallFVars #[unitExpr] type
    -- A constant and its equation, which the closer's normalization unfolds
    -- where the constant is applied to all its arguments.
    let define (constant : Name) (type value : Lean.Expr) (arity : Nat) : TermElabM Unit := do
      addDecl (.defnDecl {
        name := constant, levelParams := [], type, value, hints := .regular 0, safety := .safe })
      let statement ← forallBoundedTelescope type arity fun binders _ => do
        mkForallFVars binders (← mkEq (mkAppN (mkConst constant) binders) (value.beta binders))
      let proof ← forallTelescope statement fun binders equality => do
        mkLambdaFVars binders (← mkEqRefl equality.appArg!)
      let equation := constant ++ `eq
      addDecl (.thmDecl { name := equation, levelParams := [], type := statement, value := proof })
      let attr ← `(attr| lir_denote_norm)
      Lean.Elab.Term.applyAttributes equation
        #[{ name := `lir_denote_norm, stx := attr, kind := .global }]
    -- Each declaration's invariant behind a constant of its own, selected
    -- by the declaration before it is applied: a value of another resource
    -- type never unfolds it.
    let arity := if readsUnit then 3 else 2
    let mut constants : Array Name := #[]
    for case in stored.cases, index in [0:stored.cases.size] do
      let constant := name ++ Name.mkSimple s!"case{index}"
      define constant (← withUnit valueProp) case arity
      constants := constants.push constant
    let argumentsProp ← mkArrow (mkConst ``NRow) valueProp
    let mut collectionConstants : Array (LeanerIR.StructHandle × Name) := #[]
    for (owner, case) in stored.collectionCases, index in [0:stored.collectionCases.size] do
      let constant := name ++ Name.mkSimple s!"collectionCase{index}"
      define constant (← withUnit argumentsProp) case (arity + 1)
      collectionConstants := collectionConstants.push (owner, constant)
    let predicate ← withLocalDeclD `unit (mkConst ``ValidatedUnit) fun unitExpr =>
      withLocalDeclD `executable (Contract.executableType unitExpr) fun executable =>
      withLocalDeclD `resource (mkConst ``ResourceType) fun resource =>
        withLocalDeclD `value (mkConst ``RuntimeValue) fun held => do
          let leading := if readsUnit then #[unitExpr, executable] else #[unitExpr]
          let handle? := mkApp (mkConst ``LeanerIR.Proofs.Denote.NTy.handle?)
            (mkApp (mkConst ``LeanerIR.Proofs.Denote.ResourceType.type) resource)
          let mut selected ← withLocalDeclD `value (mkConst ``RuntimeValue) fun other =>
            mkLambdaFVars #[other] (mkConst ``True)
          for (owner, constant) in (stored.carriers.zip constants).reverse do
            let test ← mkEq handle? (← mkAppM ``Option.some #[toExpr owner])
            selected ← mkAppM ``ite #[test, mkAppN (mkConst constant) leading, selected]
          let globals ← mkLambdaFVars #[resource, held] (mkApp selected held)
          if collectionConstants.isEmpty then return ← mkLambdaFVars leading globals
          let declared ← withLocalDeclD `owner (mkConst ``LeanerIR.StructHandle) fun owner =>
            withLocalDeclD `arguments (mkConst ``NRow) fun arguments =>
            withLocalDeclD `raw (mkConst ``RuntimeValue) fun raw => do
              let mut selected ← withLocalDeclD `arguments (mkConst ``NRow) fun arguments =>
                withLocalDeclD `raw (mkConst ``RuntimeValue) fun raw =>
                  mkLambdaFVars #[arguments, raw] (mkConst ``True)
              for (handle, constant) in collectionConstants.reverse do
                selected ← mkAppM ``ite #[← mkEq owner (toExpr handle),
                  mkAppN (mkConst constant) leading, selected]
              mkLambdaFVars #[owner, arguments, raw] (mkApp2 selected arguments raw)
          mkLambdaFVars leading
            (← mkAppM ``LeanerIR.Proofs.Denote.DataInvariant.withCollections #[globals, declared])
    define name (← withUnit (← mkArrow (mkConst ``ResourceType) valueProp)) predicate (arity + 1)
  return { stored with predicate := some (mkConst name) }

/-- Define the typed and public contracts of a function, once: the contract
over its native arguments, and its runtime form. -/
def ensureContracts (segments : Array String) (function : String) (unit : ValidatedUnit)
    (namespaceIndex : Nat) (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (params : NRow) (result : ResultShape) (nativeModel : Option NativeModel := none) :
    CommandElabM Unit := do
  let typedName := typedContractName segments function
  let publicName := contractName segments function
  if (← getEnv).contains publicName then
    let some info := (← getEnv).find? typedName
      | throwError m!"`{function}` has a contract this unit did not generate"
    unless info.type.getUsedConstants.contains ``HList do
      throwError m!"`{function}` has a contract this unit did not generate"
    return
  let twins ← SpecTypes.ensureSpecTypes segments unit
  for info in twins do
    if info.variants.isEmpty && info.typeParameterCount == 0 then ensureDecodeBridge twins info
  let stored ← ensureStoredInvariant segments unit twins
  liftTermElabM do
    ensureTwinBridges unit twins
    -- A body is proved against its implementation; a native, which has
    -- none, is only ever seen by its callers.
    let view := if declaration.body == .absent then ContractView.interface
      else .implementation
    addAbbrev typedName (← typedContractOf unit namespaceIndex ns (view.of declaration)
      params result twins nativeModel stored)
    addAbbrev publicName
      (← publicContractOf params result (isGeneric declaration) (mkConst typedName)
        (Contract.contractReadsUnit unit ⟨namespaceIndex⟩ ns (view.of declaration))
        (Contract.contractReadsRequires unit ⟨namespaceIndex⟩ (view.of declaration)))

private def countErrors (log : MessageLog) : Nat :=
  log.reportedPlusUnreported.toList.filter (·.severity == .error) |>.length

/-- The proof file a namespace's `proof_file` pragma names: where the
authored proofs of its functions live. -/
private def proofFile? (ns : ValidatedNamespace) : Option String :=
  ns.pragmas.findSome? fun
    | .assign "proof_file" (.constant (.string path)) _ => some path
    | _ => none

/-- Where a function's proof goes: `verify f by …` in the module's proof file
when it has one. -/
private def proofRequest (ns : ValidatedNamespace) (function : String) : MessageData :=
  let place := match proofFile? ns with
    | some path => m!"in `{path}`"
    | none => m!"in the module"
  m!"provide a proof: `verify {function} by …` {place} (`verify {function} by skip` shows the \
    obligations it leaves)"

/-- What a failed verification asks for: an authored proof did not close
its obligations; an automatic one asks for a proof, or for a larger budget
when the attempt ran out. -/
private def failureMessage (ns : ValidatedNamespace) (function : String) (authored : Bool)
    (overBudget : Bool) (budget : Nat) : MessageData :=
  if authored then
    m!"leaner verification failed: the proof of `{function}` does not establish its \
      specification"
  else
    let attempt := if overBudget then
        m!"the automatic verification of `{function}` exceeded its budget of {budget} \
          maxHeartbeats"
      else m!"the automatic verification of `{function}` failed"
    let raise := if overBudget then
        m!", or raise the budget with `pragma heartbeats = N`, in thousands of maxHeartbeats"
      else m!""
    m!"leaner verification failed: {attempt}; {proofRequest ns function}{raise}"

/-- Generate the contract a function's callers see of it, where that
differs from the one its body is proved against: they assume it. -/
def ensureInterfaceContract (segments : Array String) (function : String) (unit : ValidatedUnit)
    (namespaceIndex : Nat) (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (params : NRow) (result : ResultShape) : CommandElabM Unit := do
  let name := typedInterfaceContractName segments function
  if (← getEnv).contains name then return
  let twins ← SpecTypes.ensureSpecTypes segments unit
  for info in twins do
    if info.variants.isEmpty && info.typeParameterCount == 0 then ensureDecodeBridge twins info
  let stored ← ensureStoredInvariant segments unit twins
  liftTermElabM do
    ensureTwinBridges unit twins
    let contract ← typedContractOf unit namespaceIndex ns
      (ContractView.interface.of declaration) params result twins (stored := stored)
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

/-- The clause-level value of a native slot binder, when a clause can read it. -/
private def logicalValue (τ : NTy) (value : Lean.Expr) : MetaM (Option Lean.Expr) := do
  match τ with
  | .int _ _ => return some (← mkAppM ``LeanerIR.SpecInt.val #[value])
  | .bool | .address | .signer | .string => return some value
  | .unit | .bytes => return none
  | .param _ => return some value
  | .tuple _ | .struct _ _ _ | .enum _ _ _ _ _ | .vector _ | .function _ _ _ =>
      return some (← mkAppM ``NTy.encode #[← quoteNTy τ, value])
  | .ref referent => logicalValue referent (← mkAppM ``Prod.fst #[value])

/-- The locals a pattern binds. -/
private partial def patternLocals (ns : ValidatedNamespace) (pattern : LeanerIR.PatternId) :
    Array Nat :=
  match ns.patterns[pattern.index]? with
  | some { kind := .variable localId, .. } => #[localId.index]
  | some { kind := .tuple children, .. }
  | some { kind := .constructor _ _ _ children, .. } => children.flatMap (patternLocals ns)
  | _ => #[]

/-- The locals an expression tree reads from its context: those it binds
itself, by a quantifier, a `let`, or a match arm, are its own. -/
private partial def referencedLocals (ns : ValidatedNamespace) (id : LeanerIR.ExprId)
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
      (LeanerIR.Validation.expressionChildren expression.kind).foldl
        (fun found child => referencedLocals ns child found bound) found

/-- The locals a clause reads at its own state: those outside `old(..)`,
which reads another state. -/
private partial def currentLocals (ns : ValidatedNamespace) (id : LeanerIR.ExprId)
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
      (LeanerIR.Validation.expressionChildren expression.kind).foldl
        (fun found child => currentLocals ns child found bound) found

/-- The locals read under `old(..)` within a clause: in a loop invariant,
their values at the function's start. -/
private partial def oldReferencedLocals (ns : ValidatedNamespace) (id : LeanerIR.ExprId)
    (found : Array Nat := #[]) : Array Nat :=
  match ns.expressions[id.index]? with
  | none => found
  | some expression => match expression.kind with
      | .operation (.specification .old) _ arguments _ =>
          arguments.foldl (fun found argument => referencedLocals ns argument found) found
      | kind => (LeanerIR.Validation.expressionChildren kind).foldl
          (fun found child => oldReferencedLocals ns child found) found

/-- Whether an expression reads the function's start state: a behavioral
predicate without a state label reads the invocation from it. -/
private partial def readsEntryState (ns : ValidatedNamespace) (id : LeanerIR.ExprId) : Bool :=
  match ns.expressions[id.index]? with
  | none => false
  | some expression => match expression.kind with
      | .operation (.specification (.behavior _ range)) _ arguments _ =>
          range.pre.isNone || arguments.any (readsEntryState ns)
      | kind => (LeanerIR.Validation.expressionChildren kind).any (readsEntryState ns)

/-- The root local of a place, and whether the place goes through a
dereference of it. -/
private partial def placeRootLocal? (ns : ValidatedNamespace) (place : LeanerIR.PlaceId)
    (fuel : Nat := ns.places.size) : Option (Nat × Bool) :=
  match fuel, ns.places[place.index]? with
  | 0, _ | _, none => none
  | _ + 1, some (.localVar localId) => some (localId.index, false)
  | fuel + 1, some (.deref base) =>
      (placeRootLocal? ns base fuel).map fun (root, _) => (root, true)
  | fuel + 1, some (.field base _ _) | fuel + 1, some (.index base _)
  | fuel + 1, some (.subslice base ..) | fuel + 1, some (.downcast base _) =>
      placeRootLocal? ns base fuel

/-- The locals a loop body can change, each with whether the body only
writes through it as a reference: those it binds, assigns, or lends
mutably, since a lender holds its borrow's prophecy from the borrow on.  A local
written through only keeps its prophecy; every other local keeps its entry
value through the loop. -/
private partial def loopBodyLocals (ns : ValidatedNamespace) (id : LeanerIR.ExprId)
    (found : Array (Nat × Bool) := #[]) : Array (Nat × Bool) :=
  match ns.expressions[id.index]? with
  | none => found
  | some expression =>
      let rec bound (pattern : LeanerIR.PatternId) : Array (Nat × Bool) :=
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
            | some (LeanerIR.ExprKind.localVar localId) => #[(localId.index, true)]
            | _ => #[]
        | _ => #[]
      let found := direct.foldl (init := found) fun found (slot, through) =>
        match found.findIdx? (·.1 == slot) with
        | some index => found.set! index (slot, found[index]!.2 && through)
        | none => found.push (slot, through)
      (LeanerIR.Validation.expressionChildren expression.kind).foldl
        (fun found child => loopBodyLocals ns child found) found

mutual
/-- Whether evaluating an expression may change the global store: through a
global operation, or through a call to a function that may. An invocation
of a `kept` local, a function-typed parameter the body does not reassign,
keeps the store (`KeepsMemoryAt`). -/
private partial def mayTouchStore (unit : ValidatedUnit) (namespaceId : NamespaceId)
    (id : LeanerIR.ExprId) (visiting : Array FunctionHandle) (kept : Array Nat := #[]) : Bool :=
  match unit.namespaces[namespaceId.index]? with
  | none => true
  | some ns =>
  match ns.expressions[id.index]? with
  | none => true
  | some expression =>
      let direct := match expression.kind with
        | .operation (.global _) _ _ _ => true
        | .operation (.call (.function reference)) _ _ _ =>
            match LeanerIR.SemanticOperations.resolveFunction? unit namespaceId reference with
            | some callee => calleeMayTouchStore unit callee visiting
            | none => true
        | .operation (.call (.constructor ..)) _ _ _
        | .operation (.call (.destructor ..)) _ _ _ => false
        | .operation (.call .invoke) _ arguments _ =>
            match (arguments[0]?.bind fun callee => ns.expressions[callee.index]?).map (·.kind) with
            | some (LeanerIR.ExprKind.localVar localId) => !kept.contains localId.index
            | _ => true
        | .operation (.call (.closure _ _)) _ _ _
        | .operation (.call (.extension ..)) _ _ _ => true
        | _ => false
      direct || (LeanerIR.Validation.expressionChildren expression.kind).any
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
private def bodySpecifications (ns : ValidatedNamespace) (root : LeanerIR.ExprId)
    (accept : LeanerIR.ConditionKind → Bool) : Array (LeanerIR.ExprId × LeanerIR.SpecBlock) :=
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
      pending := pending ++ LeanerIR.Validation.expressionChildren expression.kind
    return found

/-- The types a clause reads the locals of a function at: a reference local
at its referent. -/
private def clauseLocalTypes (unit : ValidatedUnit)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) :
    MetaM (Array LeanerIR.Ty) :=
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
  return mkApp (mkConst ``LeanerIR.Proofs.Denote.Memory) (← Contract.executableUnit executable)

/-- The invariant of each loop of a function, keyed by the loop's site:
over the function's starting locals and state (what `old` reads), the loop's
entry locals and state, and the current locals and state; first over the
executable unit where a clause states a behavioral predicate, which the
returned flag marks. -/
def loopInvariants (unit : ValidatedUnit) (outer : Lean.Expr) (namespaceId : NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (params : NRow) (row : NRow) (codecs types : Option Lean.Expr)
    (twins : Array SpecTypes.TwinInfo) (storedInvariant : Option Lean.Expr) :
    TermElabM (Array (Nat × Lean.Expr × Bool)) := do
  let .structured root := declaration.body | return #[]
  let envType ← mkAppM ``HEnv #[← quoteRow row]
  let argumentsType ← mkAppM ``HList #[← quoteRow params]
  let parameterTypes := params.toList
  let row := row.toList
  let localTypes ← clauseLocalTypes unit declaration
  let mut invariants := #[]
  -- The function-typed parameters without a frame that the body does not
  -- reassign keep memory.
  let written := loopBodyLocals ns root
  let kept := (List.range parameterTypes.length).toArray.filter fun index =>
    parameterTypes[index]? matches some (.function ..) && !written.any (·.1 == index) &&
      !declaration.contract.parameterFrames.any (·.parameter.index == index)
  for (site, block) in loopSpecifications ns root do
    -- An unannotated loop whose body runs once keeps its entry exactly.
    let runsOnce := match ns.expressions[site.index]? with
      | some { kind := .loop _ body, .. } => LeanerIR.Validation.loopBodyRunsOnce ns body
      | _ => false
    if runsOnce && !block.conditions.any (·.kind == .loopInvariant) then
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
        let dataInvariantSlots := changed.filterMap fun (slot, _) =>
          if header.any (·.index == slot) && slot < row.length &&
              localTypes[slot]?.any (Contract.hasDataInvariant unit ·) then some slot
          else none
        let referenced := block.conditions.foldl (fun found condition =>
          if condition.kind == .loopInvariant then currentLocals ns condition.expression found
          else found) dataInvariantSlots
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
            translateLoopInvariants unit namespaceId ns block locals localTypes codecs types
              (declaration.locals.map (·.name)) oldLocals startState twins
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
              (← mkAppM ``LeanerIR.Proofs.Denote.MemoryInvariants #[invariant, startState])
              (← mkAppM ``LeanerIR.Proofs.Denote.MemoryInvariants #[invariant, state]))
        else
          stateFrame := stateFrame.push (← mkEq state initial)
        let conjunction ← ((frame.push authored) ++ stateFrame).foldrM (init := mkConst ``True)
          fun clause rest => mkAppM ``And #[clause, rest]
        mkLambdaFVars #[start, startState, entry, initial, env, state] conjunction
    invariants := invariants.push (loopSite namespaceId.index site.index, invariant, readsUnit)
  return invariants

/-- The labels of the state anchors an expression tree reads. -/
private partial def anchorLabels (ns : ValidatedNamespace) (id : LeanerIR.ExprId)
    (found : Array Nat := #[]) : Array Nat :=
  match ns.expressions[id.index]? with
  | none => found
  | some expression =>
      let found := match expression.kind with
        | .operation (.specification (.withStateAnchor label)) _ _ _ =>
            if found.contains label then found else found.push label
        | _ => found
      (LeanerIR.Validation.expressionChildren expression.kind).foldl
        (fun found child => anchorLabels ns child found) found

/-- The locals read under `old(..)` within the state anchor `anchor`, or,
for `none`, outside every anchor; `within` is the anchor around `id`. -/
private partial def oldLocalsAt (ns : ValidatedNamespace) (id : LeanerIR.ExprId)
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
      | kind => (LeanerIR.Validation.expressionChildren kind).foldl
          (fun found child => oldLocalsAt ns child anchor within found) found

/-- The site of the specification block saving the state anchor `label`. -/
private def anchorSite? (namespaceId : NamespaceId) (ns : ValidatedNamespace)
    (root : LeanerIR.ExprId) (label : Nat) : Option Nat :=
  let saves (condition : LeanerIR.Condition) : Bool :=
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
private def overDefined (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
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
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (params : NRow) (row : NRow) (codecs types : Option Lean.Expr)
    (twins : Array SpecTypes.TwinInfo) (ownProof : Bool := true) :
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
                          { block with conditions := #[asserted] } locals localTypes codecs
                          types (declaration.locals.map (·.name)) oldLocals startState twins
                          (state := some state) (executable := executable)
                          (kind := asserted.kind) (labeledAnchors := labeled)
                return Contract.markCondition unit asserted (← overAnchors 0 #[])
              let conjunction ← clauses.foldrM (init := mkConst ``True)
                fun clause rest => mkAppM ``And #[clause, rest]
              mkLambdaFVars (bound ++ #[env, state]) conjunction
        mkLambdaFVars #[start, startState] (← withAnchors 0 #[])
    conditions := conditions.push (loopSite namespaceId.index site.index, condition, readsUnit)
  return conditions

/-- Whether a condition of a specification block marks the state or a
derivation instead of stating an assumption. -/
private def isMarker (ns : ValidatedNamespace) (condition : LeanerIR.Condition) : Bool :=
  (ns.expressions[condition.expression.index]?).any fun expression =>
    expression.kind matches .operation (.specification (.saveStateAnchor _)) _ _ _ ||
      expression.kind matches .operation (.specification (.foldsCaptureAnchor _)) _ _ _ ||
      expression.kind matches .operation (.specification .inlineCallSummary) _ _ _

/-- The in-body assumptions of a body: each block stating one, with the
conditions it assumes. -/
private def bodyAssumptions (ns : ValidatedNamespace) (root : LeanerIR.ExprId) :
    Array (LeanerIR.ExprId × LeanerIR.SpecBlock) :=
  (bodySpecifications ns root (· matches .assumption)).filterMap fun (id, block) =>
    let assumed := block.conditions.filter fun condition =>
      condition.kind matches .assumption && !isMarker ns condition
    if assumed.isEmpty then none else some (id, { block with conditions := assumed })

/-- Whether a function's body states an in-body assumption. -/
def statesAssumptions (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : Bool :=
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
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (params : NRow) (row : NRow) (codecs types : Option Lean.Expr)
    (twins : Array SpecTypes.TwinInfo) : TermElabM Lean.Expr := do
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
                  locals localTypes codecs types (declaration.locals.map (·.name)) oldLocals
                  startState twins (state := some state) (executable := some outer)
                  (kind := .assumption)
          let conjunction ← clauses.foldrM (init := mkConst ``True)
            fun clause rest => mkAppM ``And #[clause, rest]
          mkLambdaFVars #[env, state] conjunction
      let entry ← mkAppOptM ``Sigma.mk #[mkConst ``NRow, entryType, rowType, condition]
      table ← mkAppM ``ite #[← mkEq site (toExpr (loopSite namespaceId.index id.index)),
        ← mkAppM ``Option.some #[entry], table]
    mkLambdaFVars #[start, startState, site] table

/-- The expressions of a body, each once. -/
private def bodyExpressionIds (ns : ValidatedNamespace) (root : LeanerIR.ExprId) :
    Array LeanerIR.ExprId := Id.run do
  let mut found := #[]
  let mut pending := #[root]
  let mut visited : Std.HashSet Nat := {}
  while let some id := pending.back? do
    pending := pending.pop
    if visited.contains id.index then continue
    visited := visited.insert id.index
    let some expression := ns.expressions[id.index]? | continue
    found := found.push id
    pending := pending ++ LeanerIR.Validation.expressionChildren expression.kind
  return found

/-- The data invariant owed where a mutation of a local whose type carries
one ends: where a loan of the local dies, keyed by the loan, and after a
write into the local, keyed by the write (`Term.mutationEnd`). Each is over
the function's starting arguments and state and the current locals and
state; first over the executable unit where a data invariant states a
behavioral predicate, which the returned flag marks. -/
def mutationConditions (unit : ValidatedUnit) (outer : Lean.Expr) (handle : FunctionHandle)
    (view : CompileNamespace)
    (ns : ValidatedNamespace) (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (params : NRow) (row : NRow) (codecs types : Option Lean.Expr)
    (twins : Array SpecTypes.TwinInfo) : TermElabM (Array (Nat × Lean.Expr × Bool)) := do
  let .structured root := declaration.body | return #[]
  let namespaceId := handle.namespaceId
  let rowType ← quoteRow row
  let row := row.toList
  let carries (slot : LeanerIR.LocalId) : Bool := (row[slot.index]?).any view.carriesInvariant
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
  let block : LeanerIR.SpecBlock := { loc := ⟨0⟩ }
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
          fun locals => translateLoopInvariants unit namespaceId ns block locals localTypes codecs
            types (declaration.locals.map (·.name)) #[] startState twins #[slot]
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
    (ns : ValidatedNamespace) (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (params : NRow) (codecs types : Option Lean.Expr) (twins : Array SpecTypes.TwinInfo) :
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
          let encoded ← mkAppM ``NTy.encode #[← quoteNTy τ, value]
          let owed ← Contract.valueDataInvariants unit namespaceId ns codecs types twins ty
            encoded state executable
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
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (params : NRow) (row : NRow) (codecs types : Option Lean.Expr)
    (twins : Array SpecTypes.TwinInfo) : TermElabM (Array (Nat × Lean.Expr × Bool)) := do
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
    let some (LeanerIR.GenericArgument.typeArg resource) := instantiations[0]? | continue
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
        let owed ← Contract.memoryWriteInvariants unit namespaceId ns declaration codecs types
          twins written state before executable
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
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) (params : NRow)
    (codecs types : Option Lean.Expr) (twins : Array SpecTypes.TwinInfo) :
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
        let owed ← Contract.callWriteInvariants unit handle.namespaceId ns declaration codecs types
          twins written before after executable
        mkLambdaFVars #[start, startState, before, after] owed
    conditions := conditions.push (loopSite handle.namespaceId.index id.index, condition, readsUnit)
  return conditions

/-! ## Verification -/

/-- The declared field names of a constructor, or positions when unnamed. -/
private def fieldNames (unit : ValidatedUnit) (source : LeanerIR.StructHandle)
    (variant : Option String) (count : Nat) : List String :=
  let declared : Option (List String) := do
    let (targetNs, fields) ← LeanerIR.SemanticOperations.handleFields? unit source variant
    fields.toList.mapM fun (field : LeanerIR.FieldDecl) =>
      (targetNs.tables.names[field.name.index]?).map (·.name)
  match declared with
  | some names => if names.length == count then names else (List.range count).map toString
  | none => (List.range count).map toString

/-- The `rcases` pattern destructuring a native value of a type into its
scalar parts, named from `base`.  A variant value becomes one alternative
per variant; the empty tail of the variant sum closes its case. -/
private partial def carrierPattern (unit : ValidatedUnit) (base : String) : NTy → String
  | .tuple elements => rowPattern (elements.toList.zipIdx.map fun (σ, index) => (σ, s!"{base}_{index}"))
  | .struct source _ fields =>
      rowPattern (fields.toList.zip ((fieldNames unit source none fields.length).map (s!"{base}_{·}")))
  | .enum source _ names rows _ =>
      let alternatives := (names.zip rows.toList).map fun (name, fields) =>
        rowPattern (fields.toList.zip
          ((fieldNames unit source (some name) fields.length).map (s!"{base}_{name}_{·}")))
      "(" ++ " | ".intercalate (alternatives ++ ["(⟨⟩ : Empty)"]) ++ ")"
  | .ref referent => s!"⟨{carrierPattern unit base referent}, «{base}_final»⟩"
  -- A binder is written escaped, as a source name may be a Lean keyword.
  | _ => s!"«{base}»"
where
  rowPattern (parts : List (NTy × String)) : String :=
    if parts.isEmpty then "_"
    else "⟨" ++ ", ".intercalate (parts.map fun (σ, name) => carrierPattern unit name σ) ++ ", _⟩"

/-- The argument pattern naming a function's parameters. -/
private def argumentPattern (unit : ValidatedUnit)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) (params : NRow) :
    CommandElabM (TSyntax `rcasesPat) := do
  let names := declaration.signature.parameters.mapIdx fun index parameter =>
    if parameter.name.isEmpty then s!"argument{index}" else parameter.name
  if names.isEmpty then `(rcasesPat| _)
  else
    let parts := (params.toList.zip names.toList).map fun (τ, name) => carrierPattern unit name τ
    let text := "⟨" ++ ", ".intercalate parts ++ ", _⟩"
    match Parser.runParserCategory (← getEnv) `rcasesPat text with
    | .ok pattern => pure ⟨pattern⟩
    | .error message => throwError m!"internal: argument pattern `{text}`: {message}"

/-- Compare certificate types without treating holes as proof obligations. -/
def certificateTypesMatch (actual expected : Lean.Expr) : MetaM Bool := do
  let actual ← instantiateMVars actual
  let expected ← instantiateMVars expected
  if actual.hasMVar || actual.hasFVar || actual.hasLooseBVars ||
      expected.hasMVar || expected.hasFVar || expected.hasLooseBVars then
    return false
  isDefEq actual expected

/-- The statement a caller assumes of a native, rebuilt from its declaration:
its meaning satisfies its typed contract at every family, and at every
instantiation for a generic native. A non-generic native's types mention no
type parameter, so the statement is the same at every family; stated at each,
it serves a generic caller as well as the runtime family. -/
private def nativeHypothesisType (unit : ValidatedUnit) (twins : Array SpecTypes.TwinInfo)
    (stored : Contract.StoredInvariants) (executable requiresTable : Lean.Expr)
    (handle : FunctionHandle) : MetaM Lean.Expr := do
  let some nativeNs := unit.namespaces[handle.namespaceId.index]?
    | throwError "a native's namespace is out of range"
  let some declaration := nativeNs.functions[handle.functionId.index]?
    | throwError "a native is out of range"
  let some (params, result) := nativeSignature? unit handle.namespaceId declaration
    | throwError "a native has no native signature"
  -- A caller assumes what callers see: the interface; of a native without
  -- a specification, its prelude model.
  let nativeModel := nativeModelOf? unit handle nativeNs declaration
  let typed ← typedContractOf unit handle.namespaceId.index nativeNs
    (ContractView.interface.of declaration) params result twins nativeModel stored
  let view := ContractView.interface.of declaration
  let readsUnit := Contract.contractReadsUnit unit handle.namespaceId nativeNs view
  let readsRequires := Contract.contractReadsRequires unit handle.namespaceId view
  let unitExpr ← Contract.executableUnit executable
  -- The contract at a frame of the executable's unit: over the executable
  -- where it reads it, and its table last.
  let typedAt (skolems : Lean.Expr) (arguments : Array Lean.Expr) : Lean.Expr :=
    let leading := if readsUnit then #[unitExpr, executable, skolems] else #[unitExpr, skolems]
    let applied := mkAppN typed (leading ++ arguments)
    (if readsUnit && readsRequires then mkApp applied requiresTable else applied).headBeta
  let row ← quoteRow params
  let shape ← quoteShape result
  let meaningAt (skolems instantiation : Lean.Expr) :=
    mkAppN (mkConst ``propheticMeaning)
      #[unitExpr, executable, skolems, instantiation, toExpr handle, row, shape]
  withLocalDecl `Θ .instImplicit (mkApp (mkConst ``Skolems) unitExpr) fun skolems =>
    if isGeneric declaration then
      withLocalDeclD `typeInstantiation
          (toTypeExpr (Array (LeanerIR.TypeId × LeanerIR.TypeId))) fun instantiation => do
        let statement ← mkAppM ``LeanerIR.Proofs.Satisfies
          #[meaningAt skolems instantiation, typedAt skolems #[instantiation]]
        mkForallFVars #[skolems, instantiation] statement
    else do
      let statement ← mkAppM ``LeanerIR.Proofs.Satisfies
        #[meaningAt skolems (toExpr (#[] : Array (LeanerIR.TypeId × LeanerIR.TypeId))),
          typedAt skolems #[]]
      mkForallFVars #[skolems] statement

/-- The axiom dependencies of this file's own constants, kept across the
audits of one file: each audit walks only what an earlier audit has not. An
environment extension, so that a rolled-back verification forgets its
entries with its declarations. -/
private initialize localAxiomsExt : EnvExtension (NameMap (Array Name)) ←
  registerEnvExtension (pure {})

/-- The axioms a constant depends on, and whether the walk met a constant
still being walked. An imported constant's are those its module recorded; a
constant of this file is walked once and kept, unless it lies on a cycle
(an inductive and its constructors), whose members are walked again. -/
private partial def axiomsOf (active : NameSet) (name : Name) :
    CommandElabM (NameSet × Bool) := do
  let env ← getEnv
  if (env.getModuleIdxFor? name).isSome then
    return ((← collectAxioms name).foldl (·.insert ·) {}, false)
  if let some known := (localAxiomsExt.getState env).find? name then
    return (known.foldl (·.insert ·) {}, false)
  if active.contains name then return ({}, true)
  let some info := env.find? name | return ({}, false)
  let active := active.insert name
  let mut axioms : NameSet := if info matches .axiomInfo _ then ({} : NameSet).insert name else {}
  let mut cyclic := false
  let constructors := match info with
    | .inductInfo declaration => declaration.ctors.toArray
    | _ => #[]
  let used := info.type.getUsedConstants ++
    ((info.value? (allowOpaque := true)).map (·.getUsedConstants)).getD #[] ++ constructors
  for dependency in used do
    let (found, onCycle) ← axiomsOf active dependency
    axioms := found.foldl (·.insert ·) axioms
    cyclic := cyclic || onCycle
  unless cyclic do
    modifyEnv fun env => localAxiomsExt.modifyState env (·.insert name axioms.toArray)
  return (axioms, cyclic)

/-- Whether an axiom is a `bv_decide` certificate: that its checker of an
UNSAT proof, evaluated natively, returns `true`. The checker is evaluated
again, as `bv_decide` evaluated it: a name and a shape alone admit nothing. -/
private def isBitVectorCertificate (name : Name) : CommandElabM Bool := do
  unless name.components.contains `_native && name.components.contains `bv_decide do
    return false
  let some (.axiomInfo info) := (← getEnv).find? name | return false
  let_expr Eq _ checked expected := info.type | return false
  unless checked.isAppOf ``Std.Tactic.BVDecide.Reflect.verifyBVExpr &&
      expected.isConstOf ``Bool.true && info.levelParams.isEmpty do
    return false
  liftTermElabM <| withoutModifyingEnv do
    try
      match ← Lean.Meta.nativeEqTrue `leaner_audit checked with
      | .success _ => pure true
      | .notTrue => pure false
    catch _ => pure false

/-- The no-fallback audit of one verified function, including the transitive
axiom closure. The agreement axiom is the sole project-specific exception,
explicitly deferred to D4 in `designs/denotation.md`; a function selecting
`pragma bv` may also rest on `bv_decide`'s natively checked certificates. -/
def requireNativeArtifacts (base : Name) (bitVectors : Bool) : CommandElabM Unit := do
  let function := base.getString!
  let artifacts := base.replacePrefix (← getCurrNamespace) .anonymous
  let env ← getEnv
  let some theoremInfo := env.find? (base ++ `typedVerified)
    | throwError m!"`{function}` is not verified"
  unless theoremInfo.type.getUsedConstants.contains ``Term.denote do
    throwError m!"`{function}` was verified by a retired route"
  unless env.contains (artifacts ++ `compiled_eq) do
    throwError m!"missing compilation certificate for `{function}`"
  let roots := #[base ++ `typedVerified, artifacts ++ `compiled_eq, artifacts ++ `compiled]
  let mut certificates : NameSet := {}
  for root in roots ++ #[base ++ `verified] do
    for axiomName in (← axiomsOf {} root).1.toArray do
      if bitVectors && certificates.contains axiomName then continue
      if bitVectors && (← isBitVectorCertificate axiomName) then
        certificates := certificates.insert axiomName
        continue
      unless #[``propext, ``Classical.choice, ``Quot.sound,
          ``compileFunction_agrees, ``compileFunction_least_cycle].contains axiomName do
        throwError m!"artifact `{root}` depends on unapproved axiom `{axiomName}`"
  let mut pending := roots
  let mut visited : NameSet := {}
  while let some name := pending.back? do
    pending := pending.pop
    if visited.contains name then continue
    visited := visited.insert name
    let some declaration := env.find? name
      | throwError m!"missing artifact `{name}`"
    let constants := declaration.type.getUsedConstants ++
      ((declaration.value? (allowOpaque := true)).map (·.getUsedConstants)).getD #[]
    for dependency in constants do
      -- Rule 1 of the design: a verified function reasons over values,
      -- never over frames or loan bookkeeping.
      if dependency == ``sorryAx || dependency == ``LeanerIR.RuntimeFrame ||
          dependency == ``LeanerIR.SemanticOperations.FreshStorageLoanIds ||
          dependency == ``LeanerIR.SemanticOperations.LoanDiscipline ||
          dependency == ``LeanerIR.SemanticOperations.storageLoanTargetIn? ||
          dependency == ``LeanerIR.SemanticOperations.removeStorageLoan then
        throwError m!"artifact `{name}` retains forbidden dependency `{dependency}`"
      if base.isPrefixOf dependency || artifacts.isPrefixOf dependency then
        pending := pending.push dependency
  unless completedDenotations.isTagged env (base ++ `typedVerified) do
    throwError m!"`{function}` has no completed denotation verification"
  -- The completion tag is only bookkeeping: source metaprograms can mutate
  -- extensions. Require an actual theorem for this registered unit, function
  -- and authored contract, reconstructed without trusting cached definitions.
  let some publicInfo := env.find? (base ++ `verified)
    | throwError m!"`{function}` has no public verification certificate"
  let namespaceName := artifacts.getPrefix
  let some unit := LeanerLang.registeredUnit? env namespaceName
    | throwError m!"unknown Leaner namespace `{namespaceName}`"
  let some (namespaceIndex, ns, functionIndex, declaration) := findFunction? unit function
    | throwError m!"unknown function `{function}` in `{namespaceName}`"
  let some (params, result) := nativeSignature? unit ⟨namespaceIndex⟩ declaration
    | throwError m!"`{function}` has no native signature"
  let segments := namespaceName.components.toArray.map (·.getString!)
  let twins ← SpecTypes.ensureSpecTypes segments unit
  let stored ← ensureStoredInvariant segments unit twins
  let valid ← liftTermElabM do
    let typed ← typedContractOf unit namespaceIndex ns (ContractView.implementation.of declaration)
      params result twins (stored := stored)
    let readsUnit := Contract.contractReadsUnit unit ⟨namespaceIndex⟩ ns
      (ContractView.implementation.of declaration)
    let readsRequires :=
      Contract.contractReadsRequires unit ⟨namespaceIndex⟩
        (ContractView.implementation.of declaration)
    let unitExpr := toExpr unit
    let requiresTable := mkApp (mkConst (requiresTableName segments)) unitExpr
    let publicContract ← publicContractOf params result (isGeneric declaration) typed readsUnit
      readsRequires
    let handle : FunctionHandle := ⟨⟨namespaceIndex⟩, ⟨functionIndex⟩⟩
    let expected ← withLocalDecl `registry .implicit
        (mkConst ``LeanerIR.Validation.SemanticsRegistry) fun registry =>
      withLocalDecl `executable .implicit (Contract.executableType unitExpr) fun executable => do
        let prepared ← mkAppM ``LeanerIR.Validation.prepareExecution #[registry, unitExpr]
        let errorType := (← inferType prepared).getAppArgs[0]!
        let success ← mkAppOptM ``Except.ok #[some errorType,
          some (Contract.executableType unitExpr), some executable]
        let preparation ← mkEq prepared success
        let contractAt (arguments : Array Lean.Expr) :=
          (mkAppN publicContract (#[unitExpr, executable] ++ arguments ++
            (if readsRequires then #[requiresTable] else #[]))).headBeta
        -- A generic function's theorem holds at every instantiation a call
        -- gives it, coherent with the frame its type arguments induce.
        let conclusion ← if isGeneric declaration then
            let instantiationType ← mkAppM ``Array
              #[← mkAppM ``Prod #[mkConst ``LeanerIR.TypeId, mkConst ``LeanerIR.TypeId]]
            let arity := (declaration.signature.generics.filter (·.kind == .typeArg)).size
            withLocalDeclD `θ (mkConst ``TypeArgs) fun θ => do
            withLocalDeclD `free (← mkEq (mkApp (mkConst ``NRow.refFree)
                (← mkAppM ``Subtype.val #[θ])) (mkConst ``Bool.true)) fun free =>
            withLocalDeclD `typeInstantiation instantiationType fun instantiation =>
            withLocalDeclD `frame (mkAppN (mkConst ``FrameOf)
                #[unitExpr, toExpr handle, toExpr arity, instantiation]) fun frame => do
            let induced := mkApp3 (mkConst ``Skolems.instantiate) unitExpr θ
              (mkApp (mkConst ``Skolems.runtime) unitExpr)
            withLocalDeclD `coherent (mkAppN (mkConst ``Coherent)
                #[unitExpr, induced, toExpr handle, instantiation]) fun coherent => do
            mkForallFVars #[θ, free, instantiation, frame, coherent]
              (← mkAppM ``LeanerIR.Proofs.SatisfiesFunctionAt
                #[executable, toExpr handle, instantiation, contractAt #[θ, instantiation]])
          else
            mkAppM ``LeanerIR.Proofs.SatisfiesFunction #[executable, toExpr handle, contractAt #[]]
        -- The natives the theorems assume, in their binding order.
        let natives := ((nativeDependencies.getState env).find? base).getD #[]
        let hypotheses ← natives.mapM fun (nativeHandle, _) =>
          nativeHypothesisType unit twins stored executable requiresTable nativeHandle
        -- Then that the in-body assumptions of the functions it relies on hold.
        let assumptions := ((assumptionDependencies.getState env).find? base).getD #[]
        let instantiationType ← mkAppM ``Array
          #[← mkAppM ``Prod #[mkConst ``LeanerIR.TypeId, mkConst ``LeanerIR.TypeId]]
        let hypotheses := hypotheses ++ (← assumptions.mapM fun (_, name) => do
          let artifacts := Name.str namespaceName name
          withLocalDecl `Θ .implicit (mkApp (mkConst ``Skolems) unitExpr) fun family =>
          withLocalDecl `typeInstantiation .implicit instantiationType fun instantiation => do
            let reading (suffix : Name) :=
              mkAppN (mkConst (artifacts ++ suffix))
                #[executable, family, instantiation]
            mkForallFVars #[family, instantiation] (← mkAppM ``LeanerIR.Proofs.AssumptionsHold
              #[reading `plainMeaning, reading `assumedMeaning]))
        -- Then the assumed steps of the lemmas it applies.
        let trusted := ((lemmaTrustDependencies.getState env).find? base).getD #[]
        let hypotheses := hypotheses ++ trusted.map mkConst
        -- The public theorem takes that the unit's runs keep memory typed.
        let hypotheses := #[← mkAppM ``LeanerIR.Proofs.Denote.GlobalsPreserved #[executable]] ++
          hypotheses
        -- A theorem assuming typing takes what the natives preserve, last,
        -- and one resolving `result_of` that the unit's runs end.
        let assumesTermination := Contract.assumesTermination unit ⟨namespaceIndex⟩ ns declaration
        let hypotheses ← if Contract.assumesTyping unit ⟨namespaceIndex⟩ ns declaration ||
            assumesTermination then
            pure (hypotheses ++ #[← mkAppM ``LeanerIR.NativesTyped #[executable],
              ← mkAppM ``LeanerIR.NativesShift #[executable]])
          else pure hypotheses
        let hypotheses ← if assumesTermination then
            pure (hypotheses.push (← mkAppM ``LeanerIR.Proofs.Terminating #[executable]))
          else pure hypotheses
        let rec assume (index : Nat) (bound : Array Lean.Expr) : MetaM Lean.Expr := do
          if h : index < hypotheses.size then
            withLocalDeclD `native hypotheses[index] fun hypothesis =>
              assume (index + 1) (bound.push hypothesis)
          else
            mkForallFVars #[registry, executable]
              (← mkArrow preparation (← mkForallFVars bound conclusion))
        assume 0 #[]
    certificateTypesMatch publicInfo.type expected
  unless valid do
    throwError m!"`{function}` has an invalid public verification certificate"

/-- Publish the compilation view of a namespace as a literal, with the kernel
certificate that it is the view the unit's namespace has: functions'
certificates then read its arenas instead of rebuilding them. -/
private def publishCompileView (segments : Array String) (unit : ValidatedUnit)
    (namespaceId : NamespaceId) : TermElabM Name := do
  let viewName := Name.str (semanticsName segments) s!"compileView_{namespaceId.index}"
  let viewEqName := viewName ++ `eq
  if (← getEnv).contains viewEqName then return viewEqName
  let some view := compileNamespaceAt? unit namespaceId
    | throwError "namespace {namespaceId.index} is out of range"
  let semantics := mkConst (unitName segments)
  let source ← mkAppM ``Option.get!
    #[← mkAppM ``getElem? #[mkApp (mkConst ``ValidatedUnit.namespaces) semantics,
      toExpr namespaceId.index]]
  let value := mkAppN (mkConst ``CompileNamespace.mk) #[source, toExpr view.namespaceId,
    toExpr view.expressions, toExpr view.places, toExpr view.patterns, toExpr view.types,
    toExpr view.deaths, toExpr view.shared, toExpr view.invariants, toExpr view.typeFuel,
    toExpr view.fuel, toExpr view.placeFuel]
  addDecl (.defnDecl {
    name := viewName, levelParams := []
    type := mkConst ``CompileNamespace
    value, hints := .abbrev, safety := .safe })
  let some' := mkApp (mkConst ``Option.some [Level.zero]) (mkConst ``CompileNamespace)
  addDecl (.thmDecl {
    name := viewEqName, levelParams := []
    type := ← mkEq (mkAppN (mkConst ``compileNamespaceAt?) #[semantics, toExpr namespaceId])
      (mkApp some' (mkConst viewName))
    value := ← mkEqRefl (mkApp some' (mkConst viewName)) })
  return viewEqName

/-- Publish the compiled function of a target: its rows, shape, body,
mutable parameters, the function itself, and the kernel certificate of its
compilation, read through its namespace's view.  Nothing is republished. -/
private def publishCompiled (segments : Array String) (unit : ValidatedUnit)
    (function : String) (handle : FunctionHandle) (compiled : Function unit) : TermElabM Unit := do
  let artifacts := Name.str (pathName segments) function
  if (← getEnv).contains (artifacts ++ `compiled_eq) then return
  let semantics := mkConst (unitName segments)
  addAbbrev (artifacts ++ `row) (← quoteRow (compiled.params ++ compiled.locals))
  -- The signature a caller assuming the function's interface published.
  unless (← getEnv).contains (artifacts ++ `params) do
    addAbbrev (artifacts ++ `params) (← quoteRow compiled.params)
  addAbbrev (artifacts ++ `locals) (← quoteRow compiled.locals)
  unless (← getEnv).contains (artifacts ++ `shape) do
    addAbbrev (artifacts ++ `shape) (← quoteShape compiled.result)
  addAbbrev (artifacts ++ `body) (← quoteTerm semantics compiled.body)
  addAbbrev (artifacts ++ `mutables) (← quoteMutables compiled.mutables)
  addAbbrev (artifacts ++ `compiled)
    (← quoteFunction semantics compiled (mkConst (artifacts ++ `body))
      (mkConst (artifacts ++ `mutables)))
  let viewEq ← publishCompileView segments unit handle.namespaceId
  let lhs := mkAppN (mkConst ``compileFunction) #[semantics, toExpr handle]
  let rhs := mkAppN (mkConst ``Except.ok [Level.zero, Level.zero])
    #[mkConst ``String, mkApp (mkConst ``Function) semantics, mkConst (artifacts ++ `compiled)]
  let view := (Name.str (semanticsName segments) s!"compileView_{handle.namespaceId.index}")
  let throughView ← mkEq
    (mkAppN (mkConst ``compileFunctionIn) #[semantics, mkConst view, toExpr handle]) rhs
  addDecl (.thmDecl {
    name := artifacts ++ `compiled_eq
    levelParams := []
    type := ← mkEq lhs rhs
    value := ← mkEqTrans
      (mkAppN (mkConst ``compileFunction_of_view)
        #[semantics, toExpr handle, mkConst view, mkConst viewEq])
      (← mkExpectedTypeHint (← mkEqRefl rhs) throughView) })

/-- The signature artifacts of a native: it has no compiled body, so its
parameter row and result shape come from its declaration. -/
private def publishNativeSignature (segments : Array String) (function : String)
    (params : NRow) (result : ResultShape) : TermElabM Unit := do
  let artifacts := Name.str (pathName segments) function
  unless (← getEnv).contains (artifacts ++ `params) do
    addAbbrev (artifacts ++ `params) (← quoteRow params)
  unless (← getEnv).contains (artifacts ++ `shape) do
    addAbbrev (artifacts ++ `shape) (← quoteShape result)

/-- A function handle as a literal term. -/
private def handleSyntax (handle : FunctionHandle) : CommandElabM Term :=
  `(term| (⟨⟨$(Syntax.mkNatLit handle.namespaceId.index)⟩,
    ⟨$(Syntax.mkNatLit handle.functionId.index)⟩⟩ : LeanerIR.FunctionHandle))

/-- The functions a unit's closures target. -/
private def closureTargets (unit : ValidatedUnit) : Array FunctionHandle := Id.run do
  let mut targets := #[]
  for ns in unit.namespaces, index in [0:unit.namespaces.size] do
    for expression in ns.expressions do
      if let .operation (.call (.closure reference _)) .. := expression.kind then
        if let some target := LeanerIR.SemanticOperations.resolveFunction? unit ⟨index⟩ reference then
          unless targets.contains target do targets := targets.push target
  return targets

/-- The functions the closures a function's contract names target: what the
contract's behavioral predicates may read of their targets' theorems. -/
private def contractClosureTargets (unit : ValidatedUnit) (namespaceId : NamespaceId)
    (ns : ValidatedNamespace) (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) :
    Array FunctionHandle := Id.run do
  let mut targets := #[]
  let mut pending := declaration.contract.conditions.map (·.expression)
  let mut visited : Array Nat := #[]
  while let some id := pending.back? do
    pending := pending.pop
    if visited.contains id.index then continue
    visited := visited.push id.index
    let some expression := ns.expressions[id.index]? | continue
    if let .operation (.call (.closure reference _)) .. := expression.kind then
      if let some target := LeanerIR.SemanticOperations.resolveFunction? unit namespaceId reference then
        unless targets.contains target do targets := targets.push target
    pending := pending ++ LeanerIR.Validation.expressionChildren expression.kind
  return targets

/-- Define a module's table of declared preconditions, once: each target of
its closures maps to its `requires` over runtime arguments, every other
function to `True`; an equation per target, a `lir_denote_norm` lemma,
reads a target's entry. A precondition stating a behavioral predicate would
read the table itself and is rejected. -/
def ensureRequiresTable (segments : Array String) (unit : ValidatedUnit) : CommandElabM Name := do
  let name := requiresTableName segments
  if (← getEnv).contains name then return name
  let twins ← SpecTypes.ensureSpecTypes segments unit
  liftTermElabM do
    let mut entries : Array (FunctionHandle × Lean.Expr) := #[]
    for target in closureTargets unit do
      let some ns := unit.namespaces[target.namespaceId.index]? | continue
      let some declaration := ns.functions[target.functionId.index]? | continue
      if Contract.preconditionReadsUnit unit target.namespaceId declaration then
        throwError m!"`requires_of` cannot read `{functionKey unit target}`: its precondition \
          states a behavioral predicate"
      entries := entries.push (target,
        ← Contract.buildDeclaredRequires unit target.namespaceId ns declaration twins)
    let tableType ← withLocalDeclD `unit (mkConst ``ValidatedUnit) fun unitExpr =>
      mkForallFVars #[unitExpr] (mkApp (mkConst ``LeanerIR.Proofs.RequiresTable) unitExpr)
    let values ← mkAppM ``Array #[mkConst ``RuntimeValue]
    let value ← withLocalDeclD `unit (mkConst ``ValidatedUnit) fun unitExpr =>
      withLocalDeclD `handle (mkConst ``FunctionHandle) fun handle =>
      withLocalDeclD `arguments values fun arguments =>
        withLocalDeclD `state (mkApp (mkConst ``LeanerIR.Proofs.Denote.Memory) unitExpr) fun state => do
          let mut body := mkConst ``True
          for (target, requires) in entries.reverse do
            body ← mkAppM ``ite #[← mkEq handle (toExpr target),
              requires.beta #[unitExpr, arguments, state], body]
          mkLambdaFVars #[unitExpr, handle, arguments, state] body
    addDecl (.defnDecl {
      name, levelParams := [], type := tableType, value, hints := .regular 0, safety := .safe })
    for (target, requires) in entries, index in [0:entries.size] do
      let equation := name ++ Name.mkSimple s!"eq_{index}"
      let statement ← withLocalDeclD `unit (mkConst ``ValidatedUnit) fun unitExpr =>
        withLocalDeclD `arguments values fun arguments =>
        withLocalDeclD `state (mkApp (mkConst ``LeanerIR.Proofs.Denote.Memory) unitExpr) fun state => do
          mkForallFVars #[unitExpr, arguments, state] (← mkEq
            (mkAppN (mkConst name) #[unitExpr, toExpr target, arguments, state])
            (requires.beta #[unitExpr, arguments, state]))
      let proof ← forallTelescope statement fun binders equality => do
        mkLambdaFVars binders (← mkEqRefl equality.appArg!)
      addDecl (.thmDecl { name := equation, levelParams := [], type := statement, value := proof })
      let attr ← `(attr| lir_denote_norm)
      Lean.Elab.Term.applyAttributes equation
        #[{ name := `lir_denote_norm, stx := attr, kind := .global }]
  return name

/-- Whether a contract definition takes the executable unit (its contract
states a behavioral predicate) and the table of declared preconditions
(it states `requires_of`), as its last parameters. -/
def contractTakes (env : Environment) (name : Name) : Bool × Bool :=
  match env.find? name with
  | some info => takes info.type
  | none => (false, false)
where
  takes : Lean.Expr → Bool × Bool
    | .forallE _ domain body _ =>
        let (unit, table) := takes body
        (unit || domain.isAppOf ``LeanerIR.Validation.ExecutableUnit,
          table || domain.isAppOf ``LeanerIR.Proofs.RequiresTable)
    | _ => (false, false)

/-- Prove once per unit, by evaluation, that its resource types correspond
to their keys' types (`ResourcesTyped`): every memory's runtime encoding is
then typed. -/
def ensureResourcesTyped (segments : Array String) (unit : ValidatedUnit) : CommandElabM Name := do
  let name := resourcesTypedName segments
  if (← getEnv).contains name then return name
  let unitDefinition ← ensureUnitDefinition segments unit
  elabCommand (← `(command|
    theorem $(rootIdent name) : LeanerIR.Proofs.ResourcesTyped $(rootIdent unitDefinition) :=
      LeanerIR.Proofs.ResourcesTyped.ofCheck (by decide +kernel)))
  return name

/-- Prove once per unit, by evaluation, that its native and semantic readings
agree (`TypesAgree`), where they do: a function value is then typed by its
carrier. -/
def ensureTypesAgree (segments : Array String) (unit : ValidatedUnit) :
    CommandElabM (Option Name) := do
  let name := typesAgreeName segments
  if (← getEnv).contains name then return some name
  unless LeanerIR.Proofs.typesAgreeCheck unit do return none
  let unitDefinition ← ensureUnitDefinition segments unit
  elabCommand (← `(command|
    theorem $(rootIdent name) : LeanerIR.Proofs.TypesAgree $(rootIdent unitDefinition) :=
      LeanerIR.Proofs.TypesAgree.ofCheck (by decide +kernel)))
  return some name

/-- The module's unit, the constant a theorem's frames and memory are at. -/
def unitTermOf (segments : Array String) (unit : ValidatedUnit) : CommandElabM Term :=
  return rootIdent (← ensureUnitDefinition segments unit)

/-- A contract term at the module's unit, the executable unit, and the
module's table of declared preconditions, where the contract takes them. -/
def atUnit (segments : Array String) (unit : ValidatedUnit) (name : Name) (contract : Term)
    (frame? : Option Term := none) (args : Array Term := #[]) : CommandElabM Term := do
  let (readsUnit, readsTable) := contractTakes (← getEnv) name
  let unitTerm ← unitTermOf segments unit
  let table : Array Term ← if readsTable then
      pure #[← `(term| ($(rootIdent (← ensureRequiresTable segments unit)) $unitTerm))]
    else pure #[]
  match frame? with
  | some frame =>
      let leading : Array Term ← if readsUnit then pure #[unitTerm, ← `(term| executable)]
        else pure #[unitTerm]
      `(term| (@$contract $leading* $frame $args* $table*))
  | none =>
      if readsUnit then `(term| ($contract executable $args* $table*))
      else if args.isEmpty then pure contract
      else `(term| ($contract $args*))

/-- The hypothesis a caller's theorems take for a native, or for a function
seen through its interface: the statement a verified callee's theorem
makes, that its meaning satisfies its contract, at every family; a generic
one's at every instantiation as well. -/
private def nativeBinder (segments : Array String) (unit : ValidatedUnit)
    (entry : FunctionHandle × String) (generic : Bool) (interface : Bool) :
    CommandElabM (TSyntax ``Lean.Parser.Term.bracketedBinder) := do
  let (handle, name) := entry
  let unitTerm ← unitTermOf segments unit
  let artifacts := Name.str (pathName segments) name
  let handleTerm ← handleSyntax handle
  let params := rootIdent (artifacts ++ `params)
  let shape := rootIdent (artifacts ++ `shape)
  let contractName := if interface then typedInterfaceContractName segments name
    else typedContractName segments name
  let contract := rootIdent contractName
  let binder := mkIdent (Name.mkSimple s!"native_{name}")
  if generic then
    let atFamily ← atUnit segments unit contractName contract (some (← `(term| Θ)))
      #[← `(term| typeInstantiation)]
    `(bracketedBinder| ($binder : ∀ {Θ : LeanerIR.Proofs.Denote.Skolems $unitTerm}
        {typeInstantiation : Array (LeanerIR.TypeId × LeanerIR.TypeId)},
        LeanerIR.Proofs.Satisfies
          (LeanerIR.Proofs.Denote.propheticMeaning executable typeInstantiation $handleTerm
            $params $shape)
          $atFamily))
  else
    let atFamily ← atUnit segments unit contractName contract (some (← `(term| Θ)))
    `(bracketedBinder| ($binder : ∀ {Θ : LeanerIR.Proofs.Denote.Skolems $unitTerm},
        LeanerIR.Proofs.Satisfies
          (@LeanerIR.Proofs.Denote.propheticMeaning _ executable Θ #[] $handleTerm $params $shape)
          $atFamily))

/-- The standard-library modules whose intrinsic functions the Move Prover's
prelude implements. -/
private def preludeIntrinsicModules : List String :=
  ["vector", "event", "aggregator", "aggregator_v2"]

/-- Whether a function is intrinsic, as the Move Prover reads the pragma: it
declares `pragma intrinsic` and has a meaning other than its body (it is
native, a map role, or in a module the prelude implements), or it is opaque,
so that callers rely on its contract by the author's choice. Elsewhere the
pragma leaves the body its meaning, verified as any other. -/
private def isIntrinsic (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : Bool :=
  let declared := declaration.pragmas.any fun
    | .assign "intrinsic" (.constant (.bool false)) _ => false
    | .assign "intrinsic" _ _ => true
    | _ => false
  let prelude := match (unit.tables.namespaces[namespaceId.index]?).map (·.segments) with
    | some #["0x1", name] => preludeIntrinsicModules.contains name
    | _ => false
  declared && (declaration.body == .absent || prelude || isOpaque declaration ||
    (Contract.mapRoleOf? unit namespaceId ns declaration).isSome)

/-- Whether a function's specification or its module sets
`pragma verify = false`, or the function is intrinsic: the function's
pragmas merge both. -/
private def automaticVerificationDisabled (unit : ValidatedUnit)
    (namespaceId : LeanerIR.NamespaceId) (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : Bool :=
  isIntrinsic unit namespaceId ns declaration || declaration.pragmas.any fun
    | .assign "verify" (.constant (.bool false)) _ => true
    | _ => false

/-- A member of a cycle of calls standing for the calls to it, and its
contract of them. -/
private def cycleSelf (name : String) : Ident := mkIdent (Name.mkSimple s!"self_{name}")
private def cycleSelfVerified (name : String) : Ident :=
  mkIdent (Name.mkSimple s!"selfVerified_{name}")

/-- The position of the `index`th member of a cycle. -/
private def cycleIndexSyntax : Nat → CommandElabM Term
  | 0 => `(term| LeanerIR.Proofs.Denote.CycleIndex.here)
  | index + 1 => do `(term| LeanerIR.Proofs.Denote.CycleIndex.there $(← cycleIndexSyntax index))

/-- The meanings of a cycle's members at every slot. -/
private def cycleSelves : Ident := mkIdent `leanerSelves

/-- The members of a cycle, each a handle with its compiled function. -/
private def cycleMembersSyntax (segments : Array String) (cycle : Array (FunctionHandle × String)) :
    CommandElabM Term := do
  let pairs ← cycle.mapM fun (member, name) => do
    `(term| ($(← handleSyntax member), $(rootIdent (compiledName segments name))))
  `(term| [$pairs,*])

/-- The declaration a handle names. -/
private def declarationOf? (unit : ValidatedUnit) (handle : FunctionHandle) :
    Option (LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) := do
  let ns ← unit.namespaces[handle.namespaceId.index]?
  ns.functions[handle.functionId.index]?

/-- The hypothesis a native assumes: its contract at every family and
instantiation when it is generic, at the runtime family otherwise. -/
private def nativeBinderOf (unit : ValidatedUnit) (segments : Array String)
    (entry : FunctionHandle × String) :
    CommandElabM (TSyntax ``Lean.Parser.Term.bracketedBinder) :=
  let declaration? := declarationOf? unit entry.1
  nativeBinder segments unit entry (declaration?.any isGeneric)
    (declaration?.any fun declaration => declaration.body != .absent)

/-- The hypothesis that a function's in-body assumptions hold
(`AssumptionsHold`), at every family and instantiation. -/
private def assumptionBinderOf (segments : Array String) (unit : ValidatedUnit)
    (entry : FunctionHandle × String) : CommandElabM (TSyntax ``Lean.Parser.Term.bracketedBinder) := do
  let unitTerm ← unitTermOf segments unit
  let artifacts := Name.str (pathName segments) entry.2
  let binder := mkIdent (Name.mkSimple s!"assumptions_{entry.2}")
  `(bracketedBinder| ($binder : ∀ {Θ : LeanerIR.Proofs.Denote.Skolems $unitTerm}
      {typeInstantiation : Array (LeanerIR.TypeId × LeanerIR.TypeId)},
      LeanerIR.Proofs.AssumptionsHold
        (@$(rootIdent (artifacts ++ `plainMeaning)) executable Θ typeInstantiation)
        (@$(rootIdent (artifacts ++ `assumedMeaning)) executable Θ typeInstantiation)))

/-- The arguments passing the hypotheses `assumptionBinderOf` binds. -/
private def assumptionArgs (entries : Array (FunctionHandle × String)) : Array Term :=
  entries.map fun entry => ⟨mkIdent (Name.mkSimple s!"assumptions_{entry.2}")⟩

/-- The binder of a trusted step's hypothesis. -/
def trustBinderName (name : Name) : Name :=
  Name.mkSimple s!"trusted_{name.toString (escape := false) |>.replace "." "_"}"

/-- The hypothesis that an assumed step of a lemma holds. -/
private def trustBinderOf (name : Name) :
    CommandElabM (TSyntax ``Lean.Parser.Term.bracketedBinder) := do
  `(bracketedBinder| ($(mkIdent (trustBinderName name)) : $(rootIdent name)))

/-- The arguments passing the hypotheses `trustBinderOf` binds. -/
private def trustArgs (names : Array Name) : Array Term :=
  names.map fun name => ⟨mkIdent (trustBinderName name)⟩

/-- A function reached again along calls that are inlined, if any: such a
cycle has no finite inlining. -/
private partial def inliningCycle? (edges : Array (FunctionHandle × Array FunctionHandle))
    (path : Array FunctionHandle) (node : FunctionHandle) : Option FunctionHandle :=
  if path.contains node then some node
  else
    let next := ((edges.find? (·.1 == node)).map (·.2)).getD #[]
    next.findSome? (inliningCycle? edges (path.push node))

/-- A kernel-decided theorem: the statement's `Decidable` instance evaluates
to `true`. -/
private def decidedByKernel (declName : Name) (statement : Term) : TermElabM Unit := do
  let statement ← Lean.Elab.Term.elabType statement
  Lean.Elab.Term.synthesizeSyntheticMVarsNoPostponing
  let statement ← instantiateMVars statement
  let decision ← synthInstance (mkApp (mkConst ``Decidable) statement)
  addDecl (.thmDecl {
    name := declName, levelParams := []
    type := statement
    value := mkApp3 (mkConst ``of_decide_eq_true) statement decision
      (mkApp2 (mkConst ``Eq.refl [1]) (mkConst ``Bool) (mkConst ``Bool.true)) })

/-- A kernel-checked equation, proved by `Eq.refl` of its right side. -/
private def reflexiveTheorem (declName : Name) (statement : Term) : TermElabM Unit := do
  let statement ← Lean.Elab.Term.elabType statement
  Lean.Elab.Term.synthesizeSyntheticMVarsNoPostponing
  let statement ← instantiateMVars statement
  addDecl (.thmDecl {
    name := declName, levelParams := []
    type := statement
    value := ← mkEqRefl statement.appArg! })

/-- A reducible definition of a literal value. -/
private def literalDefinition (declName : Name) (type value : Lean.Expr) : TermElabM Unit :=
  addDecl (.defnDecl {
    name := declName, levelParams := [], type, value, hints := .abbrev, safety := .safe })

/-! ## Lemmas

A lemma becomes a theorem over its statement's definitions
(`Contract.ensureLemmaDefinitions`): its parameters, then its premise, give
its conclusion. Its proof proves each step, asserting what the step owes and
assuming what it gives, then the conclusion. A recursion group is one
`mutual` block whose theorems recurse on the lemmas' measure.
-/

/-- The lemma instances an expression states. -/
private partial def lemmaInstancesOf (ns : ValidatedNamespace) (id : LeanerIR.ExprId) :
    Array LeanerIR.QualifiedRef := Id.run do
  let mut work := [id]
  let mut found := #[]
  for _ in [0:ns.expressions.size + 1] do
    match work with
    | [] => break
    | current :: rest =>
        work := rest
        let some expression := ns.expressions[current.index]? | continue
        if let .operation (.specification (.lemma reference _)) _ _ _ := expression.kind then
          unless found.contains reference do found := found.push reference
        work := work ++ (LeanerIR.Validation.expressionChildren expression.kind).toList
  return found

/-- The lemmas a lemma's proof applies. -/
private def appliedLemmas (unit : ValidatedUnit) (reference : LeanerIR.QualifiedRef) :
    Array LeanerIR.QualifiedRef :=
  match Contract.lemmaOf? unit reference with
  | some (ns, declaration) => declaration.proof.foldl (init := #[]) fun found condition =>
      (lemmaInstancesOf ns condition.expression).foldl (init := found) fun found reference =>
        if found.contains reference then found else found.push reference
  | none => #[]

/-- The lemmas a function's body applies. -/
private def functionLemmas (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) :
    Array LeanerIR.QualifiedRef :=
  match declaration.body with
  | .structured root => (bodySpecifications ns root (· matches .apply)).foldl (init := #[])
      fun found (_, block) => block.conditions.foldl (init := found) fun found condition =>
        if condition.kind != .apply then found
        else (lemmaInstancesOf ns condition.expression).foldl (init := found) fun found reference =>
          if found.contains reference then found else found.push reference
  | .absent => #[]

/-- The lemmas a lemma reaches through the lemmas its proof applies. -/
private def reachableLemmas (unit : ValidatedUnit) (start : LeanerIR.QualifiedRef) :
    Array LeanerIR.QualifiedRef := Id.run do
  let mut visited : Array LeanerIR.QualifiedRef := #[]
  let mut work := (appliedLemmas unit start).toList
  let bound := unit.namespaces.foldl (fun total ns => total + ns.lemmas.size) 1
  for _ in [0:bound * bound + 1] do
    match work with
    | [] => break
    | current :: rest =>
        work := rest
        if visited.contains current then continue
        visited := visited.push current
        work := work ++ (appliedLemmas unit current).toList
  return visited

/-- The recursion groups of the lemmas `roots` reach, each after the groups
it applies. -/
private def lemmaGroups (unit : ValidatedUnit) (roots : Array LeanerIR.QualifiedRef) :
    Array (Array LeanerIR.QualifiedRef) := Id.run do
  let mut lemmas := roots
  for root in roots do
    for reached in reachableLemmas unit root do
      unless lemmas.contains reached do lemmas := lemmas.push reached
  let reaches := lemmas.map fun lemma => (lemma, reachableLemmas unit lemma)
  let reached (source target : LeanerIR.QualifiedRef) : Bool :=
    (reaches.find? (·.1 == source)).any (·.2.contains target)
  let mut groups : Array (Array LeanerIR.QualifiedRef) := #[]
  let mut placed : Array LeanerIR.QualifiedRef := #[]
  for _ in [0:lemmas.size + 1] do
    for lemma in lemmas do
      if placed.contains lemma then continue
      let group := lemmas.filter fun other =>
        other == lemma || (reached lemma other && reached other lemma)
      -- A group comes after every group it applies.
      let ready := (reachableLemmas unit lemma).all fun applied =>
        group.contains applied || placed.contains applied
      if ready then
        groups := groups.push group
        placed := placed ++ group
  return groups

/-- The lemma items of a module, not descending into its Lean items. -/
private partial def lemmaItemsOf (stx : Syntax) : Array Syntax :=
  stx.getArgs.foldl (fun found child =>
    if child.isOfKind ``leanerLemmaItem then found.push child
    else if child.isOfKind ``leanerTheoremItem then found
    else found ++ lemmaItemsOf child) #[]

/-- The trusted steps a lemma's theorem takes as hypotheses: its own and
those of every lemma it reaches. -/
def lemmaTrustClosure (unit : ValidatedUnit) (reference : LeanerIR.QualifiedRef) :
    MetaM (Array Name) := do
  let mut found : Array Name := #[]
  for lemma in #[reference] ++ reachableLemmas unit reference do
    for (_, name) in (← Contract.lemmaNames unit lemma).trusted do
      unless found.contains name do found := found.push name
  return found

/-- The syntax of a definition's parameter type. -/
private def lemmaTypeSyntax (type : Lean.Expr) : CommandElabM Term := do
  match type with
  | .sort .zero => `(term| Prop)
  | .const name _ => pure (rootIdent name)
  | _ => throwError m!"a lemma parameter has no spelled type: {type}"

/-- A lemma's theorem, in its recursion group: over what its statement
reads, its parameters, and its premise; with its measure when the group
recurses. -/
private def lemmaTheoremCommand (unit : ValidatedUnit) (twins : Array SpecTypes.TwinInfo)
    (reference : LeanerIR.QualifiedRef) (recursive : Bool) :
    CommandElabM (TSyntax `command) := do
  let names ← liftTermElabM (Contract.ensureLemmaDefinitions unit twins reference)
  let some (_, declaration) := Contract.lemmaOf? unit reference
    | throwError "a lemma does not resolve"
  let (readsUnit, readsRequires, readsFrame, readsState) := Contract.lemmaReadsOf unit reference
  let mut readBinders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) := #[]
  let mut readArguments : Array Term := #[]
  if readsRequires || readsFrame || readsState then
    readBinders := readBinders.push
      (← `(bracketedBinder| (unit : LeanerIR.Validation.ValidatedUnit)))
    readArguments := readArguments.push (← `(term| unit))
  if readsUnit then
    readBinders := readBinders.push
      (← `(bracketedBinder| (executable : LeanerIR.Validation.ExecutableUnit)))
    readArguments := readArguments.push (← `(term| executable))
  if readsRequires then
    readBinders := readBinders.push
      (← `(bracketedBinder| (requiresTable : LeanerIR.Proofs.RequiresTable unit)))
    readArguments := readArguments.push (← `(term| requiresTable))
  if readsFrame then
    readBinders := readBinders.push (← `(bracketedBinder| (frame : LeanerIR.Proofs.Denote.Skolems unit)))
    readArguments := readArguments.push (← `(term| frame))
  if readsState then
    readBinders := readBinders.push (← `(bracketedBinder| (state : LeanerIR.Proofs.Denote.Memory unit)))
    readArguments := readArguments.push (← `(term| state))
  -- What the lemma and those it applies assume, as hypotheses.
  let trust ← liftTermElabM (lemmaTrustClosure unit reference)
  let trustBinders ← trust.mapM fun name =>
    `(bracketedBinder| ($(mkIdent (trustBinderName name)) : $(rootIdent name)))
  let parameterTypes ← liftTermElabM (Contract.lemmaParameterTypes unit reference)
  let parameters := (declaration.signature.parameters.zip parameterTypes).mapIdx
    fun index (parameter, type) => (mkIdent (Name.mkSimple s!"{parameter.name}_{index}"), type)
  let parameterBinders ← parameters.mapM fun (name, type) => do
    `(bracketedBinder| ($name : $(← lemmaTypeSyntax type)))
  let bundle ← parameters.foldrM (init := ← `(term| ())) fun (name, _) rest =>
    `(term| ($name, $rest))
  let premise := mkIdent `premise
  let stated := mkIdent `statedPremise
  let requires := rootIdent names.requires
  let ensures := rootIdent names.ensures
  -- Each step proved, then what it gives assumed; a split continues once per
  -- case, so each later tactic runs on every case.
  let mut steps : Array (TSyntax `tactic) := #[]
  for (condition, stepName) in declaration.proof.zip names.steps do
    let step := mkIdent (Name.mkSimple s!"step_{steps.size}")
    match condition.kind with
    | .assumption =>
        -- An assumed step holds by its hypothesis, of the premise and the
        -- assumptions before it.
        let some (_, trusted) := names.trusted.find? (·.1 == steps.size)
          | throwError "an assumed step has no hypothesis"
        let earlier := (names.trusted.filter (·.1 < steps.size)).map fun (index, _) =>
          (mkIdent (Name.mkSimple s!"assumed_{index}") : Term)
        let assumed := mkIdent (Name.mkSimple s!"assumed_{steps.size}")
        steps := steps.push (← `(tactic| all_goals
          (have $assumed : $(rootIdent stepName) $readArguments* $bundle :=
             $(mkIdent (trustBinderName trusted)) $readArguments* $bundle $stated $earlier*
           have $step := $assumed
           simp only [$(rootIdent stepName):ident] at $step:ident
           try leaner_denote_normalize at $step:ident
           leaner_denote_split_hypotheses)))
    | _ =>
        steps := steps.push (← `(tactic| all_goals
          (have $step : $(rootIdent stepName) $readArguments* $bundle := by
             simp only [$(rootIdent stepName):ident]
             leaner_denote_lemma_owed
           simp only [$(rootIdent stepName):ident] at $step:ident
           leaner_denote_lemma_holds $step)))
  let tactics : Array (TSyntax `tactic) :=
    -- The premise as stated, which an assumed step's hypothesis takes.
    #[← `(tactic| have $stated := $premise),
      ← `(tactic| simp only [$requires:ident, LeanerIR.Proofs.Obligation_iff] at $premise:ident),
      ← `(tactic| try leaner_denote_normalize at $premise:ident),
      ← `(tactic| leaner_denote_split_hypotheses)] ++ steps ++
    #[← `(tactic| all_goals
        (simp only [$ensures:ident]
         try leaner_denote_normalize
         all_goals leaner_denote_close))]
  let script ← `(tactic| ($tactics;*))
  let theoremName := rootIdent names.theorem_
  unless recursive do
    return ← `(command|
      theorem $theoremName $trustBinders* $readBinders* $parameterBinders*
          ($premise : $requires $readArguments* $bundle) : $ensures $readArguments* $bundle := by
        $script:tactic)
  -- The measure: the declared components, or the integer parameters.
  let measures ← if names.measures.isEmpty then
      (parameters.filter (·.2 == mkConst ``Int)).mapM fun (name, _) =>
        `(term| LeanerIR.Proofs.lemmaMeasure $name)
    else names.measures.mapM fun measure =>
      `(term| LeanerIR.Proofs.lemmaMeasure ($(rootIdent measure) $readArguments* $bundle))
  let some last := measures.back?
    | throwError m!"a recursive lemma needs a `decreases` measure: it has no integer parameter"
  let measure ← measures.pop.foldrM (init := last) fun component rest =>
    `(term| ($component, $rest))
  let measureDefinitions : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) ←
    names.measures.mapM fun measure =>
      `(Lean.Parser.Tactic.simpLemma| $(rootIdent measure):ident)
  `(command|
    theorem $theoremName $trustBinders* $readBinders* $parameterBinders*
        ($premise : $requires $readArguments* $bundle) : $ensures $readArguments* $bundle := by
      $script:tactic
    termination_by $measure
    decreasing_by
      all_goals
        ((try simp only [$measureDefinitions,*])
         leaner_denote_lemma_decreasing))

/-- Verify the lemmas `roots` reach, a recursion group at a time, each after
the groups it applies; a group whose theorems exist is skipped. -/
private def verifyLemmas (unit : ValidatedUnit) (segments : Array String)
    (roots : Array LeanerIR.QualifiedRef) (items : Array (String × Syntax)) (reference : Syntax) :
    CommandElabM Unit := do
  if roots.isEmpty then return
  let twins ← SpecTypes.ensureSpecTypes segments unit
  let budget := leaner.verifyHeartbeats.get (← getOptions)
  for group in lemmaGroups unit roots do
    let names ← liftTermElabM <| group.mapM fun lemma => Contract.lemmaNames unit lemma
    let environment ← getEnv
    if names.all fun names => environment.contains names.theorem_ then continue
    let recursive := group.size > 1 || group.any fun lemma => (appliedLemmas unit lemma).contains lemma
    -- A group is reported at its first lemma this module declares.
    let reference := (group.findSome? fun lemma => do
      let qualified ← unit.tables.names[lemma.name.index]?
      (items.find? (·.1 == qualified.name)).map (·.2)).getD reference
    let spelled := ", ".intercalate (group.filterMap fun lemma =>
      (unit.tables.names[lemma.name.index]?).map (s!"`{·.name}`")).toList
    let errorsBefore := countErrors (← get).messages
    -- A group that fails leaves the others, and the functions, to be tried.
    try
      -- Built at the lemma, so that what fails in it is reported there.
      let theorems ← withRef reference <|
        group.mapM fun lemma => lemmaTheoremCommand unit twins lemma recursive
      let command ← if group.size == 1 then pure theorems[0]!
        else `(command| mutual $[$theorems]* end)
      let budgetSyntax := Syntax.mkNumLit (toString budget)
      stageLog s!"lemmas {names.map (·.theorem_)}: theorem start"
      withRef reference do
        Perf.withPhase .verification <|
          elabCommand (← `(command| set_option Elab.async false in
            set_option maxHeartbeats $budgetSyntax in $command:command))
      stageLog s!"lemmas {names.map (·.theorem_)}: theorem done"
      if countErrors (← get).messages > errorsBefore then
        logErrorAt reference m!"leaner verification failed: the lemma {spelled} is not established"
      else
        for lemmaNames in names do
          modifyEnv (LeanerIR.Proofs.Denote.establishedLemmas.tag · lemmaNames.theorem_)
    catch error =>
      logErrorAt reference m!"leaner verification failed: the lemma {spelled} is not \
        established: {error.toMessageData}"

/-- Verify one function through its denotation.  A member of a cycle of
calls is proved over a meaning for each member satisfying its contract,
standing for the calls to them: at the runtime family, or, when `family`
holds, at every skolem family and type instantiation.  Its typed theorem
alone is elaborated, and the natives it assumes returned, for `verifyCycle`
to conclude. -/
private def verifyMember (reference : Syntax) (segments : Array String) (function : String)
    (script? : Option (TSyntax ``Lean.Parser.Tactic.tacticSeq))
    (cycle : Array (FunctionHandle × String)) (family : Bool := false) :
    CommandElabM (Array (FunctionHandle × String) × Array (FunctionHandle × String) ×
      Array Name) := do
  let namespaceName := pathName segments
  let some unit := LeanerLang.registeredUnit? (← getEnv) namespaceName
    | throwErrorAt reference s!"unknown Leaner namespace `{namespaceName}`"
  let some (namespaceIndex, ns, functionIndex, declaration) := findFunction? unit function
    | throwErrorAt reference s!"unknown function `{function}` in `{namespaceName}`"
  let unitDefinition ← ensureUnitDefinition segments unit
  let unitTerm ← unitTermOf segments unit
  discard <| ensureUnitDefinitions segments unit
  stageLog s!"{function}: semantics"
  let handle : FunctionHandle := ⟨⟨namespaceIndex⟩, ⟨functionIndex⟩⟩
  let compiled ← match compileFunction unit handle with
    | .ok compiled => pure compiled
    | .error reason => throwErrorAt reference m!"no denotation for `{function}`: {reason}"
  let some (params, result) := nativeSignature? unit ⟨namespaceIndex⟩ declaration
    | throwErrorAt reference m!"no denotation for `{function}`: its signature is not native"
  ensureContracts segments function unit namespaceIndex ns declaration params result
  stageLog s!"{function}: compiled and contracts"
  let artifacts := Name.str namespaceName function
  let base := (← getCurrNamespace) ++ artifacts
  let typedVerified := typedVerifiedName segments function
  if (← getEnv).contains (base ++ `typedVerified) then
    -- A colliding name alone is not evidence of a denotation proof.
    withRef reference <| requireNativeArtifacts base (selectsBitVectors declaration)
    return (#[], #[], #[])
  if script?.isNone && requiresAuthoredProof declaration then
    throwErrorAt reference m!"leaner verification failed: `{function}` sets \
      `pragma verify = manual`; {proofRequest ns function}"
  let compiledIdent := rootIdent (compiledName segments function)
  let bodyIdent := rootIdent (Name.str (Name.str namespaceName function) "body")
  let row := rootIdent (Name.str (Name.str namespaceName function) "row")
  let paramsTerm := rootIdent (Name.str (Name.str namespaceName function) "params")
  let localsTerm := rootIdent (Name.str (Name.str namespaceName function) "locals")
  let shapeTerm := rootIdent (Name.str (Name.str namespaceName function) "shape")
  let mutablesTerm := rootIdent (Name.str (Name.str namespaceName function) "mutables")
  let compiledEqIdent := rootIdent (compiledEqName segments function)
  let typedContract := rootIdent (typedContractName segments function)
  let publicContract := rootIdent (contractName segments function)
  let handleTerm ← handleSyntax ⟨⟨namespaceIndex⟩, ⟨functionIndex⟩⟩
  let mut calleePairs : Array Term := #[]
  -- The inlined callees, whose loops the proof meets with their invariants.
  let mut inlined : Array (FunctionHandle × ValidatedNamespace ×
    LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody × Function unit) := #[]
  -- The natives the theorems assume: those called here or by an inlined
  -- callee, and those a verified callee assumes.
  let mut natives : Array (FunctionHandle × String) := #[]
  -- The functions whose in-body assumptions the theorems assume to hold:
  -- its own, and those a verified callee assumes.
  let ownAssumptions := statesAssumptions ns declaration
  if ownAssumptions && !cycle.isEmpty then
    throwErrorAt reference m!"`{function}` states an in-body assumption and is verified by \
      induction with the members of its cycle; the hypothesis that its assumptions hold is \
      stated of a function verified on its own"
  let mut assumptionDeps : Array (FunctionHandle × String) :=
    if ownAssumptions then #[(handle, function)] else #[]
  -- The assumed steps of the lemmas its own proof applies, and those a
  -- verified callee's theorems take.
  let mut trustDeps : Array Name ← liftTermElabM do
    (functionLemmas ns declaration).foldlM (init := #[]) fun found lemma => do
      return (← lemmaTrustClosure unit lemma).foldl (init := found) fun found name =>
        if found.contains name then found else found.push name
  -- A call to any member of its cycle, itself included, is assumed at its
  -- contract, by fixed-point induction.
  let members := cycle.map (·.1)
  -- A generic function, and every member of a cycle through one, is proved
  -- over every skolem family and type instantiation; any other at the
  -- runtime family, a closed term the normalizer's caches keep, and the
  -- empty instantiation.
  let generic := isGeneric declaration || family
  -- A generic function outside a cycle is proved for the frames it runs in
  -- (`FrameOf`); a call passing its own type parameters runs in its frame.
  let framed := isGeneric declaration && cycle.isEmpty
  let arity := (declaration.signature.generics.filter (·.kind == .typeArg)).size
  let ownCalls : Array (FunctionHandle × Array LeanerIR.TypeUse) := if !framed then #[] else
    compiled.body.foldCalls (fun callee typeArgs found =>
      if !typeArgs.isEmpty && typeArgs.size == arity &&
          callee.namespaceId == handle.namespaceId &&
          LeanerIR.SemanticOperations.ownParametersIn unit handle typeArgs &&
          !found.contains (callee, typeArgs)
        then found.push (callee, typeArgs) else found) #[]
  let mut edges : Array (FunctionHandle × Array FunctionHandle) :=
    #[(handle, (compiled.body.callees #[]).filter fun callee =>
      callee != handle && !members.contains callee)]
  -- An inlined callee's body brings its own callees into the proof, so the
  -- callees are collected through every inlined body.
  let mut worklist := (compiled.body.callees #[]).filter fun callee =>
    callee != handle && !members.contains callee
  let mut visited : Array FunctionHandle := #[handle] ++ members
  -- The generic calls of the bodies proved at the empty instantiation: the
  -- target's, when it has no type parameters, and every inlined callee's.
  let recordGeneric (callee : FunctionHandle) (typeArgs : Array LeanerIR.TypeUse)
      (found : Array (FunctionHandle × Array LeanerIR.TypeUse)) :=
    if typeArgs.isEmpty || found.contains (callee, typeArgs) then found
    else found.push (callee, typeArgs)
  let mut instantiated : Array (FunctionHandle × Array LeanerIR.TypeUse) :=
    if generic then ownCalls else compiled.body.foldCalls recordGeneric #[]
  -- Every certificate and theorem from here on is elaborated synchronously,
  -- so a failure is counted at the target.
  let errorsBefore := countErrors (← get).messages
  let targetsOfClosures := closureTargets unit
  let mut behaviorPairs : Array Term := #[]
  let mut behaviorTargets : Array FunctionHandle := #[]
  -- The targets of the closures the contracts the proof meets name: its own
  -- and those of the callees it uses through their contracts.
  let mut contractTargets := contractClosureTargets unit handle.namespaceId ns declaration
  while let some callee := worklist.back? do
    worklist := worklist.pop
    if visited.contains callee then continue
    visited := visited.push callee
    let some calleeNs := unit.namespaces[callee.namespaceId.index]?
      | throwErrorAt reference "a callee's namespace is out of range"
    let some calleeDeclaration := calleeNs.functions[callee.functionId.index]?
      | throwErrorAt reference "a callee is out of range"
    let calleeKey := functionKey unit callee
    let theoremName := (← getCurrNamespace) ++ typedSemanticsVerifiedName segments calleeKey
    let handleTerm ← handleSyntax callee
    let usesContract := contractStandsFor unit callee.namespaceId calleeNs calleeDeclaration
    let throughContract := usedThroughContract unit callee calleeNs calleeDeclaration
    if throughContract || calleeDeclaration.body == .absent then
      for target in contractClosureTargets unit callee.namespaceId calleeNs calleeDeclaration do
        unless contractTargets.contains target do contractTargets := contractTargets.push target
    if calleeDeclaration.body == .absent then
      -- A native has no body: its contract is assumed, as a hypothesis. One
      -- without a specification is assumed by the model the Move Prover's
      -- prelude gives it (`NativeModel`), and rejected without one, as the
      -- Prover rejects it.
      let nativeModel := nativeModelOf? unit callee calleeNs calleeDeclaration
      if readsNativeModel calleeDeclaration && nativeModel.isNone then
        throwErrorAt reference m!"`{function}` calls the native `{calleeKey}`, which has \
          neither a specification nor a prelude model; specify it"
      unless usesContract do
        throwErrorAt reference m!"`{function}` calls the native `{calleeKey}`, which returns \
          a mutable reference without stating its `final` value: a caller reasons over such a \
          callee through its body, and a native has none"
      let some (nativeParams, nativeResult) :=
          nativeSignature? unit callee.namespaceId calleeDeclaration
        | throwErrorAt reference m!"the native `{calleeKey}` has no native signature"
      ensureContracts segments calleeKey unit callee.namespaceId.index calleeNs
        calleeDeclaration nativeParams nativeResult nativeModel
      liftTermElabM (publishNativeSignature segments calleeKey nativeParams nativeResult)
      unless natives.any (·.1 == callee) do natives := natives.push (callee, calleeKey)
      -- Unapplied, so a generic native's family and instantiation are found
      -- at each call, not fixed at the first.
      calleePairs := calleePairs.push
        (← `(term| PProd.mk $handleTerm (PProd.mk $(Syntax.mkStrLit calleeKey)
          @$(mkIdent (Name.mkSimple s!"native_{calleeKey}")))))
    else if throughContract &&
        (hasInterfaceView calleeDeclaration || automaticVerificationDisabled unit callee.namespaceId calleeNs calleeDeclaration ||
          (Contract.mapRoleOf? unit callee.namespaceId calleeNs calleeDeclaration).isSome) then
      -- Callers see such a callee through the interface its body is not
      -- proved against, or through the contract of a function whose
      -- verification is disabled: they assume it, as they assume a
      -- native's contract.
      let some (nativeParams, nativeResult) :=
          nativeSignature? unit callee.namespaceId calleeDeclaration
        | throwErrorAt reference m!"`{calleeKey}` has no native signature"
      ensureInterfaceContract segments calleeKey unit callee.namespaceId.index calleeNs
        calleeDeclaration nativeParams nativeResult
      liftTermElabM (publishNativeSignature segments calleeKey nativeParams nativeResult)
      unless natives.any (·.1 == callee) do natives := natives.push (callee, calleeKey)
      -- Unapplied, so a generic native's family and instantiation are found
      -- at each call, not fixed at the first.
      calleePairs := calleePairs.push
        (← `(term| PProd.mk $handleTerm (PProd.mk $(Syntax.mkStrLit calleeKey)
          @$(mkIdent (Name.mkSimple s!"native_{calleeKey}")))))
    else if (← getEnv).contains theoremName && throughContract then
      for entry in ((nativeDependencies.getState (← getEnv)).find? theoremName.getPrefix).getD #[] do
        unless natives.any (·.1 == entry.1) do natives := natives.push entry
      for entry in ((assumptionDependencies.getState (← getEnv)).find? theoremName.getPrefix).getD #[] do
        unless assumptionDeps.any (·.1 == entry.1) do assumptionDeps := assumptionDeps.push entry
      for name in ((lemmaTrustDependencies.getState (← getEnv)).find? theoremName.getPrefix).getD #[] do
        unless trustDeps.contains name do trustDeps := trustDeps.push name
      calleePairs := calleePairs.push
        (← `(term| PProd.mk $handleTerm
          (PProd.mk $(Syntax.mkStrLit calleeKey) (@$(mkIdent theoremName) _ _ prepared))))
    else if throughContract then
      throwErrorAt reference m!"`{function}` calls `{calleeKey}`, which is not verified; \
        verify the callee first"
    else
      -- Any other callee is inlined: its compiled body stands for its
      -- meaning through the agreement theorem.
      let calleeCompiled ← match compileFunction unit callee with
        | .ok compiled => pure compiled
        | .error reason =>
            throwErrorAt reference m!"no denotation for the callee `{calleeKey}`: {reason}"
      liftTermElabM (publishCompiled segments unit calleeKey callee calleeCompiled)
      inlined := inlined.push (callee, calleeNs, calleeDeclaration, calleeCompiled)
      worklist := calleeCompiled.body.callees worklist
      -- As the target's own: a generic target's calls are proved over every
      -- family and instantiation, with no empty-instantiation frames.
      unless generic do
        instantiated := calleeCompiled.body.foldCalls recordGeneric instantiated
      edges := edges.push (callee, calleeCompiled.body.callees #[])
      let certificate := Name.str (Name.str namespaceName calleeKey) "compiledSemantics"
      unless (← getEnv).contains certificate do
        elabCommand (← `(command|
          set_option Elab.async false in
          theorem $(rootIdent certificate) :
              LeanerIR.Proofs.Denote.compileFunction $unitTerm $handleTerm =
                .ok $(rootIdent (compiledName segments calleeKey)) :=
            $(rootIdent (compiledEqName segments calleeKey))))
      calleePairs := calleePairs.push
        (← `(term| PProd.mk $handleTerm
          (PProd.mk $(Syntax.mkStrLit calleeKey) $(rootIdent certificate))))
      -- A closure's target with a theorem is known by it as well, after the
      -- body a call takes: what `ensures_of` and `aborts_of` of a closure
      -- over it entail.
      if targetsOfClosures.contains callee && (← getEnv).contains theoremName then
        for entry in ((nativeDependencies.getState (← getEnv)).find? theoremName.getPrefix).getD #[] do
          unless natives.any (·.1 == entry.1) do natives := natives.push entry
        for entry in ((assumptionDependencies.getState (← getEnv)).find? theoremName.getPrefix).getD #[] do
          unless assumptionDeps.any (·.1 == entry.1) do assumptionDeps := assumptionDeps.push entry
        for name in ((lemmaTrustDependencies.getState (← getEnv)).find? theoremName.getPrefix).getD #[] do
          unless trustDeps.contains name do trustDeps := trustDeps.push name
        behaviorPairs := behaviorPairs.push
          (← `(term| PProd.mk $handleTerm
            (PProd.mk $(Syntax.mkStrLit calleeKey) (@$(mkIdent theoremName) _ _ prepared))))
        behaviorTargets := behaviorTargets.push callee
  for target in contractTargets do
    if behaviorTargets.contains target then continue
    let targetKey := functionKey unit target
    let theoremName := (← getCurrNamespace) ++ typedSemanticsVerifiedName segments targetKey
    unless (← getEnv).contains theoremName do continue
    for entry in ((nativeDependencies.getState (← getEnv)).find? theoremName.getPrefix).getD #[] do
      unless natives.any (·.1 == entry.1) do natives := natives.push entry
    for entry in ((assumptionDependencies.getState (← getEnv)).find? theoremName.getPrefix).getD #[] do
      unless assumptionDeps.any (·.1 == entry.1) do assumptionDeps := assumptionDeps.push entry
    for name in ((lemmaTrustDependencies.getState (← getEnv)).find? theoremName.getPrefix).getD #[] do
      unless trustDeps.contains name do trustDeps := trustDeps.push name
    behaviorPairs := behaviorPairs.push
      (← `(term| PProd.mk $(← handleSyntax target)
        (PProd.mk $(Syntax.mkStrLit targetKey) (@$(mkIdent theoremName) _ _ prepared))))
    behaviorTargets := behaviorTargets.push target
  calleePairs := calleePairs ++ behaviorPairs
  -- Each generic call at the empty instantiation instantiates its callee's
  -- frame as the runtime computes it: a kernel-checked literal the call's
  -- meaning is stated at.
  let mut instantiationCertificates : Array Term := #[]
  for (callee, typeArgs) in instantiated, index in [0:instantiated.size] do
    let name := artifacts ++ Name.mkSimple s!"frameInstantiation_{index}"
    let certificate := rootIdent name
    unless (← getEnv).contains name do
      let value := frameInstantiation unit callee #[] typeArgs
      let calleeTerm ← handleSyntax callee
      let typeIdTerm (typeId : LeanerIR.TypeId) : CommandElabM Term :=
        `(term| (⟨$(Syntax.mkNatLit typeId.index)⟩ : LeanerIR.TypeId))
      let typeArgsSyntax (typeArgs : Array LeanerIR.TypeUse) : CommandElabM Term := do
        let typeUses ← typeArgs.mapM fun (typeUse : LeanerIR.TypeUse) => do
          `(term| (⟨$(← typeIdTerm typeUse.typeId), ⟨$(Syntax.mkNatLit typeUse.loc.index)⟩⟩ :
            LeanerIR.TypeUse))
        `(term| (#[$typeUses,*] : Array LeanerIR.TypeUse))
      let typeArgsTerm ← typeArgsSyntax typeArgs
      -- The frame reads which types the arguments name, not where they
      -- occur (`frameInstantiation_congr`): it is certified once for those
      -- types, at location 0, and shared by every call naming them.
      let frameArgs := typeArgs.map fun typeUse => { typeUse with loc := ⟨0⟩ }
      let frameArgsTerm ← typeArgsSyntax frameArgs
      let entries ← value.mapM fun (symbolic, concrete) => do
        `(term| ($(← typeIdTerm symbolic), $(← typeIdTerm concrete)))
      let valueTerm ← `(term| (#[$entries,*] : Array (LeanerIR.TypeId × LeanerIR.TypeId)))
      -- The runtime's computation over the unit is certified by
      -- witness: the callee namespace's key map, decided by the kernel once,
      -- and this instantiation's witnesses, which the kernel checks in one
      -- pass over the table with logarithmic reads — never by replaying its
      -- searches.
      -- Every proof is a kernel decision (`Eq.refl`), stated from syntax and
      -- built as a term, so the elaborator evaluates nothing.
      let namespaceIndex := callee.namespaceId.index
      let some calleeNs := unit.namespaces[namespaceIndex]?
        | throwErrorAt reference m!"the callee's namespace is out of range"
      let viewEq ← liftTermElabM (publishCompileView segments unit callee.namespaceId)
      let view := rootIdent (Name.str (semanticsName segments) s!"compileView_{namespaceIndex}")
      let mapName := Name.str (semanticsName segments) s!"keyMap_{namespaceIndex}"
      let mapCorrect := mapName ++ `correct
      let map := rootIdent mapName
      let instantiations :=
        LeanerIR.SemanticOperations.instantiateGenericArguments #[] (frameArgs.map .typeArg)
      let witnessValue := LeanerIR.Validation.computeWitnesses calleeNs instantiations
      -- Once per unit for a namespace and type arguments: every call at them
      -- shares the instantiation.
      let frameKey := "_".intercalate (frameArgs.toList.map fun (typeUse : LeanerIR.TypeUse) =>
        s!"{typeUse.typeId.index}")
      let frameName := Name.str (semanticsName segments) s!"frame_{namespaceIndex}_{frameKey}"
      let witnessesName := frameName ++ `witnesses
      let witnesses := rootIdent witnessesName
      let treeName := frameName ++ `witnessTree
      let tree := rootIdent treeName
      let tableName := Name.str (semanticsName segments) s!"typesTable_{namespaceIndex}"
      let tableEq := tableName ++ `eq
      let table := rootIdent tableName
      let kindsName := Name.str (semanticsName segments) s!"lifetimeKinds_{namespaceIndex}"
      let kindsEq := kindsName ++ `eq
      let kinds := rootIdent kindsName
      if !(← getEnv).contains frameName then liftTermElabM do
        unless (← getEnv).contains mapName do
          -- Once per namespace: the types as a literal, so the kernel reads
          -- them without deriving them from the unit (`Array.mk` of the list
          -- literal: its `toList` is a projection), and the key map over
          -- their fingerprints, answering every search by one lookup.
          literalDefinition tableName (mkApp (mkConst ``Array [0]) (mkConst ``LeanerIR.Ty))
            (mkApp2 (mkConst ``Array.mk [0]) (mkConst ``LeanerIR.Ty)
              (toExpr calleeNs.tables.types.toList))
          reflexiveTheorem tableEq (← `(term| ($view).source.tables.types = $table))
          literalDefinition mapName (mkConst ``LeanerIR.Validation.KeyMap)
            (toExpr (LeanerIR.Validation.computeKeyMap calleeNs.tables.types))
          decidedByKernel mapCorrect (← `(term| LeanerIR.Validation.Correct ($view).types
            ($table).size $map))
          -- The lifetimes' kinds, indexed: a reference type's instantiation
          -- reads its lifetime's.
          literalDefinition kindsName
            (mkApp (mkConst ``LeanerIR.Validation.IndexedArena) (mkConst ``LeanerIR.LifetimeKind))
            (toExpr (LeanerIR.Validation.IndexedArena.ofArray
              (LeanerIR.Validation.lifetimeKinds calleeNs)))
          reflexiveTheorem kindsEq (← `(term| LeanerIR.Validation.IndexedArena.ofArray
            (LeanerIR.Validation.lifetimeKinds ($view).source) = $kinds))
        -- The witnesses, as the array the theorem folds and as the index the
        -- checker reads them through.
        literalDefinition witnessesName
          (mkApp (mkConst ``Array [0]) (mkConst ``LeanerIR.Validation.Witness))
          (toExpr witnessValue)
        literalDefinition treeName
          (mkApp (mkConst ``LeanerIR.Validation.IndexedArena) (mkConst ``LeanerIR.Validation.Witness))
          (toExpr (LeanerIR.Validation.IndexedArena.ofArray witnessValue))
        reflexiveTheorem (frameName ++ `treeEq)
          (← `(term| LeanerIR.Validation.IndexedArena.ofArray $witnesses = $tree))
        reflexiveTheorem (frameName ++ `checked) (← `(term|
          LeanerIR.Validation.checkAll $kinds ($view).types $map
          (LeanerIR.SemanticOperations.instantiateGenericArguments #[]
            (($frameArgsTerm).map LeanerIR.GenericArgument.typeArg))
          $tree ($table).size = true))
        reflexiveTheorem (frameName ++ `sizes) (← `(term| ($witnesses).size = ($table).size))
        reflexiveTheorem (frameName ++ `depths) (← `(term| ($witnesses).toList.all
          (fun witness => decide (witness.depth ≤ ($table).size)) = true))
        reflexiveTheorem (frameName ++ `value)
          (← `(term| LeanerIR.Validation.foldWitnesses $tree ($table).size = $valueTerm))
        let statement ← Lean.Elab.Term.elabType (← `(term|
          LeanerIR.Proofs.Denote.frameInstantiationIn $view #[] $frameArgsTerm = $valueTerm))
        let proof ← Lean.Elab.Term.elabTermEnsuringType (← `(term|
          (LeanerIR.Proofs.Denote.frameInstantiationIn_eq_of_check $frameArgsTerm rfl
            (LeanerIR.Proofs.Denote.types_of_view $(rootIdent viewEq)) $(rootIdent tableEq)
            $(rootIdent kindsEq) $(rootIdent mapCorrect) $(rootIdent (frameName ++ `treeEq))
            $(rootIdent (frameName ++ `checked)) $(rootIdent (frameName ++ `sizes))
            $(rootIdent (frameName ++ `depths))).trans $(rootIdent (frameName ++ `value))))
          statement
        Lean.Elab.Term.synthesizeSyntheticMVarsNoPostponing
        let proof ← instantiateMVars proof
        addDecl (.thmDecl
          { name := frameName, levelParams := [], type := ← instantiateMVars statement, value := proof })
      elabCommand (← `(command|
        set_option Elab.async false in
        theorem $certificate :
            LeanerIR.Proofs.Denote.frameInstantiation $unitTerm $calleeTerm #[] $typeArgsTerm =
              $valueTerm := by
          rw [LeanerIR.Proofs.Denote.frameInstantiation_congr (left := $typeArgsTerm)
              (right := $frameArgsTerm) rfl,
            LeanerIR.Proofs.Denote.frameInstantiation_of_view $(rootIdent viewEq)]
          exact $(rootIdent frameName)))
    instantiationCertificates := instantiationCertificates.push
      (← `(term| $certificate))
  -- A call passing the target's own type parameters runs in its frame: at
  -- the empty frame as the kernel computes the call's, otherwise by
  -- `frameInstantiation_own`.
  for (callee, typeArgs) in ownCalls, index in [0:ownCalls.size] do
    let name := artifacts ++ Name.mkSimple s!"ownFrame_{index}"
    unless (← getEnv).contains name do
      unless (frameInstantiation unit callee #[] typeArgs).isEmpty do
        throwErrorAt reference m!"`{function}` calls a function of its namespace with its own \
          type parameters, whose frame outside any call is not the empty instantiation"
      let some emptyIndex := instantiated.idxOf? (callee, typeArgs)
        | throwErrorAt reference "an own call has no frame certificate"
      let empty := rootIdent (artifacts ++ Name.mkSimple s!"frameInstantiation_{emptyIndex}")
      let calleeTerm ← handleSyntax callee
      let typeUses ← typeArgs.mapM fun (typeUse : LeanerIR.TypeUse) =>
        `(term| (⟨⟨$(Syntax.mkNatLit typeUse.typeId.index)⟩, ⟨$(Syntax.mkNatLit typeUse.loc.index)⟩⟩ :
          LeanerIR.TypeUse))
      let typeArgsTerm ← `(term| (#[$typeUses,*] : Array LeanerIR.TypeUse))
      let parametersName := name ++ `parameters
      -- Decided by the kernel on the unit.
      liftTermElabM <| reflexiveTheorem parametersName (← `(term|
        LeanerIR.SemanticOperations.ownParametersIn $(mkIdent unitDefinition) $handleTerm
          $typeArgsTerm = true))
      elabCommand (← `(command|
        set_option Elab.async false in
        theorem $(rootIdent name) {typeInstantiation : Array (LeanerIR.TypeId × LeanerIR.TypeId)}
            (frame : LeanerIR.Proofs.Denote.FrameOf $unitTerm $handleTerm
              $(Syntax.mkNatLit arity) typeInstantiation) :
            LeanerIR.Proofs.Denote.frameInstantiation $unitTerm $calleeTerm typeInstantiation
              $typeArgsTerm = typeInstantiation :=
          LeanerIR.Proofs.Denote.frameInstantiation_own rfl rfl $(rootIdent parametersName) $empty
            frame))
    instantiationCertificates := instantiationCertificates.push
      (← `(term| $(rootIdent name) frame))
  stageLog s!"{function}: callees and certificates"
  if let some cycle := inliningCycle? edges #[] handle then
    let name := (do
      let calleeNs ← unit.namespaces[cycle.namespaceId.index]?
      let declaration ← calleeNs.functions[cycle.functionId.index]?
      let name ← calleeNs.tables.names[declaration.name.index]?
      pure name.name).getD "a callee"
    throwErrorAt reference m!"`{function}` reaches `{name}` again through callees that are \
      inlined; only a function calling itself directly is verified by induction, so specify \
      and verify the callees on the cycle"
  for (_, name) in cycle do
    calleePairs := calleePairs.push
      (← `(term| PProd.mk $(cycleSelf name) (PProd.mk $(Syntax.mkStrLit name)
        $(cycleSelfVerified name))))
  let saved ← get
  try
    let twins ← SpecTypes.ensureSpecTypes segments unit
    let stored ← ensureStoredInvariant segments unit twins
    -- The invariants of the loops and the conditions of the assertions the
    -- proof meets: the function's own and those of its inlined callees, each
    -- keyed by its site and function.
    let owners := #[(handle, ns, declaration, compiled)] ++ inlined
    let invariants ← liftTermElabM <| withExecutableSkolems fun executable skolems => do
      owners.flatMapM fun (owner, ownerNs, ownerDeclaration, ownerCompiled) => do
        let codecs := mkApp (mkConst ``Carriers.codec) (← Contract.frameCarriers skolems)
        let types ← Contract.frameTypes skolems
        let row := ownerCompiled.params ++ ownerCompiled.locals
        let invariants ← loopInvariants unit executable owner.namespaceId ownerNs ownerDeclaration
          ownerCompiled.params row codecs types twins stored.predicate
        let assertions ← assertionConditions unit executable owner.namespaceId ownerNs ownerDeclaration
          ownerCompiled.params row codecs types twins (ownProof := owner == handle)
        -- The data invariants owed where mutations end and values are built,
        -- read off the namespace's compilation view where the unit has any.
        let view? :=
          if unitCarriesInvariants unit then compileNamespaceAt? unit owner.namespaceId else none
        let mutations ← view?.mapM fun view => mutationConditions unit executable owner view ownerNs
          ownerDeclaration ownerCompiled.params row codecs types twins
        let constructions ← view?.mapM fun view => constructionConditions unit executable owner view
          ownerNs ownerDeclaration ownerCompiled.params codecs types twins
        let mutations := mutations.getD #[]
        let constructions := constructions.getD #[]
        -- The invariants owed where the function's own writes of memory end
        -- and its own calls return; an inlined callee's owe none in its
        -- caller.
        let writes ← if owner == handle then
            memoryConditions unit executable owner ownerNs ownerDeclaration ownerCompiled.params row codecs
              types twins
          else pure #[]
        let calls ← if owner == handle then
            afterCallConditions unit executable owner ownerNs ownerDeclaration ownerCompiled.params codecs
              types twins
          else pure #[]
        let invariants := invariants.map (fun (site, invariant, readsUnit) =>
          (site, invariant, readsUnit, "loopInvariant")) ++
          assertions.map (fun (site, condition, readsUnit) =>
            (site, condition, readsUnit, "assertion")) ++
          mutations.map (fun (site, condition, readsUnit) =>
            (site, condition, readsUnit, "mutationEnd")) ++
          constructions.map (fun (site, condition, readsUnit) =>
            (site, condition, readsUnit, "construction")) ++
          writes.map (fun (site, condition, readsUnit) =>
            (site, condition, readsUnit, "memoryWritten")) ++
          calls.map fun (site, condition, readsUnit) =>
            (site, condition, readsUnit, "afterCall")
        invariants.mapM fun (site, invariant, readsUnit, kind) =>
          return (site, owner, ← mkLambdaFVars (← executableBinders executable skolems) invariant,
            readsUnit, kind)
    let mut loopPairs : Array Term := #[]
    for (site, owner, invariant, _, kind) in invariants do
      let name := artifacts ++ Name.mkSimple s!"{kind}_{site}"
      liftTermElabM (addAbbrev name invariant)
      -- The invariant's skolem instance is the loop's own: an inlined generic
      -- callee runs under its frame's, so the closer applies it, at the unit
      -- and frame its marker names, not the ambient instance an elaborated
      -- constant would take. The executable unit is the theorem's.
      let applied ← `(term| fun (_ : LeanerIR.Validation.ValidatedUnit)
          (Θ : LeanerIR.Proofs.Denote.Skolems $unitTerm) => @$(rootIdent name) $unitTerm executable Θ)
      loopPairs := loopPairs.push
        (← `(term| ($(Syntax.mkNumLit (toString site)), $(← handleSyntax owner), $applied)))
    liftTermElabM (publishCompiled segments unit function handle compiled)
    let pattern ← argumentPattern unit declaration compiled.params
    let budgetValue := (heartbeatBudget? declaration).getD (leaner.verifyHeartbeats.get (← getOptions))
    let budget := Syntax.mkNumLit (toString budgetValue)
    let familyBinders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) ← if generic then
        pure #[← `(bracketedBinder| {Θ : LeanerIR.Proofs.Denote.Skolems $unitTerm}),
          ← `(bracketedBinder| {typeInstantiation : Array (LeanerIR.TypeId × LeanerIR.TypeId)})]
      else pure #[]
    let frameBinders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) ← if framed then
        pure #[← `(bracketedBinder| (frame : LeanerIR.Proofs.Denote.FrameOf $unitTerm
          $handleTerm $(Syntax.mkNatLit arity) typeInstantiation))]
      else pure #[]
    let familyBinders := familyBinders ++ frameBinders
    let frameArgs : Array Term ← if framed then pure #[← `(term| frame)] else pure #[]
    let instantiation ← if generic then `(term| typeInstantiation) else `(term| #[])
    let contract ← atUnit segments unit (typedContractName segments function) typedContract none
      (if isGeneric declaration then #[instantiation] else #[])
    -- A member of a cycle is verified over a meaning per member satisfying
    -- its contract: at the runtime family, or at every family and
    -- instantiation.
    let selfBinders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) ←
      cycle.flatMapM fun (member, name) => do
        let memberArtifacts := Name.str namespaceName name
        let memberParams := rootIdent (memberArtifacts ++ `params)
        let memberShape := rootIdent (memberArtifacts ++ `shape)
        let memberContract := rootIdent (typedContractName segments name)
        if family then
          let atFamily ← atUnit segments unit (typedContractName segments name) memberContract
            (some (← `(term| family)))
            (← if (declarationOf? unit member).any isGeneric then
              pure #[← `(term| instantiation)] else pure #[])
          pure #[← `(bracketedBinder| ($(cycleSelf name) :
                LeanerIR.Proofs.Denote.SelfFamily $unitTerm $memberParams $memberShape)),
            ← `(bracketedBinder| ($(cycleSelfVerified name) :
                ∀ (family : LeanerIR.Proofs.Denote.Skolems $unitTerm)
                  (instantiation : Array (LeanerIR.TypeId × LeanerIR.TypeId)),
                LeanerIR.Proofs.Satisfies ($(cycleSelf name) family instantiation) $atFamily))]
        else
          -- At the runtime frame, spelled out: binders precede the statement's.
          pure #[← `(bracketedBinder| ($(cycleSelf name) :
                @LeanerIR.Proofs.Denote.HList
                  (LeanerIR.Proofs.Denote.Carriers.runtime $unitTerm) $memberParams →
                LeanerIR.Proofs.Denote.Comp $unitTerm
                  (@LeanerIR.Proofs.Denote.ResultShape.carrier
                    (LeanerIR.Proofs.Denote.Carriers.runtime $unitTerm) $memberShape))),
            ← `(bracketedBinder| ($(cycleSelfVerified name) :
                LeanerIR.Proofs.Satisfies $(cycleSelf name)
                  $(← atUnit segments unit (typedContractName segments name) memberContract
                    (some (← `(term| ($(mkCIdent ``LeanerIR.Proofs.Denote.Skolems.runtime) $unitTerm)))))))]
    -- The readings of the function with and without its in-body assumptions,
    -- which the hypothesis that they hold relates.
    let assumptionsIdent := mkIdent (Name.mkSimple s!"assumptions_{function}")
    let assumedIdent := rootIdent (artifacts ++ `assumedMeaning)
    let tableIdent := rootIdent (artifacts ++ `assumptions)
    if ownAssumptions then
      let table ← liftTermElabM <| withExecutableSkolems fun executable skolems => do
        let codecs := mkApp (mkConst ``Carriers.codec) (← Contract.frameCarriers skolems)
        let types ← Contract.frameTypes skolems
        mkLambdaFVars (← executableBinders executable skolems) (← assumptionTable unit executable
          handle.namespaceId ns declaration compiled.params (compiled.params ++ compiled.locals)
          codecs types twins)
      liftTermElabM (addAbbrev (artifacts ++ `assumptions) table)
      elabCommand (← `(command|
        def $(rootIdent (artifacts ++ `plainMeaning))
            (executable : LeanerIR.Validation.ExecutableUnit $unitTerm)
            [LeanerIR.Proofs.Denote.Skolems $unitTerm]
            (typeInstantiation : Array (LeanerIR.TypeId × LeanerIR.TypeId)) :=
          fun (args : LeanerIR.Proofs.Denote.HList $paramsTerm) => LeanerIR.Proofs.Spec.bind
            (LeanerIR.Proofs.Denote.Term.denote
              (LeanerIR.Proofs.Denote.closedMeanings executable typeInstantiation)
              (ρ := $shapeTerm) (Γ := $row) $bodyIdent
              (LeanerIR.Proofs.Denote.initialEnv $paramsTerm $localsTerm args))
            (LeanerIR.Proofs.Denote.ResultShape.finish (Γ := $row) $mutablesTerm $shapeTerm)))
      elabCommand (← `(command|
        def $assumedIdent (executable : LeanerIR.Validation.ExecutableUnit $unitTerm)
            [LeanerIR.Proofs.Denote.Skolems $unitTerm]
            (typeInstantiation : Array (LeanerIR.TypeId × LeanerIR.TypeId)) :=
          fun (args : LeanerIR.Proofs.Denote.HList $paramsTerm) => LeanerIR.Proofs.Spec.bind
            LeanerIR.Proofs.Spec.get fun start => LeanerIR.Proofs.Spec.bind
              (LeanerIR.Proofs.Denote.Term.denote
                ({ LeanerIR.Proofs.Denote.closedMeanings executable typeInstantiation with
                  assumption := $tableIdent executable args start } :
                  LeanerIR.Proofs.Denote.Meanings executable)
                (ρ := $shapeTerm) (Γ := $row) $bodyIdent
                (LeanerIR.Proofs.Denote.initialEnv $paramsTerm $localsTerm args))
              (LeanerIR.Proofs.Denote.ResultShape.finish (Γ := $row) $mutablesTerm $shapeTerm)))
    let nativeBinders ← natives.mapM (nativeBinderOf unit segments)
    let nativeArgs : Array Term := natives.map fun entry =>
      ⟨mkIdent (Name.mkSimple s!"native_{entry.2}")⟩
    let nativeBinders := nativeBinders ++ (← assumptionDeps.mapM (assumptionBinderOf segments unit)) ++
      (← trustDeps.mapM trustBinderOf)
    let nativeArgs := nativeArgs ++ assumptionArgs assumptionDeps ++ trustArgs trustDeps
    -- A theorem assuming typing takes what the natives preserve: typing,
    -- and agreement up to the loans they mint.
    -- A theorem resolving `result_of` of a known function value takes that
    -- the unit's runs end, with the typing its runs rest on.
    let assumesTermination := Contract.assumesTermination unit ⟨namespaceIndex⟩ ns declaration
    let assumesTyping := Contract.assumesTyping unit ⟨namespaceIndex⟩ ns declaration ||
      assumesTermination
    let (nativeBinders, nativeArgs) ← if assumesTyping then do
        let typing := mkIdent `nativesTyped
        let shifts := mkIdent `nativesShift
        pure (nativeBinders ++
            #[← `(bracketedBinder| ($typing : LeanerIR.NativesTyped executable)),
              ← `(bracketedBinder| ($shifts : LeanerIR.NativesShift executable))],
          nativeArgs ++ #[(⟨typing⟩ : Term), ⟨shifts⟩])
      else pure (nativeBinders, nativeArgs)
    let (nativeBinders, nativeArgs) ← if assumesTermination then do
        let terminating := mkIdent `leanerTerminating
        pure (nativeBinders.push
            (← `(bracketedBinder| ($terminating : LeanerIR.Proofs.Terminating executable))),
          nativeArgs.push ⟨terminating⟩)
      else pure (nativeBinders, nativeArgs)
    -- A function without type parameters is stated and proved at the runtime
    -- frame of the executable's unit.
    let runtimeFrame ← `(term| ($(mkCIdent ``LeanerIR.Proofs.Denote.Skolems.runtime) $unitTerm))
    let atRuntime (statement : Term) : CommandElabM Term :=
      if generic then pure statement
      else `(term| letI := $runtimeFrame
          $statement)

    -- A cycle's meanings route each member, spelled out as `cycleMeanings`
    -- unfolds at the member's slot, so that the normalizer meets the routes
    -- directly.  At the runtime family, calls with type arguments stay
    -- closed.
    let meanings ← if cycle.isEmpty then
        `(term| LeanerIR.Proofs.Denote.closedMeanings executable $instantiation)
      else do
        let routes ← cycle.foldrM (init := ← `(term|
            LeanerIR.Proofs.Denote.propheticMeaning executable #[]))
          fun (member, name) rest => do
            let memberArtifacts := Name.str namespaceName name
            let self ← if family then `(term| ($(cycleSelf name) Θ typeInstantiation))
              else `(term| $(cycleSelf name))
            `(term| LeanerIR.Proofs.Denote.routeMeaning $(← handleSyntax member)
              $(rootIdent (memberArtifacts ++ `params)) $(rootIdent (memberArtifacts ++ `shape))
              $self $rest)
        let closed ← `(term| LeanerIR.Proofs.Denote.closedGeneric executable $instantiation)
        let generics ← if !family then pure closed
          else cycle.foldrM (init := closed) fun (member, name) rest => do
            let memberArtifacts := Name.str namespaceName name
            `(term| LeanerIR.Proofs.Denote.routeGeneric $(← handleSyntax member)
              $(rootIdent (memberArtifacts ++ `params)) $(rootIdent (memberArtifacts ++ `shape))
              $(cycleSelf name)
              (fun callee typeArgs => LeanerIR.Proofs.Denote.frameInstantiation $unitTerm
                callee $instantiation typeArgs)
              $rest)
        `(term| (⟨$routes, $generics, $instantiation, fun _ => none⟩ :
          LeanerIR.Proofs.Denote.Meanings executable))
    -- An authored script proves the obligations the closer leaves. A
    -- contract binding a state label has the closer keep its program points.
    let mut flags : Array (TSyntax `closeFlag) := #[]
    if script?.isSome then flags := flags.push (← `(closeFlag| residual))
    if Contract.contractBindsStateLabel unit ⟨namespaceIndex⟩ declaration then
      flags := flags.push (← `(closeFlag| labeled))
    let unrollBound? := declaration.pragmas.findSome? fun
      | .assign "unroll" (.constant (.integer bound)) _ =>
          if 0 ≤ bound then some bound.toNat else none
      | _ => none
    let unrollPairs ← match unrollBound?, declaration.body with
      | some bound, .structured root => (loopSpecifications ns root).mapM fun (site, _) =>
          `(term| ($(quote (loopSite namespaceIndex site.index)), $(quote bound)))
      | _, _ => pure #[]
    -- An obligation no clause locates is reported at the function.
    let closeTactic ← withRef reference `(tactic| leaner_denote_close $flags* [$loopPairs,*]
      with [$calleePairs,*] using [$instantiationCertificates,*] unrolling [$unrollPairs,*])
    -- A loop invariant reading `old` needs the function's start: the
    -- arguments and state are recorded as a hypothesis the closer finds.
    let readsStart : Bool := match declaration.body with
      | .structured root => (loopSpecifications ns root ++
            bodySpecifications ns root (· matches .assertion | .apply)).any fun (_, block) =>
          block.conditions.any fun condition =>
            (condition.kind matches .loopInvariant | .assertion | .apply | .split) &&
              (!(oldReferencedLocals ns condition.expression).isEmpty ||
                readsEntryState ns condition.expression)
      | _ => false
    -- The hypotheses the introduction names and the proof below uses share
    -- one spelling across the two quotations.
    let permitted := mkIdent `permitted
    let assumed := mkIdent `assumed
    let introduction ← if readsStart then
        `(tactic| (intro leanerArguments initialState $assumed:ident $permitted:ident
                   have := LeanerIR.Proofs.Denote.FunctionStart.intro $handleTerm leanerArguments
                     initialState
                   rcases leanerArguments with $pattern:rcasesPat))
      else `(tactic| rintro $pattern:rcasesPat initialState $assumed:ident $permitted:ident)
    -- A theorem assuming typing types global memory by its unit's resource
    -- types, which the leaves keep.
    let unitFact ← if assumesTyping then do
        let resources ← ensureResourcesTyped segments unit
        -- Where the unit's readings agree, a function value is typed by its
        -- carrier.
        let resourcesFact ← `(tactic|
          have $(mkIdent `leanerResources) : LeanerIR.Proofs.ResourcesTyped $unitTerm :=
            $(rootIdent resources))
        match ← ensureTypesAgree segments unit with
        | some agreement =>
            `(tactic| (have $(mkIdent `leanerAgree) : LeanerIR.Proofs.TypesAgree $unitTerm :=
                         $(rootIdent agreement)
                       $resourcesFact:tactic))
        | none => pure resourcesFact
      else `(tactic| skip)
    let scriptTactic ← match script? with
      | some script => `(tactic| ($script:tacticSeq))
      | none => `(tactic| skip)
    -- Under its in-body assumptions: the reading that assumes them is
    -- verified, and the hypothesis that they hold carries the contract over.
    let assumptionsTactic ← if ownAssumptions then
        `(tactic| refine LeanerIR.Proofs.satisfies_of_assumptions $assumptionsIdent ?_)
      else `(tactic| skip)
    let assumptionLemmas : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) ← if ownAssumptions then
        pure #[← `(Lean.Parser.Tactic.simpLemma| $assumedIdent:ident),
          ← `(Lean.Parser.Tactic.simpLemma| $tableIdent:ident),
          ← `(Lean.Parser.Tactic.simpLemma| LeanerIR.Proofs.wp_bind_get)]
      else pure #[]
    -- The signatures of the cycle's members, which route their calls.
    let memberSignatures : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) ←
      cycle.flatMapM fun (_, name) => do
        let memberArtifacts := Name.str namespaceName name
        pure #[← `(Lean.Parser.Tactic.simpLemma| $(rootIdent (memberArtifacts ++ `params)):ident),
          ← `(Lean.Parser.Tactic.simpLemma| $(rootIdent (memberArtifacts ++ `shape)):ident)]
    let memberSignatures := memberSignatures ++ assumptionLemmas
    -- A function without type parameters is proved at the runtime family,
    -- a closed term the normalizer's caches keep.
    -- A scripted theorem's messages land on the script: an unsolved
    -- obligation is reported at the authored proof.
    let typedStatement ← atRuntime (← `(term|
          LeanerIR.Proofs.Satisfies
            (fun args => LeanerIR.Proofs.Spec.bind
              (LeanerIR.Proofs.Denote.Term.denote $meanings (ρ := $shapeTerm) (Γ := $row)
                $bodyIdent (LeanerIR.Proofs.Denote.initialEnv $paramsTerm $localsTerm args))
              (LeanerIR.Proofs.Denote.ResultShape.finish (Γ := $row) $mutablesTerm $shapeTerm))
            $contract))
    let typedCommand ← withRef (script?.map (·.raw) |>.getD reference) do `(command|
      set_option Elab.async false in
      set_option maxHeartbeats $budget:num in
      theorem $(mkIdent typedVerified)
          {registry : LeanerIR.Validation.SemanticsRegistry}
          {executable : LeanerIR.Validation.ExecutableUnit $unitTerm}
          (prepared : LeanerIR.Validation.prepareExecution registry
            $(mkIdent unitDefinition) = .ok executable) $familyBinders* $nativeBinders*
            $selfBinders* : $typedStatement := by
        $unitFact:tactic
        $assumptionsTactic:tactic
        apply LeanerIR.Proofs.satisfies_of_wp
        $introduction:tactic
        all_goals
        (simp only [$bodyIdent:ident, $row:ident, $paramsTerm:ident, $localsTerm:ident,
          $shapeTerm:ident, $mutablesTerm:ident, $typedContract:ident, lir_denote_norm,
          reduceCtorEq, Nat.reduceEqDiff, String.reduceBEq, String.reduceEq, String.reduceBNe,
          String.reduceNe, $memberSignatures,*] at $assumed:ident $permitted:ident ⊢
         all_goals leaner_denote_normalize at $permitted:ident ⊢
         all_goals $closeTactic:tactic)
        $scriptTactic:tactic)
    let typedCommand ← if selectsBitVectors declaration then
        `(command| set_option leaner.bitVectors true in $typedCommand)
      else pure typedCommand
    let loggedBefore := (← get).messages.reportedPlusUnreported.size
    stageLog s!"{function}: theorem start"
    let heartbeatsBefore ← IO.getNumHeartbeats
    Perf.withPhase .verification <|
      Perf.measure s!"{namespaceName}::{function} typed" (base ++ `typedVerified)
        (elabCommand typedCommand)
    stageLog s!"{function}: theorem done, {((← IO.getNumHeartbeats) - heartbeatsBefore) / 1000000}M heartbeats"
    if countErrors (← get).messages > errorsBefore then
      let logged := (← get).messages.reportedPlusUnreported.toList.drop loggedBefore
      let overBudget := logged.any fun message =>
        message.data.hasTag (· == `runtime.maxHeartbeats)
      throwErrorAt reference (failureMessage ns function script?.isSome overBudget budgetValue)
    -- A member of a cycle concludes with the whole cycle.
    unless cycle.isEmpty do return (natives, assumptionDeps, trustDeps)
    -- Elaborated synchronously, as the typed theorem is, so that an error
    -- is counted before the artifacts are accepted.
    -- The proof names the frame its statement is at.
    let frame ← if generic then `(term| Θ) else pure runtimeFrame
    let semanticsStatement ← atRuntime (← `(term|
          LeanerIR.Proofs.Satisfies
            (LeanerIR.Proofs.Denote.propheticMeaning executable $instantiation $handleTerm $paramsTerm
              $shapeTerm)
            $contract))
    let semanticsCommand ← `(command|
      set_option Elab.async false in
      theorem $(mkIdent (typedSemanticsVerifiedName segments function))
          {registry : LeanerIR.Validation.SemanticsRegistry}
          {executable : LeanerIR.Validation.ExecutableUnit $unitTerm}
          (prepared : LeanerIR.Validation.prepareExecution registry
            $(mkIdent unitDefinition) = .ok executable) $familyBinders* $nativeBinders* :
          $semanticsStatement := by
        have compiledAt : LeanerIR.Proofs.Denote.compileFunction $unitTerm $handleTerm =
            .ok $compiledIdent := $compiledEqIdent
        exact @LeanerIR.Proofs.Denote.satisfies_propheticMeaning _ executable $frame
          $instantiation $handleTerm $compiledIdent compiledAt _
          ($(mkIdent typedVerified) prepared $frameArgs* $nativeArgs*))
    let verifiedCommand ← if isGeneric declaration then
        -- A generic function's runs are at the instantiations calls give it:
        -- its theorem holds at every one coherent with the frame its type
        -- arguments induce.
        let publicContract ← atUnit segments unit (contractName segments function) publicContract
          none #[← `(term| θ), ← `(term| typeInstantiation)]
        let induced ← `(term| (LeanerIR.Proofs.Denote.Skolems.instantiate θ $runtimeFrame))
        `(command|
          set_option Elab.async false in
          theorem $(mkIdent (verifiedName segments function))
              {registry : LeanerIR.Validation.SemanticsRegistry}
              {executable : LeanerIR.Validation.ExecutableUnit $unitTerm}
              (prepared : LeanerIR.Validation.prepareExecution registry
                $(mkIdent unitDefinition) = .ok executable)
              (globalsPreserved : LeanerIR.Proofs.Denote.GlobalsPreserved executable)
              $nativeBinders* (θ : LeanerIR.Proofs.Denote.TypeArgs) (free : θ.1.refFree = true)
              (typeInstantiation : Array (LeanerIR.TypeId × LeanerIR.TypeId)) $frameBinders*
              (coherent : @LeanerIR.Proofs.Denote.Coherent $unitTerm $induced $handleTerm
                typeInstantiation) :
              LeanerIR.Proofs.SatisfiesFunctionAt executable $handleTerm typeInstantiation
                $publicContract :=
            LeanerIR.Proofs.Denote.satisfies_generic_at executable globalsPreserved $handleTerm θ
              free typeInstantiation $paramsTerm $shapeTerm _ coherent
              (@$(mkIdent (typedSemanticsVerifiedName segments function)) _ _ prepared $induced
                typeInstantiation $frameArgs* $nativeArgs*))
      else
        let publicContract ← atUnit segments unit (contractName segments function) publicContract
        `(command|
          set_option Elab.async false in
          theorem $(mkIdent (verifiedName segments function))
              {registry : LeanerIR.Validation.SemanticsRegistry}
              {executable : LeanerIR.Validation.ExecutableUnit $unitTerm}
              (prepared : LeanerIR.Validation.prepareExecution registry
                $(mkIdent unitDefinition) = .ok executable)
              (globalsPreserved : LeanerIR.Proofs.Denote.GlobalsPreserved executable)
              $nativeBinders* :
              LeanerIR.Proofs.SatisfiesFunction executable $handleTerm $publicContract :=
            LeanerIR.Proofs.Denote.satisfies_prophetic executable globalsPreserved $handleTerm
              $paramsTerm $shapeTerm
              _
              ($(mkIdent (typedSemanticsVerifiedName segments function)) prepared $nativeArgs*))
    Perf.withPhase .verification <|
      Perf.measure s!"{namespaceName}::{function} transport" (base ++ `verified) do
        elabCommand semanticsCommand
        elabCommand verifiedCommand
    if countErrors (← get).messages > errorsBefore then
      throwErrorAt reference "leaner verification failed"
    modifyEnv fun env => completedDenotations.tag env (base ++ `typedVerified)
    modifyEnv fun env => nativeDependencies.addEntry env (base, natives)
    modifyEnv fun env => assumptionDependencies.addEntry env (base, assumptionDeps)
    modifyEnv fun env => lemmaTrustDependencies.addEntry env (base, trustDeps)
    -- Audit fresh proofs as well as cached ones. On failure the existing
    -- rollback removes both the artifacts and the completion tag.
    withRef reference <| requireNativeArtifacts base (selectsBitVectors declaration)
    return (natives, assumptionDeps, trustDeps)
  catch failure =>
    modify fun state => { saved with messages := state.messages }
    throw failure

/-- The functions of the cycle of calls through `handle`, `handle` first:
those it calls, directly or not, that call it again.  Empty when no call of
`handle` reaches it. -/
private def callCycle (unit : ValidatedUnit) (handle : FunctionHandle) :
    Array FunctionHandle := Id.run do
  let callees (caller : FunctionHandle) : Array FunctionHandle :=
    match compileFunction unit caller with
    | .ok compiled => compiled.body.callees #[]
    | .error _ => #[]
  let mut reached : Array FunctionHandle := #[]
  let mut worklist := callees handle
  while let some next := worklist.back? do
    worklist := worklist.pop
    if reached.contains next then continue
    reached := reached.push next
    worklist := worklist ++ callees next
  unless reached.contains handle do return #[]
  let mut members := #[handle]
  let mut grown := true
  while grown do
    grown := false
    for node in reached do
      if !members.contains node && (callees node).any members.contains then
        members := members.push node
        grown := true
  return members

/-- Verify the members of a cycle of calls together, a function calling
itself being a cycle of one: each over the members' meanings at every slot,
then all of them by fixed-point induction over the slots.  Without a generic
member every member is proved at the runtime family, the other slots
standing vacuous; with one, every member at every skolem family and type
instantiation, the runtime slots standing vacuous. -/
private def verifyCycle (reference : Syntax) (segments : Array String)
    (members : Array (FunctionHandle × String))
    (scripts : String → Option (TSyntax ``Lean.Parser.Tactic.tacticSeq)) :
    CommandElabM Unit := do
  let namespaceName := pathName segments
  let some unit := LeanerLang.registeredUnit? (← getEnv) namespaceName
    | throwErrorAt reference s!"unknown Leaner namespace `{namespaceName}`"
  let unitDefinition ← ensureUnitDefinition segments unit
  let unitTerm ← unitTermOf segments unit
  discard <| ensureUnitDefinitions segments unit
  let isGenericMember (member : FunctionHandle) := (declarationOf? unit member).any isGeneric
  let memberArity (member : FunctionHandle) := ((declarationOf? unit member).map fun declaration =>
    (declaration.signature.generics.filter (·.kind == .typeArg)).size).getD 0
  let family := members.any (isGenericMember ·.1)
  let saved ← get
  try
    -- Each member's typed theorem speaks of every member's signature and
    -- contract.
    for (member, name) in members do
      let some (namespaceIndex, ns, _, declaration) := findFunction? unit name
        | throwErrorAt reference s!"unknown function `{name}` in `{namespaceName}`"
      let some (params, result) := nativeSignature? unit ⟨namespaceIndex⟩ declaration
        | throwErrorAt reference m!"no denotation for `{name}`: its signature is not native"
      ensureContracts segments name unit namespaceIndex ns declaration params result
      let compiled ← match compileFunction unit member with
        | .ok compiled => pure compiled
        | .error reason => throwErrorAt reference m!"no denotation for `{name}`: {reason}"
      liftTermElabM (publishCompiled segments unit name member compiled)
    let mut assumed : Array (Array (FunctionHandle × String) × Array (FunctionHandle × String) ×
      Array Name) := #[]
    for (_, name) in members do
      assumed := assumed.push
        (← verifyMember reference segments name (scripts name) members family)
    let mut natives : Array (FunctionHandle × String) := #[]
    let mut assumptionDeps : Array (FunctionHandle × String) := #[]
    let mut trustDeps : Array Name := #[]
    for (memberNatives, memberAssumptions, memberTrust) in assumed do
      for entry in memberNatives do
        unless natives.any (·.1 == entry.1) do natives := natives.push entry
      for entry in memberAssumptions do
        unless assumptionDeps.any (·.1 == entry.1) do assumptionDeps := assumptionDeps.push entry
      for name in memberTrust do
        unless trustDeps.contains name do trustDeps := trustDeps.push name
    let nativeBinders ← natives.mapM (nativeBinderOf unit segments)
    let nativeBinders := nativeBinders ++ (← assumptionDeps.mapM (assumptionBinderOf segments unit)) ++
      (← trustDeps.mapM trustBinderOf)
    let nativeArgs (entries : Array (FunctionHandle × String) × Array (FunctionHandle × String) ×
        Array Name) : Array Term :=
      entries.1.map (fun entry => ⟨mkIdent (Name.mkSimple s!"native_{entry.2}")⟩) ++
        assumptionArgs entries.2.1 ++ trustArgs entries.2.2
    let artifactsOf (name : String) := Name.str namespaceName name
    let compiledAt (name : String) := mkIdent (Name.mkSimple s!"compiledAt_{name}")
    let membersTerm ← cycleMembersSyntax segments members
    let indices ← (List.range members.size).toArray.mapM cycleIndexSyntax
    let compiledHaves ← members.mapM fun (member, name) => do
      `(tactic| have $(compiledAt name) : LeanerIR.Proofs.Denote.compileFunction $unitTerm
            $(← handleSyntax member) = .ok $(rootIdent (compiledName segments name)) :=
          $(rootIdent (compiledEqName segments name)))
    let compiledAlternatives ← (members.zip indices).mapM fun ((_, name), index) =>
      `(Lean.Parser.Term.matchAltExpr| | $index => $(compiledAt name))
    let contractAlternatives ← (members.zip indices).mapM fun ((member, name), index) => do
      let typedContract := rootIdent (typedContractName segments name)
      let contract : Term ← if !family then
          atUnit segments unit (typedContractName segments name) typedContract
            (some (← `(term| ($(mkCIdent ``LeanerIR.Proofs.Denote.Skolems.runtime) $unitTerm))))
        else if isGenericMember member then do
          let atFamily ← atUnit segments unit (typedContractName segments name) typedContract
            (some (← `(term| family))) #[← `(term| instantiation)]
          `(term| fun family instantiation => $atFamily)
        else do
          let atFamily ← atUnit segments unit (typedContractName segments name) typedContract
            (some (← `(term| family)))
          `(term| fun family _ => $atFamily)
      `(Lean.Parser.Term.matchAltExpr| | $index => $contract)
    let selvesVerified := mkIdent `selvesVerified
    let mut selfArguments : Array Term := #[]
    for index in indices do
      if family then
        selfArguments := selfArguments.push
          (← `(term| (fun family instantiation => $cycleSelves ⟨$index, some (family, instantiation)⟩)))
      else
        selfArguments := selfArguments.push (← `(term| ($cycleSelves ⟨$index, none⟩)))
      selfArguments := selfArguments.push (← `(term| ($selvesVerified $index)))
    let verifiedAlternatives ← ((members.zip indices).zip assumed).mapM
      fun (((_, name), index), memberNatives) => do
        let typedVerified := mkIdent (typedVerifiedName segments name)
        let proof ← if family then
            `(term| fun family instantiation => @$typedVerified _ _ prepared family instantiation
              $(nativeArgs memberNatives)* $selfArguments*)
          else `(term| $typedVerified prepared $(nativeArgs memberNatives)* $selfArguments*)
        `(Lean.Parser.Term.matchAltExpr| | $index => $proof)
    for ((member, name), index) in members.zip indices do
      let handleTerm ← handleSyntax member
      let params := rootIdent (artifactsOf name ++ `params)
      let shape := rootIdent (artifactsOf name ++ `shape)
      let typedContract := rootIdent (typedContractName segments name)
      let generic := isGenericMember member
      -- A generic member's theorem holds at every family and instantiation;
      -- any other at the runtime family.
      let familyBinders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) ← if generic then
          pure #[← `(bracketedBinder| {Θ : LeanerIR.Proofs.Denote.Skolems $unitTerm}),
            ← `(bracketedBinder| {typeInstantiation : Array (LeanerIR.TypeId × LeanerIR.TypeId)})]
        else pure #[]
      let instantiation ← if generic then `(term| typeInstantiation) else `(term| #[])
      let runtimeFrame ← `(term| ($(mkCIdent ``LeanerIR.Proofs.Denote.Skolems.runtime) $unitTerm))
      let contract ← atUnit segments unit (typedContractName segments name) typedContract none
        (if generic then #[instantiation] else #[])
      let cycleProof ← if !family then
          `(term| @LeanerIR.Proofs.Denote.satisfies_cycle_runtime _ executable $runtimeFrame #[]
            $membersTerm
            (fun $compiledAlternatives:matchAlt*)
            (fun $contractAlternatives:matchAlt*)
            (fun $cycleSelves $selvesVerified index => match index with
              $verifiedAlternatives:matchAlt*)
            $index)
        else
          `(term| @LeanerIR.Proofs.Denote.satisfies_cycle_family
            _ executable $runtimeFrame #[] $membersTerm
            (fun $compiledAlternatives:matchAlt*)
            (fun $contractAlternatives:matchAlt*)
            (fun $cycleSelves $selvesVerified index => match index with
              $verifiedAlternatives:matchAlt*)
            $index $(← if generic then `(term| Θ) else pure runtimeFrame)
            $(← if generic then `(term| typeInstantiation) else `(term| #[])))
      -- A member without type parameters is stated and proved at the runtime
      -- frame of the executable's unit.
      let statement ← `(term|
            LeanerIR.Proofs.Satisfies
              (LeanerIR.Proofs.Denote.propheticMeaning executable $instantiation $handleTerm $params
                $shape)
              $contract)
      let statement ← if generic then pure statement
        else `(term| letI := $runtimeFrame
            $statement)
      let semanticsTheorem ← `(command|
        set_option Elab.async false in
        theorem $(mkIdent (typedSemanticsVerifiedName segments name))
            {registry : LeanerIR.Validation.SemanticsRegistry}
            {executable : LeanerIR.Validation.ExecutableUnit $unitTerm}
            (prepared : LeanerIR.Validation.prepareExecution registry
              $(mkIdent unitDefinition) = .ok executable) $familyBinders* $nativeBinders* :
            $statement := by
          $[$compiledHaves]*
          exact $cycleProof)
      let semanticsCommand := semanticsTheorem
      let memberNativeArgs := nativeArgs (natives, assumptionDeps, trustDeps)
      let verifiedCommand ← if isGenericMember member then
          let publicContract ← atUnit segments unit (contractName segments name)
            (rootIdent (contractName segments name)) none
            #[← `(term| θ), ← `(term| typeInstantiation)]
          let induced ← `(term| (LeanerIR.Proofs.Denote.Skolems.instantiate θ $runtimeFrame))
          `(command|
            set_option Elab.async false in
            theorem $(mkIdent (verifiedName segments name))
                {registry : LeanerIR.Validation.SemanticsRegistry}
                {executable : LeanerIR.Validation.ExecutableUnit $unitTerm}
                (prepared : LeanerIR.Validation.prepareExecution registry
                  $(mkIdent unitDefinition) = .ok executable)
                (globalsPreserved : LeanerIR.Proofs.Denote.GlobalsPreserved executable)
                $nativeBinders* (θ : LeanerIR.Proofs.Denote.TypeArgs) (free : θ.1.refFree = true)
                (typeInstantiation : Array (LeanerIR.TypeId × LeanerIR.TypeId))
                (_frame : LeanerIR.Proofs.Denote.FrameOf $unitTerm $handleTerm
                  $(Syntax.mkNatLit (memberArity member)) typeInstantiation)
                (coherent : @LeanerIR.Proofs.Denote.Coherent $unitTerm $induced $handleTerm
                  typeInstantiation) :
                LeanerIR.Proofs.SatisfiesFunctionAt executable $handleTerm typeInstantiation
                  $publicContract :=
              LeanerIR.Proofs.Denote.satisfies_generic_at executable globalsPreserved $handleTerm θ
                free typeInstantiation $params $shape _ coherent
                (@$(mkIdent (typedSemanticsVerifiedName segments name)) _ _ prepared $induced
                  typeInstantiation $memberNativeArgs*))
        else
          `(command|
            set_option Elab.async false in
            theorem $(mkIdent (verifiedName segments name))
                {registry : LeanerIR.Validation.SemanticsRegistry}
                {executable : LeanerIR.Validation.ExecutableUnit $unitTerm}
                (prepared : LeanerIR.Validation.prepareExecution registry
                  $(mkIdent unitDefinition) = .ok executable)
                (globalsPreserved : LeanerIR.Proofs.Denote.GlobalsPreserved executable)
                $nativeBinders* :
                LeanerIR.Proofs.SatisfiesFunction executable $handleTerm
                  $(← atUnit segments unit (contractName segments name)
                    (rootIdent (contractName segments name))) :=
              LeanerIR.Proofs.Denote.satisfies_prophetic executable globalsPreserved $handleTerm
                $params $shape _
                ($(mkIdent (typedSemanticsVerifiedName segments name)) prepared
                  $memberNativeArgs*))
      let base := (← getCurrNamespace) ++ artifactsOf name
      let errorsBefore := countErrors (← get).messages
      Perf.withPhase .verification <|
        Perf.measure s!"{namespaceName}::{name} transport" (base ++ `verified) do
          elabCommand semanticsCommand
          elabCommand verifiedCommand
      if countErrors (← get).messages > errorsBefore then
        throwErrorAt reference "leaner verification failed"
    for (_, name) in members do
      let base := (← getCurrNamespace) ++ artifactsOf name
      modifyEnv fun env => completedDenotations.tag env (base ++ `typedVerified)
      modifyEnv fun env => nativeDependencies.addEntry env (base, natives)
      modifyEnv fun env => assumptionDependencies.addEntry env (base, assumptionDeps)
      modifyEnv fun env => lemmaTrustDependencies.addEntry env (base, trustDeps)
    for (member, name) in members do
      withRef reference <| requireNativeArtifacts ((← getCurrNamespace) ++ artifactsOf name)
        ((declarationOf? unit member).any selectsBitVectors)
  catch failure =>
    modify fun state => { saved with messages := state.messages }
    throw failure

/-- Verify one function through its denotation, with the other members of
its cycle of calls, if any, whose authored proofs `scripts` gives.  The
members are reported to `covering` before they are verified, succeeding or
failing together. -/
def verifyFunction (reference : Syntax) (segments : Array String) (function : String)
    (script? : Option (TSyntax ``Lean.Parser.Tactic.tacticSeq) := none)
    (scripts : String → Option (TSyntax ``Lean.Parser.Tactic.tacticSeq) := fun _ => none)
    (covering : Array String → CommandElabM Unit := fun _ => pure ()) :
    CommandElabM Unit := do
  let namespaceName := pathName segments
  let some unit := LeanerLang.registeredUnit? (← getEnv) namespaceName
    | throwErrorAt reference s!"unknown Leaner namespace `{namespaceName}`"
  let some (namespaceIndex, _, functionIndex, _) := findFunction? unit function
    | throwErrorAt reference s!"unknown function `{function}` in `{namespaceName}`"
  let handle : FunctionHandle := ⟨⟨namespaceIndex⟩, ⟨functionIndex⟩⟩
  let base := (← getCurrNamespace) ++ Name.str namespaceName function
  let cycle := if (← getEnv).contains (base ++ `typedVerified) then #[]
    else callCycle unit handle
  if cycle.isEmpty then
    discard <| verifyMember reference segments function script? #[]
    return
  let mut members : Array (FunctionHandle × String) := #[]
  for member in cycle do
    let some memberNs := unit.namespaces[member.namespaceId.index]?
      | throwErrorAt reference "a callee's namespace is out of range"
    let some memberDeclaration := memberNs.functions[member.functionId.index]?
      | throwErrorAt reference "a callee is out of range"
    let some name := memberNs.tables.names[memberDeclaration.name.index]?
      | throwErrorAt reference "a callee has no name"
    if member != handle then
      unless member.namespaceId == handle.namespaceId do
        throwErrorAt reference m!"`{function}` reaches itself through `{name.name}` of another \
          module"
      unless memberDeclaration.contract.loc.isSome do
        throwErrorAt reference m!"`{function}` reaches itself through `{name.name}`, which is \
          unspecified; a function on a cycle of calls is used through its contract, so specify \
          `{name.name}`"
      if automaticVerificationDisabled unit member.namespaceId memberNs memberDeclaration then
        throwErrorAt reference m!"`{function}` reaches itself through `{name.name}`, which sets \
          `pragma verify = false`; the members of a cycle of calls are verified together"
    members := members.push (member, name.name)
  covering (members.map (·.2))
  verifyCycle reference segments members fun name =>
    if name == function then script? else scripts name

/-! ## Commands -/

private partial def pathSegments (stx : Syntax) : Array String :=
  match stx with
  | .ident _ raw _ _ => #[raw.toString]
  | .atom _ value => if value == "::" then #[] else #[value]
  | .node _ _ arguments => arguments.flatMap pathSegments
  | _ => #[]

/-- The optional authored script of a `verify` form. -/
def scriptOfOptional (optional : Syntax) :
    Option (TSyntax ``Lean.Parser.Tactic.tacticSeq) :=
  match optional.getArgs with
  | #[_, script] => some ⟨script⟩
  | _ => none

/-- Verify one function of a registered unit from outside its module. A
qualified path names the module; a bare name must be declared by exactly one
registered module. -/
syntax (name := leanerVerifyCommand)
  "verify" leanerPath ("by" Lean.Parser.Tactic.tacticSeq)? : command

@[command_elab leanerVerifyCommand]
def elabLeanerVerify : CommandElab := fun stx => Perf.withPhase .certification do
  let some pathSyntax := stx[1]? | throwErrorAt stx "expected a path"
  let segments := pathSegments pathSyntax
  -- A module path may lead with a Move address alias; a namespace
  -- registered at the path as written takes precedence.
  let canonical := LeanerLang.canonicalMovePath (← getEnv) segments.pop
  let segments := if segments.size > 1 &&
      (LeanerLang.registeredUnit? (← getEnv) (pathName segments.pop)).isNone &&
      (LeanerLang.registeredUnit? (← getEnv) (pathName canonical)).isSome then
    canonical.push segments.back!
  else segments
  let script := scriptOfOptional (stx[2]?.getD .missing)
  let function := segments.back!
  if segments.size > 1 then
    return ← verifyFunction pathSyntax segments.pop function script
  let owners := (LeanerLang.registeredUnits (← getEnv)).filter fun (_, unit) =>
    (findFunction? unit function).isSome
  match owners with
  | [(owner, _)] =>
      let ownerSegments := owner.components.toArray.map (·.toString (escape := false))
      verifyFunction pathSyntax ownerSegments function script
  | [] => throwErrorAt pathSyntax s!"no Leaner module declares a function `{function}`"
  | _ =>
      let names := owners.map fun (owner, _) =>
        "::".intercalate (owner.components.map (·.toString (escape := false)))
      throwErrorAt pathSyntax
        s!"`{function}` is declared by several Leaner modules ({", ".intercalate names}); \
          qualify it with its module path"

/-- A function a module verifies: its name, the syntax errors are reported
at, and its authored proof, if any. -/
private structure VerificationTarget where
  function : String
  reference : Syntax
  script : Option (TSyntax ``Lean.Parser.Tactic.tacticSeq)

/-- The specified functions a function's proof uses through their verified
theorems: its callees with a specification, found through the unspecified
callees it inlines, and the specified targets of its closures. Natives and
callees returning a reference are not among them. -/
private def verifiedCallees (unit : ValidatedUnit)
    (function : FunctionHandle) : Array FunctionHandle := Id.run do
  let callees (handle : FunctionHandle) : Array FunctionHandle :=
    match compileFunction unit handle with
    | .ok compiled => compiled.body.callees #[]
    | .error _ => #[]
  let targetsOfClosures := closureTargets unit
  let specified (target : FunctionHandle) : Bool :=
    match unit.namespaces[target.namespaceId.index]? with
    | some targetNs => (targetNs.functions[target.functionId.index]?).any (·.contract.loc.isSome)
    | none => false
  let contractTargets (callee : FunctionHandle) : Array FunctionHandle :=
    match unit.namespaces[callee.namespaceId.index]? with
    | some calleeNs => match calleeNs.functions[callee.functionId.index]? with
        | some calleeDeclaration =>
            (contractClosureTargets unit callee.namespaceId calleeNs calleeDeclaration).filter
              specified
        | none => #[]
    | none => #[]
  let mut found := contractTargets function
  let mut visited := #[function]
  let mut worklist := callees function
  while let some callee := worklist.back? do
    worklist := worklist.pop
    if visited.contains callee then continue
    visited := visited.push callee
    let some ns := unit.namespaces[callee.namespaceId.index]? | continue
    let some declaration := ns.functions[callee.functionId.index]? | continue
    if declaration.body == .absent then
      found := found ++ (contractTargets callee).filter (!found.contains ·)
      continue
    if usedThroughContract unit callee ns declaration then
      -- A callee seen through its interface is assumed, not used by its theorem.
      unless hasInterfaceView declaration do found := found.push callee
      found := found ++ (contractTargets callee).filter (!found.contains ·)
    else
      -- What `ensures_of` and `aborts_of` of a closure entail is its
      -- target's theorem.
      if targetsOfClosures.contains callee && declaration.contract.loc.isSome then
        found := found.push callee
      worklist := worklist ++ callees callee
  return found

/-- Verify `target` after the module's targets among the specified callees
its proof uses, so a caller finds its callees' theorems whatever the order
of their declarations. A target already visited, on a cycle among them
included, is not entered again. -/
private partial def verifyInOrder (unit : ValidatedUnit) (segments : Array String)
    (targets : Array VerificationTarget) (visited covered : IO.Ref (Array String))
    (target : VerificationTarget) : CommandElabM Unit := do
  if (← visited.get).contains target.function then return
  visited.modify (·.push target.function)
  if let some (namespaceIndex, _, functionIndex, _) := findFunction? unit target.function then
    for callee in verifiedCallees unit ⟨⟨namespaceIndex⟩, ⟨functionIndex⟩⟩ do
      let key := functionKey unit callee
      if callee.namespaceId.index == namespaceIndex then
        if let some calleeTarget := targets.find? (·.function == key) then
          verifyInOrder unit segments targets visited covered calleeTarget
      else
        -- A callee of another module used through its contract is verified
        -- in this unit too: a theorem about its own unit does not carry over.
        let some (_, ns, _, declaration) := findFunction? unit key | continue
        unless automaticVerificationDisabled unit callee.namespaceId ns declaration do
          verifyInOrder unit segments targets visited covered
            ⟨key, target.reference, none⟩
  -- An authored proof sees the module's theorems by their short names.
  let moduleNamespace := pathName segments
  let openModule (scope : Scope) := { scope with
    openDecls := .simple moduleNamespace [] :: scope.openDecls }
  -- A member of a cycle verified from another member is not entered again.
  if (← covered.get).contains target.function then return
  let scripts (name : String) := (targets.find? (·.function == name)).bind (·.script)
  let covering (names : Array String) : CommandElabM Unit := covered.modify (· ++ names)
  let measuring ← Perf.measuring.get
  let errorsBefore ← if measuring then do pure (countErrors (← get).messages) else pure 0
  let messagesBefore ← if measuring then do
      pure (← get).messages.reportedPlusUnreported.size
    else pure 0
  try
    if targets.any (·.script.isSome) then
      withScope openModule
        (verifyFunction target.reference segments target.function target.script scripts covering)
    else verifyFunction target.reference segments target.function none scripts covering
  catch error =>
    -- A failure without a position of its own, such as a translation's, is
    -- reported at the target and names it.
    match error with
    | .error ref message =>
        if ref.getPos?.isSome then logException error
        else logErrorAt target.reference m!"leaner verification of `{target.function}` failed: \
          {message}"
    | _ => logException error
  if measuring then
    let errors := countErrors (← get).messages - errorsBefore
    let logged := (← get).messages.reportedPlusUnreported.toList.drop messagesBefore
    let timedOut := logged.any fun message =>
      message.severity == .error && message.data.hasTag (· == `runtime.maxHeartbeats)
    Perf.outcomes.modify (·.push {
      target := s!"{moduleNamespace}::{target.function}"
      status := if errors == 0 then "verified" else if timedOut then "timeout" else "rejected"
      errors })

/-- Elaborate `commands` in the module namespace `moduleNamespace`, taken
from the root: the namespace of the module's generated definitions. -/
private def withModuleNamespace (moduleNamespace : Name) (commands : CommandElabM Unit) :
    CommandElabM Unit := do
  modify fun state => { state with
    env := state.env.registerNamespace moduleNamespace
    scopes := { state.scopes.head! with header := "", currNamespace := moduleNamespace } ::
      state.scopes }
  pushScope
  activateScoped moduleNamespace
  try commands
  finally
    modify fun state => { state with scopes := state.scopes.drop 1 }
    popScope

/-- The function items of a namespace command, in source order. -/
private partial def functionItems (stx : Syntax) : Array Syntax :=
  stx.getArgs.foldl (fun found child =>
    if child.isOfKind ``leanerFunctionItem then found.push child
    else if child.isOfKind ``leanerTheoremItem then found
    else found ++ functionItems child) #[]

/-- The first identifier of a syntax tree. -/
private partial def firstIdentifier? (stx : Syntax) : Option Syntax :=
  if stx.isIdent then some stx else stx.getArgs.findSome? firstIdentifier?

/-- Whether a function's parameters, through a reference, its results, its
locals, or the values it constructs carry data invariants: what the Move
Prover checks of every function that handles such values. -/
private def handlesDataInvariants (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : Bool :=
  let direct (typeId : LeanerIR.TypeId) : Bool :=
    (ns.tables.types[typeId.index]?).any (Contract.hasDataInvariant unit)
  let carries (typeId : LeanerIR.TypeId) : Bool :=
    match ns.tables.types[typeId.index]? with
    | some (.reference reference) => direct reference.referent
    | some _ => direct typeId
    | none => false
  let constructs : Bool := match declaration.body with
    | .structured root => (bodyExpressionIds ns root).any fun id =>
        match ns.expressions[id.index]? with
        | some { kind := .operation (.call (.constructor _ _)) _ _ _, typeId, .. } => direct typeId
        | _ => false
    | _ => false
  declaration.signature.parameters.any (carries ·.typeUse.typeId) ||
    declaration.signature.results.any (carries ·.typeId) ||
    declaration.locals.any (direct ·.type.typeId) || constructs

/-- Whether a function's body states a condition of a kind `accept` admits. -/
private def bodyStates (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (accept : LeanerIR.ConditionKind → Bool) : Bool :=
  match declaration.body with
  | .structured root => !(bodySpecifications ns root accept).isEmpty
  | _ => false

/-- The registered modules whose invariants concern a module's unit, which
it links: those whose invariants read memory its functions reach, and those
registered in this file whose functions write memory its own invariants
read, whose functions it verifies against them. Iterated to the closure: a
linked writer's memory reaches further invariants. -/
private def invariantRelatedModules (environment : Environment) (unit : ValidatedUnit) :
    Array (Array String) := Id.run do
  let modulePath (module : ValidatedUnit) : Option (Array String) := do
    let ns ← module.namespaces[0]?
    module.tables.namespaces[ns.identity.index]? |>.map (·.segments)
  let some ownPath := modulePath unit | return #[]
  let modules := (registeredUnits environment).filterMap fun (name, module) => do
    let path ← modulePath module
    guard (pathName path == name && path != ownPath)
    pure (path, module)
  let inFile := (moduleUnits environment).map (·.1)
  let ownInvariants := Contract.namespaceInvariantMemory unit ⟨0⟩
  let mut reached := Contract.namespaceFunctionMemory unit ⟨0⟩ false
  let mut related : Array (Array String) := #[]
  for _ in [0:modules.length + 1] do
    let mut changed := false
    for (path, module) in modules do
      if related.contains path then continue
      let readsReached := (Contract.namespaceInvariantMemory module ⟨0⟩).any reached.contains
      let writesOwn := inFile.contains (pathName path) &&
        (Contract.namespaceFunctionMemory module ⟨0⟩ true).any ownInvariants.contains
      unless readsReached || writesOwn do continue
      related := related.push path
      changed := true
      if writesOwn then
        for resource in Contract.namespaceFunctionMemory module ⟨0⟩ false do
          unless reached.contains resource do reached := reached.push resource
    unless changed do break
  return related

/-- Elaborate a Leaner namespace command, then its theorems in its namespace,
then, with that namespace open, its `verify` items and its specified
functions without one, in source order. This registration shadows
the plain namespace elaborator. -/
@[command_elab leanerNamespaceCommand, command_elab leanerMoveModuleCommand,
  command_elab leanerRustNamespaceCommand]
def elaborateNamespaceWithVerification : CommandElab := fun stx =>
    Perf.withPhase .certification do
  LeanerLang.elaborateNamespaceWith invariantRelatedModules stx
  stageLog "module elaborated"
  -- Its module is registered at its canonical path.
  let stx ← match LeanerLang.canonicalMoveCommand (← getEnv) stx with
    | .ok (canonical, _) => pure canonical
    | .error (location, message) => throwErrorAt location message
  let some pathSyntax := stx.getArgs.find? (·.isOfKind ``leanerPathSyntax)
    | throwErrorAt stx "a Leaner namespace requires a path"
  -- A recursive specification function is a definition of the module.
  let segments := pathSegments pathSyntax
  if let some unit := LeanerLang.registeredUnit? (← getEnv) (pathName segments) then
    unless (recursiveSpecFunctions unit).isEmpty do
      let twins ← SpecTypes.ensureSpecTypes segments unit
      liftTermElabM (ensureSpecFunctionDefinitions unit twins)
  let moduleNamespace := pathName segments
  let theorems := LeanerLang.theoremItems stx
  unless theorems.isEmpty do
    withModuleNamespace moduleNamespace <| Perf.withPhase .verification do
      for theoremCommand in theorems do elabCommand theoremCommand
  let some unit := LeanerLang.registeredUnit? (← getEnv) (pathName segments) | return
  -- The module's lemmas, and those its functions apply, before its functions.
  let ownLemmas := (unit.namespaces.toList.zipIdx.filter fun (ns, _) =>
      (unit.tables.namespaces[ns.identity.index]?.map (·.segments)) == some segments).flatMap
    fun (ns, index) => ns.lemmas.toList.map fun lemma =>
      ({ namespaceId := ⟨index⟩, name := lemma.name } : LeanerIR.QualifiedRef)
  let appliedByFunctions := unit.namespaces.toList.flatMap fun ns =>
    ns.functions.toList.flatMap fun declaration => (functionLemmas ns declaration).toList
  let roots := (ownLemmas ++ appliedByFunctions).foldl (init := #[]) fun roots lemma =>
    if roots.contains lemma then roots else roots.push lemma
  let lemmaItems := (lemmaItemsOf stx).filterMap fun item => do
    let identifier ← item.getArgs.find? (·.isIdent)
    pure (identifier.getId.toString (escape := false), identifier)
  verifyLemmas unit segments roots lemmaItems pathSyntax
  let items := LeanerLang.verificationItems stx
  let explicit := items.filterMap fun item =>
    if item.isOfKind ``leanerVerifyItem then item[1]?.map (·.getId.toString (escape := false))
    else none
  -- The explicit `verify` items, and the specified functions with a body
  -- that no explicit item names and whose specification does not set
  -- `pragma verify = false`, in source order.
  let targets : Array VerificationTarget := items.filterMap fun item =>
    if item.isOfKind ``leanerVerifyItem then
      item[1]?.map fun identifier =>
        ⟨identifier.getId.toString (escape := false), identifier,
          scriptOfOptional (item[2]?.getD .missing)⟩
    else do
      let identifier ← item.getArgs.find? (·.isIdent)
      let function := identifier.getId.toString (escape := false)
      if explicit.contains function then none
      let (namespaceIndex, ns, _, declaration) ← findFunction? unit function
      if declaration.body == .absent || statesNothing declaration.contract ||
          automaticVerificationDisabled unit ⟨namespaceIndex⟩ ns declaration then none
      some ⟨function, identifier, none⟩
  -- A function without a specification is verified against its loop
  -- invariants, its in-body assertions, the data invariants of the values
  -- it takes, returns, constructs, and mutates, and the invariants of the
  -- memory it writes.
  let named := targets.map (·.function)
  let targets := targets ++ (functionItems stx).filterMap fun item => do
    let identifier ← firstIdentifier? item[4]
    let function := identifier.getId.toString (escape := false)
    if named.contains function then none
    let (namespaceIndex, ns, _, declaration) ← findFunction? unit function
    if declaration.body == .absent ||
        (declaration.contract.loc.isSome && !statesNothing declaration.contract) ||
        automaticVerificationDisabled unit ⟨namespaceIndex⟩ ns declaration ||
        !(handlesDataInvariants unit ns declaration ||
          bodyStates ns declaration (· matches .loopInvariant | .assertion | .apply) ||
          Contract.owesMemoryInvariants unit ⟨namespaceIndex⟩ declaration) then
      none
    some ⟨function, identifier, none⟩
  -- A function of a module this file registered before, which writes memory
  -- an invariant of this module reads, is verified against it here.
  let inFile := (moduleUnits (← getEnv)).map (·.1)
  let writers : Array VerificationTarget :=
    (unit.namespaces.toList.zipIdx.drop 1).toArray.flatMap fun (ns, index) =>
      let path := (unit.tables.namespaces[ns.identity.index]?.map (·.segments)).getD #[]
      if !inFile.contains (pathName path) then #[] else
      ns.functions.zipIdx.filterMap fun (declaration, functionIndex) =>
        if declaration.body == .absent ||
            automaticVerificationDisabled unit ⟨index⟩ ns declaration ||
            !Contract.owesInvariantsOf unit ⟨0⟩ ⟨index⟩ declaration then none
        else some ⟨functionKey unit ⟨⟨index⟩, ⟨functionIndex⟩⟩, pathSyntax, none⟩
  let targets := targets ++ writers
  if targets.isEmpty then return
  let visited ← IO.mkRef (#[] : Array String)
  let covered ← IO.mkRef (#[] : Array String)
  stageLog s!"module {pathName segments}: verification start"
  for target in targets do verifyInOrder unit segments targets visited covered target
  stageLog s!"module {pathName segments}: verification done"

end LeanerLang.Verify
