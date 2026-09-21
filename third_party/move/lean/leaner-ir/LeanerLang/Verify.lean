-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Contract
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

deriving instance ToExpr for CheckedOp
deriving instance ToExpr for ModularOp
deriving instance ToExpr for CompareOp
deriving instance ToExpr for BitOp
deriving instance ToExpr for FunctionHandle
deriving instance ToExpr for LeanerIR.TypeId
deriving instance ToExpr for LeanerIR.LocId
deriving instance ToExpr for LeanerIR.TypeUse
deriving instance ToExpr for LeanerIR.StructHandle
deriving instance ToExpr for Family

attribute [lir_denote_norm] LeanerLang.Contract.lengthVector_vector
  LeanerLang.Contract.containsVector_vector

/-- A variant test of an encoded enum value tests the variant it holds. -/
theorem LeanerLang.Contract.testVariants_encode_enum [LeanerIR.Proofs.Denote.Skolems]
    (source : LeanerIR.StructHandle) (names : List String)
    (rows : LeanerIR.Proofs.Denote.NRows) (distinct : names.Nodup)
    (value : LeanerIR.Proofs.Denote.variantCarrier names rows)
    (owner : LeanerIR.StructHandle) (variants : Array String) :
    LeanerLang.Contract.testVariants
        (LeanerIR.Proofs.Denote.NTy.encode (.enum source names rows distinct) value)
        owner variants =
      (source == owner && LeanerLang.Contract.variantMember
        (LeanerIR.Proofs.Denote.variantName names rows value) variants.toList) := by
  obtain ⟨fields, encoded⟩ :=
    LeanerIR.Proofs.Denote.NTy.encode_enum_nominal source names rows distinct value
  rw [encoded]; rfl

attribute [lir_denote_norm] LeanerLang.Contract.testVariants_encode_enum
  LeanerLang.Contract.testVariants_nominal_self
  LeanerLang.Contract.testVariants_nominal LeanerLang.Contract.variantMember
  LeanerLang.Contract.selectVariantField LeanerLang.Contract.variantIndex
  LeanerIR.SemanticOperations.resolveReturnedBorrows_empty
  LeanerIR.SemanticOperations.resolveReturnedBorrows_integer

/-! ## Quotation of compiled functions -/

