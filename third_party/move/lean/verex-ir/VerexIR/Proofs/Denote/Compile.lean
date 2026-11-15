-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Term
import VerexIR.Validation.IndexedArena
import VerexIR.Validation.InstantiationCertificate

/-!
# Compilation of validated VIR to typed terms

`compileFunction` is a Lean function of the validated unit: it reads a
function's body out of the expression arena and produces the typed term
that denotes it, or names the construct the denotation does not carry.
It is structural recursion on a fuel, so the kernel unfolds it on a quoted
unit and a per-target certificate is a `rfl`.

Everything it decides is decided on identities and indexes — never on
strings — so that reduction stays cheap.  A rejected construct is a
negative check naming it, never an approximation.
-/

namespace VerexIR.Proofs.Denote

open VerexIR.Validation VerexIR.SemanticOperations

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
      | .readDiscriminant _ => "read of an enum discriminant"
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

/-- The site of the initializer of a pack's `k`-th ghost field, apart from
every other pack's and ghost field's. -/
def ghostInitSite (namespaceIndex expressionIndex k : Nat) : Nat :=
  loopSite (loopSite namespaceIndex expressionIndex) k

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
  carrierDeaths : KeyTree DeadPlaces
  shared : KeyTree Unit
  /-- The unit's struct and enum declarations stating a data invariant or
  holding an intrinsic map, keyed by `loopSite` of namespace and index. -/
  invariants : KeyTree Unit
  /-- The unit's type and expression fuels, and the namespace's place count,
  counted once. -/
  typeFuel : Nat
  fuel : Nat
  placeFuel : Nat
  /-- The type arguments a function is compiled at, where its body is read at a type
  argument holding a mutable reference; `nty` applies them. -/
  substitution : Option NRow := none

/-- Move function-valued fields carry a checked modification frame, empty
unless explicitly declared. Only a function type at the field has one, not an
arbitrary function nested in a vector or supplied as a type argument. -/
def fieldKeepsMemory (ns : ValidatedNamespace) (field : FieldDecl) : Bool :=
  ns.profile == some .move &&
    (ns.typeOf field.type.typeId).any fun ty => ty matches .function ..

/-- A declaration whose fields have closure frames. The compiler's
invariant checkpoints and contract generation use the same criterion. -/
def declarationHasClosureFields (ns : ValidatedNamespace) (declaration : StructDecl) : Bool :=
  declaration.fields.any (fieldKeepsMemory ns) ||
    declaration.variants.any fun variant => variant.fields.any (fieldKeepsMemory ns)

/-- Whether a declaration of a namespace carries a data invariant: a stated
one, a closure field's frame, or the order of an intrinsic map's keys. -/
def declarationCarriesInvariant (ns : ValidatedNamespace) (declaration : StructDecl) : Bool :=
  declarationHasClosureFields ns declaration ||
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
    .ofArray ns.tables.types, deathIndex unit namespaceId, carrierDeathIndex unit namespaceId,
    sharedDereferenceIndex unit namespaceId,
    invariantIndex unit, VerexIR.Proofs.Denote.typeFuel unit, unitFuel unit, ns.places.size + 1,
    none⟩

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
      | some (.unsize base) | some (.downcast base _) => ns.ownedRoot? fuel base
      | _ => none

/-- The local a loan borrows from without a dereference. -/
def CompileNamespace.lender? (ns : CompileNamespace) (loan : CheckedLoanFact) : Option LocalId :=
  match ns.expressions.get? loan.expression.index with
  | some { kind := .operation (.borrow _ place) _ _ _, .. } => ns.ownedRoot? ns.placeFuel place
  | _ => none

/-- The place a loan borrows, where it is a borrow of a place. -/
def CompileNamespace.borrowedPlace? (ns : CompileNamespace) (loan : CheckedLoanFact) :
    Option PlaceId :=
  match ns.expressions.get? loan.expression.index with
  | some { kind := .operation (.borrow _ place) _ _ _, .. } => some place
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
  | some expression => isSharedReferenceType ns.source (ns.source.typeId expression.typeId) ns.types.get?
  | none => false

/-- The native type a type identifier denotes in its own scope, read through the
view's type table for its own namespace: what a callee's signature states. -/
def CompileNamespace.declaredNty (ns : CompileNamespace) (unit : ValidatedUnit)
    (namespaceId : NamespaceId) (typeId : TypeId) : Option NTy :=
  ntyOfFuel (fun owner typeId => if owner == ns.namespaceId then ns.types.get? typeId.index
      else unitTypes unit owner typeId)
    unit .nil ns.typeFuel namespaceId typeId

/-- The native type a type identifier denotes at a substitution. A shared view erases
after the substitution: `&T` at `T := &mut u32` holds a `u32`, in a declaration's fields too. -/
def CompileNamespace.ntyAt (ns : CompileNamespace) (unit : ValidatedUnit) (θ : NRow)
    (namespaceId : NamespaceId) (typeId : TypeId) : Option NTy :=
  ntyOfFuel (fun owner typeId => if owner == ns.namespaceId then ns.types.get? typeId.index
      else unitTypes unit owner typeId)
    unit θ ns.typeFuel namespaceId typeId

/-- The native type a type identifier denotes in the function compiled: at its
substitution, if any. -/
def CompileNamespace.nty (ns : CompileNamespace) (unit : ValidatedUnit)
    (namespaceId : NamespaceId) (typeId : TypeId) : Option NTy :=
  match ns.substitution with
  | none => ns.declaredNty unit namespaceId typeId
  | some θ => ns.ntyAt unit θ namespaceId typeId

/-- The types and the calls of the expressions below a root of a namespace's arena, each
read through the namespace. -/
def frameExpressionReads (source : ValidatedNamespace) : Nat → ExprId →
    Array TypeId × Array (QualifiedRef × Array GenericArgument) →
    Array TypeId × Array (QualifiedRef × Array GenericArgument)
  | 0, _, found => found
  | fuel + 1, id, found => match source.expressions[id.index]? with
    | none => found
    | some expression =>
        let typeArguments := fun (arguments : Array GenericArgument) =>
          (source.genericArguments arguments).filterMap fun argument => match argument with
            | .typeArg value => some value.typeId
            | _ => none
        let found := (found.1.push (source.typeId expression.typeId), found.2)
        let found := match expression.kind with
          | .operation (.call (.function reference)) instantiations _ _
          | .operation (.call (.closure reference _)) instantiations _ _ =>
              (found.1 ++ typeArguments instantiations,
                found.2.push (reference, source.genericArguments instantiations))
          | .operation _ instantiations _ _ => (found.1 ++ typeArguments instantiations, found.2)
          | _ => found
        (expressionChildren expression.kind).foldl (init := found) fun found child =>
          frameExpressionReads source fuel child found

