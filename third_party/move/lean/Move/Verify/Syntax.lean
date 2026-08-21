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
instance : AbortCodeValue Move.U64 := ⟨Move.U64.toNat⟩

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
the Move compiler lowers. `U64` uses its mathematical value; every other
source type uses Move's sealed structural comparison marker. This is direct
dispatch on the type, rather than typeclass dispatch, so source verification
cannot select semantics different from the generated Move instruction. -/
noncomputable def logicalLT {T : Type} (left right : T) : Prop := by
  classical
  exact if h : T = Move.U64 then
    (cast h left).toNat < (cast h right).toNat
  else
    Move.Compare.Less left right

noncomputable instance (left right : T) : Decidable (logicalLT left right) := by
  classical
  unfold logicalLT
  split <;> infer_instance

@[simp] theorem logicalLT_u64 (left right : Move.U64) :
    logicalLT left right ↔ left.toNat < right.toNat := by simp [logicalLT]

/-- The compiler's generic comparison marker denotes the same native ordering
as the direct `U64` operation at a `U64` instantiation. The marker is opaque
in executable source, so this is the verification interface for that compiler
semantic fact. The comparator is fixed to `Move.Compare.genericLT`; it never
uses a caller-selected `LT` instance. -/
axiom logicalLT_move [Move.Compare.Total T] (left right : T) :
    logicalLT left right ↔
      @LT.lt T (Move.Compare.genericLT (T := T)) left right

attribute [simp] logicalLT_move

/-- Logical interpretation of authored `≤`, sealed to the same `U64`
semantics used by the generated Move `U64.lessEq` instruction. -/
def logicalLE (left right : Move.U64) : Prop := left.toNat ≤ right.toNat

instance (left right : Move.U64) : Decidable (logicalLE left right) :=
  by unfold logicalLE; infer_instance

@[simp] theorem logicalLE_u64 (left right : Move.U64) :
    logicalLE left right ↔ left.toNat ≤ right.toNat := Iff.rfl

/-- Sealed logical equality for authored `==`. It selects the same native
`U64.equal` semantics for `U64` and Move's fixed structural equality marker
for every other source type, without consulting a caller-provided `BEq`
instance when a source contract is generated. -/
noncomputable def logicalBEq {T : Type} (left right : T) : Bool := by
  classical
  exact if h : T = Move.U64 then
    decide ((cast h left).toNat = (cast h right).toNat)
  else
    Move.Compare.equal left right

@[simp] theorem logicalBEq_u64 (left right : Move.U64) :
    logicalBEq left right = true ↔ left.toNat = right.toNat := by
  simp [logicalBEq]

/-- The fixed generic equality marker is the source-level representation used
by Move's compiler for a type parameter constrained by `Compare.Total`. -/
axiom logicalBEq_move [Move.Compare.Total T] (left right : T) :
    logicalBEq left right = Move.Compare.equal left right

attribute [simp] logicalBEq_move

private def lastString? : Name → Option String
  | .str _ suffix => some suffix
  | _ => none

private def sameLastName (left right : Name) : Bool :=
  left == right || match lastString? left, lastString? right with
    | some left, some right => left == right
    | _, _ => false

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

private partial def collectResources (stx : Syntax)
    (resources : Array (TSyntax `ident) := #[]) : CommandElabM (Array (TSyntax `ident)) := do
  let mut resources := resources
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
  name == ``Move.write || name == ``Move.exists_ || name == ``Move.moveFrom ||
  name == ``Move.moveTo || name == ``Move.assert || name == ``Move.abort ||
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
      term[1].getAtomVal == "%")) ||
  term.getArgs.any containsArithmetic

