-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.Term
import LeanerIR.Validation.IndexedArena
import LeanerIR.Validation.InstantiationCertificate

/-!
# Compilation of validated LIR to typed terms

`compileFunction` is a Lean function of the validated unit: it reads a
function's body out of the expression arena and produces the typed term
that denotes it, or names the construct the denotation does not carry.
It is structural recursion on a fuel, so the kernel unfolds it on a quoted
unit and a per-target certificate is a `rfl`.

Everything it decides is decided on identities and indexes — never on
strings — so that reduction stays cheap.  A rejected construct is a
negative check naming it, never an approximation.
-/

namespace LeanerIR.Proofs.Denote

open LeanerIR.Validation LeanerIR.SemanticOperations

variable {unit : ValidatedUnit}

/-- The construct a node uses, for a rejection message. -/
private def describeKind : ExprKind → String
  | .value .. => "value"
  | .constant _ => "constant"
  | .localVar _ => "local"
  | .operation operation _ _ _ =>
      match operation with
      | .move _ => "move of a place"
      | .copy _ => "copy of a place"
      | .borrow .mutable _ => "mutable borrow"
      | .borrow _ _ => "borrow"
      | .read _ => "read of a place"
      | .write _ => "write of a place"
      | .call _ => "call"
      | .global _ => "global storage operation"
      | .primitive primitive => s!"primitive {repr primitive}"
      | .reference _ => "reference operation"
      | .data _ => "data operation"
      | .specification _ => "specification operation"
      | .assert => "assert"
      | .drop _ => "drop"
      | .profile .. => "profile operation"
  | .block .. => "block"
  | .letDecl .. => "let"
  | .ifElse .. => "if"
  | .match_ .. => "match"
  | .loop .. => "loop"
  | .break_ _ (some _) => "break with a value"
  | .break_ .. => "break"
  | .continue_ _ => "continue"
  | .return_ _ => "return"
  | .throw_ .. => "throw"
  | .assign .. => "assignment"
  | .assignPattern .. => "pattern assignment"
  | .quantifier .. => "quantifier"
  | .spec _ => "specification block"

/-- The site naming a loop: its expression paired injectively with its
namespace, so that the loops of callees from several namespaces inlined
into one proof keep distinct sites. -/
def loopSite (namespaceIndex expressionIndex : Nat) : Nat :=
  (namespaceIndex + expressionIndex) * (namespaceIndex + expressionIndex + 1) / 2 +
    expressionIndex

/-- The rejection of a construct. -/
def notCarried (what : String) : Except String α :=
  .error s!"{what} is not carried by the denotation"

/-- A namespace as a compilation reads it. The kernel evaluates a
compilation and reads a plain array in time linear in the index, so the
arenas and the type table are viewed as balanced trees, built once per
compiled function, and each lookup is logarithmic. -/
structure CompileNamespace where
  source : ValidatedNamespace
  namespaceId : NamespaceId
  expressions : IndexedArena Expr
  places : IndexedArena Place
  patterns : IndexedArena Pattern
  types : IndexedArena Ty
  /-- The loan deaths at each anchor, and the places dereferencing a shared
  reference, as the borrow certificates record them. -/
  deaths : KeyTree AnchorDeaths
  shared : KeyTree Unit
  /-- The unit's struct and enum declarations stating a data invariant or
  holding an intrinsic map, keyed by `loopSite` of namespace and index. -/
  invariants : KeyTree Unit
  /-- The unit's type and expression fuels, and the namespace's place count,
  counted once. -/
  typeFuel : Nat
  fuel : Nat
  placeFuel : Nat

/-- Whether a declaration of a namespace carries a data invariant: a stated
one, or the order of an intrinsic map's keys. -/
def declarationCarriesInvariant (ns : ValidatedNamespace) (declaration : StructDecl) : Bool :=
  declaration.contract.conditions.any (·.kind == .structInvariant) ||
    ns.intrinsics.any fun intrinsic =>
      intrinsic.model == "map" && intrinsic.owner == declaration.name

/-- The unit's struct and enum declarations stating a data invariant or
holding an intrinsic map, keyed by `loopSite` of namespace and index: a
value of a type carrying one is checked where it is constructed and where a
mutation of it ends. -/
def invariantIndex (unit : ValidatedUnit) : KeyTree Unit :=
  let entries := unit.namespaces.toList.zipIdx.foldl (init := []) fun entries (ns, nsIndex) =>
    ns.structs.toList.zipIdx.foldl (init := entries) fun entries (declaration, index) =>
      if declarationCarriesInvariant ns declaration then (loopSite nsIndex index, ()) :: entries
      else entries
  KeyTree.ofSorted (sortByIndex entries)

/-- Whether any declaration of a unit carries a data invariant. -/
def unitCarriesInvariants (unit : ValidatedUnit) : Bool :=
  unit.namespaces.any fun ns => ns.structs.any (declarationCarriesInvariant ns)

def CompileNamespace.of (unit : ValidatedUnit) (namespaceId : NamespaceId)
    (ns : ValidatedNamespace) : CompileNamespace :=
  ⟨ns, namespaceId, .ofArray ns.expressions, .ofArray ns.places, .ofArray ns.patterns,
    .ofArray ns.tables.types, deathIndex unit namespaceId, sharedDereferenceIndex unit namespaceId,
    invariantIndex unit, LeanerIR.Proofs.Denote.typeFuel unit, unitFuel unit, ns.places.size + 1⟩

/-- Whether a value of a type carries a data invariant at any depth. -/
def CompileNamespace.carriesInvariant (ns : CompileNamespace) (τ : NTy) : Bool :=
  τ.carriesInvariant fun source =>
    (ns.invariants.find? (loopSite source.namespaceId.index source.structId)).isSome

/-- The local a place belongs to without a dereference: a write into the
place mutates the local's value. -/
def CompileNamespace.ownedRoot? (ns : CompileNamespace) : Nat → PlaceId → Option LocalId
  | 0, _ => none
  | fuel + 1, place =>
      match ns.places.get? place.index with
      | some (.localVar localId) => some localId
      | some (.field base _ _) | some (.index base _) | some (.subslice base ..)
      | some (.downcast base _) => ns.ownedRoot? fuel base
      | _ => none

/-- The local a loan borrows from without a dereference. -/
def CompileNamespace.lender? (ns : CompileNamespace) (loan : CheckedLoanFact) : Option LocalId :=
  match ns.expressions.get? loan.expression.index with
  | some { kind := .operation (.borrow _ place) _ _ _, .. } => ns.ownedRoot? ns.placeFuel place
  | _ => none

/-- Whether a loan is a mutable borrow of global memory, whose death ends a
write of it. -/
def CompileNamespace.writesGlobal (ns : CompileNamespace) (loan : CheckedLoanFact) : Bool :=
  match ns.expressions.get? loan.expression.index with
  | some { kind := .operation (.global (.borrow .mutable)) _ _ _, .. } => true
  | _ => false

/-- A place as the semantics reads it: a dereference of a shared reference is
its base, since the reference is the observed value itself. -/
def CompileNamespace.placeFuel? (ns : CompileNamespace) : Nat → PlaceId → Option Place
  | 0, _ => none
  | fuel + 1, place =>
      match ns.places.get? place.index with
      | some (.deref base) =>
          if (ns.shared.find? place.index).isSome then ns.placeFuel? fuel base
          else some (.deref base)
      | other => other

def CompileNamespace.place? (ns : CompileNamespace) (place : PlaceId) : Option Place :=
  ns.placeFuel? ns.placeFuel place

/-- Whether an operand is a shared reference, by its recorded type. -/
def CompileNamespace.sharedAt (ns : CompileNamespace) (operand : ExprId) : Bool :=
  match ns.expressions.get? operand.index with
  | some expression => isSharedReferenceType ns.source expression.typeId ns.types.get?
  | none => false

/-- The native type a type identifier denotes, read through the view's type
table for its own namespace. -/
def CompileNamespace.nty (ns : CompileNamespace) (unit : ValidatedUnit)
    (namespaceId : NamespaceId) (typeId : TypeId) : Option NTy :=
  ntyOfFuel (fun owner typeId => if owner == ns.namespaceId then ns.types.get? typeId.index
      else unitTypes unit owner typeId)
    unit ns.typeFuel namespaceId typeId

/-- The resource type a type identifier denotes, read through the view's
type table: its native type and its declaration's type arguments. -/
def CompileNamespace.resource (ns : CompileNamespace) (unit : ValidatedUnit)
    (namespaceId : NamespaceId) (typeId : TypeId) : Option (NTy × NRow) := do
  let ty ← if namespaceId == ns.namespaceId then ns.types.get? typeId.index
    else unitTypes unit namespaceId typeId
  match ty with
  | .nominal _ arguments =>
      let native ← ns.nty unit namespaceId typeId
      let arguments ← arguments.toList.mapM fun argument => match argument with
        | .typeArg value => ns.nty unit namespaceId value.typeId
        | _ => none
      some (native, NRow.ofList arguments)
  | _ => none

/-- The native type of a node, `none` for the `never` type of a node that
transfers control. -/
def typeOf? (unit : ValidatedUnit) (ns : CompileNamespace) (namespaceId : NamespaceId)
    (typeId : TypeId) : Except String (Option NTy) :=
  match ns.types.get? typeId.index with
  | some .never => .ok none
  | _ => match ns.nty unit namespaceId typeId with
    | some τ => .ok (some τ)
    | none => notCarried "the type of an expression"

/-- The typed position of a declaration index in a row. -/
def Var.ofIndex : (Γ : NRow) → Nat → Option (Σ τ : NTy, Var Γ τ)
  | .nil, _ => none
  | .cons τ _, 0 => some ⟨τ, .here⟩
  | .cons _ Γ, index + 1 => do
      let ⟨τ, x⟩ ← Var.ofIndex Γ index
      some ⟨τ, .there x⟩

/-- The position of a declaration index at an expected type. -/
def Var.at? (Γ : NRow) (index : Nat) (τ : NTy) : Except String (Var Γ τ) :=
  match Var.ofIndex Γ index with
  | none => .error "a local is out of range"
  | some ⟨τ', x⟩ =>
      if equal : τ' = τ then .ok (equal ▸ x) else .error "a local has an unexpected type"