/-- What a function's reading depends on: the types of its signature, its locals, its contract
and its body, and the calls of the last two. None of a function with contracts at shapes, whose
generics are the shapes' own. -/
def functionFrameReads (unit : ValidatedUnit) (handle : FunctionHandle) :
    Option (Array TypeId × Array (QualifiedRef × Array GenericArgument)) := do
  let source ← unit.namespaces[handle.namespaceId.index]?
  let declaration ← source.functions[handle.functionId.index]?
  guard declaration.shapes.isEmpty
  let contract := declaration.contract
  let declared := (declaration.signature.parameters.map (·.typeUse.typeId)) ++
    (declaration.signature.results.map (·.typeId)) ++ (declaration.locals.map (·.type.typeId)) ++
    (contract.reads.map (·.typeId))
  let conditions := contract.conditions.foldl (init := contract.modifies) fun roots condition =>
    condition.auxiliary.foldl (fun roots auxiliary => roots.push auxiliary.2)
      (roots.push condition.expression)
  let conditions := contract.parameterFrames.foldl (init := conditions) fun roots frame =>
    roots ++ frame.modifies
  let roots := match declaration.body with
    | .structured root => conditions.push root
    | .absent => conditions
  return roots.foldl (init := (declared.map source.typeId, #[])) fun found root =>
    frameExpressionReads source (source.expressions.size + 1) root found

/-- Whether reading a type at `θ` agrees with substituting `θ` in its declared reading. -/
def CompileNamespace.readingAgrees (ns : CompileNamespace) (unit : ValidatedUnit) (θ : NRow)
    (namespaceId : NamespaceId) (typeId : TypeId) : Bool :=
  decide (ns.ntyAt unit θ namespaceId typeId =
    (NTy.subst θ) <$> ns.declaredNty unit namespaceId typeId)

/-- Unit in a type binder can erase a generic callable result slot. Lifetime and
constant binders also use Unit placeholders, but no value type reads those slots.
Missing declaration/binder information is conservatively treated as a real argument. -/
def hasUnitTypeArgument (unit : ValidatedUnit) (handle : FunctionHandle) (θ : NRow) : Bool :=
  θ.toList.zipIdx.any fun (argument, index) =>
    argument == .unit &&
      match unit.namespaces[handle.namespaceId.index]? >>= fun source =>
          source.functions[handle.functionId.index]? >>= fun declaration =>
            declaration.signature.generics[index]? with
      | some binder => binder.kind == .typeArg
      | none => true

/-- Whether the reading of each function pending at its type arguments, and of what it calls at
theirs, agrees with their substitution. Mutable references and Unit arguments need this
check: substituting Unit in a generic callable result row does not erase its slot, whereas
reading that callable at Unit does. Running out of fuel counts as disagreeing. -/
def CompileNamespace.framesAgree (ns : CompileNamespace) (unit : ValidatedUnit) :
    Nat → List (FunctionHandle × NRow) → List (FunctionHandle × NRow) → Bool
  | 0, _, _ => false
  | _ + 1, [], _ => true
  | fuel + 1, (handle, θ) :: rest, seen =>
      if seen.any (fun other => other.1 == handle && decide (other.2 = θ)) then
        ns.framesAgree unit fuel rest seen
      else match functionFrameReads unit handle with
      | none => false
      | some (types, calls) =>
          let next : Option (List (FunctionHandle × NRow)) :=
            calls.toList.mapM fun (reference, arguments) => do
              let callee ← resolveFunction? unit handle.namespaceId reference
              let inner ← arguments.toList.mapM fun (argument : GenericArgument) =>
                match argument with
                | .typeArg value => ns.ntyAt unit θ handle.namespaceId value.typeId
                | .lifetime _ | .const _ => some NTy.unit
                | .evidence _ => none
              return (callee, NRow.ofList inner)
          match next with
          | none => false
          | some next =>
              types.all (ns.readingAgrees unit θ handle.namespaceId) &&
                ns.framesAgree unit fuel
                  (next.filter (fun call => !call.2.refAbsent || hasUnitTypeArgument unit call.1 call.2) ++ rest)
                  ((handle, θ) :: seen)

/-- Whether a function's generic contract or theorem transfers to type arguments: their values
hold no loan, and reading the function at them agrees with substituting them, over everything its
reading depends on. A call that does is `Term.callGeneric`, any other `Term.callInstance`. -/
def CompileNamespace.transfers (ns : CompileNamespace) (unit : ValidatedUnit)
    (handle : FunctionHandle) (θ : NRow) : Bool :=
  θ.refFree && ((θ.refAbsent && !hasUnitTypeArgument unit handle θ) ||
    ns.framesAgree unit (unitFuel unit) [(handle, θ)] [])

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
      if equal : τ' = τ then .ok (equal ▸ term)
      else .error s!"an expression of type {repr τ'} is used at type {repr τ}"
  | .never term => .ok (term τ)

/-- A compiled expression at its own type, taking unit for one of every type. -/
def Compiled.some {ρ : ResultShape} {Γ : NRow} : Compiled unit ρ Γ → Σ τ : NTy, Term unit ρ Γ τ
  | .at τ term => ⟨τ, term⟩
  | .never term => ⟨.unit, term .unit⟩

/-- A copy of a mutable reference's current value: the term that owes the
data invariants of the referent at `site` where its type carries a
certificate, and the one that owes nothing where it carries none, which
the compiler decides of the type (`Term.copy`,
`doc/designs/data-invariants.md`). -/
def copyOut {ρ : ResultShape} {Γ : NRow} {τ : NTy} (site : Nat)
    (value : Term unit ρ Γ (.ref τ)) : Term unit ρ Γ τ :=
  if free : τ.certificateFree = true then .copy value free else .deref site value

/-- A read of the component a path reaches: through a dereference, the
copy that owes the component's data invariants at `site` where its type
carries a certificate and the one that owes nothing where it carries none;
without a dereference, the certified read, which owes nothing either way
(`Term.copyPlace`). -/
def readOut {ρ : ResultShape} {Γ : NRow} {τ σ : NTy} (site : Nat) (mismatch : Option Failure)
    (x : Var Γ τ) (path : Proj τ σ) : Term unit ρ Γ σ :=
  if free : σ.certificateFree = true then
    if path.hasDeref then .copyPlace mismatch x path free
    else .readPlace site mismatch x path
  else .readPlace site mismatch x path

/-- Forget a shared slice's runtime type only while reading its contents. -/
def Compiled.sequenceContents {ρ : ResultShape} {Γ : NRow} : Compiled unit ρ Γ → Compiled unit ρ Γ
  | .at (.slice element) value => .at (.vector element) (.sliceVector value)
  | value => value

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
  | .num, .integer value => .ok ⟨value, rfl⟩
  | .bool, .bool value => .ok value
  | .unit, .unit => .ok ()
  | .address, .address value => .ok value
  | .string, .string value => .ok value
  | .bytes, .bytes value => .ok value
  | .char, .character value =>
      if valid : isUnicodeScalar value = true then .ok ⟨value, valid⟩
      else .error "a character literal is not a Unicode scalar value"
  | .tuple elements, .tuple values => literalRow elements values.toList
  | .slice element, .vector values
  | .vector element, .vector values => do
      let values := (← values.toList.mapM (literalValue element)).toArray
      if bounded : values.size < 2 ^ 64 then .ok ⟨values, bounded⟩
      else .error "a vector literal is out of range"
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
  | _, _, _, .element x path, rest => .element x (path.append rest)
  | _, _, _, .sliceElement position path, rest => .sliceElement position (path.append rest)
  | _, _, _, .sliceIndex position path, rest => .sliceIndex position (path.append rest)
  | _, _, _, .arrayIndex position path, rest => .arrayIndex position (path.append rest)
  | _, _, _, .index position path, rest => .index position (path.append rest)
  | _, _, _, .variant choices path, rest => .variant choices (path.append rest)
  | _, _, _, .ghost path, rest => .ghost (path.append rest)

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
  | _, _, .deref path | _, _, .field _ path | _, _, .element _ path | _, _, .index _ path
  | _, _, .arrayIndex _ path | _, _, .sliceIndex _ path | _, _, .sliceElement _ path
  | _, _, .ghost path => path.mayMismatch
  | _, _, .variant choices path => choices.partial || path.mayMismatch

/-- The failure a pattern mismatch makes (`patternMismatch?`), its codes the
throw's arguments, as the runtime throws it (`patternMismatchThrow?`). -/
def mismatchFailure? (profile : Option Profile) : Option Failure :=
  (patternMismatch? profile).map fun (kind, codes) => Failure.ofInts kind codes

/-- The throw an access along a path makes where a variant lacks the field:
the profile's mismatch throw, for a path that may meet one. -/
def CompileNamespace.mismatchOn (ns : CompileNamespace) {τ σ : NTy} (path : Proj τ σ) :
    Option Failure :=
  if path.mayMismatch then mismatchFailure? ns.source.profile else none

/-- The slot of a named ghost field in each listed variant of a declaration. -/
def ghostChoices? (unit : ValidatedUnit) (handle : StructHandle) (variants : List String)
    (field : String) : Option (List (String × Nat)) :=
  variants.mapM fun variant => (handleGhostIndex? unit handle (some variant) field).map (variant, ·)

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
              let some handle := resolveStruct? unit ns.source.unitIdentity owner
                | .error "a field place does not resolve"
              unless handle = source do .error "a field place's declaration differs from its type"
              let some name := sourceFieldName? ns.source field | .error "a field place has no name"
              let some index := handleFieldIndex? unit handle none name
                | .error "a field place names an unknown field"
              let some ⟨σ, position⟩ := Var.ofIndex fields index
                | .error "a field place is out of range"
              .ok ⟨root, σ, x, path.append (.field position .nil)⟩
          | .enum source _ names rows _, path =>
              let some handle := resolveStruct? unit ns.source.unitIdentity owner
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
      | .ghostField base owner field =>
          -- A ghost field below a downcast is the named variant's slot.
          let (base, variant?) := match ns.place? base with
            | some (.downcast inner variant) => (inner, some variant)
            | _ => (base, none)
          let ⟨root, component, x, path⟩ ← compilePlace unit Γ ns fuel base
          let some handle := resolveStruct? unit ns.source.unitIdentity owner
            | .error "a ghost field place does not resolve"
          let some name := sourceFieldName? ns.source field
            | .error "a ghost field place has no name"
          match component, path with
          | .struct source _ fields, path =>
              unless handle = source do
                .error "a ghost field place's declaration differs from its type"
              let some index := handleGhostIndex? unit handle none name
                | .error "a ghost field place names an unknown ghost field"
              let some ⟨σ, position⟩ := Var.ofIndex fields index
                | .error "a ghost field place is out of range"
              match σ, position with
              | .ghost slot, position =>
                  .ok ⟨root, slot, x, path.append (.field position (.ghost .nil))⟩
              | _, _ => .error "a ghost field place is not a ghost slot"
          | .enum source _ names rows _, path =>
              unless handle = source do
                .error "a ghost field place's declaration differs from its type"
              let listed ← match variant? with
                | none => .ok names
                | some variant => do
                    let some variantName := sourceFieldName? ns.source variant
                      | .error "a downcast place has no variant name"
                    .ok [variantName]
              let some choices := ghostChoices? unit handle listed name
                | .error "a ghost field place names an unknown ghost field"
              let (firstVariant, firstIndex) :: _ := choices
                | .error "a ghost field place names no variant"
              let some ⟨σs, _⟩ := Which.ofName names rows firstVariant
                | .error "a ghost field place names an unknown variant"
              let some ⟨σ, _⟩ := Var.ofIndex σs firstIndex
                | .error "a ghost field place is out of range"
              match σ with
              | .ghost slot => do
                  let choices ← Choices.ofList names rows (.ghost slot) choices
                  .ok ⟨root, slot, x, path.append (.variant choices (.ghost .nil))⟩
              | _ => .error "a ghost field place is not a ghost slot"
          | _, _ => notCarried "a ghost field place on a non-nominal value"
      | .index base indexExpr =>
          let ⟨root, component, x, path⟩ ← match ns.place? base with
            | some (.deref holder) => do
                let held ← compilePlace unit Γ ns fuel holder
                if held.component matches .sliceRef _ then pure held
                else compilePlace unit Γ ns fuel base
            | _ => compilePlace unit Γ ns fuel base
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
          | .slice σ, path => .ok ⟨root, σ, x, path.append (.sliceElement position .nil)⟩
          | .sliceRef σ, path => .ok ⟨root, σ, x, path.append (.sliceIndex position .nil)⟩
          | .vector σ, path => .ok ⟨root, σ, x, path.append (.index position .nil)⟩
          | .array σ _, path => .ok ⟨root, σ, x, path.append (.arrayIndex position .nil)⟩
          | .tuple elements, path =>
              let .literal index := position
                | notCarried "a tuple element at a position that is not a literal"
              let some ⟨σ, element⟩ := Var.ofIndex elements index
                | .error "a tuple element place is out of range"
              .ok ⟨root, σ, x, path.append (.element element .nil)⟩
          | _, _ => notCarried "an element place on a non-vector"
      | _ => notCarried "a place of this kind"

/-- Whether a value holds a mutable reference, so a read of its local as a
value moves it out of the slot, which then resolves nothing. -/
def NTy.movesOut : NTy → Bool
  | .ref _ | .sliceRef _ => true
  | .tuple elements => !elements.refFree
  | .struct _ _ fields => !fields.refFree
  | .enum _ _ _ rows _ => !rows.refFree
  | .vector element | .slice element | .array element _ => !element.refFree
  | .table key value => !key.refFree || !value.refFree
  | .ghost slot => !slot.refFree
  | .nominal _ arguments _ => !arguments.refFree
  | .observed referent => referent.movesOut
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char | .param _
  | .function _ _ _ => false

/-- Whether the components of a tuple hold references only at their top. -/
def NRow.componentsLendable : NRow → Bool
  | .nil => true
  | .cons (.sliceRef element) rest => element.refFree && rest.componentsLendable
  | .cons (.ref referent) rest => referent.refFree && rest.componentsLendable
  | .cons τ rest => τ.refFree && rest.componentsLendable

/-- Whether a function value's parameter takes its references where an
invocation reads them: at its top, or as components of a tuple. -/
def NTy.lendableAtTop : NTy → Bool
  | .sliceRef element => element.refFree
  | .ref referent => referent.refFree
  | .tuple elements => elements.componentsLendable
  | τ => τ.refFree

mutual
  /-- Whether a value's references are where the prophetic meaning lends them:
  behind references, and in tuples, structs and enums' variants. -/
  def NTy.lendable : NTy → Bool
    | .sliceRef element => element.refFree
    | .ref referent => referent.lendable
    | .tuple elements => NRow.lendable elements
    | .struct _ _ fields => NRow.lendable fields
    | .enum _ _ _ rows _ => NRows.lendable rows
    | τ => τ.refFree

  /-- Whether the references of a row of values are where they are lent. -/
  def NRow.lendable : NRow → Bool
    | .nil => true
    | .cons τ rest => τ.lendable && NRow.lendable rest

  /-- Whether the references of an enum's variants are where they are lent. -/
  def NRows.lendable : NRows → Bool
    | .nil => true
    | .cons fields rest => NRow.lendable fields && NRows.lendable rest
end

/-- Whether a result's references are lendable. -/
def ResultShape.lendable : ResultShape → Bool
  | .none => true
  | .one τ => τ.lendable

/-- Whether a function value's result takes its references where an invocation
reads them. -/
def ResultShape.lendableAtTop : ResultShape → Bool
  | .none => true
  | .one τ => τ.lendableAtTop

/-- The loan facts of a function's borrow certificate, looked up once per
compiled function: the kernel evaluates the compilation, so a loan death
reads its loan from these rather than searching every certificate. -/
def loanFactsOf (unit : ValidatedUnit) (handle : FunctionHandle) : Array CheckedLoanFact :=
  ((unit.namespaces[handle.namespaceId.index]?).map (·.loanFacts handle.functionId)).getD #[]

/-- Whether the borrow analysis found a write to an element index under a live
mutable loan in this function (`BorrowCertificate.writesIndexUnderLoan`). -/
def writesIndexUnderLoan (unit : ValidatedUnit) (handle : FunctionHandle) : Bool :=
  ((unit.namespaces[handle.namespaceId.index]?).bind fun ns =>
    (ns.borrowCertificates.toList.find? (·.functionId == handle.functionId)).map
      (·.writesIndexUnderLoan)).getD false

/-- The lender a dying loan writes back to: the local a borrow of a place is
rooted in and the path it names, so that the owner rebuilt with the final
current is certified at the death (`Lender`).  A borrow of global memory
names the slot, and so does a place the compiler cannot resolve at the
referent's type, where the current is certified instead. -/
def lenderOf (unit : ValidatedUnit) (Γ : NRow) (ns : CompileNamespace) (loan : CheckedLoanFact)
    (referent : NTy) : Except String (Lender Γ referent) := do
  if ns.writesGlobal loan then return .slot
  let some place := ns.borrowedPlace? loan | return .slot
  let ⟨_, component, x, path⟩ ← compilePlace unit Γ ns ns.placeFuel place
  if equal : component = referent then return .place x (equal ▸ path) else return .slot

/-- One step of a path, comparable: two borrows of the same place have the
same steps however their expressions encode the place. -/
inductive ProjStep where
  | deref
  | field (index : Nat)
  | index (position : PlaceIndex)
  | ghost
  deriving DecidableEq, Repr, Inhabited

/-- A path as its steps, `none` where a step is one this comparison does not
read exactly (a variant's choices).  An unread step makes the identity
incomparable rather than equal to another, so two lenders are never merged on
a step that was not compared. -/
def Proj.steps? : {τ σ : NTy} → Proj τ σ → Option (List ProjStep)
  | _, _, .nil => some []
  | _, _, .deref rest => (ProjStep.deref :: ·) <$> rest.steps?
  | _, _, .field x rest => (ProjStep.field x.index :: ·) <$> rest.steps?
  | _, _, .index position rest => (ProjStep.index position :: ·) <$> rest.steps?
  | _, _, .variant _ _ => none
  | _, _, .ghost rest => (ProjStep.ghost :: ·) <$> rest.steps?
  -- A tuple's element and a sequence's position are exact steps: the root's
  -- type fixes which kind of value each step selects from.
  | _, _, .element x rest => (ProjStep.field x.index :: ·) <$> rest.steps?
  | _, _, .arrayIndex position rest => (ProjStep.index position :: ·) <$> rest.steps?
  | _, _, .sliceElement position rest => (ProjStep.index position :: ·) <$> rest.steps?
  | _, _, .sliceIndex position rest => (ProjStep.index position :: ·) <$> rest.steps?

/-- Which lender a loan writes back to, canonically: the slot, or the local a
borrow is rooted in with the full projection it names, index operands
included.  A `PlaceId` is the identity of a borrow *expression*, so two
borrows of one place carry different ones; resolving the place first is what
makes them compare equal.  A path this comparison cannot read exactly keeps
its place's identity, so it is merged with nothing. -/
inductive LenderIdentity where
  | slot
  | place (root : Nat) (steps : List ProjStep)
  | incomparable (place : PlaceId)
  deriving DecidableEq

/-- The canonical identity of the lender `lenderOf` builds for this loan at
this referent; it answers `slot` exactly where `lenderOf` does. -/
def lenderIdentity (unit : ValidatedUnit) (Γ : NRow) (ns : CompileNamespace)
    (loan : CheckedLoanFact) (referent : NTy) : Except String LenderIdentity := do
  if ns.writesGlobal loan then return .slot
  let some place := ns.borrowedPlace? loan | return .slot
  let ⟨_, component, x, path⟩ ← compilePlace unit Γ ns ns.placeFuel place
  unless component = referent do return .slot
  match path.steps? with
  | some steps => return .place x.index steps
  | none => return .incomparable place

/-- A value leaving the function as its result, with the data invariants of
a mutable reference's referent certified where it leaves (`Term.returned`).

Every normal exit is certified, not only the one that falls off the end: an
explicit `return` leaves through `Flow.return_`, which `Flow.bind` passes on
without running a continuation, so the certification is stated on the
returned value itself, before the control outcome is built.  The case is
decided here, over the result shape the compiler knows, so the denotation
keeps no case on it.  A certificate-free referent, and a result that is not a
mutable reference, owe nothing. -/
def certifiedResult {Γ : NRow} (site : Nat) :
    {shape : ResultShape} → Term unit shape Γ shape.bodyType →
      Term unit shape Γ shape.bodyType
  | .one (.ref σ), value => if σ.certificateFree then value else .returned site value
  | _, value => value

/-- The typed path certificate steps name into a value of a type. -/
def holderProj (unit : ValidatedUnit) (ns : CompileNamespace) :
    (τ : NTy) → List LoanProjection → Except String ((σ : NTy) × Proj τ σ)
  | τ, [] => .ok ⟨τ, .nil⟩
  | .ref referent, .dereference :: rest => do
      let ⟨target, path⟩ ← holderProj unit ns referent rest
      .ok ⟨target, .deref path⟩
  | .sliceRef element, .dereference :: .index (some position) :: rest => do
      let ⟨target, path⟩ ← holderProj unit ns element rest
      .ok ⟨target, .sliceIndex (.literal position) path⟩
  | .slice element, .index (some position) :: rest => do
      let ⟨target, path⟩ ← holderProj unit ns element rest
      .ok ⟨target, .sliceElement (.literal position) path⟩
  | .vector element, .index (some position) :: rest => do
      let ⟨target, path⟩ ← holderProj unit ns element rest
      .ok ⟨target, .index (.literal position) path⟩
  | .array element _, .index (some position) :: rest => do
      let ⟨target, path⟩ ← holderProj unit ns element rest
      .ok ⟨target, .arrayIndex (.literal position) path⟩
  | .tuple elements, .index (some position) :: rest => do
      let some ⟨σ, x⟩ := Var.ofIndex elements position
        | .error "a holder's path names a tuple element out of range"
      let ⟨target, path⟩ ← holderProj unit ns σ rest
      .ok ⟨target, .element x path⟩
  | .struct source _ fields, .field name :: rest => do
      let some fieldName := sourceFieldName? ns.source name
        | .error "a holder's path names a field without a name"
      let some index := handleFieldIndex? unit source none fieldName
        | .error "a holder's path names an unknown field"
      let some ⟨σ, x⟩ := Var.ofIndex fields index
        | .error "a holder's path names a field out of range"
      let ⟨target, path⟩ ← holderProj unit ns σ rest
      .ok ⟨target, .field x path⟩
  | .enum source _ names rows _, .downcast variant :: .field name :: rest => do
      let some variantName := sourceFieldName? ns.source variant
        | .error "a holder's path names a variant without a name"
      let some fieldName := sourceFieldName? ns.source name
        | .error "a holder's path names a field without a name"
      let some index := handleFieldIndex? unit source (some variantName) fieldName
        | .error "a holder's path names an unknown field"
      let some ⟨σs, which⟩ := Which.ofName names rows variantName
        | .error "a holder's path names an unknown variant"
      let some ⟨σ, x⟩ := Var.ofIndex σs index
        | .error "a holder's path names a field out of range"
      let ⟨target, path⟩ ← holderProj unit ns σ rest
      .ok ⟨target, .variant (.cons which x .nil) path⟩
  | _, _ => notCarried "a loan held inside an aggregate"

/-- The deaths of a carrier at the paths an aggregate holder holds it. A
carrier names no loan of this function, so no lender is known and each
reference resolves at the slot. A path to a component without references
observes the loan through an erased shared borrow and resolves nothing. -/
def holderDeaths (unit : ValidatedUnit) (ns : CompileNamespace) (ρ : ResultShape) {Γ : NRow}
    {τ : NTy} (site : Nat) (x : Var Γ τ) (paths : List (Array LoanProjection)) :
    Except String (List (Term unit ρ Γ .unit)) := do
  if paths.isEmpty then notCarried "a loan held inside an aggregate"
  paths.filterMapM fun steps => do
    let ⟨target, path⟩ ← holderProj unit ns τ steps.toList
    match target, path with
    | .sliceRef _, path => .ok (some (Term.resolveSliceAt x path))
    | .ref _, path => .ok (some (Term.resolveAt site x path .slot))
    | target, _ =>
        if target.refFree then .ok none else notCarried "a loan held inside an aggregate"

/-- How a place holding references resolves a loan ending here: the
obligations of every loan it may hold, then one prophecy equation.

A reference joined from borrows of different owners keeps no record of the
owner it came from, so writing this holder's current back into a lender it
need not hold is unsound: it would constrain a foreign owner, and together
with that owner's own holder's resolve it makes the state contradictory.
The equation is the same relation for every one of those lenders — a borrow
constrains the owner it sets to agree with the reference's prophecy at the
path (`Term.borrowPlace`) — so `Lender.equation` states it once.

The `origins` are every loan that may sit at this place, whichever loan's
death this is; each owes its own obligation here — the referent's invariants
for a loan no local lends, the owner rebuilt for a borrow of a local place,
nothing for a reborrow through a dereference, as that lender owes nothing
anywhere. Only where the place is `certain` to hold the dying loan — the
loan's death names this place alone — and may hold no other lender is the
precise resolve of that lender stated, write-back and all. `resolve` states a
lender's resolution at the place: at a local, or at a path within an
aggregate (`Term.resolve`, `Term.resolveAt`). An ancestor loan naming the
place is treated as a possible origin and so may be asked for an obligation
it does not owe: sound, and a known over-approximation. -/
def holderResolves (unit : ValidatedUnit) (Γ : NRow) (ns : CompileNamespace)
    (ρ : ResultShape) (origins : List CheckedLoanFact) (certain : Bool) {referent : NTy}
    (resolve : Lender Γ referent → Term unit ρ Γ .unit) :
    Except String (Term unit ρ Γ .unit) := do
  -- Each lender once, canonically, so two borrows of one place are one lender.
  let keyed ← origins.mapM fun loan => do
    return (← lenderIdentity unit Γ ns loan referent, loan)
  let distinct := keyed.foldl (init := ([] : List (LenderIdentity × CheckedLoanFact)))
    fun seen entry => if seen.any (·.1 == entry.1) then seen else seen ++ [entry]
  match distinct, certain with
  | [(_, only)], true => return resolve (← lenderOf unit Γ ns only referent)
  | _, _ => do
      let obligations ← distinct.filterMapM fun (_, loan) => do
        match ← lenderOf unit Γ ns loan referent with
        | .slot => .ok (some (resolve .referent))
        | .place owner path =>
            if path.hasDeref then .ok none else .ok (some (resolve (.candidate owner path)))
        -- `lenderOf` builds only the two lenders above.
        | .candidate .. | .referent | .equation => .ok none
      -- The obligations first, the equation last: a violation must be undefined
      -- before an equation could prune the run (`Lender.resolve`).
      return obligations.foldr (fun effect rest => Term.drop effect rest) (resolve .equation)

/-- The effect of ending loans, `none` when it resolves nothing. A loan's
death resolves the reference wherever it is held, at its path in an
aggregate; a holder whose slot is empty passed it on, and a loan consumed by
a call has no holder here, since the callee resolves it.  A holder without
references observes the loan through an erased shared borrow and resolves
nothing.  Each place resolves once per death, not once per dying loan
(`holderResolves`). -/
def deathEffect (unit : ValidatedUnit) (ns : CompileNamespace)
    (loanFacts : Array CheckedLoanFact) (ρ : ResultShape) (Γ : NRow) (anchor : ExprId)
    (before : Bool) (ended : Array LoanId) :
    Except String (Option (Term unit ρ Γ .unit)) := do
  -- Innermost first, as `settleLoans` ends them.
  let dying ← ended.toList.reverse.mapM fun loan => do
    let some fact := loanFacts[loan.index]?
      | .error "a loan death names an unknown loan"
    .ok (fact, fact.deaths.toList.filter fun death =>
      death.anchor == anchor && death.before == before)
  -- The places the dying loans are held at: a holder, and the path to the
  -- reference within it where the holder is an aggregate. Each once, at the
  -- first death that reaches it, with whether that death names it alone.
  let places := dying.foldl
    (init := ([] : List (LocalId × Option (Array LoanProjection) × CheckedLoanFact × Bool)))
    fun seen (fact, records) =>
      let holders := records.foldl (init := ([] : List LocalId)) fun holders death =>
        death.holders.toList.foldl (fun holders holder =>
          if holders.contains holder then holders else holders ++ [holder]) holders
      holders.foldl (init := seen) fun seen holder =>
        let paths := (records.flatMap fun death =>
          (death.holderPaths.toList.filter (·.1 == holder)).map (·.2)).eraseDups
        let alone := holders.length == 1
        let entries := if paths.isEmpty then [(holder, none, fact, alone)]
          else paths.map fun path => (holder, some path, fact, alone && paths.length == 1)
        entries.foldl (init := seen) fun seen entry =>
          if seen.any fun (holder, path, _, _) => holder == entry.1 && path == entry.2.1 then seen
          else seen ++ [entry]
  -- The loans that may sit at a place: those a death names it at.
  let originsAt (holder : LocalId) (path? : Option (Array LoanProjection)) :=
    loanFacts.toList.filter fun loan => loan.deaths.any fun death =>
      match path? with
      | none => death.holders.contains holder
      | some path => death.holderPaths.contains (holder, path)
  let deaths ← places.filterMapM fun (holder, path?, fact, certain) => do
    let some ⟨σ, x⟩ := Var.ofIndex Γ holder.index | .error "a local is out of range"
    let site := loopSite ns.namespaceId.index fact.expression.index
    let origins := originsAt holder path?
    match path? with
    | some steps =>
        let ⟨target, path⟩ ← holderProj unit ns σ steps.toList
        match target, path with
        | .sliceRef _, path => .ok (some (Term.resolveSliceAt (ρ := ρ) x path))
        | .ref _, path =>
            .ok (some (← holderResolves unit Γ ns ρ origins certain
              (Term.resolveAt (ρ := ρ) site x path)))
        | target, _ =>
            if target.refFree then .ok none else notCarried "a loan held inside an aggregate"
    | none =>
        match σ, x with
        | .sliceRef _, x => .ok (some (Term.resolveSliceAt (ρ := ρ) x .nil))
        | .ref _, x =>
            .ok (some (← holderResolves unit Γ ns ρ origins certain
              (Term.resolve (ρ := ρ) site x)))
        | σ, _ => if σ.refFree then .ok none else notCarried "a loan held inside an aggregate"
  -- Once resolved, a local a loan mutated owes its invariant: a mutation
  -- ends where its loan dies. A borrow of global memory ends its write, and
  -- its referent owes its invariant through the reference, since no local
  -- lends it. Both follow the resolution, which ties the prophecy memory
  -- holds to the value the body wrote.
  let checks ← ended.toList.reverse.mapM fun loan => do
    let some fact := loanFacts[loan.index]? | .error "a loan death names an unknown loan"
    -- The holders are each death's (`LoanDeath.holders`): a place holding the
    -- loan elsewhere resolves at its own death, not at this one.
    let records := fact.deaths.toList.filter fun death =>
      death.anchor == anchor && death.before == before && !death.alone
    if records.isEmpty then return []
    let site := loopSite ns.namespaceId.index fact.expression.index
    let written := Term.memoryWritten (ρ := ρ) (Γ := Γ) site
    let mutated := Term.mutationEnd (ρ := ρ) (Γ := Γ) site
    if ns.writesGlobal fact then
      let holders := records.foldl (init := []) fun holders death =>
        death.holders.toList.foldl (fun holders holder =>
          if holders.contains holder then holders else holders ++ [holder]) holders
      let referents ← holders.filterMapM fun holder => do
        let some ⟨σ, _⟩ := Var.ofIndex Γ holder.index | .error "a local is out of range"
        match σ with
        | .ref referent => .ok (if ns.carriesInvariant referent then some referent else none)
        | _ => .ok none
      return if referents.isEmpty then [written] else [mutated, written]
    let some lender := ns.lender? fact | .ok []
    let some ⟨σ, _⟩ := Var.ofIndex Γ lender.index | .error "a local is out of range"
    if ns.carriesInvariant σ then .ok [mutated] else .ok []
  let effects := deaths ++ checks.flatten
  if effects.isEmpty then .ok none
  else .ok (some (effects.foldr (fun effect rest => Term.drop effect rest) (.lit () rfl)))

/-- A node's term with the loan deaths anchored at it: those before it
resolve first, those after it once it produced its value. -/
def withDeaths (ns : CompileNamespace) (loanFacts : Array CheckedLoanFact) (id : ExprId)
    (node : Compiled unit ρ Γ) : Except String (Compiled unit ρ Γ) := do
  let deaths? := ns.deaths.find? id.index
  let carriers? := ns.carrierDeaths.find? id.index
  if deaths?.isNone && carriers?.isNone then return node
  let deaths := deaths?.getD {}
  let carriers := carriers?.getD {}
  let effect (before : Bool) (loans : Array LoanId)
      (places : Array (LocalId × List LoanProjection)) := do
    let loanEffect ← deathEffect unit ns loanFacts ρ Γ id before loans
    if places.isEmpty then return loanEffect
    let carrierEffects ← places.toList.flatMapM fun (holder, path) => do
      let some ⟨_, x⟩ := Var.ofIndex Γ holder.index | .error "a local is out of range"
      holderDeaths unit ns ρ (loopSite ns.namespaceId.index id.index) x [path.toArray]
    let effects := loanEffect.toList ++ carrierEffects
    if effects.isEmpty then .ok none
    else .ok (some (effects.foldr (fun effect rest => Term.drop effect rest) (.lit () rfl)))
  let node ← match ← effect false deaths.after carriers.after, node with
    | some effect, .at τ term => .ok (.at τ (.seqAfter term effect))
    | _, node => .ok node
  match ← effect true deaths.before carriers.before with
  | some effect => .ok (node.map fun _ term => .drop effect term)
  | none => .ok node

/-- Whether a unit declares a ghost field or a spec variable's memory, which
belong to the verification model (`doc/designs/ghost-fields.md`). -/
def declaresGhosts (unit : ValidatedUnit) : Bool :=
  unit.namespaces.any fun ns => ns.structs.any fun declaration =>
    !declaration.ghosts.isEmpty || declaration.isGhostMemory

/-- Whether `==` at a type compares the values' runtime encodings with
ghost contents dropped: in a unit declaring ghosts, at a type mentioning a
type parameter (`doc/designs/ghost-fields.md`). -/
def comparesErased (unit : ValidatedUnit) (τ : NTy) : Bool :=
  declaresGhosts unit && !τ.paramsAll (fun _ => false)

/-- The literal of a constant at its declared type. -/
def compileLiteral {ρ : ResultShape} {Γ : NRow} (τ : NTy) (literal : ConstValue) :
    Except String (Term unit ρ Γ τ) :=
  match held : τ.literal with
  | true => (.lit · held) <$> literalValue τ literal
  | false => notCarried "a literal of this type"

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

/-- Whether a type is logical only: it types specification values, never
executable ones, so no body reads a local of it. -/
def logicalOnly (unit : ValidatedUnit) (namespaceId : NamespaceId) (typeId : TypeId) : Bool :=
  match unit.namespaces[namespaceId.index]?.bind (·.tables.types[typeId.index]?) with
  | some (.integer .unbounded _) | some .range | some (.typeDomain _)
  | some (.resourceDomain ..) | some .stateDomain => true
  | _ => false

/-- Whether a type reaches the declaration `target`, through type arguments and
declared fields. -/
private def typeReachesHandle (unit : ValidatedUnit) (target : StructHandle) :
    Nat → List StructHandle → NamespaceId → TypeId → Bool
  | 0, _, _, _ => false
  | depth + 1, visited, nsId, tid =>
      match unitTypes unit nsId tid with
      | some (.nominal name arguments) =>
          let inArgs := arguments.any fun arg => match arg with
            | .typeArg value =>
                typeReachesHandle unit target depth visited nsId value.typeId
            | _ => false
          if inArgs then true
          else
            match unit.namespaces[nsId.index]?.bind (fun ns => resolveNominal? unit ns name) with
            | some handle =>
                handle == target ||
                if visited.contains handle then false
                else
                  (unit.namespaces[handle.namespaceId.index]?).any fun owner =>
                    (owner.structs[handle.structId]?).any fun decl =>
                      let allFields := decl.fields.toList ++
                        decl.variants.toList.flatMap (·.fields.toList)
                      allFields.any fun field =>
                        typeReachesHandle unit target depth (handle :: visited)
                          handle.namespaceId (owner.typeId field.type.typeId)
            | none => false
      | some (.reference ref) =>
          typeReachesHandle unit target depth visited nsId ref.referent
      | some (.tuple elements) =>
          elements.toList.any (typeReachesHandle unit target depth visited nsId)
      | some (.vector elem _) | some (.slice elem) =>
          typeReachesHandle unit target depth visited nsId elem
      | _ => false

/-- Whether a field of the declaration reaches the declaration again. -/
private def isNominalRecursive (unit : ValidatedUnit) (handle : StructHandle) (depth : Nat) : Bool :=
  (unit.namespaces[handle.namespaceId.index]?).any fun owner =>
    (owner.structs[handle.structId]?).any fun decl =>
      let allFields := decl.fields.toList ++ decl.variants.toList.flatMap (·.fields.toList)
      allFields.any fun field =>
        typeReachesHandle unit handle depth [handle] handle.namespaceId
          (owner.typeId field.type.typeId)

/-- The recursive nominal type a type holds, for the rejection's message. -/
private def nominalIn? (unit : ValidatedUnit) :
    Nat → NamespaceId → TypeId → Option String
  | 0, _, _ => none
  | depth + 1, nsId, tid =>
      match unitTypes unit nsId tid with
      | some (.nominal name _) =>
          unit.namespaces[nsId.index]?.bind fun ns =>
            (resolveNominal? unit ns name).bind fun handle =>
              if isNominalRecursive unit handle (depth + 1) then
                unit.namespaces[handle.namespaceId.index]?.bind fun targetNs =>
                  targetNs.structs[handle.structId]?.bind fun decl =>
                    targetNs.nameOf decl.name |>.map (·.name)
              else none
      | some (.reference ref) => nominalIn? unit depth nsId ref.referent
      | some (.tuple elements) => elements.toList.findSome? (nominalIn? unit depth nsId)
      | some (.vector elem _) | some (.slice elem) => nominalIn? unit depth nsId elem
      | _ => none

/-- The native types of a function's locals. A specification local (a
quantifier's binder) keeps an empty slot. -/
def CompileNamespace.localTypes? (view : CompileNamespace) (unit : ValidatedUnit)
    (namespaceId : NamespaceId) (declaration : FunctionDecl FunctionBody) : Option (List NTy) :=
  declaration.locals.toList.mapM fun localDecl =>
    let typeId := view.source.typeId localDecl.type.typeId
    match view.nty unit namespaceId typeId with
    | some τ => some τ
    | none => if logicalOnly unit namespaceId typeId then some .unit else none

/-! ## Drops

A drop has no effect on the values a body reads, as in the runtime. A
destructor's effects are outside the denotation, so a drop that could run one
has none (`doc/designs/rust-verification.md`, "Drops"). -/

/-- Whether dropping a value of a type could run a destructor: one the unit states,
or a function value's captures'. A type parameter's is the theorem's assumption
(`doc/status/assumptions.md`). -/
def holdsDestructor (unit : ValidatedUnit) (τ : NTy) : Bool :=
  τ.owns fun
    | some source, _ =>
        ((unit.namespaces[source.namespaceId.index]?).bind (·.structs[source.structId]?)).any
          (·.properties.any (·.tag == destructorTag))
    | none, some _ => false
    | none, none => true

/-- The drops and calls of a body. -/
def CompileNamespace.dropsAndCalls (ns : CompileNamespace) : Nat → ExprId → List Expr → List Expr
  | 0, _, found => found
  | fuel + 1, id, found =>
      match ns.expressions.get? id.index with
      | none => found
      | some expression =>
          let found := match expression.kind with
            | .operation (.drop _) .. | .operation (.call (.function _)) ..
            | .operation (.call (.closure ..)) .. => expression :: found
            | _ => found
          match expression.kind with
          | .spec _ => found
          | kind => (expressionChildren kind).foldl
              (fun found child => ns.dropsAndCalls fuel child found) found

/-- The local a contract expression reads, where it reads one. -/
private def contractLocal? (ns : ValidatedNamespace) (id : ExprId) : Option Nat :=
  let placeRoot : PlaceId → Option Nat := fun place =>
    match (ns.places[place.index]? : Option Place) with
    | some (.localVar localId) => some localId.index
    | _ => none
  match (ns.expressions[id.index]?.map (·.kind) : Option ExprKind) with
  | some (.localVar localId) => some localId.index
  | some (.operation (.read place) _ _ _) | some (.operation (.copy place) _ _ _) =>
      placeRoot place
  | _ => none

/-- Whether a declaration's contract states that its result is one of its
parameters, of a type that `holds`: the parameter is handed to the result, so a
call of it drops nothing of that type. The library's
`<I as IntoIterator>::into_iter` states `result == _1` of the iterator it
returns unchanged. A bodiless declaration's contract is its model and is
trusted as such (`doc/status/assumptions.md`). -/
private def contractHandsOnParameter (ns : ValidatedNamespace) (Γ : NRow)
    (contract : VerexIR.FunctionContract) (holds : NTy → Bool) : Bool :=
  contract.conditions.any fun condition =>
    condition.kind == .ensures &&
      match (ns.expressions[condition.expression.index]?.map (·.kind) : Option ExprKind) with
      | some (.operation (.primitive .equal) _ arguments _) =>
          arguments.size == 2 &&
            (arguments.toList.any fun side =>
              ((ns.expressions[side.index]?.map (·.kind) : Option ExprKind)).any fun kind =>
                kind matches .operation (.specification (.result _)) _ _ _) &&
            (arguments.toList.any fun side =>
              (contractLocal? ns side).any fun slot =>
                slot < Γ.length && holds (Γ.getD slot .unit))
      | _ => false

/-- The parameters of callees a function hands its type parameter `index` to,
or `none` where a body drops it, or where it cannot say: a function without a
body, unless its contract hands a parameter of that type to its result. -/
def handedOn? (unit : ValidatedUnit) (function : FunctionHandle) (index : Nat) :
    Option (List (FunctionHandle × Nat)) := do
  let ns ← unit.namespaces[function.namespaceId.index]?
  let view := CompileNamespace.of unit function.namespaceId ns
  let declaration ← ns.functions[function.functionId.index]?
  let holds := NTy.owns fun _ parameter => parameter == some index
  let .structured root := declaration.body
    | do
        let Γ := NRow.ofList (← view.localTypes? unit function.namespaceId declaration)
        if contractHandsOnParameter ns Γ declaration.contract holds then some [] else none
  let Γ := NRow.ofList (← view.localTypes? unit function.namespaceId declaration)
  (view.dropsAndCalls view.fuel root []).foldlM (init := []) fun found expression =>
    match expression.kind with
    | .operation (.drop place) .. =>
        match compilePlace unit Γ view view.placeFuel place with
        | .ok ⟨_, σ, _, _⟩ => if holds σ then none else some found
        | .error _ => none
    | .operation (.call (.function reference)) instantiations _ _
    | .operation (.call (.closure reference _)) instantiations _ _ => do
        let handle ← resolveFunction? unit function.namespaceId reference
        -- A binder's position is its index, and a lifetime holds nothing.
        let θ ← instantiations.toList.mapM fun argument => match argument with
          | .typeArg value => view.nty unit function.namespaceId value.typeId
          | .lifetime _ => some .unit
          | _ => none
        some (θ.zipIdx.foldl (init := found) fun found (τ, parameter) =>
          if holds τ then (handle, parameter) :: found else found)
    | _ => some found

/-- Whether running a function can drop its type parameters in `pending`,
directly or through the generic calls it makes. Out of fuel it says it can. -/
def dropsParameter (unit : ValidatedUnit) :
    Nat → List (FunctionHandle × Nat) → List (FunctionHandle × Nat) → Bool
  | 0, _, _ => true
  | _ + 1, _, [] => false
  | fuel + 1, visited, (function, index) :: pending =>
      if visited.contains (function, index) then dropsParameter unit fuel visited pending
      else match handedOn? unit function index with
        | none => true
        | some next => dropsParameter unit fuel ((function, index) :: visited) (next ++ pending)

/-- Whether a call instantiates a type parameter its callee drops at a type with
a destructor, or at one holding a mutable reference, whose prophecy the drop would
resolve where the callee has no place for it. A shared reference as the argument has
neither, whatever it refers to. -/
def CompileNamespace.dropsDestructor (ns : CompileNamespace) (unit : ValidatedUnit)
    (callee : FunctionHandle) (typeArgs : List GenericArgument) (θ : NRow) : Bool :=
  ns.source.profile == some .rust &&
    (typeArgs.zip θ.toList).zipIdx.any fun ((argument, τ), index) =>
      match argument with
      | .typeArg argument =>
          !isSharedReferenceType ns.source argument.typeId ns.types.get? &&
            (holdsDestructor unit τ || !τ.refFree) && dropsParameter unit ns.fuel [] [(callee, index)]
      | _ => false

/-- The public Rust theorem's restriction on instantiated parameters that the
function can drop, directly or through callees. Until effectful drop is modelled,
such an argument may neither run a destructor nor hold an unresolved mutable loan.
This is a check on the instantiated types, not a claim that arbitrary Rust drop
glue is pure. Alignment of those types with Rust remains the frontend obligation. -/
def DroppedTypeArgumentsTrivial (unit : ValidatedUnit) (handle : FunctionHandle)
    (θ : TypeArgs) : Prop :=
  ∀ index, index < θ.1.toList.length →
    dropsParameter unit (unit.namespaces.foldl (fun n ns => n + ns.expressions.size + 1) 1)
      [] [(handle, index)] = true →
    holdsDestructor unit (θ.1.getD index .unit) = false ∧
      (θ.1.getD index .unit).refFree = true

/-- A generic instantiation in the caller's types over all binder positions: no
carrier reads a lifetime's or a constant's position, which holds a placeholder. -/
def CompileNamespace.instantiation? (ns : CompileNamespace) (unit : ValidatedUnit)
    (namespaceId : NamespaceId) (arguments : List GenericArgument) : Option NRow :=
  NRow.ofList <$> arguments.mapM fun argument => match argument with
    | .typeArg value => ns.nty unit namespaceId value.typeId
    | .lifetime _ | .const _ => some .unit
    | .evidence _ => none

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
        | .literal value, σ =>
            match held : σ.literal with
            | true => (.literal · held) <$> literalValue σ value
            | false => notCarried "a literal of this type"
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
  -- A constructor pattern binds the declared fields; the ghost slots after
  -- them match anything.
  | fuel + 1, [], .cons (.ghost _) σs => do
      .ok (.cons .wildcard (← compilePatterns Γ ns fuel [] σs))
  | fuel + 1, pattern :: patterns, .cons σ σs => do
      let head ← compilePattern Γ ns fuel pattern σ
      .ok (.cons head (← compilePatterns Γ ns fuel patterns σs))
  | _ + 1, _, _ => .error "a pattern's arity differs from its declaration's"
end

/-- Match arms at a result type. -/
def armsAt {ρ : ResultShape} {Γ : NRow} {σ : NTy} (mismatch : Option Failure) (τ : NTy) :
    List (Option (Nat × String) × Pat Γ σ × Option (Term unit ρ Γ .bool) × Compiled unit ρ Γ) →
      Except String (Arms unit ρ Γ σ τ)
  | [] => .ok (.nil mismatch)
  | (arm, pattern, guard, body) :: rest => do
      let body ← body.at? τ
      let rest ← armsAt mismatch τ rest
      match guard with
      | none => .ok (.cons arm pattern body rest)
      | some guard => .ok (.guarded arm pattern guard body rest)

/-- Match arms whose bodies never complete, at every result type. -/
def neverArms {ρ : ResultShape} {Γ : NRow} {σ : NTy} (mismatch : Option Failure) :
    List (Option (Nat × String) × Pat Γ σ × Option (Term unit ρ Γ .bool) ×
      ((τ : NTy) → Term unit ρ Γ τ)) → (τ : NTy) → Arms unit ρ Γ σ τ
  | [], _ => .nil mismatch
  | (arm, pattern, none, body) :: rest, τ => .cons arm pattern (body τ) (neverArms mismatch rest τ)
  | (arm, pattern, some guard, body) :: rest, τ =>
      .guarded arm pattern guard (body τ) (neverArms mismatch rest τ)

/-- Patterns matching every value of a row, binding nothing. -/
def Pats.wildcards {Γ : NRow} : (σs : NRow) → Pats Γ σs
  | .nil => .nil
  | .cons _ σs => .cons .wildcard (Pats.wildcards σs)

/-- One arm per listed variant of an enum, each the variant's declared
discriminant; every variant is listed, so no value falls through. -/
def discriminantArms {ρ : ResultShape} {Γ : NRow} (unit : ValidatedUnit) (handle : StructHandle)
    {source : StructHandle} {arguments : NRow} {all : List String} {distinct : all.Nodup}
    {width : Nat} {signed : Bool} (rows : NRows) : List String →
    Except String (Arms unit ρ Γ (.enum source arguments all rows distinct) (.int width signed))
  | [] => .ok (.nil none)
  | name :: names => do
      let some ⟨σs, choice⟩ := Which.ofName all rows name
        | .error "a discriminant names an unknown variant"
      let some value := variantDiscriminant? unit handle name
        | notCarried "a discriminant of a variant declaring none"
      let literal ← compileLiteral (.int width signed) (.integer value)
      .ok (.cons none (.variant choice (Pats.wildcards σs)) literal
        (← discriminantArms unit handle rows names))

/-- A bitwise operation on signed integers: on their bit patterns, read back. -/
def signedBitwise {ρ : ResultShape} {Γ : NRow} {width : Nat} (op : BitOp)
    (left right : Term unit ρ Γ (.int width true)) :
    Except String (Term unit ρ Γ (.int width true)) :=
  if nonzero : width ≠ 0 then
    .ok (.wrap nonzero (.bitwise op (.wrap nonzero left : Term unit ρ Γ (.int width false))
      (.wrap nonzero right)))
  else .error "an integer has no width"

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
        let τ? ← typeOf? unit ns namespaceId (ns.source.typeId expression.typeId)
        match expression.kind with
        | .value literal _ =>
            let some τ := τ? | notCarried "a literal of type never"
            let term ← compileLiteral τ literal
            .ok (.at τ term)
        | .localVar localId =>
            let some τ := τ? | notCarried "a local of type never"
            let x ← Var.at? Γ localId.index τ
            -- A value holding a mutable reference moves out of its slot.
            if τ.movesOut then .ok (.at τ (.take x)) else .ok (.at τ (.var x))
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
                ns.declaredNty unit handle.namespaceId (calleeNs.typeId parameter.typeUse.typeId)
              | notCarried "the type of a callee parameter"
            unless parameters.all NTy.lendable do notCarried "a reference inside an aggregate"
            let parameters := NRow.ofList parameters
            let shape ← match callee.signature.results.toList with
              | [] => .ok ResultShape.none
              | [result] => match ns.declaredNty unit handle.namespaceId (calleeNs.typeId result.typeId) with
                  | some τ => .ok (ResultShape.one τ)
                  | none => notCarried "the type of a callee result"
              | _ => notCarried "a callee with several results"
            unless shape.lendable do notCarried "a reference inside an aggregate"
            -- No value reads a lifetime argument, so a call with only those
            -- runs as one with no type arguments.
            if instantiations.all (· matches .lifetime _) then
              let arguments ←
                compileArgs unit loanFacts ρ Γ ns namespaceId true fuel arguments.toList
                parameters
              let value : Term unit ρ Γ shape.bodyType :=
                .call (loopSite namespaceId.index id.index) handle shape arguments
              match τ? with
              | some τ => .ok (.at τ (← Compiled.at? τ (.at shape.bodyType value)))
              | none => .ok (.never fun _ => .drop value .unreachable)
            else
              let typeArgs := ns.source.genericArguments instantiations
              let some θ := ns.instantiation? unit namespaceId typeArgs.toList
                | notCarried "the type of a generic argument"
              if ns.dropsDestructor unit handle typeArgs.toList θ then
                notCarried (if θ.refFree then
                  "a call dropping a type parameter at a type with a destructor"
                  else "a call dropping a type parameter at a type holding a mutable reference")
              let some (θ : TypeArgs) := if inhabitable : θ.inhabitable = true then
                  some ⟨θ, inhabitable⟩ else none
                | notCarried "a type argument without values"
              -- Where no generic theorem transfers, the callee is read at the type arguments.
              unless ns.transfers unit handle θ.1 do
                let instanceAt := ns.ntyAt unit θ.1 handle.namespaceId
                let some parameters := callee.signature.parameters.toList.mapM
                    fun parameter => instanceAt (calleeNs.typeId parameter.typeUse.typeId)
                  | notCarried "the type of a callee parameter"
                let parameters := NRow.ofList parameters
                let shape ← match callee.signature.results.toList with
                  | [] => .ok ResultShape.none
                  | [result] => match instanceAt (calleeNs.typeId result.typeId) with
                      | some τ => .ok (ResultShape.one τ)
                      | none => notCarried "the type of a callee result"
                  | _ => notCarried "a callee with several results"
                unless NRow.lendable parameters && shape.lendable do
                  notCarried "a reference inside an aggregate"
                let arguments ← compileArgs unit loanFacts ρ Γ ns namespaceId true fuel
                  arguments.toList parameters
                let value : Term unit ρ Γ shape.bodyType :=
                  .callInstance (loopSite namespaceId.index id.index) handle typeArgs θ shape
                    arguments
                return ← match τ? with
                  | some τ => .ok (.at τ (← Compiled.at? τ (.at shape.bodyType value)))
                  | none => .ok (.never fun _ => .drop value .unreachable)
              -- The signature θ instantiates, evaluated here; the node keeps the evidence.
              let parameters' := NRow.subst θ.1 parameters
              let shape' := shape.subst θ.1
              let resolved : (NRow.subst θ.1 parameters == parameters' &&
                  shape.subst θ.1 == shape') = true := by simp [parameters', shape']
              let arguments ← compileArgs unit loanFacts ρ Γ ns namespaceId true fuel arguments.toList
                parameters'
              let value : Term unit ρ Γ shape'.bodyType :=
                .callGeneric (loopSite namespaceId.index id.index) handle typeArgs θ shape
                  shape' resolved arguments
              match τ? with
              | some τ => .ok (.at τ (← Compiled.at? τ (.at shape'.bodyType value)))
              | none => .ok (.never fun _ => .drop value .unreachable)
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
                ns.declaredNty unit handle.namespaceId (targetNs.typeId parameter.typeUse.typeId)
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
                  let captures ← compileArgs unit loanFacts ρ Γ ns namespaceId false fuel
                    captures.toList captured
                  let value : Term unit ρ Γ (.function supplied shared results) :=
                    .closure handle weave results shared rows sharing closed faithful captures
                  .ok (.at τ (← Compiled.at? τ (.at (.function supplied shared results) value)))
                else notCarried "a closure whose empty instantiation is not faithful to its target"
                else notCarried "a closure without type arguments over type parameters"
                else notCarried "a closure whose shared parameters are not its target's"
              else notCarried "a closure whose rows are not its target's"
            else
              let typeArgs := ns.source.genericArguments instantiations
              let some θ := ns.instantiation? unit namespaceId typeArgs.toList
                | notCarried "the type of a generic argument"
              unless ns.transfers unit handle θ do notCarried "a reference type argument"
              if ns.dropsDestructor unit handle typeArgs.toList θ then
                notCarried "a closure dropping a type parameter at a type with a destructor"
              let some (θ : TypeArgs) := if inhabitable : θ.inhabitable = true then
                  some ⟨θ, inhabitable⟩ else none
                | notCarried "a type argument without values"
              let shape ← match target.signature.results.toList with
                | [] => .ok ResultShape.none
                | [result] => match ns.declaredNty unit handle.namespaceId (targetNs.typeId result.typeId) with
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
                  let captured' := NRow.subst θ.1 captured
                  let supplied' := NRow.subst θ.1 supplied
                  let shape' := shape.subst θ.1
                  let resolved : (NRow.subst θ.1 captured == captured' &&
                      NRow.subst θ.1 supplied == supplied' && shape.subst θ.1 == shape') = true := by
                    simp [captured', supplied', shape']
                  let captures ← compileArgs unit loanFacts ρ Γ ns namespaceId false fuel
                    captures.toList captured'
                  let value : Term unit ρ Γ (.function supplied' shared shape'.row) :=
                    .closureGeneric handle weave typeArgs θ shape shape' shared rows sharing
                      below resolved captures
                  .ok (.at τ (← Compiled.at? τ (.at (.function supplied' shared shape'.row) value)))
                else notCarried "a closure target whose signature names foreign type parameters"
                else notCarried "a closure whose shared parameters are not its target's"
              else notCarried "a closure whose rows are not its target's"
        | .operation (.call .invoke) _ arguments _ =>
            let callable :: arguments := arguments.toList
              | .error "an invocation has no function value"
            let some callableExpression := ns.expressions.get? callable.index
              | .error "an invocation's function value is out of range"
            let some (.function parameters shared results) :=
                ns.nty unit namespaceId (ns.source.typeId callableExpression.typeId)
              | notCarried "an invocation of a value that is not a function"
            unless parameters.toList.all NTy.lendableAtTop do
              notCarried "a reference inside an aggregate"
            let shape ← match results.toList with
              | [] => .ok ResultShape.none
              | [result] => .ok (ResultShape.one result)
              | _ => notCarried "a function value with several results"
            unless shape.lendableAtTop do notCarried "a reference inside an aggregate"
            let function ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel callable).at?
              (.function parameters shared shape.row)
            let arguments ← compileArgs unit loanFacts ρ Γ ns namespaceId true fuel arguments parameters
            let value : Term unit ρ Γ shape.bodyType :=
              .invoke (loopSite namespaceId.index id.index) shape function arguments
            match τ? with
            | some τ => .ok (.at τ (← Compiled.at? τ (.at shape.bodyType value)))
            | none => .ok (.never fun _ => .drop value .unreachable)
        | .operation (.borrow .mutable _) _ _ _ =>
            let some τ := τ? | notCarried "a borrow of type never"
            match expression.kind with
            | .operation operation _ arguments _ =>
                compileOperation unit loanFacts ρ Γ ns namespaceId fuel id τ
                  operation arguments.toList
            | _ => .error "internal: a borrow is not an operation"
        | .operation (.global kind) instantiations arguments _ =>
            let some τ := τ? | notCarried "a storage operation of type never"
            let #[.typeArg resource] := instantiations
              | notCarried "a storage operation without one resource type"
            let some (native, typeArguments) := ns.resource unit namespaceId (ns.source.typeId resource.typeId)
              | notCarried "a storage operation on a type without a native resource type"
            match kind, arguments.toList with
            | .contains, [key] => do
                let ⟨_, key⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel key).some
                .ok (.at .bool (.globalContains native typeArguments key))
            | .borrow .immutable, [key] => do
                let ⟨_, key⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel key).some
                unless τ = native do .error "a global read's type differs from its resource type"
                .ok (.at native (.globalRead (loopSite namespaceId.index id.index) typeArguments key))
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
                let fields ← compilePackArgs unit loanFacts ρ Γ ns namespaceId fuel
                  arguments.toList σs
                -- Each initialized ghost slot takes its initializer's value
                -- (`doc/designs/ghost-fields.md`), before the invariants are owed.
                let ghosts := nominalGhosts unit handle
                let inits ← (List.range ghosts.size).foldrM
                    (init := (GhostInits.nil : GhostInits σs)) fun k rest => do
                  unless (ghosts[k]?.bind (·.initializer)).isSome do return rest
                  let some ⟨σ, x⟩ := Var.ofIndex σs (arguments.size + k)
                    | .error "a ghost slot is out of range"
                  match σ, x with
                  | .ghost _, x =>
                      pure (GhostInits.cons (ghostInitSite namespaceId.index id.index k) x rest)
                  | _, _ => .error "a ghost field's position holds no ghost slot"
                let packed := Term.pack (loopSite namespaceId.index id.index) source
                  nominalArguments fields inits
                .ok (.at (.struct source nominalArguments σs)
                  (if ns.carriesInvariant (.struct source nominalArguments σs) then
                    .constructed (loopSite namespaceId.index id.index) packed else packed))
            | .enum source nominalArguments names rows distinct, some name =>
                unless handle = source do .error "a constructor's declaration differs from its type"
                let some ⟨σs, choice⟩ := Which.ofName names rows name
                  | .error "a constructor names an unknown variant"
                let fields ← compilePackArgs unit loanFacts ρ Γ ns namespaceId fuel
                  arguments.toList σs
                let ghosts := nominalGhosts unit handle
                let inits ← (List.range ghosts.size).foldrM
                    (init := (GhostInits.nil : GhostInits σs)) fun k rest => do
                  unless (ghosts[k]?.bind (·.initializer)).isSome do return rest
                  let some ⟨σ, x⟩ := Var.ofIndex σs (arguments.size + k)
                    | .error "a ghost slot is out of range"
                  match σ, x with
                  | .ghost _, x =>
                      pure (GhostInits.cons (ghostInitSite namespaceId.index id.index k) x rest)
                  | _, _ => .error "a ghost field's position holds no ghost slot"
                let packed := Term.variant (loopSite namespaceId.index id.index) source
                  nominalArguments distinct choice fields inits
                .ok (.at (.enum source nominalArguments names rows distinct)
                  (if ns.carriesInvariant (.enum source nominalArguments names rows distinct) then
                    .constructed (loopSite namespaceId.index id.index) packed else packed))
            | _, _ => .error "a constructor's type is not its declaration's"
        | .operation operation _ arguments _ =>
            let some τ := τ? | notCarried "an operation of type never"
            compileOperation unit loanFacts ρ Γ ns namespaceId fuel id τ
                  operation arguments.toList
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
            compileBinding Γ ns fuel pattern σ value (.at .unit (.lit () rfl))
        | .match_ scrutinee arms => do
            let compiledScrutinee ← compileExpr unit loanFacts ρ Γ ns namespaceId fuel scrutinee
            if let .never term := compiledScrutinee then return .never term
            let ⟨σ, scrutinee⟩ := compiledScrutinee.some
            let arms ← arms.toList.mapM fun arm => do
              let pattern ← compilePattern Γ ns fuel arm.pattern σ
              let guard ← arm.guard.mapM fun guard => do
                (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel guard).at? .bool
              let body ← compileExpr unit loanFacts ρ Γ ns namespaceId fuel arm.body
              -- The variant the arm takes, as a selector names it.
              let variant := match ns.patterns.get? arm.pattern.index with
                | some { kind := .constructor _ _ (some variant) _, .. } => variant
                | _ => "_"
              .ok (some (loopSite namespaceId.index id.index, variant), pattern, guard, body)
            -- A match of type never may still have arms that complete.
            let resultType? := τ?.orElse fun _ => arms.findSome? fun (_, _, _, body) =>
              match body with
              | .at τ _ => some τ
              | .never _ => none
            match resultType? with
            | some τ => .ok (.at τ (.caseOf scrutinee
                (← armsAt (mismatchFailure? ns.source.profile) τ arms)))
            | none =>
                let some arms := arms.mapM fun (arm, pattern, guard, body) => match body with
                    | .never body => some (arm, pattern, guard, body)
                    | .at .. => none
                  | .error "internal: a match of type never has an arm with a type"
                .ok (.never fun τ =>
                  .caseOf scrutinee (neverArms (mismatchFailure? ns.source.profile) arms τ))
        | .ifElse condition thenBranch elseBranch => do
            -- One of the source's decisions: an arm of a lowered `match`, or a
            -- conditional not standing for an `&&` (an `else` of `false`), an
            -- `||` (a `then` of `true`) or an `assert!` (a `then` of unit, an
            -- `else` that throws), as the source printer reads them back.
            let kindOf (id : ExprId) := (ns.expressions.get? id.index).map (·.kind)
            let lowered := expression.origin matches some (.matchArm ..)
            let shaped : Bool := match elseBranch with
              | some elseBranch =>
                  !(kindOf elseBranch matches some (.value (.bool false) _)) &&
                    !(kindOf thenBranch matches some (.value (.bool true) _)) &&
                    !((kindOf thenBranch matches some (.value .unit _) ||
                        kindOf thenBranch matches some (.block #[] none)) &&
                      kindOf elseBranch matches some (.throw_ ..))
              | none => true
            let site := if lowered || shaped then some (loopSite namespaceId.index id.index)
              else none
            let condition ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel condition).at? .bool
            let thenBranch ← compileExpr unit loanFacts ρ Γ ns namespaceId fuel thenBranch
            let elseBranch ← match elseBranch with
              | some elseBranch => compileExpr unit loanFacts ρ Γ ns namespaceId fuel elseBranch
              | none => .ok (.at .unit (.lit () rfl))
            match τ? with
            | some τ => do
                let thenBranch ← thenBranch.at? τ
                let elseBranch ← elseBranch.at? τ
                .ok (.at τ (.ite site condition thenBranch elseBranch))
            | none =>
                match thenBranch, elseBranch with
                | .never thenBranch, .never elseBranch =>
                    .ok (.never fun τ => .ite site condition (thenBranch τ) (elseBranch τ))
                | .at σ thenBranch, elseBranch => do
                    let elseBranch ← elseBranch.at? σ
                    .ok (.at σ (.ite site condition thenBranch elseBranch))
                | thenBranch, .at σ elseBranch => do
                    let thenBranch ← thenBranch.at? σ
                    .ok (.at σ (.ite site condition thenBranch elseBranch))
        | .throw_ kind arguments =>
            match arguments.toList with
            | [] => .ok (.never fun _ => .throw0 (loopSite namespaceId.index id.index) kind)
            | [code] => do
                let ⟨σ, code⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel code).some
                match σ, code with
                | .int _ _, code =>
                    .ok (.never fun _ => .throw1 (loopSite namespaceId.index id.index) kind code)
                | _, _ => notCarried "a throw of a value that is not an integer"
            | _ => notCarried "a throw with several arguments"
        | .return_ values =>
            match values.toList with
            | [] => do
                let value ← Compiled.at? ρ.bodyType (Compiled.at (ρ := ρ) (Γ := Γ) .unit (.lit () rfl))
                .ok (.never fun _ => .return_ value)
            | [value] => do
                let value ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel value).at? ρ.bodyType
                -- An explicit return is a normal exit: a mutable reference
                -- leaving here owes its referent's invariants (`certifiedResult`).
                .ok (.never fun _ =>
                  .return_ (certifiedResult (loopSite namespaceId.index id.index) value))
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
                -- ends a mutation of it; the write itself certifies the
                -- local's own value where it writes into it
                -- (`Proj.setOwner?`).
                let owner := (ns.ownedRoot? ns.placeFuel place).bind fun root =>
                  (Var.ofIndex Γ root.index).map (·.1)
                if owner.any ns.carriesInvariant then
                  .ok (.at .unit (.seqAfter
                    (.writePlace (loopSite namespaceId.index id.index) (ns.mismatchOn path)
                      x path value)
                    (.mutationEnd (loopSite namespaceId.index id.index))))
                else .ok (.at .unit (.writePlace (loopSite namespaceId.index id.index)
                  (ns.mismatchOn path) x path value))
        | .loop _ body => do
            let body ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel body).at? .unit
            let loop := Term.loop (loopSite namespaceId.index id.index) body
            match τ? with
            | none | some .unit | some (.tuple .nil) => .ok (.at .unit loop)
            -- A loop of another type completes only by a break with a value,
            -- which is not carried, so it never completes: what follows it
            -- is unreachable.
            | some τ => .ok (.at τ (.drop loop (.throw0 (loopSite namespaceId.index id.index) .panic)))
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
            -- An update writes the ghost field it names the value the
            -- meanings give its site (`doc/designs/ghost-fields.md`), before or
            -- after the block's other conditions.
            let isUpdate := fun (condition : Condition) => condition.kind == .update
            let all := block.conditions.toList
            let updatesFirst := (all.dropWhile isUpdate).all (!isUpdate ·)
            let updatesLast := (all.dropWhile (!isUpdate ·)).all isUpdate
            if !updatesFirst && !updatesLast then
              notCarried "a specification block interleaving updates and other conditions"
            else
            let writes ← (all.filter isUpdate).mapM fun condition => do
              let some (_, target) := condition.auxiliary.find? (·.1 == "updateTarget")
                | .error "an update has no target"
              let site := loopSite namespaceId.index condition.expression.index
              match ns.expressions.get? target.index with
              | some { kind := .operation (.read place) _ _ _, .. } =>
                  unless ns.place? place matches some (.ghostField ..) do
                    notCarried "an update whose target is not a ghost field"
                  let ⟨_, _, x, path⟩ ← compilePlace unit Γ ns fuel place
                  pure (Term.writePlace site (ns.mismatchOn path) x path (.ghostValue site) :
                    Term unit ρ Γ .unit)
              -- A spec variable's write: its memory's resource, repacked with
              -- the value, which only the model's memory holds.
              | some { kind := .operation (.data (.select reference _)) _ #[base] _, .. } =>
                  let some baseExpression := ns.expressions.get? base.index
                    | .error "an update's target is out of range"
                  let .operation (.specification (.global none)) #[.typeArg resource] #[key] _ :=
                      baseExpression.kind
                    | notCarried "an update whose target is not a ghost field or a spec variable"
                  let some handle := resolveStruct? unit namespaceId reference
                    | .error "an update's resource does not resolve"
                  let ghostMemory := ((unit.namespaces[handle.namespaceId.index]?).bind
                    (·.structs[handle.structId]?)).any (·.isGhostMemory)
                  unless ghostMemory do
                    notCarried "an update of memory other than a spec variable's"
                  let some (native, typeArguments) := ns.resource unit namespaceId (ns.source.typeId resource.typeId)
                    | notCarried "a spec variable's memory without a native resource type"
                  let .struct source arguments (.cons τ .nil) := native
                    | notCarried "a spec variable's memory with other than one field"
                  let ⟨_, key⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel key).some
                  let value : Term unit ρ Γ (.struct source arguments (.cons τ .nil)) :=
                    .pack site source arguments (.cons (.ghostValue site) .nil) .nil
                  pure (.seqAfter (.ghostStore site typeArguments key value) (.memoryWritten site))
              | _ => notCarried "an update whose target is not a place"
            let conditions := all.filter (!isUpdate ·)
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
              let steps := if updatesFirst then writes ++ steps else steps ++ writes
              match steps with
              | [] => .ok (.at .unit (.lit () rfl))
              | first :: rest => .ok (.at .unit (rest.foldl .seqAfter first))
        | kind => notCarried (describeKind kind)

  /-- A read of a place at its declared type; `consume` moves a mutable
  reference out of its local. -/
  def compileRead (unit : ValidatedUnit) (Γ : NRow) (ns : CompileNamespace) (site : Nat)
      (consume : Bool) : Nat → NTy → PlaceId → Except String (Compiled unit ρ Γ)
    | 0, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, τ, place => do
        if let some (.deref holder) := ns.place? place then
          let ⟨_, component, x, path⟩ ← compilePlace unit Γ ns fuel holder
          match component, path with
          | .sliceRef element, path =>
              if equal : NTy.slice element = τ then
                return .at τ (equal ▸ .sliceCurrent (readOut site (ns.mismatchOn path) x path))
              else throw "a slice read has an unexpected type"
          | _, _ => pure ()
        match ns.place? place with
        | some (.localVar localId) => do
            let x ← Var.at? Γ localId.index τ
            if consume && τ.movesOut then .ok (.at τ (.take x)) else .ok (.at τ (.var x))
        | some (.subslice base start stop fromEnd) => do
            let .slice element := τ | notCarried "a subslice snapshot of a non-slice type"
            let vector ← match compilePlace unit Γ ns fuel base with
              | .ok ⟨_, .array held length, _, _⟩ => do
                  if equal : held = element then
                    let array ← (← compileRead unit Γ ns site false fuel (.array held length) base).at?
                      (.array held length)
                    pure (equal ▸ Term.arrayVector array)
                  else .error "a subslice has an unexpected element type"
              | .ok ⟨_, .vector held, _, _⟩ => do
                  if equal : held = element then
                    pure (equal ▸ (← (← compileRead unit Γ ns site false fuel (.vector held) base).at?
                      (.vector held)))
                  else .error "a subslice has an unexpected element type"
              | _ => do
                  let slice ←
                    (← compileRead unit Γ ns site false fuel (.slice element) base).at?
                      (.slice element)
                  pure (Term.sliceVector slice)
            let lower : Term unit ρ Γ (.int 64 false) :=
              .lit (← literalValue (.int 64 false) (.integer (Int.ofNat start))) rfl
            let offset : Term unit ρ Γ (.int 64 false) :=
              .lit (← literalValue (.int 64 false) (.integer (Int.ofNat stop))) rfl
            -- Wrapping an underflow gives an upper bound greater than the
            -- sequence length; the slice's checked bounds reject it.
            let upper := if fromEnd then Term.modular .subtract (.length vector) offset else offset
            .ok (.at (.slice element)
              (.vectorSlice (.slice (loopSite ns.namespaceId.index place.index) vector lower upper)))
        -- Reading an unsize observes the array contents without moving it.
        | some (.unsize base) => do
            let ⟨_, baseType, _, _⟩ ← compilePlace unit Γ ns fuel base
            let ⟨σ, array⟩ := (← compileRead unit Γ ns site false fuel baseType base).some
            match σ, array with
            | .array element _, array =>
                if equal : NTy.slice element = τ then .ok (.at τ (equal ▸ .vectorSlice (.arrayVector array)))
                else .error "an unsize read has an unexpected type"
            | _, _ => notCarried "an unsize place on a non-array"
        | _ => do
            let ⟨_, component, x, path⟩ ← compilePlace unit Γ ns fuel place
            if equal : component = τ then
              if consume && ns.substitution.isSome && τ.movesOut then
                -- Validation reads a reference moved out of an aggregate as a reborrow
                -- through it, so its component stays true; a type parameter did not show
                -- it there.
                match component, path with
                | .sliceRef element, path =>
                    .ok (.at (.sliceRef element) (.reborrowSlice (ns.mismatchOn path) x path))
                | .ref referent, path =>
                    let reborrow := path.append (.deref .nil)
                    .ok (.at (.ref referent) (.borrowPlace (ns.mismatchOn reborrow) x reborrow))
                | _, _ =>
                    notCarried "a move of a value holding a mutable reference out of an aggregate"
              -- Validation leaves such a move only where an overwrite follows it.
              else
                let read : Except String (Compiled unit ρ Γ) :=
                  .ok (.at τ (readOut site (ns.mismatchOn path) x (equal ▸ path)))
                if consume then
                  match τ, equal with
                  | .sliceRef element, equal =>
                      .ok (.at (.sliceRef element) (.takeSliceAt (ns.mismatchOn path) x (equal ▸ path)))
                  | .ref σ, equal =>
                      .ok (.at (.ref σ) (.takeAt site (ns.mismatchOn path) x (equal ▸ path)))
                  | _, _ => read
                else read
            else .error "a place read has an unexpected type"

  /-- An argument row at the expected types.  A row `handsOver` to a call
  certifies the current of each of its mutable references at the argument's
  own site, which the callee assumes at its entry (`Term.handedOver`,
  `doc/designs/data-invariants.md`, "A call boundary"); a row that
  builds a value hands nothing over. -/
  def compileArgs (unit : ValidatedUnit) (loanFacts : Array CheckedLoanFact)
      (ρ : ResultShape) (Γ : NRow) (ns : CompileNamespace) (namespaceId : NamespaceId)
      (handsOver : Bool) :
      Nat → List ExprId → (σs : NRow) → Except String (Args unit ρ Γ σs)
    | 0, _, _ => .error "the compiler ran out of fuel"
    | _ + 1, [], .nil => .ok .nil
    | fuel + 1, argument :: arguments, .cons σ σs => do
        let head ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel argument).at? σ
        let head : Term unit ρ Γ σ := match σ, head with
          | .ref referent, head =>
              if handsOver && !referent.certificateFree then
                .handedOver (loopSite namespaceId.index argument.index) head
              else head
          | _, head => head
        let tail ← compileArgs unit loanFacts ρ Γ ns namespaceId handsOver fuel arguments σs
        .ok (.cons head tail)
    | _ + 1, _, _ => .error "an argument row's arity differs from its declaration's"

  /-- A constructor's fields, then each ghost slot of its row: any value of
  its type, the model's value for a ghost field the runtime does not hold
  (`doc/designs/ghost-fields.md`). -/
  def compilePackArgs (unit : ValidatedUnit) (loanFacts : Array CheckedLoanFact)
      (ρ : ResultShape) (Γ : NRow) (ns : CompileNamespace) (namespaceId : NamespaceId) :
      Nat → List ExprId → (σs : NRow) → Except String (Args unit ρ Γ σs)
    | 0, _, _ => .error "the compiler ran out of fuel"
    | _ + 1, [], .nil => .ok .nil
    | fuel + 1, [], .cons (.ghost _) σs => do
        .ok (.cons (.ghostSlot .arbitrary)
          (← compilePackArgs unit loanFacts ρ Γ ns namespaceId fuel [] σs))
    | fuel + 1, argument :: arguments, .cons σ σs => do
        let head ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel argument).at? σ
        let tail ← compilePackArgs unit loanFacts ρ Γ ns namespaceId fuel arguments σs
        .ok (.cons head tail)
    | _ + 1, _, _ => .error "a constructor's arity differs from its declaration's"

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
            .caseOf value (.cons none typed body (.nil (mismatchFailure? ns.source.profile))))
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
    | _ + 1, [], none => .ok (.at .unit (.lit () rfl))
    | fuel + 1, [], some result => compileExpr unit loanFacts ρ Γ ns namespaceId fuel result
    | fuel + 1, statement :: statements, result => do
        let ⟨_, statement⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel statement).some
        let rest ← compileBlock unit loanFacts ρ Γ ns namespaceId fuel statements result
        .ok (rest.map fun _ rest => .drop statement rest)

  /-- An operation at its declared result type. -/
  def compileOperation (unit : ValidatedUnit) (loanFacts : Array CheckedLoanFact)
      (ρ : ResultShape) (Γ : NRow) (ns : CompileNamespace) (namespaceId : NamespaceId) :
      Nat → ExprId → NTy → Operation → List ExprId → Except String (Compiled unit ρ Γ)
    | 0, _, _, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, origin, τ, .primitive primitive, arguments =>
        compilePrimitive unit loanFacts ρ Γ ns namespaceId fuel origin τ primitive arguments
    | fuel + 1, origin, τ, .copy place, [] =>
        compileRead unit Γ ns (loopSite namespaceId.index origin.index) false fuel τ place
    | fuel + 1, origin, τ, .move place, [] =>
        compileRead unit Γ ns (loopSite namespaceId.index origin.index) true fuel τ place
    | fuel + 1, origin, τ, .read place, [] =>
        compileRead unit Γ ns (loopSite namespaceId.index origin.index) false fuel τ place
    | fuel + 1, origin, .int width signed, .readDiscriminant place, [] => do
        let ⟨_, σ, x, path⟩ ← compilePlace unit Γ ns fuel place
        match σ, path with
        | .enum source _ names rows _, path =>
            .ok (.at (.int width signed)
              (.caseOf (readOut (loopSite namespaceId.index origin.index) (ns.mismatchOn path) x path)
                (← discriminantArms unit source rows names)))
        | _, _ => notCarried "a discriminant read of a non-enum place"
    | fuel + 1, origin, τ, .borrow .immutable place, [] => do
        -- A shared view holds no loan: what the place holds, observed.
        let site := loopSite namespaceId.index origin.index
        match compilePlace unit Γ ns fuel place with
        | .ok ⟨_, component, x, path⟩ =>
            if component = τ then compileRead unit Γ ns site false fuel τ place
            else if equal : component.erase = τ then
              .ok (.at τ (equal ▸ .observe site (readOut site (ns.mismatchOn path) x path)))
            else compileRead unit Γ ns site false fuel τ place
        | .error _ => compileRead unit Γ ns site false fuel τ place
    | fuel + 1, _, .sliceRef element, .borrow .mutable place, [] => do
        let base ← match ns.place? place with
          | some (.unsize base) | some (.deref base) => pure base
          | some (.subslice ..) => notCarried "a mutable subslice without a typed fixed-extent splice"
          | _ => pure place
        let ⟨_, component, x, path⟩ ← compilePlace unit Γ ns fuel base
        match component, path with
        | .array held _, path =>
            if equal : held = element then
              .ok (.at (.sliceRef element) (equal ▸ .borrowArraySlice (ns.mismatchOn path) x path))
            else .error "a mutable array slice has an unexpected element type"
        | .vector held, path =>
            if equal : held = element then
              .ok (.at (.sliceRef element) (equal ▸ .borrowVectorSlice (ns.mismatchOn path) x path))
            else .error "a mutable vector slice has an unexpected element type"
        | .sliceRef held, path =>
            if equal : held = element then
              .ok (.at (.sliceRef element) (equal ▸ .reborrowSlice (ns.mismatchOn path) x path))
            else .error "a mutable slice reborrow has an unexpected element type"
        | _, _ => notCarried "a mutable slice borrow of a non-sequence"
    | fuel + 1, origin, .ref referent, .borrow .mutable place, [] => do
        let ⟨_, component, x, path⟩ ← compilePlace unit Γ ns fuel place
        if equal : component = referent then
          .ok (.at (.ref referent) (.borrowPlace (ns.mismatchOn path) x (equal ▸ path)))
        else .error "a mutable borrow has an unexpected type"
    | fuel + 1, origin, τ, .reference .dereference, [operand] => do
        -- A shared reference is the observed value itself.
        if ns.sharedAt operand then
          return .at τ (← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).at? τ)
        -- Reading through a local reference leaves it in place.
        if let some { kind := .localVar localId, .. } := ns.expressions.get? operand.index then
          if let some ⟨.sliceRef element, x⟩ := Var.ofIndex Γ localId.index then
            if equal : NTy.slice element = τ then
              return .at τ (equal ▸ .sliceCurrent (.var x))
            else throw "a slice dereference has an unexpected type"
          let x ← Var.at? Γ localId.index (.ref τ)
          return .at τ (copyOut (loopSite namespaceId.index origin.index) (.var x))
        let ⟨σ, operand⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).some
        match σ, operand with
        | .sliceRef element, operand =>
            if equal : NTy.slice element = τ then
              .ok (.at τ (equal ▸ .sliceCurrent operand))
            else .error "a slice dereference has an unexpected type"
        | .ref referent, operand =>
            if equal : referent = τ then
              .ok (.at τ (copyOut (loopSite namespaceId.index origin.index) (equal ▸ operand)))
            else .error "a dereference has an unexpected type"
        | _, _ => notCarried "a dereference of a non-reference"
    | fuel + 1, origin, τ, .reference (.borrow .immutable), [operand] => do
        -- A shared borrow of a computed value is that value: a shared
        -- reference denotes its referent, as a shared borrow of a place
        -- denotes the place's read.
        let ⟨σ, value⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).some
        let site := loopSite namespaceId.index origin.index
        if equal : σ = τ then .ok (.at τ (equal ▸ value))
        else if equal : σ.erase = τ then .ok (.at τ (equal ▸ .observe site value))
        else match σ, value, τ with
          | .array element _, value, .slice target =>
              if equal : element.erase = target then
                .ok (.at (.slice target)
                  (equal ▸ .vectorSlice (.arrayVector (.observe site value))))
              else .error "a shared slice borrow has an unexpected element type"
          | .vector element, value, .slice target =>
              if equal : element.erase = target then
                .ok (.at (.slice target) (equal ▸ .vectorSlice (.observe site value)))
              else .error "a shared slice borrow has an unexpected element type"
          | _, _, _ => .error "a shared borrow has an unexpected type"
    | fuel + 1, origin, τ, .reference (.freeze _), [operand] => do
        -- Freezing a shared reference reads it.
        if ns.sharedAt operand then
          return .at τ (← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).at? τ)
        -- A freeze of a mutable reference is a shared reborrow: the result
        -- is the current value, and the loan's death, not the freeze,
        -- resolves its prophecy.
        let some localId := heldLocal? ns operand
          | notCarried "a freeze of a reference that no local holds"
        let site := loopSite namespaceId.index origin.index
        if let some ⟨.sliceRef element, x⟩ := Var.ofIndex Γ localId.index then
          if equal : (NTy.slice element).erase = τ then
            return .at τ (equal ▸ .observe site (.sliceCurrent (.var x)))
          else throw "a frozen slice has an unexpected type"
        let some ⟨.ref σ, x⟩ := Var.ofIndex Γ localId.index
          | .error "a freeze's local is not a mutable reference"
        if equal : σ = τ then .ok (.at τ (equal ▸ copyOut site (.var x)))
        else if equal : σ.erase = τ then
          .ok (.at τ (equal ▸ .observe site (copyOut site (.var x))))
        else .error "a freeze has an unexpected type"
    | fuel + 1, origin, .unit, .reference .mutate, [target, value] => do
        let some { kind := .localVar localId, .. } := ns.expressions.get? target.index
          | notCarried "a mutation of a reference that is not a local"
        let some ⟨σ, x⟩ := Var.ofIndex Γ localId.index | .error "a local is out of range"
        match σ, x with
        | .ref referent, x => do
            let value ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel value).at? referent
            .ok (.at .unit (.mutate x value))
        | _, _ => notCarried "a mutation of a non-reference local"
    | fuel + 1, origin, τ, .data (.select reference field), [operand] => do
        let ⟨σ, operand⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).some
        match σ, operand with
        | .struct _ _ σs, operand => do
            let some index := referencedFieldIndex? unit namespaceId reference none field
              | .error "a field selection does not resolve"
            let x ← Var.at? σs index τ
            .ok (.at τ (.field x operand))
        | _, _ => notCarried "a field selection on a non-struct"
    -- A ghost field reads as its slot's contents (`doc/designs/ghost-fields.md`).
    | fuel + 1, origin, τ, .data (.selectGhost reference field), [operand] => do
        let ⟨σ, operand⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).some
        let some handle := resolveStruct? unit namespaceId reference
          | .error "a ghost field selection does not resolve"
        match σ, operand with
        | .struct _ _ σs, operand => do
            let some index := handleGhostIndex? unit handle none field
              | .error "a ghost field selection names an unknown ghost field"
            let x ← Var.at? σs index (.ghost τ)
            .ok (.at τ (.ghostRead (.field x operand)))
        | .enum source _ names rows _, operand => do
            unless handle = source do .error "a ghost field selection's declaration differs"
            let some choices := ghostChoices? unit handle names field
              | .error "a ghost field selection names an unknown ghost field"
            let choices ← Choices.ofList names rows (.ghost τ) choices
            .ok (.at τ (.ghostRead (.payload none choices operand)))
        | _, _ => notCarried "a ghost field selection on a non-nominal value"
    | fuel + 1, origin, .bool, .data (.testVariants _ variants), [operand] => do
        let ⟨σ, operand⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).some
        match σ, operand with
        | .enum _ _ _ _ _, operand => .ok (.at .bool (.isVariant variants.toList operand))
        | _, _ => notCarried "a variant test on a non-enum"
    | fuel + 1, origin, τ, .data (.selectVariants reference fields), [operand] => do
        let ⟨σ, operand⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).some
        match σ, operand with
        | .enum source _ names rows _, operand => do
            let some handle := resolveStruct? unit namespaceId reference
              | .error "a variant field selection does not resolve"
            unless handle = source do .error "a variant field selection's declaration differs"
            let some choices := variantFieldChoices? unit handle fields
              | .error "a variant field selection has no choices"
            let choices ← Choices.ofList names rows τ choices.toList
            let mismatch := if choices.partial then mismatchFailure? ns.source.profile else none
            .ok (.at τ (.payload mismatch choices operand))
        | _, _ => notCarried "a variant field selection on a non-enum"
    -- A discriminant is the declared value of the variant the operand holds.
    | fuel + 1, origin, .int width signed, .data (.discriminant reference), [operand] => do
        let ⟨σ, operand⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).some
        match σ, operand with
        | .enum source _ names rows _, operand =>
            let some handle := resolveStruct? unit namespaceId reference
              | .error "a discriminant does not resolve"
            unless handle = source do .error "a discriminant's declaration differs from its type"
            .ok (.at (.int width signed)
              (.caseOf operand (← discriminantArms unit handle rows names)))
        | _, _ => notCarried "a discriminant of a non-enum"
    | fuel + 1, _, .unit, .drop place, [] => do
        let ⟨_, σ, _, _⟩ ← compilePlace unit Γ ns fuel place
        if holdsDestructor unit σ then notCarried "a drop at a type with a destructor"
        .ok (.at .unit (.lit () rfl))
    | _ + 1, _, _, operation, _ => notCarried (describeKind (.operation operation #[] #[] none))

  /-- A pure primitive at its declared result type. -/
  def compilePrimitive (unit : ValidatedUnit) (loanFacts : Array CheckedLoanFact)
      (ρ : ResultShape) (Γ : NRow) (ns : CompileNamespace) (namespaceId : NamespaceId) :
      Nat → ExprId → NTy → PrimitiveOperation → List ExprId → Except String (Compiled unit ρ Γ)
    | 0, _, _, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, origin, τ, .copyValue, [operand] => do
        let operand ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).at? τ
        .ok (.at τ operand)
    | fuel + 1, origin, τ, .moveValue, [operand] => do
        let operand ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).at? τ
        .ok (.at τ operand)
    | fuel + 1, origin, .tuple σs, .tuple, elements => do
        let elements ← compileArgs unit loanFacts ρ Γ ns namespaceId false fuel elements σs
        .ok (.at (.tuple σs) (.tuple elements))
    | fuel + 1, origin, τ, .add, [left, right] =>
        compileModular unit loanFacts ρ Γ ns namespaceId fuel τ .add left right
    | fuel + 1, origin, τ, .subtract, [left, right] =>
        compileModular unit loanFacts ρ Γ ns namespaceId fuel τ .subtract left right
    | fuel + 1, origin, τ, .multiply, [left, right] =>
        compileModular unit loanFacts ρ Γ ns namespaceId fuel τ .multiply left right
    | fuel + 1, origin, τ, .checkedAdd failure, [left, right] =>
        compileChecked unit loanFacts ρ Γ ns namespaceId fuel origin τ .add failure left right
    | fuel + 1, origin, τ, .checkedSubtract failure, [left, right] =>
        compileChecked unit loanFacts ρ Γ ns namespaceId fuel origin τ .subtract failure left right
    -- A checked negation fails exactly where the checked subtraction from
    -- zero does, with the same result.
    | fuel + 1, origin, .int width signed, .checkedNegate failure, [operand] => do
        let zero ← compileLiteral (.int width signed) (.integer 0)
        let operand ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).at?
          (.int width signed)
        .ok (.at (.int width signed) (.checked (loopSite namespaceId.index origin.index) .subtract failure zero operand))
    | fuel + 1, origin, τ, .checkedMultiply failure, [left, right] =>
        compileChecked unit loanFacts ρ Γ ns namespaceId fuel origin τ .multiply failure left right
    | fuel + 1, origin, τ, .checkedDivide failure, [left, right] =>
        compileChecked unit loanFacts ρ Γ ns namespaceId fuel origin τ .divide failure left right
    | fuel + 1, origin, τ, .checkedModulo failure, [left, right] =>
        compileChecked unit loanFacts ρ Γ ns namespaceId fuel origin τ .modulo failure left right
    | fuel + 1, origin, τ, .less, [left, right] =>
        compileCompare unit loanFacts ρ Γ ns namespaceId fuel τ .less left right
    | fuel + 1, origin, τ, .greater, [left, right] =>
        compileCompare unit loanFacts ρ Γ ns namespaceId fuel τ .greater left right
    | fuel + 1, origin, τ, .lessEqual, [left, right] =>
        compileCompare unit loanFacts ρ Γ ns namespaceId fuel τ .lessEqual left right
    | fuel + 1, origin, τ, .greaterEqual, [left, right] =>
        compileCompare unit loanFacts ρ Γ ns namespaceId fuel τ .greaterEqual left right
    | fuel + 1, origin, .bool, .equal, [left, right] => do
        let ⟨σ, left⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).some
        -- `NTy.eqb` is equality only on a ref-free type; at a reference it is
        -- `false` however the executor compares the borrows. Decline rather
        -- than carry an equality the execution does not agree with.
        unless σ.refFree do notCarried "an equality of a value holding a reference"
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? σ
        .ok (.at .bool (if comparesErased unit σ then .equalErased false left right
          else .equal false left right))
    | fuel + 1, origin, .bool, .notEqual, [left, right] => do
        let ⟨σ, left⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).some
        unless σ.refFree do notCarried "an inequality of a value holding a reference"
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? σ
        .ok (.at .bool (if comparesErased unit σ then .equalErased true left right
          else .equal true left right))
    | fuel + 1, origin, .bool, .logicalNot, [operand] => do
        let operand ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).at? .bool
        .ok (.at .bool (.not operand))
    | fuel + 1, origin, .bool, .logicalAnd, [left, right] => do
        let left ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).at? .bool
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? .bool
        .ok (.at .bool (.logical true left right))
    | fuel + 1, origin, .bool, .logicalOr, [left, right] => do
        let left ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).at? .bool
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? .bool
        .ok (.at .bool (.logical false left right))
    | fuel + 1, origin, .int width false, .bitwiseAnd, [left, right] => do
        let left ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).at? (.int width false)
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? (.int width false)
        .ok (.at (.int width false) (.bitwise .and left right))
    | fuel + 1, origin, .int width false, .bitwiseOr, [left, right] => do
        let left ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).at? (.int width false)
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? (.int width false)
        .ok (.at (.int width false) (.bitwise .or left right))
    | fuel + 1, origin, .int width false, .bitwiseXor, [left, right] => do
        let left ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).at? (.int width false)
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? (.int width false)
        .ok (.at (.int width false) (.bitwise .xor left right))
    | fuel + 1, origin, .int width true, .bitwiseAnd, [left, right] =>
        compileSignedBitwise unit loanFacts ρ Γ ns namespaceId fuel width .and left right
    | fuel + 1, origin, .int width true, .bitwiseOr, [left, right] =>
        compileSignedBitwise unit loanFacts ρ Γ ns namespaceId fuel width .or left right
    | fuel + 1, origin, .int width true, .bitwiseXor, [left, right] =>
        compileSignedBitwise unit loanFacts ρ Γ ns namespaceId fuel width .xor left right
    | fuel + 1, origin, .bool, .bitwiseAnd, [left, right] => do
        let left ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).at? .bool
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? .bool
        .ok (.at .bool (.logical true left right))
    | fuel + 1, origin, .bool, .bitwiseOr, [left, right] => do
        let left ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).at? .bool
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? .bool
        .ok (.at .bool (.logical false left right))
    | fuel + 1, origin, .bool, .bitwiseXor, [left, right] => do
        let left ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).at? .bool
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? .bool
        .ok (.at .bool (.equal true left right))
    -- The complement of a bit pattern: `ones - value`, where ones reads as `-1`
    -- at a signed type; the wrap is idle.
    | fuel + 1, origin, .int width signed, .bitwiseNot, [operand] => do
        let ones ← compileLiteral (.int width signed)
          (.integer (if signed then -1 else 2 ^ width - 1))
        let operand ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).at?
          (.int width signed)
        .ok (.at (.int width signed) (.modular .subtract ones operand))
    | fuel + 1, origin, .int width signed, .negate, [operand] => do
        let zero ← compileLiteral (.int width signed) (.integer 0)
        let operand ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel operand).at?
          (.int width signed)
        .ok (.at (.int width signed) (.modular .subtract zero operand))
    | fuel + 1, origin, .int width signed, .divide, [left, right] => do
        let left ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).at? (.int width signed)
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? (.int width signed)
        .ok (.at (.int width signed)
          (.divideUnchecked (loopSite namespaceId.index origin.index) false left right))
    | fuel + 1, origin, .int width signed, .modulo, [left, right] => do
        let left ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).at? (.int width signed)
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? (.int width signed)
        .ok (.at (.int width signed)
          (.divideUnchecked (loopSite namespaceId.index origin.index) true left right))
    | fuel + 1, _, .tuple (.cons (.int width signed) (.cons .bool .nil)), .overflowingAdd, [left, right] =>
        compileOverflowing unit loanFacts ρ Γ ns namespaceId fuel width signed .add left right
    | fuel + 1, _, .tuple (.cons (.int width signed) (.cons .bool .nil)), .overflowingSubtract, [left, right] =>
        compileOverflowing unit loanFacts ρ Γ ns namespaceId fuel width signed .subtract left right
    | fuel + 1, _, .tuple (.cons (.int width signed) (.cons .bool .nil)), .overflowingMultiply, [left, right] =>
        compileOverflowing unit loanFacts ρ Γ ns namespaceId fuel width signed .multiply left right
    | fuel + 1, origin, τ, .shiftLeft, [value, distance] =>
        compileUncheckedShift unit loanFacts ρ Γ ns namespaceId fuel origin τ true value distance
    | fuel + 1, origin, τ, .shiftRight, [value, distance] =>
        compileUncheckedShift unit loanFacts ρ Γ ns namespaceId fuel origin τ false value distance
    | fuel + 1, origin, .int width' signed', .cast, [value] => do
        let ⟨σ, value⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel value).some
        if nonzero : width' ≠ 0 then
          match σ, value with
          | .int _ _, value => .ok (.at (.int width' signed') (.wrap nonzero value))
          | .char, value => .ok (.at (.int width' signed') (.wrap nonzero (.charCode value)))
          | _, _ => notCarried "a cast from a non-integer"
        else .error "an integer has no width"
    | fuel + 1, origin, .char, .cast, [value] => do
        let ⟨σ, value⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel value).some
        match σ, value with
        | .int _ _, value => .ok (.at .char (.charOf (loopSite namespaceId.index origin.index) value))
        | _, _ => notCarried "a cast to a character from a non-integer"
    | fuel + 1, origin, .char, .checkedCast failure, [value] => do
        let ⟨σ, value⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel value).some
        match σ, value with
        | .int _ _, value =>
            .ok (.at .char (.charOfChecked (loopSite namespaceId.index origin.index) failure value))
        | _, _ => notCarried "a cast to a character from a non-integer"
    | fuel + 1, origin, τ, .checkedShiftLeft failure, [value, distance] =>
        compileShift unit loanFacts ρ Γ ns namespaceId fuel origin τ true failure value distance
    | fuel + 1, origin, τ, .checkedShiftRight failure, [value, distance] =>
        compileShift unit loanFacts ρ Γ ns namespaceId fuel origin τ false failure value distance
    | fuel + 1, origin, .int width' signed', .checkedCast failure, [value] => do
        let ⟨σ, value⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel value).some
        match σ, value with
        | .int _ _, value => .ok (.at (.int width' signed') (.cast (loopSite namespaceId.index origin.index) failure value))
        | .char, value =>
            .ok (.at (.int width' signed') (.cast (loopSite namespaceId.index origin.index) failure (.charCode value)))
        | _, _ => notCarried "a cast from a non-integer"
    | fuel + 1, origin, .array τ length, .vector, elements => do
        let count := elements.length
        unless count = length do .error "an array literal's size differs from its type's"
        let elements ← compileArgs unit loanFacts ρ Γ ns namespaceId false fuel elements
          (NRow.replicate count τ)
        .ok (.at (.array τ length) (.arrayOf (.vectorLit count elements)))
    | fuel + 1, origin, .array τ length, .repeatVector, [value] => do
        let value ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel value).at? τ
        .ok (.at (.array τ length) (.arrayOf (.repeatVector length value)))
    | fuel + 1, origin, .vector τ, .vector, elements => do
        let count := elements.length
        let elements ← compileArgs unit loanFacts ρ Γ ns namespaceId false fuel elements
          (NRow.replicate count τ)
        .ok (.at (.vector τ) (.vectorLit count elements))
    | fuel + 1, origin, .address, .signerAddress, [signer] => do
        let ⟨σ, signer⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel signer).some
        match σ, signer with
        | .signer, signer => .ok (.at .address (.signerAddress signer))
        | _, _ => notCarried "a signer's address of a non-signer"
    | fuel + 1, origin, .int 64 false, .length, [vector] => do
        let ⟨σ, vector⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel vector).sequenceContents.some
        match σ, vector with
        | .vector _, vector => .ok (.at (.int 64 false) (.length vector))
        | .array _ _, array => .ok (.at (.int 64 false) (.length (.arrayVector array)))
        | .string, value => .ok (.at (.int 64 false) (.stringLength value))
        | _, _ => notCarried "a length of a non-vector"
    | fuel + 1, origin, _, .index, [vector, position] => do
        let ⟨σ, vector⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel vector).sequenceContents.some
        let ⟨π, position⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel position).some
        match σ, vector with
        | .vector τ, vector =>
            match π, position with
            | .int _ _, position => .ok (.at τ (.index (loopSite namespaceId.index origin.index) vector position))
            | _, _ => notCarried "an element read at a non-integer position"
        | .array τ _, array =>
            match π, position with
            | .int _ _, position =>
                .ok (.at τ (.index (loopSite namespaceId.index origin.index) (.arrayVector array) position))
            | _, _ => notCarried "an element read at a non-integer position"
        | _, _ => notCarried "an element read of a non-vector"
    | fuel + 1, origin, .unit, .checkVectorIndex failure, [vector, position] => do
        let ⟨σ, vector⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel vector).sequenceContents.some
        let ⟨π, position⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel position).some
        match σ, vector with
        | .vector _, vector =>
            match π, position with
            | .int _ _, position => .ok (.at .unit (.checkIndex (loopSite namespaceId.index origin.index) failure vector position))
            | _, _ => notCarried "a bounds check at a non-integer position"
        | .array _ _, array =>
            match π, position with
            | .int _ _, position =>
                .ok (.at .unit (.checkIndex (loopSite namespaceId.index origin.index) failure
                  (.arrayVector array) position))
            | _, _ => notCarried "a bounds check at a non-integer position"
        | _, _ => notCarried "a bounds check of a non-vector"
    | fuel + 1, origin, _, .pushVector, [vector, element] => do
        let ⟨σ, vector⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel vector).some
        match σ, vector with
        | .vector τ, vector => do
            let element ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel element).at? τ
            .ok (.at (.vector τ) (.push vector element))
        | _, _ => notCarried "a push onto a non-vector"
    | fuel + 1, origin, _, .insertVector, [vector, position, element] => do
        let ⟨σ, vector⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel vector).some
        let ⟨π, position⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel position).some
        match σ, vector with
        | .vector τ, vector =>
            match π, position with
            | .int _ _, position => do
                let element ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel element).at? τ
                .ok (.at (.vector τ) (.insert (loopSite namespaceId.index origin.index) vector position element))
            | _, _ => notCarried "an insertion at a non-integer position"
        | _, _ => notCarried "an insertion into a non-vector"
    | fuel + 1, origin, _, .removeVector, [vector, position] => do
        let ⟨σ, vector⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel vector).some
        let ⟨π, position⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel position).some
        match σ, vector with
        | .vector τ, vector =>
            match π, position with
            | .int _ _, position =>
                .ok (.at (.tuple (.cons τ (.cons (.vector τ) .nil))) (.remove (loopSite namespaceId.index origin.index) vector position))
            | _, _ => notCarried "a removal at a non-integer position"
        | _, _ => notCarried "a removal from a non-vector"
    | fuel + 1, origin, _, .swapVector, [vector, left, right] => do
        let ⟨σ, vector⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel vector).some
        let ⟨π, left⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).some
        match σ, vector with
        | .vector τ, vector =>
          match π, left with
          | .int width signed, left => do
              let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at?
                (.int width signed)
              .ok (.at (.vector τ) (.swap (loopSite namespaceId.index origin.index) vector left right))
          | _, _ => notCarried "a swap at non-integer positions"
        | _, _ => notCarried "a swap in a non-vector"
    | fuel + 1, origin, _, .concatVector, [left, right] => do
        let ⟨σ, left⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).some
        match σ, left with
        | .vector τ, left => do
            let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? (.vector τ)
            .ok (.at (.vector τ) (.concat left right))
        | _, _ => notCarried "a concatenation of non-vectors"
    | fuel + 1, origin, _, .slice, [vector, start, stop] => do
        let ⟨σ, vector⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel vector).some
        let ⟨π, start⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel start).some
        match σ, vector with
        | .vector τ, vector =>
          match π, start with
          | .int width signed, start => do
              let stop ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel stop).at?
                (.int width signed)
              .ok (.at (.vector τ) (.slice (loopSite namespaceId.index origin.index) vector start stop))
          | _, _ => notCarried "a slice at non-integer positions"
        | _, _ => notCarried "a slice of a non-vector"
    | fuel + 1, origin, _, .reverseSliceVector, [vector, start, stop] => do
        let ⟨σ, vector⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel vector).some
        let ⟨π, start⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel start).some
        match σ, vector with
        | .vector τ, vector =>
          match π, start with
          | .int width signed, start => do
              let stop ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel stop).at?
                (.int width signed)
              .ok (.at (.vector τ) (.reverseSlice (loopSite namespaceId.index origin.index) vector start stop))
          | _, _ => notCarried "a reversal at non-integer positions"
        | _, _ => notCarried "a reversal of a non-vector"
    | fuel + 1, origin, .unit, .destroyEmptyVector, [vector] => do
        let ⟨σ, vector⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel vector).some
        match σ, vector with
        | .vector _, vector => .ok (.at .unit (.destroyEmpty (loopSite namespaceId.index origin.index) vector))
        | _, _ => notCarried "a destruction of a non-vector"
    | fuel + 1, origin, _, .containsVector, [vector, needle] => do
        let ⟨σ, vector⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel vector).sequenceContents.some
        match σ, vector with
        | .vector τ, vector => do
            -- Both searches compare elements with `NTy.eqb`, which is `false`
            -- at a reference whatever the executor's value equality reports.
            unless τ.refFree do notCarried "a search for a value holding a reference"
            let needle ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel needle).at? τ
            .ok (.at .bool (.contains vector needle))
        | _, _ => notCarried "a search in a non-vector"
    | fuel + 1, origin, _, .indexOfVector, [vector, needle] => do
        let ⟨σ, vector⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel vector).sequenceContents.some
        match σ, vector with
        | .vector τ, vector => do
            unless τ.refFree do notCarried "a search for a value holding a reference"
            let needle ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel needle).at? τ
            .ok (.at (.tuple (.cons .bool (.cons (.int 64 false) .nil))) (.indexOf vector needle))
        | _, _ => notCarried "a search in a non-vector"
    | fuel + 1, origin, .int 8 true, .compare, [left, right] => do
        let ⟨σ, left⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).some
        -- A reference is carried as its (current, prophecy) pair, so the
        -- denotation would order it by its contents. The executor orders a
        -- borrow by its loan before its contents, which the pair does not
        -- record, so the order of a value holding one is not carried.
        unless σ.refFree do notCarried "a structural order of a value holding a reference"
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? σ
        .ok (.at (.int 8 true) (.order ns.source.orders left right))
    | _ + 1, _, _, primitive, _ => notCarried s!"primitive {repr primitive} at this type or arity"

  def compileChecked (unit : ValidatedUnit) (loanFacts : Array CheckedLoanFact)
      (ρ : ResultShape) (Γ : NRow) (ns : CompileNamespace) (namespaceId : NamespaceId) :
      Nat → ExprId → NTy → CheckedOp → ThrowKind → ExprId → ExprId → Except String (Compiled unit ρ Γ)
    | 0, _, _, _, _, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, origin, .int width signed, op, failure, left, right => do
        let left ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).at? (.int width signed)
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? (.int width signed)
        .ok (.at (.int width signed) (.checked (loopSite namespaceId.index origin.index) op failure left right))
    | _ + 1, _, _, _, _, _, _ => notCarried "checked arithmetic at a non-integer type"

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
        | .char, left => do
            let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? .char
            .ok (.at .bool (.compare op (.charCode left) (.charCode right)))
        -- Booleans order `false` before `true`.
        | .bool, left => do
            let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? .bool
            .ok (.at .bool (match op with
              | .less => .logical true (.not left) right
              | .greater => .logical true left (.not right)
              | .lessEqual => .logical false (.not left) right
              | .greaterEqual => .logical false left (.not right)))
        | _, _ => notCarried "a comparison of this type"
    | _ + 1, _, _, _, _ => notCarried "a comparison at a non-Boolean type"

  def compileShift (unit : ValidatedUnit) (loanFacts : Array CheckedLoanFact)
      (ρ : ResultShape) (Γ : NRow) (ns : CompileNamespace) (namespaceId : NamespaceId) :
      Nat → ExprId → NTy → Bool → ThrowKind → ExprId → ExprId → Except String (Compiled unit ρ Γ)
    | 0, _, _, _, _, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, origin, .int width signed, left, failure, value, distance => do
        let value ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel value).at? (.int width signed)
        let ⟨σ, distance⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel distance).some
        let site := loopSite namespaceId.index origin.index
        match σ, distance with
        | .int _ true, distance =>
            .ok (.at (.int width signed) (.shiftSigned site left failure value distance))
        | .int _ false, distance =>
            match signed, value with
            | false, value => .ok (.at (.int width false) (.shift site left failure value distance))
            | true, value => .ok (.at (.int width true) (.shiftSigned site left failure value distance))
        | _, _ => notCarried "a shift by a non-integer distance"
    | _ + 1, _, _, _, _, _, _ => notCarried "a shift of a non-integer"

  def compileUncheckedShift (unit : ValidatedUnit) (loanFacts : Array CheckedLoanFact)
      (ρ : ResultShape) (Γ : NRow) (ns : CompileNamespace) (namespaceId : NamespaceId) :
      Nat → ExprId → NTy → Bool → ExprId → ExprId → Except String (Compiled unit ρ Γ)
    | 0, _, _, _, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, origin, .int width signed, left, value, distance => do
        let value ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel value).at? (.int width signed)
        let ⟨σ, distance⟩ := (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel distance).some
        match σ, distance with
        | .int _ _, distance =>
            .ok (.at (.int width signed)
              (.shiftUnchecked (loopSite namespaceId.index origin.index) left value distance))
        | _, _ => notCarried "a shift by a non-integer distance"
    | _ + 1, _, _, _, _, _ => notCarried "a shift of a non-integer"

  def compileSignedBitwise (unit : ValidatedUnit) (loanFacts : Array CheckedLoanFact)
      (ρ : ResultShape) (Γ : NRow) (ns : CompileNamespace) (namespaceId : NamespaceId) :
      Nat → (width : Nat) → BitOp → ExprId → ExprId → Except String (Compiled unit ρ Γ)
    | 0, _, _, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, width, op, left, right => do
        let left ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).at? (.int width true)
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? (.int width true)
        .ok (.at (.int width true) (← signedBitwise op left right))

  def compileOverflowing (unit : ValidatedUnit) (loanFacts : Array CheckedLoanFact)
      (ρ : ResultShape) (Γ : NRow) (ns : CompileNamespace) (namespaceId : NamespaceId) :
      Nat → (width : Nat) → (signed : Bool) → ModularOp → ExprId → ExprId →
        Except String (Compiled unit ρ Γ)
    | 0, _, _, _, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, width, signed, op, left, right => do
        let left ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel left).at? (.int width signed)
        let right ← (← compileExpr unit loanFacts ρ Γ ns namespaceId fuel right).at? (.int width signed)
        .ok (.at (.tuple (.cons (.int width signed) (.cons .bool .nil))) (.overflowing op left right))
