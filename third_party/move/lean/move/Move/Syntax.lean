-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import Lean.Elab.Do.InferControlInfo
import Move.Action
import Move.Attributes
import Move.ConventionalAddresses

/-!
# Leaner surface syntax

One scoped borrow parser covers reference types and Move 2 places. Assignment
uses Lean's existing `doReassign` syntax and dispatches by the left-hand local's
type: `MutRef α` writes through the reference, while ordinary `let mut` locals
fall back to Lean's built-in reassignment elaborator. `continue f args...`
marks one direct self-call for checked tail-call-to-loop lowering. `while` /
`loop` / `break` / `continue` are in-function loops: the `do` elaborator emits
first-order markers that Normalize lowers to internal CFG headers.
-/

namespace Move

open Lean Meta Elab Term

/-- Move's variant test, `value is Enum.Variant`. The constructor arity is
read from the elaborated enum, so nullary and payload variants share one
surface. -/
scoped syntax:50 (name := moveIsVariant) term:51 " is " ident : term

@[term_elab moveIsVariant]
private def elabMoveIsVariant : TermElab := fun stx expectedType? => do
  let value : TSyntax `term := ⟨stx[0]⟩
  let variant : TSyntax `ident := ⟨stx[2]⟩
  let constructor ← resolveGlobalConstNoOverload variant
  let some (.ctorInfo ctor) := (← getEnv).find? constructor
    | throwErrorAt variant "`{constructor}` is not an enum variant"
  unless moveEnumAttr.hasTag (← getEnv) ctor.induct do
    throwErrorAt variant "`{constructor}` is not a Move enum variant"
  let mut pattern : TSyntax `term := ⟨mkIdentFrom variant constructor⟩
  for _ in [0:ctor.numFields] do
    pattern ← `($pattern _)
  let expanded ← `(match $value:term with | $pattern:term => true | _ => false)
  elabTerm expanded expectedType?

/-! Compiler-only loop markers. They are private so authored Leaner cannot
forge CFG control by calling the normalizer protocol directly. -/

/-- Compiler-only source marker. Its monadic dependency keeps the call in
LCNF; normalization consumes it and attaches the half-open byte range to the
instructions produced by the following authored `do` element. -/
@[never_extract, noinline] partial def sourceSpanMarkerAction
    (start stop : Nat) (localName : String) : Action Unit :=
  sourceSpanMarkerAction start stop localName

@[never_extract, noinline] partial def sourceSpanMarkerId
    (start stop : Nat) (localName : String) : Id Unit :=
  sourceSpanMarkerId start stop localName

@[never_extract, noinline] def sourceNamedValue
    {α : Type} (_start _stop : Nat) (_localName : String) (value : α) : α :=
  value

@[simp] theorem sourceNamedValue_eq
    {α : Type} (start stop : Nat) (localName : String) (value : α) :
    sourceNamedValue start stop localName value = value := rfl

def isSourceSpanMarker (name : Name) : Bool :=
  name == ``sourceSpanMarkerAction || name == ``sourceSpanMarkerId

def isSourceNamedValue (name : Name) : Bool := name == ``sourceNamedValue

@[never_extract, noinline] private partial def loopEnter
    {σ : Type} [Inhabited σ] (label nonce arity token : Nat)
    (state : σ) : Nat :=
  loopEnter label nonce arity token state

@[never_extract, noinline] private partial def loopContinue
    {σ : Type} [Inhabited σ] (label nonce arity token : Nat)
    (state : σ) : Nat :=
  loopContinue label nonce arity token state

@[never_extract, noinline] private partial def loopContinueTail
    {σ : Type} [Inhabited σ] (label nonce arity token : Nat)
    (state : σ) : Nat :=
  loopContinueTail label nonce arity token state

@[never_extract, noinline] private partial def loopBreak
    {σ : Type} [Inhabited σ] (label nonce arity token : Nat)
    (state : σ) : Nat :=
  loopBreak label nonce arity token state

@[never_extract, noinline] private partial def loopTokenLive
    (token : Nat) : Bool :=
  loopTokenLive token

@[never_extract, noinline] private partial def loopTokenJoin
    (token outerToken : Nat) : Nat :=
  loopTokenJoin token outerToken

def isLoopEnterMarker (name : Name) : Bool := name == ``loopEnter
def isLoopContinueMarker (name : Name) : Bool := name == ``loopContinue
def isLoopContinueTailMarker (name : Name) : Bool := name == ``loopContinueTail
def isLoopBreakMarker (name : Name) : Bool := name == ``loopBreak
def isLoopTokenLiveMarker (name : Name) : Bool := name == ``loopTokenLive
def isLoopTokenJoinMarker (name : Name) : Bool := name == ``loopTokenJoin

private structure LoopMarkerRegistration where
  function : Name
  marker : Name
  label : Nat
  nonce : Nat
  deriving BEq

private def addLoopMarkerRegistration
    (registrations : List LoopMarkerRegistration)
    (registration : LoopMarkerRegistration) : List LoopMarkerRegistration :=
  if registrations.contains registration then registrations
  else registration :: registrations

private initialize loopMarkerRegistrations :
    SimplePersistentEnvExtension LoopMarkerRegistration
      (List LoopMarkerRegistration) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := addLoopMarkerRegistration
    addImportedFn := fun entries =>
      mkStateFromImportedEntries addLoopMarkerRegistration [] entries
  }

def isRegisteredLoopMarker (env : Environment) (function : Name)
    (marker : Name) (label nonce : Nat) : Bool :=
  (loopMarkerRegistrations.getState env).contains
    { function, marker, label, nonce }

private def registerLoopMarker (function marker : Name) (label nonce : Nat) :
  TermElabM Unit :=
  modifyEnv fun env =>
    loopMarkerRegistrations.addEntry env { function, marker, label, nonce }

scoped syntax:max (name := borrowTerm) "&" term:40 : term
scoped syntax:max (name := borrowMutTerm) "&mut " term:40 : term
scoped syntax:max (name := borrowIndexTerm) (priority := high)
  "&" ident "[" term "]" : term
scoped syntax:max (name := borrowMutIndexTerm) (priority := high)
  "&mut " ident "[" term "]" : term
scoped syntax:max (name := derefTerm) "*" term:max : term
scoped syntax:max (name := vectorLiteralTerm) "vector![" term,* "]" : term
scoped syntax:max (name := byteStringLiteral) ident noWs str : term
scoped syntax:max (name := typedIntegerLiteral) num noWs ident : term

declare_syntax_cat movePrimitiveMatchPattern
syntax (name := moveMatchLiteralPattern) num : movePrimitiveMatchPattern
syntax (name := moveMatchByteStringPattern)
  ident noWs str : movePrimitiveMatchPattern
syntax (name := moveMatchHalfOpenPattern)
  num ".." num : movePrimitiveMatchPattern
syntax (name := moveMatchClosedPattern)
  num "..=" num : movePrimitiveMatchPattern
syntax (name := moveMatchUpperOpenPattern)
  ".." num : movePrimitiveMatchPattern
syntax (name := moveMatchUpperClosedPattern)
  "..=" num : movePrimitiveMatchPattern
syntax (name := moveMatchLowerPattern)
  num ".." : movePrimitiveMatchPattern
syntax (name := moveMatchWildcardPattern) "_" : movePrimitiveMatchPattern

declare_syntax_cat movePrimitiveMatchAlt
syntax (name := movePrimitiveMatchAlternative)
  "| " movePrimitiveMatchPattern (" if " term)? " => " term : movePrimitiveMatchAlt

/-- Primitive Move matches lower to typed comparisons before Lean exposes the
representation of `MoveInt`. The discriminant is evaluated exactly once. -/
scoped syntax (name := movePrimitiveMatch) (priority := high)
  "match " term " with " movePrimitiveMatchAlt+ : term
syntax (name := movePrimitiveMatchElab)
  "__movePrimitiveMatch " term " with " movePrimitiveMatchAlt+ : term
scoped macro:max (name := moveAssert) "assert!(" condition:term ", " code:term ")" : term =>
  `(Move.assert $condition $code)