/-- The variant choice a name denotes. -/
def Which.ofName : (names : List String) → (rows : NRows) → String →
    Option (Σ σs : NRow, Which names rows σs)
  | _, .nil, _ => none
  | [], .cons _ _, _ => none
  | name :: names, .cons fields rest, wanted =>
      if name = wanted then some ⟨fields, .here⟩
      else do
        let ⟨σs, later⟩ ← Which.ofName names rest wanted
        some ⟨σs, .there later⟩

/-- The payload choices of a variant-field selection at an expected type. -/
def Choices.ofList (names : List String) (rows : NRows) (τ : NTy) :
    List (String × Nat) → Except String (Choices names rows τ)
  | [] => .ok .nil
  | (variant, index) :: rest => do
      let some ⟨σs, choice⟩ := Which.ofName names rows variant
        | .error "a variant field selection names an unknown variant"
      let x ← Var.at? σs index τ
      let rest ← Choices.ofList names rows τ rest
      .ok (.cons choice x rest)

/-- A compiled expression: a term at its declared type, or a term of every
type, which is what a node of type `never` denotes. -/
inductive Compiled (unit : ValidatedUnit) (ρ : ResultShape) (Γ : NRow) : Type where
  | at (τ : NTy) (term : Term unit ρ Γ τ)
  | never (term : (τ : NTy) → Term unit ρ Γ τ)

/-- A compiled expression at an expected type. -/
def Compiled.at? {ρ : ResultShape} {Γ : NRow} (τ : NTy) :
    Compiled unit ρ Γ → Except String (Term unit ρ Γ τ)
  | .at τ' term =>
      if equal : τ' = τ then .ok (equal ▸ term) else .error "an expression has an unexpected type"
  | .never term => .ok (term τ)

/-- A compiled expression at its own type, taking unit for one of every type. -/
def Compiled.some {ρ : ResultShape} {Γ : NRow} : Compiled unit ρ Γ → Σ τ : NTy, Term unit ρ Γ τ
  | .at τ term => ⟨τ, term⟩
  | .never term => ⟨.unit, term .unit⟩

/-- Wrap a compiled expression in a type-preserving context. -/
def Compiled.map {ρ : ResultShape} {Γ : NRow}
    (wrap : (τ : NTy) → Term unit ρ Γ τ → Term unit ρ Γ τ) : Compiled unit ρ Γ → Compiled unit ρ Γ
  | .at τ term => .at τ (wrap τ term)
  | .never term => .never fun τ => wrap τ (term τ)

mutual
/-- The native value of a constant at its declared type. -/
def literalValue : (τ : NTy) → ConstValue → Except String τ.groundCarrier
  | .int width signed, .integer value =>
      if fits : IntegerValueFits (.bits width) signed value then .ok ⟨value, fits⟩
      else .error "an integer literal is out of range"
  | .bool, .bool value => .ok value
  | .unit, .unit => .ok ()
  | .address, .address value => .ok value
  | .string, .string value => .ok value
  | .bytes, .bytes value => .ok value
  | .tuple elements, .tuple values => literalRow elements values.toList
  | _, _ => notCarried "a literal of this type"

def literalRow : (row : NRow) → List ConstValue → Except String (@HList Carriers.ground row)
  | .nil, [] => .ok ()
  | .cons τ rest, value :: values => do
      let head ← literalValue τ value
      let tail ← literalRow rest values
      .ok (head, tail)
  | _, _ => .error "a tuple literal's arity differs from its type's"
end

/-- A typed place: the local it is rooted in and the path to its component. -/
structure TypedPlace (Γ : NRow) where
  root : NTy
  component : NTy
  x : Var Γ root
  path : Proj root component

/-- Extend a path by one more step at its end. -/
def Proj.append : {τ σ υ : NTy} → Proj τ σ → Proj σ υ → Proj τ υ
  | _, _, _, .nil, rest => rest
  | _, _, _, .deref path, rest => .deref (path.append rest)
  | _, _, _, .field x path, rest => .field x (path.append rest)
  | _, _, _, .index position path, rest => .index position (path.append rest)
  | _, _, _, .variant choices path, rest => .variant choices (path.append rest)

/-- The number of variants a selection chooses a field of. -/
def Choices.length {names : List String} {rows : NRows} {τ : NTy} :
    Choices names rows τ → Nat
  | .nil => 0
  | .cons _ _ rest => rest.length + 1

/-- Whether a selection leaves a variant without a field to select. -/
def Choices.partial {names : List String} {rows : NRows} {τ : NTy}
    (choices : Choices names rows τ) : Bool :=
  choices.length < names.length

/-- Whether a path steps into a field some variant lacks. -/
def Proj.mayMismatch : {τ σ : NTy} → Proj τ σ → Bool
  | _, _, .nil => false
  | _, _, .deref path | _, _, .field _ path | _, _, .index _ path => path.mayMismatch
  | _, _, .variant choices path => choices.partial || path.mayMismatch

/-- The throw an access along a path makes where a variant lacks the field:
the profile's mismatch throw, for a path that may meet one. -/
def CompileNamespace.mismatchOn (ns : CompileNamespace) {τ σ : NTy} (path : Proj τ σ) :
    Option Failure :=
  if path.mayMismatch then patternMismatchThrow? ns.source.profile else none

/-- The typed place a place names. -/
def compilePlace (unit : ValidatedUnit) (Γ : NRow) (ns : CompileNamespace) :
    Nat → PlaceId → Except String (TypedPlace Γ)
  | 0, _ => .error "the compiler ran out of fuel"
  | fuel + 1, placeId => do
      let some place := ns.place? placeId | .error "a place is out of range"
      match place with
      | .localVar localId =>
          let some ⟨τ, x⟩ := Var.ofIndex Γ localId.index | .error "a local is out of range"
          .ok ⟨τ, τ, x, .nil⟩
      | .deref base =>
          let ⟨root, component, x, path⟩ ← compilePlace unit Γ ns fuel base
          match component, path with
          | .ref referent, path => .ok ⟨root, referent, x, path.append (.deref .nil)⟩
          | _, _ => notCarried "a dereference of a place that is not a mutable reference"
      | .field base owner field =>
          -- A field below a downcast is the named variant's.
          let (base, variant?) := match ns.place? base with
            | some (.downcast inner variant) => (inner, some variant)
            | _ => (base, none)
          let ⟨root, component, x, path⟩ ← compilePlace unit Γ ns fuel base
          match component, path with
          | .struct source _ fields, path =>
              let some handle := resolveStruct? unit ns.source.identity owner
                | .error "a field place does not resolve"
              unless handle = source do .error "a field place's declaration differs from its type"
              let some name := sourceFieldName? ns.source field | .error "a field place has no name"
              let some index := handleFieldIndex? unit handle none name
                | .error "a field place names an unknown field"
              let some ⟨σ, position⟩ := Var.ofIndex fields index
                | .error "a field place is out of range"
              .ok ⟨root, σ, x, path.append (.field position .nil)⟩
          | .enum source _ names rows _, path =>
              let some handle := resolveStruct? unit ns.source.identity owner
                | .error "a variant field place does not resolve"
              unless handle = source do .error "a variant field place's declaration differs from its type"
              let some name := sourceFieldName? ns.source field | .error "a variant field place has no name"
              -- A field place reads the field of whichever variant declares
              -- it; below a downcast, of that variant.
              let listed ← match variant? with
                | none => .ok names
                | some variant => do
                    let some variantName := sourceFieldName? ns.source variant
                      | .error "a downcast place has no variant name"
                    .ok [variantName]
              let some choices := variantFieldChoices? unit handle
                  (listed.map (·, name)).toArray
                | .error "a variant field place has no choices"
              let (firstVariant, firstIndex) :: _ := choices.toList
                | .error "a variant field place names a field of no variant"
              let some ⟨σs, _⟩ := Which.ofName names rows firstVariant
                | .error "a variant field place names an unknown variant"
              let some ⟨σ, _⟩ := Var.ofIndex σs firstIndex
                | .error "a variant field place is out of range"
              let choices ← Choices.ofList names rows σ choices.toList
              .ok ⟨root, σ, x, path.append (.variant choices .nil)⟩
          | _, _ => notCarried "a field place on a non-struct"
      | .index base indexExpr =>
          let ⟨root, component, x, path⟩ ← compilePlace unit Γ ns fuel base
          let some form := placeIndexForm? ns.source indexExpr
            | notCarried "an element place with this index form"
          let position ← match form with
            | .literal value =>
                if value < 0 then notCarried "a negative element index"
                else .ok (PlaceIndex.literal value.toNat)
            | .local localId | .copyLocal localId => .ok (PlaceIndex.slot localId.index)
            | .fromEnd source offset =>
                if source == base then .ok (PlaceIndex.fromEnd offset)
                else notCarried "an element index from the end of another place"
          match component, path with
          | .vector σ, path => .ok ⟨root, σ, x, path.append (.index position .nil)⟩
          | _, _ => notCarried "an element place on a non-vector"
      | _ => notCarried "a place of this kind"

/-- Whether the components of a tuple hold references only at their top. -/
def NRow.componentsLendable : NRow → Bool
  | .nil => true
  | .cons (.ref referent) rest => referent.refFree && rest.componentsLendable
  | .cons τ rest => τ.refFree && rest.componentsLendable

/-- Whether a value's references are where the prophetic meaning lends
them: at its top, or as components of a tuple. -/
def NTy.lendable : NTy → Bool
  | .ref referent => referent.refFree
  | .tuple elements => elements.componentsLendable
  | τ => τ.refFree

/-- Whether a result's references are lendable. -/
def ResultShape.lendable : ResultShape → Bool
  | .none => true
  | .one τ => τ.lendable

/-- The loan facts of a function's borrow certificate, looked up once per
compiled function: the kernel evaluates the compilation, so a loan death
reads its loan from these rather than searching every certificate. -/
def loanFactsOf (unit : ValidatedUnit) (handle : FunctionHandle) : Array CheckedLoanFact :=
  -- A list search: the kernel searches an array by reading each element
  -- through the array's list.
  ((unit.borrowCertificates.toList.find? fun certificate =>
    certificate.namespaceId == handle.namespaceId &&
      certificate.functionId == handle.functionId).map (·.loans)).getD #[]