end


/-- The compilation view of a namespace. A certificate states it once per
namespace, so a function's certificate does not rebuild its arenas. -/
def compileNamespaceAt? (unit : ValidatedUnit) (namespaceId : NamespaceId) :
    Option CompileNamespace :=
  (unit.namespaces[namespaceId.index]?).map (CompileNamespace.of unit namespaceId)

/-- `CompileNamespace.transfers`, of a function of a unit. -/
def transfersAt (unit : ValidatedUnit) (handle : FunctionHandle) (θ : NRow) : Bool :=
  (compileNamespaceAt? unit handle.namespaceId).any (·.transfers unit handle θ)

mutual
  /-- The paths to the mutable references a value holds, not through one. -/
  def NTy.referencePaths : (τ : NTy) → Except String (List ((σ : NTy) × Proj τ (.ref σ)))
    | .ref σ => .ok [⟨σ, .nil⟩]
    | .sliceRef _ => .ok []
    | .tuple elements => do
        return (← NRow.referencePaths elements).map fun ⟨σ, _, x, path⟩ => ⟨σ, .element x path⟩
    | .struct source arguments fields => do
        return (← NRow.referencePaths fields).map fun ⟨σ, _, x, path⟩ =>
          ⟨σ, .field (source := source) (arguments := arguments) x path⟩
    | .enum source arguments names rows distinct => do
        return (← NRows.referencePaths names rows).map fun ⟨σ, _, _, which, x, path⟩ =>
          ⟨σ, .variant (source := source) (arguments := arguments) (distinct := distinct)
            (.cons which x .nil) path⟩
    | τ => if τ.refFree then .ok [] else notCarried "a reference inside an aggregate"

  /-- The paths to the mutable references a row's values hold. -/
  def NRow.referencePaths : (row : NRow) →
      Except String (List ((σ : NTy) × (τ : NTy) × Var row τ × Proj τ (.ref σ)))
    | .nil => .ok []
    | .cons τ rest => do
        let here := (← NTy.referencePaths τ).map fun ⟨σ, path⟩ => ⟨σ, τ, .here, path⟩
        let later := (← NRow.referencePaths rest).map fun ⟨σ, ν, x, path⟩ => ⟨σ, ν, .there x, path⟩
        return here ++ later

  /-- The paths to the mutable references an enum's variants hold. -/
  def NRows.referencePaths : (names : List String) → (rows : NRows) →
      Except String (List ((σ : NTy) × (ν : NTy) × (fields : NRow) ×
        Which names rows fields × Var fields ν × Proj ν (.ref σ)))
    | _, .nil => .ok []
    | [], .cons _ _ => .ok []
    | _ :: names, .cons fields rest => do
        let here := (← NRow.referencePaths fields).map fun ⟨σ, ν, x, path⟩ =>
          ⟨σ, ν, fields, .here, x, path⟩
        let later := (← NRows.referencePaths names rest).map fun ⟨σ, ν, row, which, x, path⟩ =>
          ⟨σ, ν, row, .there which, x, path⟩
        return here ++ later
