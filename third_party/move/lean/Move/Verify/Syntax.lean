-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import Move.Attributes
import Move.Syntax
import Move.Semantics.Global
import Move.Semantics.Vector
import Move.Verify.Compare
import Move.Verify.Contract

/-!
# Declarative source contracts

`spec f ...` attaches a source contract to `f` by defining `f.contract`.
`verify f ...` proves that contract as `f.verified`. Pure functions are
verified by reduction. For `Action` functions, the declaration macro records
the unexpanded source body and the effectful `spec` form translates its
accepted source constructs directly into `Semantics.Spec`.
-/

namespace Move.Spec

/-- Pre-state observation inside an effectful `ensures` clause. The
specification elaborator consumes this syntax before ordinary term
elaboration. -/
scoped syntax (name := oldResourceTerm) "old(" term ")" : term

/-- Test whether a typed resource exists at an address in the clause's
current state. -/
scoped syntax (name := resourceExistsTerm)
  "exists<" ident ">(" term ")" : term

/-- Values accepted after `with` in an `aborts_if` clause. Move source abort
constants are `U64`, while the relational core stores codes as `Nat`. -/
class AbortCodeValue (Code : Type) where
  toNat : Code → Nat

instance : AbortCodeValue Nat := ⟨id⟩
instance : AbortCodeValue Move.U64 := ⟨Move.MoveInt.toNat⟩

def abortCodeOf [AbortCodeValue Code] (code : Code) : Nat :=
  AbortCodeValue.toNat code

@[simp] theorem abortCodeOf_nat (code : Nat) : abortCodeOf code = code := rfl
@[simp] theorem abortCodeOf_u64 (code : Move.U64) :
    abortCodeOf code = code.toNat := rfl

end Move.Spec

namespace Move.Verify.Source

open Lean Elab Command
open Lean.Parser.Term
open scoped Move Move.Spec

/-- Logical interpretation of authored `<` for the comparison operation that
the Move compiler lowers: Move's sealed structural comparison marker,
uniformly for every source type. Direct use of the marker, rather than
typeclass dispatch, ensures source verification cannot select semantics
different from the generated Move instruction. -/
def logicalLT {T : Type} (left right : T) : Prop :=
  Move.Compare.Less left right

instance (left right : T) : Decidable (logicalLT left right) :=
  inferInstanceAs (Decidable (Move.Compare.less left right = true))

/-- The comparison instruction on Move's integer types is numeric: the
sealed marker denotes the mathematical order at every width. Part of the
explicit trust base, like `logicalLT_move`. -/
axiom logicalLT_uint {W : Type} [Move.Width W] (left right : Move.UInt W) :
    logicalLT left right ↔ left.toNat < right.toNat

attribute [simp] logicalLT_uint

/-- The compiler's generic comparison marker denotes the fixed structural
ordering at a generic instantiation. The marker is opaque in executable
source, so this is the verification interface for that compiler semantic
fact. The comparator is fixed to `Move.Compare.genericLT`; it never uses a
caller-selected `LT` instance. -/
theorem logicalLT_move [Move.Compare.Total T] (left right : T) :
    logicalLT left right ↔
      @LT.lt T (Move.Compare.genericLT (T := T)) left right := Iff.rfl

attribute [simp] logicalLT_move

/-- Logical interpretation of authored `≤`, sealed to the same numeric
semantics used by the generated Move `lessEq` instruction. -/
def logicalLE {W : Type} [Move.Width W] (left right : Move.UInt W) : Prop :=
  left.toNat ≤ right.toNat

instance {W : Type} [Move.Width W] (left right : Move.UInt W) :
    Decidable (logicalLE left right) :=
  by unfold logicalLE; infer_instance

@[simp] theorem logicalLE_uint {W : Type} [Move.Width W] (left right : Move.UInt W) :
    logicalLE left right ↔ left.toNat ≤ right.toNat := Iff.rfl

/-- Sealed logical equality for authored `==`: Move's fixed structural
equality marker, uniformly for every source type, without consulting a
caller-provided `BEq` instance when a source contract is generated. -/
def logicalBEq {T : Type} (left right : T) : Bool :=
  Move.Compare.equal left right

/-- The equality instruction on Move's integer types is numeric: the sealed
marker denotes mathematical equality at every width. Part of the explicit
trust base, like `logicalBEq_move`. -/
axiom logicalBEq_uint {W : Type} [Move.Width W] (left right : Move.UInt W) :
    logicalBEq left right = true ↔ left.toNat = right.toNat

attribute [simp] logicalBEq_uint

/-- The fixed generic equality marker is the source-level representation used
by Move's compiler for a type parameter constrained by `Compare.Total`. -/
theorem logicalBEq_move [Move.Compare.Total T] (left right : T) :
    logicalBEq left right = Move.Compare.equal left right := rfl

attribute [simp] logicalBEq_move

private def lastString? : Name → Option String
  | .str _ suffix => some suffix
  | _ => none

private def sameLastName (left right : Name) : Bool :=
  left == right || match lastString? left, lastString? right with
    | some left, some right => left == right
    | _, _ => false

/-- Generated contracts package a function's parameters as one tuple, so the
operands a certified fact is about are projections of a local rather than
locals; expand a product-typed local into its components. -/
private def components (fuel : Nat) (e : Lean.Expr) (ty : Lean.Expr) :
    Lean.MetaM (Array (Lean.Expr × Lean.Expr)) := do
  let ty ← Lean.Meta.whnfR ty
  match fuel with
  | 0 => return #[(e, ty)]
  | fuel + 1 =>
    if ty.isAppOfArity ``Prod 2 then
      let args := ty.getAppArgs
      let fst ← Lean.Meta.mkAppM ``Prod.fst #[e]
      let snd ← Lean.Meta.mkAppM ``Prod.snd #[e]
      return (← components fuel fst args[0]!) ++ (← components fuel snd args[1]!)
    return #[(e, ty)]

/-- Add the certified range facts for every integer- and vector-typed
hypothesis (`x.toNat < 2^n`, `v.toList.length < 2^64`), making the
representation bounds visible to `omega` and `grind`. -/
elab "uint_bounds" : tactic => do
  Lean.Elab.Tactic.withMainContext do
    let mut goal ← Lean.Elab.Tactic.getMainGoal
    let ctx ← Lean.getLCtx
    for decl in ctx do
      if decl.isImplementationDetail then continue
      let declType ← Lean.Meta.whnfR (← Lean.instantiateMVars decl.type)
      for (target, targetType) in ← components 8 decl.toExpr declType do
       let decl : Lean.Expr := target
       let type := targetType
       if type.isAppOf ``Move.Vector then
         let bound ← Lean.Meta.mkAppM ``Move.Vector.toList_length_lt
           #[decl]
         let lengthExpr ← Lean.Meta.mkAppM ``List.length
           #[← Lean.Meta.mkAppM ``Move.Vector.toList #[decl]]
         let boundType ← Lean.Meta.mkAppM ``LT.lt
           #[lengthExpr, Lean.mkNatLit (2 ^ 64)]
         goal ← (← goal.assert (Lean.Name.mkSimple "vectorBound") boundType
           bound).intro1P <&> (·.2)
         continue
       if type.isAppOf ``Move.MoveInt then
         -- `MoveInt S W`: the sign tag is the first type argument, the width
         -- tag the second.  Unsigned locals get the natural-number bound the
         -- specification language uses; signed locals get both `Int` bounds.
         let args := type.getAppArgs
         let bits? : Option Nat :=
           match args[1]? with
           | some (Lean.Expr.const tag _) =>
               if tag == ``Move.W8 then some 8
               else if tag == ``Move.W16 then some 16
               else if tag == ``Move.W32 then some 32
               else if tag == ``Move.W64 then some 64
               else if tag == ``Move.W128 then some 128
               else if tag == ``Move.W256 then some 256
               else none
           | _ => none
         let signed? : Option Bool :=
           match args[0]? with
           | some (Lean.Expr.const tag _) =>
               if tag == ``Move.Unsigned then some false
               else if tag == ``Move.Signed then some true
               else none
           | _ => none
         match signed? with
         | some false =>
             let bound ← Lean.Meta.mkAppM ``Move.UInt.toNat_lt #[decl]
             let boundType ← match bits? with
               | some bits =>
                   let toNatExpr ← Lean.Meta.mkAppM ``Move.MoveInt.toNat #[decl]
                   Lean.Meta.mkAppM ``LT.lt #[toNatExpr, Lean.mkNatLit (2 ^ bits)]
               | none => Lean.Meta.inferType bound
             goal ← (← goal.assert (Lean.Name.mkSimple "uintBound") boundType bound).intro1P
               <&> (·.2)
             -- Only the natural-number bound.  The `Int` view of the same
             -- fact was asserted here while the checked rules still spoke the
             -- neutral `Int` form; now that they are per-view, adding it puts
             -- `Int` atoms into otherwise pure-`Nat` goals and makes the
             -- decision procedures reason over a mixed domain for nothing.
         | some true =>
             let lower ← Lean.Meta.mkAppM ``Move.SInt.neg_halfSize_le_toInt #[decl]
             goal ← (← goal.assert (Lean.Name.mkSimple "sintLower")
               (← Lean.Meta.inferType lower) lower).intro1P <&> (·.2)
             let upper ← Lean.Meta.mkAppM ``Move.SInt.toInt_lt_halfSize #[decl]
             goal ← (← goal.assert (Lean.Name.mkSimple "sintUpper")
               (← Lean.Meta.inferType upper) upper).intro1P <&> (·.2)
         | none => pure ()
    Lean.Elab.Tactic.replaceMainGoal [goal]

/-- Add the data invariant of every certified-typed hypothesis, which is what
makes the invariant "available wherever the value is" without naming the type
or its generated condition.

Deliberately *not* folded into `uint_bounds`.  A width bound is one cheap
atomic fact; a data invariant can be an arbitrarily large predicate — the
ordered map's is a sortedness condition over the whole entry list — and
asserting one into every context the automatic cascade normalizes costs far
more than the proofs that want it save.  It is a tactic a proof asks for. -/
elab "data_invariants" : tactic => do
  Lean.Elab.Tactic.withMainContext do
    let mut goal ← Lean.Elab.Tactic.getMainGoal
    let ctx ← Lean.getLCtx
    for decl in ctx do
      if decl.isImplementationDetail then continue
      let declType ← Lean.Meta.whnfR (← Lean.instantiateMVars decl.type)
      for (target, targetType) in ← components 8 decl.toExpr declType do
        let some typeName := targetType.getAppFn.constName? | continue
        unless (Move.dataInvariant? (← Lean.getEnv) typeName).isSome do continue
        try
          let invariant ← Lean.Meta.mkAppM (typeName ++ `invariant) #[target]
          -- Assert the condition with its generated name unfolded: this runs
          -- after the cascade's normalization, so `move_invariant_norm` would
          -- no longer see it.
          let condition ← Lean.Meta.inferType invariant
          let condition := (← Lean.Meta.unfoldDefinition? condition).getD condition
          goal ← (← goal.assert (Lean.Name.mkSimple "dataInvariant")
            condition invariant).intro1P <&> (·.2)
        catch _ => pure ()
    Lean.Elab.Tactic.replaceMainGoal [goal]

/-- Source retained by the `fun` command for later specification generation.
This is deliberately syntax, rather than LIR or Move IR: verification is
defined over the authored source constructs. -/
private structure Declaration where
  resultType : Syntax
  value : Syntax
  deriving Inhabited

private initialize declarations : EnvExtension (NameMap Declaration) ←
  registerEnvExtension (pure {})

syntax (name := move_source)
  "move_source" "(" term ", " str ")" : attr

initialize moveSourceAttr : Unit ← Lean.registerBuiltinAttribute {
  name := Name.mkSimple "move_source"
  descr := "retained Move source for relational specification generation"
  add := fun declarationName stx _ => do
    let `(attr| move_source ($resultType:term, $encoded:str)) := stx
      | throwErrorAt stx "invalid retained Move source"
    let some source := encoded.raw.isStrLit?
      | throwErrorAt encoded "expected encoded Move source"
    let value ← match Lean.Parser.runParserCategory (← getEnv) `term source with
      | .ok value => pure value
      | .error message => throwErrorAt encoded
          "failed to restore retained Move source: {message}\n{source}"
    let declaration := { resultType := resultType.raw, value }
    modifyEnv fun env => declarations.modifyState env
      (·.insert declarationName declaration)
}

private def declarationFor (function : Syntax) : CommandElabM Declaration := do
  let name := (← getCurrNamespace) ++ function.getId
  let some declaration := declarations.getState (← getEnv) |>.find? name
    | throwErrorAt function
        "no retained Move source declaration; use `fun`, not `def`, for an effectful Move function"
  pure declaration

private partial def findTypeApplication? (name : Name) (stx : Syntax) : Option Syntax :=
  if stx.isOfKind ``Lean.Parser.Term.app && stx.getNumArgs == 2 &&
      stx[0].isIdent && sameLastName stx[0].getId name then
    if stx[1].isOfKind `null && stx[1].getNumArgs == 1 then some stx[1][0]
    else some stx[1]
  else
    stx.getArgs.findSome? (findTypeApplication? name)

private partial def eraseReferenceType (type : TSyntax `term) : TSyntax `term :=
  match type with
  | `(($inner:term)) => eraseReferenceType inner
  | `(& $referent:term) => referent
  | _ => type

private def actionResultType (declaration : Declaration) : CommandElabM (TSyntax `term) := do
  match findTypeApplication? ``Move.Action declaration.resultType with
  | some result => pure (eraseReferenceType ⟨result⟩)
  | none =>
      -- Pure `do` functions still retain a source body for verification.
      pure (eraseReferenceType ⟨declaration.resultType⟩)

/-- Result type retained for an effectful Move source declaration.  This is
also used when a recursive function supplies its relational source semantics
as an ordinary, predeclared `f.sourceSpec` helper. -/
def resultTypeOf (function : Syntax) : CommandElabM (TSyntax `term) := do
  actionResultType (← declarationFor function)

/-- Whether a Move source function is effectful — its result is an `Action`, so
its contract observes global state rather than being a pure value predicate. -/
def isEffectfulFunction (function : Syntax) : CommandElabM Bool := do
  return (findTypeApplication? ``Move.Action (← declarationFor function).resultType).isSome

private def sourceBody (declaration : Declaration) : CommandElabM (TSyntax `term) := do
  if declaration.value.isOfKind ``Lean.Parser.Term.paren then
    pure ⟨declaration.value[1]⟩
  else
    pure ⟨declaration.value⟩

private def fieldParts : Name → List String
  | .anonymous => []
  | .str base part => fieldParts base ++ [part]
  | .num _ _ => []

private partial def splitFieldPath (place : TSyntax `term) :
    (TSyntax `term × Array (TSyntax `ident)) :=
  if place.raw.isOfKind ``Lean.Parser.Term.proj && place.raw.getNumArgs >= 3 &&
      place.raw[2].isIdent then
    let base : TSyntax `term := ⟨place.raw[0]⟩
    let fields := fieldParts place.raw[2].getId |>.toArray.map
      (fun field => mkIdentFrom place (Name.mkSimple field))
    let result := splitFieldPath base
    (result.1, result.2 ++ fields)
  else
    (place, #[])

def canonicalResourceName (resource : TSyntax `ident) : CommandElabM Name := do
  try
    resolveGlobalConstNoOverload resource.raw
  catch _ =>
    pure ((← getCurrNamespace) ++ resource.getId)

private def pushResource (resources : Array (TSyntax `ident))
    (resource : TSyntax `ident) : CommandElabM (Array (TSyntax `ident)) := do
  let resourceName ← canonicalResourceName resource
  for existing in resources do
    if resourceName == (← canonicalResourceName existing) then
      return resources
  return resources.push resource

private def isResourceIdentifier (resource : TSyntax `ident) : CommandElabM Bool := do
  let env ← getEnv
  let current := (← getCurrNamespace) ++ resource.getId
  pure <| Move.moveKeyAttr.hasTag env current ||
    Move.moveKeyAttr.hasTag env resource.getId

/-- The leading type constructor of a (possibly parenthesized, applied, or
ascribed) type expression: `Vault` from `(Vault T)`, `Counter` from
`({ … } : Counter)`. -/
private partial def typeHead? (stx : Syntax) : Option (TSyntax `ident) :=
  if stx.isIdent then some ⟨stx⟩
  else if stx.isOfKind ``Lean.Parser.Term.paren then
    stx.getArgs[1]? |>.bind typeHead?
  else if stx.isOfKind ``Lean.Parser.Term.app then
    stx.getArgs[0]? |>.bind typeHead?
  else if stx.isOfKind ``Lean.Parser.Term.typeAscription then
    -- `(expr : type)` — the type is the fourth child, wrapped in a `null` node.
    (stx.getArgs[3]?.bind (·.getArgs[0]?)) |>.bind typeHead?
  else if stx.isOfKind nullKind then
    stx.getArgs[0]? |>.bind typeHead?
  else none

/-- The resource family named by a global-storage primitive application
(`exists_`/`moveFrom` name it directly; `moveTo` names it through the published
value's ascription), if `stx` is such an application. -/
private def globalPrimitiveResource? (stx : Syntax) :
    CommandElabM (Option (TSyntax `ident)) := do
  unless stx.isOfKind ``Lean.Parser.Term.app do return none
  let head := stx[0]
  unless head.isIdent do return none
  let some name ← (try pure (some (← resolveGlobalConstNoOverload head))
      catch _ => pure none) | return none
  let arguments := stx[1].getArgs
  if name == ``Move.exists_ || name == ``Move.moveFrom then
    return arguments[0]?.bind typeHead?
  else if name == ``Move.moveTo then
    return arguments[1]?.bind typeHead?
  else
    return none

private partial def collectResources (stx : Syntax)
    (resources : Array (TSyntax `ident) := #[]) : CommandElabM (Array (TSyntax `ident)) := do
  let mut resources := resources
  if let some resource ← globalPrimitiveResource? stx then
    if ← isResourceIdentifier resource then
      resources ← pushResource resources resource
  if stx.isOfKind ``Move.borrowTerm || stx.isOfKind ``Move.borrowMutTerm then
    if let some place := stx[1]? then
      let (root, _) := splitFieldPath ⟨place⟩
      match root with
      | `($resource:ident[$_:term]) =>
          if ← isResourceIdentifier resource then
            resources ← pushResource resources resource
      | _ => pure ()
  else if stx.isOfKind ``Move.borrowIndexTerm ||
      stx.isOfKind ``Move.borrowMutIndexTerm then
    if let some candidate := stx[1]? then
      if candidate.isIdent then
        let resource : TSyntax `ident := ⟨candidate⟩
        if ← isResourceIdentifier resource then
          resources ← pushResource resources resource
  for child in stx.getArgs do
    resources ← collectResources child resources
  pure resources

private def globalPlace (place : TSyntax `term) :
    CommandElabM (TSyntax `ident × TSyntax `term × Array (TSyntax `ident)) := do
  let (root, fields) := splitFieldPath place
  match root with
  | `($resource:ident[$key:term]) => pure (resource, key, fields)
  | `(getElem $resource:ident $key:term $_:term) => pure (resource, key, fields)
  | _ => throwErrorAt place
      "automatic source specifications currently expect a global place `Resource[key]`"

