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

deriving instance ToExpr for CheckedOp
deriving instance ToExpr for CompareOp
deriving instance ToExpr for BitOp
deriving instance ToExpr for FunctionHandle
deriving instance ToExpr for LeanerIR.StructHandle
deriving instance ToExpr for Family

attribute [lir_denote_norm] LeanerLang.Contract.testVariants_nominal_self
  LeanerLang.Contract.testVariants LeanerLang.Contract.variantMember
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
  | .ref referent => return mkApp (mkConst ``NTy.ref) (← quoteNTy referent)

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
partial def quoteCarrier : (τ : NTy) → τ.carrier → MetaM Lean.Expr
  | .tuple elements, value => quoteRowValue elements value
  | .struct _ fields, value => quoteRowValue fields value
  | .enum _ names rows _, value => quoteVariantValue names rows value
  | .ref referent, value => do
      mkAppM ``Prod.mk #[toExpr value.1, ← quoteCarrier referent value.2]
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

partial def quoteRowValue : (row : NRow) → HList row → MetaM Lean.Expr
  | .nil, _ => return mkConst ``Unit.unit
  | .cons τ rest, value => do
      mkAppM ``Prod.mk #[← quoteCarrier τ value.1, ← quoteRowValue rest value.2]

partial def quoteVariantValue : (names : List String) → (rows : NRows) →
    variantCarrier names rows → MetaM Lean.Expr
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | name :: names, .cons fields rest, value => do
      let left := mkApp (mkConst ``HList) (← quoteRow fields)
      let right := mkAppN (mkConst ``variantCarrier) #[toExpr names, ← quoteRows rest]
      let _ := name
      match value with
      | .inl fields => mkAppOptM ``Sum.inl #[left, right, ← quoteRowValue _ fields]
      | .inr later => mkAppOptM ``Sum.inr #[left, right, ← quoteVariantValue names rest later]
end

def quoteVar : {Γ : NRow} → {τ : NTy} → Var Γ τ → MetaM Lean.Expr
  | .cons τ Γ, _, .here => return mkAppN (mkConst ``Var.here) #[← quoteRow Γ, ← quoteNTy τ]
  | .cons σ Γ, τ, .there rest =>
      return mkAppN (mkConst ``Var.there)
        #[← quoteRow Γ, ← quoteNTy σ, ← quoteNTy τ, ← quoteVar rest]

def quoteProj : {τ σ : NTy} → Proj τ σ → MetaM Lean.Expr
  | τ, _, .nil => return mkApp (mkConst ``Proj.nil) (← quoteNTy τ)
  | .ref τ, σ, .deref rest =>
      return mkAppN (mkConst ``Proj.deref) #[← quoteNTy τ, ← quoteNTy σ, ← quoteProj rest]
  | .struct source fields, τ, @Proj.field _ _ σ _ x rest =>
      return mkAppN (mkConst ``Proj.field) #[Lean.toExpr source, ← quoteRow fields, ← quoteNTy σ,
        ← quoteNTy τ, ← quoteVar x, ← quoteProj rest]

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
  | .unit, @Term.writeBack _ _ τ σ x path loan =>
      return mkAppN (mkConst ``Term.writeBack) #[shape, context, ← toExpr τ, ← toExpr σ,
        ← quoteVar x, ← quoteProj path, ← quoteVar loan]
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
  | .unit, @Term.publishBack _ _ τ loan =>
      return mkAppN (mkConst ``Term.publishBack) #[shape, context, ← toExpr τ, ← quoteVar loan]

partial def quoteArgs {ρ : ResultShape} {Γ : NRow} : {σs : NRow} → Args ρ Γ σs →
    MetaM Lean.Expr := fun {σs} arguments => do
  let shape ← quoteShape ρ
  let context ← quoteRow Γ
  match σs, arguments with
  | _, .nil => return mkAppN (mkConst ``Args.nil) #[shape, context]
  | .cons σ σs, .cons head tail =>
      return mkAppN (mkConst ``Args.cons) #[shape, context, ← quoteNTy σ, ← quoteRow σs,
        ← quoteTerm head, ← quoteArgs tail]
  | .cons (.ref σ) σs, @Args.reborrow _ _ τ _ _ x path tail =>
      return mkAppN (mkConst ``Args.reborrow) #[shape, context, ← quoteNTy τ, ← quoteNTy σ,
        ← quoteRow σs, ← quoteVar x, ← quoteProj path, ← quoteArgs tail]