mutual
/-- The literal of a native type.  An enum's distinctness witness is decided. -/
partial def quoteNTy : NTy → MetaM Lean.Expr
  | .unit => return mkConst ``NTy.unit
  | .bool => return mkConst ``NTy.bool
  | .int width signed => return mkAppN (mkConst ``NTy.int) #[toExpr width, toExpr signed]
  | .address => return mkConst ``NTy.address
  | .signer => return mkConst ``NTy.signer
  | .string => return mkConst ``NTy.string
  | .bytes => return mkConst ``NTy.bytes
  | .tuple elements => return mkApp (mkConst ``NTy.tuple) (← quoteRow elements)
  | .struct source fields =>
      return mkAppN (mkConst ``NTy.struct) #[toExpr source, ← quoteRow fields]
  | .enum source names rows _ => do
      let namesExpr := toExpr names
      let distinct ← mkDecideProof (← mkAppM ``List.Nodup #[namesExpr])
      return mkAppN (mkConst ``NTy.enum) #[toExpr source, namesExpr, ← quoteRows rows, distinct]
  | .vector element => return mkApp (mkConst ``NTy.vector) (← quoteNTy element)
  | .ref referent => return mkApp (mkConst ``NTy.ref) (← quoteNTy referent)
  | .param index => return mkApp (mkConst ``NTy.param) (toExpr index)

partial def quoteRow : NRow → MetaM Lean.Expr
  | .nil => return mkConst ``NRow.nil
  | .cons τ rest => return mkAppN (mkConst ``NRow.cons) #[← quoteNTy τ, ← quoteRow rest]

partial def quoteRows : NRows → MetaM Lean.Expr
  | .nil => return mkConst ``NRows.nil
  | .cons fields rest => return mkAppN (mkConst ``NRows.cons) #[← quoteRow fields, ← quoteRows rest]
end

def quoteShape : ResultShape → MetaM Lean.Expr
  | .none => return mkConst ``ResultShape.none
  | .one τ => return mkApp (mkConst ``ResultShape.one) (← quoteNTy τ)

mutual
/-- The literal of a native value. -/
partial def quoteCarrier : (τ : NTy) → τ.groundCarrier → MetaM Lean.Expr
  | .tuple elements, value => quoteRowValue elements value
  | .struct _ fields, value => quoteRowValue fields value
  | .enum _ names rows _, value => quoteVariantValue names rows value
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

partial def quoteRowValue : (row : NRow) → @HList Skolems.ground row → MetaM Lean.Expr
  | .nil, _ => return mkConst ``Unit.unit
  | .cons τ rest, value => do
      mkAppM ``Prod.mk #[← quoteCarrier τ value.1, ← quoteRowValue rest value.2]

partial def quoteVariantValue : (names : List String) → (rows : NRows) →
    @variantCarrier Skolems.ground names rows → MetaM Lean.Expr
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | _ :: names, .cons fields rest, value => do
      let ground := mkConst ``Skolems.ground
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
  | .struct source fields, τ, @Proj.field _ _ σ _ x rest =>
      return mkAppN (mkConst ``Proj.field) #[Lean.toExpr source, ← quoteRow fields, ← quoteNTy σ,
        ← quoteNTy τ, ← quoteVar x, ← quoteProj rest]
  | .enum source names rows _, τ, @Proj.variant _ _ _ _ σ _ choices rest => do
      let namesExpr := toExpr names
      let distinct ← mkDecideProof (← mkAppM ``List.Nodup #[namesExpr])
      return mkAppN (mkConst ``Proj.variant) #[Lean.toExpr source, namesExpr, ← quoteRows rows,
        distinct, ← quoteNTy σ, ← quoteNTy τ, ← quoteChoices choices, ← quoteProj rest]

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
partial def quoteTerm {ρ : ResultShape} {Γ : NRow} : {τ : NTy} → Term ρ Γ τ →
    MetaM Lean.Expr := fun {τ} term => do
  let shape ← quoteShape ρ
  let context ← quoteRow Γ
  let toExpr := fun (τ : NTy) => quoteNTy τ
  match τ, term with
  | τ, .lit value =>
      return mkAppN (mkConst ``Term.lit) #[shape, context, ← toExpr τ, ← quoteCarrier τ value]
  | τ, .var x => return mkAppN (mkConst ``Term.var) #[shape, context, ← toExpr τ, ← quoteVar x]
  | .int width signed, .checked op failure left right =>
      return mkAppN (mkConst ``Term.checked) #[shape, context, Lean.toExpr width,
        Lean.toExpr signed, Lean.toExpr op, Lean.toExpr failure, ← quoteTerm left, ← quoteTerm right]
  | .int width signed, .modular op left right =>
      return mkAppN (mkConst ``Term.modular) #[shape, context, Lean.toExpr width,
        Lean.toExpr signed, Lean.toExpr op, ← quoteTerm left, ← quoteTerm right]
  | .bool, @Term.compare _ _ width signed op left right =>
      return mkAppN (mkConst ``Term.compare) #[shape, context, Lean.toExpr width,
        Lean.toExpr signed, Lean.toExpr op, ← quoteTerm left, ← quoteTerm right]
  | .bool, @Term.equal _ _ σ negated left right =>
      return mkAppN (mkConst ``Term.equal) #[shape, context, ← toExpr σ, Lean.toExpr negated,
        ← quoteTerm left, ← quoteTerm right]
  | .bool, .not operand =>
      return mkAppN (mkConst ``Term.not) #[shape, context, ← quoteTerm operand]
  | .bool, .logical conjunction left right =>
      return mkAppN (mkConst ``Term.logical) #[shape, context, Lean.toExpr conjunction,
        ← quoteTerm left, ← quoteTerm right]
  | .int width false, .bitwise op left right =>
      return mkAppN (mkConst ``Term.bitwise) #[shape, context, Lean.toExpr width, Lean.toExpr op,
        ← quoteTerm left, ← quoteTerm right]
  | .int width false, @Term.shift _ _ _ distanceWidth left failure value distance =>
      return mkAppN (mkConst ``Term.shift) #[shape, context, Lean.toExpr width,
        Lean.toExpr distanceWidth, Lean.toExpr left, Lean.toExpr failure, ← quoteTerm value,
        ← quoteTerm distance]
  | .int width' signed', @Term.cast _ _ width signed _ _ failure value =>
      return mkAppN (mkConst ``Term.cast) #[shape, context, Lean.toExpr width, Lean.toExpr signed,
        Lean.toExpr width', Lean.toExpr signed', Lean.toExpr failure, ← quoteTerm value]
  | τ, .ite condition thenBranch elseBranch =>
      return mkAppN (mkConst ``Term.ite) #[shape, context, ← toExpr τ, ← quoteTerm condition,
        ← quoteTerm thenBranch, ← quoteTerm elseBranch]
  | τ, @Term.let_ _ _ σ _ x value body =>
      return mkAppN (mkConst ``Term.let_) #[shape, context, ← toExpr σ, ← toExpr τ, ← quoteVar x,
        ← quoteTerm value, ← quoteTerm body]
  | τ, @Term.drop _ _ σ _ value body =>
      return mkAppN (mkConst ``Term.drop) #[shape, context, ← toExpr σ, ← toExpr τ,
        ← quoteTerm value, ← quoteTerm body]
  | .unit, @Term.assign _ _ σ x value =>
      return mkAppN (mkConst ``Term.assign) #[shape, context, ← toExpr σ, ← quoteVar x,
        ← quoteTerm value]
  | τ, .const value =>
      return mkAppN (mkConst ``Term.const) #[shape, context, ← toExpr τ, ← quoteTerm value]
  | τ, .throw0 kind =>
      return mkAppN (mkConst ``Term.throw0) #[shape, context, ← toExpr τ, Lean.toExpr kind]
  | τ, @Term.throw1 _ _ σ _ kind code =>
      return mkAppN (mkConst ``Term.throw1) #[shape, context, ← toExpr σ, ← toExpr τ,
        Lean.toExpr kind, ← quoteTerm code]
  | τ, .return_ value =>
      return mkAppN (mkConst ``Term.return_) #[shape, context, ← toExpr τ, ← quoteTerm value]
  | τ, .break_ nest =>
      return mkAppN (mkConst ``Term.break_) #[shape, context, ← toExpr τ, Lean.toExpr nest]
  | τ, .continue_ nest =>
      return mkAppN (mkConst ``Term.continue_) #[shape, context, ← toExpr τ, Lean.toExpr nest]
  | .unit, .loop site body =>
      return mkAppN (mkConst ``Term.loop) #[shape, context, Lean.toExpr site, ← quoteTerm body]
  | _, @Term.call _ _ σs handle calleeShape arguments =>
      return mkAppN (mkConst ``Term.call) #[shape, context, ← quoteRow σs, Lean.toExpr handle,
        ← quoteShape calleeShape, ← quoteArgs arguments]
  | _, @Term.callGeneric _ _ σs handle typeArgs θ calleeShape arguments => do
      let row ← quoteRow θ.1
      let inhabitable (row : Lean.Expr) :=
        mkApp3 (mkConst ``Eq [1]) (mkConst ``Bool) (mkApp (mkConst ``NRow.inhabitable) row)
          (mkConst ``Bool.true)
      let predicate := Lean.mkLambda `row .default (mkConst ``NRow) (inhabitable (.bvar 0))
      let typeArguments := mkAppN (mkConst ``Subtype.mk [1])
        #[mkConst ``NRow, predicate, row, ← mkDecideProof (inhabitable row)]
      return mkAppN (mkConst ``Term.callGeneric) #[shape, context, ← quoteRow σs,
        Lean.toExpr handle, Lean.toExpr typeArgs, typeArguments, ← quoteShape calleeShape,
        ← quoteArgs arguments]
  | .tuple σs, .tuple elements =>
      return mkAppN (mkConst ``Term.tuple) #[shape, context, ← quoteRow σs, ← quoteArgs elements]
  | .struct source σs, .pack _ fields =>
      return mkAppN (mkConst ``Term.pack)
        #[shape, context, ← quoteRow σs, Lean.toExpr source, ← quoteArgs fields]
  | .enum source names rows _, @Term.variant _ _ σs _ _ _ _ choice fields => do
      let namesExpr := Lean.toExpr names
      let distinct ← mkDecideProof (← mkAppM ``List.Nodup #[namesExpr])
      return mkAppN (mkConst ``Term.variant) #[shape, context, ← quoteRow σs, namesExpr,
        ← quoteRows rows, Lean.toExpr source, distinct, ← quoteWhich choice, ← quoteArgs fields]
  | τ, @Term.field _ _ σs _ source x value =>
      return mkAppN (mkConst ``Term.field) #[shape, context, ← quoteRow σs, ← toExpr τ,
        Lean.toExpr source, ← quoteVar x, ← quoteTerm value]
  | .bool, @Term.isVariant _ _ names rows source _ tests value => do
      let namesExpr := Lean.toExpr names
      let distinct ← mkDecideProof (← mkAppM ``List.Nodup #[namesExpr])
      return mkAppN (mkConst ``Term.isVariant) #[shape, context, namesExpr, ← quoteRows rows,
        Lean.toExpr source, distinct, Lean.toExpr tests, ← quoteTerm value]
  | τ, @Term.payload _ _ _ names rows source _ choices value => do
      let namesExpr := Lean.toExpr names
      let distinct ← mkDecideProof (← mkAppM ``List.Nodup #[namesExpr])
      return mkAppN (mkConst ``Term.payload) #[shape, context, ← toExpr τ, namesExpr,
        ← quoteRows rows, Lean.toExpr source, distinct, ← quoteChoices choices, ← quoteTerm value]
  | τ, @Term.letRow _ _ σs _ targets value body =>
      return mkAppN (mkConst ``Term.letRow) #[shape, context, ← quoteRow σs, ← toExpr τ,
        ← quoteVars targets, ← quoteTerm value, ← quoteTerm body]
  | τ, @Term.letFields _ _ σs _ source targets value body =>
      return mkAppN (mkConst ``Term.letFields) #[shape, context, ← quoteRow σs, ← toExpr τ,
        Lean.toExpr source, ← quoteVars targets, ← quoteTerm value, ← quoteTerm body]
  | τ, .deref value =>
      return mkAppN (mkConst ``Term.deref) #[shape, context, ← toExpr τ, ← quoteTerm value]
  | .unit, @Term.mutate _ _ τ x value =>
      return mkAppN (mkConst ``Term.mutate) #[shape, context, ← toExpr τ, ← quoteVar x,
        ← quoteTerm value]
  | σ, @Term.readPlace _ _ τ _ x path =>
      return mkAppN (mkConst ``Term.readPlace) #[shape, context, ← toExpr τ, ← toExpr σ,
        ← quoteVar x, ← quoteProj path]
  | .unit, @Term.writePlace _ _ τ σ x path value =>
      return mkAppN (mkConst ``Term.writePlace) #[shape, context, ← toExpr τ, ← toExpr σ,
        ← quoteVar x, ← quoteProj path, ← quoteTerm value]
  | .ref σ, @Term.borrowPlace _ _ τ _ x path =>
      return mkAppN (mkConst ``Term.borrowPlace) #[shape, context, ← toExpr τ, ← toExpr σ,
        ← quoteVar x, ← quoteProj path]
  | τ, .take x => return mkAppN (mkConst ``Term.take) #[shape, context, ← toExpr τ, ← quoteVar x]
  | .unit, @Term.resolve _ _ τ x =>
      return mkAppN (mkConst ``Term.resolve) #[shape, context, ← toExpr τ, ← quoteVar x]
  | .vector τ, .vectorLit count elements =>
      return mkAppN (mkConst ``Term.vectorLit) #[shape, context, ← toExpr τ, Lean.toExpr count,
        ← quoteArgs elements]
  | .int 64 false, @Term.length _ _ τ vector =>
      return mkAppN (mkConst ``Term.length) #[shape, context, ← toExpr τ, ← quoteTerm vector]
  | .address, .signerAddress signer =>
      return mkAppN (mkConst ``Term.signerAddress) #[shape, context, ← quoteTerm signer]
  | τ, @Term.index _ _ _ width signed vector position =>
      return mkAppN (mkConst ``Term.index) #[shape, context, ← toExpr τ, Lean.toExpr width,
        Lean.toExpr signed, ← quoteTerm vector, ← quoteTerm position]
  | .unit, @Term.checkIndex _ _ τ width signed failure vector position =>
      return mkAppN (mkConst ``Term.checkIndex) #[shape, context, ← toExpr τ, Lean.toExpr width,
        Lean.toExpr signed, Lean.toExpr failure, ← quoteTerm vector, ← quoteTerm position]
  | .vector τ, .push vector element =>
      return mkAppN (mkConst ``Term.push) #[shape, context, ← toExpr τ, ← quoteTerm vector,
        ← quoteTerm element]
  | .vector τ, @Term.insert _ _ _ width signed vector position element =>
      return mkAppN (mkConst ``Term.insert) #[shape, context, ← toExpr τ, Lean.toExpr width,
        Lean.toExpr signed, ← quoteTerm vector, ← quoteTerm position, ← quoteTerm element]
  | .tuple (.cons τ (.cons (.vector _) .nil)), @Term.remove _ _ _ width signed vector position =>
      return mkAppN (mkConst ``Term.remove) #[shape, context, ← toExpr τ, Lean.toExpr width,
        Lean.toExpr signed, ← quoteTerm vector, ← quoteTerm position]
  | .vector τ, @Term.swap _ _ _ width signed vector left right =>
      return mkAppN (mkConst ``Term.swap) #[shape, context, ← toExpr τ, Lean.toExpr width,
        Lean.toExpr signed, ← quoteTerm vector, ← quoteTerm left, ← quoteTerm right]
  | .vector τ, .concat left right =>
      return mkAppN (mkConst ``Term.concat) #[shape, context, ← toExpr τ, ← quoteTerm left,
        ← quoteTerm right]
  | .vector τ, @Term.slice _ _ _ width signed vector start stop =>
      return mkAppN (mkConst ``Term.slice) #[shape, context, ← toExpr τ, Lean.toExpr width,
        Lean.toExpr signed, ← quoteTerm vector, ← quoteTerm start, ← quoteTerm stop]
  | .vector τ, @Term.reverseSlice _ _ _ width signed vector start stop =>
      return mkAppN (mkConst ``Term.reverseSlice) #[shape, context, ← toExpr τ, Lean.toExpr width,
        Lean.toExpr signed, ← quoteTerm vector, ← quoteTerm start, ← quoteTerm stop]
  | .unit, @Term.destroyEmpty _ _ τ vector =>
      return mkAppN (mkConst ``Term.destroyEmpty) #[shape, context, ← toExpr τ, ← quoteTerm vector]
  | .bool, @Term.contains _ _ τ vector needle =>
      return mkAppN (mkConst ``Term.contains) #[shape, context, ← toExpr τ, ← quoteTerm vector,
        ← quoteTerm needle]
  | .tuple (.cons .bool (.cons (.int 64 false) .nil)), @Term.indexOf _ _ τ vector needle =>
      return mkAppN (mkConst ``Term.indexOf) #[shape, context, ← toExpr τ, ← quoteTerm vector,
        ← quoteTerm needle]
  | .int 8 true, @Term.order _ _ τ orders left right =>
      return mkAppN (mkConst ``Term.order) #[shape, context, ← toExpr τ, Lean.toExpr orders,
        ← quoteTerm left, ← quoteTerm right]
  | τ, .seqAfter value effect =>
      return mkAppN (mkConst ``Term.seqAfter) #[shape, context, ← toExpr τ, ← quoteTerm value,
        ← quoteTerm effect]
  | τ, @Term.globalRead _ _ κ _ family key =>
      return mkAppN (mkConst ``Term.globalRead) #[shape, context, ← toExpr κ, ← toExpr τ,
        Lean.toExpr family, ← quoteTerm key]
  | .bool, @Term.globalContains _ _ κ family key =>
      return mkAppN (mkConst ``Term.globalContains) #[shape, context, ← toExpr κ,
        Lean.toExpr family, ← quoteTerm key]
  | .ref τ, @Term.globalBorrow _ _ κ _ family key =>
      return mkAppN (mkConst ``Term.globalBorrow) #[shape, context, ← toExpr κ, ← toExpr τ,
        Lean.toExpr family, ← quoteTerm key]
  | .unit, @Term.globalPublish _ _ κ τ family key value =>
      return mkAppN (mkConst ``Term.globalPublish) #[shape, context, ← toExpr κ, ← toExpr τ,
        Lean.toExpr family, ← quoteTerm key, ← quoteTerm value]
  | τ, @Term.globalTake _ _ κ _ family key =>
      return mkAppN (mkConst ``Term.globalTake) #[shape, context, ← toExpr κ, ← toExpr τ,
        Lean.toExpr family, ← quoteTerm key]

partial def quoteArgs {ρ : ResultShape} {Γ : NRow} : {σs : NRow} → Args ρ Γ σs →
    MetaM Lean.Expr := fun {σs} arguments => do
  let shape ← quoteShape ρ
  let context ← quoteRow Γ
  match σs, arguments with
  | _, .nil => return mkAppN (mkConst ``Args.nil) #[shape, context]
  | .cons σ σs, .cons head tail =>
      return mkAppN (mkConst ``Args.cons) #[shape, context, ← quoteNTy σ, ← quoteRow σs,
        ← quoteTerm head, ← quoteArgs tail]
end

mutual
/-- Fold over the calls a term makes, in evaluation order: each callee
with the type arguments of the call. -/
partial def _root_.LeanerIR.Proofs.Denote.Term.foldCalls {α : Type}
    (visit : FunctionHandle → Array LeanerIR.TypeUse → α → α) {ρ : ResultShape} :
    {Γ : NRow} → {τ : NTy} → Term ρ Γ τ → α → α
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
  | _, _, .call handle _ arguments, found => arguments.foldCalls visit (visit handle #[] found)
  | _, _, .callGeneric handle typeArgs _ _ arguments, found =>
      arguments.foldCalls visit (visit handle typeArgs found)
  | _, _, .tuple elements, found => elements.foldCalls visit found
  | _, _, .pack _ fields, found => fields.foldCalls visit found
  | _, _, .variant _ _ _ fields, found => fields.foldCalls visit found
  | _, _, .field _ value, found => value.foldCalls visit found
  | _, _, .isVariant _ value, found => value.foldCalls visit found
  | _, _, .payload _ value, found => value.foldCalls visit found
  | _, _, .letRow _ value body, found => body.foldCalls visit (value.foldCalls visit found)
  | _, _, .letFields _ value body, found => body.foldCalls visit (value.foldCalls visit found)
  | _, _, .deref value, found => value.foldCalls visit found
  | _, _, .mutate _ value, found => value.foldCalls visit found
  | _, _, .writePlace _ _ value, found => value.foldCalls visit found
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
  | _, _, .globalRead _ key, found => key.foldCalls visit found
  | _, _, .globalContains _ key, found => key.foldCalls visit found
  | _, _, .globalBorrow _ key, found => key.foldCalls visit found
  | _, _, .globalPublish _ key value, found => value.foldCalls visit (key.foldCalls visit found)
  | _, _, .globalTake _ key, found => key.foldCalls visit found
  | _, _, _, found => found

partial def _root_.LeanerIR.Proofs.Denote.Args.foldCalls {α : Type}
    (visit : FunctionHandle → Array LeanerIR.TypeUse → α → α) {ρ : ResultShape} :
    {Γ σs : NRow} → Args ρ Γ σs → α → α
  | _, _, .nil, found => found
  | _, _, .cons head tail, found => tail.foldCalls visit (head.foldCalls visit found)
end

/-- The callees a term calls, in order of first occurrence. -/
def _root_.LeanerIR.Proofs.Denote.Term.callees {ρ : ResultShape} {Γ : NRow} {τ : NTy}
    (term : Term ρ Γ τ) (found : Array FunctionHandle) : Array FunctionHandle :=
  term.foldCalls (fun handle _ found => if found.contains handle then found else found.push handle)
    found

/-- The mutable parameters of a compiled function as a literal. -/
def quoteMutables {Γ : NRow} : Mutables Γ → MetaM Lean.Expr
  | .nil => return mkApp (mkConst ``Mutables.nil) (← quoteRow Γ)
  | @Mutables.cons _ τ x rest =>
      return mkAppN (mkConst ``Mutables.cons)
        #[← quoteRow Γ, ← quoteNTy τ, ← quoteVar x, ← quoteMutables rest]

def quoteFunction (f : Function) (body mutables : Lean.Expr) : MetaM Lean.Expr :=
  return mkAppN (mkConst ``Function.mk)
    #[← quoteRow f.params, ← quoteRow f.locals, ← quoteShape f.result, body, mutables]

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
  withLocalDecl `Θ .instImplicit (mkConst ``Skolems) k

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
      | .nominal twin _, .struct _ innerRow =>
          match twins.find? (·.twin == twin) with
          | some inner => twinFields twins inner innerRow projection
          | none => pure projection
      | .nominal twin _, .enum .. => mkAppM (twin ++ `native) #[projection]
      -- A vector of nested twins is viewed element by element.
      | .vector (.nominal twin _) bounded, .vector (.struct _ innerRow) =>
          match twins.find? (·.twin == twin) with
          | some inner => do
              -- `SpecVector α` or `Array α`: the element type is the argument.
              let elementType := (← whnfR (← inferType projection)).appArg!
              let element ← withLocalDeclD `element elementType fun element => do
                mkLambdaFVars #[element] (← twinFields twins inner innerRow element)
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
      | .nominal twin _ =>
          let some inner := twins.find? (·.twin == twin) | return none
          let some innerShapes ← twinShapes twins inner s!"{base}_{name}" | return none
          pure innerShapes
      | _ => return none
    shapes := shapes.flatMap fun shape => fieldShapes.map shape.append
  return some shapes

/-- The literal shapes of a twin value: one per struct, one per variant of
an enum, each carrying the literal and the twin value as one component. -/
private partial def twinShapes (twins : Array SpecTypes.TwinInfo) (info : SpecTypes.TwinInfo)
    (base : String) : CommandElabM (Option (Array LiteralShape)) := do
  let handle := (Syntax.mkNumLit (toString info.namespaceIndex),
    Syntax.mkNumLit (toString info.structIndex))
  let decoder := rootIdent (info.twin ++ `decode?)
  if info.variants.isEmpty then
    let some shapes ← fieldShapes twins info.fields base | return none
    shapes.mapM fun shape => do
      let literal ← `(term| LeanerIR.RuntimeValue.nominal ⟨⟨$(handle.1)⟩, $(handle.2)⟩ none
        #[$(shape.eraseTerms),*])
      let component ← `(term| (⟨$(shape.componentTerms),*⟩ : $(rootIdent info.twin)))
      let decoderNames := #[decoder] ++ shape.decoderNames
      let eraseTerms := #[literal]
      let componentTerms := #[component]
      pure { shape with eraseTerms, componentTerms, decoderNames }
  else
    let mut shapes : Array LiteralShape := #[]
    for variant in info.variants do
      let some variantShapes ← fieldShapes twins variant.fields s!"{base}_{variant.name}"
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
    elabCommand (← `(@[lir_denote_norm] theorem $(rootIdent name):ident $binderSyntax* :
        $(rootIdent (info.twin ++ `decode?)) $literal = $rhs := by
      simp only [$lemmas,*, LeanerIR.decodeInt?]
      repeat' (first | rfl | (split <;> simp_all))))

/-- The syntax of a twin field's native carrier from a bound variable of
the field's twin type: a nested struct expands to its row, a nested enum
takes its native view. -/
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
  | _ => pure variable_

/-- The native view of an enum twin, `Twin.native : Twin → variantCarrier`,
and the lemma that its erasure is the native encoding of that view. -/
private partial def ensureEnumNative (unit : ValidatedUnit) (twins : Array SpecTypes.TwinInfo)
    (info : SpecTypes.TwinInfo) : TermElabM Unit := do
  let nativeName := info.twin ++ `native
  if (← getEnv).contains nativeName then return
  -- A variant's enum-typed field bridges through the inner twin's view.
  let innerEnums := info.variants.flatMap fun variant => variant.fields.filterMap fun (_, rep) =>
    match rep with
    | .nominal twin _ => (twins.find? (·.twin == twin)).bind fun inner =>
        if inner.variants.isEmpty || inner.typeParameterCount != 0 then none else some inner
    | _ => none
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
    mkLambdaFVars #[skolems] (← instantiateMVars definition)
  addAbbrev nativeName definition
  let normAttr ← `(attr| lir_denote_norm)
  Lean.Elab.Term.applyAttributes nativeName #[{ name := `lir_denote_norm, stx := normAttr, kind := .global }]
  let eraseName := info.twin ++ `erase_eq_encode
  let statement ← withSkolems fun skolems => withLocalDeclD `value twinType fun value => do
    let lhs ← mkAppM (info.twin ++ `erase) #[value]
    let rhs ← mkAppM ``NTy.encode #[τE, mkApp2 (mkConst nativeName) skolems value]
    mkForallFVars #[skolems, value] (← mkEq lhs rhs)
  let lemmaNames := info.variants.map (fun variant =>
      info.twin ++ Name.mkSimple s!"erase_{variant.name}") ++
    innerEnums.map (·.twin ++ `erase_eq_encode) ++
    #[``NTy.encode_enum_inl, ``NTy.encode_enum_inr, ``HList.encode_cons, ``HList.encode_nil]
  let lemmas ← lemmaNames.mapM fun name => `(Lean.Parser.Tactic.simpLemma| $(mkIdent name):ident)
  let proofSyntax ← `(term| fun [Skolems] ($value:ident : $(mkIdent info.twin)) => by
    cases $value:ident <;> (try simp only [$lemmas,*]) <;> rfl)
  let proof ← Lean.Elab.Term.elabTermEnsuringType proofSyntax (some statement)
  Lean.Elab.Term.synthesizeSyntheticMVarsNoPostponing
  let proof ← instantiateMVars proof
  addDecl (.thmDecl { name := eraseName, levelParams := [], type := statement, value := proof })

/-- Whether a twin views a field element by element: a vector of nested
twins, whose bridge needs the composition of the two maps. -/
private def mapsField (info : SpecTypes.TwinInfo) : Bool :=
  info.fields.any fun (_, rep) => rep matches .vector (.nominal ..) _

/-- The proof of a bridge whose twin maps a field: both sides unfold to the
same element-wise images once the two maps compose. -/
private def mappedBridgeProof (statement : Lean.Expr) (erase : Name) : TermElabM Lean.Expr := do
  let proofSyntax ← `(term| by
    intros
    simp only [$(mkIdent erase):ident, LeanerIR.Proofs.Codec.boundedVector_encode,
      LeanerIR.Proofs.Denote.NTy.encode_struct, LeanerIR.Proofs.Denote.NTy.encode_vector,
      LeanerIR.Proofs.Denote.HList.encode_cons, LeanerIR.Proofs.Denote.HList.encode_nil,
      LeanerIR.SpecVector.map_values]
    repeat erw [Array.map_map]
    rfl)
  let proof ← Lean.Elab.Term.elabTermEnsuringType proofSyntax (some statement)
  Lean.Elab.Term.synthesizeSyntheticMVarsNoPostponing
  instantiateMVars proof

/-- The bridge between a struct twin and its native type: the twin's
erasure is the native encoding, and the twin decodes the literal a native
encoding unfolds to.  With both, a clause's typed view of a stored
resource and the denotation's native view of the same runtime value meet.
An enum-typed field is bridged through the enum twin's native view. -/
private def ensureTwinBridges (unit : ValidatedUnit) (twins : Array SpecTypes.TwinInfo)
    (families : Array SpecTypes.FamilyInfo) :
    TermElabM Unit := do
  for info in twins do
    if !info.variants.isEmpty && info.typeParameterCount == 0 then
      ensureEnumNative unit twins info
  for info in twins do
    unless info.variants.isEmpty && info.typeParameterCount == 0 do continue
    let eraseName := info.twin ++ `erase_eq_encode
    if (← getEnv).contains eraseName then continue
    let handle : LeanerIR.StructHandle := ⟨⟨info.namespaceIndex⟩, info.structIndex⟩
    let some τ := structNTy unit handle | continue
    let τE ← quoteNTy τ
    let twinType := mkConst info.twin
    let .struct _ row := τ | continue
    let statement ← withSkolems fun skolems => withLocalDeclD `value twinType fun value => do
      let lhs ← mkAppM (info.twin ++ `erase) #[value]
      let rhs ← mkAppM ``NTy.encode #[τE, ← twinFields twins info row value]
      mkForallFVars #[skolems, value] (← mkEq lhs rhs)
    let enumFields := info.fields.filterMap fun (_, rep) => match rep with
      | .nominal twin _ => (twins.find? (·.twin == twin)).bind fun inner =>
          if inner.variants.isEmpty then none else some (inner.twin ++ `erase_eq_encode)
      | _ => none
    let proof ← if mapsField info then mappedBridgeProof statement (info.twin ++ `erase)
      else if enumFields.isEmpty then
        withSkolems fun skolems => withLocalDeclD `value twinType fun value => do
          mkLambdaFVars #[skolems, value] (← mkEqRefl (← mkAppM (info.twin ++ `erase) #[value]))
      else do
        let value := mkIdent `value
        let lemmas ← enumFields.mapM fun name => `(Lean.Parser.Tactic.simpLemma| $(mkIdent name):ident)
        let proofSyntax ← `(term| fun [Skolems] ($value:ident : $(mkIdent info.twin)) => by
          simp only [$(mkIdent (info.twin ++ `erase)):ident, $lemmas,*]
          rfl)
        let proof ← Lean.Elab.Term.elabTermEnsuringType proofSyntax (some statement)
        Lean.Elab.Term.synthesizeSyntheticMVarsNoPostponing
        instantiateMVars proof
    addDecl (.thmDecl { name := eraseName, levelParams := [], type := statement, value := proof })
    let attr ← `(attr| lir_denote_norm)
    Lean.Elab.Term.applyAttributes eraseName #[{ name := `lir_denote_norm, stx := attr, kind := .global }]
  -- A generic struct twin bridges at the skolem family, where its type
  -- parameters are the family's carriers, and at each concrete spelling a
  -- storage family uses; its decoder unfolds on the literal an encoding
  -- reduces to.
  let bridge (name : Name) (eraseFn : Lean.Expr) (row : NRow) (τ : NTy) (info : SpecTypes.TwinInfo)
      (binders : Array Lean.Expr) : TermElabM Unit := do
    let twinType := (← whnfD (← inferType eraseFn)).bindingDomain!
    let (statement, proof) ← withLocalDeclD `value twinType fun value => do
      let lhs := mkApp eraseFn value
      let rhs ← mkAppM ``NTy.encode #[← quoteNTy τ, ← twinFields twins info row value]
      pure (← mkForallFVars (binders.push value) (← mkEq lhs rhs),
        ← mkLambdaFVars (binders.push value) (← mkEqRefl lhs))
    let proof ← if mapsField info then mappedBridgeProof statement (info.twin ++ `erase)
      else pure proof
    addDecl (.thmDecl { name, levelParams := [], type := statement, value := proof })
    let attr ← `(attr| lir_denote_norm)
    Lean.Elab.Term.applyAttributes name #[{ name := `lir_denote_norm, stx := attr, kind := .global }]
  for info in twins do
    unless info.variants.isEmpty && info.typeParameterCount != 0 do continue
    let eraseName := info.twin ++ `erase_eq_encode
    if (← getEnv).contains eraseName then continue
    let handle : LeanerIR.StructHandle := ⟨⟨info.namespaceIndex⟩, info.structIndex⟩
    let some τ := structNTy unit handle | continue
    let .struct _ row := τ | continue
    withSkolems fun skolems => do
      let codecs := (Array.range info.typeParameterCount).map fun index =>
        mkApp2 (mkConst ``Skolems.codec) skolems (toExpr index)
      bridge eraseName (← mkAppM (info.twin ++ `erase) codecs) row τ info #[skolems]
    let decodeAttr ← `(attr| lir_denote_norm)
    Lean.Elab.Term.applyAttributes (info.twin ++ `decode?)
      #[{ name := `lir_denote_norm, stx := decodeAttr, kind := .global }]
  for family in families do
    unless family.info.variants.isEmpty && family.info.typeParameterCount != 0 &&
        family.arguments.all (·.parameterCount == 0) do continue
    let eraseName := family.accessor ++ `erase_eq_encode
    if (← getEnv).contains eraseName then continue
    let some τ := ntyOf unit ⟨family.info.namespaceIndex⟩ ⟨family.typeIndex⟩ | continue
    let .struct _ row := τ | continue
    withSkolems fun skolems => do
      bridge eraseName (← family.erase none) row τ family.info #[skolems]

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
    hasFinalContract ns declaration

/-- Whether a function has type parameters: it is proved over every skolem
family and type instantiation. -/
def isGeneric (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : Bool :=
  declaration.signature.generics.any (·.kind == .typeArg)

/-- The typed contract of a function over its native arguments, abstracted
over the skolem family and, for a generic function, the type instantiation
that keys its generic families. -/
def typedContractOf (unit : ValidatedUnit) (namespaceIndex : Nat) (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (params : NRow) (result : ResultShape) (twins : Array SpecTypes.TwinInfo)
    (families : Array SpecTypes.FamilyInfo) : MetaM Lean.Expr :=
  withSkolems fun skolems => do
    let signature ← quoteNativeSignature skolems params result
    let build (typeInstantiation : Option Lean.Expr) :=
      buildContract unit ⟨namespaceIndex⟩ ns declaration signature twins families
        (carrier := mkApp (mkConst ``Skolems.carrier) skolems)
        (codecs := mkApp (mkConst ``Skolems.codec) skolems) (typeInstantiation := typeInstantiation)
    if isGeneric declaration then
      let instantiationType ← mkAppM ``Array
        #[← mkAppM ``Prod #[mkConst ``LeanerIR.TypeId, mkConst ``LeanerIR.TypeId]]
      withLocalDeclD `typeInstantiation instantiationType fun typeInstantiation => do
        mkLambdaFVars #[skolems, typeInstantiation] (← build (some typeInstantiation))
    else mkLambdaFVars #[skolems] (← build none)