private def projectPath (owner : TSyntax `term)
    (fields : Array (TSyntax `ident)) : CommandElabM (TSyntax `term) := do
  fields.foldlM (init := owner) fun value field => `($value.$field)

private partial def replaceIdentifier (name : Name) (replacement stx : Syntax) : Syntax :=
  if stx.isIdent && stx.getId == name then replacement
  else stx.setArgs (stx.getArgs.map (replaceIdentifier name replacement))

private partial def updatePath (owner newValue : TSyntax `term)
    (fields : List (TSyntax `ident)) : CommandElabM (TSyntax `term) := do
  match fields with
  | [] => pure newValue
  | field :: rest =>
      let oldField ← `($owner.$field)
      let newField ← updatePath oldField newValue rest
      let ownerPlaceholder := `_moveSpecUpdateOwner
      let valuePlaceholder := `_moveSpecUpdateValue
      let source := "{ _moveSpecUpdateOwner with " ++ field.getId.toString ++
        " := _moveSpecUpdateValue }"
      let parsed ← match Lean.Parser.runParserCategory (← getEnv) `term source with
        | .ok parsed => pure parsed
        | .error message => throwErrorAt field
            "failed to generate update for source field `{field.getId}`: {message}"
      let parsed := replaceIdentifier ownerPlaceholder owner.raw parsed
      let parsed := replaceIdentifier valuePlaceholder newField.raw parsed
      pure ⟨parsed⟩

/-- The type a mutable parameter refers to, if source translation can name
it. -/
private def referentTypeName? (type : TSyntax `term) : CommandElabM (Option Name) := do
  let head := if type.raw.isIdent then type.raw else
    (Lean.Syntax.getArgs type.raw)[0]? |>.getD type.raw
  unless head.isIdent do return none
  try
    return some (← resolveGlobalConstNoOverload head)
  catch _ =>
    return none

/-- Rebuild the owner of a mutated field.  A certified owner is re-created
through `Spec.certified`, which is where its data invariant is owed; an
ordinary owner is a plain structure update. -/
private def rebuildOwner (owner newValue : TSyntax `term)
    (fields : List (TSyntax `ident)) (certified? : Option (Name × Name)) :
    CommandElabM (Option (TSyntax `term)) := do
  let some (typeName, invariantName) := certified? | return none
  let field :: rest := fields
    | throwError "a mutable borrow into `{typeName}` must select a field"
  let env ← getEnv
  let some info := getStructureInfo? env typeName | return none
  let dataFields := info.fieldNames.filter (· != `invariant)
  let arguments ← dataFields.mapM fun fieldName =>
    if fieldName == field.getId then
      if rest.isEmpty then pure newValue
      else do updatePath (← `($owner.$field)) newValue rest
    else
      `($owner.$(mkIdent fieldName))
  let rawValue ← `($(mkIdent (typeName ++ `Raw ++ `mk)) $arguments*)
  let holds := mkIdentFrom field `_moveSpecInvariant
  let built ← `(fun $holds =>
    $(mkIdent (typeName ++ `mk)) $arguments* $holds)
  return some (← `(Move.Semantics.Spec.certified
    (Invariant := $(mkIdent invariantName) $rawValue) $built))

private structure ResourceBinding where
  typeName : Name
  descriptor : TSyntax `term

private def resourceFor (resources : Array ResourceBinding)
    (resource : TSyntax `ident) : CommandElabM (TSyntax `term) := do
  let wanted ← canonicalResourceName resource
  let some binding := resources.find? fun binding =>
      binding.typeName == wanted
    | throwErrorAt resource
        "no resource descriptor was supplied for `{resource.getId}`"
  pure binding.descriptor

private def hasResource (resources : Array ResourceBinding)
    (candidate : TSyntax `ident) : CommandElabM Bool := do
  let candidate ← canonicalResourceName candidate
  return resources.any fun binding => binding.typeName == candidate

private def localVectorPlace? (contextResources : Array ResourceBinding)
    (place : TSyntax `term) :
    CommandElabM (Option (TSyntax `ident × TSyntax `term × Array (TSyntax `ident))) := do
  let (root, fields) := splitFieldPath place
  match root with
  | `($owner:ident[$index:term]) =>
      if ← hasResource contextResources owner then pure none
      else pure (some (owner, index, fields))
  | `(getElem $owner:ident $index:term $_:term) =>
      if ← hasResource contextResources owner then pure none
      else pure (some (owner, index, fields))
  | _ => pure none

private def localPlace? (contextResources : Array ResourceBinding)
    (place : TSyntax `term) : CommandElabM (Option (TSyntax `ident × Array (TSyntax `ident))) := do
  if place.raw.isIdent then
    let parts := fieldParts place.raw.getId
    if let ownerName :: fields := parts then
      let owner := mkIdentFrom place (Name.mkSimple ownerName)
      if !(← hasResource contextResources owner) then
        return some (owner, fields.toArray.map fun field =>
          mkIdentFrom place (Name.mkSimple field))
  let (root, fields) := splitFieldPath place
  unless root.raw.isIdent do return none
  let owner : TSyntax `ident := ⟨root.raw⟩
  if ← hasResource contextResources owner then return none
  return some (owner, fields)

private structure VerificationLoopFrame where
  sourceLabel? : Option Name
  recursive : TSyntax `term
  after : TSyntax `term
  assigned : List (TSyntax `ident)
  state : List (TSyntax `ident)

private structure TranslationContext where
  world : TSyntax `term
  resources : Array ResourceBinding
  functionName : Name
  recursiveSpec? : Option (TSyntax `term) := none
  mutation? : Option (TSyntax `ident) := none
  /-- The type and data invariant of the mutable parameter's referent, when it
  certifies one.  Rebuilding it after a field mutation is a creation site. -/
  certifiedOwner? : Option (Name × Name) := none
  loops : List VerificationLoopFrame := []

private def mutationValue (context : TranslationContext)
    (owner : TSyntax `ident) : CommandElabM (TSyntax `term) := do
  if let some mutation := context.mutation? then
    if mutation.getId == owner.getId then
      return ← `(Move.Semantics.Mutation.read $owner)
  pure ⟨owner.raw⟩

private def application? (term : TSyntax `term) :
    Option (TSyntax `term × Array (TSyntax `term)) :=
  if term.raw.isOfKind ``Lean.Parser.Term.app && term.raw.getNumArgs == 2 then
    let arguments := term.raw[1].getArgs.map (⟨·⟩)
    some (⟨term.raw[0]⟩, arguments)
  else
    none

/-- Core primitives whose executable Move behavior is not yet represented by
the automatically generated source semantics. -/
private def unsupportedSourceOperation (name : Name) : Bool :=
  name == ``Move.borrowLocal || name == ``Move.borrowLocalMut ||
  name == ``Move.borrowGlobal || name == ``Move.borrowGlobalMut ||
  name == ``Move.borrowField || name == ``Move.borrowFieldMut ||
  name == ``Move.borrowElem || name == ``Move.borrowElemMut ||
  name == ``Move.freeze || name == ``Move.read || name == ``Move.readImm ||
  name == ``Move.write || name == ``Move.assert || name == ``Move.abort ||
  name == ``Move.Vector.get || name == ``Move.Vector.set

private def unsupportedSourceOperation? (term : TSyntax `term) :
    CommandElabM (Option Name) := do
  let some (head, _) := application? term | return none
  unless head.raw.isIdent do return none
  let name ← try
      pure (some (← resolveGlobalConstNoOverload head.raw))
    catch _ => pure none
  return name.filter unsupportedSourceOperation

/-- Refuse source fragments for which `Spec.pure` would erase an executable
Move effect or abort. -/
private partial def ensureSupportedSourceTerm (term : TSyntax `term) :
    CommandElabM Unit := do
  if let some operation ← unsupportedSourceOperation? term then
    throwErrorAt term
      "automatic source specifications do not yet model `{operation}`; provide an explicit `sourceSpec` or omit `verify`"
  for child in term.raw.getArgs do
    ensureSupportedSourceTerm ⟨child⟩

/-- Arithmetic must be sequenced through `Checked.*Spec` so its VM abort
behavior remains visible. `rewritePure` is used only in source contexts which
cannot currently sequence a `Spec`, such as vector indices. -/
private partial def containsArithmetic (term : Syntax) : Bool :=
  (term.getNumArgs == 3 && term[1].isAtom &&
    (term[1].getAtomVal == "+" || term[1].getAtomVal == "-" ||
      term[1].getAtomVal == "*" || term[1].getAtomVal == "/" ||
      term[1].getAtomVal == "%" || term[1].getAtomVal == "<<<" ||
      term[1].getAtomVal == ">>>")) ||
  (term.isIdent && (match term.getId with
    | .str _ "cast" => true
    | _ => false)) ||
  term.getArgs.any containsArithmetic

private def checkedArithmeticCall? (term : TSyntax `term) :
    CommandElabM (Option (Name × TSyntax `term × TSyntax `term)) := do
  let some (head, arguments) := application? term | return none
  unless head.raw.isIdent && arguments.size == 2 do return none
  let functionName ← try
      pure (some (← resolveGlobalConstNoOverload head.raw))
    catch _ => pure none
  let operation? := functionName.bind fun functionName =>
    -- one map: the operation markers are shared by both signednesses
    if functionName == ``Move.MoveInt.add then some ``Move.Semantics.Checked.addSpec
    else if functionName == ``Move.MoveInt.sub then some ``Move.Semantics.Checked.subSpec
    else if functionName == ``Move.MoveInt.mul then some ``Move.Semantics.Checked.mulSpec
    else if functionName == ``Move.MoveInt.div then some ``Move.Semantics.Checked.divSpec
    else if functionName == ``Move.MoveInt.mod then some ``Move.Semantics.Checked.modSpec
    else if functionName == ``Move.MoveInt.shl then some ``Move.Semantics.Checked.shlSpec
    else if functionName == ``Move.MoveInt.shr then some ``Move.Semantics.Checked.shrSpec
    else if functionName == ``Move.MoveInt.cast then some ``Move.Semantics.Checked.castSpec
    else none
  return operation?.map (·, arguments[0]!, arguments[1]!)

private partial def containsCheckedArithmeticCall (term : Syntax) :
    CommandElabM Bool := do
  if (← checkedArithmeticCall? ⟨term⟩).isSome then return true
  for child in term.getArgs do
    if ← containsCheckedArithmeticCall child then return true
  return false

private def resolvePureMoveFunction? (identifier : TSyntax `ident) :
    CommandElabM (Option Name) := do
  let env ← getEnv
  let functionName? ← try
      pure (some (← resolveGlobalConstNoOverload identifier.raw))
    catch _ => pure none
  let some functionName := functionName? | return none
  unless Move.isMoveFunction env functionName do
    return none
  let some declaration := declarations.getState env |>.find? functionName
    | return none
  if (findTypeApplication? ``Move.Action declaration.resultType).isSome then
    return none
  return some functionName

private def pureMoveCallAtRoot? (term : TSyntax `term) :
    CommandElabM (Option Name) := do
  let some (head, _) := application? term | return none
  unless head.raw.isIdent do return none
  resolvePureMoveFunction? ⟨head.raw⟩

private partial def nestedPureMoveCall? (term : Syntax) :
    CommandElabM (Option Name) := do
  if let some functionName ← pureMoveCallAtRoot? ⟨term⟩ then
    return some functionName
  for child in term.getArgs do
    if let some functionName ← nestedPureMoveCall? child then
      return some functionName
  return none

private partial def rewritePure (mutation? : Option (TSyntax `ident))
    (term : TSyntax `term) : CommandElabM (TSyntax `term) := do
  ensureSupportedSourceTerm term
  if containsArithmetic term.raw || (← containsCheckedArithmeticCall term.raw) then
    throwErrorAt term
      "automatic source specifications do not yet support arithmetic in this context; bind it to a local first"
  if let some functionName ← nestedPureMoveCall? term.raw then
    throwErrorAt term
      "automatic source specifications do not yet model pure Move callee `{functionName}`; inline it or omit `verify`"
  match term with
  | `($value:ident) =>
      let parts := fieldParts value.getId
      if parts.length > 1 && !(← getEnv).contains value.getId then
        let owner := mkIdentFrom value (Name.mkSimple parts.head!)
        let fields := parts.tail.toArray.map fun field =>
          mkIdentFrom value (Name.mkSimple field)
        let ownerTerm ← if let some mutation := mutation? then
          if mutation.getId == owner.getId then
            `(Move.Semantics.Mutation.read $owner)
          else
            pure ⟨owner.raw⟩
        else
          pure ⟨owner.raw⟩
        return ← projectPath ownerTerm fields
      if let some mutation := mutation? then
        if value.getId == mutation.getId then
          return ← `(Move.Semantics.Mutation.read $value)
      pure term
  | `(* $reference:term) =>
      if let some mutation := mutation? then
        if reference.raw.isIdent && reference.raw.getId == mutation.getId then
          return ← `(Move.Semantics.Mutation.read $reference)
      rewritePure mutation? reference
  | `(($value:term)) => `(($(← rewritePure mutation? value)))
  | `($lhs:term < $rhs:term) =>
      `(logicalLT $(← rewritePure mutation? lhs) $(← rewritePure mutation? rhs))
  | `($lhs:term <= $rhs:term) =>
      `(logicalLE $(← rewritePure mutation? lhs) $(← rewritePure mutation? rhs))
  | `($lhs:term == $rhs:term) =>
      `(logicalBEq $(← rewritePure mutation? lhs) $(← rewritePure mutation? rhs))
  | _ => pure term

private inductive VectorMutationCall where
  | insert (reference index value : TSyntax `term)
  | remove (reference index : TSyntax `term)

private def nativeVectorMutationCall? (functionName : Name)
    (reference : TSyntax `term) (arguments : Array (TSyntax `term)) :
    Option VectorMutationCall :=
  if functionName == ``Move.Vector.insert ||
      functionName == ``Move.MutRef.insert then
    if arguments.size == 2 then
      some (.insert reference arguments[0]! arguments[1]!)
    else
      none
  else if functionName == ``Move.Vector.remove ||
      functionName == ``Move.MutRef.remove then
    if arguments.size == 1 then
      some (.remove reference arguments[0]!)
    else
      none
  else
    none

/-- Receiver notation does not retain which declaration it resolves to in the
raw source syntax used for automatic specifications. Reject it rather than
assuming that a field named `insert` or `remove` is a native vector operation.
Use the fully qualified `Move.Vector` operation instead. -/
private def receiverStyleVectorMutation? (term : TSyntax `term) : Bool :=
  match application? term with
  | none => false
  | some (head, _) =>
      if head.raw.isOfKind ``Lean.Parser.Term.proj then
        let projection := head.raw.getArgs
        match projection[2]? with
        | some field => field.isIdent &&
            (field.getId == `insert || field.getId == `remove)
        | none => false
      else if head.raw.isIdent then
        match head.raw.getId with
        | Name.str _ field => field == "insert" || field == "remove"
        | _ => false
      else
        false

private def vectorMutationCall? (term : TSyntax `term) :
    CommandElabM (Option VectorMutationCall) :=
  do
    let some (head, arguments) := application? term | return none
    if head.raw.isIdent then
      let resolved? ← try
          pure (some (← resolveGlobalConstNoOverload head.raw))
        catch _ => pure none
      if let some functionName := resolved? then
        if (← getEnv).contains functionName then
          let some reference := arguments[0]? | return none
          return nativeVectorMutationCall? functionName reference
            (arguments.extract 1 arguments.size)
    return none

private def packCallArguments (_anchor : Syntax)
    (arguments : Array (TSyntax `term)) : CommandElabM (TSyntax `term) := do
  match arguments.size with
  | 0 => `(())
  | 1 => pure arguments[0]!
  | _ =>
      let reversed := arguments.toList.reverse
      reversed.tail.foldlM (init := reversed.head!) fun result argument =>
        `(($argument, $result))

private def resolveMoveFunction? (identifier : TSyntax `ident) :
    CommandElabM (Option Name) := do
  let env ← getEnv
  let functionName? ← try
      pure (some (← resolveGlobalConstNoOverload identifier.raw))
    catch _ => pure none
  let some functionName := functionName? | return none
  if Move.isMoveFunction env functionName then
    return some functionName
  return none

private def hasExplicitMutableParameter (functionName : Name) :
    CommandElabM Bool :=
  liftTermElabM do
    let function ← Lean.Meta.mkConstWithFreshMVarLevels functionName
    let (parameters, binderInfos, _) ←
      Lean.Meta.forallMetaTelescope (← Lean.Meta.inferType function)
    for (parameter, binderInfo) in parameters.zip binderInfos do
      if binderInfo.isExplicit then
        let parameterType ← Lean.Meta.whnf (← Lean.Meta.inferType parameter)
        if parameterType.isAppOfArity ``Move.MutRef 1 then
          return true
    return false