end

mutual
/-- The callees a term calls, in order of first occurrence. -/
partial def _root_.LeanerIR.Proofs.Denote.Term.callees {ρ : ResultShape} : {Γ : NRow} → {τ : NTy} → Term ρ Γ τ →
    Array FunctionHandle → Array FunctionHandle
  | _, _, .checked _ _ left right, found => right.callees (left.callees found)
  | _, _, .compare _ left right, found => right.callees (left.callees found)
  | _, _, .equal _ left right, found => right.callees (left.callees found)
  | _, _, .not operand, found => operand.callees found
  | _, _, .logical _ left right, found => right.callees (left.callees found)
  | _, _, .bitwise _ left right, found => right.callees (left.callees found)
  | _, _, .shift _ _ value distance, found => distance.callees (value.callees found)
  | _, _, .cast _ value, found => value.callees found
  | _, _, .ite condition thenBranch elseBranch, found =>
      elseBranch.callees (thenBranch.callees (condition.callees found))
  | _, _, .let_ _ value body, found => body.callees (value.callees found)
  | _, _, .drop value body, found => body.callees (value.callees found)
  | _, _, .assign _ value, found => value.callees found
  | _, _, .const value, found => value.callees found
  | _, _, .throw1 _ code, found => code.callees found
  | _, _, .return_ value, found => value.callees found
  | _, _, .loop _ body, found => body.callees found
  | _, _, .call handle _ arguments, found =>
      arguments.callees (if found.contains handle then found else found.push handle)
  | _, _, .tuple elements, found => elements.callees found
  | _, _, .pack _ fields, found => fields.callees found
  | _, _, .variant _ _ _ fields, found => fields.callees found
  | _, _, .field _ value, found => value.callees found
  | _, _, .isVariant _ value, found => value.callees found
  | _, _, .payload _ value, found => value.callees found
  | _, _, .letRow _ value body, found => body.callees (value.callees found)
  | _, _, .letFields _ value body, found => body.callees (value.callees found)
  | _, _, .deref value, found => value.callees found
  | _, _, .mutate _ value, found => value.callees found
  | _, _, .writePlace _ _ value, found => value.callees found
  | _, _, .seqAfter value effect, found => effect.callees (value.callees found)
  | _, _, .globalRead _ key, found => key.callees found
  | _, _, .globalContains _ key, found => key.callees found
  | _, _, .globalBorrow _ key, found => key.callees found
  | _, _, .globalPublish _ key value, found => value.callees (key.callees found)
  | _, _, .globalTake _ key, found => key.callees found
  | _, _, _, found => found

partial def _root_.LeanerIR.Proofs.Denote.Args.callees {ρ : ResultShape} : {Γ σs : NRow} → Args ρ Γ σs →
    Array FunctionHandle → Array FunctionHandle
  | _, _, .nil, found => found
  | _, _, .cons head tail, found => tail.callees (head.callees found)
  | _, _, .reborrow _ _ tail, found => tail.callees found
end

/-- The compiled function as a literal, given its quoted body. -/
def quoteExports {Γ : NRow} : Exports Γ → MetaM Lean.Expr
  | .nil => return mkApp (mkConst ``Exports.nil) (← quoteRow Γ)
  | @Exports.cons _ τ x rest =>
      return mkAppN (mkConst ``Exports.cons)
        #[← quoteRow Γ, ← quoteNTy τ, ← quoteVar x, ← quoteExports rest]

def quoteFunction (f : Function) (body exports : Lean.Expr) : MetaM Lean.Expr :=
  return mkAppN (mkConst ``Function.mk)
    #[← quoteRow f.params, ← quoteRow f.locals, ← quoteShape f.result, body, exports]

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
      | _, _ => pure projection
    components := components.push (τ, component)
  let mut tuple := mkConst ``Unit.unit
  let mut tailRow : NRow := .nil
  for (τ, component) in components.reverse do
    tuple ← mkAppOptM ``Prod.mk #[some (mkApp (mkConst ``NTy.carrier) (← quoteNTy τ)),
      some (mkApp (mkConst ``HList) (← quoteRow tailRow)), some component, some tuple]
    tailRow := .cons τ tailRow
  return tuple