private def checkedArithmeticCall? (term : TSyntax `term) :
    CommandElabM (Option (Name × TSyntax `term × TSyntax `term)) := do
  let some (head, arguments) := application? term | return none
  unless head.raw.isIdent && arguments.size == 2 do return none
  let functionName ← try
      pure (some (← resolveGlobalConstNoOverload head.raw))
    catch _ => pure none
  let operation? := functionName.bind fun functionName =>
    if functionName == ``Move.U64.add then some ``Move.Semantics.Checked.addSpec
    else if functionName == ``Move.U64.sub then some ``Move.Semantics.Checked.subSpec
    else if functionName == ``Move.U64.mul then some ``Move.Semantics.Checked.mulSpec
    else if functionName == ``Move.U64.div then some ``Move.Semantics.Checked.divSpec
    else if functionName == ``Move.U64.mod then some ``Move.Semantics.Checked.modSpec
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
  unless Move.moveFunAttr.hasTag env functionName ||
      Move.movePublicAttr.hasTag env functionName ||
      Move.moveEntryAttr.hasTag env functionName do
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
  if Move.moveFunAttr.hasTag env functionName ||
      Move.movePublicAttr.hasTag env functionName ||
      Move.moveEntryAttr.hasTag env functionName then
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
  | _ =>
      if let some call ← effectfulCallSpec? (expressionSpec context) context term then
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
        let updated ← updatePath parentValue finalField fields.toList
        let borrow ← `(Move.Semantics.withMutation $focused (fun $name => $nested))
        let after ← if continuation.isEmpty then
          finish context (← `(Move.Semantics.Spec.pure $outputTerm.1))
        else
          translateDo context continuation
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
        let updated ← updatePath ownerTerm replacementTerm fields.toList
        let borrow ← `(Move.Semantics.Resource.withBorrowMutFocusSpec $descriptor $key
            (fun $owner => $focused)
            (fun $owner $replacement => $updated)
            (fun $name => $nested))
        if continuation.isEmpty then
          pure borrow
        else
          let after ← translateDo context continuation
          `(Move.Semantics.Spec.bind $borrow (fun _moveSpecBorrowResult => $after))
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
  let spec ← translateTerm {
    world := ⟨world⟩
    resources
    functionName
    recursiveSpec?
    mutation? := mutableParameter?.map (·.1)
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

/-- Resource families actually borrowed by an effectful source function. -/
def inferredResources (function : Syntax) : CommandElabM (Array (TSyntax `ident)) := do
  let declaration ← declarationFor function
  let body ← sourceBody declaration
  collectResources body.raw

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

/-- A declarative contract for a pure Move source function. The result binder
denotes the result of applying the named function to the contract arguments. -/
scoped macro "spec " function:ident binder:moveSpecBinder* " where "
    "ensures " ensures:term : command => do
  let contractName := associatedName function `contract
  let parameters ← unpackSpecParameters binder
  let application ← applyArguments function parameters.arguments
  let result := findResult? ensures.raw |>.getD (mkIdentFrom ensures `result)
  let ensures : TSyntax `term := ⟨bindResult ensures.raw result⟩
  let result : TSyntax `ident := ⟨result⟩
  let body ← `((fun $result => $ensures) $application)
  let contract ← quantifyArguments parameters.arguments parameters.types body
  let contract ← quantifyContext parameters.context contract
  `(def $contractName : Prop := $contract)

/-- A declarative contract for an effectful Move source function. The
function's relational semantics is generated from its retained `fun` body.
`initial`, `final`, `result`, and `abortCode` are implicit clause binders.
Resource descriptors only define the typed representation of global storage;
they do not restate the function's behavior. -/
scoped syntax (name := effectfulSourceSpec)
  "spec " ident moveSpecBinder* " on " term
    " using " "[" moveSpecResource,* "]" " where "
    "requires " term ";"
    "ensures " term ";"
    "aborts " term : command

/-- User-facing effectful contract. The global state and one typed store
instance per borrowed resource are implicit and universally quantified. -/
scoped syntax (name := inferredEffectfulSourceSpec)
  "spec " ident moveSpecBinder* " where "
    "requires " term ";"
    "ensures " term ";"
    "aborts " term : command

/-- Omitted effectful preconditions mean `True`. -/
scoped macro "spec " function:ident binder:moveSpecBinder* " on " world:term
    " using " "[" resources:moveSpecResource,* "]" " where "
    "ensures " postcondition:term ";"
    "aborts " abortCondition:term : command =>
  `(spec $function $binder* on $world using [$resources,*] where
      requires True;
      ensures $postcondition;
      aborts $abortCondition)

/-- Omitted inferred effectful preconditions mean `True`. -/
scoped macro "spec " function:ident binder:moveSpecBinder* " where "
    "ensures " postcondition:term ";"
    "aborts " abortCondition:term : command =>
  `(spec $function $binder* where
      requires True;
      ensures $postcondition;
      aborts $abortCondition)