private def effectfulCallSpec?
    (translateArgument : TSyntax `term → CommandElabM (TSyntax `term))
    (context : TranslationContext)
    (term : TSyntax `term) : CommandElabM (Option (TSyntax `term)) := do
  let (head, arguments, markedContinue) ← match term with
    | `(continue $head:term $arguments:term*) =>
        pure (head, arguments, true)
    | _ =>
        let some (head, arguments) := application? term | return none
        pure (head, arguments, false)
  unless head.raw.isIdent do return none
  let identifier : TSyntax `ident := ⟨head.raw⟩
  let some functionName ← resolveMoveFunction? identifier | return none
  let some declaration := declarations.getState (← getEnv) |>.find? functionName
    | return none
  unless (findTypeApplication? ``Move.Action declaration.resultType).isSome do
    return none
  if ← hasExplicitMutableParameter functionName then
    throwErrorAt term
      "automatic source specifications do not yet model calls to effectful Move callee `{functionName}` with a mutable-reference parameter"
  let mut valueNames : Array (TSyntax `term) := #[]
  let mut argumentSpecs : Array (TSyntax `term × TSyntax `ident) := #[]
  for (argument, index) in arguments.zipIdx do
    let valueName := mkIdentFrom argument (Name.mkSimple s!"_moveSpecCallArg{index}")
    valueNames := valueNames.push ⟨valueName.raw⟩
    argumentSpecs := argumentSpecs.push (← translateArgument argument, valueName)
  let packed ← packCallArguments term.raw valueNames
  let mut call ← if functionName == context.functionName then
    let some recursiveSpec := context.recursiveSpec?
      | throwErrorAt term
          "recursive Move call requires generated fixed-point source semantics"
    `($recursiveSpec $packed)
  else do
    if markedContinue then
      throwErrorAt term "`continue` must target the current recursive Move function"
    let sourceSpecName := functionName ++ `sourceSpec
    unless (← getEnv).contains sourceSpecName do
      throwErrorAt term
        "effectful Move callee `{functionName}` has no source specification; declare its `spec` before specifying this caller"
    let sourceSpec := mkIdentFrom head sourceSpecName
    `($sourceSpec $packed)
  for (argumentSpec, valueName) in argumentSpecs.reverse do
    call ← `(Move.Semantics.Spec.bind $argumentSpec fun $valueName => $call)
  return some call

/-- A pure Move helper has executable behavior but no relational summary yet.
It must not fall through to `Spec.pure`, which would incorrectly make its
aborts invisible to an automatically generated caller specification. -/
private def pureMoveCall? (term : TSyntax `term) : CommandElabM (Option Name) := do
  let some (head, _) := application? term | return none
  unless head.raw.isIdent do return none
  let identifier : TSyntax `ident := ⟨head.raw⟩
  let some functionName ← resolveMoveFunction? identifier | return none
  let some declaration := declarations.getState (← getEnv) |>.find? functionName
    | return none
  if (findTypeApplication? ``Move.Action declaration.resultType).isSome then
    return none
  return some functionName

/-- Source semantics for the built-in global-storage primitives.  `exists_`
becomes `containsSpec`; `moveFrom`/`moveTo` become `moveFromSpec`/`moveToSpec`,
and — since they change the resource state — re-certify any global invariant on
the family immediately afterward, exactly as a `&mut` write does. -/
private def globalPrimitiveSpec?
    (translateArgument : TSyntax `term → CommandElabM (TSyntax `term))
    (context : TranslationContext) (term : TSyntax `term) :
    CommandElabM (Option (TSyntax `term)) := do
  let some (head, arguments) := application? term | return none
  unless head.raw.isIdent do return none
  let some name ← (try pure (some (← resolveGlobalConstNoOverload head.raw))
      catch _ => pure none) | return none
  let some resource ← globalPrimitiveResource? term
    | return none
  let descriptor ← resourceFor context.resources resource
  let resourceName ← canonicalResourceName resource
  let invariants := Move.globalInvariants (← getEnv) resourceName
  -- Re-establish the family's global invariants at this state change: an
  -- `update` invariant wraps the op (relating pre/post); a regular invariant
  -- is asserted after it, then the op's own result is returned.
  let recertify (result : TSyntax `term) (core : TSyntax `term) :
      CommandElabM (TSyntax `term) := do
    let mut wrapped := core
    for (isUpdate, body, _) in invariants do
      if isUpdate then
        let bodyId := mkIdentFrom head body
        wrapped ← `(Move.Semantics.Spec.certifyUpdate $bodyId $wrapped)
    let regulars := invariants.filterMap fun (u, b, _) => if u then none else some b
    if regulars.isEmpty then
      return wrapped
    let mut tail ← `(Move.Semantics.Spec.pure $result)
    for body in regulars.reverse do
      let bodyId := mkIdentFrom head body
      tail ← `(Move.Semantics.Spec.bind
        (Move.Semantics.Spec.certifyState $bodyId)
        (fun _moveSpecCertify => $tail))
    `(Move.Semantics.Spec.bind $wrapped (fun $result => $tail))
  if name == ``Move.exists_ then
    let some addr := arguments[1]? | return none
    let addrSpec ← translateArgument addr
    let key := mkIdentFrom addr `_moveSpecKey
    return some (← `(Move.Semantics.Spec.bind $addrSpec (fun $key =>
      Move.Semantics.Resource.containsSpec $descriptor $key)))
  else if name == ``Move.moveFrom then
    let some addr := arguments[1]? | return none
    let addrSpec ← translateArgument addr
    let key := mkIdentFrom addr `_moveSpecKey
    let removed := mkIdentFrom addr `_moveSpecRemoved
    let core ← `(Move.Semantics.Resource.moveFromSpec $descriptor $key)
    let body ← recertify removed core
    return some (← `(Move.Semantics.Spec.bind $addrSpec (fun $key => $body)))
  else if name == ``Move.moveTo then
    let some signer := arguments[0]? | return none
    let some value := arguments[1]? | return none
    let signerSpec ← translateArgument signer
    let valueSpec ← translateArgument value
    let signerName := mkIdentFrom signer `_moveSpecSigner
    let valueName := mkIdentFrom value `_moveSpecPublished
    let unitName := mkIdentFrom term `_moveSpecMoveToResult
    let core ← `(Move.Semantics.Resource.moveToSpec $descriptor
      (Move.Signer.address $signerName) $valueName)
    let body ← recertify unitName core
    return some (← `(Move.Semantics.Spec.bind $signerSpec (fun $signerName =>
      Move.Semantics.Spec.bind $valueSpec (fun $valueName => $body))))
  else
    return none

/-- Translate an expression in value position. Arithmetic is sequenced
relationally so overflow, underflow, and division by zero remain observable. -/
private partial def expressionSpec (context : TranslationContext)
    (term : TSyntax `term) :
    CommandElabM (TSyntax `term) := do
  let binary (operation : Name) (lhs rhs : TSyntax `term) := do
    let lhsSpec ← expressionSpec context lhs
    let rhsSpec ← expressionSpec context rhs
    let op := mkIdentFrom term operation
    `(Move.Semantics.Spec.bind $lhsSpec fun _moveSpecLhs =>
        Move.Semantics.Spec.bind $rhsSpec fun _moveSpecRhs =>
          $op _moveSpecLhs _moveSpecRhs)
  -- `*` is both Lean's multiplication token and Move's prefix dereference
  -- token. Before term elaboration, `lhs * rhs` is therefore represented as
  -- a `choice` between the infix parse and application to `*rhs`. Source
  -- verification works on retained pre-elaboration syntax, so select the
  -- ordinary three-child infix alternative explicitly.
  if term.raw.isOfKind `choice then
    if let some multiplication := term.raw.getArgs.find? fun alternative =>
        alternative.getNumArgs == 3 && alternative[1].isAtom &&
          alternative[1].getAtomVal == "*" then
      let lhs : TSyntax `term := ⟨multiplication[0]⟩
      let rhs : TSyntax `term := ⟨multiplication[2]⟩
      return ← binary ``Move.Semantics.Checked.mulSpec lhs rhs
  if let some (operation, lhs, rhs) ← checkedArithmeticCall? term then
    return ← binary operation lhs rhs
  match term with
  | `(($value:term)) => expressionSpec context value
  | `($lhs:term + $rhs:term) => binary ``Move.Semantics.Checked.addSpec lhs rhs
  | `($lhs:term - $rhs:term) => binary ``Move.Semantics.Checked.subSpec lhs rhs
  | `($lhs:term * $rhs:term) => binary ``Move.Semantics.Checked.mulSpec lhs rhs
  | `($lhs:term / $rhs:term) => binary ``Move.Semantics.Checked.divSpec lhs rhs
  | `($lhs:term % $rhs:term) => binary ``Move.Semantics.Checked.modSpec lhs rhs
  | `($lhs:term <<< $rhs:term) =>
      binary ``Move.Semantics.Checked.shlSpec lhs rhs
  | `($lhs:term >>> $rhs:term) =>
      binary ``Move.Semantics.Checked.shrSpec lhs rhs
  | `(($value:term : $type:term)) =>
      -- An ascribed integer cast, Move's `(x as T)`. The ascription
      -- supplies the target width of the checked cast.
      if value.raw.isIdent then
        match value.raw.getId with
        | .str base "cast" =>
            let operand : TSyntax `term :=
              ⟨mkIdentFrom value.raw base⟩
            let operand ← rewritePure context.mutation? operand
            ``((Move.Semantics.Checked.castSpec $operand :
                Move.Semantics.Spec _ $type))
        | _ =>
            ``(Move.Semantics.Spec.pure
              $(← rewritePure context.mutation? term))
      else
        ``(Move.Semantics.Spec.pure
          $(← rewritePure context.mutation? term))
  | _ =>
      if let some call ← globalPrimitiveSpec? (expressionSpec context) context term then
        pure call
      else if let some call ← effectfulCallSpec? (expressionSpec context) context term then
        pure call
      else if let some functionName ← pureMoveCall? term then
        throwErrorAt term
          "automatic source specifications do not yet model pure Move callee `{functionName}`; inline it or omit `verify`"
      else
        `(Move.Semantics.Spec.pure $(← rewritePure context.mutation? term))

private def finish (context : TranslationContext) (valueSpec : TSyntax `term) :
    CommandElabM (TSyntax `term) := do
  match context.mutation? with
  | none => pure valueSpec
  | some mutation =>
      `(Move.Semantics.Spec.bind $valueSpec fun _moveSpecValue =>
          Move.Semantics.Spec.pure (_moveSpecValue, $mutation))

private partial def packLoopState (ids : List (TSyntax `ident)) :
    CommandElabM (TSyntax `term) := do
  match ids with
  | [] => `(())
  | [id] => `($id)
  | id :: rest =>
      let tail ← packLoopState rest
      `(($id, $tail))

private def identifierExtends (root candidate : Name) : Bool :=
  let rootParts := fieldParts root
  let candidateParts := fieldParts candidate
  !rootParts.isEmpty && candidateParts.take rootParts.length == rootParts

private partial def containsIdentifier (name : Name) (stx : Syntax) : Bool :=
  (stx.isIdent && identifierExtends name stx.getId) ||
    stx.getArgs.any (containsIdentifier name)

/-- Whether syntax projects a field from a local rather than merely using the
local itself. Compound field names need to remain in the borrow's generated
scope so Lean can resolve the projection against the bound value. -/
private partial def containsProjectionOfIdentifier (name : Name)
    (stx : Syntax) : Bool :=
  (stx.isIdent &&
    let rootParts := fieldParts name
    let candidateParts := fieldParts stx.getId
    candidateParts.length > rootParts.length &&
      candidateParts.take rootParts.length == rootParts) ||
    stx.getArgs.any (containsProjectionOfIdentifier name)

private partial def containsReturn (stx : Syntax) : Bool :=
  stx.isOfKind ``Lean.Parser.Term.doReturn || stx.getArgs.any containsReturn

private def freshLoopStateIdents (ref : Syntax)
    (assigned : List (TSyntax `ident)) : List (TSyntax `ident) :=
  assigned.zipIdx.map fun (_, index) =>
    mkIdentFrom ref (Name.mkSimple s!"_moveSpecLoopVar{index}")

private def replaceLoopState (assigned state : List (TSyntax `ident))
    (stx : Syntax) : Syntax :=
  (assigned.zip state).foldl (init := stx) fun stx (source, replacement) =>
    replaceIdentifier source.getId replacement.raw stx

private def findVerificationLoop? (loops : List VerificationLoopFrame)
    (sourceLabel? : Option Name) :
    Option (List VerificationLoopFrame × VerificationLoopFrame) :=
  match sourceLabel? with
  | none => loops.head?.map ([], ·)
  | some sourceLabel =>
      let rec find (inner : List VerificationLoopFrame)
          : List VerificationLoopFrame →
            Option (List VerificationLoopFrame × VerificationLoopFrame)
        | [] => none
        | frame :: rest =>
            if frame.sourceLabel? == some sourceLabel then
              some (inner, frame)
            else
              find (frame :: inner) rest
      find [] loops

private def resolvedLoopState (inner : List VerificationLoopFrame)
    (target : VerificationLoopFrame) : List (TSyntax `ident) :=
  inner.foldl (init := target.state) fun current frame =>
    current.map fun id =>
      match (frame.assigned.zip frame.state).find? (·.1.getId == id.getId) with
      | some (_, replacement) => replacement
      | none => id

private def loopContinueSpec (context : TranslationContext)
    (sourceLabel? : Option Name) (ref : Syntax) :
    CommandElabM (TSyntax `term) := do
  let some (inner, frame) := findVerificationLoop? context.loops sourceLabel?
    | match sourceLabel? with
      | none => throwErrorAt ref "`continue` requires an enclosing `loop` or `while`"
      | some sourceLabel => throwErrorAt ref "unknown loop label `{sourceLabel}`"
  let pack ← packLoopState (resolvedLoopState inner frame)
  `($(frame.recursive) $pack)

private def loopBreakSpec (context : TranslationContext)
    (sourceLabel? : Option Name) (ref : Syntax) :
    CommandElabM (TSyntax `term) := do
  let some (inner, frame) := findVerificationLoop? context.loops sourceLabel?
    | match sourceLabel? with
      | none => throwErrorAt ref "`break` requires an enclosing `loop` or `while`"
      | some sourceLabel => throwErrorAt ref "unknown loop label `{sourceLabel}`"
  let current := resolvedLoopState inner frame
  return ⟨replaceLoopState frame.state current frame.after.raw⟩