/-- The effect of ending loans, `none` when it resolves nothing. A loan's
death resolves the reference wherever it is held; a holder whose slot is
empty passed it on, and a loan consumed by a call has no holder here, since
the callee resolves it.  A holder without references observes the loan
through an erased shared borrow and resolves nothing. -/
def deathEffect (ns : CompileNamespace) (loanFacts : Array CheckedLoanFact) (ρ : ResultShape)
    (Γ : NRow) (ended : Array LoanId) :
    Except String (Option (Term unit ρ Γ .unit)) := do
  -- Innermost first, as `settleLoans` ends them.
  let deaths ← ended.toList.reverse.mapM fun loan => do
    let some holders := (loanFacts[loan.index]?).map (·.holders)
      | .error "a loan death names an unknown loan"
    holders.toList.filterMapM fun holder => do
      let some ⟨σ, x⟩ := Var.ofIndex Γ holder.index | .error "a local is out of range"
      match σ, x with
      | .ref _, x => .ok (some (Term.resolve (ρ := ρ) x))
      | σ, _ => if σ.refFree then .ok none else notCarried "a loan held inside an aggregate"
  -- Once resolved, a local a loan mutated owes its invariant: a mutation
  -- ends where its loan dies. A borrow of global memory ends its write.
  let checks ← ended.toList.reverse.filterMapM fun loan => do
    let some fact := loanFacts[loan.index]? | .error "a loan death names an unknown loan"
    let site := loopSite ns.namespaceId.index fact.expression.index
    if ns.writesGlobal fact then return some (Term.memoryWritten (ρ := ρ) (Γ := Γ) site)
    let some lender := ns.lender? fact | .ok none
    let some ⟨σ, _⟩ := Var.ofIndex Γ lender.index | .error "a local is out of range"
    if ns.carriesInvariant σ then .ok (some (Term.mutationEnd (ρ := ρ) (Γ := Γ) site))
    else .ok none
  let effects := deaths.flatten ++ checks
  if effects.isEmpty then .ok none
  else .ok (some (effects.foldr (fun effect rest => Term.drop effect rest) (.lit ())))

/-- A node's term with the loan deaths anchored at it: those before it
resolve first, those after it once it produced its value. -/
def withDeaths (ns : CompileNamespace) (loanFacts : Array CheckedLoanFact) (id : ExprId)
    (node : Compiled unit ρ Γ) : Except String (Compiled unit ρ Γ) := do
  let some deaths := ns.deaths.find? id.index | .ok node
  let node ← match ← deathEffect ns loanFacts ρ Γ deaths.after, node with
    | some effect, .at τ term => .ok (.at τ (.seqAfter term effect))
    | _, node => .ok node
  match ← deathEffect ns loanFacts ρ Γ deaths.before with
  | some effect => .ok (node.map fun _ term => .drop effect term)
  | none => .ok node

/-- The literal of a constant at its declared type. -/
def compileLiteral {ρ : ResultShape} {Γ : NRow} (τ : NTy) (literal : ConstValue) :
    Except String (Term unit ρ Γ τ) :=
  (.lit ·) <$> literalValue τ literal

/-- The local a place names, when it is a bare local. -/
def placeLocal? (ns : CompileNamespace) (place : PlaceId) : Except String LocalId :=
  match ns.place? place with
  | some (.localVar localId) => .ok localId
  | _ => notCarried "a place other than a local"