/-- The literal a twin's erasure unfolds to, over one binder per scalar
field, with the twin value those binders build under the range checks the
integer fields need, and the decoders the proof unfolds. `none` when a
field has a representation the bridge does not carry. -/
private partial def twinLiteral (twins : Array SpecTypes.TwinInfo) (info : SpecTypes.TwinInfo)
    (base : String) : CommandElabM (Option (Array (Ident × Term) × Term ×
      (Term → CommandElabM Term) × Array Ident)) := do
  let handle : LeanerIR.StructHandle := ⟨⟨info.namespaceIndex⟩, info.structIndex⟩
  let mut binders : Array (Ident × Term) := #[]
  let mut erasures : Array Term := #[]
  let mut components : Array Term := #[]
  -- Range checks, applied outermost first around the final `some`.
  let mut checks : Array (Ident × Term) := #[]
  let mut decoders : Array Ident := #[rootIdent (info.twin ++ `decode?)]
  for (name, rep) in info.fields do
    let binder := mkIdent (Name.mkSimple s!"{base}_{name}")
    match rep with
    | .int (.bits width) signed =>
        binders := binders.push (binder, ← `(term| Int))
        erasures := erasures.push (← `(term| LeanerIR.RuntimeValue.integer $binder))
        let fits := mkIdent (Name.mkSimple s!"{base}_{name}_fits")
        checks := checks.push (fits, ← `(term| LeanerIR.IntegerValueFits
          (LeanerIR.IntWidth.bits $(Syntax.mkNumLit (toString width))) $(quote signed) $binder))
        components := components.push (← `(term| ⟨$binder, $fits⟩))
    | .bool =>
        binders := binders.push (binder, ← `(term| Bool))
        erasures := erasures.push (← `(term| LeanerIR.RuntimeValue.bool $binder))
        components := components.push binder
    | .address =>
        binders := binders.push (binder, ← `(term| String))
        erasures := erasures.push (← `(term| LeanerIR.RuntimeValue.address $binder))
        components := components.push binder
    | .nominal twin _ =>
        let some inner := twins.find? (·.twin == twin) | return none
        let some (innerBinders, literal, _, innerDecoders) ←
            twinLiteral twins inner s!"{base}_{name}"
          | return none
        binders := binders ++ innerBinders
        erasures := erasures.push literal
        decoders := decoders ++ innerDecoders
        -- The inner checks join the outer chain; the inner value is bare.
        components := components.push (← innerComponent twins inner s!"{base}_{name}")
        checks := checks ++ (← innerChecks twins inner s!"{base}_{name}")
    | _ => return none
  let literal ← `(term| LeanerIR.RuntimeValue.nominal
    ⟨⟨$(Syntax.mkNumLit (toString info.namespaceIndex))⟩,
      $(Syntax.mkNumLit (toString info.structIndex))⟩ none #[$erasures,*])
  let checksNow := checks
  let wrap : Term → CommandElabM Term := fun inner => do
    checksNow.foldrM (init := inner) fun (fits, check) rest =>
      `(term| if $fits:ident : $check then $rest else none)
  return some (binders, literal, wrap, decoders)