private def emptyFinish (context : TranslationContext) : CommandElabM (TSyntax `term) := do
  if !context.loops.isEmpty then
    loopContinueSpec context none Syntax.missing
  else
    let unit ← `(term| ())
    finish context (← expressionSpec context unit)

private partial def unpackLoopState (ids : List (TSyntax `ident)) (packed : TSyntax `term)
    (body : TSyntax `term) : CommandElabM (TSyntax `term) := do
  match ids with
  | [] => `(let _ := $packed; $body)
  | [id] => `(let $id := $packed; $body)
  | id :: rest =>
      let tail := mkIdentFrom id `_moveSpecLoopTail
      let nested ← unpackLoopState rest ⟨tail.raw⟩ body
      `(let ($id, $tail) := $packed; $nested)

private def boundIdentifier? (element : Lean.DoElem) : Option (Name × Bool) :=
  match element with
  | `(doElem| let mut $name:ident ← $_value:term) => some (name.getId, true)
  | `(doElem| let mut $name:ident := $_value:term) => some (name.getId, true)
  | `(doElem| let $name:ident ← $_value:term) => some (name.getId, false)
  | `(doElem| let $name:ident := $_value:term) => some (name.getId, false)
  | `(doElem| let $name:ident : $_type:term := $_value:term) =>
      some (name.getId, false)
  | _ => none

private partial def closeBorrowScope (elements : Array Lean.DoElem)
    (size : Nat) : Nat :=
  let extended := (elements.extract 0 size).foldl (init := size) fun result element =>
    match boundIdentifier? element with
    | none => result
    | some (name, includePlainUses) =>
        elements.zipIdx.foldl (init := result) fun result (candidate, index) =>
          if (if includePlainUses then containsIdentifier
              else containsProjectionOfIdentifier) name candidate.raw then
            max result (index + 1)
          else
            result
  if extended = size then size else closeBorrowScope elements extended

/-- Split the statements following a mutable borrow at the last use of its
reference local. The prefix is the loan body; the suffix executes after the
loan has been reconciled. -/
private def mutableBorrowScope (name : Name) (elements : Array Lean.DoElem) :
    Array Lean.DoElem × Array Lean.DoElem :=
  let lastUse := elements.zipIdx.foldl (init := none) fun result (element, index) =>
    if containsIdentifier name element.raw then some index else result
  let size := closeBorrowScope elements (lastUse.map (· + 1) |>.getD 0)
  (elements.extract 0 size, elements.extract size elements.size)

mutual
private partial def translateDo (context : TranslationContext)
    (elements : Array Lean.DoElem) :
    CommandElabM (TSyntax `term) := do
  let translateRest (rest : Array Lean.DoElem) :=
    if rest.isEmpty then emptyFinish context else translateDo context rest
  if elements.isEmpty then return ← emptyFinish context
  let first : Lean.DoElem := elements[0]!
  let rest := elements.extract 1 elements.size
  if first.raw.isOfKind ``Lean.Parser.Term.doReassign then
    let assignment : TSyntax ``Lean.Parser.Term.doReassign := ⟨first.raw⟩
    match assignment with
    | `(doReassign| $name:ident $[: $_]? :=%$_ $rhs:term) =>
        if let some mutation := context.mutation? then
          if mutation.getId == name.getId then
            let rhsSpec ← expressionSpec context rhs
            let nested ← translateRest rest
            return ← `(Move.Semantics.Spec.bind $rhsSpec fun _moveSpecValue =>
                let $name := Move.Semantics.Mutation.write $name _moveSpecValue
                $nested)
        let rhsSpec ← expressionSpec context rhs
        let nested ← translateRest rest
        return ← `(Move.Semantics.Spec.bind $rhsSpec fun $name => $nested)
    | _ => throwErrorAt first "unsupported assignment in automatic source specification"
  if first.raw.isOfKind ``Move.moveLoopDo ||
      first.raw.isOfKind ``Move.moveLoopLabeledDo then
    let sourceLabel? :=
      if first.raw.isOfKind ``Move.moveLoopLabeledDo then
        some first.raw[1]!.getId
      else
        none
    if let some sourceLabel := sourceLabel? then
      if context.loops.any (·.sourceLabel? == some sourceLabel) then
        throwErrorAt first "duplicate active loop label `{sourceLabel}`"
    let bodyIndex := if sourceLabel?.isSome then 2 else 1
    let body : TSyntax ``Lean.Parser.Term.doSeq := ⟨first.raw[bodyIndex]!⟩
    let body ← Lean.Elab.Command.liftCoreM <| Move.freshenLoopLocals body
    if containsReturn body.raw then
      throwErrorAt first
        "`return` inside `loop` / `while` is not yet supported for `verify`"
    let assigned := Move.loopAssignedIdents body
    let state := freshLoopStateIdents first.raw assigned
    let pack ← packLoopState assigned
    let recName := mkIdentFrom first `_moveSpecLoop
    let recTerm : TSyntax `term := ⟨recName.raw⟩
    let afterElements := rest.map fun element =>
      (⟨replaceLoopState assigned state element.raw⟩ : Lean.DoElem)
    let after ← translateRest afterElements
    let bodyElements := (Lean.Parser.Term.getDoElems body).map fun element =>
      (⟨replaceLoopState assigned state element.raw⟩ : Lean.DoElem)
    let frame : VerificationLoopFrame := {
      sourceLabel?, recursive := recTerm, after, assigned, state }
    let bodySpec ← translateDo
      { context with
        loops := frame :: context.loops }
      bodyElements
    let stateName := mkIdentFrom first `_moveSpecLoopState
    let stateTerm : TSyntax `term := ⟨stateName.raw⟩
    let unpacked ← unpackLoopState state stateTerm bodySpec
    return ← `(Move.Semantics.Spec.fix
        (fun $recName $stateName => $unpacked) $pack)
  match first with
  | `(doElem| let $name:ident ← &mut $vector:ident[$index:term]) =>
      unless context.mutation?.isNone do
        throwErrorAt first "nested mutable borrows are not yet supported by source specification generation"
      if ← hasResource context.resources vector then
        throwErrorAt first "direct mutable global borrows are not yet supported by source specification generation"
      let (loanBody, continuation) := mutableBorrowScope name.getId rest
      let nested ← translateDo { context with mutation? := some name } loanBody
      let borrow ← `(Move.Semantics.Vector.withBorrowElemMutSpec $vector $index
        (fun $name => $nested))
      let output := mkIdentFrom first `_moveSpecVectorOutput
      let after ← if continuation.isEmpty then
        `(Move.Semantics.Spec.pure $output.1)
      else
        let continuationSpec ← translateDo context continuation
        `(let $vector := $output.2; $continuationSpec)
      `(Move.Semantics.Spec.bind $borrow (fun $output => $after))
  | `(doElem| let $name:ident ← & $vector:ident[$index:term]) =>
      if ← hasResource context.resources vector then
        throwErrorAt first "direct immutable global borrows are not yet supported by source specification generation"
      let nested ← translateRest rest
      `(Move.Semantics.Spec.bind
          (Move.Semantics.Vector.borrowElemSpec $vector $index)
          (fun $name => $nested))
  | `(doElem| let $name:ident ← &mut $place:term) =>
      let (loanBody, continuation) := mutableBorrowScope name.getId rest
      let nested ← translateDo { context with mutation? := some name } loanBody
      if let some parent := context.mutation? then
        let some (owner, fields) ← localPlace? context.resources place
          | throwErrorAt place
              "a nested mutable borrow must select a field of the active mutable parameter"
        unless owner.getId == parent.getId && !fields.isEmpty do
          throwErrorAt place
            "a nested mutable borrow must select a field of the active mutable parameter"
        let parentValue ← `(Move.Semantics.Mutation.read $parent)
        let focused ← projectPath parentValue fields
        let output := mkIdentFrom place `_moveSpecFieldOutput
        let outputTerm : TSyntax `term := ⟨output.raw⟩
        let finalField ← `($outputTerm.2)
        let borrow ← `(Move.Semantics.withMutation $focused (fun $name => $nested))
        let after ← if continuation.isEmpty then
          finish context (← `(Move.Semantics.Spec.pure $outputTerm.1))
        else
          translateDo context continuation
        -- Re-creating a certified owner is a creation site: its data
        -- invariant is owed here, when the loan dies, and nowhere else.
        match ← rebuildOwner parentValue finalField fields.toList
            context.certifiedOwner? with
        | some creation =>
            let rebuilt := mkIdentFrom place `_moveSpecRebuilt
            `(Move.Semantics.Spec.bind $borrow (fun $output =>
                Move.Semantics.Spec.bind $creation (fun $rebuilt =>
                  let $parent := Move.Semantics.Mutation.write $parent $rebuilt
                  $after)))
        | none =>
            let updated ← updatePath parentValue finalField fields.toList
            `(Move.Semantics.Spec.bind $borrow (fun $output =>
                let $parent := Move.Semantics.Mutation.write $parent $updated
                $after))
      else if place.raw.isIdent && !(← hasResource context.resources ⟨place.raw⟩) then
        let localIdent : TSyntax `ident := ⟨place.raw⟩
        -- Keep values produced in the loan body in lexical scope, but refresh
        -- the owner from the prophecy before translating code after the
        -- reference's last use.  Such code observes the reconciled value under
        -- Move's non-lexical borrow semantics.
        let scopedRest ← if continuation.isEmpty then
          pure rest
        else do
          let refresh ← `(doElem|
            let $localIdent := Move.Semantics.Mutation.read $name)
          pure (loanBody ++ #[refresh] ++ continuation)
        let nested ← translateDo { context with mutation? := some name } scopedRest
        let borrow ← `(Move.Semantics.withMutation $localIdent (fun $name => $nested))
        let output := mkIdentFrom place `_moveSpecLocalOutput
        `(Move.Semantics.Spec.bind $borrow
            (fun $output => Move.Semantics.Spec.pure $output.1))
      else if let some (vector, index, fields) ←
          localVectorPlace? context.resources place then
        unless fields.isEmpty do
          throwErrorAt place
            "mutable vector-element field borrows are not yet supported by source specification generation"
        let borrow ← `(Move.Semantics.Vector.withBorrowElemMutSpec $vector $index
          (fun $name => $nested))
        let output := mkIdentFrom place `_moveSpecVectorOutput
        let after ← if continuation.isEmpty then
          `(Move.Semantics.Spec.pure $output.1)
        else
          let continuationSpec ← translateDo context continuation
          `(let $vector := $output.2; $continuationSpec)
        `(Move.Semantics.Spec.bind $borrow (fun $output => $after))
      else
        let (resourceType, key, fields) ← globalPlace place
        let descriptor ← resourceFor context.resources resourceType
        let owner := mkIdentFrom place `_moveSpecOwner
        let replacement := mkIdentFrom place `_moveSpecReplacement
        let ownerTerm : TSyntax `term := ⟨owner.raw⟩
        let replacementTerm : TSyntax `term := ⟨replacement.raw⟩
        let focused ← projectPath ownerTerm fields
        let resourceName ← Move.Verify.Source.canonicalResourceName resourceType
        let certified? := (Move.dataInvariant? (← getEnv) resourceName).map
          (resourceName, ·)
        let borrow ← match ← rebuildOwner ownerTerm replacementTerm fields.toList
            certified? with
          | some creation =>
              -- A certified resource is re-created when the loan dies: its
              -- data invariant is owed there, and the stored value stays
              -- certified.
              let output := mkIdentFrom place `_moveSpecFocusOutput
              let rebuilt := mkIdentFrom place `_moveSpecRebuilt
              `(Move.Semantics.Resource.withBorrowMutSpec $descriptor $key
                  (fun $owner =>
                    Move.Semantics.Spec.bind
                      (Move.Semantics.withMutation $focused (fun $name => $nested))
                      (fun $output =>
                        let $replacement := $output.2
                        Move.Semantics.Spec.bind $creation
                          (fun $rebuilt =>
                            Move.Semantics.Spec.pure ($output.1, $rebuilt)))))
          | none =>
              let updated ← updatePath ownerTerm replacementTerm fields.toList
              `(Move.Semantics.Resource.withBorrowMutFocusSpec $descriptor $key
                  (fun $owner => $focused)
                  (fun $owner $replacement => $updated)
                  (fun $name => $nested))
        -- Global invariants re-certify the store at this write: an `update`
        -- invariant wraps the write (relating pre/post state); a regular
        -- invariant is asserted immediately after it.
        let invariants := Move.globalInvariants (← getEnv) resourceName
        let mut borrow := borrow
        for (isUpdate, body, _) in invariants do
          if isUpdate then
            let bodyId := mkIdentFrom place body
            borrow ← `(Move.Semantics.Spec.certifyUpdate $bodyId $borrow)
        let regulars := invariants.filterMap fun (u, b, _) => if u then none else some b
        let assertGlobal (cont : TSyntax `term) : CommandElabM (TSyntax `term) := do
          let mut tail := cont
          for body in regulars.reverse do
            let bodyId := mkIdentFrom place body
            tail ← `(Move.Semantics.Spec.bind
              (Move.Semantics.Spec.certifyState $bodyId)
              (fun _moveSpecCertify => $tail))
          pure tail
        if continuation.isEmpty then
          if regulars.isEmpty then pure borrow
          else
            let tail ← assertGlobal
              (← `(Move.Semantics.Spec.pure _moveSpecBorrowResult))
            `(Move.Semantics.Spec.bind $borrow
                (fun _moveSpecBorrowResult => $tail))
        else
          let after ← translateDo context continuation
          let tail ← assertGlobal after
          `(Move.Semantics.Spec.bind $borrow (fun _moveSpecBorrowResult => $tail))
  | `(doElem| let $name:ident ← & $place:term) =>
      let nested ← translateRest rest
      if let some (vector, index, fields) ←
          localVectorPlace? context.resources place then
        let element := mkIdentFrom place `_moveSpecVectorElement
        let elementTerm : TSyntax `term := ⟨element.raw⟩
        let focused ← projectPath elementTerm fields
        `(Move.Semantics.Spec.bind
            (Move.Semantics.Vector.borrowElemSpec $vector $index)
            (fun $element => let $name := $focused; $nested))
      else if let some (owner, fields) ←
          localPlace? context.resources place then
        let ownerTerm ← mutationValue context owner
        let focused ← projectPath ownerTerm fields
        `(let $name := $focused; $nested)
      else
        let (resourceType, key, fields) ← globalPlace place
        let descriptor ← resourceFor context.resources resourceType
        let owner := mkIdentFrom place `_moveSpecOwner
        let ownerTerm : TSyntax `term := ⟨owner.raw⟩
        let focused ← projectPath ownerTerm fields
        `(Move.Semantics.Spec.bind
            (Move.Semantics.Resource.borrowSpec $descriptor $key)
            (fun $owner => let $name := $focused; $nested))
  | `(doElem| let $name:ident ← * $reference:term) =>
      let nested ← translateRest rest
      `(let $name := $(← rewritePure context.mutation? (← `(* $reference))); $nested)
  | `(doElem| let $name:ident ← $value:term) =>
      if let some call ← vectorMutationCall? value then
        let some mutation := context.mutation?
          | throwErrorAt value
              "`vector::insert` and `vector::remove` require a live mutable vector borrow"
        let output := mkIdentFrom value `_moveSpecVectorMutationOutput
        let nested ← translateRest rest
        match call with
        | .insert reference index inserted =>
            unless reference.raw.isIdent && reference.raw.getId == mutation.getId do
              throwErrorAt reference "vector insert must use the currently borrowed vector"
            let index ← rewritePure context.mutation? index
            let inserted ← rewritePure context.mutation? inserted
            `(Move.Semantics.Spec.bind
                (Move.Semantics.Vector.insertSpec $mutation $index $inserted)
                (fun $output =>
                  let $name := $output.1
                  let $mutation := $output.2
                  $nested))
        | .remove reference index =>
            unless reference.raw.isIdent && reference.raw.getId == mutation.getId do
              throwErrorAt reference "vector remove must use the currently borrowed vector"
            let index ← rewritePure context.mutation? index
            `(Move.Semantics.Spec.bind
                (Move.Semantics.Vector.removeSpec $mutation $index)
                (fun $output =>
                  let $name := $output.1
                  let $mutation := $output.2
                  $nested))
      else
        if receiverStyleVectorMutation? value then
          throwErrorAt value
            "automatic source specifications require fully qualified `Move.Vector.insert` or `Move.Vector.remove`"
        let valueSpec ← expressionSpec context value
        let nested ← translateRest rest
        `(Move.Semantics.Spec.bind $valueSpec (fun $name => $nested))
  | `(doElem| let $name:ident := $value:term) =>
      let valueSpec ← expressionSpec context value
      let nested ← translateRest rest
      `(Move.Semantics.Spec.bind $valueSpec (fun $name => $nested))
  | `(doElem| let $name:ident : $_type:term := $value:term) =>
      let valueSpec ← expressionSpec context value
      let nested ← translateRest rest
      `(Move.Semantics.Spec.bind $valueSpec (fun $name => $nested))
  | `(doElem| let mut $name:ident := $value:term) =>
      let valueSpec ← expressionSpec context value
      let nested ← translateRest rest
      `(Move.Semantics.Spec.bind $valueSpec (fun $name => $nested))
  | `(doElem| let mut $name:ident : $_type:term := $value:term) =>
      let valueSpec ← expressionSpec context value
      let nested ← translateRest rest
      `(Move.Semantics.Spec.bind $valueSpec (fun $name => $nested))
  | `(doElem| let mut $name:ident ← * $reference:term) =>
      let nested ← translateRest rest
      `(let $name := $(← rewritePure context.mutation? (← `(* $reference))); $nested)
  | `(doElem| let mut $name:ident ← $value:term) =>
      let valueSpec ← expressionSpec context value
      let nested ← translateRest rest
      `(Move.Semantics.Spec.bind $valueSpec (fun $name => $nested))
  | `(doElem| return $value:term) =>
      if !context.loops.isEmpty then
        throwErrorAt first
          "`return` inside `loop` / `while` is not yet supported for `verify`"
      translateTerm context value
  | `(doElem| break@$sourceLabel:ident) =>
      loopBreakSpec context (some sourceLabel.getId) first.raw
  | `(doElem| continue@$sourceLabel:ident) =>
      loopContinueSpec context (some sourceLabel.getId) first.raw
  | `(doElem| break) =>
      loopBreakSpec context none first.raw
  | `(doElem| continue) =>
      loopContinueSpec context none first.raw
  | `(doElem| while $condition:doIfCond do $body:doSeq) =>
      if containsReturn body.raw then
        throwErrorAt first
          "`return` inside `loop` / `while` is not yet supported for `verify`"
      let condition ← conditionTerm condition
      let body ← Lean.Elab.Command.liftCoreM <| Move.freshenLoopLocals body
      let assigned := Move.loopAssignedIdents body
      let state := freshLoopStateIdents first.raw assigned
      let condition : TSyntax `term := ⟨replaceLoopState assigned state condition.raw⟩
      let condition ← rewritePure context.mutation? condition
      let pack ← packLoopState assigned
      let recName := mkIdentFrom first `_moveSpecLoop
      let recTerm : TSyntax `term := ⟨recName.raw⟩
      let afterElements := rest.map fun element =>
        (⟨replaceLoopState assigned state element.raw⟩ : Lean.DoElem)
      let after ← translateRest afterElements
      let bodyElements := (Lean.Parser.Term.getDoElems body).map fun element =>
        (⟨replaceLoopState assigned state element.raw⟩ : Lean.DoElem)
      let frame : VerificationLoopFrame := {
        sourceLabel? := none
        recursive := recTerm
        after
        assigned
        state }
      let bodySpec ← translateDo
        { context with loops := frame :: context.loops }
        bodyElements
      let step ← `(if $condition then $bodySpec else $after)
      let stateName := mkIdentFrom first `_moveSpecLoopState
      let stateTerm : TSyntax `term := ⟨stateName.raw⟩
      let unpacked ← unpackLoopState state stateTerm step
      `(Move.Semantics.Spec.fix
          (fun $recName $stateName => $unpacked) $pack)
  | `(doElem| if $condition:doIfCond then $thenBranch:doSeq) =>
      let condition ← conditionTerm condition
      let condition ← rewritePure context.mutation? condition
      let thenSpec ← translateDo context
        (Lean.Parser.Term.getDoElems thenBranch ++ rest)
      let elseSpec ← translateRest rest
      `(if $condition then $thenSpec else $elseSpec)
  | `(doElem| if $condition:doIfCond then $thenBranch:doSeq else $elseBranch:doSeq) =>
      unless rest.isEmpty do
        throwErrorAt first "an `if` with `else` must be in tail position for source specification generation"
      let condition ← conditionTerm condition
      let condition ← rewritePure context.mutation? condition
      let thenSpec ← translateDo context (Lean.Parser.Term.getDoElems thenBranch)
      let elseSpec ← translateDo context (Lean.Parser.Term.getDoElems elseBranch)
      `(if $condition then $thenSpec else $elseSpec)
  | `(doElem| $value:term) =>
      if let some (.insert reference index inserted) ← vectorMutationCall? value then
        let some mutation := context.mutation?
          | throwErrorAt value "`vector::insert` requires a live mutable vector borrow"
        unless reference.raw.isIdent && reference.raw.getId == mutation.getId do
          throwErrorAt reference "vector insert must use the currently borrowed vector"
        let output := mkIdentFrom value `_moveSpecVectorMutationOutput
        let index ← rewritePure context.mutation? index
        let inserted ← rewritePure context.mutation? inserted
        let nested ← translateRest rest
        `(Move.Semantics.Spec.bind
            (Move.Semantics.Vector.insertSpec $mutation $index $inserted)
            (fun $output =>
              let $mutation := $output.2
              $nested))
      else if receiverStyleVectorMutation? value then
        throwErrorAt value
          "automatic source specifications require fully qualified `Move.Vector.insert` or `Move.Vector.remove`"
      else if value.raw.isOfKind ``Move.abortTerm then
        -- Abort is terminal, so this branch never executes its syntactic rest.
        translateTerm context value
      else
        unless rest.isEmpty do
          throwErrorAt first
            "unsupported effectful statement in automatic source specification: {first.raw}"
        translateTerm context value
  | _ => throwErrorAt first
      "unsupported `do` statement in automatic source specification: {first.raw}"