/-- The local an operand reads a reference from: the local itself, or a
copy or move of it. -/
def heldLocal? (ns : CompileNamespace) (operand : ExprId) : Option LocalId :=
  match ns.expressions.get? operand.index with
  | some { kind := .localVar localId, .. } => some localId
  | some { kind := .operation (.move place) _ #[] _, .. }
  | some { kind := .operation (.copy place) _ #[] _, .. } =>
      match ns.place? place with
      | some (.localVar localId) => some localId
      | _ => none
  | _ => none

mutual
/-- The typed pattern of a validated one at the type of the value it matches. -/
def compilePattern (Γ : NRow) (ns : CompileNamespace) :
    Nat → PatternId → (σ : NTy) → Except String (Pat Γ σ)
  | 0, _, _ => .error "the compiler ran out of fuel"
  | fuel + 1, pattern, σ =>
      match ns.patterns.get? pattern.index with
      | none => .error "a pattern is out of range"
      | some pattern =>
        match pattern.kind, σ with
        | .wildcard, _ => .ok .wildcard
        | .variable localId, σ => .var <$> Var.at? Γ localId.index σ
        | .literal value, σ => .literal <$> literalValue σ value
        | .range lower upper inclusive, .int _ _ => do
            let bound : Option ConstValue → Except String (Option Int)
              | none => .ok none
              | some (.integer value) => .ok (some value)
              | some _ => .error "an integer range has a bound of another type"
            .ok (.range (← bound lower) (← bound upper) inclusive)
        | .tuple elements, .tuple σs => .tuple <$> compilePatterns Γ ns fuel elements.toList σs
        | .constructor _ _ none fields, .struct _ _ σs =>
            .struct <$> compilePatterns Γ ns fuel fields.toList σs
        | .constructor _ _ (some variant) fields, .enum _ _ names rows _ =>
            match Which.ofName names rows variant with
            | some ⟨σs, choice⟩ => .variant choice <$> compilePatterns Γ ns fuel fields.toList σs
            | none => .error "a pattern names an unknown variant"
        | .range .., _ => notCarried "a range pattern over this type"
        | _, _ => .error "a pattern's shape differs from its type's"

/-- The typed patterns of a row of components. -/
def compilePatterns (Γ : NRow) (ns : CompileNamespace) :
    Nat → List PatternId → (σs : NRow) → Except String (Pats Γ σs)
  | 0, _, _ => .error "the compiler ran out of fuel"
  | _ + 1, [], .nil => .ok .nil
  | fuel + 1, pattern :: patterns, .cons σ σs => do
      let head ← compilePattern Γ ns fuel pattern σ
      .ok (.cons head (← compilePatterns Γ ns fuel patterns σs))
  | _ + 1, _, _ => .error "a pattern's arity differs from its declaration's"
end

/-- Match arms at a result type. -/
def armsAt {ρ : ResultShape} {Γ : NRow} {σ : NTy} (mismatch : Option Failure) (τ : NTy) :
    List (Pat Γ σ × Option (Term unit ρ Γ .bool) × Compiled unit ρ Γ) → Except String (Arms unit ρ Γ σ τ)
  | [] => .ok (.nil mismatch)
  | (pattern, guard, body) :: rest => do
      let body ← body.at? τ
      let rest ← armsAt mismatch τ rest
      match guard with
      | none => .ok (.cons pattern body rest)
      | some guard => .ok (.guarded pattern guard body rest)

/-- Match arms whose bodies never complete, at every result type. -/
def neverArms {ρ : ResultShape} {Γ : NRow} {σ : NTy} (mismatch : Option Failure) :
    List (Pat Γ σ × Option (Term unit ρ Γ .bool) × ((τ : NTy) → Term unit ρ Γ τ)) →
      (τ : NTy) → Arms unit ρ Γ σ τ
  | [], _ => .nil mismatch
  | (pattern, none, body) :: rest, τ => .cons pattern (body τ) (neverArms mismatch rest τ)
  | (pattern, some guard, body) :: rest, τ =>
      .guarded pattern guard (body τ) (neverArms mismatch rest τ)

mutual
  /-- The term of one expression: its node's, with the loan deaths at it. -/
  def compileExpr (unit : ValidatedUnit) (loanFacts : Array CheckedLoanFact)
      (ρ : ResultShape) (Γ : NRow) (ns : CompileNamespace) (namespaceId : NamespaceId) :
      Nat → ExprId → Except String (Compiled unit ρ Γ)
    | 0, _ => .error "the compiler ran out of fuel"
    | fuel + 1, id => do
        withDeaths ns loanFacts id (← compileNode unit loanFacts ρ Γ ns namespaceId fuel id)

  /-- The term of one node. -/
  def compileNode (unit : ValidatedUnit) (loanFacts : Array CheckedLoanFact)
      (ρ : ResultShape) (Γ : NRow) (ns : CompileNamespace) (namespaceId : NamespaceId) :
      Nat → ExprId → Except String (Compiled unit ρ Γ)
    | 0, _ => .error "the compiler ran out of fuel"
    | fuel + 1, id => do
        let some expression := ns.expressions.get? id.index
          | .error "an expression is out of range"
        let τ? ← typeOf? unit ns namespaceId expression.typeId
        match expression.kind with
        | .value literal _ =>
            let some τ := τ? | notCarried "a literal of type never"
            let term ← compileLiteral τ literal
            .ok (.at τ term)
        | .localVar localId =>
            let some τ := τ? | notCarried "a local of type never"
            let x ← Var.at? Γ localId.index τ
            -- A mutable reference used as a value moves out of its slot.
            match τ with
            | .ref _ => .ok (.at τ (.take x))
            | _ => .ok (.at τ (.var x))
        | .constant reference =>
            let some τ := τ? | notCarried "a constant of type never"
            let some handle := resolveConstant? unit namespaceId reference
              | .error "a constant does not resolve"
            let some targetNs := unit.namespaces[handle.namespaceId.index]?
              | .error "a constant's namespace is out of range"
            let some declaration := targetNs.constants[handle.constantId]?
              | .error "a constant is out of range"
            -- A constant of this namespace reads the view already built.
            let targetView := if handle.namespaceId == namespaceId then ns
              else CompileNamespace.of unit handle.namespaceId targetNs
            let value ← compileExpr unit loanFacts ρ .nil targetView handle.namespaceId fuel
              declaration.value
            let value ← value.at? τ
            .ok (.at τ (.const value))
        | .operation (.call (.function reference)) instantiations arguments _ =>
            let some handle := resolveFunction? unit namespaceId reference
              | .error "a callee does not resolve"
            let some calleeNs := unit.namespaces[handle.namespaceId.index]?
              | .error "a callee's namespace is out of range"
            let some callee := calleeNs.functions[handle.functionId.index]?
              | .error "a callee is out of range"
            let some parameters := callee.signature.parameters.toList.mapM fun parameter =>
                ns.nty unit handle.namespaceId parameter.typeUse.typeId
              | notCarried "the type of a callee parameter"
            unless parameters.all NTy.lendable do notCarried "a reference inside an aggregate"
            let parameters := NRow.ofList parameters
            let shape ← match callee.signature.results.toList with
              | [] => .ok ResultShape.none
              | [result] => match ns.nty unit handle.namespaceId result.typeId with
                  | some τ => .ok (ResultShape.one τ)
                  | none => notCarried "the type of a callee result"
              | _ => notCarried "a callee with several results"
            unless shape.lendable do notCarried "a reference inside an aggregate"
            if instantiations.isEmpty then
              let arguments ← compileArgs unit loanFacts ρ Γ ns namespaceId fuel arguments.toList
                parameters
              let value : Term unit ρ Γ shape.bodyType :=
                .call (loopSite namespaceId.index id.index) handle shape arguments
              match τ? with
              | some τ => .ok (.at τ (← Compiled.at? τ (.at shape.bodyType value)))
              | none => notCarried "a call of type never"
            else
              let some typeArgs := instantiations.toList.mapM fun argument => match argument with
                  | .typeArg value => some value
                  | _ => none
                | notCarried "a generic argument that is not a type"
              let some θ := typeArgs.mapM fun value => ns.nty unit namespaceId value.typeId
                | notCarried "the type of a type argument"
              let θ := NRow.ofList θ
              unless θ.refFree do notCarried "a reference type argument"
              let some (θ : TypeArgs) := if inhabitable : θ.inhabitable = true then
                  some ⟨θ, inhabitable⟩ else none
                | notCarried "a type argument without values"
              let arguments ← compileArgs unit loanFacts ρ Γ ns namespaceId fuel arguments.toList
                (NRow.subst θ.1 parameters)
              let value : Term unit ρ Γ (shape.subst θ.1).bodyType :=
                .callGeneric (loopSite namespaceId.index id.index) handle typeArgs.toArray θ shape
                  arguments
              match τ? with
              | some τ => .ok (.at τ (← Compiled.at? τ (.at (shape.subst θ.1).bodyType value)))
              | none => notCarried "a call of type never"
        | .operation (.call (.closure reference mask)) instantiations captures _ =>
            let some τ := τ? | notCarried "a closure of type never"
            let .function _ shared results := τ
              | .error "a closure's type is not a function type"
            let some handle := resolveFunction? unit namespaceId reference
              | .error "a closure's target does not resolve"
            let some targetNs := unit.namespaces[handle.namespaceId.index]?
              | .error "a closure target's namespace is out of range"
            let some target := targetNs.functions[handle.functionId.index]?
              | .error "a closure's target is out of range"
            let some targetParameters := target.signature.parameters.toList.mapM fun parameter =>
                ns.nty unit handle.namespaceId parameter.typeUse.typeId
              | notCarried "the type of a closure target's parameter"
            let ⟨captured, supplied, weave⟩ := Weave.ofMask mask targetParameters
            unless weave.mask == mask do notCarried "a mask beyond its target's parameters"
            if instantiations.isEmpty then
              unless captured.refFree do notCarried "a captured reference"
              -- The node carries its target's rows, as the runtime family reads them.
              if rows : closureRows? unit handle weave.mask = some (captured, supplied, results)
              then
                if sharing : closureShared unit handle weave.mask = shared then
                if closed : (captured.paramFree && supplied.paramFree && results.paramFree) = true
                then
                if faithful : closureFaithful unit handle #[] = true then
                  let captures ← compileArgs unit loanFacts ρ Γ ns namespaceId fuel
                    captures.toList captured
                  let value : Term unit ρ Γ (.function supplied shared results) :=
                    .closure handle weave results shared rows sharing closed faithful captures
                  .ok (.at τ (← Compiled.at? τ (.at (.function supplied shared results) value)))
                else notCarried "a closure whose empty instantiation is not faithful to its target"
                else notCarried "a closure without type arguments over type parameters"
                else notCarried "a closure whose shared parameters are not its target's"
              else notCarried "a closure whose rows are not its target's"
            else
              let some typeArgs := instantiations.toList.mapM fun argument => match argument with
                  | .typeArg value => some value
                  | _ => none
                | notCarried "a generic argument that is not a type"
              let some θ := typeArgs.mapM fun value => ns.nty unit namespaceId value.typeId
                | notCarried "the type of a type argument"
              let θ := NRow.ofList θ
              unless θ.refFree do notCarried "a reference type argument"
              let some (θ : TypeArgs) := if inhabitable : θ.inhabitable = true then
                  some ⟨θ, inhabitable⟩ else none
                | notCarried "a type argument without values"
              let shape ← match target.signature.results.toList with
                | [] => .ok ResultShape.none
                | [result] => match ns.nty unit handle.namespaceId result.typeId with
                    | some σ => .ok (ResultShape.one σ)
                    | none => notCarried "the type of a closure target's result"
                | _ => notCarried "a closure target with several results"
              unless (NRow.subst θ.1 captured).refFree do notCarried "a captured reference"
              -- The node carries its target's own rows; its frame's coherence is taken at
              -- creation.
              if rows : closureRows? unit handle weave.mask = some (captured, supplied, shape.row)
              then
                if sharing : closureShared unit handle weave.mask = shared then
                if below : closureSignatureBelow unit handle = true then
                  let captures ← compileArgs unit loanFacts ρ Γ ns namespaceId fuel
                    captures.toList (NRow.subst θ.1 captured)
                  let value : Term unit ρ Γ
                      (.function (NRow.subst θ.1 supplied) shared (shape.subst θ.1).row) :=
                    .closureGeneric handle weave typeArgs.toArray θ shape shared rows sharing below
                      captures
                  .ok (.at τ (← Compiled.at? τ
                    (.at (.function (NRow.subst θ.1 supplied) shared (shape.subst θ.1).row)
                      value)))
                else notCarried "a closure target whose signature names foreign type parameters"
                else notCarried "a closure whose shared parameters are not its target's"
              else notCarried "a closure whose rows are not its target's"
        | .operation (.call .invoke) _ arguments _ =>
            let callable :: arguments := arguments.toList
              | .error "an invocation has no function value"
            let some callableExpression := ns.expressions.get? callable.index
              | .error "an invocation's function value is out of range"
            let some (.function parameters shared results) :=
                ns.nty unit namespaceId callableExpression.typeId
              | notCarried "an invocation of a value that is not a function"
            unless parameters.toList.all NTy.lendable do
              notCarried "a reference inside an aggregate"
            let shape ← match results.toList with
              | [] => .ok ResultShape.none
              | [result] => .ok (ResultShape.one result)
              | _ => notCarried "a function value with several results"
            unless shape.lendable do notCarried "a reference inside an aggregate"
            let function ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel callable).at?
              (.function parameters shared shape.row)
            let arguments ← compileArgs unit loanFacts ρ Γ ns namespaceId fuel arguments parameters
            let value : Term unit ρ Γ shape.bodyType := .invoke shape function arguments
            match τ? with
            | some τ => .ok (.at τ (← Compiled.at? τ (.at shape.bodyType value)))
            | none => notCarried "an invocation of type never"
        | .operation (.borrow .mutable _) _ _ _ =>
            let some τ := τ? | notCarried "a borrow of type never"
            match expression.kind with
            | .operation operation _ arguments _ =>
                compileOperation unit loanFacts ρ Γ ns namespaceId fuel τ operation arguments.toList
            | _ => .error "internal: a borrow is not an operation"
        | .operation (.global kind) instantiations arguments _ =>
            let some τ := τ? | notCarried "a storage operation of type never"
            let #[.typeArg resource] := instantiations
              | notCarried "a storage operation without one resource type"
            let some (native, typeArguments) := ns.resource unit namespaceId resource.typeId
              | notCarried "a storage operation on a type without a native resource type"
            match kind, arguments.toList with
            | .contains, [key] => do
                let ⟨_, key⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel key).some
                .ok (.at .bool (.globalContains native typeArguments key))
            | .borrow .immutable, [key] => do
                let ⟨_, key⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel key).some
                unless τ = native do .error "a global read's type differs from its resource type"
                .ok (.at native (.globalRead typeArguments key))
            | .borrow .mutable, [key] => do
                let ⟨_, key⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel key).some
                unless τ = .ref native do
                  .error "a mutable global borrow's type differs from its resource type"
                .ok (.at (.ref native)
                  (.globalBorrow (τ := native) (loopSite namespaceId.index id.index) typeArguments key))
            | .take, [key] => do
                let ⟨_, key⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel key).some
                unless τ = native do .error "a take's type differs from its resource type"
                -- The write ends with the operation.
                let site := loopSite namespaceId.index id.index
                .ok (.at native (.seqAfter (.globalTake site typeArguments key) (.memoryWritten site)))
            | .publish, [key, value] => do
                let ⟨_, key⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel key).some
                let value ← Compiled.at? native
                  (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel value)
                let site := loopSite namespaceId.index id.index
                .ok (.at .unit (.seqAfter (.globalPublish site typeArguments key value)
                  (.memoryWritten site)))
            | _, _ => notCarried "a storage operation of this kind or arity"
        | .operation (.call (.constructor reference variant)) _ arguments _ =>
            let some τ := τ? | notCarried "a constructor of type never"
            let some handle := resolveStruct? unit namespaceId reference
              | .error "a constructor does not resolve"
            match τ, variant with
            | .struct source nominalArguments σs, none =>
                unless handle = source do .error "a constructor's declaration differs from its type"
                let fields ← compileArgs unit loanFacts ρ Γ ns namespaceId fuel arguments.toList σs
                let packed := Term.pack source nominalArguments fields
                .ok (.at (.struct source nominalArguments σs)
                  (if ns.carriesInvariant (.struct source nominalArguments σs) then
                    .constructed (loopSite namespaceId.index id.index) packed else packed))
            | .enum source nominalArguments names rows distinct, some name =>
                unless handle = source do .error "a constructor's declaration differs from its type"
                let some ⟨σs, choice⟩ := Which.ofName names rows name
                  | .error "a constructor names an unknown variant"
                let fields ← compileArgs unit loanFacts ρ Γ ns namespaceId fuel arguments.toList σs
                let packed := Term.variant source nominalArguments distinct choice fields
                .ok (.at (.enum source nominalArguments names rows distinct)
                  (if ns.carriesInvariant (.enum source nominalArguments names rows distinct) then
                    .constructed (loopSite namespaceId.index id.index) packed else packed))
            | _, _ => .error "a constructor's type is not its declaration's"
        | .operation operation _ arguments _ =>
            let some τ := τ? | notCarried "an operation of type never"
            compileOperation unit loanFacts ρ Γ ns namespaceId fuel τ operation arguments.toList
        | .block statements result =>
            compileBlock unit loanFacts ρ Γ ns namespaceId fuel statements.toList result
        | .letDecl _ none body =>
            compileExpr unit loanFacts ρ Γ ns namespaceId fuel body
        | .letDecl pattern (some value) body => do
            let compiledValue ← compileExpr unit loanFacts ρ Γ ns namespaceId fuel value
            -- A value that never completes never binds: the body is unreachable.
            if let .never term := compiledValue then return .never term
            let ⟨σ, value⟩ := compiledValue.some
            let body ← compileExpr unit loanFacts ρ Γ ns namespaceId fuel body
            compileBinding Γ ns fuel pattern σ value body
        -- A pattern assignment binds its value as a `let` does, to a unit.
        | .assignPattern pattern value => do
            let compiledValue ← compileExpr unit loanFacts ρ Γ ns namespaceId fuel value
            if let .never term := compiledValue then return .never term
            let ⟨σ, value⟩ := compiledValue.some
            compileBinding Γ ns fuel pattern σ value (.at .unit (.lit ()))
        | .match_ scrutinee arms => do
            let compiledScrutinee ← compileExpr unit loanFacts ρ Γ ns namespaceId fuel scrutinee
            if let .never term := compiledScrutinee then return .never term
            let ⟨σ, scrutinee⟩ := compiledScrutinee.some
            let arms ← arms.toList.mapM fun arm => do
              let pattern ← compilePattern Γ ns fuel arm.pattern σ
              let guard ← arm.guard.mapM fun guard => do
                (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel guard).at? .bool
              let body ← compileExpr unit loanFacts ρ Γ ns namespaceId fuel arm.body
              .ok (pattern, guard, body)
            -- A match of type never may still have arms that complete.
            let resultType? := τ?.orElse fun _ => arms.findSome? fun (_, _, body) =>
              match body with
              | .at τ _ => some τ
              | .never _ => none
            match resultType? with
            | some τ => .ok (.at τ (.caseOf scrutinee
                (← armsAt (patternMismatchThrow? ns.source.profile) τ arms)))
            | none =>
                let some arms := arms.mapM fun (pattern, guard, body) => match body with
                    | .never body => some (pattern, guard, body)
                    | .at .. => none
                  | .error "internal: a match of type never has an arm with a type"
                .ok (.never fun τ =>
                  .caseOf scrutinee (neverArms (patternMismatchThrow? ns.source.profile) arms τ))
        | .ifElse condition thenBranch elseBranch => do
            let condition ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel condition).at? .bool
            let thenBranch ← compileExpr unit loanFacts ρ Γ ns namespaceId fuel thenBranch
            let elseBranch ← match elseBranch with
              | some elseBranch => compileExpr unit loanFacts ρ Γ ns namespaceId fuel elseBranch
              | none => .ok (.at .unit (.lit ()))
            match τ? with
            | some τ => do
                let thenBranch ← thenBranch.at? τ
                let elseBranch ← elseBranch.at? τ
                .ok (.at τ (.ite condition thenBranch elseBranch))
            | none =>
                match thenBranch, elseBranch with
                | .never thenBranch, .never elseBranch =>
                    .ok (.never fun τ => .ite condition (thenBranch τ) (elseBranch τ))
                | .at σ thenBranch, elseBranch => do
                    let elseBranch ← elseBranch.at? σ
                    .ok (.at σ (.ite condition thenBranch elseBranch))
                | thenBranch, .at σ elseBranch => do
                    let thenBranch ← thenBranch.at? σ
                    .ok (.at σ (.ite condition thenBranch elseBranch))
        | .throw_ kind arguments =>
            match arguments.toList with
            | [] => .ok (.never fun _ => .throw0 kind)
            | [code] => do
                let ⟨_, code⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel code).some
                .ok (.never fun _ => .throw1 kind code)
            | _ => notCarried "a throw with several arguments"
        | .return_ values =>
            match values.toList with
            | [] => do
                let value ← Compiled.at? ρ.bodyType (Compiled.at (ρ := ρ) (Γ := Γ) .unit (.lit ()))
                .ok (.never fun _ => .return_ value)
            | [value] => do
                let value ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel value).at? ρ.bodyType
                .ok (.never fun _ => .return_ value)
            | _ => notCarried "a return of several values"
        | .assign place value =>
            match ns.place? place with
            | some (.localVar localId) => do
                let some ⟨σ, x⟩ := Var.ofIndex Γ localId.index | .error "a local is out of range"
                let value ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel value).at? σ
                .ok (.at .unit (.assign x value))
            | _ => do
                let ⟨_, σ, x, path⟩ ← compilePlace unit Γ ns fuel place
                let value ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel value).at? σ
                -- A write into a local whose type carries a data invariant
                -- ends a mutation of it.
                let owner := (ns.ownedRoot? ns.placeFuel place).bind fun root =>
                  (Var.ofIndex Γ root.index).map (·.1)
                if owner.any ns.carriesInvariant then
                  .ok (.at .unit (.seqAfter (.writePlace (ns.mismatchOn path) x path value)
                    (.mutationEnd (loopSite namespaceId.index id.index))))
                else .ok (.at .unit (.writePlace (ns.mismatchOn path) x path value))
        | .loop _ body => do
            let body ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel body).at? .unit
            .ok (.at .unit (.loop (loopSite namespaceId.index id.index) body))
        | .break_ nest none => .ok (.never fun _ => .break_ nest)
        | .continue_ nest => .ok (.never fun _ => .continue_ nest)
        -- An anchor records the state a later assertion reads, and the
        -- block's assumptions and its assertions are each one step at the
        -- block's site, in the order the block states them; the rest is no
        -- step.
        | .spec block =>
            let site := loopSite namespaceId.index id.index
            let isSave := fun (condition : Condition) =>
              condition.kind matches .assumption &&
                (ns.expressions.get? condition.expression.index).any
                  (·.kind matches .operation (.specification (.saveStateAnchor _)) _ _ _)
            -- A marker of the state or of a derivation is not assumed.
            let isMarker := fun (condition : Condition) =>
              (ns.expressions.get? condition.expression.index).any fun expression =>
                expression.kind matches .operation (.specification (.saveStateAnchor _)) _ _ _ ||
                  expression.kind matches
                    .operation (.specification (.foldsCaptureAnchor _)) _ _ _ ||
                  expression.kind matches .operation (.specification .inlineCallSummary) _ _ _
            let isAssumption := fun (condition : Condition) =>
              condition.kind matches .assumption && !isMarker condition
            -- A proof step is a site as an assertion is: what it owes is
            -- proved there and what it gives assumed after.
            let isAssertion := fun (condition : Condition) =>
              condition.kind matches .assertion | .apply | .split
            let conditions := block.conditions.toList
            let assumedAfter := (conditions.dropWhile (!isAssertion ·)).any isAssumption
            let assertedAfter := (conditions.dropWhile (!isAssumption ·)).any isAssertion
            if assumedAfter && assertedAfter then
              notCarried "a specification block interleaving assumptions and assertions"
            else
              let assumption : List (Term unit ρ Γ .unit) :=
                if conditions.any isAssumption then [.assume site] else []
              let assertion : List (Term unit ρ Γ .unit) :=
                if conditions.any isAssertion then [.assertion site] else []
              let steps := (if conditions.any isSave then [.anchor site] else []) ++
                if assumedAfter then assertion ++ assumption else assumption ++ assertion
              match steps with
              | [] => .ok (.at .unit (.lit ()))
              | first :: rest => .ok (.at .unit (rest.foldl .seqAfter first))
        | kind => notCarried (describeKind kind)

  /-- A read of a place at its declared type; `consume` moves a mutable
  reference out of its local. -/
  def compileRead (unit : ValidatedUnit) (Γ : NRow) (ns : CompileNamespace) (consume : Bool) :
      Nat → NTy → PlaceId → Except String (Compiled unit ρ Γ)
    | 0, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, τ, place =>
        match ns.place? place with
        | some (.localVar localId) => do
            let x ← Var.at? Γ localId.index τ
            match consume, τ with
            | true, .ref _ => .ok (.at τ (.take x))
            | _, _ => .ok (.at τ (.var x))
        | _ => do
            let ⟨_, component, x, path⟩ ← compilePlace unit Γ ns fuel place
            if equal : component = τ then .ok (.at τ (.readPlace (ns.mismatchOn path) x (equal ▸ path)))
            else .error "a place read has an unexpected type"

  /-- An argument row at the expected types. -/
  def compileArgs (unit : ValidatedUnit) (loanFacts : Array CheckedLoanFact)
      (ρ : ResultShape) (Γ : NRow) (ns : CompileNamespace) (namespaceId : NamespaceId) :
      Nat → List ExprId → (σs : NRow) → Except String (Args unit ρ Γ σs)
    | 0, _, _ => .error "the compiler ran out of fuel"
    | _ + 1, [], .nil => .ok .nil
    | fuel + 1, argument :: arguments, .cons σ σs => do
        let head ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel argument).at? σ
        let tail ← compileArgs unit loanFacts ρ Γ ns namespaceId fuel arguments σs
        .ok (.cons head tail)
    | _ + 1, _, _ => .error "an argument row's arity differs from its declaration's"

  /-- Bind a value to a pattern around a body. -/
  def compileBinding {unit : ValidatedUnit} {ρ : ResultShape} (Γ : NRow) (ns : CompileNamespace) :
      Nat → PatternId → (σ : NTy) → Term unit ρ Γ σ → Compiled unit ρ Γ → Except String (Compiled unit ρ Γ)
    | 0, _, _, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, patternId, σ, value, body => do
        let some pattern := ns.patterns.get? patternId.index | .error "a pattern is out of range"
        -- A row of binders destructures directly; any other pattern is a
        -- match of one arm.
        let flat (children : Array PatternId) := children.all fun child =>
          match ns.patterns.get? child.index with
          | some { kind := .wildcard, .. } | some { kind := .variable _, .. } => true
          | _ => false
        let general : Except String (Compiled unit ρ Γ) := do
          let typed ← compilePattern Γ ns fuel patternId σ
          .ok (body.map fun _ body =>
            .caseOf value (.cons typed body (.nil (patternMismatchThrow? ns.source.profile))))
        match pattern.kind with
        | .variable localId => do
            let x ← Var.at? Γ localId.index σ
            .ok (body.map fun _ body => .let_ x value body)
        | .wildcard => .ok (body.map fun _ body => .drop value body)
        | .tuple elements =>
            if !flat elements then general else
            match σ, value with
            | .tuple σs, value => do
                let targets ← compileTargets Γ ns fuel elements.toList σs
                .ok (body.map fun _ body => .letRow targets value body)
            | _, _ => .error "a tuple pattern binds a non-tuple"
        | .constructor _ _ none fields =>
            if !flat fields then general else
            match σ, value with
            | .struct _ _ σs, value => do
                let targets ← compileTargets Γ ns fuel fields.toList σs
                .ok (body.map fun _ body => .letFields targets value body)
            | _, _ => .error "a struct pattern binds a non-struct"
        | _ => general

  /-- The slots a row of variable or wildcard patterns binds. -/
  def compileTargets (Γ : NRow) (ns : CompileNamespace) :
      Nat → List PatternId → (σs : NRow) → Except String (Vars Γ σs)
    | 0, _, _ => .error "the compiler ran out of fuel"
    | _ + 1, [], .nil => .ok .nil
    | fuel + 1, pattern :: patterns, .cons σ σs => do
        let some pattern := ns.patterns.get? pattern.index | .error "a pattern is out of range"
        let target ← match pattern.kind with
          | .variable localId => some <$> Var.at? Γ localId.index σ
          | .wildcard => .ok none
          | _ => notCarried "a nested destructuring pattern"
        let rest ← compileTargets Γ ns fuel patterns σs
        .ok (.cons target rest)
    | _ + 1, _, _ => .error "a pattern's arity differs from its declaration's"

  /-- The statements of a block, then its result. -/
  def compileBlock (unit : ValidatedUnit) (loanFacts : Array CheckedLoanFact)
      (ρ : ResultShape) (Γ : NRow) (ns : CompileNamespace) (namespaceId : NamespaceId) :
      Nat → List ExprId → Option ExprId → Except String (Compiled unit ρ Γ)
    | 0, _, _ => .error "the compiler ran out of fuel"
    | _ + 1, [], none => .ok (.at .unit (.lit ()))
    | fuel + 1, [], some result => compileExpr unit loanFacts ρ Γ ns namespaceId fuel result
    | fuel + 1, statement :: statements, result => do
        let ⟨_, statement⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel statement).some
        let rest ← compileBlock unit loanFacts ρ Γ ns namespaceId fuel statements result
        .ok (rest.map fun _ rest => .drop statement rest)

  /-- An operation at its declared result type. -/
  def compileOperation (unit : ValidatedUnit) (loanFacts : Array CheckedLoanFact)
      (ρ : ResultShape) (Γ : NRow) (ns : CompileNamespace) (namespaceId : NamespaceId) :
      Nat → NTy → Operation → List ExprId → Except String (Compiled unit ρ Γ)
    | 0, _, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, τ, .primitive primitive, arguments =>
        compilePrimitive unit loanFacts ρ Γ ns namespaceId fuel τ primitive arguments
    | fuel + 1, τ, .copy place, [] => compileRead unit Γ ns false fuel τ place
    | fuel + 1, τ, .move place, [] => compileRead unit Γ ns true fuel τ place
    | fuel + 1, τ, .read place, [] => compileRead unit Γ ns false fuel τ place
    | fuel + 1, τ, .borrow .immutable place, [] => compileRead unit Γ ns false fuel τ place
    | fuel + 1, .ref referent, .borrow .mutable place, [] => do
        let ⟨_, component, x, path⟩ ← compilePlace unit Γ ns fuel place
        if equal : component = referent then
          .ok (.at (.ref referent) (.borrowPlace (ns.mismatchOn path) x (equal ▸ path)))
        else .error "a mutable borrow has an unexpected type"
    | fuel + 1, τ, .reference .dereference, [operand] => do
        -- A shared reference is the observed value itself.
        if ns.sharedAt operand then
          return .at τ (← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).at? τ)
        -- Reading through a local reference leaves it in place.
        if let some { kind := .localVar localId, .. } := ns.expressions.get? operand.index then
          let x ← Var.at? Γ localId.index (.ref τ)
          return .at τ (.deref (.var x))
        let ⟨σ, operand⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).some
        match σ, operand with
        | .ref referent, operand =>
            if equal : referent = τ then .ok (.at τ (.deref (equal ▸ operand)))
            else .error "a dereference has an unexpected type"
        | _, _ => notCarried "a dereference of a non-reference"
    | fuel + 1, τ, .reference (.borrow .immutable), [operand] => do
        -- A shared borrow of a computed value is that value: a shared
        -- reference denotes its referent, as a shared borrow of a place
        -- denotes the place's read.
        let value ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).at? τ
        .ok (.at τ value)
    | fuel + 1, τ, .reference (.freeze _), [operand] => do
        -- Freezing a shared reference reads it.
        if ns.sharedAt operand then
          return .at τ (← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).at? τ)
        -- A freeze of a mutable reference is a shared reborrow: the result
        -- is the current value, and the loan's death, not the freeze,
        -- resolves its prophecy.
        let some localId := heldLocal? ns operand
          | notCarried "a freeze of a reference that no local holds"
        let x ← Var.at? Γ localId.index (.ref τ)
        .ok (.at τ (.deref (.var x)))
    | fuel + 1, .unit, .reference .mutate, [target, value] => do
        let some { kind := .localVar localId, .. } := ns.expressions.get? target.index
          | notCarried "a mutation of a reference that is not a local"
        let some ⟨σ, x⟩ := Var.ofIndex Γ localId.index | .error "a local is out of range"
        match σ, x with
        | .ref referent, x => do
            let value ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel value).at? referent
            .ok (.at .unit (.mutate x value))
        | _, _ => notCarried "a mutation of a non-reference local"
    | fuel + 1, τ, .data (.select reference field), [operand] => do
        let ⟨σ, operand⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).some
        match σ, operand with
        | .struct _ _ σs, operand => do
            let some index := referencedFieldIndex? unit namespaceId reference none field
              | .error "a field selection does not resolve"
            let x ← Var.at? σs index τ
            .ok (.at τ (.field x operand))
        | _, _ => notCarried "a field selection on a non-struct"
    | fuel + 1, .bool, .data (.testVariants _ variants), [operand] => do
        let ⟨σ, operand⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).some
        match σ, operand with
        | .enum _ _ _ _ _, operand => .ok (.at .bool (.isVariant variants.toList operand))
        | _, _ => notCarried "a variant test on a non-enum"
    | fuel + 1, τ, .data (.selectVariants reference fields), [operand] => do
        let ⟨σ, operand⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).some
        match σ, operand with
        | .enum source _ names rows _, operand => do
            let some handle := resolveStruct? unit namespaceId reference
              | .error "a variant field selection does not resolve"
            unless handle = source do .error "a variant field selection's declaration differs"
            let some choices := variantFieldChoices? unit handle fields
              | .error "a variant field selection has no choices"
            let choices ← Choices.ofList names rows τ choices.toList
            let mismatch := if choices.partial then patternMismatchThrow? ns.source.profile else none
            .ok (.at τ (.payload mismatch choices operand))
        | _, _ => notCarried "a variant field selection on a non-enum"
    | _ + 1, _, operation, _ => notCarried (describeKind (.operation operation #[] #[] none))

  /-- A pure primitive at its declared result type. -/
  def compilePrimitive (unit : ValidatedUnit) (loanFacts : Array CheckedLoanFact)
      (ρ : ResultShape) (Γ : NRow) (ns : CompileNamespace) (namespaceId : NamespaceId) :
      Nat → NTy → PrimitiveOperation → List ExprId → Except String (Compiled unit ρ Γ)
    | 0, _, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, τ, .copyValue, [operand] => do
        let operand ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).at? τ
        .ok (.at τ operand)
    | fuel + 1, τ, .moveValue, [operand] => do
        let operand ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).at? τ
        .ok (.at τ operand)
    | fuel + 1, .tuple σs, .tuple, elements => do
        let elements ← compileArgs unit loanFacts ρ Γ ns namespaceId fuel elements σs
        .ok (.at (.tuple σs) (.tuple elements))
    | fuel + 1, τ, .add, [left, right] =>
        compileModular unit loanFacts ρ Γ ns namespaceId fuel τ .add left right
    | fuel + 1, τ, .subtract, [left, right] =>
        compileModular unit loanFacts ρ Γ ns namespaceId fuel τ .subtract left right
    | fuel + 1, τ, .multiply, [left, right] =>
        compileModular unit loanFacts ρ Γ ns namespaceId fuel τ .multiply left right
    | fuel + 1, τ, .checkedAdd failure, [left, right] =>
        compileChecked unit loanFacts ρ Γ ns namespaceId fuel τ .add failure left right
    | fuel + 1, τ, .checkedSubtract failure, [left, right] =>
        compileChecked unit loanFacts ρ Γ ns namespaceId fuel τ .subtract failure left right
    -- A checked negation fails exactly where the checked subtraction from
    -- zero does, with the same result.
    | fuel + 1, .int width signed, .checkedNegate failure, [operand] => do
        let zero ← compileLiteral (.int width signed) (.integer 0)
        let operand ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).at?
          (.int width signed)
        .ok (.at (.int width signed) (.checked .subtract failure zero operand))
    | fuel + 1, τ, .checkedMultiply failure, [left, right] =>
        compileChecked unit loanFacts ρ Γ ns namespaceId fuel τ .multiply failure left right
    | fuel + 1, τ, .checkedDivide failure, [left, right] =>
        compileChecked unit loanFacts ρ Γ ns namespaceId fuel τ .divide failure left right
    | fuel + 1, τ, .checkedModulo failure, [left, right] =>
        compileChecked unit loanFacts ρ Γ ns namespaceId fuel τ .modulo failure left right
    | fuel + 1, τ, .less, [left, right] =>
        compileCompare unit loanFacts ρ Γ ns namespaceId fuel τ .less left right
    | fuel + 1, τ, .greater, [left, right] =>
        compileCompare unit loanFacts ρ Γ ns namespaceId fuel τ .greater left right
    | fuel + 1, τ, .lessEqual, [left, right] =>
        compileCompare unit loanFacts ρ Γ ns namespaceId fuel τ .lessEqual left right
    | fuel + 1, τ, .greaterEqual, [left, right] =>
        compileCompare unit loanFacts ρ Γ ns namespaceId fuel τ .greaterEqual left right
    | fuel + 1, .bool, .equal, [left, right] => do
        let ⟨σ, left⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).some
        -- `NTy.eqb` is equality only on a ref-free type; at a reference it is
        -- `false` however the executor compares the borrows. Decline rather
        -- than carry an equality the execution does not agree with.
        unless σ.refFree do notCarried "an equality of a value holding a reference"
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? σ
        .ok (.at .bool (.equal false left right))
    | fuel + 1, .bool, .notEqual, [left, right] => do
        let ⟨σ, left⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).some
        unless σ.refFree do notCarried "an inequality of a value holding a reference"
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? σ
        .ok (.at .bool (.equal true left right))
    | fuel + 1, .bool, .logicalNot, [operand] => do
        let operand ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).at? .bool
        .ok (.at .bool (.not operand))
    | fuel + 1, .bool, .logicalAnd, [left, right] => do
        let left ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).at? .bool
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? .bool
        .ok (.at .bool (.logical true left right))
    | fuel + 1, .bool, .logicalOr, [left, right] => do
        let left ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).at? .bool
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? .bool
        .ok (.at .bool (.logical false left right))
    | fuel + 1, .int width false, .bitwiseAnd, [left, right] => do
        let left ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).at? (.int width false)
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? (.int width false)
        .ok (.at (.int width false) (.bitwise .and left right))
    | fuel + 1, .int width false, .bitwiseOr, [left, right] => do
        let left ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).at? (.int width false)
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? (.int width false)
        .ok (.at (.int width false) (.bitwise .or left right))
    | fuel + 1, .int width false, .bitwiseXor, [left, right] => do
        let left ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).at? (.int width false)
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? (.int width false)
        .ok (.at (.int width false) (.bitwise .xor left right))
    | fuel + 1, τ, .checkedShiftLeft failure, [value, distance] =>
        compileShift unit loanFacts ρ Γ ns namespaceId fuel τ true failure value distance
    | fuel + 1, τ, .checkedShiftRight failure, [value, distance] =>
        compileShift unit loanFacts ρ Γ ns namespaceId fuel τ false failure value distance
    | fuel + 1, .int width' signed', .checkedCast failure, [value] => do
        let ⟨σ, value⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel value).some
        match σ, value with
        | .int _ _, value => .ok (.at (.int width' signed') (.cast failure value))
        | _, _ => notCarried "a cast from a non-integer"
    | fuel + 1, .vector τ, .vector, elements => do
        let count := elements.length
        let elements ← compileArgs unit loanFacts ρ Γ ns namespaceId fuel elements
          (NRow.replicate count τ)
        .ok (.at (.vector τ) (.vectorLit count elements))
    | fuel + 1, .address, .signerAddress, [signer] => do
        let ⟨σ, signer⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel signer).some
        match σ, signer with
        | .signer, signer => .ok (.at .address (.signerAddress signer))
        | _, _ => notCarried "a signer's address of a non-signer"
    | fuel + 1, .int 64 false, .length, [vector] => do
        let ⟨σ, vector⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel vector).some
        match σ, vector with
        | .vector _, vector => .ok (.at (.int 64 false) (.length vector))
        | _, _ => notCarried "a length of a non-vector"
    | fuel + 1, _, .index, [vector, position] => do
        let ⟨σ, vector⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel vector).some
        let ⟨π, position⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel position).some
        match σ, vector with
        | .vector τ, vector =>
            match π, position with
            | .int _ _, position => .ok (.at τ (.index vector position))
            | _, _ => notCarried "an element read at a non-integer position"
        | _, _ => notCarried "an element read of a non-vector"
    | fuel + 1, .unit, .checkVectorIndex failure, [vector, position] => do
        let ⟨σ, vector⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel vector).some
        let ⟨π, position⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel position).some
        match σ, vector with
        | .vector _, vector =>
            match π, position with
            | .int _ _, position => .ok (.at .unit (.checkIndex failure vector position))
            | _, _ => notCarried "a bounds check at a non-integer position"
        | _, _ => notCarried "a bounds check of a non-vector"
    | fuel + 1, _, .pushVector, [vector, element] => do
        let ⟨σ, vector⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel vector).some
        match σ, vector with
        | .vector τ, vector => do
            let element ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel element).at? τ
            .ok (.at (.vector τ) (.push vector element))
        | _, _ => notCarried "a push onto a non-vector"
    | fuel + 1, _, .insertVector, [vector, position, element] => do
        let ⟨σ, vector⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel vector).some
        let ⟨π, position⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel position).some
        match σ, vector with
        | .vector τ, vector =>
            match π, position with
            | .int _ _, position => do
                let element ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel element).at? τ
                .ok (.at (.vector τ) (.insert vector position element))
            | _, _ => notCarried "an insertion at a non-integer position"
        | _, _ => notCarried "an insertion into a non-vector"
    | fuel + 1, _, .removeVector, [vector, position] => do
        let ⟨σ, vector⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel vector).some
        let ⟨π, position⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel position).some
        match σ, vector with
        | .vector τ, vector =>
            match π, position with
            | .int _ _, position =>
                .ok (.at (.tuple (.cons τ (.cons (.vector τ) .nil))) (.remove vector position))
            | _, _ => notCarried "a removal at a non-integer position"
        | _, _ => notCarried "a removal from a non-vector"
    | fuel + 1, _, .swapVector, [vector, left, right] => do
        let ⟨σ, vector⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel vector).some
        let ⟨π, left⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).some
        match σ, vector with
        | .vector τ, vector =>
          match π, left with
          | .int width signed, left => do
              let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at?
                (.int width signed)
              .ok (.at (.vector τ) (.swap vector left right))
          | _, _ => notCarried "a swap at non-integer positions"
        | _, _ => notCarried "a swap in a non-vector"
    | fuel + 1, _, .concatVector, [left, right] => do
        let ⟨σ, left⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).some
        match σ, left with
        | .vector τ, left => do
            let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? (.vector τ)
            .ok (.at (.vector τ) (.concat left right))
        | _, _ => notCarried "a concatenation of non-vectors"
    | fuel + 1, _, .slice, [vector, start, stop] => do
        let ⟨σ, vector⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel vector).some
        let ⟨π, start⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel start).some
        match σ, vector with
        | .vector τ, vector =>
          match π, start with
          | .int width signed, start => do
              let stop ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel stop).at?
                (.int width signed)
              .ok (.at (.vector τ) (.slice vector start stop))
          | _, _ => notCarried "a slice at non-integer positions"
        | _, _ => notCarried "a slice of a non-vector"
    | fuel + 1, _, .reverseSliceVector, [vector, start, stop] => do
        let ⟨σ, vector⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel vector).some
        let ⟨π, start⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel start).some
        match σ, vector with
        | .vector τ, vector =>
          match π, start with
          | .int width signed, start => do
              let stop ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel stop).at?
                (.int width signed)
              .ok (.at (.vector τ) (.reverseSlice vector start stop))
          | _, _ => notCarried "a reversal at non-integer positions"
        | _, _ => notCarried "a reversal of a non-vector"
    | fuel + 1, .unit, .destroyEmptyVector, [vector] => do
        let ⟨σ, vector⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel vector).some
        match σ, vector with
        | .vector _, vector => .ok (.at .unit (.destroyEmpty vector))
        | _, _ => notCarried "a destruction of a non-vector"
    | fuel + 1, _, .containsVector, [vector, needle] => do
        let ⟨σ, vector⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel vector).some
        match σ, vector with
        | .vector τ, vector => do
            -- Both searches compare elements with `NTy.eqb`, which is `false`
            -- at a reference whatever the executor's value equality reports.
            unless τ.refFree do notCarried "a search for a value holding a reference"
            let needle ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel needle).at? τ
            .ok (.at .bool (.contains vector needle))
        | _, _ => notCarried "a search in a non-vector"
    | fuel + 1, _, .indexOfVector, [vector, needle] => do
        let ⟨σ, vector⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel vector).some
        match σ, vector with
        | .vector τ, vector => do
            unless τ.refFree do notCarried "a search for a value holding a reference"
            let needle ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel needle).at? τ
            .ok (.at (.tuple (.cons .bool (.cons (.int 64 false) .nil))) (.indexOf vector needle))
        | _, _ => notCarried "a search in a non-vector"
    | fuel + 1, .int 8 true, .compare, [left, right] => do
        let ⟨σ, left⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).some
        -- A reference is carried as its (current, prophecy) pair, so the
        -- denotation would order it by its contents. The executor orders a
        -- borrow by its loan before its contents, which the pair does not
        -- record, so the order of a value holding one is not carried.
        unless σ.refFree do notCarried "a structural order of a value holding a reference"
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? σ
        .ok (.at (.int 8 true) (.order ns.source.orders left right))
    | _ + 1, _, primitive, _ => notCarried s!"primitive {repr primitive} at this type or arity"

  def compileChecked (unit : ValidatedUnit) (loanFacts : Array CheckedLoanFact)
      (ρ : ResultShape) (Γ : NRow) (ns : CompileNamespace) (namespaceId : NamespaceId) :
      Nat → NTy → CheckedOp → ThrowKind → ExprId → ExprId → Except String (Compiled unit ρ Γ)
    | 0, _, _, _, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, .int width signed, op, failure, left, right => do
        let left ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).at? (.int width signed)
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? (.int width signed)
        .ok (.at (.int width signed) (.checked op failure left right))
    | _ + 1, _, _, _, _, _ => notCarried "checked arithmetic at a non-integer type"

  def compileModular (unit : ValidatedUnit) (loanFacts : Array CheckedLoanFact)
      (ρ : ResultShape) (Γ : NRow) (ns : CompileNamespace) (namespaceId : NamespaceId) :
      Nat → NTy → ModularOp → ExprId → ExprId → Except String (Compiled unit ρ Γ)
    | 0, _, _, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, .int width signed, op, left, right => do
        let left ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).at? (.int width signed)
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? (.int width signed)
        .ok (.at (.int width signed) (.modular op left right))
    | _ + 1, _, _, _, _ => notCarried "modular arithmetic at a non-integer type"

  def compileCompare (unit : ValidatedUnit) (loanFacts : Array CheckedLoanFact)
      (ρ : ResultShape) (Γ : NRow) (ns : CompileNamespace) (namespaceId : NamespaceId) :
      Nat → NTy → CompareOp → ExprId → ExprId → Except String (Compiled unit ρ Γ)
    | 0, _, _, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, .bool, op, left, right => do
        let ⟨σ, left⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).some
        match σ, left with
        | .int width signed, left => do
            let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? (.int width signed)
            .ok (.at .bool (.compare op left right))
        | _, _ => notCarried "a comparison of non-integers"
    | _ + 1, _, _, _, _ => notCarried "a comparison at a non-Boolean type"

  def compileShift (unit : ValidatedUnit) (loanFacts : Array CheckedLoanFact)
      (ρ : ResultShape) (Γ : NRow) (ns : CompileNamespace) (namespaceId : NamespaceId) :
      Nat → NTy → Bool → ThrowKind → ExprId → ExprId → Except String (Compiled unit ρ Γ)
    | 0, _, _, _, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, .int width false, left, failure, value, distance => do
        let value ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel value).at? (.int width false)
        let ⟨σ, distance⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel distance).some
        match σ, distance with
        | .int _ false, distance => .ok (.at (.int width false) (.shift left failure value distance))
        | _, _ => notCarried "a shift by a signed or non-integer distance"
    | _ + 1, _, _, _, _, _ => notCarried "a shift of a signed integer"