end

mutual
  /-- The paths to the mutable references a value holds, not through one. -/
  def NTy.sliceReferencePaths : (τ : NTy) → Except String (List ((σ : NTy) × Proj τ (.sliceRef σ)))
    | .sliceRef σ => .ok [⟨σ, .nil⟩]
    | .tuple elements => do
        return (← NRow.sliceReferencePaths elements).map fun ⟨σ, _, x, path⟩ => ⟨σ, .element x path⟩
    | .struct source arguments fields => do
        return (← NRow.sliceReferencePaths fields).map fun ⟨σ, _, x, path⟩ =>
          ⟨σ, .field (source := source) (arguments := arguments) x path⟩
    | .enum source arguments names rows distinct => do
        return (← NRows.sliceReferencePaths names rows).map fun ⟨σ, _, _, which, x, path⟩ =>
          ⟨σ, .variant (source := source) (arguments := arguments) (distinct := distinct)
            (.cons which x .nil) path⟩
    | .ref _ => .ok []
    | τ => if τ.refFree then .ok [] else notCarried "a reference inside an aggregate"

  /-- The paths to the mutable references a row's values hold. -/
  def NRow.sliceReferencePaths : (row : NRow) →
      Except String (List ((σ : NTy) × (τ : NTy) × Var row τ × Proj τ (.sliceRef σ)))
    | .nil => .ok []
    | .cons τ rest => do
        let here := (← NTy.sliceReferencePaths τ).map fun ⟨σ, path⟩ => ⟨σ, τ, .here, path⟩
        let later := (← NRow.sliceReferencePaths rest).map fun ⟨σ, ν, x, path⟩ => ⟨σ, ν, .there x, path⟩
        return here ++ later

  /-- The paths to the mutable references an enum's variants hold. -/
  def NRows.sliceReferencePaths : (names : List String) → (rows : NRows) →
      Except String (List ((σ : NTy) × (ν : NTy) × (fields : NRow) ×
        Which names rows fields × Var fields ν × Proj ν (.sliceRef σ)))
    | _, .nil => .ok []
    | [], .cons _ _ => .ok []
    | _ :: names, .cons fields rest => do
        let here := (← NRow.sliceReferencePaths fields).map fun ⟨σ, ν, x, path⟩ =>
          ⟨σ, ν, fields, .here, x, path⟩
        let later := (← NRows.sliceReferencePaths names rest).map fun ⟨σ, ν, row, which, x, path⟩ =>
          ⟨σ, ν, row, .there which, x, path⟩
        return here ++ later