where
  /-- The twin value of a nested field from its binders. -/
  innerComponent (twins : Array SpecTypes.TwinInfo) (info : SpecTypes.TwinInfo) (base : String) :
      CommandElabM Term := do
    let mut components : Array Term := #[]
    for (name, rep) in info.fields do
      let binder := mkIdent (Name.mkSimple s!"{base}_{name}")
      match rep with
      | .int _ _ =>
          let fits := mkIdent (Name.mkSimple s!"{base}_{name}_fits")
          components := components.push (← `(term| ⟨$binder, $fits⟩))
      | .nominal twin _ =>
          match twins.find? (·.twin == twin) with
          | some inner => components := components.push (← innerComponent twins inner s!"{base}_{name}")
          | none => components := components.push binder
      | _ => components := components.push binder
    `(term| (⟨$components,*⟩ : $(rootIdent info.twin)))
  /-- The range checks of a nested field's integers. -/
  innerChecks (twins : Array SpecTypes.TwinInfo) (info : SpecTypes.TwinInfo) (base : String) :
      CommandElabM (Array (Ident × Term)) := do
    let mut checks : Array (Ident × Term) := #[]
    for (name, rep) in info.fields do
      let binder := mkIdent (Name.mkSimple s!"{base}_{name}")
      match rep with
      | .int (.bits width) signed =>
          let fits := mkIdent (Name.mkSimple s!"{base}_{name}_fits")
          checks := checks.push (fits, ← `(term| LeanerIR.IntegerValueFits
            (LeanerIR.IntWidth.bits $(Syntax.mkNumLit (toString width))) $(quote signed) $binder))
      | .nominal twin _ =>
          match twins.find? (·.twin == twin) with
          | some inner => checks := checks ++ (← innerChecks twins inner s!"{base}_{name}")
          | none => pure ()
      | _ => pure ()
    return checks

/-- The decode bridge of one struct twin: decoding the literal its erasure
unfolds to is the twin of the fields, under the integer range checks. -/
private def ensureDecodeBridge (twins : Array SpecTypes.TwinInfo) (info : SpecTypes.TwinInfo) :
    CommandElabM Unit := do
  let name := info.twin ++ `decode?_fields
  if (← getEnv).contains name then return
  let some (binders, literal, wrap, decoders) ← twinLiteral twins info "field" | return
  let value ← twinLiteral.innerComponent twins info "field"
  let rhs ← wrap (← `(term| some $value))
  let binderSyntax ← binders.mapM fun (binder, type) => `(bracketedBinder| ($binder:ident : $type))
  let lemmas ← decoders.mapM fun decoder => `(Lean.Parser.Tactic.simpLemma| $decoder:ident)
  elabCommand (← `(@[lir_denote_norm] theorem $(rootIdent name):ident $binderSyntax* :
      $(rootIdent (info.twin ++ `decode?)) $literal = $rhs := by
    simp only [$lemmas,*, LeanerIR.decodeInt?]
    repeat' (first | rfl | (split <;> simp_all))))

/-- The bridge between a struct twin and its native type: the twin's
erasure is the native encoding, and the twin decodes the literal a native
encoding unfolds to.  With both, a clause's typed view of a stored
resource and the denotation's native view of the same runtime value meet. -/
private def ensureTwinBridges (unit : ValidatedUnit) (twins : Array SpecTypes.TwinInfo) :
    TermElabM Unit := do
  for info in twins do
    unless info.variants.isEmpty && info.typeParameterCount == 0 do continue
    let eraseName := info.twin ++ `erase_eq_encode
    if (← getEnv).contains eraseName then continue
    let handle : LeanerIR.StructHandle := ⟨⟨info.namespaceIndex⟩, info.structIndex⟩
    let some τ := structNTy unit handle | continue
    let τE ← quoteNTy τ
    let twinType := mkConst info.twin
    let .struct _ row := τ | continue
    let statement ← withLocalDeclD `value twinType fun value => do
      let lhs ← mkAppM (info.twin ++ `erase) #[value]
      let rhs := mkAppN (mkConst ``NTy.encode) #[τE, ← twinFields twins info row value]
      mkForallFVars #[value] (← mkEq lhs rhs)
    let proof ← withLocalDeclD `value twinType fun value => do
      mkLambdaFVars #[value] (← mkEqRefl (← mkAppM (info.twin ++ `erase) #[value]))
    addDecl (.thmDecl { name := eraseName, levelParams := [], type := statement, value := proof })
    let attr ← `(attr| lir_denote_norm)
    Lean.Elab.Term.applyAttributes eraseName #[{ name := `lir_denote_norm, stx := attr, kind := .global }]

/-- Define the raw, typed, and public contracts of a function, once. -/
def ensureContracts (segments : Array String) (function : String) (unit : ValidatedUnit)
    (namespaceIndex : Nat) (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (params : NRow) (result : ResultShape) : CommandElabM Unit := do
  let rawName := rawContractName segments function
  let typedName := typedContractName segments function
  let publicName := contractName segments function
  if (← getEnv).contains publicName then
    let some info := (← getEnv).find? typedName
      | throwError m!"`{function}` has a contract from a retired route"
    unless info.type.getUsedConstants.contains ``HList do
      throwError m!"`{function}` has a contract from a retired route"
    return
  let (twins, families) ← SpecTypes.ensureSpecTypes segments unit
  for info in twins do
    if info.variants.isEmpty && info.typeParameterCount == 0 then ensureDecodeBridge twins info
  liftTermElabM do
    ensureTwinBridges unit twins
    let raw ← buildContract unit ⟨namespaceIndex⟩ ns declaration twins families
    addAbbrev rawName raw
    let argumentsCodec := mkApp (mkConst ``hlistCodec) (← quoteRow params)
    let resultsCodec := mkApp (mkConst ``resultCodec) (← quoteShape result)
    let typed ← mkAppM ``LeanerIR.Proofs.Contract.typed
      #[argumentsCodec, resultsCodec, mkConst rawName]
    addAbbrev typedName typed
    let publicValue ← mkAppM ``LeanerIR.Proofs.Contract.runtime
      #[argumentsCodec, resultsCodec, mkConst typedName]
    addAbbrev publicName publicValue

/-! ## Loop invariants

Each loop of a compiled body gets an invariant over the locals: the
authored `invariant` clauses, quantified over the slots they read, and the
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
  | .tuple _ | .struct _ _ | .enum _ _ _ _ =>
      return some (mkAppN (mkConst ``NTy.encode) #[← quoteNTy τ, value])
  | .ref referent => logicalValue referent (← mkAppM ``Prod.snd #[value])

/-- The locals an expression tree reads. -/
private partial def referencedLocals (ns : ValidatedNamespace) (id : LeanerIR.ExprId)
    (found : Array Nat := #[]) : Array Nat :=
  match ns.expressions[id.index]? with
  | none => found
  | some expression =>
      let found := match expression.kind with
        | .localVar localId => if found.contains localId.index then found else found.push localId.index
        | _ => found
      (LeanerIR.Validation.expressionChildren expression.kind).foldl
        (fun found child => referencedLocals ns child found) found

/-- The invariant of each loop of a function, keyed by the loop's site. -/
def loopInvariants (unit : ValidatedUnit) (namespaceId : NamespaceId) (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (row : NRow) : TermElabM (Array (Nat × Lean.Expr)) := do
  let .structured root := declaration.body | return #[]
  let envType := mkApp (mkConst ``HEnv) (← quoteRow row)
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
    let parameters := declaration.locals.extract 0 declaration.signature.parameters.size
    let available := (loopHeaderLocals ns root site (parameters.map (·.id))).getD #[]
    let invariant ← withLocalDeclD `entry envType fun entry =>
      withLocalDeclD `env envType fun env => do
        let entrySlots ← slotProjections entry row.length
        let slots ← slotProjections env row.length
        let mut frame := #[]
        for (localDecl, index) in declaration.locals.zipIdx do
          if index < row.length && !localDecl.mutable && available.contains localDecl.id then
            frame := frame.push (← mkEq slots[index]! entrySlots[index]!)
        let referenced := block.conditions.foldl (fun found condition =>
          if condition.kind == .loopInvariant then referencedLocals ns condition.expression found
          else found) #[]
        let rec authoredOver (index : Nat) (binders : Array (Option Lean.Expr)) :
            TermElabM Lean.Expr := do
          if h : index < row.length then
            let τ := row[index]
            if referenced.contains index then
              withLocalDeclD (Name.mkSimple s!"slot{index}")
                  (mkApp (mkConst ``NTy.carrier) (← quoteNTy τ)) fun binder => do
                let inner ← authoredOver (index + 1) (binders.push (some binder))
                mkAppM ``Option.elim
                  #[slots[index]!, mkConst ``True, ← mkLambdaFVars #[binder] inner]
            else authoredOver (index + 1) (binders.push none)
          else
            let mut locals : Array (Option Lean.Expr) := #[]
            for (τ, binder) in row.toArray.zip binders do
              locals := locals.push (← match binder with
                | some binder => logicalValue τ binder
                | none => pure none)
            translateLoopInvariants unit namespaceId ns block locals localTypes
        let authored ← authoredOver 0 #[]
        let conjunction ← (frame.push authored).foldrM (init := mkConst ``True)
          fun clause rest => mkAppM ``And #[clause, rest]
        mkLambdaFVars #[entry, env] conjunction
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
  | .ref referent => s!"⟨{base}_loan, {carrierPattern unit base referent}⟩"
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
          ``compileFunction_agrees].contains axiomName do
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
      if dependency == ``sorryAx || dependency == ``LeanerIR.RuntimeFrame ||
          (`LeanerIR.Proofs.Denotation).isPrefixOf dependency ||
          (`LeanerIR.Proofs.ComputationAgreement).isPrefixOf dependency ||
          (`LeanerIR.Proofs.NativeBoundary).isPrefixOf dependency then
        throwError m!"artifact `{name}` retains forbidden dependency `{dependency}`"
      if base.isPrefixOf dependency || artifacts.isPrefixOf dependency then
        pending := pending.push dependency
  unless completedDenotations.isTagged env (base ++ `typedVerified) do
    throwError m!"`{function}` has no completed denotation verification"

/-- Verify one function through its denotation. -/
def verifyFunction (reference : Syntax) (segments : Array String) (function : String)
    (script? : Option (TSyntax ``Lean.Parser.Tactic.tacticSeq) := none) :
    CommandElabM Unit := do
  let route := leaner.route.get (← getOptions)
  unless route == "native" do
    throwErrorAt reference m!"legacy verification route `{route}` is disabled"
  if script?.isSome then
    throwErrorAt reference "verification through the denotation takes no proof script"
  let namespaceName := pathName segments
  let some unit := LeanerLang.registeredUnit? (← getEnv) namespaceName
    | throwErrorAt reference s!"unknown Leaner namespace `{namespaceName}`"
  let some (namespaceIndex, ns, functionIndex, declaration) := findFunction? unit function
    | throwErrorAt reference s!"unknown function `{function}` in `{namespaceName}`"
  let unitDefinition ← ensureUnitDefinition segments unit
  let (semanticsEq, _) ← ensureSemanticsDefinitions segments unit
  let prepared := (LeanerIR.Validation.prepareSemantics unit).1
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
    return
  let compiledIdent := rootIdent (compiledName segments function)
  let bodyIdent := rootIdent (Name.str (Name.str namespaceName function) "body")
  let row := rootIdent (Name.str (Name.str namespaceName function) "row")
  let paramsTerm := rootIdent (Name.str (Name.str namespaceName function) "params")
  let localsTerm := rootIdent (Name.str (Name.str namespaceName function) "locals")
  let shapeTerm := rootIdent (Name.str (Name.str namespaceName function) "shape")
  let exportsTerm := rootIdent (Name.str (Name.str namespaceName function) "exports")
  let compiledEqIdent := rootIdent (compiledEqName segments function)
  let typedContract := rootIdent (typedContractName segments function)
  let rawContract := rootIdent (rawContractName segments function)
  let publicContract := rootIdent (contractName segments function)
  let handleTerm ← `(term| (⟨⟨$(Syntax.mkNatLit namespaceIndex)⟩,
    ⟨$(Syntax.mkNatLit functionIndex)⟩⟩ : LeanerIR.FunctionHandle))
  let mut calleePairs : Array Term := #[]
  for callee in compiled.body.callees #[] do
    let some calleeNs := unit.namespaces[callee.namespaceId.index]?
      | throwErrorAt reference "a callee's namespace is out of range"
    let some calleeDeclaration := calleeNs.functions[callee.functionId.index]?
      | throwErrorAt reference "a callee is out of range"
    let some calleeName := calleeNs.tables.names[calleeDeclaration.name.index]?
      | throwErrorAt reference "a callee has no name"
    let theoremName := (← getCurrNamespace) ++ typedSemanticsVerifiedName segments calleeName.name
    unless (← getEnv).contains theoremName do
      throwErrorAt reference m!"`{function}` calls `{calleeName.name}`, which is not verified; \
        verify the callee first"
    let handleTerm ← `(term| (⟨⟨$(Syntax.mkNatLit callee.namespaceId.index)⟩,
      ⟨$(Syntax.mkNatLit callee.functionId.index)⟩⟩ : LeanerIR.FunctionHandle))
    calleePairs := calleePairs.push
      (← `(term| PProd.mk $handleTerm
        (PProd.mk $(Syntax.mkStrLit calleeName.name) ($(mkIdent theoremName) prepared))))
  let saved ← get
  try
    let invariants ← liftTermElabM <| loopInvariants unit ⟨namespaceIndex⟩ ns declaration
      (compiled.params ++ compiled.locals)
    let mut loopPairs : Array Term := #[]
    for (site, invariant) in invariants do
      let name := artifacts ++ Name.mkSimple s!"loopInvariant_{site}"
      liftTermElabM (addAbbrev name invariant)
      loopPairs := loopPairs.push
        (← `(term| ($(Syntax.mkNumLit (toString site)), $(rootIdent name))))
    liftTermElabM do
      addAbbrev (artifacts ++ `row) (← quoteRow (compiled.params ++ compiled.locals))
      addAbbrev (artifacts ++ `params) (← quoteRow compiled.params)
      addAbbrev (artifacts ++ `locals) (← quoteRow compiled.locals)
      addAbbrev (artifacts ++ `shape) (← quoteShape compiled.result)
      addAbbrev (artifacts ++ `body) (← quoteTerm compiled.body)
      addAbbrev (artifacts ++ `exports) (← quoteExports compiled.exports)
      addAbbrev (artifacts ++ `compiled)
        (← quoteFunction compiled (mkConst (artifacts ++ `body)) (mkConst (artifacts ++ `exports)))
      let lhs := mkAppN (mkConst ``compileFunction)
        #[mkConst (semanticsName segments), toExpr handle]
      let rhs := mkAppN (mkConst ``Except.ok [levelZero, levelZero])
        #[mkConst ``String, mkConst ``Function, mkConst (artifacts ++ `compiled)]
      let eqName := artifacts ++ `compiled_eq
      let eqType ← mkEq lhs rhs
      let eqValue ← mkEqRefl rhs
      addDecl (.thmDecl {
        name := eqName
        levelParams := []
        type := eqType
        value := eqValue })
    let pattern ← argumentPattern unit declaration compiled.params
    let budget := Syntax.mkNumLit (toString (leaner.verifyHeartbeats.get (← getOptions)))
    let typedCommand ← `(command|
      set_option Elab.async false in
      set_option maxHeartbeats $budget:num in
      theorem $(mkIdent typedVerified)
          {registry : LeanerIR.Validation.SemanticsRegistry}
          {executable : LeanerIR.Validation.ExecutableUnit}
          (prepared : LeanerIR.Validation.prepareExecution registry
            $(mkIdent unitDefinition) = .ok executable) :
          LeanerIR.Proofs.Satisfies
            (fun args => LeanerIR.Proofs.Spec.bind
              (LeanerIR.Proofs.Denote.Term.denote executable (ρ := $shapeTerm) (Γ := $row)
                $bodyIdent (LeanerIR.Proofs.Denote.initialEnv $paramsTerm $localsTerm args))
              (LeanerIR.Proofs.Denote.ResultShape.finish (Γ := $row) $exportsTerm $shapeTerm))
            $typedContract := by
        apply LeanerIR.Proofs.satisfies_of_wp
        rintro $pattern:rcasesPat initialState permitted
        all_goals
        (simp only [$bodyIdent:ident, $row:ident, $paramsTerm:ident, $localsTerm:ident,
          $shapeTerm:ident, $exportsTerm:ident, $typedContract:ident, $rawContract:ident, lir_denote_norm,
          reduceCtorEq, Nat.reduceEqDiff, String.reduceBEq, String.reduceEq, String.reduceBNe,
          String.reduceNe] at permitted ⊢
         leaner_denote_normalize at permitted ⊢
         leaner_denote_close [$loopPairs,*] with [$calleePairs,*]))
    let errorsBefore := countErrors (← get).messages
    Perf.measure s!"{namespaceName}::{function} typed" (base ++ `typedVerified)
      (elabCommand typedCommand)
    if countErrors (← get).messages > errorsBefore then
      throwErrorAt reference "leaner verification failed"
    let semanticsCommand ← `(command|
      theorem $(mkIdent (typedSemanticsVerifiedName segments function))
          {registry : LeanerIR.Validation.SemanticsRegistry}
          {executable : LeanerIR.Validation.ExecutableUnit}
          (prepared : LeanerIR.Validation.prepareExecution registry
            $(mkIdent unitDefinition) = .ok executable) :
          LeanerIR.Proofs.Satisfies
            (LeanerIR.Proofs.Denote.typedMeaning executable $handleTerm $compiledIdent)
            $typedContract := by
        have unitEq := (LeanerIR.Validation.prepareExecution_unit prepared).trans
          $(mkIdent semanticsEq)
        have compiledAt : LeanerIR.Proofs.Denote.compileFunction executable.unit $handleTerm =
            .ok $compiledIdent := by
          rw [unitEq]
          exact $compiledEqIdent
        exact LeanerIR.Proofs.Denote.satisfies_typedMeaning executable $handleTerm
          $compiledIdent compiledAt $typedContract ($(mkIdent typedVerified) prepared))
    let verifiedCommand ← `(command|
      theorem $(mkIdent (verifiedName segments function))
          {registry : LeanerIR.Validation.SemanticsRegistry}
          {executable : LeanerIR.Validation.ExecutableUnit}
          (prepared : LeanerIR.Validation.prepareExecution registry
            $(mkIdent unitDefinition) = .ok executable) :
          LeanerIR.Proofs.SatisfiesFunction executable $handleTerm $publicContract :=
        LeanerIR.Proofs.satisfies_runtime _ _ _ $typedContract
          ($(mkIdent (typedSemanticsVerifiedName segments function)) prepared))
    Perf.measure s!"{namespaceName}::{function} transport" (base ++ `verified) do
      elabCommand semanticsCommand
      elabCommand verifiedCommand
    if countErrors (← get).messages > errorsBefore then
      throwErrorAt reference "leaner verification failed"
    modifyEnv fun env => completedDenotations.tag env (base ++ `typedVerified)
    -- Audit fresh proofs as well as cached ones. On failure the existing
    -- rollback removes both the artifacts and the completion tag.
    withRef reference <| requireNativeArtifacts base
  catch failure =>
    modify fun state => { saved with messages := state.messages }
    throw failure

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

/-- Elaborate one in-module `verify` item once its module is registered. -/
def elabVerifyItem (segments : Array String) (item : Syntax) : CommandElabM Unit := do
  let some identifier := item[1]? | throwErrorAt item "expected a function name"
  let function := identifier.getId.toString (escape := false)
  verifyFunction identifier segments function (scriptOfOptional (item[2]?.getD .missing))

/-- Verify one function of a registered unit from outside its module. -/
syntax (name := leanerVerifyCommand)
  "#leaner_verify" leanerPath ("by" Lean.Parser.Tactic.tacticSeq)? : command

@[command_elab leanerVerifyCommand]
def elabLeanerVerify : CommandElab := fun stx => do
  let some pathSyntax := stx[1]? | throwErrorAt stx "expected a path"
  let segments := pathSegments pathSyntax
  unless segments.size ≥ 2 do
    throwErrorAt pathSyntax "a verification path must name a namespace and a function"
  let function := segments[segments.size - 1]!
  verifyFunction pathSyntax segments.pop function (scriptOfOptional (stx[2]?.getD .missing))

/-- Audit one verified function's artifacts. -/
syntax (name := leanerRequireNativeCommand) "#leaner_require_native" leanerPath : command

@[command_elab leanerRequireNativeCommand]
def elabLeanerRequireNative : CommandElab := fun stx => do
  let some pathSyntax := stx[1]? | throwErrorAt stx "expected a path"
  let (segments, function, _, _, _, _, _) ← resolvePath pathSyntax
  let base := (← getCurrNamespace) ++ Name.str (pathName segments) function
  withRef pathSyntax <| requireNativeArtifacts base

/-- Audit every function verified in this file. -/
syntax (name := leanerRequireNativeAllCommand) "#leaner_require_native_all" : command

@[command_elab leanerRequireNativeAllCommand]
def elabLeanerRequireNativeAll : CommandElab := fun _ => do
  let env ← getEnv
  let bases : Array Name := env.constants.foldStage2 (fun bases name _ =>
    if name.getString! == "typedVerified" then bases.push name.getPrefix else bases) #[]
  unless !bases.isEmpty do throwError "native audit found no verified source functions in this file"
  for base in bases.qsort Name.quickLt do requireNativeArtifacts base

/-- The in-module `verify` items of a namespace command, in source order. -/
private partial def verifyItems (stx : Syntax) : Array Syntax :=
  if stx.isOfKind ``leanerVerifyItem then #[stx]
  else stx.getArgs.flatMap verifyItems

/-- Elaborate a Leaner namespace command and then its in-module `verify`
items.  This registration shadows the plain namespace elaborator. -/
@[command_elab leanerNamespaceCommand, command_elab leanerMoveModuleCommand,
  command_elab leanerRustNamespaceCommand]
def elaborateNamespaceWithVerification : CommandElab := fun stx => do
  LeanerLang.elaborateNamespace stx
  let items := verifyItems stx
  if items.isEmpty then return
  let some pathSyntax := stx.getArgs.find? (·.isOfKind ``leanerPathSyntax)
    | throwErrorAt stx "a Leaner namespace requires a path"
  for item in items do
    try elabVerifyItem (pathSegments pathSyntax) item
    catch error => logException error

end LeanerLang.Verify