end


/-- The compilation view of a namespace. A certificate states it once per
namespace, so a function's certificate does not rebuild its arenas. -/
def compileNamespaceAt? (unit : ValidatedUnit) (namespaceId : NamespaceId) :
    Option CompileNamespace :=
  (unit.namespaces[namespaceId.index]?).map (CompileNamespace.of unit namespaceId)

/-- The mutable-reference slots among the `count` slots from `index`. -/
def mutableSlots (Γ : NRow) : Nat → Nat → Except String (Mutables Γ)
  | _, 0 => .ok .nil
  | index, count + 1 => do
      let rest ← mutableSlots Γ (index + 1) count
      match Var.ofIndex Γ index with
      | some ⟨.ref _, x⟩ => .ok (.cons x rest)
      | some _ => .ok rest
      | none => .error "a parameter is out of range"

/-- Whether a type is logical only: it types specification values, never
executable ones, so no body reads a local of it. -/
def logicalOnly (unit : ValidatedUnit) (namespaceId : NamespaceId) (typeId : TypeId) : Bool :=
  match unit.namespaces[namespaceId.index]?.bind (·.tables.types[typeId.index]?) with
  | some (.integer .unbounded _) | some .range | some (.typeDomain _)
  | some (.resourceDomain ..) | some .stateDomain => true
  | _ => false