/-- The runtime form of a typed contract, at the public family and the
empty type instantiation. -/
def publicContractOf (params : NRow) (result : ResultShape) (generic : Bool) (typed : Lean.Expr) :
    MetaM Lean.Expr := do
  let runtime := mkConst ``Skolems.runtime
  let typed := mkApp typed runtime
  let typed := if generic then
      mkApp typed (toExpr (#[] : Array (LeanerIR.TypeId × LeanerIR.TypeId)))
    else typed
  mkAppOptM ``LeanerIR.Proofs.Contract.prophetic
    #[some runtime, some (← quoteRow params), some (← quoteShape result), some typed.headBeta]

/-- Define the typed and public contracts of a function, once: the contract
over its native arguments, and its runtime form. -/
def ensureContracts (segments : Array String) (function : String) (unit : ValidatedUnit)
    (namespaceIndex : Nat) (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (params : NRow) (result : ResultShape) : CommandElabM Unit := do
  let typedName := typedContractName segments function
  let publicName := contractName segments function
  if (← getEnv).contains publicName then
    let some info := (← getEnv).find? typedName
      | throwError m!"`{function}` has a contract this unit did not generate"
    unless info.type.getUsedConstants.contains ``HList do
      throwError m!"`{function}` has a contract this unit did not generate"
    return
  let (twins, families) ← SpecTypes.ensureSpecTypes segments unit
  for info in twins do
    if info.variants.isEmpty && info.typeParameterCount == 0 then ensureDecodeBridge twins info
  liftTermElabM do
    ensureTwinBridges unit twins families
    addAbbrev typedName
      (← typedContractOf unit namespaceIndex ns declaration params result twins families)
    addAbbrev publicName
      (← publicContractOf params result (isGeneric declaration) (mkConst typedName))

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
  | .tuple _ | .struct _ _ | .enum _ _ _ _ | .vector _ =>
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
global operation, or through a call to a function that may. -/
private partial def mayTouchStore (unit : ValidatedUnit) (namespaceId : NamespaceId)
    (id : LeanerIR.ExprId) (visiting : Array FunctionHandle) : Bool :=
  match unit.namespaces[namespaceId.index]?.bind (·.expressions[id.index]?) with
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
        | .operation (.call (.closure _)) _ _ _ | .operation (.call .invoke) _ _ _
        | .operation (.call (.extension ..)) _ _ _ => true
        | _ => false
      direct || (LeanerIR.Validation.expressionChildren expression.kind).any
        fun child => mayTouchStore unit namespaceId child visiting

/-- Whether a call may change the global store. A callee proved at its
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
      if declaration.contract.loc.isSome &&
          contractStandsFor unit callee.namespaceId ns declaration then
        declaration.contract.modifiesAll || !declaration.contract.modifies.isEmpty
      else match declaration.body with
        | .structured root => mayTouchStore unit callee.namespaceId root (visiting.push callee)
        | _ => true
end

/-- The invariant of each loop of a function, keyed by the loop's site:
over the function's starting locals and state (what `old` reads), the loop's
entry locals and state, and the current locals and state. -/
def loopInvariants (unit : ValidatedUnit) (namespaceId : NamespaceId) (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (params : NRow) (row : NRow) (codecs : Option Lean.Expr)
    (twins : Array SpecTypes.TwinInfo) (families : Array SpecTypes.FamilyInfo) :
    TermElabM (Array (Nat × Lean.Expr)) := do
  let .structured root := declaration.body | return #[]
  let envType ← mkAppM ``HEnv #[← quoteRow row]
  let argumentsType ← mkAppM ``HList #[← quoteRow params]
  let parameterTypes := params.toList
  let row := row.toList
  let localTypes ← declaration.locals.mapM fun localDecl => do
    let some ty := unit.tables.types[localDecl.type.typeId.index]?
      | throwError "a local has an unknown type"
    match ty with
    | .reference reference =>
        let some referent := unit.tables.types[reference.referent.index]?
          | throwError "a reference local has an unknown referent type"
        pure referent
    | _ => pure ty
  let mut invariants := #[]
  for (site, block) in loopSpecifications ns root do
    let changed := loopBodyLocals ns site
    let storeTouched := mayTouchStore unit namespaceId site #[]
    let stateType := mkConst ``LeanerIR.RuntimeState
    let invariant ← withLocalDeclD `start argumentsType fun start =>
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
        let referenced := block.conditions.foldl (fun found condition =>
          if condition.kind == .loopInvariant then referencedLocals ns condition.expression found
          else found) #[]
        let oldReferenced := block.conditions.foldl (fun found condition =>
          if condition.kind == .loopInvariant then oldReferencedLocals ns condition.expression found
          else found) #[]
        -- A parameter read under `old` is its argument at the function's
        -- start.
        let mut oldLocals : Array (Option Lean.Expr) := #[]
        for index in [0:row.length] do
          if oldReferenced.contains index then
            let some τ := parameterTypes[index]?
              | throwError "`old` in a loop invariant reads a parameter"
            oldLocals := oldLocals.push (← logicalValue τ startSlots[index]!)
          else oldLocals := oldLocals.push none
        let rec authoredOver (index : Nat) (binders : Array (Option Lean.Expr)) :
            TermElabM Lean.Expr := do
          if h : index < row.length then
            let τ := row[index]
            if referenced.contains index then
              -- A slot a clause reads is asserted defined, so that an
              -- iteration knows its value before the body reads it; the
              -- binder carries the local's name into an authored proof.
              let name := match declaration.locals[index]? with
                | some localDecl => Name.mkSimple localDecl.name
                | none => Name.mkSimple s!"slot{index}"
              withLocalDeclD name (← mkAppM ``NTy.carrier #[← quoteNTy τ]) fun binder => do
                let inner ← authoredOver (index + 1) (binders.push (some binder))
                mkAppM ``Option.elim
                  #[slots[index]!, mkConst ``False, ← mkLambdaFVars #[binder] inner]
            else authoredOver (index + 1) (binders.push none)
          else
            let mut locals : Array (Option Lean.Expr) := #[]
            for (τ, binder) in row.toArray.zip binders do
              locals := locals.push (← match binder with
                | some binder => logicalValue τ binder
                | none => pure none)
            translateLoopInvariants unit namespaceId ns block locals localTypes codecs
              (declaration.locals.map (·.name)) oldLocals startState twins families
        let authored ← authoredOver 0 #[]
        -- The store the body leaves alone keeps its entry value.
        let mut stateFrame : Array Lean.Expr := #[]
        unless storeTouched do
          stateFrame := stateFrame.push (← mkEq (← mkAppM ``LeanerIR.RuntimeState.globals #[state])
            (← mkAppM ``LeanerIR.RuntimeState.globals #[initial]))
        let conjunction ← ((frame.push authored) ++ stateFrame).foldrM (init := mkConst ``True)
          fun clause rest => mkAppM ``And #[clause, rest]
        mkLambdaFVars #[start, startState, entry, initial, env, state] conjunction
    invariants := invariants.push (site.index, invariant)
  return invariants

/-! ## Verification -/

private def countErrors (log : MessageLog) : Nat :=
  log.reportedPlusUnreported.toList.filter (·.severity == .error) |>.length

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
  | .struct source fields =>
      rowPattern (fields.toList.zip ((fieldNames unit source none fields.length).map (s!"{base}_{·}")))
  | .enum source names rows _ =>
      let alternatives := (names.zip rows.toList).map fun (name, fields) =>
        rowPattern (fields.toList.zip
          ((fieldNames unit source (some name) fields.length).map (s!"{base}_{name}_{·}")))
      "(" ++ " | ".intercalate (alternatives ++ ["(⟨⟩ : Empty)"]) ++ ")"
  | .ref referent =>
      s!"⟨{carrierPattern unit base referent}, {carrierPattern unit s!"{base}_final" referent}⟩"
  | _ => base
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
its meaning satisfies its typed contract, at every family and instantiation
for a generic native, at the runtime family otherwise. -/
private def nativeHypothesisType (unit : ValidatedUnit) (twins : Array SpecTypes.TwinInfo)
    (families : Array SpecTypes.FamilyInfo) (executable : Lean.Expr)
    (handle : FunctionHandle) : MetaM Lean.Expr := do
  let some nativeNs := unit.namespaces[handle.namespaceId.index]?
    | throwError "a native's namespace is out of range"
  let some declaration := nativeNs.functions[handle.functionId.index]?
    | throwError "a native is out of range"
  let some (params, result) := nativeSignature? unit handle.namespaceId declaration
    | throwError "a native has no native signature"
  let typed ← typedContractOf unit handle.namespaceId.index nativeNs declaration params result
    twins families
  let row ← quoteRow params
  let shape ← quoteShape result
  let meaningAt (skolems instantiation : Lean.Expr) :=
    mkAppN (mkConst ``propheticMeaning) #[skolems, executable, instantiation, toExpr handle, row, shape]
  if isGeneric declaration then
    withLocalDecl `Θ .instImplicit (mkConst ``Skolems) fun skolems =>
      withLocalDeclD `typeInstantiation
          (toTypeExpr (Array (LeanerIR.TypeId × LeanerIR.TypeId))) fun instantiation => do
        let statement ← mkAppM ``LeanerIR.Proofs.Satisfies
          #[meaningAt skolems instantiation, (mkApp2 typed skolems instantiation).headBeta]
        mkForallFVars #[skolems, instantiation] statement
  else
    let runtime := mkConst ``Skolems.runtime
    mkAppM ``LeanerIR.Proofs.Satisfies
      #[meaningAt runtime (toExpr (#[] : Array (LeanerIR.TypeId × LeanerIR.TypeId))),
        (mkApp typed runtime).headBeta]

/-- The no-fallback audit of one verified function, including the transitive
axiom closure. The agreement axiom is the sole project-specific exception,
explicitly deferred to D4 in `designs/denotation.md`. -/
def requireNativeArtifacts (base : Name) : CommandElabM Unit := do
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
  -- Lean caches axiom closures of imported declarations; this also follows
  -- helpers outside the artifact namespace without walking imported bodies.
  for root in roots ++ #[base ++ `verified] do
    for axiomName in ← collectAxioms root do
      unless #[``propext, ``Classical.choice, ``Quot.sound,
          ``compileFunction_agrees, ``compileFunction_least_cycle,
          ``compileFunction_least_generic].contains axiomName do
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
          dependency == ``LeanerIR.SemanticOperations.FreshGlobalLoanIds ||
          dependency == ``LeanerIR.SemanticOperations.LoanDiscipline ||
          dependency == ``LeanerIR.SemanticOperations.globalLoanKeyIn? ||
          dependency == ``LeanerIR.SemanticOperations.removeGlobalLoan then
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
  let (twins, families) ← SpecTypes.ensureSpecTypes segments unit
  let valid ← liftTermElabM do
    let typed ← typedContractOf unit namespaceIndex ns declaration params result twins families
    let publicContract ← publicContractOf params result (isGeneric declaration) typed
    let handle : FunctionHandle := ⟨⟨namespaceIndex⟩, ⟨functionIndex⟩⟩
    let expected ← withLocalDecl `registry .implicit
        (mkConst ``LeanerIR.Validation.SemanticsRegistry) fun registry =>
      withLocalDecl `executable .implicit
          (mkConst ``LeanerIR.Validation.ExecutableUnit) fun executable => do
        let prepared ← mkAppM ``LeanerIR.Validation.prepareExecution #[registry, toExpr unit]
        let errorType := (← inferType prepared).getAppArgs[0]!
        let success ← mkAppOptM ``Except.ok #[some errorType,
          some (mkConst ``LeanerIR.Validation.ExecutableUnit), some executable]
        let preparation ← mkEq prepared success
        let conclusion ← mkAppM ``LeanerIR.Proofs.SatisfiesFunction
          #[executable, toExpr handle, publicContract]
        -- The natives the theorems assume, in their binding order.
        let natives := ((nativeDependencies.getState env).find? base).getD #[]
        let hypotheses ← natives.mapM fun (nativeHandle, _) =>
          nativeHypothesisType unit twins families executable nativeHandle
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

/-- Publish the compiled function of a target: its rows, shape, body,
mutable parameters, the function itself, and the kernel certificate of its
compilation.  Nothing is republished. -/
private def publishCompiled (segments : Array String) (function : String)
    (handle : FunctionHandle) (compiled : Function) : TermElabM Unit := do
  let artifacts := Name.str (pathName segments) function
  if (← getEnv).contains (artifacts ++ `compiled_eq) then return
  addAbbrev (artifacts ++ `row) (← quoteRow (compiled.params ++ compiled.locals))
  addAbbrev (artifacts ++ `params) (← quoteRow compiled.params)
  addAbbrev (artifacts ++ `locals) (← quoteRow compiled.locals)
  addAbbrev (artifacts ++ `shape) (← quoteShape compiled.result)
  addAbbrev (artifacts ++ `body) (← quoteTerm compiled.body)
  addAbbrev (artifacts ++ `mutables) (← quoteMutables compiled.mutables)
  addAbbrev (artifacts ++ `compiled)
    (← quoteFunction compiled (mkConst (artifacts ++ `body)) (mkConst (artifacts ++ `mutables)))
  let lhs := mkAppN (mkConst ``compileFunction)
    #[mkConst (semanticsName segments), toExpr handle]
  let rhs := mkAppN (mkConst ``Except.ok [levelZero, levelZero])
    #[mkConst ``String, mkConst ``Function, mkConst (artifacts ++ `compiled)]
  addDecl (.thmDecl {
    name := artifacts ++ `compiled_eq
    levelParams := []
    type := ← mkEq lhs rhs
    value := ← mkEqRefl rhs })

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

/-- The hypothesis a caller's theorems take for a native: the statement a
verified callee's theorem makes, that the native's meaning satisfies its
contract. A generic native's holds at every family and instantiation; any
other's at the runtime family. -/
private def nativeBinder (segments : Array String) (entry : FunctionHandle × String)
    (generic : Bool) : CommandElabM (TSyntax ``Lean.Parser.Term.bracketedBinder) := do
  let (handle, name) := entry
  let artifacts := Name.str (pathName segments) name
  let handleTerm ← handleSyntax handle
  let params := rootIdent (artifacts ++ `params)
  let shape := rootIdent (artifacts ++ `shape)
  let contract := rootIdent (typedContractName segments name)
  let binder := mkIdent (Name.mkSimple s!"native_{name}")
  if generic then
    `(bracketedBinder| ($binder : ∀ {Θ : LeanerIR.Proofs.Denote.Skolems}
        {typeInstantiation : Array (LeanerIR.TypeId × LeanerIR.TypeId)},
        LeanerIR.Proofs.Satisfies
          (LeanerIR.Proofs.Denote.propheticMeaning executable typeInstantiation $handleTerm
            $params $shape)
          ($contract typeInstantiation)))
  else
    `(bracketedBinder| ($binder : LeanerIR.Proofs.Satisfies
        (@LeanerIR.Proofs.Denote.propheticMeaning LeanerIR.Proofs.Denote.Skolems.runtime
          executable #[] $handleTerm $params $shape)
        (@$contract LeanerIR.Proofs.Denote.Skolems.runtime)))

/-- Whether a function's specification or its module sets
`pragma verify = false`: the function's pragmas merge both. -/
private def automaticVerificationDisabled
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : Bool :=
  declaration.pragmas.any fun
    | .assign "verify" (.constant (.bool false)) _ => true
    | _ => false

/-- A member of a cycle of calls standing for its own calls, and its
contract. -/
private def cycleSelf (name : String) : Ident := mkIdent (Name.mkSimple s!"self_{name}")
private def cycleSelfVerified (name : String) : Ident :=
  mkIdent (Name.mkSimple s!"selfVerified_{name}")

/-- A function reached again along calls that are inlined, if any: such a
cycle has no finite inlining. -/
private partial def inliningCycle? (edges : Array (FunctionHandle × Array FunctionHandle))
    (path : Array FunctionHandle) (node : FunctionHandle) : Option FunctionHandle :=
  if path.contains node then some node
  else
    let next := ((edges.find? (·.1 == node)).map (·.2)).getD #[]
    next.findSome? (inliningCycle? edges (path.push node))

/-- Verify one function through its denotation.  A member of a cycle of
calls is proved over a meaning for each member of `cycle` satisfying its
contract, standing for the calls to them; its typed theorem alone is
elaborated, and the natives it assumes returned, for `verifyCycle` to
conclude. -/
private def verifyMember (reference : Syntax) (segments : Array String) (function : String)
    (prepared : ValidatedUnit) (script? : Option (TSyntax ``Lean.Parser.Tactic.tacticSeq))
    (cycle : Array (FunctionHandle × String)) :
    CommandElabM (Array (FunctionHandle × String)) := do
  let namespaceName := pathName segments
  let some unit := LeanerLang.registeredUnit? (← getEnv) namespaceName
    | throwErrorAt reference s!"unknown Leaner namespace `{namespaceName}`"
  let some (namespaceIndex, ns, functionIndex, declaration) := findFunction? unit function
    | throwErrorAt reference s!"unknown function `{function}` in `{namespaceName}`"
  let unitDefinition ← ensureUnitDefinition segments unit
  let (semanticsEq, _) ← ensureSemanticsDefinitions segments unit
  let handle : FunctionHandle := ⟨⟨namespaceIndex⟩, ⟨functionIndex⟩⟩
  let compiled ← match compileFunction prepared handle with
    | .ok compiled => pure compiled
    | .error reason => throwErrorAt reference m!"no denotation for `{function}`: {reason}"
  let some (params, result) := nativeSignature? unit ⟨namespaceIndex⟩ declaration
    | throwErrorAt reference m!"no denotation for `{function}`: its signature is not native"
  ensureContracts segments function unit namespaceIndex ns declaration params result
  let artifacts := Name.str namespaceName function
  let base := (← getCurrNamespace) ++ artifacts
  let typedVerified := typedVerifiedName segments function
  if (← getEnv).contains (base ++ `typedVerified) then
    -- A colliding name alone is not evidence of a denotation proof.
    withRef reference <| requireNativeArtifacts base
    return #[]
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
  -- The natives the theorems assume: those called here or by an inlined
  -- callee, and those a verified callee assumes.
  let mut natives : Array (FunctionHandle × String) := #[]
  -- A call to itself, and to any member of its cycle, is assumed at its
  -- contract, by fixed-point induction.  A generic function calls only
  -- itself.
  let recursive := isGeneric declaration && (compiled.body.callees #[]).contains handle
  let members := cycle.map (·.1)
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
    if isGeneric declaration then #[] else compiled.body.foldCalls recordGeneric #[]
  while let some callee := worklist.back? do
    worklist := worklist.pop
    if visited.contains callee then continue
    visited := visited.push callee
    let some calleeNs := unit.namespaces[callee.namespaceId.index]?
      | throwErrorAt reference "a callee's namespace is out of range"
    let some calleeDeclaration := calleeNs.functions[callee.functionId.index]?
      | throwErrorAt reference "a callee is out of range"
    let some calleeName := calleeNs.tables.names[calleeDeclaration.name.index]?
      | throwErrorAt reference "a callee has no name"
    let theoremName := (← getCurrNamespace) ++ typedSemanticsVerifiedName segments calleeName.name
    let handleTerm ← handleSyntax callee
    let usesContract := contractStandsFor unit callee.namespaceId calleeNs calleeDeclaration
    if calleeDeclaration.body == .absent then
      -- A native has no body: its contract is assumed, as a hypothesis.
      unless calleeDeclaration.contract.loc.isSome do
        throwErrorAt reference m!"`{function}` calls the native `{calleeName.name}`, which has no \
          contract; specify it"
      unless usesContract do
        throwErrorAt reference m!"`{function}` calls the native `{calleeName.name}`, which returns \
          a mutable reference without stating its `final` value: a caller reasons over such a \
          callee through its body, and a native has none"
      let some (nativeParams, nativeResult) :=
          nativeSignature? unit callee.namespaceId calleeDeclaration
        | throwErrorAt reference m!"the native `{calleeName.name}` has no native signature"
      ensureContracts segments calleeName.name unit callee.namespaceId.index calleeNs
        calleeDeclaration nativeParams nativeResult
      liftTermElabM (publishNativeSignature segments calleeName.name nativeParams nativeResult)
      unless natives.any (·.1 == callee) do natives := natives.push (callee, calleeName.name)
      calleePairs := calleePairs.push
        (← `(term| PProd.mk $handleTerm (PProd.mk $(Syntax.mkStrLit calleeName.name)
          $(mkIdent (Name.mkSimple s!"native_{calleeName.name}")))))
    else if (← getEnv).contains theoremName && usesContract then
      for entry in ((nativeDependencies.getState (← getEnv)).find? theoremName.getPrefix).getD #[] do
        unless natives.any (·.1 == entry.1) do natives := natives.push entry
      calleePairs := calleePairs.push
        (← `(term| PProd.mk $handleTerm
          (PProd.mk $(Syntax.mkStrLit calleeName.name) (@$(mkIdent theoremName) _ _ prepared))))
    else if calleeDeclaration.contract.loc.isSome && usesContract then
      throwErrorAt reference m!"`{function}` calls `{calleeName.name}`, which is not verified; \
        verify the callee first"
    else
      -- An unspecified callee, and one returning a reference, is inlined:
      -- its compiled body stands for its meaning through the agreement
      -- theorem.
      let calleeCompiled ← match compileFunction prepared callee with
        | .ok compiled => pure compiled
        | .error reason =>
            throwErrorAt reference m!"no denotation for the callee `{calleeName.name}`: {reason}"
      liftTermElabM (publishCompiled segments calleeName.name callee calleeCompiled)
      worklist := calleeCompiled.body.callees worklist
      instantiated := calleeCompiled.body.foldCalls recordGeneric instantiated
      edges := edges.push (callee, calleeCompiled.body.callees #[])
      let certificate := Name.str (Name.str namespaceName calleeName.name) "compiledSemantics"
      unless (← getEnv).contains certificate do
        elabCommand (← `(command|
          theorem $(rootIdent certificate)
              {registry : LeanerIR.Validation.SemanticsRegistry}
              {executable : LeanerIR.Validation.ExecutableUnit}
              (prepared : LeanerIR.Validation.prepareExecution registry
                $(mkIdent unitDefinition) = .ok executable) :
              LeanerIR.Proofs.Denote.compileFunction executable.unit $handleTerm =
                .ok $(rootIdent (compiledName segments calleeName.name)) := by
            rw [(LeanerIR.Validation.prepareExecution_unit prepared).trans $(mkIdent semanticsEq)]
            exact $(rootIdent (compiledEqName segments calleeName.name))))
      calleePairs := calleePairs.push
        (← `(term| PProd.mk $handleTerm
          (PProd.mk $(Syntax.mkStrLit calleeName.name) ($(rootIdent certificate) prepared))))
  -- Each generic call at the empty instantiation instantiates its callee's
  -- frame as the runtime computes it: a kernel-checked literal, so the
  -- callee's families key as the caller's.
  let mut instantiationCertificates : Array Term := #[]
  for (callee, typeArgs) in instantiated, index in [0:instantiated.size] do
    let name := artifacts ++ Name.mkSimple s!"frameInstantiation_{index}"
    let certificate := rootIdent name
    unless (← getEnv).contains name do
      let value := frameInstantiation prepared callee #[] typeArgs
      let calleeTerm ← handleSyntax callee
      let typeIdTerm (typeId : LeanerIR.TypeId) : CommandElabM Term :=
        `(term| (⟨$(Syntax.mkNatLit typeId.index)⟩ : LeanerIR.TypeId))
      let typeUses ← typeArgs.mapM fun (typeUse : LeanerIR.TypeUse) => do
        `(term| (⟨$(← typeIdTerm typeUse.typeId), ⟨$(Syntax.mkNatLit typeUse.loc.index)⟩⟩ :
          LeanerIR.TypeUse))
      let typeArgsTerm ← `(term| (#[$typeUses,*] : Array LeanerIR.TypeUse))
      let entries ← value.mapM fun (symbolic, concrete) => do
        `(term| ($(← typeIdTerm symbolic), $(← typeIdTerm concrete)))
      let valueTerm ← `(term| (#[$entries,*] : Array (LeanerIR.TypeId × LeanerIR.TypeId)))
      elabCommand (← `(command|
        theorem $certificate {registry : LeanerIR.Validation.SemanticsRegistry}
            {executable : LeanerIR.Validation.ExecutableUnit}
            (prepared : LeanerIR.Validation.prepareExecution registry
              $(mkIdent unitDefinition) = .ok executable) :
            LeanerIR.Proofs.Denote.frameInstantiation executable.unit $calleeTerm #[] $typeArgsTerm =
              $valueTerm := by
          rw [(LeanerIR.Validation.prepareExecution_unit prepared).trans $(mkIdent semanticsEq)]
          with_unfolding_all rfl))
    instantiationCertificates := instantiationCertificates.push
      (← `(term| $certificate prepared))
  if let some cycle := inliningCycle? edges #[] handle then
    let name := (do
      let calleeNs ← unit.namespaces[cycle.namespaceId.index]?
      let declaration ← calleeNs.functions[cycle.functionId.index]?
      let name ← calleeNs.tables.names[declaration.name.index]?
      pure name.name).getD "a callee"
    throwErrorAt reference m!"`{function}` reaches `{name}` again through callees that are \
      inlined; only a function calling itself directly is verified by induction, so specify \
      and verify the callees on the cycle"
  if recursive then
    calleePairs := calleePairs.push
      (← `(term| PProd.mk self (PProd.mk $(Syntax.mkStrLit function) selfVerified)))
  for (_, name) in cycle do
    calleePairs := calleePairs.push
      (← `(term| PProd.mk $(cycleSelf name) (PProd.mk $(Syntax.mkStrLit name)
        $(cycleSelfVerified name))))
  let saved ← get
  try
    let (twins, families) ← SpecTypes.ensureSpecTypes segments unit
    let invariants ← liftTermElabM <| withSkolems fun skolems => do
      let invariants ← loopInvariants unit ⟨namespaceIndex⟩ ns declaration compiled.params
        (compiled.params ++ compiled.locals) (mkApp (mkConst ``Skolems.codec) skolems)
        twins families
      invariants.mapM fun (site, invariant) => return (site, ← mkLambdaFVars #[skolems] invariant)
    let mut loopPairs : Array Term := #[]
    for (site, invariant) in invariants do
      let name := artifacts ++ Name.mkSimple s!"loopInvariant_{site}"
      liftTermElabM (addAbbrev name invariant)
      loopPairs := loopPairs.push
        (← `(term| ($(Syntax.mkNumLit (toString site)), $(rootIdent name))))
    liftTermElabM (publishCompiled segments function handle compiled)
    let pattern ← argumentPattern unit declaration compiled.params
    let budget := Syntax.mkNumLit (toString (leaner.verifyHeartbeats.get (← getOptions)))
    -- A generic function is proved over every skolem family and type
    -- instantiation; any other at the runtime family, a closed term the
    -- normalizer's caches keep, and the empty instantiation.
    let generic := isGeneric declaration
    let familyBinders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) ← if generic then
        pure #[← `(bracketedBinder| {Θ : LeanerIR.Proofs.Denote.Skolems}),
          ← `(bracketedBinder| {typeInstantiation : Array (LeanerIR.TypeId × LeanerIR.TypeId)})]
      else pure #[]
    let instantiation ← if generic then `(term| typeInstantiation) else `(term| #[])
    let contract ← if generic then `(term| ($typedContract $instantiation)) else pure typedContract
    -- A generic function calling itself is verified over `self` at every
    -- family and instantiation, as its calls to itself with type arguments
    -- induce.  A member of a cycle is verified over a meaning for each
    -- member satisfying its contract, standing for the calls to it.
    let selfBinders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) ← if recursive then
        pure #[← `(bracketedBinder| (self : LeanerIR.Proofs.Denote.SelfFamily $paramsTerm
            $shapeTerm)),
          ← `(bracketedBinder| (selfVerified : ∀ (family : LeanerIR.Proofs.Denote.Skolems)
            (instantiation : Array (LeanerIR.TypeId × LeanerIR.TypeId)),
            LeanerIR.Proofs.Satisfies (self family instantiation)
              (@$typedContract family instantiation)))]
      else cycle.flatMapM fun (_, name) => do
        let memberArtifacts := Name.str namespaceName name
        let memberParams := rootIdent (memberArtifacts ++ `params)
        let memberShape := rootIdent (memberArtifacts ++ `shape)
        let memberContract := rootIdent (typedContractName segments name)
        pure #[← `(bracketedBinder| ($(cycleSelf name) : LeanerIR.Proofs.Denote.HList $memberParams →
              LeanerIR.Proofs.Denote.Comp
                (LeanerIR.Proofs.Denote.ResultShape.carrier $memberShape))),
          ← `(bracketedBinder| ($(cycleSelfVerified name) :
              LeanerIR.Proofs.Satisfies $(cycleSelf name) $memberContract))]
    let nativeBinders ← natives.mapM fun entry => do
      let generic := (do
        let nativeNs ← unit.namespaces[entry.1.namespaceId.index]?
        nativeNs.functions[entry.1.functionId.index]?).any isGeneric
      nativeBinder segments entry generic
    let nativeArgs : Array Term := natives.map fun entry =>
      ⟨mkIdent (Name.mkSimple s!"native_{entry.2}")⟩
    let atFamily (command : TSyntax `command) : CommandElabM (TSyntax `command) :=
      if generic then pure command
      else `(command| attribute [local instance] LeanerIR.Proofs.Denote.Skolems.runtime in
          $command:command)
    let meanings ← if recursive then
        `(term| (⟨LeanerIR.Proofs.Denote.recursiveMeaning executable $handleTerm $paramsTerm
          $shapeTerm (self Θ typeInstantiation),
          LeanerIR.Proofs.Denote.recursiveGeneric executable $handleTerm $paramsTerm $shapeTerm
            self typeInstantiation,
          typeInstantiation⟩ : LeanerIR.Proofs.Denote.Meanings))
      else if !cycle.isEmpty then do
        let routes ← cycle.foldrM (init := ← `(term|
            LeanerIR.Proofs.Denote.propheticMeaning executable #[]))
          fun (member, name) rest => do
            let memberArtifacts := Name.str namespaceName name
            `(term| LeanerIR.Proofs.Denote.routeMeaning $(← handleSyntax member)
              $(rootIdent (memberArtifacts ++ `params)) $(rootIdent (memberArtifacts ++ `shape))
              $(cycleSelf name) $rest)
        `(term| (⟨$routes, LeanerIR.Proofs.Denote.closedGeneric executable #[], #[]⟩ :
          LeanerIR.Proofs.Denote.Meanings))
      else `(term| LeanerIR.Proofs.Denote.closedMeanings executable $instantiation)
    -- An authored script proves the obligations the closer leaves.
    let closeTactic ← if script?.isSome then
        `(tactic| leaner_denote_close residual [$loopPairs,*] with [$calleePairs,*]
          using [$instantiationCertificates,*])
      else
        `(tactic| leaner_denote_close [$loopPairs,*] with [$calleePairs,*]
          using [$instantiationCertificates,*])
    -- A loop invariant reading `old` needs the function's start: the
    -- arguments and state are recorded as a hypothesis the closer finds.
    let readsStart : Bool := match declaration.body with
      | .structured root => (loopSpecifications ns root).any fun (_, block) =>
          block.conditions.any fun condition =>
            condition.kind == .loopInvariant &&
              !(oldReferencedLocals ns condition.expression).isEmpty
      | _ => false
    -- The hypotheses the introduction names and the proof below uses share
    -- one spelling across the two quotations.
    let permitted := mkIdent `permitted
    let introduction ← if readsStart then
        `(tactic| (intro leanerArguments initialState $permitted:ident
                   have := LeanerIR.Proofs.Denote.FunctionStart.intro leanerArguments initialState
                   rcases leanerArguments with $pattern:rcasesPat))
      else `(tactic| rintro $pattern:rcasesPat initialState $permitted:ident)
    let scriptTactic ← match script? with
      | some script => `(tactic| ($script:tacticSeq))
      | none => `(tactic| skip)
    -- The signatures of the cycle's members, which route their calls.
    let memberSignatures : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) ←
      cycle.flatMapM fun (_, name) => do
        let memberArtifacts := Name.str namespaceName name
        pure #[← `(Lean.Parser.Tactic.simpLemma| $(rootIdent (memberArtifacts ++ `params)):ident),
          ← `(Lean.Parser.Tactic.simpLemma| $(rootIdent (memberArtifacts ++ `shape)):ident)]
    -- A function without type parameters is proved at the runtime family,
    -- a closed term the normalizer's caches keep.
    let typedCommand ← atFamily (← `(command|
      set_option Elab.async false in
      set_option maxHeartbeats $budget:num in
      theorem $(mkIdent typedVerified)
          {registry : LeanerIR.Validation.SemanticsRegistry}
          {executable : LeanerIR.Validation.ExecutableUnit}
          (prepared : LeanerIR.Validation.prepareExecution registry
            $(mkIdent unitDefinition) = .ok executable) $familyBinders* $nativeBinders*
            $selfBinders* :
          LeanerIR.Proofs.Satisfies
            (fun args => LeanerIR.Proofs.Spec.bind
              (LeanerIR.Proofs.Denote.Term.denote $meanings (ρ := $shapeTerm) (Γ := $row)
                $bodyIdent (LeanerIR.Proofs.Denote.initialEnv $paramsTerm $localsTerm args))
              (LeanerIR.Proofs.Denote.ResultShape.finish (Γ := $row) $mutablesTerm $shapeTerm))
            $contract := by
        apply LeanerIR.Proofs.satisfies_of_wp
        $introduction:tactic
        all_goals
        (simp only [$bodyIdent:ident, $row:ident, $paramsTerm:ident, $localsTerm:ident,
          $shapeTerm:ident, $mutablesTerm:ident, $typedContract:ident, lir_denote_norm,
          reduceCtorEq, Nat.reduceEqDiff, String.reduceBEq, String.reduceEq, String.reduceBNe,
          String.reduceNe, $memberSignatures,*] at $permitted:ident ⊢
         all_goals leaner_denote_normalize at $permitted:ident ⊢
         all_goals $closeTactic:tactic)
        $scriptTactic:tactic))
    let errorsBefore := countErrors (← get).messages
    Perf.measure s!"{namespaceName}::{function} typed" (base ++ `typedVerified)
      (elabCommand typedCommand)
    if countErrors (← get).messages > errorsBefore then
      throwErrorAt reference "leaner verification failed"
    -- A member of a cycle concludes with the whole cycle.
    unless cycle.isEmpty do return natives
    -- Elaborated synchronously, as the typed theorem is, so that an error
    -- is counted before the artifacts are accepted.
    let semanticsCommand ← atFamily (← `(command|
      set_option Elab.async false in
      theorem $(mkIdent (typedSemanticsVerifiedName segments function))
          {registry : LeanerIR.Validation.SemanticsRegistry}
          {executable : LeanerIR.Validation.ExecutableUnit}
          (prepared : LeanerIR.Validation.prepareExecution registry
            $(mkIdent unitDefinition) = .ok executable) $familyBinders* $nativeBinders* :
          LeanerIR.Proofs.Satisfies
            (LeanerIR.Proofs.Denote.propheticMeaning executable $instantiation $handleTerm $paramsTerm
              $shapeTerm)
            $contract := by
        have unitEq := (LeanerIR.Validation.prepareExecution_unit prepared).trans
          $(mkIdent semanticsEq)
        have compiledAt : LeanerIR.Proofs.Denote.compileFunction executable.unit $handleTerm =
            .ok $compiledIdent := by
          rw [unitEq]
          exact $compiledEqIdent
        $(← if recursive then
            `(tactic| exact LeanerIR.Proofs.Denote.satisfies_recursive_generic executable
              $handleTerm $compiledIdent compiledAt
              (fun family instantiation => @$typedContract family instantiation)
              (fun self selfVerified family instantiation =>
                @$(mkIdent typedVerified) _ _ prepared family instantiation $nativeArgs* self
                  selfVerified)
              typeInstantiation)
          else
            `(tactic| exact LeanerIR.Proofs.Denote.satisfies_propheticMeaning executable
              $instantiation $handleTerm $compiledIdent compiledAt $contract
              ($(mkIdent typedVerified) prepared $nativeArgs*)))))
    let publicTyped ← if generic then `(term| ($typedContract #[])) else pure typedContract
    let verifiedCommand ← `(command|
      attribute [local instance] LeanerIR.Proofs.Denote.Skolems.runtime in
      set_option Elab.async false in
      theorem $(mkIdent (verifiedName segments function))
          {registry : LeanerIR.Validation.SemanticsRegistry}
          {executable : LeanerIR.Validation.ExecutableUnit}
          (prepared : LeanerIR.Validation.prepareExecution registry
            $(mkIdent unitDefinition) = .ok executable) $nativeBinders* :
          LeanerIR.Proofs.SatisfiesFunction executable $handleTerm $publicContract :=
        LeanerIR.Proofs.Denote.satisfies_prophetic executable $handleTerm $paramsTerm $shapeTerm
          $publicTyped
          ($(mkIdent (typedSemanticsVerifiedName segments function)) prepared $nativeArgs*))
    Perf.measure s!"{namespaceName}::{function} transport" (base ++ `verified) do
      elabCommand semanticsCommand
      elabCommand verifiedCommand
    if countErrors (← get).messages > errorsBefore then
      throwErrorAt reference "leaner verification failed"
    modifyEnv fun env => completedDenotations.tag env (base ++ `typedVerified)
    modifyEnv fun env => nativeDependencies.addEntry env (base, natives)
    -- Audit fresh proofs as well as cached ones. On failure the existing
    -- rollback removes both the artifacts and the completion tag.
    withRef reference <| requireNativeArtifacts base
    return natives
  catch failure =>
    modify fun state => { saved with messages := state.messages }
    throw failure

/-- The position of the `index`th member of a cycle. -/
private def cycleIndexSyntax : Nat → CommandElabM Term
  | 0 => `(term| LeanerIR.Proofs.Denote.CycleIndex.here)
  | index + 1 => do `(term| LeanerIR.Proofs.Denote.CycleIndex.there $(← cycleIndexSyntax index))

/-- The functions of the cycle of calls through `handle`, `handle` first:
those it calls, directly or not, that call it again.  Empty when no call of
`handle` reaches it. -/
private def callCycle (prepared : ValidatedUnit) (handle : FunctionHandle) :
    Array FunctionHandle := Id.run do
  let callees (caller : FunctionHandle) : Array FunctionHandle :=
    match compileFunction prepared caller with
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

/-- Verify the members of a cycle of calls together: each over a meaning
for every member satisfying its contract, then all of them by fixed-point
induction over the cycle. -/
private def verifyCycle (reference : Syntax) (segments : Array String)
    (prepared : ValidatedUnit) (members : Array (FunctionHandle × String))
    (scripts : String → Option (TSyntax ``Lean.Parser.Tactic.tacticSeq)) :
    CommandElabM Unit := do
  let namespaceName := pathName segments
  let some unit := LeanerLang.registeredUnit? (← getEnv) namespaceName
    | throwErrorAt reference s!"unknown Leaner namespace `{namespaceName}`"
  let unitDefinition ← ensureUnitDefinition segments unit
  let (semanticsEq, _) ← ensureSemanticsDefinitions segments unit
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
      let compiled ← match compileFunction prepared member with
        | .ok compiled => pure compiled
        | .error reason => throwErrorAt reference m!"no denotation for `{name}`: {reason}"
      liftTermElabM (publishCompiled segments name member compiled)
    let mut assumed : Array (Array (FunctionHandle × String)) := #[]
    for (_, name) in members do
      assumed := assumed.push
        (← verifyMember reference segments name prepared (scripts name) members)
    let mut natives : Array (FunctionHandle × String) := #[]
    for memberNatives in assumed do
      for entry in memberNatives do
        unless natives.any (·.1 == entry.1) do natives := natives.push entry
    let nativeBinders ← natives.mapM fun entry => nativeBinder segments entry false
    let nativeArgs (entries : Array (FunctionHandle × String)) : Array Term :=
      entries.map fun entry => ⟨mkIdent (Name.mkSimple s!"native_{entry.2}")⟩
    let artifactsOf (name : String) := Name.str namespaceName name
    let compiledAt (name : String) := mkIdent (Name.mkSimple s!"compiledAt_{name}")
    let pairs ← members.mapM fun (member, name) => do
      `(term| ($(← handleSyntax member), $(rootIdent (compiledName segments name))))
    let indices ← (List.range members.size).toArray.mapM cycleIndexSyntax
    let compiledHaves ← members.mapM fun (member, name) => do
      `(tactic| have $(compiledAt name) : LeanerIR.Proofs.Denote.compileFunction executable.unit
            $(← handleSyntax member) = .ok $(rootIdent (compiledName segments name)) := by
          rw [unitEq]
          exact $(rootIdent (compiledEqName segments name)))
    let compiledAlternatives ← (members.zip indices).mapM fun ((_, name), index) =>
      `(Lean.Parser.Term.matchAltExpr| | $index => $(compiledAt name))
    let contractAlternatives ← (members.zip indices).mapM fun ((_, name), index) =>
      `(Lean.Parser.Term.matchAltExpr| | $index => $(rootIdent (typedContractName segments name)))
    let selves := mkIdent `self
    let selvesVerified := mkIdent `selvesVerified
    let mut selfArguments : Array Term := #[]
    for index in indices do
      selfArguments := selfArguments.push (← `(term| ($selves $index)))
      selfArguments := selfArguments.push (← `(term| ($selvesVerified $index)))
    let verifiedAlternatives ← ((members.zip indices).zip assumed).mapM
      fun (((_, name), index), memberNatives) =>
        `(Lean.Parser.Term.matchAltExpr| | $index =>
            $(mkIdent (typedVerifiedName segments name)) prepared $(nativeArgs memberNatives)*
              $selfArguments*)
    for ((member, name), index) in members.zip indices do
      let handleTerm ← handleSyntax member
      let params := rootIdent (artifactsOf name ++ `params)
      let shape := rootIdent (artifactsOf name ++ `shape)
      let typedContract := rootIdent (typedContractName segments name)
      let semanticsCommand ← `(command|
        attribute [local instance] LeanerIR.Proofs.Denote.Skolems.runtime in
        set_option Elab.async false in
        theorem $(mkIdent (typedSemanticsVerifiedName segments name))
            {registry : LeanerIR.Validation.SemanticsRegistry}
            {executable : LeanerIR.Validation.ExecutableUnit}
            (prepared : LeanerIR.Validation.prepareExecution registry
              $(mkIdent unitDefinition) = .ok executable) $nativeBinders* :
            LeanerIR.Proofs.Satisfies
              (LeanerIR.Proofs.Denote.propheticMeaning executable #[] $handleTerm $params $shape)
              $typedContract := by
          have unitEq := (LeanerIR.Validation.prepareExecution_unit prepared).trans
            $(mkIdent semanticsEq)
          $[$compiledHaves]*
          exact LeanerIR.Proofs.Denote.satisfies_cycle executable #[] [$pairs,*]
            (fun $compiledAlternatives:matchAlt*)
            (fun $contractAlternatives:matchAlt*)
            (fun $selves $selvesVerified index => match index with $verifiedAlternatives:matchAlt*)
            $index)
      let verifiedCommand ← `(command|
        attribute [local instance] LeanerIR.Proofs.Denote.Skolems.runtime in
        set_option Elab.async false in
        theorem $(mkIdent (verifiedName segments name))
            {registry : LeanerIR.Validation.SemanticsRegistry}
            {executable : LeanerIR.Validation.ExecutableUnit}
            (prepared : LeanerIR.Validation.prepareExecution registry
              $(mkIdent unitDefinition) = .ok executable) $nativeBinders* :
            LeanerIR.Proofs.SatisfiesFunction executable $handleTerm
              $(rootIdent (contractName segments name)) :=
          LeanerIR.Proofs.Denote.satisfies_prophetic executable $handleTerm $params $shape
            $typedContract
            ($(mkIdent (typedSemanticsVerifiedName segments name)) prepared $(nativeArgs natives)*))
      let base := (← getCurrNamespace) ++ artifactsOf name
      let errorsBefore := countErrors (← get).messages
      Perf.measure s!"{namespaceName}::{name} transport" (base ++ `verified) do
        elabCommand semanticsCommand
        elabCommand verifiedCommand
      if countErrors (← get).messages > errorsBefore then
        throwErrorAt reference "leaner verification failed"
    for (_, name) in members do
      let base := (← getCurrNamespace) ++ artifactsOf name
      modifyEnv fun env => completedDenotations.tag env (base ++ `typedVerified)
      modifyEnv fun env => nativeDependencies.addEntry env (base, natives)
    for (_, name) in members do
      withRef reference <| requireNativeArtifacts ((← getCurrNamespace) ++ artifactsOf name)
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
    (covering : Array String → CommandElabM Unit := fun _ => pure ())
    (prepared? : Option ValidatedUnit := none) :
    CommandElabM Unit := do
  let namespaceName := pathName segments
  let some unit := LeanerLang.registeredUnit? (← getEnv) namespaceName
    | throwErrorAt reference s!"unknown Leaner namespace `{namespaceName}`"
  let prepared := match prepared? with
    | some prepared => prepared
    | none => (LeanerIR.Validation.prepareSemantics unit).1
  let some (namespaceIndex, ns, functionIndex, declaration) := findFunction? unit function
    | throwErrorAt reference s!"unknown function `{function}` in `{namespaceName}`"
  let handle : FunctionHandle := ⟨⟨namespaceIndex⟩, ⟨functionIndex⟩⟩
  let base := (← getCurrNamespace) ++ Name.str namespaceName function
  let cycle := if (← getEnv).contains (base ++ `typedVerified) then #[]
    else callCycle prepared handle
  -- A generic function calling only itself is verified by induction over
  -- every family and instantiation.
  if cycle.isEmpty || (isGeneric declaration && cycle.size == 1) then
    discard <| verifyMember reference segments function prepared script? #[]
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
      if automaticVerificationDisabled memberDeclaration then
        throwErrorAt reference m!"`{function}` reaches itself through `{name.name}`, which sets \
          `pragma verify = false`; the members of a cycle of calls are verified together"
    if isGeneric memberDeclaration then
      throwErrorAt reference m!"`{function}` reaches itself through `{name.name}`; a cycle of \
        calls through a generic function is not verified"
    members := members.push (member, name.name)
  covering (members.map (·.2))
  verifyCycle reference segments prepared members fun name =>
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
def elabLeanerVerify : CommandElab := fun stx => do
  let some pathSyntax := stx[1]? | throwErrorAt stx "expected a path"
  let segments := pathSegments pathSyntax
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
callees it inlines. Natives and callees returning a reference are not among
them. -/
private def verifiedCallees (unit : ValidatedUnit) (prepared : ValidatedUnit)
    (function : FunctionHandle) : Array FunctionHandle := Id.run do
  let callees (handle : FunctionHandle) : Array FunctionHandle :=
    match compileFunction prepared handle with
    | .ok compiled => compiled.body.callees #[]
    | .error _ => #[]
  let mut found := #[]
  let mut visited := #[function]
  let mut worklist := callees function
  while let some callee := worklist.back? do
    worklist := worklist.pop
    if visited.contains callee then continue
    visited := visited.push callee
    let some ns := unit.namespaces[callee.namespaceId.index]? | continue
    let some declaration := ns.functions[callee.functionId.index]? | continue
    if declaration.body == .absent then continue
    if declaration.contract.loc.isSome &&
        contractStandsFor unit callee.namespaceId ns declaration then
      found := found.push callee
    else worklist := worklist ++ callees callee
  return found

/-- Verify `target` after the module's targets among the specified callees
its proof uses, so a caller finds its callees' theorems whatever the order
of their declarations. A target already visited, on a cycle among them
included, is not entered again. -/
private partial def verifyInOrder (unit prepared : ValidatedUnit) (segments : Array String)
    (targets : Array VerificationTarget) (visited covered : IO.Ref (Array String))
    (target : VerificationTarget) : CommandElabM Unit := do
  if (← visited.get).contains target.function then return
  visited.modify (·.push target.function)
  if let some (namespaceIndex, ns, functionIndex, _) := findFunction? unit target.function then
    for callee in verifiedCallees unit prepared ⟨⟨namespaceIndex⟩, ⟨functionIndex⟩⟩ do
      unless callee.namespaceId.index == namespaceIndex do continue
      let some declaration := ns.functions[callee.functionId.index]? | continue
      let some name := ns.tables.names[declaration.name.index]? | continue
      if let some calleeTarget := targets.find? (·.function == name.name) then
        verifyInOrder unit prepared segments targets visited covered calleeTarget
  -- An authored proof sees the module's theorems by their short names.
  let moduleNamespace := pathName segments
  let openModule (scope : Scope) := { scope with
    openDecls := .simple moduleNamespace [] :: scope.openDecls }
  -- A member of a cycle verified from another member is not entered again.
  if (← covered.get).contains target.function then return
  let scripts (name : String) := (targets.find? (·.function == name)).bind (·.script)
  let covering (names : Array String) : CommandElabM Unit := covered.modify (· ++ names)
  try
    if targets.any (·.script.isSome) then
      withScope openModule
        (verifyFunction target.reference segments target.function target.script scripts covering
          prepared)
    else verifyFunction target.reference segments target.function none scripts covering prepared
  catch error => logException error

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

/-- Elaborate a Leaner namespace command, then its theorems in its namespace,
then, with that namespace open, its `verify` items and its specified
functions without one, in source order. This registration shadows
the plain namespace elaborator. -/
@[command_elab leanerNamespaceCommand, command_elab leanerMoveModuleCommand,
  command_elab leanerRustNamespaceCommand]
def elaborateNamespaceWithVerification : CommandElab := fun stx => do
  LeanerLang.elaborateNamespace stx
  let some pathSyntax := stx.getArgs.find? (·.isOfKind ``leanerPathSyntax)
    | throwErrorAt stx "a Leaner namespace requires a path"
  -- A recursive specification function is a definition of the module.
  let segments := pathSegments pathSyntax
  if let some unit := LeanerLang.registeredUnit? (← getEnv) (pathName segments) then
    unless (recursiveSpecFunctions unit).isEmpty do
      let (twins, families) ← SpecTypes.ensureSpecTypes segments unit
      liftTermElabM (ensureSpecFunctionDefinitions unit twins families)
  let moduleNamespace := pathName segments
  let theorems := LeanerLang.theoremItems stx
  unless theorems.isEmpty do
    withModuleNamespace moduleNamespace do
      for theoremCommand in theorems do elabCommand theoremCommand
  let some unit := LeanerLang.registeredUnit? (← getEnv) (pathName segments) | return
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
      let (_, _, _, declaration) ← findFunction? unit function
      if declaration.body == .absent || automaticVerificationDisabled declaration then none
      some ⟨function, identifier, none⟩
  if targets.isEmpty then return
  let prepared := (LeanerIR.Validation.prepareSemantics unit).1
  let visited ← IO.mkRef (#[] : Array String)
  let covered ← IO.mkRef (#[] : Array String)
  for target in targets do verifyInOrder unit prepared segments targets visited covered target

end LeanerLang.Verify
