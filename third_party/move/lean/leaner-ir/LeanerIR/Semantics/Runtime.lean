-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Validation.Capability

/-!
# Runtime domain for structured LIR

This module defines the profile-neutral runtime boundary shared by the
executable interpreter and the declarative semantics. M2 extends the original
scalar core with strings, bytes, vectors, nominal data, and closures while
retaining the state shape reserved by M1 for references and resources.
-/

namespace LeanerIR

open Validation

/-- Runtime identity of an executable declaration.  Unlike a `QualifiedRef`,
this pair indexes the already validated unit directly. -/
structure FunctionHandle where
  namespaceId : NamespaceId
  functionId : FunctionId
  deriving Repr, BEq, DecidableEq, Inhabited

/-- Runtime identity of a constant declaration in a validated unit. -/
structure ConstantHandle where
  namespaceId : NamespaceId
  constantId : Nat
  deriving Repr, DecidableEq, Inhabited

/-- Runtime identity of a nominal data declaration. -/
structure StructHandle where
  namespaceId : NamespaceId
  structId : Nat
  deriving Repr, DecidableEq, Inhabited

/-- One resolved step of a runtime place or value path. Field indexes refer
to the ordered payload of a nominal value; downcasts validate an enum
variant without changing the selected value; a dereference walks into the
current value a borrow owns. -/
inductive RuntimeProjection where
  | field (index : Nat)
  | index (index : Nat)
  | subslice (start stop : Nat) (fromEnd : Bool)
  | downcast (variant : String)
  | deref
  deriving Repr, BEq, DecidableEq, Inhabited

/-- Runtime values supported by the executable semantic core. Nominal values
carry their resolved qualified spelling so pattern matching remains stable
across namespace-local `NameId` tables.

References follow the prophetic ownership model
(`designs/prophetic-references.md`): a mutable borrow owns the current value
of its loan and leaves `loanHole` where the value was taken. `loan` is the
dynamic loan instance minted at the borrow; the lexically recorded `endLoan`
markers reunite hole and current at loan death. A shared reference is not a
runtime value at all — certified exclusivity makes the observed value itself
the reference, so `borrow` is always a mutable loan. -/
inductive RuntimeValue where
  | unit
  | bool (value : Bool)
  | character (value : Nat)
  | integer (value : Int)
  | address (value : String)
  | signer (value : String)
  | string (value : String)
  | bytes (value : Array UInt8)
  | vector (elements : Array RuntimeValue)
  | tuple (elements : Array RuntimeValue)
  | nominal (source : StructHandle) (variant : Option String)
      (fields : Array RuntimeValue)
  | closure (function : FunctionHandle) (captures : Array RuntimeValue)
  | borrow (loan : Nat) (current : RuntimeValue)
  | loanHole (loan : Nat)
  deriving Repr, Inhabited

/-! Structural equality of runtime values, as the `==` primitive decides
it.  A derived `BEq` on this nested inductive is opaque — `partial`
underneath — and so says nothing in a proof; this one is defined by
well-founded recursion over the value, so each constructor's equation is
a theorem. -/

/-- An array's size measure is one more than its list's. -/
private theorem Array.sizeOf_eq_toList {α : Type} [SizeOf α] (array : Array α) :
    sizeOf array = 1 + sizeOf array.toList := by
  cases array
  simp

mutual

/-- Structural equality of two runtime values. -/
def RuntimeValue.beq : RuntimeValue → RuntimeValue → Bool
  | .unit, .unit => true
  | .bool left, .bool right => left == right
  | .character left, .character right => left == right
  | .integer left, .integer right => left == right
  | .address left, .address right => left == right
  | .signer left, .signer right => left == right
  | .string left, .string right => left == right
  | .bytes left, .bytes right => left == right
  | .vector left, .vector right => RuntimeValue.beqList left.toList right.toList
  | .tuple left, .tuple right => RuntimeValue.beqList left.toList right.toList
  | .nominal leftSource leftVariant leftFields,
      .nominal rightSource rightVariant rightFields =>
      leftSource == rightSource && leftVariant == rightVariant &&
        RuntimeValue.beqList leftFields.toList rightFields.toList
  | .closure leftFunction leftCaptures, .closure rightFunction rightCaptures =>
      leftFunction == rightFunction &&
        RuntimeValue.beqList leftCaptures.toList rightCaptures.toList
  | .borrow leftLoan leftCurrent, .borrow rightLoan rightCurrent =>
      leftLoan == rightLoan && RuntimeValue.beq leftCurrent rightCurrent
  | .loanHole left, .loanHole right => left == right
  | _, _ => false