scoped macro:max (name := moveAssertEq)
    "assert_eq!(" left:term ", " right:term ", " code:term ")" : term =>
  `(Move.assert (Move.Compare.equal $left $right) $code)
scoped macro:max (name := moveAssertNe)
    "assert_ne!(" left:term ", " right:term ", " code:term ")" : term =>
  `(Move.assert (!Move.Compare.equal $left $right) $code)
syntax:max (name := abortTerm) "abort " term:max : term
scoped syntax (name := moveAddAssign) ident " += " term : doElem
scoped syntax (name := moveSubAssign) ident " -= " term : doElem
scoped syntax (name := moveMulAssign) ident " *= " term : doElem
scoped syntax (name := moveDivAssign) ident " /= " term : doElem
scoped syntax (name := moveModAssign) ident " %= " term : doElem
scoped syntax (name := moveNamedStructLet) (priority := high)
  "let " ident "{" ident,* "}" " := " term : doElem
scoped syntax (name := movePositionalStructLet) (priority := high)
  "let " ident "(" ident,* ")" " := " term : doElem
scoped macro_rules (kind := moveNamedStructLet)
  | `(doElem| let $_type:ident { $fields:ident,* } := $value:term) =>
      `(doElem| let ⟨$fields:ident,*⟩ := $value)
scoped macro_rules (kind := movePositionalStructLet)
  | `(doElem| let $_type:ident ( $fields:ident,* ) := $value:term) =>
      `(doElem| let ⟨$fields:ident,*⟩ := $value)
scoped syntax (name := moveForRange)
  "for" "(" ident "in" term "to" term ")" "do" doSeq : doElem

scoped macro_rules (kind := typedIntegerLiteral)
  | stx => do
      let value : TSyntax `num := ⟨stx[0]⟩
      let suffix : Ident := ⟨stx[1]⟩
      let typeName ← match suffix.getId.toString with
        | "u8" => `(Move.U8) | "u16" => `(Move.U16)
        | "u32" => `(Move.U32) | "u64" => `(Move.U64)
        | "u128" => `(Move.U128) | "u256" => `(Move.U256)
        | "i8" => `(Move.I8) | "i16" => `(Move.I16)
        | "i32" => `(Move.I32) | "i64" => `(Move.I64)
        | "i128" => `(Move.I128) | "i256" => `(Move.I256)
        | _ => do
          Macro.throwErrorAt suffix.raw
            "expected a Move integer suffix (`u8`..`u256` or `i8`..`i256`)"
      `(($value : $typeName))