end

/-- The mutable references among the `count` slots from `index`, in their
slots or at paths in them. -/
def mutableSlots (Γ : NRow) : Nat → Nat → Except String (Mutables Γ)
  | _, 0 => .ok .nil
  | index, count + 1 => do
      let rest ← mutableSlots Γ (index + 1) count
      match Var.ofIndex Γ index with
      | some ⟨.ref _, x⟩ => .ok (.cons x rest)
      | some ⟨τ, x⟩ =>
          if τ.refFree then .ok rest else
          let rest := (← NTy.referencePaths τ).foldr (fun ⟨_, path⟩ rest => .at x path rest) rest
          return (← NTy.sliceReferencePaths τ).foldr (fun ⟨_, path⟩ rest => .sliceAt x path rest) rest
      | none => .error "a parameter is out of range"

/-- A body with the data invariants of every mutable-reference parameter's
current assumed before it: implicit in the parameter's type and owed by the
caller where it handed the reference over (`Term.assumeCurrent`). A referent
with no certificates needs no assumption (`Term.denote_assumeCurrent_certificateFree`). -/
def Mutables.assumedAtEntry {ρ : ResultShape} {Γ : NRow} {τ : NTy} :
    Mutables Γ → Term unit ρ Γ τ → Term unit ρ Γ τ
  | .nil, body => body
  | .cons (τ := τ) x rest, body =>
      if τ.certificateFree then rest.assumedAtEntry body
      else .drop (.assumeCurrent x) (rest.assumedAtEntry body)
  -- A reference an aggregate slot holds at a path has no slot of its own for
  -- `Term.assumeCurrent` to name.
  | .at _ _ rest, body => rest.assumedAtEntry body
  | .sliceAt _ _ rest, body => rest.assumedAtEntry body