termination_by left right => sizeOf left + sizeOf right
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); omega)

/-- Structural equality of two value lists, elementwise. -/
def RuntimeValue.beqList : List RuntimeValue → List RuntimeValue → Bool
  | [], [] => true
  | left :: lefts, right :: rights =>
      RuntimeValue.beq left right && RuntimeValue.beqList lefts rights
  | _, _ => false
termination_by lefts rights => sizeOf lefts + sizeOf rights
decreasing_by all_goals (simp_wf; omega)

end

instance : BEq RuntimeValue := ⟨RuntimeValue.beq⟩

/-- A lawful `==` on a decidable type is the decision of equality. -/
private theorem beq_eq_decide {α : Type} [BEq α] [LawfulBEq α] [DecidableEq α]
    (left right : α) : (left == right) = decide (left = right) := by
  by_cases h : left = right
  · subst h
    simp
  · simp [h]

@[simp] theorem RuntimeValue.beq_integer (left right : Int) :
    (RuntimeValue.integer left == RuntimeValue.integer right) = decide (left = right) := by
  show RuntimeValue.beq _ _ = _
  unfold RuntimeValue.beq
  exact beq_eq_decide _ _

@[simp] theorem RuntimeValue.bne_integer (left right : Int) :
    (RuntimeValue.integer left != RuntimeValue.integer right) = decide (left ≠ right) := by
  simp [bne_eq, RuntimeValue.beq_integer]

@[simp] theorem RuntimeValue.beq_bool (left right : Bool) :
    (RuntimeValue.bool left == RuntimeValue.bool right) = decide (left = right) := by
  show RuntimeValue.beq _ _ = _
  unfold RuntimeValue.beq
  exact beq_eq_decide _ _

@[simp] theorem RuntimeValue.beq_address (left right : String) :
    (RuntimeValue.address left == RuntimeValue.address right) = decide (left = right) := by
  show RuntimeValue.beq _ _ = _
  unfold RuntimeValue.beq
  exact beq_eq_decide _ _

/-! ## Specification projections

Generated contracts speak about aggregates through total projections: a
specification is a proposition, so it cannot be partial, and a read outside
a value's shape returns a junk value rather than failing.  Junk is
unreachable in a proof that has ruled the corresponding failure out — the
declared abort condition does exactly that — while totality keeps the
clause an ordinary Lean term.  A borrow reads through to its content, so a
specification sees the same value whether or not the program borrowed. -/

/-- The field or element a value holds at one position. -/
def RuntimeValue.field : RuntimeValue → Nat → RuntimeValue
  | .borrow _ current, index => RuntimeValue.field current index
  | .nominal _ _ fields, index => fields[index]?.getD .unit
  | .tuple elements, index => elements[index]?.getD .unit
  | .vector elements, index => elements[index]?.getD .unit
  | _, _ => .unit

/-- The integer a value denotes; `0` off the integer shape. -/
def RuntimeValue.asInt : RuntimeValue → Int
  | .borrow _ current => RuntimeValue.asInt current
  | .integer value => value
  | .character value => Int.ofNat value
  | _ => 0

/-- The truth value a value denotes; `false` off the boolean shape. -/
def RuntimeValue.asBool : RuntimeValue → Bool
  | .borrow _ current => RuntimeValue.asBool current
  | .bool value => value
  | _ => false