private def primitivePatternCondition (value : TSyntax `term)
    (pattern : TSyntax `movePrimitiveMatchPattern) :
    MacroM (Option (TSyntax `term)) := do
  if pattern.raw.isOfKind ``Move.moveMatchByteStringPattern then
    let literal : TSyntax `term :=
      ⟨Syntax.node .none ``Move.byteStringLiteral
        #[pattern.raw[0], pattern.raw[1]]⟩
    return some (← `(($value == $literal) = true))
  match pattern with
  | `(movePrimitiveMatchPattern| $literal:num) =>
      return some (← `(($value == $literal) = true))
  | `(movePrimitiveMatchPattern| $lower:num .. $upper:num) =>
      return some (← `($lower <= $value ∧ $value < $upper))
  | `(movePrimitiveMatchPattern| $lower:num ..= $upper:num) =>
      return some (← `($lower <= $value ∧ $value <= $upper))
  | `(movePrimitiveMatchPattern| .. $upper:num) =>
      return some (← `($value < $upper))
  | `(movePrimitiveMatchPattern| ..= $upper:num) =>
      return some (← `($value <= $upper))
  | `(movePrimitiveMatchPattern| $lower:num ..) =>
      return some (← `($lower <= $value))
  | `(movePrimitiveMatchPattern| _) => return none
  | _ => Macro.throwUnsupported

private def primitiveMatchBody (reference : Syntax) (value : TSyntax `term)
    (alternatives : Array (TSyntax `movePrimitiveMatchAlt)) : MacroM (TSyntax `term) := do
  let mut result? : Option (TSyntax `term) := none
  for alternative in alternatives.reverse do
    let `(movePrimitiveMatchAlt|
        | $pattern:movePrimitiveMatchPattern $[if $guard:term]? => $rhs:term) := alternative
      | Macro.throwUnsupported
    let condition? ← primitivePatternCondition value pattern
    match condition?, guard, result? with
    | none, none, none => result? := some rhs
    | none, none, some _ =>
        Macro.throwErrorAt alternative "a wildcard match alternative must be last"
    | none, some guard, some fallback =>
        result? := some (← `(if $guard = true then $rhs else $fallback))
    | none, some _, none =>
        Macro.throwErrorAt alternative
          "a guarded wildcard match requires a following fallback alternative"
    | some condition, guard, some fallback =>
        let condition ← match guard with
          | none => pure condition
          | some guard => `($condition ∧ $guard = true)
        result? := some (← `(if $condition then $rhs else $fallback))
    | some _, _, none =>
        Macro.throwErrorAt alternative
          "a primitive match must end with a wildcard fallback alternative"
  let some result := result?
    | Macro.throwErrorAt reference "a primitive match requires at least one alternative"
  pure result

def expandPrimitiveMatchSyntax (reference : Syntax) (discriminant : TSyntax `term)
    (alternatives : Array (TSyntax `movePrimitiveMatchAlt)) : MacroM Syntax := do
  let value := mkIdentFrom reference `_moveMatchValue
  let valueTerm : TSyntax `term := ⟨value.raw⟩
  let result ← primitiveMatchBody reference valueTerm alternatives
  `(let $value := $discriminant; $result)

scoped macro_rules (kind := movePrimitiveMatch)
  | stx => do
      let discriminant : TSyntax `term := ⟨stx[1]⟩
      let alternatives : Array (TSyntax `movePrimitiveMatchAlt) :=
        stx[3].getArgs.map fun alternative => ⟨alternative⟩
      `(__movePrimitiveMatch $discriminant with $alternatives*)

@[term_elab movePrimitiveMatchElab]
def elabPrimitiveMatch : TermElab := fun stx expectedType? => do
  let discriminant : TSyntax `term := ⟨stx[1]⟩
  let alternatives : Array (TSyntax `movePrimitiveMatchAlt) :=
    stx[3].getArgs.map fun alternative => ⟨alternative⟩
  let discriminantExpr ← elabTerm discriminant none
  let discriminantType ← whnf (← inferType discriminantExpr)
  if discriminantType.isAppOfArity ``Ref 1 ||
      discriminantType.isAppOfArity ``MutRef 1 then
    -- Reading a Move reference is represented as an `Action` in Leaner even
    -- though it cannot abort. Keep that sequencing visible to the compiler;
    -- source verification receives the referent value as its logical
    -- parameter and expands the retained syntax through the pure path below.
    let value := mkIdentFrom stx `_moveMatchValue
    let valueTerm : TSyntax `term := ⟨value.raw⟩
    let body ← liftMacroM <| primitiveMatchBody stx valueTerm alternatives
    elabTerm (← `(do
      let $value ← *$discriminant
      pure $body)) expectedType?
  else
    let expanded ← liftMacroM <|
      expandPrimitiveMatchSyntax stx discriminant alternatives
    elabTerm (⟨expanded⟩ : Term) expectedType?

/-- Tail self-call. The callee and arguments must stay on the same line so a
bare `continue` (or `if c then continue`) is not parsed as `continue <next
stmt>` across a newline. -/
syntax:max (name := continueCallTerm)
  withPosition("continue " lineEq term:max (lineEq term:max)*) : term
syntax (name := moveWhileInternal) "__moveWhile " term " do " doSeq : doElem
syntax (name := moveWhileDependentInternal)
  "__moveWhile " ident " : " term " do " doSeq : doElem
syntax (name := moveLoopDo) "loop " doSeq : doElem
syntax (name := moveLoopLabeledDo) "loop@" ident ppSpace doSeq : doElem
syntax (name := moveLoopNestedDo) "__moveLoopNested " ident ppSpace doSeq : doElem
syntax (name := moveLoopLabeledNestedDo)
  "__moveLoopNested@" ident ppSpace ident ppSpace doSeq : doElem
syntax (name := moveBreakLabeledDo) "break@" ident : doElem
syntax (name := moveContinueLabeledDo) "continue@" ident : doElem
syntax (name := moveBreakInternal) "__moveBreak" : doElem
syntax (name := moveContinueInternal) "__moveContinue" : doElem

private def byteTerms (values : Array Nat) : Array (TSyntax `term) :=
  values.map fun value => ⟨Syntax.mkNumLit (toString value)⟩

private def hexDigitValue? : Char → Option Nat
  | '0' => some 0 | '1' => some 1 | '2' => some 2 | '3' => some 3
  | '4' => some 4 | '5' => some 5 | '6' => some 6 | '7' => some 7
  | '8' => some 8 | '9' => some 9
  | 'a' | 'A' => some 10 | 'b' | 'B' => some 11 | 'c' | 'C' => some 12
  | 'd' | 'D' => some 13 | 'e' | 'E' => some 14 | 'f' | 'F' => some 15
  | _ => none

private def hexBytes? : List Char → Option (List Nat)
  | [] => some []
  | high :: low :: rest => do
      let high ← hexDigitValue? high
      let low ← hexDigitValue? low
      return (high * 16 + low) :: (← hexBytes? rest)
  | _ => none

scoped macro_rules (kind := byteStringLiteral)
  | stx => do
      let leader : Ident := ⟨stx[0]⟩
      let value : TSyntax `str := ⟨stx[1]⟩
      if leader.getId == `b then
        let bytes := value.getString.toUTF8.data.map (UInt8.toNat)
        unless bytes.all (· < 128) do
          Macro.throwErrorAt value "Move byte strings accept only ASCII characters"
        let elements := byteTerms bytes
        `(vector![$elements,*])
      else if leader.getId == `x then
        let digits := value.getString.toList
        unless digits.length % 2 == 0 do
          Macro.throwErrorAt value "Move hex strings require an even number of hexadecimal digits"
        let some bytes := hexBytes? digits
          | Macro.throwErrorAt value "Move hex string contains a non-hexadecimal digit"
        let elements := byteTerms bytes.toArray
        `(vector![$elements,*])
      else
        Macro.throwErrorAt leader "expected `b` or `x` before a Move byte string"

/-! Loop state is the set of reassigned bindings that were visible when the
loop was entered. `ControlInfo.reassigns` is intentionally textual, so it
cannot distinguish an entry binding from a same-named local declared in the
loop. Keep this small source-scope analysis here so compilation and source
verification agree on binding identity. -/

private def loopReassignedIdent? (stx : Syntax) : Option Ident :=
  let isReassignment := stx.isOfKind ``Lean.Parser.Term.doReassign ||
    stx.isOfKind ``Lean.Parser.Term.doReassignArrow ||
    stx.isOfKind ``moveAddAssign || stx.isOfKind ``moveSubAssign ||
    stx.isOfKind ``moveMulAssign || stx.isOfKind ``moveDivAssign ||
    stx.isOfKind ``moveModAssign
  if !isReassignment || stx.getNumArgs == 0 then
    none
  else
    let decl := stx[0]
    if decl.isIdent then
      some ⟨decl⟩
    else if decl.getNumArgs > 0 then
      let idNode := decl[0]
      if idNode.isIdent then some ⟨idNode⟩
      else if idNode.getNumArgs > 0 && idNode[0].isIdent then some ⟨idNode[0]⟩
      else none
    else
      none

private partial def firstIdent? (stx : Syntax) : Option Ident :=
  if stx.isIdent then some ⟨stx⟩
  else stx.getArgs.findSome? firstIdent?

private partial def patternIdents (stx : Syntax) : List Ident :=
  if stx.isIdent then
    [⟨stx⟩]
  else
    stx.getArgs.toList.flatMap patternIdents

private def loopBoundDoIdents (stx : Syntax) : List Ident :=
  if !stx.isOfKind ``Lean.Parser.Term.doLet &&
      !stx.isOfKind ``Lean.Parser.Term.doLetArrow then
    []
  else if stx.getNumArgs > 3 then
    let declaration := stx[3]
    if declaration.isOfKind ``Lean.Parser.Term.letPatDecl ||
        declaration.isOfKind ``Lean.Parser.Term.doPatDecl then
      patternIdents declaration[0]!
    else
      (firstIdent? declaration).toList
  else
    []

private def isDoSeqSyntax (stx : Syntax) : Bool :=
  stx.isOfKind ``Lean.Parser.Term.doSeqIndent ||
    stx.isOfKind ``Lean.Parser.Term.doSeqBracketed

mutual
  private partial def collectLoopAssigned (stx : Syntax) (bound : List Name)
      (found : Array Ident) : Array Ident :=
    if isDoSeqSyntax stx then
      collectLoopAssignedSeq ⟨stx⟩ bound found
    else
      let found := match loopReassignedIdent? stx with
        | some id =>
            if id.getId.toString.contains "loopTok" || bound.contains id.getId ||
                found.any (·.getId == id.getId) then found
            else found.push id
        | none => found
      stx.getArgs.foldl (init := found) fun found child =>
        collectLoopAssigned child bound found

  private partial def collectLoopAssignedSeq
      (seq : TSyntax ``Lean.Parser.Term.doSeq)
      (bound : List Name) (found : Array Ident) : Array Ident :=
    let (_, found) := (Lean.Parser.Term.getDoElems seq).foldl
      (init := (bound, found)) fun (bound, found) element =>
        let found := collectLoopAssigned element.raw bound found
        let bound := (loopBoundDoIdents element.raw).map (·.getId) ++ bound
        (bound, found)
    found
end

/-- Reassigned loop-entry bindings, in source order. Bindings introduced by a
`let` in the loop are excluded, including through nested branches and loops. -/
def loopAssignedIdents (body : TSyntax ``Lean.Parser.Term.doSeq) : List Ident :=
  (collectLoopAssignedSeq body [] #[]).toList

private partial def replaceIdent (name : Name) (replacement stx : Syntax) : Syntax :=
  if stx.isIdent && stx.getId == name then replacement
  else stx.setArgs (stx.getArgs.map (replaceIdent name replacement))

private def mkLoopDoSeq (elems : Array Syntax) : Syntax :=
  mkNode ``Lean.Parser.Term.doSeqIndent #[mkNullNode <|
    elems.map fun elem => mkNullNode #[elem, mkNullNode]]

mutual
  private partial def freshenLoopSyntax (renames : List (Name × Ident))
      (stx : Syntax) : CoreM Syntax := do
    if isDoSeqSyntax stx then
      return (← freshenLoopSeq renames ⟨stx⟩).raw
    if stx.isIdent then
      if let some (_, replacement) := renames.find? (·.1 == stx.getId) then
        return replacement.raw
      return stx
    let rewritten := stx.setArgs
      (← stx.getArgs.mapM (freshenLoopSyntax renames))
    if stx.isOfKind ``moveLoopLabeledDo ||
        stx.isOfKind ``moveLoopLabeledNestedDo ||
        stx.isOfKind ``moveBreakLabeledDo ||
        stx.isOfKind ``moveContinueLabeledDo then
      return rewritten.setArgs (rewritten.getArgs.set! 1 stx[1])
    if stx.isOfKind ``Lean.Parser.Term.proj && stx.getNumArgs > 2 then
      return rewritten.setArgs (rewritten.getArgs.set! 2 stx[2])
    return rewritten

  private partial def freshenLoopSeq (renames : List (Name × Ident))
      (seq : TSyntax ``Lean.Parser.Term.doSeq) : CoreM (TSyntax ``Lean.Parser.Term.doSeq) := do
    let (elems, _) ← (Lean.Parser.Term.getDoElems seq).foldlM
      (init := (#[], renames)) fun (elems, renames) element => do
        let rewritten ← freshenLoopSyntax renames element.raw
        match loopBoundDoIdents element.raw with
        | [] => pure (elems.push rewritten, renames)
        | bounds =>
            let freshBindings : List (Name × Ident) ← bounds.mapM fun (bound : Ident) => do
              let fresh := mkIdentFrom bound
                (← mkFreshUserName `loopLocal)
              pure (bound.getId, fresh)
            let declaration := rewritten[3]
            let declaration := freshBindings.foldl (init := declaration)
              fun declaration (bound, fresh) =>
                replaceIdent bound fresh.raw declaration
            let rewritten := rewritten.setArgs
              (rewritten.getArgs.set! 3 declaration)
            pure (elems.push rewritten, freshBindings ++ renames)
    return ⟨mkLoopDoSeq elems⟩
end

/-- Alpha-rename locals declared in a loop body. Besides making their lexical
identity explicit for generated syntax, this prevents a loop-local name from
capturing the elaborated continuation after a `break`. -/
def freshenLoopLocals (body : TSyntax ``Lean.Parser.Term.doSeq) :
    CoreM (TSyntax ``Lean.Parser.Term.doSeq) :=
  freshenLoopSeq [] body

macro_rules (kind := abortTerm)
  | stx => do
      let diagnostic : TSyntax `term := ⟨stx[1]⟩
      if diagnostic.raw.isOfKind ``byteStringLiteral then
        `(Move.abortMessage ($diagnostic : Move.Vector Move.U8))
      else
        `(Move.abort $diagnostic)

macro_rules
  | `(vector![$values:term,*]) => do
      let values := values.getElems.toList
      let mut result ← `(Move.Vector.empty)
      for value in values do
        result ← `(Move.Vector.push $result $value)
      return result

private def expectedIsType (expectedType? : Option Expr) : TermElabM Bool := do
  let some expected ← pure expectedType? | return false
  return (← whnf expected).isSort

private def ownerType? (refType : Expr) (ctor : Name) : MetaM (Option Expr) := do
  let refType ← whnf refType
  if refType.isAppOfArity ctor 1 then
    return some refType.appArg!
  return none

private def projectionName? (owner : Expr) (field : Ident) : MetaM (Option Name) := do
  let .const ownerName _ := owner.getAppFn | return none
  let candidate := ownerName ++ field.getId
  return if (← getEnv).contains candidate then some candidate else none

private def elabFieldBorrow (mutable : Bool) (refStx : Term)
    (field : Ident) (expectedType? : Option Expr) : TermElabM Expr := do
  let refExpr ← elabTerm refStx none
  let refType ← inferType refExpr
  let refType ← whnf refType
  let (owner, refExpr) ←
    if mutable then
      let some owner ← ownerType? refType ``MutRef
        | throwErrorAt refStx "expected a mutable Move reference"
      pure (owner, refExpr)
    else if let some owner ← ownerType? refType ``Ref then
      pure (owner, refExpr)
    else if let some owner ← ownerType? refType ``MutRef then
      pure (owner, ← mkAppM ``freezeRef #[refExpr])
    else
      throwErrorAt refStx "expected a Move reference"
  let some projectionName ← projectionName? owner field
    | throwErrorAt field "`{field.getId}` is not a field of {owner}"
  -- Structure projections carry the structure's type parameters before the
  -- owner argument. Building a lambda lets elaboration infer those parameters
  -- from the concrete referent (`Map K V`, for example) instead of passing the
  -- still-polymorphic projection to `fieldOfProjection`.
  let projection ← withLocalDeclD `_moveFieldOwner owner fun ownerVar => do
    let projected ← mkAppM projectionName #[ownerVar]
    mkLambdaFVars #[ownerVar] projected
  let descriptor ← mkAppM ``fieldOfProjection #[projection]
  let primitive := if mutable then ``borrowFieldMut else ``borrowField
  let result ← mkAppM primitive #[refExpr, descriptor]
  ensureHasType expectedType? result

/-- The base of a dotted place `counter.value` as an identifier spanning its
own characters of the original source.  A synthesized identifier would carry
synthetic source info, which the unused-variable linter does not count as a
use of `counter`; the source span also gives hovers the right target. -/
private def baseIdent (place : Syntax) (baseName : Name) : Ident :=
  match place.getHeadInfo with
  | .original leading pos _ _ =>
      let text := baseName.toString
      let endPos : String.Pos.Raw := ⟨pos.byteIdx + text.utf8ByteSize⟩
      ⟨Syntax.ident (.original leading pos "".toRawSubstring endPos) text.toRawSubstring baseName []⟩
  | _ => mkIdentFrom place baseName

private def fieldParts : Name → List String
  | .anonymous => []
  | .str baseName part => fieldParts baseName ++ [part]
  | .num _ _ => []

/-- Elaborate an argument to a Move function, inserting Move's implicit
`&mut T` to `&T` freeze before ordinary coercion elaboration gets stuck on
generic referent types. -/
private def elabMoveArgument (argument : Term) (expectedType : Expr) : TermElabM Expr := do
  if argument.raw.isIdent then
    let decl? ←
      try
        pure (some (← getLocalDeclFromUserName argument.raw.getId))
      catch _ => pure none
    if let some decl := decl? then
      let actualType ← whnf decl.type
      let expectedType ← whnf expectedType
      if actualType.isAppOfArity ``MutRef 1 && expectedType.isAppOfArity ``Ref 1 then
        unless ← isDefEq actualType.appArg! expectedType.appArg! do
          throwErrorAt argument "mutable reference has incompatible referent type"
        let value ← elabTerm argument (some actualType)
        return ← mkAppM ``freezeRef #[value]
  elabTerm argument (some expectedType)

/-- Move calls infer immutable views of mutable-reference arguments.  Lean's
standard coercion elaborator cannot always infer this conversion when a
generic parameter occurs only under `Ref`, so tagged Move functions use this
small application elaborator. Other applications fall through unchanged. -/
@[term_elab Lean.Parser.Term.app]
def elabMoveFunctionApplication : TermElab := fun stx expectedType? => do
  let `($function:ident $arguments:term*) := stx
    | throwUnsupportedSyntax
  let functionName ←
    try resolveGlobalConstNoOverload function
    catch _ => throwUnsupportedSyntax
  let env ← getEnv
  unless isMoveFunction env functionName do
    throwUnsupportedSyntax
  let functionExpr ← elabTerm function none
  let (parameters, binderInfos, _) ← forallMetaTelescope (← inferType functionExpr)
  let explicitParameters := (parameters.zip binderInfos).filter (·.2.isExplicit)
  unless explicitParameters.size == arguments.size do
    throwUnsupportedSyntax
  for ((parameter, _), argument) in explicitParameters.zip arguments do
    let parameterType ← instantiateMVars (← inferType parameter)
    let value ← elabMoveArgument argument parameterType
    unless ← isDefEq parameter value do
      throwErrorAt argument "argument has type {← inferType value}, expected {parameterType}"
  synthesizeSyntheticMVars
  ensureHasType expectedType? (mkAppN functionExpr parameters)

/-- Turn a chained place into nested `do` expressions.  Each generated borrow
is elaborated normally, so owner and field types are checked at every edge. -/
private partial def chainBorrowSyntax (mutable : Bool) (origin : Syntax)
    (effect : Term) : List String → TermElabM Term
  | [] => pure effect
  | fieldName :: rest => do
      let ref := mkIdentFrom origin (Name.mkSimple "_leanerRef")
      let field := mkIdentFrom origin (Name.mkSimple fieldName)
      let primitive := mkIdent (if mutable then ``borrowFieldMut else ``borrowField)
      let next ← `($primitive $ref (fieldOfProjection (fun owner => owner.$field)))
      let tail ← chainBorrowSyntax mutable origin next rest
      `($effect >>= fun $ref => $tail)

private partial def elabBorrow (mutable : Bool) (place : Term)
    (expectedType? : Option Expr) : TermElabM Expr := do
  if ← expectedIsType expectedType? then
    let ctor := if mutable then ``MutRef else ``Ref
    return ← elabTermEnsuringType (← `($(mkIdent ctor) $place)) expectedType?
  if place.raw.isOfKind ``Lean.Parser.Term.paren then
    return ← elabBorrow mutable ⟨place.raw[1]⟩ expectedType?
  match place with
  | `($base[$index]) =>
      let local? ← if base.raw.isIdent then
        try
          pure (some (← getLocalDeclFromUserName base.raw.getId))
        catch _ => pure none
      else pure none
      match local? with
      | some decl =>
          let type ← whnf decl.type
          if type.isAppOfArity ``Ref 1 then
            let referent ← whnf type.appArg!
            unless referent.isAppOfArity ``Move.Vector 1 do
              throwErrorAt base "indexed borrow expects a Move vector reference"
            if mutable then
              throwErrorAt base "cannot mutably borrow through immutable reference `{base.raw.getId}`"
            elabTerm (← `(borrowElem $base $index)) expectedType?
          else if type.isAppOfArity ``MutRef 1 then
            let referent ← whnf type.appArg!
            unless referent.isAppOfArity ``Move.Vector 1 do
              throwErrorAt base "indexed borrow expects a Move vector reference"
            if mutable then
              elabTerm (← `(borrowElemMut $base $index)) expectedType?
            else
              elabTerm (← `(do
                let _leanerVectorRef ← freeze $base
                borrowElem _leanerVectorRef $index)) expectedType?
          else if type.isAppOfArity ``Move.Vector 1 then
            let localBorrow := if mutable then ``borrowLocalMut else ``borrowLocal
            let elemBorrow := if mutable then ``borrowElemMut else ``borrowElem
            let ref := mkIdentFrom base `_leanerVectorRef
            let localCall ← `($(mkIdent localBorrow) $base)
            let elemCall ← `($(mkIdent elemBorrow) $ref $index)
            elabTerm (← `($localCall >>= fun $ref => $elemCall)) expectedType?
          else
            throwErrorAt base "indexed borrow expects a Move vector or vector reference"
      | none =>
          let primitive := if mutable then ``borrowGlobalMut else ``borrowGlobal
          elabTerm (← `($(mkIdent primitive) $base $index)) expectedType?
  | _ =>
      if place.raw.isIdent then
        try
          let decl ← getLocalDeclFromUserName place.raw.getId
          let type ← whnf decl.type
          -- An identifier in value position is a local borrow. Reference type
          -- notation was already handled above through `expectedIsType`.
          unless type.isSort do
            let primitive := if mutable then ``borrowLocalMut else ``borrowLocal
            return ← elabTerm (← `($(mkIdent primitive) $place)) expectedType?
        catch _ => pure ()
      if place.raw.isOfKind ``Lean.Parser.Term.proj then
        let args := place.raw.getArgs
        if let some field := args[2]? then
          if field.isIdent then
            let base : Term := ⟨args[0]!⟩
            let root ← if mutable then `(&mut $base) else `(& $base)
            let chained ← chainBorrowSyntax mutable place.raw root (fieldParts field.getId)
            return ← elabTerm chained expectedType?
      if place.raw.isIdent then
        match place.raw.getId with
        | .str baseName fieldName =>
            let ref := baseIdent place baseName
            let field := mkIdentFrom place (Name.mkSimple fieldName)
            return ← elabFieldBorrow mutable ref field expectedType?
        | _ => pure ()
      -- With no expected type (for example under `#check`), the remaining
      -- unambiguous form is a reference type.
      let ctor := if mutable then ``MutRef else ``Ref
      elabTerm (← `($(mkIdent ctor) $place)) expectedType?

@[term_elab borrowTerm]
def elabBorrowTerm : TermElab := fun stx expectedType? => do
  let `(& $place) := stx | throwUnsupportedSyntax
  elabBorrow false place expectedType?

@[term_elab borrowMutTerm]
def elabBorrowMutTerm : TermElab := fun stx expectedType? => do
  let `(&mut $place) := stx | throwUnsupportedSyntax
  elabBorrow true place expectedType?

@[term_elab borrowIndexTerm]
def elabBorrowIndexTerm : TermElab := fun stx expectedType? => do
  let `(& $base:ident[$index]) := stx | throwUnsupportedSyntax
  elabBorrow false (← `($base[$index])) expectedType?

@[term_elab borrowMutIndexTerm]
def elabBorrowMutIndexTerm : TermElab := fun stx expectedType? => do
  let `(&mut $base:ident[$index]) := stx | throwUnsupportedSyntax
  elabBorrow true (← `($base[$index])) expectedType?

/-- Read through either kind of Move reference.  The operation remains in
`Action`, so it composes with native `do` notation as `let x ← *ref`. -/
@[term_elab derefTerm]
def elabDerefTerm : TermElab := fun stx expectedType? => do
  let `(* $ref) := stx | throwUnsupportedSyntax
  let refExpr ← elabTerm ref none
  let type ← whnf (← inferType refExpr)
  let primitive ←
    if type.isAppOfArity ``MutRef 1 then pure ``read
    else if type.isAppOfArity ``Ref 1 then pure ``readImm
    else throwErrorAt ref "expected a Move reference after `*`"
  ensureHasType expectedType? (← mkAppM primitive #[refExpr])

open Lean.Elab.Do Lean.Parser.Term in
@[doElem_elab Lean.Parser.Term.doReassign]
def elabMoveReassign : DoElab := fun stx cont => do
  match stx with
  | `(doReassign| $x:ident $[: $_]? :=%$_ $rhs) =>
      let decl ← getLocalDeclFromUserName x.getId
      let type ← whnf decl.type
      if type.isAppOfArity ``MutRef 1 then
        match rhs with
        | `(* $ref - $amount) =>
            let old := mkIdentFrom rhs `_leanerOld
            elabDoElems1
              #[(← `(doElem| let $old ← read $ref)),
                (← `(doElem| write $x ($old - $amount)))] cont
        | `(* $ref + $amount) =>
            let old := mkIdentFrom rhs `_leanerOld
            elabDoElems1
              #[(← `(doElem| let $old ← read $ref)),
                (← `(doElem| write $x ($old + $amount)))] cont
        | _ => elabDoElem (← `(doElem| write $x $rhs)) cont
      else if type.isAppOfArity ``Ref 1 then
        throwErrorAt x "cannot write through immutable reference `{x.getId}`"
      else
        throwUnsupportedSyntax
  | _ => throwUnsupportedSyntax

open Lean.Elab.Do Lean.Parser.Term in
private def elabMoveCompoundAssign (operation : Name) : DoElab := fun stx cont => do
  let x : Ident := ⟨stx.raw[0]⟩
  let rhs : TSyntax `term := ⟨stx.raw[2]⟩
  let decl ← getLocalDeclFromUserName x.getId
  let type ← whnf decl.type
  let combine (left : TSyntax `term) : TermElabM (TSyntax `term) :=
    if operation == `add then `($left + $rhs)
    else if operation == `sub then `($left - $rhs)
    else if operation == `mul then `($left * $rhs)
    else if operation == `div then `($left / $rhs)
    else `($left % $rhs)
  if type.isAppOfArity ``MutRef 1 then
    let old := mkIdentFrom rhs `_leanerOld
    let value ← combine ⟨old.raw⟩
    elabDoElems1
      #[(← `(doElem| let $old ← read $x)),
        (← `(doElem| write $x $value))] cont
  else if type.isAppOfArity ``Ref 1 then
    throwErrorAt x "cannot write through immutable reference `{x.getId}`"
  else
    let value ← combine ⟨x.raw⟩
    elabDoElem (← `(doElem| $x:ident := $value)) cont

open Lean.Elab.Do in
@[doElem_elab moveAddAssign]
def elabMoveAddAssign : DoElab := elabMoveCompoundAssign `add

open Lean.Elab.Do in
@[doElem_elab moveSubAssign]
def elabMoveSubAssign : DoElab := elabMoveCompoundAssign `sub

open Lean.Elab.Do in
@[doElem_elab moveMulAssign]
def elabMoveMulAssign : DoElab := elabMoveCompoundAssign `mul

open Lean.Elab.Do in
@[doElem_elab moveDivAssign]
def elabMoveDivAssign : DoElab := elabMoveCompoundAssign `div

open Lean.Elab.Do in
@[doElem_elab moveModAssign]
def elabMoveModAssign : DoElab := elabMoveCompoundAssign `mod

open Lean.Elab.Do in
private def controlInfoMoveCompound : ControlInfoHandler := fun stx => do
  let name : Ident := ⟨stx.raw[0]⟩
  return { reassigns := NameSet.empty.insert name.getId }

open Lean.Elab.Do in
@[doElem_control_info moveAddAssign]
def controlInfoMoveAddAssign : ControlInfoHandler := controlInfoMoveCompound

open Lean.Elab.Do in
@[doElem_control_info moveSubAssign]
def controlInfoMoveSubAssign : ControlInfoHandler := controlInfoMoveCompound

open Lean.Elab.Do in
@[doElem_control_info moveMulAssign]
def controlInfoMoveMulAssign : ControlInfoHandler := controlInfoMoveCompound

open Lean.Elab.Do in
@[doElem_control_info moveDivAssign]
def controlInfoMoveDivAssign : ControlInfoHandler := controlInfoMoveCompound

open Lean.Elab.Do in
@[doElem_control_info moveModAssign]
def controlInfoMoveModAssign : ControlInfoHandler := controlInfoMoveCompound

private structure LoopFrame where
  label : Nat
  sourceLabel? : Option Name
  parentToken? : Option Ident
  controlTokens : List Ident
  token : Ident
  state : List Ident

private initialize loopLabelState : IO.Ref (List LoopFrame × Nat) ←
  IO.mkRef ([], 0)

private initialize loopMarkerNonce : IO.Ref Nat ← IO.mkRef 0

private def pushLoopFrame (ref : Syntax) (sourceLabel? : Option Name)
    (parentToken? : Option Ident)
    (controlTokens : List Ident)
    (state : List Ident) : TermElabM LoopFrame := do
  let (stack, next) ← loopLabelState.get
  if let some sourceLabel := sourceLabel? then
    if stack.any (·.sourceLabel? == some sourceLabel) then
      throwError "duplicate active loop label `{sourceLabel}`"
  let label := (ref.getPos?.map (·.byteIdx + 1)).getD next
  let token := mkIdent (← mkFreshUserName `loopTok)
  let frame := { label, sourceLabel?, parentToken?, controlTokens, token, state }
  loopLabelState.set (frame :: stack, next + 1)
  return frame

private def popLoopLabel : TermElabM Unit := do
  let (stack, next) ← loopLabelState.get
  match stack with
  | _ :: rest => loopLabelState.set (rest, next)
  | [] => loopLabelState.set ([], next)

private def currentLoopFrame? : TermElabM (Option LoopFrame) := do
  return (← loopLabelState.get).1.head?

private def labeledLoopFrame? (sourceLabel : Name) :
    TermElabM (Option LoopFrame) := do
  return (← loopLabelState.get).1.find? (·.sourceLabel? == some sourceLabel)

private def currentLoopLabel? : TermElabM (Option Nat) := do
  return (← currentLoopFrame?).map (·.label)

private def freshLoopMarkerNonce (marker : Name) (label : Nat) : TermElabM Nat := do
  let nonce ← loopMarkerNonce.get
  loopMarkerNonce.set (nonce + 1)
  if let some function ← getDeclName? then
    registerLoopMarker function marker label nonce
  return nonce

private def authoredLoopState
    (body : TSyntax ``Lean.Parser.Term.doSeq) : List Ident :=
  loopAssignedIdents body

private def outerControlTokens (info : Lean.Elab.Do.ControlInfo) : List Ident :=
  info.reassigns.toList
    |>.filter (fun name => name.toString.contains "loopTok")
    |>.map mkIdent

private def scopeLoopControlInfo (body : TSyntax ``Lean.Parser.Term.doSeq)
    (info : Lean.Elab.Do.ControlInfo) : Lean.Elab.Do.ControlInfo :=
  let authored := (authoredLoopState body).foldl (init := NameSet.empty)
    fun names id => names.insert id.getId
  let controls := (outerControlTokens info).foldl (init := authored)
    fun names id => names.insert id.getId
  { info with reassigns := controls }

@[term_elab continueCallTerm]
def elabContinueCall : TermElab := fun stx expectedType? => do
  if (← currentLoopLabel?).isSome then
    throwErrorAt stx "`continue f` cannot appear inside `loop` / `while`; use `continue`"
  let `(continue $fn:term $args:term*) := stx | throwUnsupportedSyntax
  elabTerm (← `(Move.continueMarker ($fn $args*))) expectedType?

open Lean.Elab.Do Lean.Parser.Term in
private partial def packLoopState : List Ident → TermElabM (TSyntax `term)
  | [] => `(())
  | [id] => `($id)
  | id :: rest => do
      let tail ← packLoopState rest
      `(($id, $tail))

open Lean.Elab.Do Lean.Parser.Term in
private def loopEnterDoElem (frame : LoopFrame) : TermElabM DoElem := do
  let labelTerm : TSyntax `term := quote frame.label
  let nonceTerm : TSyntax `term := quote (← freshLoopMarkerNonce ``loopEnter frame.label)
  let arityTerm : TSyntax `term := quote frame.state.length
  let state ← packLoopState frame.state
  let seed : TSyntax `term := match frame.parentToken? with
    | some parent => ⟨parent.raw⟩
    | none => quote 0
  `(doElem| let mut $(frame.token) :=
      loopEnter $labelTerm $nonceTerm $arityTerm $seed $state)

open Lean.Elab.Do Lean.Parser.Term in
private def loopExitDoElem (marker : Name) (frame : LoopFrame) : TermElabM DoElem := do
  let labelTerm : TSyntax `term := quote frame.label
  let nonceTerm : TSyntax `term := quote (← freshLoopMarkerNonce marker frame.label)
  let arityTerm : TSyntax `term := quote frame.state.length
  let state ← packLoopState frame.state
  let markerIdent := mkIdent marker
  let token := frame.token
  let markerToken ←
    if marker == ``loopContinueTail then
      frame.controlTokens.foldlM (init := (⟨token.raw⟩ : TSyntax `term))
        fun current (outer : Ident) =>
          let outerTerm : TSyntax `term := ⟨outer.raw⟩
          `(loopTokenJoin $current $outerTerm)
    else
      pure (⟨token.raw⟩ : TSyntax `term)
  `(doElem| $token:ident :=
      $markerIdent $labelTerm $nonceTerm $arityTerm $markerToken $state)

open Lean.Elab.Do Lean.Parser.Term Lean.Parser.Term.InternalSyntax in
private def loopLiveDoElem (frame : LoopFrame) : TermElabM DoElem :=
  let token := frame.token
  `(doElem| if loopTokenLive $token then skip else return Inhabited.default)

private partial def rewriteLoopControls (frame : LoopFrame)
    (rewriteBare : Bool) (stx : Syntax) : TermElabM Syntax := do
  let kind := stx.getKind
  if kind == ``moveLoopDo then
    let rewritten := stx.setArgs
      (← stx.getArgs.mapM (rewriteLoopControls frame false))
    return rewritten.setKind ``moveLoopNestedDo |>.setArgs
      #[rewritten[0], frame.token.raw, rewritten[1]]
  else if kind == ``moveLoopLabeledDo then
    if frame.sourceLabel? == some stx[1].getId then
      throwErrorAt stx "duplicate active loop label `{stx[1].getId}`"
    let rewritten := stx.setArgs
      (← stx.getArgs.mapM (rewriteLoopControls frame false))
    return rewritten.setKind ``moveLoopLabeledNestedDo |>.setArgs
      #[rewritten[0], rewritten[1], frame.token.raw, rewritten[2]]
  else if kind == ``moveWhileInternal || kind == ``moveWhileDependentInternal ||
      kind == ``Lean.Parser.Term.doWhile then
    return stx.setArgs (← stx.getArgs.mapM (rewriteLoopControls frame false))
  else if rewriteBare && kind == ``Lean.Parser.Term.doBreak then
    return (← loopExitDoElem ``loopBreak frame).raw
  else if rewriteBare && kind == ``Lean.Parser.Term.doContinue then
    return (← loopExitDoElem ``loopContinue frame).raw
  else if kind == ``moveBreakLabeledDo &&
      frame.sourceLabel? == some stx[1].getId then
    return (← loopExitDoElem ``loopBreak frame).raw
  else if kind == ``moveContinueLabeledDo &&
      frame.sourceLabel? == some stx[1].getId then
    return (← loopExitDoElem ``loopContinue frame).raw
  else
    return stx.setArgs (← stx.getArgs.mapM (rewriteLoopControls frame rewriteBare))

open Lean.Parser.Term in
def mkDoSeq (elems : Array Syntax) : TSyntax ``doSeq :=
  let items := elems.map fun elem => mkNode ``doSeqItem #[elem, mkNullNode]
  ⟨mkNode ``doSeqIndent #[mkNullNode items]⟩

/- Half-open Move range loop.  A private mutable counter drives the existing
`while` lowering while the authored index is rebound immutably each turn. -/
open Lean.Elab.Do Lean.Parser.Term in
@[doElem_elab moveForRange]
def elabMoveForRange : DoElab := fun stx cont => do
  let index : Ident := ⟨stx.raw[2]⟩
  let lower : TSyntax `term := ⟨stx.raw[4]⟩
  let upper : TSyntax `term := ⟨stx.raw[6]⟩
  let body : TSyntax ``doSeq := ⟨stx.raw[9]⟩
  let counter := mkIdentFrom index (← mkFreshUserName `_moveForIndex)
  let bodyElems := getDoElems body |>.map (fun element => element.raw)
  let bindIndex := (← `(doElem| let $index := $counter)).raw
  let increment := (← `(doElem| $counter:ident := $counter + 1)).raw
  let loopBody := mkDoSeq ((#[bindIndex] ++ bodyElems).push increment)
  elabDoElems1 #[(← `(doElem| let mut $counter := $lower)),
    (← `(doElem| while $counter < $upper do $loopBody))] cont

open Lean.Parser.Term in
@[macro Lean.Parser.Term.doWhile]
def expandMoveWhile : Macro := fun stx =>
  match stx with
  | `(doElem| while $cond:doIfCond do $body:doSeq) =>
      match cond with
      | `(doIfCond| $term:term) =>
          let rewritten : TSyntax `doElem := ⟨mkNode ``moveWhileInternal
            #[mkAtom "__moveWhile ", term.raw, mkAtom "do ", body.raw]⟩
          pure rewritten
      | `(doIfCond| $binder:ident : $term:term) =>
          let rewritten : TSyntax `doElem := ⟨mkNode ``moveWhileDependentInternal
            #[mkAtom "__moveWhile ", binder.raw, mkAtom " : ", term.raw,
              mkAtom "do ", body.raw]⟩
          pure rewritten
      | _ => Macro.throwErrorAt cond "unsupported `while` condition"
  | _ => Macro.throwUnsupported

open Lean.Elab.Do Lean.Parser.Term in
private def elabMoveWhileCore : DoElab := fun stx cont => do
  let (cond, body) ←
    if stx.raw.isOfKind ``moveWhileInternal then
      let term : TSyntax `term := ⟨stx.raw[1]!⟩
      pure (← `(doIfCond| $term:term), (⟨stx.raw[3]!⟩ : TSyntax ``doSeq))
    else if stx.raw.isOfKind ``moveWhileDependentInternal then
      let binder : TSyntax `ident := ⟨stx.raw[1]!⟩
      let term : TSyntax `term := ⟨stx.raw[3]!⟩
      pure (← `(doIfCond| $binder:ident : $term:term),
        (⟨stx.raw[5]!⟩ : TSyntax ``doSeq))
    else
      throwUnsupportedSyntax
  let body ← freshenLoopLocals body
  let info ← InferControlInfo.ofSeq body
  let frame ← pushLoopFrame cond.raw none none (outerControlTokens info)
    (authoredLoopState body)
  try
    let continueElem ← loopExitDoElem ``loopContinueTail frame
    let breakElem ← loopExitDoElem ``loopBreak frame
    let enterElem ← loopEnterDoElem frame
    let liveElem ← loopLiveDoElem frame
    let rewrittenBody : TSyntax ``doSeq :=
      ⟨← rewriteLoopControls frame true body.raw⟩
    let bodyElems := getDoElems rewrittenBody
    let thenSeq := mkDoSeq (bodyElems.map (·.raw) |>.push continueElem.raw)
    let elseSeq := mkDoSeq #[breakElem.raw]
    elabDoElems1 #[
      enterElem,
      (← `(doElem| if $cond:doIfCond then $thenSeq else $elseSeq)),
      liveElem
    ] cont
  finally
    popLoopLabel

open Lean.Elab.Do in
@[doElem_elab moveWhileInternal]
def elabMoveWhile : DoElab := elabMoveWhileCore

open Lean.Elab.Do in
@[doElem_elab moveWhileDependentInternal]
def elabMoveWhileDependent : DoElab := elabMoveWhileCore

open Lean.Elab.Do in
private def controlInfoMoveWhileCore : ControlInfoHandler := fun stx => do
  let body : TSyntax ``Lean.Parser.Term.doSeq :=
    if stx.raw.isOfKind ``moveWhileInternal then ⟨stx.raw[3]!⟩
    else ⟨stx.raw[5]!⟩
  let info := scopeLoopControlInfo body (← InferControlInfo.ofSeq body)
  return { info with
    numRegularExits := 1
    continues := false
    breaks := false
    noFallthrough := false }

open Lean.Elab.Do in
@[doElem_control_info moveWhileInternal]
def controlInfoMoveWhile : ControlInfoHandler := controlInfoMoveWhileCore

open Lean.Elab.Do in
@[doElem_control_info moveWhileDependentInternal]
def controlInfoMoveWhileDependent : ControlInfoHandler := controlInfoMoveWhileCore

open Lean.Elab.Do in
@[doElem_control_info moveLoopDo]
def controlInfoMoveLoop : ControlInfoHandler := fun stx => do
  let body : TSyntax ``Lean.Parser.Term.doSeq := ⟨stx.raw[1]!⟩
  let info := scopeLoopControlInfo body (← InferControlInfo.ofSeq body)
  return { info with
    numRegularExits := 1
    continues := false
    breaks := false
    noFallthrough := false }

open Lean.Elab.Do in
@[doElem_control_info moveLoopLabeledDo]
def controlInfoMoveLoopLabeled : ControlInfoHandler := fun stx => do
  let body : TSyntax ``Lean.Parser.Term.doSeq := ⟨stx.raw[2]!⟩
  let info := scopeLoopControlInfo body (← InferControlInfo.ofSeq body)
  return { info with
    numRegularExits := 1
    continues := false
    breaks := false
    noFallthrough := false }

open Lean.Elab.Do in
@[doElem_control_info moveLoopNestedDo]
def controlInfoMoveLoopNested : ControlInfoHandler := fun stx => do
  let body : TSyntax ``Lean.Parser.Term.doSeq := ⟨stx.raw[2]!⟩
  let info := scopeLoopControlInfo body (← InferControlInfo.ofSeq body)
  return { info with
    numRegularExits := 1
    continues := false
    breaks := false
    noFallthrough := false }

open Lean.Elab.Do in
@[doElem_control_info moveLoopLabeledNestedDo]
def controlInfoMoveLoopLabeledNested : ControlInfoHandler := fun stx => do
  let body : TSyntax ``Lean.Parser.Term.doSeq := ⟨stx.raw[3]!⟩
  let info := scopeLoopControlInfo body (← InferControlInfo.ofSeq body)
  return { info with
    numRegularExits := 1
    continues := false
    breaks := false
    noFallthrough := false }

open Lean.Elab.Do in
@[doElem_control_info moveBreakLabeledDo]
def controlInfoMoveBreakLabeled : ControlInfoHandler := fun stx => do
  let sourceLabel := stx.raw[1]!.getId
  if let some frame ← labeledLoopFrame? sourceLabel then
    return { reassigns := NameSet.empty.insert frame.token.getId }
  return {
    breaks := true
    numRegularExits := 0
    noFallthrough := true }

open Lean.Elab.Do in
@[doElem_control_info moveContinueLabeledDo]
def controlInfoMoveContinueLabeled : ControlInfoHandler := fun stx => do
  let sourceLabel := stx.raw[1]!.getId
  if let some frame ← labeledLoopFrame? sourceLabel then
    return { reassigns := NameSet.empty.insert frame.token.getId }
  return {
    continues := true
    numRegularExits := 0
    noFallthrough := true }

open Lean.Elab.Do in
@[doElem_control_info moveBreakInternal]
def controlInfoMoveBreakInternal : ControlInfoHandler := fun _ => do
  let some frame ← currentLoopFrame?
    | throwUnsupportedSyntax
  return { reassigns := NameSet.empty.insert frame.token.getId }

open Lean.Elab.Do in
@[doElem_control_info moveContinueInternal]
def controlInfoMoveContinueInternal : ControlInfoHandler := fun _ => do
  let some frame ← currentLoopFrame?
    | throwUnsupportedSyntax
  return { reassigns := NameSet.empty.insert frame.token.getId }

open Lean.Elab.Do Lean.Parser.Term in
@[doElem_elab moveLoopDo]
def elabMoveLoop : DoElab := fun stx cont => do
  unless stx.raw.isOfKind ``moveLoopDo do throwUnsupportedSyntax
  let body : TSyntax ``doSeq := ⟨stx.raw[1]!⟩
  let body ← freshenLoopLocals body
  let info ← InferControlInfo.ofSeq body
  let frame ← pushLoopFrame stx.raw none none (outerControlTokens info)
    (authoredLoopState body)
  try
    let enterElem ← loopEnterDoElem frame
    let continueElem ← loopExitDoElem ``loopContinueTail frame
    let liveElem ← loopLiveDoElem frame
    let rewrittenBody : TSyntax ``doSeq :=
      ⟨← rewriteLoopControls frame true body.raw⟩
    let bodyElems := getDoElems rewrittenBody
    let elems := #[enterElem] ++ bodyElems.map (⟨·⟩) ++ #[continueElem, liveElem]
    elabDoElems1 elems cont
  finally
    popLoopLabel

open Lean.Elab.Do Lean.Parser.Term in
@[doElem_elab moveLoopLabeledDo]
def elabMoveLoopLabeled : DoElab := fun stx cont => do
  unless stx.raw.isOfKind ``moveLoopLabeledDo do throwUnsupportedSyntax
  let sourceLabel := stx.raw[1]!.getId
  let body : TSyntax ``doSeq := ⟨stx.raw[2]!⟩
  let body ← freshenLoopLocals body
  let info ← InferControlInfo.ofSeq body
  let frame ← pushLoopFrame stx.raw (some sourceLabel) none
    (outerControlTokens info) (authoredLoopState body)
  try
    let enterElem ← loopEnterDoElem frame
    let continueElem ← loopExitDoElem ``loopContinueTail frame
    let liveElem ← loopLiveDoElem frame
    let rewrittenBody : TSyntax ``doSeq :=
      ⟨← rewriteLoopControls frame true body.raw⟩
    let bodyElems := getDoElems rewrittenBody
    let elems := #[enterElem] ++ bodyElems.map (⟨·⟩) ++ #[continueElem, liveElem]
    elabDoElems1 elems cont
  finally
    popLoopLabel

open Lean.Elab.Do Lean.Parser.Term in
@[doElem_elab moveLoopNestedDo]
def elabMoveLoopNested : DoElab := fun stx cont => do
  unless stx.raw.isOfKind ``moveLoopNestedDo do throwUnsupportedSyntax
  let parentToken : Ident := ⟨stx.raw[1]!⟩
  let body : TSyntax ``doSeq := ⟨stx.raw[2]!⟩
  let body ← freshenLoopLocals body
  let info ← InferControlInfo.ofSeq body
  let frame ← pushLoopFrame stx.raw none (some parentToken)
    (outerControlTokens info) (authoredLoopState body)
  try
    let enterElem ← loopEnterDoElem frame
    let continueElem ← loopExitDoElem ``loopContinueTail frame
    let liveElem ← loopLiveDoElem frame
    let rewrittenBody : TSyntax ``doSeq :=
      ⟨← rewriteLoopControls frame true body.raw⟩
    let bodyElems := getDoElems rewrittenBody
    let elems := #[enterElem] ++ bodyElems.map (⟨·⟩) ++ #[continueElem, liveElem]
    elabDoElems1 elems cont
  finally
    popLoopLabel

open Lean.Elab.Do Lean.Parser.Term in
@[doElem_elab moveLoopLabeledNestedDo]
def elabMoveLoopLabeledNested : DoElab := fun stx cont => do
  unless stx.raw.isOfKind ``moveLoopLabeledNestedDo do throwUnsupportedSyntax
  let sourceLabel := stx.raw[1]!.getId
  let parentToken : Ident := ⟨stx.raw[2]!⟩
  let body : TSyntax ``doSeq := ⟨stx.raw[3]!⟩
  let body ← freshenLoopLocals body
  let info ← InferControlInfo.ofSeq body
  let frame ← pushLoopFrame stx.raw (some sourceLabel) (some parentToken)
    (outerControlTokens info) (authoredLoopState body)
  try
    let enterElem ← loopEnterDoElem frame
    let continueElem ← loopExitDoElem ``loopContinueTail frame
    let liveElem ← loopLiveDoElem frame
    let rewrittenBody : TSyntax ``doSeq :=
      ⟨← rewriteLoopControls frame true body.raw⟩
    let bodyElems := getDoElems rewrittenBody
    let elems := #[enterElem] ++ bodyElems.map (⟨·⟩) ++ #[continueElem, liveElem]
    elabDoElems1 elems cont
  finally
    popLoopLabel

open Lean.Elab.Do Lean.Parser.Term in
@[doElem_elab moveBreakLabeledDo]
def elabMoveBreakLabeled : DoElab := fun stx cont => do
  let sourceLabel := stx.raw[1]!.getId
  let some frame ← labeledLoopFrame? sourceLabel
    | throwErrorAt stx "unknown loop label `{sourceLabel}`"
  elabDoElem (← loopExitDoElem ``loopBreak frame) cont

open Lean.Elab.Do Lean.Parser.Term in
@[doElem_elab moveContinueLabeledDo]
def elabMoveContinueLabeled : DoElab := fun stx cont => do
  let sourceLabel := stx.raw[1]!.getId
  let some frame ← labeledLoopFrame? sourceLabel
    | throwErrorAt stx "unknown loop label `{sourceLabel}`"
  elabDoElem (← loopExitDoElem ``loopContinue frame) cont

open Lean.Elab.Do Lean.Parser.Term in
@[doElem_elab moveBreakInternal]
def elabMoveBreakInternal : DoElab := fun _stx cont => do
  let some frame ← currentLoopFrame?
    | throwUnsupportedSyntax
  elabDoElem (← loopExitDoElem ``loopBreak frame) cont

open Lean.Elab.Do Lean.Parser.Term in
@[doElem_elab moveContinueInternal]
def elabMoveContinueInternal : DoElab := fun _stx cont => do
  let some frame ← currentLoopFrame?
    | throwUnsupportedSyntax
  elabDoElem (← loopExitDoElem ``loopContinue frame) cont

open Lean.Elab.Do Lean.Parser.Term in
@[doElem_elab Lean.Parser.Term.doBreak]
def elabMoveBreak : DoElab := fun _stx cont => do
  let some frame ← currentLoopFrame?
    | throwUnsupportedSyntax
  elabDoElem (← loopExitDoElem ``loopBreak frame) cont

open Lean.Elab.Do Lean.Parser.Term in
@[doElem_elab Lean.Parser.Term.doContinue]
def elabMoveContinue : DoElab := fun _stx cont => do
  let some frame ← currentLoopFrame?
    | throwUnsupportedSyntax
  elabDoElem (← loopExitDoElem ``loopContinue frame) cont

end Move