private partial def translateTerm (context : TranslationContext)
    (term : TSyntax `term) : CommandElabM (TSyntax `term) := do
  match term with
  | `(do $sequence:doSeq) =>
      translateDo context (Lean.Parser.Term.getDoElems sequence)
  | `(& $place:term) =>
      if let some (vector, index, fields) ←
          localVectorPlace? context.resources place then
        let element := mkIdentFrom place `_moveSpecVectorElement
        let elementTerm : TSyntax `term := ⟨element.raw⟩
        let focused ← projectPath elementTerm fields
        let result ← finish context (← `(Move.Semantics.Spec.pure $focused))
        `(Move.Semantics.Spec.bind
            (Move.Semantics.Vector.borrowElemSpec $vector $index)
            (fun $element => $result))
      else if let some (owner, fields) ←
          localPlace? context.resources place then
        let ownerTerm ← mutationValue context owner
        let focused ← projectPath ownerTerm fields
        finish context (← `(Move.Semantics.Spec.pure $focused))
      else
        let (resourceType, key, fields) ← globalPlace place
        let descriptor ← resourceFor context.resources resourceType
        let owner := mkIdentFrom place `_moveSpecOwner
        let ownerTerm : TSyntax `term := ⟨owner.raw⟩
        let focused ← projectPath ownerTerm fields
        let result ← finish context (← `(Move.Semantics.Spec.pure $focused))
        `(Move.Semantics.Spec.bind
            (Move.Semantics.Resource.borrowSpec $descriptor $key)
            (fun $owner => $result))
  | `(abort $code:term) =>
      let codeSpec ← expressionSpec context code
      `(Move.Semantics.Spec.bind $codeSpec fun _moveSpecAbortCode =>
          Move.Semantics.Spec.abort
            (Move.Spec.abortCodeOf _moveSpecAbortCode))
  | `(pure $value:term) => finish context (← expressionSpec context value)
  | _ =>
      let valueSpec ← expressionSpec context term
      match context.mutation? with
      | none => pure valueSpec
      | some mutation =>
          `(Move.Semantics.Spec.bind $valueSpec fun _moveSpecResult =>
              Move.Semantics.Spec.pure (_moveSpecResult, $mutation))

private partial def conditionTerm (condition : TSyntax ``Lean.Parser.Term.doIfCond) :
    CommandElabM (TSyntax `term) := do
  match condition with
  | `(doIfCond| $term:term) => pure term
  | `(doIfCond| $_:ident : $term:term) => pure term
  | _ => throwErrorAt condition
      "dependent and pattern `if` conditions are not yet supported by source specification generation"
end