/-- The compiled function a handle names in its namespace's view, or the
construct that stops it. -/
def compileFunctionIn (unit : ValidatedUnit) (view : CompileNamespace) (handle : FunctionHandle) :
    Except String (Function unit) := do
  let some declaration := view.source.functions[handle.functionId.index]?
    | .error "a function is out of range"
  let .structured root := declaration.body | notCarried "a function without a body"
  let some allLocals := view.localTypes? unit handle.namespaceId declaration
    | match declaration.locals.toList.findSome? fun localDecl =>
          let typeId := view.source.typeId localDecl.type.typeId
          if (view.nty unit handle.namespaceId typeId).isSome then none
          else if logicalOnly unit handle.namespaceId typeId then none
          else nominalIn? unit (typeFuel unit) handle.namespaceId typeId with
      | some name => .error s!"recursion through `{name}` is not carried by the denotation"
      | none => notCarried "the type of a local"
  let paramCount := declaration.signature.parameters.size
  if paramCount > allLocals.length then .error "fewer locals than parameters" else
  let params := NRow.ofList (allLocals.take paramCount)
  let locals := NRow.ofList (allLocals.drop paramCount)
  let some paramTypes := declaration.signature.parameters.toList.mapM fun parameter =>
      view.nty unit handle.namespaceId (view.source.typeId parameter.typeUse.typeId)
    | notCarried "the type of a parameter"
  if NRow.ofList paramTypes != params then
    .error "parameter types differ from the leading locals" else
  let result ← match declaration.signature.results.toList with
    | [] => .ok ResultShape.none
    | [result] => match view.nty unit handle.namespaceId (view.source.typeId result.typeId) with
        | some τ => .ok (ResultShape.one τ)
        | none => notCarried "the type of the result"
    | _ => notCarried "several results"
  unless paramTypes.all NTy.lendable && result.lendable do
    notCarried "a reference inside an aggregate"
  -- An element index of a mutable borrow is read again where the loan dies,
  -- so a write to the local it comes from while the loan is active would
  -- resolve the reference at another element.  The borrow analysis decides
  -- that with its own liveness and records it (`writesIndexUnderLoan`).
  if writesIndexUnderLoan unit handle then
    notCarried "a write to a local a live mutable borrow uses as an element index"
  let frame := params ++ locals
  let body ← compileExpr unit (loanFactsOf unit handle) result frame
    view handle.namespaceId
    view.fuel root
  let body ← body.at? result.bodyType
  -- Every mutable-reference parameter's current has its data invariants at
  -- entry, as the Move Prover assumes a parameter well-formed deep through
  -- a reference, and the caller owes them at the hand-over.
  let parameterReferences ← mutableSlots frame 0 paramCount
  let body := parameterReferences.assumedAtEntry body
  -- A parameter's reference resolves where the function leaves it: in its
  -- own slot or in a local it was moved into.
  let mutables ← mutableSlots frame 0 allLocals.length
  -- The function's exit, where a mutable parameter's loan ends and its data
  -- invariant is owed (`Mutables.resolve`).
  let exitSite := loopSite handle.namespaceId.index root.index
  -- A mutable reference the function returns owes its referent's data
  -- invariants there too (`Term.returned`): the slot it left is empty, so no
  -- resolution at the exit reaches it, while the caller reads it as a
  -- reference whose invariants hold.  Stated here, over the result shape the
  -- compiler knows, so the denotation keeps no case on that shape.
  let body : Term unit result frame result.bodyType := certifiedResult exitSite body
  .ok { params, locals, frame, frame_eq := rfl, result, body, mutables, exitSite,
        scopedMethods := scopedMethodIndices declaration.attributes }