/-- The text a value denotes; the empty string off the textual shapes. -/
def RuntimeValue.asString : RuntimeValue → String
  | .borrow _ current => RuntimeValue.asString current
  | .string value => value
  | .address value => value
  | .signer value => value
  | _ => ""

/-- The element count of an aggregate; `0` off the aggregate shapes. -/
def RuntimeValue.size : RuntimeValue → Nat
  | .borrow _ current => RuntimeValue.size current
  | .nominal _ _ fields => fields.size
  | .tuple elements => elements.size
  | .vector elements => elements.size
  | _ => 0

/-- The variant a value carries, if it is a variant-bearing nominal. -/
def RuntimeValue.variant? : RuntimeValue → Option String
  | .borrow _ current => RuntimeValue.variant? current
  | .nominal _ variant _ => variant
  | _ => none

/-! ## Storage keys

Global memory is a map, and a map's laws — reading back what was written,
and leaving every other key alone — need key equality to be decidable.
Runtime values as a whole do not have it: `RuntimeValue` recurses through
arrays, so its derived `BEq` is a `partial` definition with no equations,
and nothing can be proved through it.  Nor should it be: a closure or a live
borrow is not a key.  Storage keys are therefore their own type, covering
exactly the value shapes a profile can publish under. -/

/-- A value shape that can key global storage. -/
inductive StorageKey where
  | unit
  | bool (value : Bool)
  | character (value : Nat)
  | integer (value : Int)
  | address (value : String)
  | signer (value : String)
  | string (value : String)
  | bytes (value : Array UInt8)
  deriving Repr, BEq, DecidableEq, Inhabited

/-- The key a runtime value denotes, if it has a key shape.  Storage
operations reject anything else as malformed: validation types the key
operand of every global operation, so a well-formed unit never reaches it
with an aggregate, a closure, or a reference. -/
def RuntimeValue.storageKey? : RuntimeValue → Option StorageKey
  | .unit => some .unit
  | .bool value => some (.bool value)
  | .character value => some (.character value)
  | .integer value => some (.integer value)
  | .address value => some (.address value)
  -- A signer keys storage by the address it holds: publishing under a
  -- signer and reading under its address are the same location.
  | .signer value => some (.address value)
  | .string value => some (.string value)
  | .bytes value => some (.bytes value)
  | _ => none

/-- The key a runtime value denotes, `unit` off the key shapes.  A clause is
a proposition and so reads storage totally; this is the specification-side
counterpart of `storageKey?`, and the two agree wherever a program can
actually reach global memory. -/
def RuntimeValue.storageKey (value : RuntimeValue) : StorageKey :=
  value.storageKey?.getD .unit

theorem RuntimeValue.storageKey?_eq_some {value : RuntimeValue} {key : StorageKey}
    (shaped : value.storageKey? = some key) : value.storageKey = key := by
  simp [RuntimeValue.storageKey, shaped]

/-! ## Global memory

Global memory is a finite map from keys to published resources.  Its
operations are the vocabulary a specification speaks, so a program's read of
a key and a clause's read of the same key are one term with nothing between
them.  The array of entries is an implementation of the map; the laws below
are what execution and specification both rely on, and no consumer outside
this section looks at the entries. -/

/-- The identity of one published resource: which family, at which key. -/
structure GlobalKey where
  namespaceId : NamespaceId
  typeId : TypeId
  key : StorageKey
  deriving Repr, BEq, DecidableEq, Inhabited

/-- One published resource. -/
structure GlobalSlot where
  key : GlobalKey
  value : RuntimeValue
  deriving Repr, BEq, Inhabited

/-- Global memory: at most one resource per key. -/
structure GlobalMap where
  entries : Array GlobalSlot := #[]
  deriving Repr, BEq, Inhabited

namespace GlobalMap

/-- The resource published at a key. -/
def lookup (globals : GlobalMap) (key : GlobalKey) : Option RuntimeValue :=
  (globals.entries.find? (·.key = key)).map (·.value)