/-- The compiled function a handle names in its namespace's view, or the
construct that stops it. -/
def compileFunctionIn (unit : ValidatedUnit) (view : CompileNamespace) (handle : FunctionHandle) :
    Except String (Function unit) := do
  let some declaration := view.source.functions[handle.functionId.index]?
    | .error "a function is out of range"
  let .structured root := declaration.body | notCarried "a function without a body"
  -- A specification local (a quantifier's binder) keeps an empty slot.
  let some allLocals := declaration.locals.toList.mapM fun localDecl =>
      match view.nty unit handle.namespaceId localDecl.type.typeId with
      | some τ => some τ
      | none => if logicalOnly unit handle.namespaceId localDecl.type.typeId then some .unit else none
    | notCarried "the type of a local"
  let paramCount := declaration.signature.parameters.size
  if paramCount > allLocals.length then .error "fewer locals than parameters" else
  let params := NRow.ofList (allLocals.take paramCount)
  let locals := NRow.ofList (allLocals.drop paramCount)
  let some paramTypes := declaration.signature.parameters.toList.mapM fun parameter =>
      view.nty unit handle.namespaceId parameter.typeUse.typeId
    | notCarried "the type of a parameter"
  if NRow.ofList paramTypes != params then
    .error "parameter types differ from the leading locals" else
  let result ← match declaration.signature.results.toList with
    | [] => .ok ResultShape.none
    | [result] => match view.nty unit handle.namespaceId result.typeId with
        | some τ => .ok (ResultShape.one τ)
        | none => notCarried "the type of the result"
    | _ => notCarried "several results"
  unless paramTypes.all NTy.lendable && result.lendable do
    notCarried "a reference inside an aggregate"
  let body ← compileExpr unit (loanFactsOf unit handle) result (params ++ locals)
    view handle.namespaceId
    view.fuel root
  let body ← body.at? result.bodyType
  -- A parameter's reference resolves where the function leaves it: in its
  -- own slot or in a local it was moved into.
  let mutables ← mutableSlots (params ++ locals) 0 allLocals.length
  .ok { params, locals, result, body, mutables }

/-- The compiled function a handle names, or the construct that stops it. -/
def compileFunction (unit : ValidatedUnit)
    (handle : FunctionHandle) : Except String (Function unit) :=
  match compileNamespaceAt? unit handle.namespaceId with
  | none => .error "a namespace is out of range"
  | some view => compileFunctionIn unit view handle

theorem compileFunction_of_view {unit : ValidatedUnit} {handle : FunctionHandle}
    {view : CompileNamespace} (viewed : compileNamespaceAt? unit handle.namespaceId = some view) :
    compileFunction unit handle = compileFunctionIn unit view handle := by
  simp [compileFunction, viewed]

/-- `frameInstantiation` read through a namespace's view: its types indexed,
iterated as the list they are. -/
def frameInstantiationIn (view : CompileNamespace) (outer : Array (TypeId × TypeId))
    (typeArgs : Array TypeUse) : Array (TypeId × TypeId) :=
  if typeArgs.isEmpty then #[] else
    LeanerIR.SemanticOperations.invocationTypeInstantiationIn view.source view.types
      view.source.tables.types.toList outer (typeArgs.map .typeArg)

theorem frameInstantiation_of_view {unit : ValidatedUnit} {handle : FunctionHandle}
    {view : CompileNamespace} (viewed : compileNamespaceAt? unit handle.namespaceId = some view)
    (outer : Array (TypeId × TypeId)) (typeArgs : Array TypeUse) :
    frameInstantiation unit handle outer typeArgs = frameInstantiationIn view outer typeArgs := by
  simp only [compileNamespaceAt?, Option.map_eq_some_iff] at viewed
  obtain ⟨ns, found, rfl⟩ := viewed
  simp only [frameInstantiation, LeanerIR.SemanticOperations.callTypeInstantiation,
    frameInstantiationIn, CompileNamespace.of, found, Array.map_eq_empty_iff, Array.isEmpty_iff,
    LeanerIR.SemanticOperations.invocationTypeInstantiationIn_eq]

/-- A view's types are its source's, indexed. -/
theorem types_of_view {unit : ValidatedUnit} {namespaceId : NamespaceId} {view : CompileNamespace}
    (viewed : compileNamespaceAt? unit namespaceId = some view) :
    view.types = .ofArray view.source.tables.types := by
  simp only [compileNamespaceAt?, Option.map_eq_some_iff] at viewed
  obtain ⟨ns, _, rfl⟩ := viewed
  rfl

/-- The frame instantiation of a generic call, from a checked certificate: the
callee namespace's types as a literal table, its lifetimes' kinds indexed,
its key map, and the witnesses of this instantiation, all verified by the
kernel by evaluation, never by search. -/
theorem frameInstantiationIn_eq_of_check {view : CompileNamespace} {table : Array Ty}
    {kinds : Validation.IndexedArena LifetimeKind}
    {map : Validation.KeyMap} {witnesses : Array Validation.Witness}
    {tree : Validation.IndexedArena Validation.Witness}
    (typeArgs : Array TypeUse) (nonempty : typeArgs.isEmpty = false)
    (viewTypes : view.types = .ofArray view.source.tables.types)
    (tableEq : view.source.tables.types = table)
    (kindsEq : Validation.IndexedArena.ofArray (Validation.lifetimeKinds view.source) = kinds)
    (correct : Validation.Correct view.types table.size map)
    (treeEq : Validation.IndexedArena.ofArray witnesses = tree)
    (checked : Validation.checkAll kinds view.types map
      (LeanerIR.SemanticOperations.instantiateGenericArguments #[] (typeArgs.map .typeArg))
      tree table.size = true)
    (sizes : witnesses.size = table.size)
    (depths : witnesses.toList.all (fun witness => decide (witness.depth ≤ table.size)) = true) :
    frameInstantiationIn view #[] typeArgs = Validation.foldWitnesses tree table.size := by
  unfold frameInstantiationIn
  rw [nonempty, viewTypes, LeanerIR.SemanticOperations.invocationTypeInstantiationIn_eq]
  simp only [Bool.false_eq_true, ↓reduceIte]
  rw [viewTypes, ← kindsEq] at checked
  rw [viewTypes] at correct
  rw [← tableEq] at checked correct sizes depths ⊢
  rw [← sizes] at checked
  exact Validation.invocationTypeInstantiation_eq_of_check #[] _ treeEq correct checked sizes depths

end LeanerIR.Proofs.Denote