/-- The location of the innermost node of a function's body a compilation's
rejection comes from, for its message; `none` where the function as a whole is
rejected. -/
partial def rejectionSite? (unit : ValidatedUnit) (handle : FunctionHandle) (reason : String) :
    Option LocId := do
  let view ← compileNamespaceAt? unit handle.namespaceId
  let declaration ← view.source.functions[handle.functionId.index]?
  let .structured root := declaration.body | none
  let Γ := NRow.ofList (← view.localTypes? unit handle.namespaceId declaration)
  let ρ ← match declaration.signature.results.toList with
    | [] => some ResultShape.none
    | [result] => ResultShape.one <$> view.nty unit handle.namespaceId result.typeId
    | _ => none
  let rec search (id : ExprId) : Option LocId := do
    let expression ← view.expressions.get? id.index
    if expression.kind matches .spec _ then none
    match (expressionChildren expression.kind).findSome? search with
    | some site => some site
    | none =>
        match compileExpr unit (loanFactsOf unit handle) ρ Γ view handle.namespaceId view.fuel id with
        | .error rejection => if rejection == reason then some expression.loc else none
        | .ok _ => none
  search root

/-- The compiled function a handle names, or the construct that stops it:
its denotation, which agrees with its runtime meaning. A unit declaring
ghost fields has none: its model holds ghost state the runtime does
not, and is verified through the model (`doc/designs/ghost-fields.md`). -/
def compileFunction (unit : ValidatedUnit)
    (handle : FunctionHandle) : Except String (Function unit) :=
  if declaresGhosts unit then .error "a unit declaring ghost fields is verified through its model"
  else match compileNamespaceAt? unit handle.namespaceId with
  | none => .error "a namespace is out of range"
  | some view => compileFunctionIn unit view handle