/-- Whether a resource is published at a key. -/
def contains (globals : GlobalMap) (key : GlobalKey) : Bool :=
  (globals.lookup key).isSome

/-- Remove whatever a key publishes. -/
def erase (globals : GlobalMap) (key : GlobalKey) : GlobalMap :=
  ⟨globals.entries.filter (·.key ≠ key)⟩

/-- Publish a resource at a key, replacing whatever the key held. -/
def insert (globals : GlobalMap) (key : GlobalKey) (value : RuntimeValue) : GlobalMap :=
  ⟨(globals.erase key).entries.push ⟨key, value⟩⟩

/-! The map laws.  Distinct keys are disjoint locations, which is what lets a
contract frame the global memory it does not modify. -/

private theorem listFind?_filter_ne {entries : List GlobalSlot} {written query : GlobalKey}
    (distinct : query ≠ written) :
    (entries.filter (·.key ≠ written)).find? (·.key = query)
      = entries.find? (·.key = query) := by
  induction entries with
  | nil => rfl
  | cons slot rest ih =>
      by_cases matched : slot.key = written
      · have missed : ¬ slot.key = query := fun eq => distinct (eq ▸ matched)
        rw [List.filter_cons_of_neg (by simp [matched]),
          List.find?_cons_of_neg (by simp [missed]), ih]
      · rw [List.filter_cons_of_pos (by simp [matched])]
        by_cases hit : slot.key = query
        · rw [List.find?_cons_of_pos (by simp [hit]),
            List.find?_cons_of_pos (by simp [hit])]
        · rw [List.find?_cons_of_neg (by simp [hit]),
            List.find?_cons_of_neg (by simp [hit]), ih]

private theorem listFind?_filter_self {entries : List GlobalSlot} {key : GlobalKey} :
    (entries.filter (·.key ≠ key)).find? (·.key = key) = none := by
  induction entries with
  | nil => rfl
  | cons slot rest ih =>
      by_cases matched : slot.key = key
      · rw [List.filter_cons_of_neg (by simp [matched]), ih]
      · rw [List.filter_cons_of_pos (by simp [matched]),
          List.find?_cons_of_neg (by simp [matched]), ih]

private theorem find?_filter_ne {entries : Array GlobalSlot} {written query : GlobalKey}
    (distinct : query ≠ written) :
    (entries.filter (·.key ≠ written)).find? (·.key = query)
      = entries.find? (·.key = query) := by
  rw [← Array.find?_toList, ← Array.find?_toList, Array.toList_filter]
  exact listFind?_filter_ne distinct

private theorem find?_filter_self {entries : Array GlobalSlot} {key : GlobalKey} :
    (entries.filter (·.key ≠ key)).find? (·.key = key) = none := by
  rw [← Array.find?_toList, Array.toList_filter]
  exact listFind?_filter_self

@[simp] theorem lookup_insert_self (globals : GlobalMap) (key : GlobalKey)
    (value : RuntimeValue) : (globals.insert key value).lookup key = some value := by
  simp only [lookup, insert, erase, Array.find?_push, find?_filter_self, Option.none_or,
    decide_true, ↓reduceIte, Option.map_some]

@[simp] theorem lookup_insert_other (globals : GlobalMap) (written query : GlobalKey)
    (value : RuntimeValue) (distinct : query ≠ written) :
    (globals.insert written value).lookup query = globals.lookup query := by
  simp only [lookup, insert, erase, Array.find?_push, find?_filter_ne distinct,
    decide_eq_false (fun equal : written = query => distinct equal.symm),
    Bool.false_eq_true, ↓reduceIte, Option.or_none]

@[simp] theorem lookup_erase_self (globals : GlobalMap) (key : GlobalKey) :
    (globals.erase key).lookup key = none := by
  simp only [lookup, erase, find?_filter_self, Option.map_none]

@[simp] theorem lookup_erase_other (globals : GlobalMap) (written query : GlobalKey)
    (distinct : query ≠ written) :
    (globals.erase written).lookup query = globals.lookup query := by
  simp only [lookup, erase, find?_filter_ne distinct]