def translate (function world : Syntax) (resources : Array ResourceBinding)
    (recursiveSpec? : Option (TSyntax `term) := none)
    (mutableParameter? : Option (TSyntax `ident × TSyntax `term) := none) :
    CommandElabM (TSyntax `term × TSyntax `term) := do
  let declaration ← declarationFor function
  let resultType ← actionResultType declaration
  let body ← sourceBody declaration
  let functionName := (← getCurrNamespace) ++ function.getId
  let certifiedOwner? ← match mutableParameter? with
    | none => pure none
    | some (_, referent) => do
        match ← referentTypeName? referent with
        | none => pure none
        | some typeName =>
            pure ((Move.dataInvariant? (← getEnv) typeName).map (typeName, ·))
  let spec ← translateTerm {
    world := ⟨world⟩
    resources
    functionName
    recursiveSpec?
    mutation? := mutableParameter?.map (·.1)
    certifiedOwner?
  } body
  match mutableParameter? with
  | none => pure (spec, resultType)
  | some (parameter, referent) =>
      let wrapped ← `(Move.Semantics.withMutation $parameter
        (fun $parameter => $spec))
      pure (wrapped, ← `($resultType × $referent))

private partial def containsFunctionCall
    (fullName shortName : Name) (stx : Syntax) : Bool :=
  let direct := stx.isOfKind ``Lean.Parser.Term.app && stx.getNumArgs > 0 &&
    stx[0].isIdent &&
      (stx[0].getId == fullName || stx[0].getId == shortName)
  let marked := stx.isOfKind ``Move.continueCallTerm && stx.getNumArgs > 1 &&
    stx[1].isIdent &&
      (stx[1].getId == fullName || stx[1].getId == shortName)
  direct || marked || stx.getArgs.any (containsFunctionCall fullName shortName)

/-- Whether the retained body directly refers to its own Move declaration.
Move has no first-class functions, so such an occurrence is a recursive call. -/
def isRecursive (function : Syntax) : CommandElabM Bool := do
  let declaration ← declarationFor function
  let body ← sourceBody declaration
  let fullName := (← getCurrNamespace) ++ function.getId
  pure (containsFunctionCall fullName function.getId body.raw)

/-- Resource families an effectful source function must bring into scope: those
it borrows or publishes/removes, closed under global invariants.  A write to a
family re-checks every invariant naming it, and that obligation refers to every
family the invariant mentions — so those families' stores must be in scope even
when the function never touches them directly. -/
def inferredResources (function : Syntax) : CommandElabM (Array (TSyntax `ident)) := do
  let declaration ← declarationFor function
  let body ← sourceBody declaration
  let touched ← collectResources body.raw
  let env ← getEnv
  -- Close the touched families under global-invariant mentions, at the level
  -- of canonical names (an added ident re-canonicalizes to the same name, so
  -- the fixed point terminates).
  let touchedNames ← touched.mapM canonicalResourceName
  -- Bounded fixed point over invariant mentions, deduplicating by string to
  -- avoid `Name` representation pitfalls.  The mention lists are already
  -- complete per invariant, so a handful of passes reach closure.
  let has (arr : Array Name) (n : Name) : Bool := arr.any (·.toString == n.toString)
  let mut all := touchedNames
  for _ in [0:8] do
    let previous := all
    for family in previous do
      for (_, _, mentioned) in Move.globalInvariants env family do
        for m in mentioned do
          unless has all m do all := all.push m
    if all.size == previous.size then break
  let mut idents := touched
  for m in all do
    unless has touchedNames m do
      idents := idents.push (mkIdentFrom function m)
  return idents

/-- Translate against the abstract compositional resource-store interface. -/
def translateWithStores (function : Syntax) (world : TSyntax `ident)
    (resourceTypes : Array (TSyntax `ident))
    (recursiveSpec? : Option (TSyntax `term) := none)
    (mutableParameter? : Option (TSyntax `ident × TSyntax `term) := none) :
    CommandElabM (TSyntax `term × TSyntax `term) := do
  let mut resources := #[]
  for resourceType in resourceTypes do
    let descriptor ← `(Move.Semantics.ResourceStore.descriptor
      (State := $world) (Value := $resourceType))
    resources := resources.push {
      typeName := ← canonicalResourceName resourceType
      descriptor := descriptor
    }
  translate function world.raw resources recursiveSpec? mutableParameter?

private def knownResource (resources : Array (TSyntax `ident))
    (candidate : TSyntax `ident) : CommandElabM Bool := do
  let candidate ← canonicalResourceName candidate
  for resource in resources do
    if candidate == (← canonicalResourceName resource) then
      return true
  return false

private def rewriteGlobalPlace (resources : Array (TSyntax `ident))
    (state place : TSyntax `term) : CommandElabM (Option (TSyntax `term)) := do
  let (root, fields) := splitFieldPath place
  let rootInfo : Option (TSyntax `ident × TSyntax `term) := match root with
    | `($resourceType:ident[$key:term]) => some (resourceType, key)
    | _ => none
  let some (resourceType, key) := rootInfo | return none
  unless ← knownResource resources resourceType do return none
  let owner ← `(Move.Semantics.ResourceStore.get
    (Value := $resourceType) $state $key)
  return some (← projectPath owner fields)

/-- Rewrite global-place observations in a contract clause. Bare places refer
to `current`; `old(place)` refers to `previous`. -/
partial def rewriteClause (resources : Array (TSyntax `ident))
    (current previous : TSyntax `term) (clause : TSyntax `term) :
    CommandElabM (TSyntax `term) := do
  match clause with
  | `(old($place:term)) =>
      let some rewritten ← rewriteGlobalPlace resources previous place
        | throwErrorAt place "`old` expects a global resource place"
      pure rewritten
  | `(exists<$resourceType:ident>($address:term)) =>
      unless ← knownResource resources resourceType do
        throwErrorAt resourceType
          "resource `{resourceType.getId}` is not used by the specified function"
      `(Move.Semantics.ResourceStore.contains
        (Value := $resourceType) $current $address)
  | _ =>
      if let some rewritten ← rewriteGlobalPlace resources current clause then
        return rewritten
      let args ← clause.raw.getArgs.mapM fun child => do
        let rewritten ← rewriteClause resources current previous ⟨child⟩
        pure rewritten.raw
      pure ⟨clause.raw.setArgs args⟩

end Move.Verify.Source

namespace Move.Spec

open Lean Elab Command Parser Command Macro
open scoped Move

private def nameSuffix? : Name → Option String
  | .str _ suffix => some suffix
  | _ => none

declare_syntax_cat moveSpecBinder
syntax "(" ident " : " term ")" : moveSpecBinder
syntax "{" ident " : " term "}" : moveSpecBinder
syntax "{" ident "}" : moveSpecBinder
syntax "[" term "]" : moveSpecBinder

declare_syntax_cat moveSpecResource
syntax ident " => " term : moveSpecResource

private def associatedName (function : TSyntax `ident) (suffix : Name) : TSyntax `ident :=
  mkIdentFrom function (function.getId ++ suffix)

private def applyArguments (function : TSyntax `ident)
    (arguments : Array (TSyntax `ident)) : MacroM (TSyntax `term) := do
  arguments.foldlM (init := ⟨function.raw⟩) fun application argument =>
    `($application $argument)

private def quantifyArguments (arguments : Array (TSyntax `ident))
    (types : Array (TSyntax `term)) (body : TSyntax `term) : MacroM (TSyntax `term) := do
  arguments.zip types |>.foldrM (init := body) fun (argument, type) body =>
    `(∀ ($argument : $type), $body)

private structure SpecParameters where
  arguments : Array (TSyntax `ident) := #[]
  types : Array (TSyntax `term) := #[]
  context : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) := #[]
  mutableParameter? : Option (TSyntax `ident × TSyntax `term) := none

private partial def mutableReferent? (type : TSyntax `term) : Option (TSyntax `term) :=
  match type with
  | `(($inner:term)) => mutableReferent? inner
  | `(&mut $referent:term) => some referent
  | _ => none

/-- Separate runtime arguments from generic proof context.  Every Move type
parameter contributes an internal `Inhabited` instance, mirroring the `fun`
command without exposing that compiler requirement in source contracts. -/
private def unpackSpecParameters
    (binders : Array (TSyntax `moveSpecBinder)) : MacroM SpecParameters := do
  let mut result : SpecParameters := {}
  for binder in binders do
    match binder with
    | `(moveSpecBinder|($argument:ident : $type:term)) =>
        let mut logicalType := type
        let mut mutableParameter? := result.mutableParameter?
        if let some referent := mutableReferent? type then
          if mutableParameter?.isSome then
            Macro.throwErrorAt binder
              "source contracts currently support at most one mutable-reference parameter"
          logicalType := referent
          mutableParameter? := some (argument, referent)
        result := { result with
          arguments := result.arguments.push argument
          types := result.types.push logicalType
          mutableParameter? }
    | `(moveSpecBinder|{$typeName:ident : $type:term}) =>
        let typeBinder ← `(bracketedBinder| {$typeName : $type})
        let inhabitedBinder ← `(bracketedBinder| [Inhabited $typeName])
        result := { result with
          context := result.context.push typeBinder |>.push inhabitedBinder }
    | `(moveSpecBinder|{$typeName:ident}) =>
        let typeBinder ← `(bracketedBinder| {$typeName : Type})
        let inhabitedBinder ← `(bracketedBinder| [Inhabited $typeName])
        result := { result with
          context := result.context.push typeBinder |>.push inhabitedBinder }
    | `(moveSpecBinder|[$instanceType:term]) =>
        let instanceBinder ← `(bracketedBinder| [$instanceType])
        result := { result with context := result.context.push instanceBinder }
    | _ => Macro.throwErrorAt binder "invalid specification binder"
  pure result

private def quantifyContext
    (binders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder))
    (body : TSyntax `term) : MacroM (TSyntax `term) :=
  binders.foldrM (init := body) fun binder body => `(∀ $binder, $body)

private partial def findResult? (stx : Syntax) : Option Syntax :=
  if stx.isIdent && stx.getId == `result then
    some stx
  else
    stx.getArgs.findSome? findResult?

private partial def bindResult (stx binder : Syntax) : Syntax :=
  if stx.isIdent && stx.getId == `result then
    binder
  else
    stx.setArgs (stx.getArgs.map (bindResult · binder))

private partial def bindImplicit (name : Name) (stx binder : Syntax) : Syntax :=
  if stx.isIdent && stx.getId == name then
    binder
  else
    stx.setArgs (stx.getArgs.map (bindImplicit name · binder))

private def specFieldParts : Name → List String
  | .anonymous => []
  | .str baseName part => specFieldParts baseName ++ [part]
  | .num _ _ => []

private def specIdentifierExtends (root candidate : Name) : Bool :=
  let rootParts := specFieldParts root
  let candidateParts := specFieldParts candidate
  !rootParts.isEmpty && candidateParts.take rootParts.length == rootParts

/-- In a mutable-reference postcondition, the parameter name denotes the
post-call referent while `old(parameter)` denotes its pre-call value. -/
private partial def rewriteMutablePost (parameter : TSyntax `ident)
    (finalValue : TSyntax `term) (stx : Syntax) : MacroM Syntax := do
  let term : TSyntax `term := ⟨stx⟩
  match term with
  | `(old($value:term)) =>
      if value.raw.isIdent && value.raw.getId == parameter.getId then
        return parameter.raw
  | _ => pure ()
  if stx.isIdent && specIdentifierExtends parameter.getId stx.getId then
    let parameterParts := specFieldParts parameter.getId
    let suffix := (specFieldParts stx.getId).drop parameterParts.length
    let mut rewritten := finalValue
    for field in suffix do
      let field := mkIdentFrom stx (Name.mkSimple field)
      rewritten ← `($rewritten.$field)
    return rewritten.raw
  let args ← stx.getArgs.mapM (rewriteMutablePost parameter finalValue)
  pure (stx.setArgs args)

private def argumentType (types : Array (TSyntax `term)) : MacroM (TSyntax `term) := do
  match types.size with
  | 0 => `(Unit)
  | 1 => pure types[0]!
  | _ =>
      let reversed := types.toList.reverse
      let result := reversed.head!
      reversed.tail.foldlM (init := result) fun result type => `($type × $result)

private def argumentProjection (base : TSyntax `term) (index count : Nat) :
    MacroM (TSyntax `term) := do
  let mut projection := base
  for _ in [:index] do
    projection ← `($projection.2)
  if index + 1 < count then `($projection.1) else pure projection

private def unpackArguments (arguments : Array (TSyntax `ident))
    (body : TSyntax `term) : MacroM (TSyntax `term) := do
  match arguments.size with
  | 0 => `(fun _moveSpecArgs => $body)
  | 1 => `(fun $(arguments[0]!) => $body)
  | _ =>
      let args := mkIdentFrom arguments[0]! `_moveSpecArgs
      let argsTerm : TSyntax `term := ⟨args.raw⟩
      let mut result := body
      for index in (List.range arguments.size).reverse do
        let argument := arguments[index]!
        let projection ← argumentProjection argsTerm index arguments.size
        result ← `(let $argument := $projection; $result)
      `(fun $args => $result)

private def clauseLambda (arguments : Array (TSyntax `ident))
    (extra : Array (Name × TSyntax `ident)) (clause : TSyntax `term) :
    MacroM (TSyntax `term) := do
  let rewritten := extra.foldl (init := clause.raw) fun result (name, binder) =>
    bindImplicit name result binder.raw
  let mut body : TSyntax `term := ⟨rewritten⟩
  for (_, binder) in extra.reverse do
    body ← `(fun $binder => $body)
  unpackArguments arguments body

/-- The frame condition of a specification: every resource family the
function uses is unchanged except at the addresses the `modifies` clause
lists.  Without a clause no global memory changes at all, which the abstract
state expresses directly. -/
private def frameCondition (resourceTypes : Array (TSyntax `ident))
    (world initial final : TSyntax `term)
    (clause? : Option Syntax) : CommandElabM (Option (TSyntax `term)) := do
  let some clause := clause?
    | return some (← `($final = $initial))
  let mut targets : Array (Name × Array (TSyntax `term)) := #[]
  for target in clause[1].getSepArgs do
    let family ← Move.Verify.Source.canonicalResourceName ⟨target[0]⟩
    let address? : Option (TSyntax `term) :=
      if target.getNumArgs == 4 then some ⟨target[2]⟩ else none
    match targets.findIdx? (·.1 == family) with
    | some index =>
        let (name, addresses) := targets[index]!
        let addresses := match address? with
          | none => #[]
          | some address =>
              if addresses.isEmpty then #[] else addresses.push address
        targets := targets.set! index (name, addresses)
    | none =>
        targets := targets.push (family, address?.toArray)
  let mut conjuncts : Array (TSyntax `term) := #[]
  for resourceType in resourceTypes do
    let family ← Move.Verify.Source.canonicalResourceName resourceType
    let addresses? := (targets.find? (·.1 == family)).map (·.2)
    if let some addresses := addresses? then
      if addresses.isEmpty then
        continue
    let address := mkIdentFrom resourceType `_moveSpecAddress
    let addressTerm : TSyntax `term := ⟨address.raw⟩
    let mut body ←
      `(Move.Semantics.ResourceStore.lookup (State := $world)
            (Value := $resourceType) $final $addressTerm =
          Move.Semantics.ResourceStore.lookup (State := $world)
            (Value := $resourceType) $initial $addressTerm)
    for modified in (addresses?.getD #[]).reverse do
      body ← `($addressTerm ≠ $modified → $body)
    conjuncts := conjuncts.push (← `(∀ $address:ident, $body))
  if conjuncts.isEmpty then
    return none
  let mut frame := conjuncts[0]!
  for index in [1:conjuncts.size] do
    frame ← `($frame ∧ $(conjuncts[index]!))
  return some frame

/-- Where a contract excuses its postcondition.  A written `may_abort`
component is used directly; otherwise it is the states in which the declared
abort predicate admits some outcome. -/
private def mayAbortLambdaFor (arguments : Array (TSyntax `ident))
    (initial : TSyntax `ident) (abortsLambda : TSyntax `term)
    (written? : Option (TSyntax `term)) : MacroM (TSyntax `term) := do
  match written? with
  | some condition => clauseLambda arguments #[( `initial, initial)] condition
  | none =>
      `(fun _moveSpecArgs _moveSpecInitial =>
          ∃ _moveSpecAbortCode,
            $abortsLambda _moveSpecArgs _moveSpecInitial _moveSpecAbortCode)

/-- Further invariant clauses of a data specification; they are conjoined. -/
declare_syntax_cat moveExtraInvariant
scoped syntax ";" "invariant " term : moveExtraInvariant

/-- A data invariant: every value of the named struct or enum satisfies the
declared conditions, and carries their proof.  `this` denotes the value being
constrained and a leading `.field` abbreviates `this.field`.  Clauses read
like the other spec blocks and may be repeated:

```lean
spec Map {K} {V} where
  invariant Model.SortedEntries this.entries.toList
```

The declaration is consumed by the enclosing `move_module`, which attaches
the invariant to the type it names. -/
scoped syntax (name := dataInvariantSpec)
  "spec " ident moveSpecBinder* " where "
    "invariant " term moveExtraInvariant* : command

/-- A global-invariant predicate, quantified over an address.  A *regular*
invariant `(all a: … R[a] …)` constrains the current state; an *update*
invariant `update (all a: … old(R[a]) … R[a] …)` relates the pre- and
post-state of a change.  The `exists<R>(a)` guard is implicit: the address
ranges over the stored resources, so absent addresses are unconstrained. -/
declare_syntax_cat moveGlobalInvariant
scoped syntax (name := globalInvariantRegular)
  "(" &"all" ident ": " term ")" : moveGlobalInvariant
scoped syntax (name := globalInvariantUpdate)
  &"update" "(" &"all" ident ": " term ")" : moveGlobalInvariant

/-- Further clauses of a global-invariant spec. -/
declare_syntax_cat moveExtraGlobalInvariant
scoped syntax ";" "invariant " moveGlobalInvariant : moveExtraGlobalInvariant

/-- A global invariant over the resource state, re-established at each change
(not at function end).  A regular invariant is assumed at reads and asserted
at writes; an `update` invariant is asserted at each write only:

```lean
spec global where
  invariant (all a: 0 < Counter[a].value.toNat);
  invariant update (all a: old(Counter[a]).value ≤ Counter[a].value)
```

Consumed by the enclosing `move_module`. -/
scoped syntax (name := globalInvariantSpec) (priority := high)
  "spec " &"global" " where "
    "invariant " moveGlobalInvariant moveExtraGlobalInvariant* : command

/-- Whether `name` occurs as an identifier anywhere in `stx`. -/
partial def occursIdentifier (name : Name) (stx : Syntax) : Bool :=
  (stx.isIdent && stx.getId == name) ||
    stx.getArgs.any (occursIdentifier name)

/-- Whether an `old(…)` pre-state observation occurs anywhere in `stx`. -/
partial def occursOld (stx : Syntax) : Bool :=
  stx.getKind == ``oldResourceTerm || stx.getArgs.any occursOld

/-- Rewrite the resource places of a global-invariant body over the bound
address `addr` into a state predicate: `R[addr]` becomes `get (Value := R)
state addr` and `exists<R>(addr)` becomes `contains (Value := R) state addr`,
where `state` is the post-state (`old(…)` selects the pre-state).  Returns the
rewritten body, the families accessed *by value* with their `old`-ness (needing
an existence guard), and every family the body mentions. -/
partial def rewriteStatePlaces (addr : Name) (preState postState addrIdent : Syntax)
    (inOld : Bool) (body : Syntax) :
    MacroM (Syntax × Array (Syntax × Bool) × Array Syntax) := do
  let state := if inOld then preState else postState
  let stateT : TSyntax `term := ⟨state⟩
  let addrT : TSyntax `term := ⟨addrIdent⟩
  if body.getKind == ``oldResourceTerm && body.getNumArgs == 3 then
    rewriteStatePlaces addr preState postState addrIdent true body[1]
  else if body.getKind == `«term__[_]» && body.getNumArgs == 4 &&
      body[0].isIdent && body[2].isIdent && body[2].getId == addr then
    let famT : TSyntax `term := ⟨body[0]⟩
    let repl ← `(Move.Semantics.ResourceStore.get (Value := $famT) $stateT $addrT)
    return (repl.raw, #[(body[0], inOld)], #[body[0]])
  else if body.getKind == ``resourceExistsTerm && body.getNumArgs == 5 &&
      body[3].isIdent && body[3].getId == addr then
    let famT : TSyntax `term := ⟨body[1]⟩
    let repl ← `(Move.Semantics.ResourceStore.contains (Value := $famT) $stateT $addrT)
    return (repl.raw, #[], #[body[1]])
  else
    let mut valueAccess : Array (Syntax × Bool) := #[]
    let mut allFamilies : Array Syntax := #[]
    let mut args : Array Syntax := #[]
    for child in body.getArgs do
      let (child', va, fs) ← rewriteStatePlaces addr preState postState addrIdent inOld child
      args := args.push child'; valueAccess := valueAccess ++ va; allFamilies := allFamilies ++ fs
    return (body.setArgs args, valueAccess, allFamilies)

/-- Desugar one global-invariant clause into `(isUpdate, families, addr,
atBody)`.  `atBody` is the *per-address* predicate `guard → body` over the
fixed binders `_moveSpecState` (regular) or `_moveSpecPre`/`_moveSpecPost`
(update) and the address `addr`; the full invariant is `∀ addr, atBody`.
Factoring out `atBody` lets the generated reestablishment lemmas name the
changed address's obligation without unfolding the whole quantifier.  The guard
requires existence of each value-accessed family at `addr`.  `families` is
every family the clause mentions — the invariant is registered under each. -/
def elabGlobalInvariantClause (clause : Syntax) :
    MacroM (Bool × Array (TSyntax `ident) × TSyntax `ident × TSyntax `term) := do
  let isUpdate := clause.isOfKind ``globalInvariantUpdate
  let offset := if isUpdate then 1 else 0
  let addr := clause[offset + 2]
  let bodyStx := clause[offset + 4]
  unless addr.isIdent do
    Macro.throwErrorAt clause "a global invariant must bind an address, `all a: …`"
  unless isUpdate do
    if occursOld bodyStx then
      Macro.throwErrorAt clause
        "a regular global invariant may not use `old`; write `invariant update (…)`"
  let addrIdent := mkIdentFrom bodyStx `_moveSpecAddr
  let preState := mkIdentFrom bodyStx `_moveSpecPre
  let postState := mkIdentFrom bodyStx (if isUpdate then `_moveSpecPost else `_moveSpecState)
  let (rewritten, valueAccess, allFamilies) ←
    rewriteStatePlaces addr.getId preState.raw postState.raw addrIdent.raw false bodyStx
  let mut families : Array (TSyntax `ident) := #[]
  for f in allFamilies do
    unless families.any (·.getId == f.getId) do families := families.push ⟨f⟩
  if families.isEmpty then
    Macro.throwErrorAt clause
      "a global invariant must reference a stored resource `R[a]` or `exists<R>(a)`"
  if occursIdentifier addr.getId rewritten then
    Macro.throwErrorAt addr
      "the quantified address may appear only inside resource places `R[a]`"
  -- Existence guards for value-accessed families (deduplicated by family and
  -- `old`-ness), so `R[a].field` constrains only addresses where `R` exists.
  let mut guards : Array (TSyntax `term) := #[]
  let mut seen : Array (Syntax × Bool) := #[]
  for (fam, fromOld) in valueAccess do
    unless seen.any (fun (f, o) => f.getId == fam.getId && o == fromOld) do
      seen := seen.push (fam, fromOld)
      let famT : TSyntax `term := ⟨fam⟩
      let stateT : TSyntax `term := ⟨(if fromOld then preState else postState).raw⟩
      let addrT : TSyntax `term := ⟨addrIdent.raw⟩
      guards := guards.push
        (← `(Move.Semantics.ResourceStore.contains (Value := $famT) $stateT $addrT))
  let bodyTerm : TSyntax `term := ⟨rewritten⟩
  let guarded ← if h : guards.size = 0 then pure bodyTerm else do
    let mut conjunction := guards[guards.size - 1]!
    for i in [1:guards.size] do
      conjunction ← `($(guards[guards.size - 1 - i]!) ∧ $conjunction)
    `($conjunction → $bodyTerm)
  return (isUpdate, families, addrIdent, guarded)

@[macro globalInvariantSpec] def expandGlobalInvariantSpec : Macro := fun stx =>
  Macro.throwErrorAt stx
    "a global invariant must be declared inside a `move_module`"

@[macro dataInvariantSpec] def expandDataInvariantSpec : Macro := fun stx =>
  Macro.throwErrorAt stx
    ("a data invariant must be declared inside the `move_module` which " ++
      "declares its type")

/-- Total primitives of the relational semantics, with the lemma that says
so.  Every step of the discharger below applies exactly one of these, or one
combinator rule, chosen by the head symbol of the goal — it never lets
unification unfold a body. -/
private def totalPrimitiveLemma : Name → Option Name
  | ``Move.Semantics.Spec.pure => some ``Move.Semantics.Spec.pure_undefined
  | ``Move.Semantics.Spec.abort => some ``Move.Semantics.Spec.abort_undefined
  | ``Move.Semantics.Spec.bottom => some ``Move.Semantics.Spec.bottom_undefined
  | ``Move.Semantics.Spec.get => some ``Move.Semantics.Spec.total_get
  | ``Move.Semantics.Spec.set => some ``Move.Semantics.Spec.total_set
  | ``Move.Semantics.Spec.modify => some ``Move.Semantics.Spec.total_modify
  | ``Move.Semantics.Spec.ofTxn => some ``Move.Semantics.Spec.total_ofTxn
  | ``Move.Semantics.Checked.addSpec => some ``Move.Semantics.Checked.total_addSpec
  | ``Move.Semantics.Checked.subSpec => some ``Move.Semantics.Checked.total_subSpec
  | ``Move.Semantics.Checked.mulSpec => some ``Move.Semantics.Checked.total_mulSpec
  | ``Move.Semantics.Checked.divSpec => some ``Move.Semantics.Checked.total_divSpec
  | ``Move.Semantics.Checked.modSpec => some ``Move.Semantics.Checked.total_modSpec
  | ``Move.Semantics.Checked.shlSpec => some ``Move.Semantics.Checked.total_shlSpec
  | ``Move.Semantics.Checked.shrSpec => some ``Move.Semantics.Checked.total_shrSpec
  | ``Move.Semantics.Checked.castSpec => some ``Move.Semantics.Checked.total_castSpec
  | ``Move.Semantics.Resource.containsSpec =>
      some ``Move.Semantics.Resource.total_containsSpec
  | ``Move.Semantics.Resource.borrowSpec =>
      some ``Move.Semantics.Resource.total_borrowSpec
  | ``Move.Semantics.Resource.moveFromSpec =>
      some ``Move.Semantics.Resource.total_moveFromSpec
  | ``Move.Semantics.Resource.moveToSpec =>
      some ``Move.Semantics.Resource.total_moveToSpec
  | ``Move.Semantics.Vector.borrowElemSpec =>
      some ``Move.Semantics.Vector.total_borrowElemSpec
  | ``Move.Semantics.Vector.setSpec => some ``Move.Semantics.Vector.total_setSpec
  | ``Move.Semantics.Vector.insertSpec =>
      some ``Move.Semantics.Vector.insertSpec_undefined
  | ``Move.Semantics.Vector.removeSpec =>
      some ``Move.Semantics.Vector.removeSpec_undefined
  | _ => none

/-- The computation a well-definedness goal `¬ X.undefined s` is about. -/
private def definednessSubject? (target : Expr) : Option Expr := do
  guard (target.isAppOfArity ``Not 1)
  let inner := target.appArg!
  guard (inner.isAppOfArity ``Move.Semantics.Spec.undefined 4)
  return inner.getArg! 2

/-- A hypothesis `∀ a s, ¬ (recursive a).undefined s` for the recursive call
of an enclosing fixed point. -/
private def recursiveDefinedHypothesis? (recursive : FVarId) :
    Lean.Elab.Tactic.TacticM (Option FVarId) := do
  let lctx ← getLCtx
  for decl in lctx do
    if decl.isImplementationDetail then continue
    let type ← instantiateMVars decl.type
    let isHypothesis ← Lean.Meta.forallTelescopeReducing type fun _ body => do
      match definednessSubject? body with
      | some subject =>
          pure (subject.getAppFn == .fvar recursive && subject.getAppNumArgs == 1)
      | none => pure false
    if isHypothesis then return some decl.fvarId
  return none

/-- Establish that a source computation owes no proof, by its structure: the
primitives are total by definition, the combinators preserve it, and the only
obligation that survives is the data invariant of a created value, which is
left as the goal `Invariant`.  Each step is chosen by the goal's head symbol
and applies one lemma, so the cost is linear in the body and a body the
discharger does not understand fails immediately with its head symbol. -/
syntax (name := specDefined) "spec_defined" : tactic

private partial def dischargeDefined (fuel : Nat := 10000) :
    Lean.Elab.Tactic.TacticM Unit := Lean.Elab.Tactic.withMainContext do
  if fuel == 0 then throwError "spec_defined: body too large"
  let goal ← Lean.Elab.Tactic.getMainGoal
  let target ← instantiateMVars (← goal.getType)
  let some subject := definednessSubject? target
    | throwError "spec_defined: expected a goal `¬ X.undefined s`, got{indentExpr target}"
  let recurse := dischargeDefined (fuel - 1)
  let step (stx : TSyntax `tactic) : Lean.Elab.Tactic.TacticM Unit := do
    Lean.Elab.Tactic.evalTactic stx
  let onEveryGoal (action : Lean.Elab.Tactic.TacticM Unit) :
      Lean.Elab.Tactic.TacticM Unit := do
    let produced ← Lean.Elab.Tactic.getGoals
    let mut remaining := #[]
    for produced in produced do
      Lean.Elab.Tactic.setGoals [produced]
      action
      remaining := remaining ++ (← Lean.Elab.Tactic.getGoals)
    Lean.Elab.Tactic.setGoals remaining.toList
  match subject.getAppFn with
  | .lam .. | .letE .. | .mdata .. | .proj .. =>
      step (← `(tactic| dsimp only))
      Lean.Elab.Tactic.withMainContext do
        let after ← instantiateMVars (← (← Lean.Elab.Tactic.getMainGoal).getType)
        if after == target then
          throwError "spec_defined: cannot reduce{indentExpr subject}"
        recurse
  | .fvar recursive =>
      let some hypothesis ← recursiveDefinedHypothesis? recursive
        | throwError "spec_defined: no well-definedness hypothesis for recursive call{indentExpr subject}"
      step (← `(tactic| exact $(mkIdent (← hypothesis.getUserName)) _ _))
  | .const name _ =>
      if name == ``Move.Semantics.Spec.bind then
        step (← `(tactic| refine Move.Semantics.Spec.bind_defined ?_ (fun _ _ _ => ?_)))
        onEveryGoal recurse
      else if name == ``Move.Semantics.Spec.fix then
        step (← `(tactic| refine Move.Semantics.Spec.fix_defined
          (fun _recursive _recursiveDefined _ _ => ?_)))
        recurse
      else if name == ``Move.Semantics.withMutation then
        step (← `(tactic| refine Move.Semantics.withMutation_defined (fun _ => ?_)))
        recurse
      else if name == ``Move.Semantics.Resource.withBorrowMutSpec then
        step (← `(tactic| refine Move.Semantics.Resource.withBorrowMutSpec_defined
          (fun _ _ => ?_)))
        recurse
      else if name == ``Move.Semantics.Resource.withBorrowMutFocusSpec then
        step (← `(tactic| refine Move.Semantics.Resource.withBorrowMutFocusSpec_defined
          (fun _ => ?_)))
        recurse
      else if name == ``Move.Semantics.Vector.withBorrowElemMutSpec then
        step (← `(tactic| refine Move.Semantics.Vector.withBorrowElemMutSpec_defined
          (fun _ => ?_)))
        recurse
      else if name == ``Move.Semantics.Spec.certified then
        -- The one genuine obligation: the data invariant of a created value.
        step (← `(tactic| rw [Move.Semantics.Spec.certified_defined_iff]))
      else if name == ``ite || name == ``dite ||
          (← Lean.Meta.isMatcher name) then
        step (← `(tactic| split))
        onEveryGoal recurse
      else if let some lemma := totalPrimitiveLemma name then
        step (← `(tactic| apply $(mkIdent lemma)))
      else
        throwError "spec_defined: no well-definedness rule for `{name}`"
  | other =>
      throwError "spec_defined: unexpected head{indentExpr other}"

@[tactic specDefined] private def elabSpecDefined : Lean.Elab.Tactic.Tactic :=
  fun _ => dischargeDefined

/-- Discharge the data invariant of a value being created.  Creation is the
only place an invariant is owed, so this runs wherever a literal of a
certified type is elaborated; when it cannot close the goal the error points
at the literal. -/
syntax "move_invariant" : tactic
macro_rules
  | `(tactic| move_invariant) =>
    `(tactic| first
        | trivial
        | rfl
        | decide
        | assumption
        -- Every alternative must close the goal, or `first` would stop at
        -- a partial simplification and report it as the failure.
        | (simp [move_invariant_norm, move_norm, Nat.reducePow, Nat.reduceMod]
           done)
        | (simp [move_invariant_norm, move_norm, Nat.reducePow, Nat.reduceMod]
           omega)
        | (simp_all [move_invariant_norm, move_norm, Nat.reducePow,
            Nat.reduceMod]
           done)
        | (simp_all [move_invariant_norm, move_norm, Nat.reducePow,
            Nat.reduceMod]
           omega)
        | fail "cannot establish the data invariant of this value here (if this is a pattern of a certified enum, bind the proof with a trailing `_`)")

/-- Rewrite the leading-dot field abbreviations of an invariant condition to
projections of the constrained value. -/
partial def bindInvariantValue (this : TSyntax `ident) (condition : Syntax) :
    Syntax :=
  if condition.isOfKind ``Lean.Parser.Term.dotIdent then
    mkNode ``Lean.Parser.Term.proj #[this.raw, mkAtom ".", condition[1]]
  else if condition.isOfKind ``Lean.Parser.Term.matchAlt then
    -- The patterns of a `match this with | .ctor …` are constructor names,
    -- not field abbreviations: rewrite only the right-hand side.
    let last := condition.getNumArgs - 1
    condition.setArg last (bindInvariantValue this condition[last])
  else
    condition.setArgs (condition.getArgs.map (bindInvariantValue this))

/-- A contract stating only a postcondition.  For a *pure* function it is a
value predicate over the function applied to its arguments.  For an *effectful*
function it routes through the effectful path (trivial precondition and abort
behavior) so resource observations — `R[a]`, `exists<R>`, `old` — in the
postcondition are interpreted against global state. -/
scoped syntax (name := ensuresOnlySpec) "spec " ident moveSpecBinder* " where "
    "ensures " term : command

/-- Build the pure value contract `def f.contract : Prop := ∀ args, ensures`. -/
private def pureEnsuresContract (function : TSyntax `ident)
    (binders : Array (TSyntax `moveSpecBinder)) (postcondition : TSyntax `term) :
    MacroM (TSyntax `command) := do
  let contractName := associatedName function `contract
  let parameters ← unpackSpecParameters binders
  let application ← applyArguments function parameters.arguments
  let result := findResult? postcondition.raw |>.getD (mkIdentFrom postcondition `result)
  let ensured : TSyntax `term := ⟨bindResult postcondition.raw result⟩
  let result : TSyntax `ident := ⟨result⟩
  let body ← `((fun $result => $ensured) $application)
  let contract ← quantifyArguments parameters.arguments parameters.types body
  let contract ← quantifyContext parameters.context contract
  let command : TSyntax `command ← `(def $contractName : Prop := $contract)
  return command

/-- One global location a specification is allowed to change: a resource
family, optionally narrowed to one address. -/
declare_syntax_cat moveModifiesTarget
scoped syntax ident "[" term "]" : moveModifiesTarget
scoped syntax ident : moveModifiesTarget

/-- The global memory a function may change.  Everything else is unchanged,
so contracts never state a frame condition explicitly.  An omitted clause
means the function changes no global memory at all. -/
declare_syntax_cat moveModifiesClause
scoped syntax "modifies " moveModifiesTarget,+ ";" : moveModifiesClause

/-- A declarative contract for an effectful Move source function. The
function's relational semantics is generated from its retained `fun` body.
`initial`, `final`, `result`, and `abortCode` are implicit clause binders.
Resource descriptors only define the typed representation of global storage;
they do not restate the function's behavior. -/
scoped syntax (name := effectfulSourceSpec)
  "spec " ident moveSpecBinder* " on " term
    " using " "[" moveSpecResource,* "]" " where "
    "requires " term ";"
    (moveModifiesClause)?
    "ensures " term ";"
    "aborts " term (";" "may_abort " term)? : command

/-- User-facing effectful contract. The global state and one typed store
instance per borrowed resource are implicit and universally quantified. -/
scoped syntax (name := inferredEffectfulSourceSpec)
  "spec " ident moveSpecBinder* " where "
    "requires " term ";"
    (moveModifiesClause)?
    "ensures " term ";"
    "aborts " term (";" "may_abort " term)? : command

/-- Omitted effectful preconditions mean `True`. -/
scoped macro "spec " function:ident binder:moveSpecBinder* " on " world:term
    " using " "[" resources:moveSpecResource,* "]" " where "
    modifiesClause:(moveModifiesClause)?
    "ensures " postcondition:term ";"
    "aborts " abortCondition:term : command =>
  `(spec $function $binder* on $world using [$resources,*] where
      requires True;
      $[$modifiesClause]?
      ensures $postcondition;
      aborts $abortCondition)

/-- Omitted inferred effectful preconditions mean `True`. -/
scoped macro "spec " function:ident binder:moveSpecBinder* " where "
    modifiesClause:(moveModifiesClause)?
    "ensures " postcondition:term ";"
    "aborts " abortCondition:term : command =>
  `(spec $function $binder* where
      requires True;
      $[$modifiesClause]?
      ensures $postcondition;
      aborts $abortCondition)

/-- An effectful contract that declares no abort condition. Abort behavior is
then uninterpreted: every abort code is permitted, and no state excuses the
postcondition, so every successful execution must still establish `ensures`. -/
scoped macro "spec " function:ident binder:moveSpecBinder* " where "
    "requires " precondition:term ";"
    modifiesClause:(moveModifiesClause)?
    "ensures " postcondition:term : command =>
  `(spec $function $binder* where
      requires $precondition;
      $[$modifiesClause]?
      ensures $postcondition;
      aborts True;
      may_abort False)

/-- An explicit-resource contract that declares no abort condition. -/
scoped macro "spec " function:ident binder:moveSpecBinder* " on " world:term
    " using " "[" resources:moveSpecResource,* "]" " where "
    "requires " precondition:term ";"
    modifiesClause:(moveModifiesClause)?
    "ensures " postcondition:term : command =>
  `(spec $function $binder* on $world using [$resources,*] where
      requires $precondition;
      $[$modifiesClause]?
      ensures $postcondition;
      aborts True;
      may_abort False)

/-- Further Move-style abort clauses of a specification: each disjoins its
condition and exact code into the abort predicate. -/
declare_syntax_cat moveExtraAbortsIf
scoped syntax ";" "aborts_if " term " with " term : moveExtraAbortsIf

/-- Move-style abort clauses with exact abort codes, one or more. The abort
predicate is their disjunction.  That the postcondition needs to hold only
where the declared aborts are ruled out is the semantics of the contract
(`Move.Verify.Satisfies`), not anything written into the clauses. -/
scoped macro "spec " function:ident binder:moveSpecBinder* " where "
    "requires " precondition:term ";"
    modifiesClause:(moveModifiesClause)?
    "ensures " postcondition:term ";"
    "aborts_if " condition:term " with " code:term
    more:moveExtraAbortsIf* : command => do
  let mut conditions := #[condition]
  let mut codes := #[code]
  for clause in more do
    conditions := conditions.push ⟨clause.raw[2]⟩
    codes := codes.push ⟨clause.raw[4]⟩
  let abortCode := mkIdentFrom condition `abortCode
  let mut abortsTerm ←
    `($(conditions[0]!) ∧ $abortCode = Move.Spec.abortCodeOf $(codes[0]!))
  let mut mayAbortTerm := conditions[0]!
  for i in [1:conditions.size] do
    let clauseTerm ←
      `($(conditions[i]!) ∧ $abortCode = Move.Spec.abortCodeOf $(codes[i]!))
    abortsTerm ← `($abortsTerm ∨ $clauseTerm)
    mayAbortTerm ← `($mayAbortTerm ∨ $(conditions[i]!))
  `(spec $function $binder* where
      requires $precondition;
      $[$modifiesClause]?
      ensures $postcondition;
      aborts $abortsTerm;
      may_abort $mayAbortTerm)

scoped macro "spec " function:ident binder:moveSpecBinder* " where "
    modifiesClause:(moveModifiesClause)?
    "ensures " postcondition:term ";"
    "aborts_if " condition:term " with " code:term
    more:moveExtraAbortsIf* : command =>
  `(spec $function $binder* where
      requires True;
      $[$modifiesClause]?
      ensures $postcondition;
      aborts_if $condition with $code
      $more:moveExtraAbortsIf*)

/-- Move-style abort clause which constrains the abort condition but permits
any abort code. -/
scoped macro "spec " function:ident binder:moveSpecBinder* " where "
    "requires " precondition:term ";"
    modifiesClause:(moveModifiesClause)?
    "ensures " postcondition:term ";"
    "aborts_if " condition:term : command =>
  `(spec $function $binder* where
      requires $precondition;
      $[$modifiesClause]?
      ensures $postcondition;
      aborts $condition;
      may_abort $condition)

scoped macro "spec " function:ident binder:moveSpecBinder* " where "
    modifiesClause:(moveModifiesClause)?
    "ensures " postcondition:term ";"
    "aborts_if " condition:term : command =>
  `(spec $function $binder* where
      requires True;
      $[$modifiesClause]?
      ensures $postcondition;
      aborts_if $condition)

/-- The registered global-invariant body for each resource family that has
one, among the families a function uses. -/
private def globalInvariantsFor (resourceTypes : Array (TSyntax `ident)) :
    CommandElabM (Array (TSyntax `ident × TSyntax `ident)) := do
  let env ← getEnv
  let mut result := #[]
  for family in resourceTypes do
    let familyName ← Move.Verify.Source.canonicalResourceName family
    -- Only regular invariants are assumed on entry; `update` invariants
    -- constrain transitions and are asserted at writes only.
    for (isUpdate, body, _) in Move.globalInvariants env familyName do
      unless isUpdate do
        result := result.push (family, mkIdentFrom family body)
  return result

/-- Conjoin each regular global invariant `Inv initial` into the entry
precondition: the invariant is assumed on entry (it holds because every prior
write re-established it) and re-established at each write. -/
private def assumeGlobalInvariants (initial : TSyntax `term)
    (invariants : Array (TSyntax `ident × TSyntax `ident))
    (precondition : TSyntax `term) : CommandElabM (TSyntax `term) := do
  let mut condition := precondition
  for (_, body) in invariants do
    condition ← `($condition ∧ $body $initial)
  return condition

/-- Build `def f.contract : Prop := …Satisfies…` for an effectful function from
its declarative clauses.  Shared by the surface elaborator and the ensures-only
routing (which supplies a trivial precondition and abort behavior). -/
def buildEffectfulContract (function : TSyntax `ident)
    (binders : Array (TSyntax `moveSpecBinder))
    (precondition : TSyntax `term) (modifiesClause? : Option Syntax)
    (postcondition abortCondition : TSyntax `term)
    (mayAbortCondition? : Option (TSyntax `term)) : CommandElabM Unit := do
  let parameters ← liftMacroM <| unpackSpecParameters binders
  let arguments := parameters.arguments
  let argsType ← liftMacroM <| argumentType parameters.types
  let world := mkIdentFrom function `_moveSpecState
  let resourceTypes ← Move.Verify.Source.inferredResources function.raw
  let sourceSpecName := associatedName function `sourceSpec
  let sourceSpecFullName := (← getCurrNamespace) ++ sourceSpecName.getId
  let hasSourceSpec := (← getEnv).contains sourceSpecFullName
  let resultType ← Move.Verify.Source.resultTypeOf function.raw
  let sourceResultType ← match parameters.mutableParameter? with
    | none => pure resultType
    | some (_, referent) => `($resultType × $referent)
  let generated? ← if hasSourceSpec then
      pure none
    else do
      let recursive ← Move.Verify.Source.isRecursive function.raw
      if recursive && parameters.mutableParameter?.isSome then
        throwErrorAt function
          "recursive source contracts with mutable-reference parameters are not yet supported"
      let recursiveName := mkIdentFrom function `_moveSpecRecursive
      let recursiveTerm : TSyntax `term := ⟨recursiveName.raw⟩
      let (body, _) ←
        Move.Verify.Source.translateWithStores function.raw world resourceTypes
          (if recursive then some recursiveTerm else none)
          parameters.mutableParameter?
      pure (some (body, recursive, recursiveName))
  let contractName := associatedName function `contract
  let initial := mkIdentFrom precondition `_moveSpecInitial
  let final := mkIdentFrom postcondition `_moveSpecFinal
  let result := mkIdentFrom postcondition `result
  let abortCode := mkIdentFrom abortCondition `abortCode
  let initialTerm : TSyntax `term := ⟨initial.raw⟩
  let finalTerm : TSyntax `term := ⟨final.raw⟩
  let globalInvariants ← globalInvariantsFor resourceTypes
  let precondition ← Move.Verify.Source.rewriteClause
    resourceTypes initialTerm initialTerm precondition
  let precondition ← assumeGlobalInvariants initialTerm globalInvariants precondition
  let output := mkIdentFrom postcondition `_moveSpecOutput
  let outputTerm : TSyntax `term := ⟨output.raw⟩
  let mut postconditionRaw := postcondition.raw
  if let some (parameter, _) := parameters.mutableParameter? then
    let resultValue ← `($outputTerm.1)
    postconditionRaw := bindImplicit `result postconditionRaw resultValue.raw
    let finalReferent ← `($outputTerm.2)
    postconditionRaw ← liftMacroM <|
      rewriteMutablePost parameter finalReferent postconditionRaw
  let postcondition ← Move.Verify.Source.rewriteClause
    resourceTypes finalTerm initialTerm ⟨postconditionRaw⟩
  let frame? ← frameCondition resourceTypes ⟨world.raw⟩ initialTerm finalTerm
    modifiesClause?
  let abortCondition ← Move.Verify.Source.rewriteClause
    resourceTypes initialTerm initialTerm abortCondition
  let requiresLambda ← liftMacroM <| clauseLambda arguments
    #[( `initial, initial)] precondition
  let ensuresLambda ← liftMacroM <| match parameters.mutableParameter? with
    | none => clauseLambda arguments
        #[( `initial, initial), (`result, result), (`final, final)] postcondition
    | some _ => clauseLambda arguments
        #[( `initial, initial), (`_moveSpecOutput, output), (`final, final)] postcondition
  let abortsLambda ← liftMacroM <| clauseLambda arguments
    #[( `initial, initial), (`abortCode, abortCode)] abortCondition
  let mayAbortCondition? ← match mayAbortCondition? with
    | none => pure none
    | some condition => do
        let rewritten ← Move.Verify.Source.rewriteClause
          resourceTypes initialTerm initialTerm condition
        pure (some rewritten)
  let mayAbortLambda ← liftMacroM <|
    mayAbortLambdaFor arguments initial abortsLambda mayAbortCondition?
  let frameLambda ← liftMacroM <| match frame? with
    | none => `(fun _moveSpecArgs _moveSpecInitial _moveSpecFinal => True)
    | some frame => clauseLambda arguments
        #[( `initial, initial), (`final, final)] frame
  let worldBinder ← `(bracketedBinder| {$world : Type})
  let mut storeBinders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) := #[]
  for (resourceType, index) in resourceTypes.zipIdx do
    let storeName := mkIdentFrom resourceType
      (Name.mkSimple s!"_moveSpecStore{index}")
    let storeBinder ← `(bracketedBinder|
      [$storeName : Move.Semantics.ResourceStore $world $resourceType])
    storeBinders := storeBinders.push storeBinder
  if let some (body, recursive, recursiveName) := generated? then
    let sourceLambda ← liftMacroM <| unpackArguments arguments body
    if recursive then
      let bodySpecName := associatedName function `bodySpec
      let recursiveBinder ← `(bracketedBinder|
        ($recursiveName : $argsType → Move.Semantics.Spec $world $resultType))
      let bodyCommand ← `(noncomputable def $bodySpecName $parameters.context* $worldBinder
          $storeBinders* $recursiveBinder :
          $argsType → Move.Semantics.Spec $world $sourceResultType := $sourceLambda)
      elabCommand bodyCommand
      let sourceCommand ← `(noncomputable def $sourceSpecName $parameters.context* $worldBinder
          $storeBinders* : $argsType → Move.Semantics.Spec $world $sourceResultType :=
          Move.Semantics.Spec.fix $bodySpecName)
      elabCommand sourceCommand
    else
      let sourceCommand ← `(noncomputable def $sourceSpecName $parameters.context* $worldBinder
          $storeBinders* : $argsType → Move.Semantics.Spec $world $sourceResultType :=
          $sourceLambda)
      elabCommand sourceCommand
  let contractRecord ← `(@Move.Verify.Contract.mk $world $argsType $sourceResultType
    $requiresLambda $ensuresLambda $abortsLambda $mayAbortLambda $frameLambda)
  let mut contractBody ← `(Move.Verify.Satisfies $sourceSpecName $contractRecord)
  let mut resourcePairs : Array (TSyntax `ident × TSyntax `ident) := #[]
  for leftIndex in [:resourceTypes.size] do
    for rightIndex in [leftIndex + 1:resourceTypes.size] do
      -- Both directions: independence is symmetric, and a global-invariant
      -- reestablishment lemma frames the *other* family across the written
      -- one, needing whichever direction the write happens to take.
      resourcePairs := resourcePairs.push
        (resourceTypes[leftIndex]!, resourceTypes[rightIndex]!)
      resourcePairs := resourcePairs.push
        (resourceTypes[rightIndex]!, resourceTypes[leftIndex]!)
  for ((left, right), index) in resourcePairs.zipIdx.reverse do
    let independenceName := mkIdentFrom left
      (Name.mkSimple s!"_moveSpecIndependent{index}")
    contractBody ← `(∀ [$independenceName :
      Move.Semantics.IndependentResourceStores $world $left $right],
      $contractBody)
  for (resourceType, index) in resourceTypes.zipIdx.reverse do
    let storeName := mkIdentFrom resourceType
      (Name.mkSimple s!"_moveSpecStore{index}")
    contractBody ← `(∀ [$storeName : Move.Semantics.ResourceStore
      $world $resourceType], $contractBody)
  contractBody ← `(∀ ($world : Type), $contractBody)
  contractBody ← liftMacroM <| quantifyContext parameters.context contractBody
  let contractCommand ← `(def $contractName : Prop := $contractBody)
  elabCommand contractCommand

@[command_elab inferredEffectfulSourceSpec]
private def elabInferredEffectfulSourceSpec : CommandElab := fun stx => do
  let modifiesClause? : Option Syntax :=
    if stx[7].getNumArgs == 1 then some stx[7][0] else none
  let mayAbortCondition? : Option (TSyntax `term) :=
    if stx[13].getNumArgs == 3 then some ⟨stx[13][2]⟩ else none
  buildEffectfulContract ⟨stx[1]⟩ (stx[2].getArgs.map (⟨·⟩)) ⟨stx[5]⟩
    modifiesClause? ⟨stx[9]⟩ ⟨stx[12]⟩ mayAbortCondition?

@[command_elab ensuresOnlySpec]
private def elabEnsuresOnlySpec : CommandElab := fun stx => do
  let function : TSyntax `ident := ⟨stx[1]⟩
  let postcondition : TSyntax `term := ⟨stx[5]⟩
  let binders : Array (TSyntax `moveSpecBinder) := stx[2].getArgs.map (⟨·⟩)
  if ← Move.Verify.Source.isEffectfulFunction function.raw then
    -- Effectful: a trivial precondition and uninterpreted aborts, routed so
    -- the postcondition observes global state.
    buildEffectfulContract function binders (← `(True)) none postcondition
      (← `(True)) (some (← `(False)))
  else
    elabCommand (← liftMacroM (pureEnsuresContract function binders postcondition))

@[command_elab effectfulSourceSpec]
private def elabEffectfulSourceSpec : CommandElab := fun stx => do
  let function : TSyntax `ident := ⟨stx[1]⟩
  let binders : Array (TSyntax `moveSpecBinder) := stx[2].getArgs.map (⟨·⟩)
  let world : TSyntax `term := ⟨stx[4]⟩
  let resourceSyntax := stx[7].getSepArgs
  let mut resources := #[]
  let mut resourceTypes : Array (TSyntax `ident) := #[]
  for resource in resourceSyntax do
    let resource : TSyntax `moveSpecResource := ⟨resource⟩
    let `(moveSpecResource| $typeName:ident => $descriptor:term) := resource
      | throwErrorAt resource "invalid resource descriptor"
    resourceTypes := resourceTypes.push typeName
    resources := resources.push {
      typeName := ← Move.Verify.Source.canonicalResourceName typeName
      descriptor := descriptor
    }
  let precondition : TSyntax `term := ⟨stx[11]⟩
  let modifiesClause? : Option Syntax :=
    if stx[13].getNumArgs == 1 then some stx[13][0] else none
  let postcondition : TSyntax `term := ⟨stx[15]⟩
  let abortCondition : TSyntax `term := ⟨stx[18]⟩
  let mayAbortCondition? : Option (TSyntax `term) :=
    if stx[19].getNumArgs == 3 then some ⟨stx[19][2]⟩ else none
  let parameters ← liftMacroM <| unpackSpecParameters binders
  let arguments := parameters.arguments
  let argsType ← liftMacroM <| argumentType parameters.types
  let recursive ← Move.Verify.Source.isRecursive function.raw
  let recursiveName := mkIdentFrom function `_moveSpecRecursive
  let recursiveTerm : TSyntax `term := ⟨recursiveName.raw⟩
  let (body, resultType) ← Move.Verify.Source.translate function world.raw resources
    (if recursive then some recursiveTerm else none)
  let sourceSpecName := associatedName function `sourceSpec
  let contractName := associatedName function `contract
  let initial := mkIdentFrom precondition `_moveSpecInitial
  let final := mkIdentFrom postcondition `_moveSpecFinal
  let result := mkIdentFrom postcondition `result
  let abortCode := mkIdentFrom abortCondition `abortCode
  let initialTerm : TSyntax `term := ⟨initial.raw⟩
  let finalTerm : TSyntax `term := ⟨final.raw⟩
  let frame? ← frameCondition resourceTypes world
    initialTerm finalTerm modifiesClause?
  let requiresLambda ← liftMacroM <| clauseLambda arguments
    #[( `initial, initial)] precondition
  let ensuresLambda ← liftMacroM <| clauseLambda arguments
    #[( `initial, initial), (`result, result), (`final, final)] postcondition
  let abortsLambda ← liftMacroM <| clauseLambda arguments
    #[( `initial, initial), (`abortCode, abortCode)] abortCondition
  let mayAbortLambda ← liftMacroM <|
    mayAbortLambdaFor arguments initial abortsLambda mayAbortCondition?
  let frameLambda ← liftMacroM <| match frame? with
    | none => `(fun _moveSpecArgs _moveSpecInitial _moveSpecFinal => True)
    | some frame => clauseLambda arguments
        #[( `initial, initial), (`final, final)] frame
  let sourceLambda ← liftMacroM <| unpackArguments arguments body
  if recursive then
    let bodySpecName := associatedName function `bodySpec
    let recursiveBinder ← `(bracketedBinder|
      ($recursiveName : $argsType → Move.Semantics.Spec $world $resultType))
    let bodyCommand ← `(noncomputable def $bodySpecName $parameters.context* $recursiveBinder :
        $argsType → Move.Semantics.Spec $world $resultType := $sourceLambda)
    elabCommand bodyCommand
    let sourceCommand ← `(noncomputable def $sourceSpecName $parameters.context* : $argsType →
        Move.Semantics.Spec $world $resultType :=
        Move.Semantics.Spec.fix $bodySpecName)
    elabCommand sourceCommand
  else
    let sourceCommand ← `(noncomputable def $sourceSpecName $parameters.context* : $argsType →
        Move.Semantics.Spec $world $resultType := $sourceLambda)
    elabCommand sourceCommand
  let contractBody ← `(Move.Verify.Satisfies $sourceSpecName
      (@Move.Verify.Contract.mk $world $argsType $resultType
        $requiresLambda $ensuresLambda $abortsLambda $mayAbortLambda
        $frameLambda))
  let contractBody ← liftMacroM <| quantifyContext parameters.context contractBody
  let contractCommand ← `(def $contractName : Prop := $contractBody)
  elabCommand contractCommand

private partial def introUntilSatisfies : Lean.Elab.Tactic.TacticM Unit := Lean.Elab.Tactic.withMainContext do
  let target ← instantiateMVars (← Lean.Elab.Tactic.getMainTarget)
  if target.getAppFn.constName? == some ``Move.Verify.Satisfies then
    return
  match target with
  | .forallE .. =>
      let goal ← Lean.Elab.Tactic.getMainGoal
      let (_, next) ← goal.intro1P
      Lean.Elab.Tactic.replaceMainGoal [next]
      introUntilSatisfies
  | _ =>
      throwError
        "expected generated contract context followed by `Move.Verify.Satisfies`, got {target}"

private def introNamed (name : Name) : Lean.Elab.Tactic.TacticM Unit := Lean.Elab.Tactic.withMainContext do
  let goal ← Lean.Elab.Tactic.getMainGoal
  let (_, next) ← goal.intro name
  Lean.Elab.Tactic.replaceMainGoal [next]

private def hasLastName (name : Name) (suffix : String) : Bool :=
  match name with
  | .str _ last => last == suffix
  | _ => false

/-- Whether a source semantics *is* a fixed point — `Spec.fix body`, possibly
eta-expanded over the function's argument — as opposed to a function whose
body merely contains a loop (`fun n => Spec.fix loop ()`), which opens with
the ordinary rule and meets the loop as a `wp (fix …)` sub-goal. -/
private partial def hasFixHead : Expr → Bool
  | .lam _ _ body _ =>
      body.getAppFn.constName? == some ``Move.Semantics.Spec.fix &&
        body.getAppNumArgs == 5 && body.appArg! == .bvar 0
  | .letE _ _ _ body _ => hasFixHead body
  | expression =>
      expression.getAppFn.constName? == some ``Move.Semantics.Spec.fix &&
        expression.getAppNumArgs == 4

private def targetUsesFix : Lean.Elab.Tactic.TacticM Bool := Lean.Elab.Tactic.withMainContext do
  let target ← instantiateMVars (← Lean.Elab.Tactic.getMainTarget)
  unless target.getAppFn.constName? == some ``Move.Verify.Satisfies do
    throwError "expected a `Move.Verify.Satisfies` goal, got {target}"
  let arguments := target.getAppArgs
  if h : 2 ≤ arguments.size then
    let function := arguments[arguments.size - 2]'(by omega)
    return hasFixHead function
  return false

/-- Normalize the semantic `¬ mayAbort → ...` guard on the postcondition into
one negated hypothesis per declared abort condition — and none at all when no
abort condition is declared or it is `False`. `contract_intro` applies this
automatically; use it directly after a manual
`satisfies_of_wp`/`satisfies_fix_of_wp`. -/
syntax "abort_norm" : tactic
macro_rules
  | `(tactic| abort_norm) =>
    `(tactic|
      try simp only [false_and, and_false, exists_false, exists_const,
        not_false_eq_true, true_implies, not_true_eq_false, false_implies,
        implies_true, exists_and_left, exists_eq, exists_eq', and_true,
        not_or, exists_or, and_imp])

/-- Open the generated contract at the current goal and switch to weakest-
precondition reasoning. The source function is recovered from a goal of the
form `f.contract`. Nonrecursive functions use `satisfies_of_wp`; recursive
functions unfold `f.sourceSpec`, use `satisfies_fix_of_wp`, and expose
`recursive` and `recursiveVerified`. In both cases the authored source body is
unfolded and the remaining binders are named `args`, `initial`, and
`permitted`. -/
syntax (name := contractIntro) "contract_intro" : tactic

private def normalizeMayAbort : Lean.Elab.Tactic.TacticM Unit := do
  Lean.Elab.Tactic.evalTactic (← `(tactic| abort_norm))

@[tactic contractIntro]
private def elabContractIntro : Lean.Elab.Tactic.Tactic := fun stx => Lean.Elab.Tactic.withMainContext do
  let target ← instantiateMVars (← Lean.Elab.Tactic.getMainTarget)
  let some contractName := target.getAppFn.constName?
    | throwErrorAt stx
        "`contract_intro` must start on a generated goal of the form `f.contract`"
  let .str functionName "contract" := contractName
    | throwErrorAt stx
        "`contract_intro` expected a generated `f.contract` goal, got `{contractName}`"
  let sourceSpecName := functionName ++ `sourceSpec
  let bodySpecName := functionName ++ `bodySpec
  let env ← getEnv
  unless env.contains sourceSpecName do
    throwErrorAt stx
      "`contract_intro` supports effectful source contracts, but `{sourceSpecName}` is not defined"
  if let some info := env.find? sourceSpecName then
    if let some value := info.value? (allowOpaque := true) then
      for dependency in value.getUsedConstants do
        if hasLastName dependency "mutualSourceSpec" then
          throwErrorAt stx
            "`contract_intro` does not yet open mutually recursive contract families; use `satisfies_fixFamily` explicitly"
  let contract := mkIdentFrom stx contractName
  let sourceSpec := mkIdentFrom stx sourceSpecName
  Lean.Elab.Tactic.evalTactic (← `(tactic| unfold $contract))
  introUntilSatisfies
  Lean.Elab.Tactic.withMainContext do
    Lean.Elab.Tactic.evalTactic (← `(tactic| unfold $sourceSpec))
    Lean.Elab.Tactic.evalTactic (← `(tactic|
      try simp only [Move.Semantics.Spec.pure_bind]))
    if ← targetUsesFix then
      let bodySpec := mkIdentFrom stx bodySpecName
      Lean.Elab.Tactic.evalTactic (← `(tactic| apply Move.Verify.satisfies_fix_of_wp))
      introNamed `recursive
      introNamed `recursiveVerified
      introNamed `args
      introNamed `initial
      introNamed `permitted
      if env.contains bodySpecName then
        Lean.Elab.Tactic.evalTactic (← `(tactic| unfold $bodySpec))
      normalizeMayAbort
    else
      Lean.Elab.Tactic.evalTactic (← `(tactic| apply Move.Verify.satisfies_of_wp))
      introNamed `args
      introNamed `initial
      introNamed `permitted
      normalizeMayAbort

/-- Wrap a proof to record its cost when benchmarking.  With the environment
variable `MOVE_PROOF_BENCH` unset it is exactly the wrapped tactics, so it is
free to leave in every generated proof; when set it logs a parseable line

    ‖MOVE_BENCH‖\t<name>\t<heartbeats>\t<elapsed-ms>

per verified function.  Heartbeats are deterministic — independent of the
machine, load, and the `aptos` CLI the test suite otherwise spends its wall
time in — so summing them over the suite benchmarks proof work directly. -/
scoped syntax (name := moveBench) "move_bench " tacticSeq : tactic

@[tactic moveBench]
private def elabMoveBench : Lean.Elab.Tactic.Tactic := fun stx => do
  let proof := stx[1]
  match ← IO.getEnv "MOVE_PROOF_BENCH" with
  | none => Lean.Elab.Tactic.evalTactic proof
  | some _ =>
      let name := (← Lean.Elab.Term.getDeclName?).getD `_
      let name := name.replacePrefix (`_root_) Name.anonymous
      let start ← IO.monoNanosNow
      let (_, heartbeats) ← Lean.withHeartbeats (Lean.Elab.Tactic.evalTactic proof)
      let elapsed := (← IO.monoNanosNow) - start
      Lean.logInfo m!"‖MOVE_BENCH‖\t{name}\t{heartbeats}\t{elapsed / 1000000}"

/-- Prove the contract associated with the named source function with an
explicit tactic proof. Requiring `by` keeps the command unambiguous when the
next Move declaration starts with the term-level keyword `fun`. -/
scoped macro "verify " function:ident " by " proof:tacticSeq : command => do
  let contractName := associatedName function `contract
  let verifiedName := associatedName function `verified
  `(theorem $verifiedName : $contractName := by move_bench $proof)

/-- Symbolically execute the supported effectful source fragment and discharge
its declarative contract using the typed store laws and arithmetic solver. -/
scoped syntax (name := automaticSourceVerify) "verify " ident : command

@[command_elab automaticSourceVerify]
private def elabAutomaticSourceVerify : CommandElab := fun stx => do
  let function : TSyntax `ident := ⟨stx[1]⟩
  let functionName := (← getCurrNamespace) ++ function.getId
  -- Parse the fully qualified source name as a term. Tactic identifiers do not
  -- inherit the declaration-name expansion used by command elaboration.
  let parseTerm (name : Name) : CommandElabM (TSyntax `term) := do
    match Lean.Parser.runParserCategory (← getEnv) `term name.toString with
    | .ok parsed => pure ⟨parsed⟩
    | .error message => throwErrorAt function
        "failed to generate verification name `{name}`: {message}"
  let sourceSpecName := functionName ++ `sourceSpec
  let contractName := associatedName function `contract
  let verifiedName := associatedName function `verified
  unless (← getEnv).contains sourceSpecName do
    let functionTerm ← parseTerm functionName
    let functionLemma ←
      `(Lean.Parser.Tactic.simpLemma| $functionTerm:term)
    let env ← getEnv
    let mut unfoldLemmas := #[functionLemma]
    if let some info := env.find? functionName then
      if let some value := info.value? (allowOpaque := true) then
        for dependency in value.getUsedConstants do
          if dependency != functionName &&
              Move.isMoveFunction env dependency then
            let dependencyTerm ← parseTerm dependency
            let dependencyLemma ←
              `(Lean.Parser.Tactic.simpLemma| $dependencyTerm:term)
            unfoldLemmas := unfoldLemmas.push dependencyLemma
    let command ← `(theorem $verifiedName : $contractName := by
      move_bench
      unfold $contractName
      simp_all [$unfoldLemmas,*, move_spec, move_invariant_norm, move_norm,
        Nat.reducePow, Nat.reduceMod, Move.UInt.numeral_eq_ofNat, and_assoc,
        exists_const] <;>
      (try uint_bounds) <;>
      grind [Move.UInt.toNat_ofNat_u8, Move.UInt.toNat_ofNat_u16,
        Move.UInt.toNat_ofNat_u32, Move.UInt.toNat_ofNat_u64,
        Move.UInt.toNat_ofNat_u128, Move.UInt.toNat_ofNat_u256,
        Move.UInt.toNat_zero, Move.UInt.toNat_one,
        Move.UInt.toNat_cast,
        Move.UInt.toNat_lt,
        Move.Semantics.ResourceStore.get, Move.Semantics.ResourceStore.contains,
        Move.Semantics.ResourceStore.get_insert_same,
        Move.UInt.lt_iff_toNat_lt, Move.UInt.le_iff_toNat_le])
    elabCommand command
    return
  let sourceSpecTerm ← parseTerm sourceSpecName
  let sourceSpecLemma ←
    `(Lean.Parser.Tactic.simpLemma| $sourceSpecTerm:term)
  let env ← getEnv
  let mut sourceUnfoldLemmas := #[sourceSpecLemma]
  if let some info := env.find? sourceSpecName then
    if let some value := info.value? (allowOpaque := true) then
      for dependency in value.getUsedConstants do
          if dependency != sourceSpecName &&
              (Move.isMoveFunction env dependency ||
                nameSuffix? dependency == some "sourceSpec" ||
                nameSuffix? dependency == some "bodySpec") then
          let dependencyTerm ← parseTerm dependency
          let dependencyLemma ←
            `(Lean.Parser.Tactic.simpLemma| $dependencyTerm:term)
          sourceUnfoldLemmas := sourceUnfoldLemmas.push dependencyLemma
  -- Callees are inlined: their `sourceSpec`s are unfolded into the caller's
  -- body before symbolic execution (the function's own `sourceSpec` and
  -- `bodySpec` are already opened by `contract_intro`).
  let calleeUnfoldLemmas := sourceUnfoldLemmas.filter fun lemma =>
    lemma.raw.getId != sourceSpecName &&
      lemma.raw.getId != functionName ++ `bodySpec
  let command ← `(set_option maxHeartbeats 400000 in
    theorem $verifiedName : $contractName := by
      move_bench
      -- Open the contract into one weakest-precondition goal, then execute
      -- the body symbolically by the wp rules: linear in the body, no
      -- existentials, well-definedness discharged per primitive, the only
      -- residue being a created value's data invariant.
      contract_intro
      try simp only [$calleeUnfoldLemmas,*, wp_norm, move_norm, move_data,
        Nat.reducePow, Nat.reduceMod, and_imp, forall_eq, forall_eq',
        Classical.not_not,
        exists_eq_left, exists_eq_left', exists_eq_right, exists_and_left,
        exists_and_right, and_true, true_and, true_implies, implies_true,
        and_self, Prod.mk.injEq, not_false_eq_true, not_true_eq_false,
        ite_true, ite_false, dite_true, dite_false]
      all_goals
        simp_all (config := { maxSteps := 1000000 })
          [$calleeUnfoldLemmas,*, move_spec, move_data, move_invariant_norm,
            move_norm, Nat.reducePow, Nat.reduceMod,
            and_assoc, exists_const] <;>
        (try uint_bounds) <;>
        grind [Move.UInt.toNat_ofNat_u8, Move.UInt.toNat_ofNat_u16,
          Move.UInt.toNat_ofNat_u32, Move.UInt.toNat_ofNat_u64,
          Move.UInt.toNat_ofNat_u128, Move.UInt.toNat_ofNat_u256,
          Move.UInt.toNat_zero, Move.UInt.toNat_one,
          Move.UInt.toNat_cast,
          Move.UInt.toNat_lt, Nat.shiftRight_le,
          Move.Semantics.ResourceStore.get, Move.Semantics.ResourceStore.contains,
          Move.Semantics.ResourceStore.get_insert_same,
          Move.UInt.lt_iff_toNat_lt, Move.UInt.le_iff_toNat_le])
  elabCommand command

end Move.Spec