/-- Move-style abort clause with an exact abort code. -/
scoped macro "spec " function:ident binder:moveSpecBinder* " where "
    "requires " precondition:term ";"
    "ensures " postcondition:term ";"
    "aborts_if " condition:term " with " code:term : command => do
  let abortCode := mkIdentFrom condition `abortCode
  `(spec $function $binder* where
      requires $precondition;
      ensures $postcondition;
      aborts ($condition ∧ $abortCode = Move.Spec.abortCodeOf $code))

scoped macro "spec " function:ident binder:moveSpecBinder* " where "
    "ensures " postcondition:term ";"
    "aborts_if " condition:term " with " code:term : command =>
  `(spec $function $binder* where
      requires True;
      ensures $postcondition;
      aborts_if $condition with $code)

/-- Move-style abort clause which constrains the abort condition but permits
any abort code. -/
scoped macro "spec " function:ident binder:moveSpecBinder* " where "
    "requires " precondition:term ";"
    "ensures " postcondition:term ";"
    "aborts_if " condition:term : command =>
  `(spec $function $binder* where
      requires $precondition;
      ensures $postcondition;
      aborts $condition)

scoped macro "spec " function:ident binder:moveSpecBinder* " where "
    "ensures " postcondition:term ";"
    "aborts_if " condition:term : command =>
  `(spec $function $binder* where
      requires True;
      ensures $postcondition;
      aborts_if $condition)

@[command_elab inferredEffectfulSourceSpec]
private def elabInferredEffectfulSourceSpec : CommandElab := fun stx => do
  let function : TSyntax `ident := ⟨stx[1]⟩
  let binders : Array (TSyntax `moveSpecBinder) := stx[2].getArgs.map (⟨·⟩)
  let precondition : TSyntax `term := ⟨stx[5]⟩
  let postcondition : TSyntax `term := ⟨stx[8]⟩
  let abortCondition : TSyntax `term := ⟨stx[11]⟩
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
  let initial := mkIdentFrom precondition `initial
  let final := mkIdentFrom postcondition `final
  let result := mkIdentFrom postcondition `result
  let abortCode := mkIdentFrom abortCondition `abortCode
  let initialTerm : TSyntax `term := ⟨initial.raw⟩
  let finalTerm : TSyntax `term := ⟨final.raw⟩
  let precondition ← Move.Verify.Source.rewriteClause
    resourceTypes initialTerm initialTerm precondition
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
    $requiresLambda $ensuresLambda $abortsLambda)
  let mut contractBody ← `(Move.Verify.Satisfies $sourceSpecName $contractRecord)
  let mut resourcePairs : Array (TSyntax `ident × TSyntax `ident) := #[]
  for leftIndex in [:resourceTypes.size] do
    for rightIndex in [leftIndex + 1:resourceTypes.size] do
      resourcePairs := resourcePairs.push
        (resourceTypes[leftIndex]!, resourceTypes[rightIndex]!)
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

@[command_elab effectfulSourceSpec]
private def elabEffectfulSourceSpec : CommandElab := fun stx => do
  let function : TSyntax `ident := ⟨stx[1]⟩
  let binders : Array (TSyntax `moveSpecBinder) := stx[2].getArgs.map (⟨·⟩)
  let world : TSyntax `term := ⟨stx[4]⟩
  let resourceSyntax := stx[7].getSepArgs
  let mut resources := #[]
  for resource in resourceSyntax do
    let resource : TSyntax `moveSpecResource := ⟨resource⟩
    let `(moveSpecResource| $typeName:ident => $descriptor:term) := resource
      | throwErrorAt resource "invalid resource descriptor"
    resources := resources.push {
      typeName := ← Move.Verify.Source.canonicalResourceName typeName
      descriptor := descriptor
    }
  let precondition : TSyntax `term := ⟨stx[11]⟩
  let postcondition : TSyntax `term := ⟨stx[14]⟩
  let abortCondition : TSyntax `term := ⟨stx[17]⟩
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
  let initial := mkIdentFrom precondition `initial
  let final := mkIdentFrom postcondition `final
  let result := mkIdentFrom postcondition `result
  let abortCode := mkIdentFrom abortCondition `abortCode
  let requiresLambda ← liftMacroM <| clauseLambda arguments
    #[( `initial, initial)] precondition
  let ensuresLambda ← liftMacroM <| clauseLambda arguments
    #[( `initial, initial), (`result, result), (`final, final)] postcondition
  let abortsLambda ← liftMacroM <| clauseLambda arguments
    #[( `initial, initial), (`abortCode, abortCode)] abortCondition
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
        $requiresLambda $ensuresLambda $abortsLambda))
  let contractBody ← liftMacroM <| quantifyContext parameters.context contractBody
  let contractCommand ← `(def $contractName : Prop := $contractBody)
  elabCommand contractCommand

/-- Prove the contract associated with the named source function with an
explicit tactic proof. Requiring `by` keeps the command unambiguous when the
next Move declaration starts with the term-level keyword `fun`. -/
scoped macro "verify " function:ident " by " proof:tacticSeq : command => do
  let contractName := associatedName function `contract
  let verifiedName := associatedName function `verified
  `(theorem $verifiedName : $contractName := by $proof)

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
              (moveFunAttr.hasTag env dependency ||
                movePublicAttr.hasTag env dependency ||
                moveEntryAttr.hasTag env dependency) then
            let dependencyTerm ← parseTerm dependency
            let dependencyLemma ←
              `(Lean.Parser.Tactic.simpLemma| $dependencyTerm:term)
            unfoldLemmas := unfoldLemmas.push dependencyLemma
    let command ← `(theorem $verifiedName : $contractName := by
      unfold $contractName
      simp_all [$unfoldLemmas,*,
        Id.run, Pure.pure, Bind.bind,
        Move.U64.add, Move.U64.sub, Move.U64.mul,
        Move.U64.div, Move.U64.mod] <;>
      grind)
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
              (moveFunAttr.hasTag env dependency ||
                movePublicAttr.hasTag env dependency ||
                moveEntryAttr.hasTag env dependency ||
                nameSuffix? dependency == some "sourceSpec" ||
                nameSuffix? dependency == some "bodySpec") then
          let dependencyTerm ← parseTerm dependency
          let dependencyLemma ←
            `(Lean.Parser.Tactic.simpLemma| $dependencyTerm:term)
          sourceUnfoldLemmas := sourceUnfoldLemmas.push dependencyLemma
  let command ← `(set_option maxHeartbeats 2000000 in
    theorem $verifiedName : $contractName := by
      unfold $contractName Move.Verify.Satisfies
      intros
      constructor <;> intros
      all_goals
        simp_all (config := { maxSteps := 1000000 }) [$sourceUnfoldLemmas,*,
          Move.Semantics.ResourceStore.contains,
          Move.Semantics.ResourceStore.get,
          Move.Semantics.ResourceStore.descriptor,
          Move.Semantics.Resource.withBorrowMutFocusSpec,
          Move.Semantics.Resource.withBorrowMutSpec,
          Move.Semantics.Vector.borrowElemSpec,
          Move.Semantics.Vector.withBorrowElemMutSpec,
          Move.Semantics.Vector.insertSpec,
          Move.Semantics.Vector.removeSpec,
          Move.Semantics.withMutation,
          Bind.bind, Pure.pure,
          Move.Semantics.Spec.bind, Move.Semantics.Spec.pure,
          Move.Semantics.Spec.abort,
          Move.Semantics.Mutation.read, Move.Semantics.Mutation.write,
          Move.Vector.empty, Move.Vector.push, Move.Vector.set,
          Move.Vector.ofList,
          Move.Vector.toList,
          Move.Semantics.Checked.addSpec, Move.Semantics.Checked.subSpec,
          Move.Semantics.Checked.mulSpec, Move.Semantics.Checked.divSpec,
          Move.Semantics.Checked.modSpec,
          Move.U64.instOfNat, OfNat.ofNat,
          Move.U64.toNat, Move.U64.ofNat,
          Move.U64.add, Move.U64.sub, Move.U64.mul,
          Move.U64.div, Move.U64.mod] <;>
        grind)
  elabCommand command

end Move.Spec