@[simp] theorem lookup_empty (key : GlobalKey) : GlobalMap.lookup {} key = none := by
  show ((#[] : Array GlobalSlot).find? (·.key = key)).map (·.value) = none
  rw [← Array.find?_toList]
  rfl

/-! The map is an abstraction, and these laws are its whole interface.
Sealing the operations keeps it that way: against a state a contract
quantifies over, none of them has a normal form, and unfolding one would
expose the search or filter behind the map to every reduction that walks
past.  A write therefore stays the folded `insert`/`erase` the laws read. -/

attribute [irreducible] lookup erase insert

end GlobalMap

/-- Root of a resolved runtime place: a frame local or a key of global
memory.  Dereferences are projections into the borrow value stored at the
root.  A global root is the key itself rather than a position in memory, so
reading one is the same lookup a specification writes. -/
inductive RuntimePlaceRoot where
  | local (localId : LocalId)
  | global (key : GlobalKey)
  deriving Repr, BEq, Inhabited

/-- Fully resolved storage target used by the shared executable semantics.
`writable` records whether an enclosing dereference grants mutable access. -/
structure RuntimePlace where
  root : RuntimePlaceRoot
  projections : Array RuntimeProjection := #[]
  writable : Bool := true
  deriving Repr, BEq, Inhabited

/-- Function-local storage.  `none` is a declared but not yet initialized
local, which lets the interpreter diagnose reads before initialization.
`activeLoans` maps a lexical loan site to its live dynamic instance, so
the site's `endLoan` marker finds the loan it ends; a site borrowed again
in a later loop iteration overwrites its entry.  `loanLocations` is the
native address cache for dynamic loans visible in this frame.  It lets
mutation and callee write-back revisit the already resolved place instead
of searching every local value for the loan or its hole. -/
structure RuntimeFrame where
  locals : Array (Option RuntimeValue) := #[]
  activeLoans : Array (ExprId × Nat) := #[]
  loanLocations : Array (Nat × RuntimePlace) := #[]
  /-- Declaration-local type ids mapped to the concrete members selected by
  this invocation.  The map is computed once at a generic call boundary;
  ordinary functions carry the empty identity map. -/
  typeInstantiation : Array (TypeId × TypeId) := #[]
  deriving Repr, BEq, Inhabited

/-- Whole runtime state threaded through expressions and calls.  `nextLoan`
mints dynamic loan instances.  `globalLoans` registers each live mutable
borrow of a global slot with the key whose slot holds its hole, so the
loan's death writes back by key instead of searching global memory — the
certified exclusivity of a live loan is what makes the recorded key the
hole's location.  `pending` carries loan write-backs whose holes live in an
ancestor frame: a callee's dying borrow exports its current value here, and
every caller applies what becomes visible at its call site. -/
structure RuntimeState where
  globals : GlobalMap := {}
  globalLoans : List (Nat × GlobalKey) := []
  nextLoan : Nat := 0
  pending : Array (Nat × RuntimeValue) := #[]
  deriving Repr, BEq, Inhabited

/-- Source location paired with the namespace whose location table owns it. -/
structure RuntimeLocation where
  namespaceId : NamespaceId
  loc : LocId
  deriving Repr, BEq, DecidableEq, Inhabited

/-- Resolve a runtime source point back through the validated namespace's
location table. -/
def RuntimeLocation.resolve? (unit : ValidatedUnit) (location : RuntimeLocation) :
    Option Location := do
  let targetNamespace ← unit.namespaces[location.namespaceId.index]?
  targetNamespace.tables.locations[location.loc.index]?

/-- Primary authored byte range for a runtime source point, when supplied by
the frontend. -/
def RuntimeLocation.primaryRange? (unit : ValidatedUnit) (location : RuntimeLocation) :
    Option SourceRange := do
  let resolved ← location.resolve? unit
  resolved.primary

/-- A semantic result with its primary source point and innermost-first call
sites.  The payload stays location-free so the same values and control modes
can appear in declarative judgments. -/
structure Located (α : Type) where
  value : α
  primary : RuntimeLocation
  callers : Array RuntimeLocation := #[]
  deriving Repr, BEq

namespace Located

/-- Add the call expression which propagated a callee result or failure. -/
def pushCaller (located : Located α) (caller : RuntimeLocation) : Located α :=
  { located with callers := located.callers.push caller }

end Located

/-- Structured, function-local control result.  Loop nesting is represented
relative to the expression currently handling the control transfer. -/
inductive Control where
  | value (value : RuntimeValue)
  | break_ (nest : Nat) (value : Option RuntimeValue)
  | continue_ (nest : Nat)
  | return_ (values : Array RuntimeValue)
  | throw_ (kind : ThrowKind) (arguments : Array RuntimeValue)
  deriving Repr, BEq, Inhabited

abbrev LocatedControl := Located Control

/-- Observable completion of a function.  A language-level throw is an
ordinary semantic result, not an interpreter error. -/
inductive Outcome where
  | returned (values : Array RuntimeValue)
  | threw (kind : ThrowKind) (arguments : Array RuntimeValue)
  deriving Repr, BEq, Inhabited

abbrev LocatedOutcome := Located Outcome

/-- Stable reasons why execution cannot derive a language result. -/
inductive InterpreterError where
  | outOfFuel
  | unknownFunction (reference : QualifiedRef)
  | unknownConstant (reference : QualifiedRef)
  | functionHasNoBody (function : FunctionHandle)
  | argumentArity (expected actual : Nat)
  | resultArity (expected actual : Nat)
  | uninitializedLocal (localId : LocalId)
  | expectedBoolean (actual : RuntimeValue)
  | expectedTuple (actual : RuntimeValue)
  | expectedClosure (actual : RuntimeValue)
  | invalidConstructor (reference : QualifiedRef) (variant : Option String)
  | invalidDataOperation (operation : DataOperation)
  | invalidProfileOperation (value : ProfileValue)
  | patternMismatch
  | nonExhaustiveMatch
  | invalidPlace (placeId : PlaceId)
  | escapedLoopControl
  | unsupportedPreparedNode
  deriving Repr, BEq, Inhabited

abbrev LocatedInterpreterError := Located InterpreterError

/-- Stable diagnostic code for tooling which must not parse error text. -/
def InterpreterError.code : InterpreterError → String
  | .outOfFuel => "LIR-EXEC-FUEL"
  | .unknownFunction _ => "LIR-EXEC-FUNCTION"
  | .unknownConstant _ => "LIR-EXEC-CONSTANT"
  | .functionHasNoBody _ => "LIR-EXEC-NO-BODY"
  | .argumentArity .. => "LIR-EXEC-ARGUMENT-ARITY"
  | .resultArity .. => "LIR-EXEC-RESULT-ARITY"
  | .uninitializedLocal _ => "LIR-EXEC-UNINITIALIZED-LOCAL"
  | .expectedBoolean _ => "LIR-EXEC-EXPECTED-BOOL"
  | .expectedTuple _ => "LIR-EXEC-EXPECTED-TUPLE"
  | .expectedClosure _ => "LIR-EXEC-EXPECTED-CLOSURE"
  | .invalidConstructor .. => "LIR-EXEC-CONSTRUCTOR"
  | .invalidDataOperation _ => "LIR-EXEC-DATA-OPERATION"
  | .invalidProfileOperation _ => "LIR-EXEC-PROFILE-OPERATION"
  | .patternMismatch => "LIR-EXEC-PATTERN"
  | .nonExhaustiveMatch => "LIR-EXEC-NONEXHAUSTIVE-MATCH"
  | .invalidPlace _ => "LIR-EXEC-PLACE"
  | .escapedLoopControl => "LIR-EXEC-LOOP-CONTROL"
  | .unsupportedPreparedNode => "LIR-EXEC-INTERNAL-CAPABILITY"

end LeanerIR