/-- A function's model in a unit declaring ghost fields: its denotation with
the ghost state the runtime does not hold, which the unit's verification
reads instead of its runtime meaning (`doc/designs/ghost-fields.md`). -/
def compileModelFunction (unit : ValidatedUnit)
    (handle : FunctionHandle) : Except String (Function unit) :=
  match compileNamespaceAt? unit handle.namespaceId with
  | none => .error "a namespace is out of range"
  | some view => compileFunctionIn unit view handle

/-- A namespace's view read at type arguments. -/
def CompileNamespace.at (view : CompileNamespace) (θ : TypeArgs) : CompileNamespace :=
  { view with substitution := some θ.1 }

/-- A function compiled at a substitution: `compileFunction` without one. At type
arguments `θ` its body is read at them, under the same frames and certificates. -/
def compileFunctionAt (unit : ValidatedUnit) (handle : FunctionHandle) :
    Option TypeArgs → Except String (Function unit)
  | none => compileFunction unit handle
  | some θ =>
      if declaresGhosts unit then
        .error "a unit declaring ghost fields is verified through its model"
      else match compileNamespaceAt? unit handle.namespaceId with
      | none => .error "a namespace is out of range"
      | some view => compileFunctionIn unit (view.at θ) handle

theorem compileFunctionAt_of_view {unit : ValidatedUnit} {handle : FunctionHandle}
    {view : CompileNamespace} (noGhosts : declaresGhosts unit = false)
    (viewed : compileNamespaceAt? unit handle.namespaceId = some view) (θ : TypeArgs) :
    compileFunctionAt unit handle (some θ) = compileFunctionIn unit (view.at θ) handle := by
  simp [compileFunctionAt, noGhosts, viewed]

theorem compileFunction_of_view {unit : ValidatedUnit} {handle : FunctionHandle}
    {view : CompileNamespace} (noGhosts : declaresGhosts unit = false)
    (viewed : compileNamespaceAt? unit handle.namespaceId = some view) :
    compileFunction unit handle = compileFunctionIn unit view handle := by
  simp [compileFunction, noGhosts, viewed]

/-- `frameInstantiation` read through a namespace's view: its types indexed,
iterated as the list they are. -/
def frameInstantiationIn (view : CompileNamespace) (outer : Array (TypeId × TypeId))
    (typeArgs : Array GenericArgument) : Array (TypeId × TypeId) :=
  if typeArgs.isEmpty then #[] else
    VerexIR.SemanticOperations.invocationTypeInstantiationIn view.source view.types
      view.source.tables.types.toList outer typeArgs

theorem frameInstantiation_of_view {unit : ValidatedUnit} {handle : FunctionHandle}
    {view : CompileNamespace} (viewed : compileNamespaceAt? unit handle.namespaceId = some view)
    (outer : Array (TypeId × TypeId)) (typeArgs : Array GenericArgument) :
    frameInstantiation unit handle outer typeArgs = frameInstantiationIn view outer typeArgs := by
  simp only [compileNamespaceAt?, Option.map_eq_some_iff] at viewed
  obtain ⟨ns, found, rfl⟩ := viewed
  simp only [frameInstantiation, VerexIR.SemanticOperations.callTypeInstantiation,
    frameInstantiationIn, CompileNamespace.of, found, Array.map_eq_empty_iff, Array.isEmpty_iff,
    VerexIR.SemanticOperations.invocationTypeInstantiationIn_eq]

/-- A substitution leaves the frame instantiation as the view reads it: a frame is the
runtime's, over the source's own types. -/
theorem frameInstantiation_of_substituted_view {unit : ValidatedUnit} {handle : FunctionHandle}
    {view : CompileNamespace} (viewed : compileNamespaceAt? unit handle.namespaceId = some view)
    (substitution : Option NRow) (outer : Array (TypeId × TypeId))
    (typeArgs : Array GenericArgument) :
    frameInstantiation unit handle outer typeArgs =
      frameInstantiationIn { view with substitution } outer typeArgs :=
  frameInstantiation_of_view viewed outer typeArgs

/-- A view's types are its source's, indexed. -/
theorem types_of_view {unit : ValidatedUnit} {namespaceId : NamespaceId} {view : CompileNamespace}
    (viewed : compileNamespaceAt? unit namespaceId = some view) :
    view.types = .ofArray view.source.tables.types := by
  simp only [compileNamespaceAt?, Option.map_eq_some_iff] at viewed
  obtain ⟨ns, _, rfl⟩ := viewed
  rfl

/-- A substitution leaves a view's types its source's: `nty` applies it to what it reads. -/
theorem types_of_substituted_view {unit : ValidatedUnit} {namespaceId : NamespaceId}
    {view : CompileNamespace} (viewed : compileNamespaceAt? unit namespaceId = some view)
    (substitution : Option NRow) :
    ({ view with substitution } : CompileNamespace).types =
      .ofArray ({ view with substitution } : CompileNamespace).source.tables.types :=
  types_of_view (view := view) viewed

/-- The frame instantiation of a generic call, from a checked certificate: the
callee namespace's types as a literal table, its lifetimes' kinds indexed,
its key map, and the witnesses of this instantiation, all verified by the
kernel by evaluation, never by search. -/
theorem frameInstantiationIn_eq_of_check {view : CompileNamespace} {table : Array Ty}
    {kinds : Validation.IndexedArena LifetimeKind}
    {map : Validation.KeyMap} {witnesses : Array Validation.Witness}
    {tree : Validation.IndexedArena Validation.Witness}
    (typeArgs : Array GenericArgument) (nonempty : typeArgs.isEmpty = false)
    (viewTypes : view.types = .ofArray view.source.tables.types)
    (tableEq : view.source.tables.types = table)
    (kindsEq : Validation.IndexedArena.ofArray (Validation.lifetimeKinds view.source) = kinds)
    (correct : Validation.Correct view.types table.size map)
    (treeEq : Validation.IndexedArena.ofArray witnesses = tree)
    (checked : Validation.checkAll kinds view.types map
      (VerexIR.SemanticOperations.instantiateGenericArguments #[] typeArgs)
      tree table.size = true)
    (sizes : witnesses.size = table.size)
    (depths : witnesses.toList.all (fun witness => decide (witness.depth ≤ table.size)) = true) :
    frameInstantiationIn view #[] typeArgs = Validation.foldWitnesses tree table.size := by
  unfold frameInstantiationIn
  rw [nonempty, viewTypes, VerexIR.SemanticOperations.invocationTypeInstantiationIn_eq]
  simp only [Bool.false_eq_true, ↓reduceIte]
  rw [viewTypes, ← kindsEq] at checked
  rw [viewTypes] at correct
  rw [← tableEq] at checked correct sizes depths ⊢
  rw [← sizes] at checked
  exact Validation.invocationTypeInstantiation_eq_of_check #[] _ treeEq correct checked sizes depths

end VerexIR.Proofs.Denote
