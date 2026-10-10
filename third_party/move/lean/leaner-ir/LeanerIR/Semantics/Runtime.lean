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
  deriving Repr, DecidableEq, Inhabited

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
dynamic loan instance minted at the borrow; the lexically recorded loan
deaths reunite hole and current (`settleLoans`). A shared reference is not a
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
  /-- A function value: its target, the mask of the parameters it
  captures, the type instantiation its construction fixed, and the
  captured values. -/
  | closure (function : FunctionHandle) (mask : Nat)
      (typeInstantiation : Array (TypeId × TypeId)) (captures : Array RuntimeValue)
  | borrow (loan : Nat) (current : RuntimeValue)
  | loanHole (loan : Nat)
  deriving Repr, Inhabited

/-! Structural equality of runtime values, as the `==` primitive decides
it.  A derived `BEq` on this nested inductive is opaque — `partial`
underneath — and so says nothing in a proof; this one is defined by
well-founded recursion over the value, so each constructor's equation is
a theorem. -/

/-- An array's size measure is one more than its list's. -/
theorem Array.sizeOf_eq_toList {α : Type} [SizeOf α] (array : Array α) :
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
  | .closure leftFunction leftMask leftInstantiation leftCaptures,
      .closure rightFunction rightMask rightInstantiation rightCaptures =>
      leftFunction == rightFunction && leftMask == rightMask &&
        leftInstantiation == rightInstantiation &&
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

/-- Structural equality below a size bound is equality; by induction on the
bound, so that the recursion needs no termination argument. -/
private theorem RuntimeValue.eq_of_beq_below : (bound : Nat) →
    (∀ left right : RuntimeValue, sizeOf left + sizeOf right < bound →
      RuntimeValue.beq left right = true → left = right) ∧
    (∀ lefts rights : List RuntimeValue, sizeOf lefts + sizeOf rights < bound →
      RuntimeValue.beqList lefts rights = true → lefts = rights)
  | 0 => ⟨fun _ _ small => absurd small (Nat.not_lt_zero _),
      fun _ _ small => absurd small (Nat.not_lt_zero _)⟩
  | bound + 1 => by
      obtain ⟨values, lists⟩ := RuntimeValue.eq_of_beq_below bound
      constructor
      · intro left right small h
        cases left <;> cases right
        all_goals (try (simp [RuntimeValue.beq] at h; done))
        all_goals (try (simp_all [RuntimeValue.beq]; done))
        case vector.vector left right | tuple.tuple left right =>
          simp only [RuntimeValue.beq] at h
          simp only [RuntimeValue.vector.sizeOf_spec, RuntimeValue.tuple.sizeOf_spec,
            Array.sizeOf_eq_toList] at small
          rw [Array.ext' (lists _ _ (by omega) h)]
        case nominal.nominal source variant left rightSource rightVariant right =>
          simp only [RuntimeValue.beq, Bool.and_eq_true, beq_iff_eq] at h
          obtain ⟨⟨rfl, rfl⟩, fields⟩ := h
          simp only [RuntimeValue.nominal.sizeOf_spec, Array.sizeOf_eq_toList] at small
          rw [Array.ext' (lists _ _ (by omega) fields)]
        case closure.closure function mask instantiation left rightFunction rightMask
            rightInstantiation right =>
          simp only [RuntimeValue.beq, Bool.and_eq_true, beq_iff_eq] at h
          obtain ⟨⟨⟨rfl, rfl⟩, rfl⟩, captures⟩ := h
          simp only [RuntimeValue.closure.sizeOf_spec, Array.sizeOf_eq_toList] at small
          rw [Array.ext' (lists _ _ (by omega) captures)]
        case borrow.borrow loan left rightLoan right =>
          simp only [RuntimeValue.beq, Bool.and_eq_true, beq_iff_eq] at h
          obtain ⟨rfl, current⟩ := h
          simp only [RuntimeValue.borrow.sizeOf_spec] at small
          rw [values _ _ (by omega) current]
      · intro lefts rights small h
        cases lefts <;> cases rights
        case nil.nil => rfl
        case cons.cons left lefts right rights =>
          simp only [RuntimeValue.beqList, Bool.and_eq_true] at h
          simp only [List.cons.sizeOf_spec] at small
          rw [values _ _ (by omega) h.1, lists _ _ (by omega) h.2]
        all_goals simp [RuntimeValue.beqList] at h

/-- Every value is structurally equal to itself. -/
private theorem RuntimeValue.beq_self_below : (bound : Nat) →
    (∀ value : RuntimeValue, sizeOf value < bound → RuntimeValue.beq value value = true) ∧
    (∀ values : List RuntimeValue, sizeOf values < bound →
      RuntimeValue.beqList values values = true)
  | 0 => ⟨fun _ small => absurd small (Nat.not_lt_zero _),
      fun _ small => absurd small (Nat.not_lt_zero _)⟩
  | bound + 1 => by
      obtain ⟨values, lists⟩ := RuntimeValue.beq_self_below bound
      constructor
      · intro value small
        cases value
        all_goals (try (simp [RuntimeValue.beq]; done))
        case vector elements | tuple elements =>
          simp only [RuntimeValue.vector.sizeOf_spec, RuntimeValue.tuple.sizeOf_spec,
            Array.sizeOf_eq_toList] at small
          simp only [RuntimeValue.beq]
          exact lists _ (by omega)
        case nominal source variant fields =>
          simp only [RuntimeValue.nominal.sizeOf_spec, Array.sizeOf_eq_toList] at small
          simp only [RuntimeValue.beq, beq_self_eq_true, Bool.true_and]
          exact lists _ (by omega)
        case closure function mask instantiation captures =>
          simp only [RuntimeValue.closure.sizeOf_spec, Array.sizeOf_eq_toList] at small
          simp only [RuntimeValue.beq, beq_self_eq_true, Bool.true_and]
          exact lists _ (by omega)
        case borrow loan current =>
          simp only [RuntimeValue.borrow.sizeOf_spec] at small
          simp only [RuntimeValue.beq, beq_self_eq_true, Bool.true_and]
          exact values _ (by omega)
      · intro list small
        cases list
        case nil => simp [RuntimeValue.beqList]
        case cons value rest =>
          simp only [List.cons.sizeOf_spec] at small
          simp only [RuntimeValue.beqList, Bool.and_eq_true]
          exact ⟨values _ (by omega), lists _ (by omega)⟩

instance : LawfulBEq RuntimeValue where
  eq_of_beq {left right} :=
    (RuntimeValue.eq_of_beq_below (sizeOf left + sizeOf right + 1)).1 left right (by omega)
  rfl {value} := (RuntimeValue.beq_self_below (sizeOf value + 1)).1 value (by omega)

/-- Equality of runtime values, decided by their structural equality. -/
instance : DecidableEq RuntimeValue := fun left right =>
  decidable_of_iff (left == right) beq_iff_eq

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

/-! ## Structural order

The order of `std::cmp::compare`: a primitive value by its natural order;
vectors, tuples, and fields lexicographically; an enum value by its
variant's declaration position, then its fields. Values of different kinds
order by kind, and spellings break the ties a position leaves, so the order
is total on every value. -/

/-- The position of a value's kind in the structural order. -/
def RuntimeValue.kindRank : RuntimeValue → Nat
  | .unit => 0
  | .bool _ => 1
  | .character _ => 2
  | .integer _ => 3
  | .address _ => 4
  | .signer _ => 5
  | .string _ => 6
  | .bytes _ => 7
  | .vector _ => 8
  | .tuple _ => 9
  | .nominal .. => 10
  | .closure .. => 11
  | .borrow .. => 12
  | .loanHole _ => 13

/-- What the order of runtime values reads from the unit: each enum
variant's declaration position, and each function's position in the order
of qualified names. -/
structure ValueRanks where
  variant : StructHandle → String → Nat
  function : FunctionHandle → Nat

/-- Variants order by declaration position, then by name; a plain struct
value precedes a variant. -/
def compareVariant (rank : ValueRanks) (leftSource rightSource : StructHandle) :
    Option String → Option String → Ordering
  | none, none => .eq
  | none, some _ => .lt
  | some _, none => .gt
  | some left, some right =>
      (compare (rank.variant leftSource left) (rank.variant rightSource right)).then
        (compare left right)

/-- A type instantiation as the numbers of its type identifiers, pairwise. -/
def instantiationKey (instantiation : Array (TypeId × TypeId)) : List Nat :=
  instantiation.toList.flatMap fun (source, target) => [source.index, target.index]

/-- Type instantiations in the order of their type identifiers. -/
def compareInstantiation (left right : Array (TypeId × TypeId)) : Ordering :=
  compare (instantiationKey left) (instantiationKey right)

mutual

/-- The structural order of two runtime values, given each declaration's
variant positions and each function's position by name. -/
def RuntimeValue.order (rank : ValueRanks) (left right : RuntimeValue) :
    Ordering :=
  (compare left.kindRank right.kindRank).then (RuntimeValue.orderPayload rank left right)
termination_by sizeOf left + sizeOf right + 1

/-- The order of two values of the same kind. -/
def RuntimeValue.orderPayload (rank : ValueRanks) :
    RuntimeValue → RuntimeValue → Ordering
  | .bool left, .bool right => compare left right
  | .character left, .character right => compare left right
  | .integer left, .integer right => compare left right
  | .address left, .address right => compareAddress left right
  | .signer left, .signer right => compareAddress left right
  | .string left, .string right => compare left right
  | .bytes left, .bytes right => List.compareLex compare left.toList right.toList
  | .vector left, .vector right => RuntimeValue.orderList rank left.toList right.toList
  | .tuple left, .tuple right => RuntimeValue.orderList rank left.toList right.toList
  | .nominal leftSource leftVariant leftFields,
      .nominal rightSource rightVariant rightFields =>
      ((compare leftSource.namespaceId.index rightSource.namespaceId.index).then
        (compare leftSource.structId rightSource.structId)).then
      ((compareVariant rank leftSource rightSource leftVariant rightVariant).then
        (RuntimeValue.orderList rank leftFields.toList rightFields.toList))
  -- As Move compares function values: by the function's name, its type
  -- arguments, the mask, then the captured values. The handle breaks the
  -- ties a rank leaves.
  | .closure leftFunction leftMask leftInstantiation leftCaptures,
      .closure rightFunction rightMask rightInstantiation rightCaptures =>
      ((compare (rank.function leftFunction) (rank.function rightFunction)).then
        ((compare leftFunction.namespaceId.index rightFunction.namespaceId.index).then
          (compare leftFunction.functionId.index rightFunction.functionId.index))).then
      ((compareInstantiation leftInstantiation rightInstantiation).then
        ((compare leftMask rightMask).then
          (RuntimeValue.orderList rank leftCaptures.toList rightCaptures.toList)))
  | .borrow leftLoan leftCurrent, .borrow rightLoan rightCurrent =>
      (compare leftLoan rightLoan).then (RuntimeValue.order rank leftCurrent rightCurrent)
  | .loanHole left, .loanHole right => compare left right
  | _, _ => .eq
termination_by left right => sizeOf left + sizeOf right
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); omega)

/-- The lexicographic order of two value lists. -/
def RuntimeValue.orderList (rank : ValueRanks) :
    List RuntimeValue → List RuntimeValue → Ordering
  | [], [] => .eq
  | [], _ :: _ => .lt
  | _ :: _, [] => .gt
  | left :: lefts, right :: rights =>
      (RuntimeValue.order rank left right).then (RuntimeValue.orderList rank lefts rights)
termination_by lefts rights => sizeOf lefts + sizeOf rights
decreasing_by all_goals (simp_wf; omega)

end

/-- An ordering as the integer `compare` returns. -/
def orderValue : Ordering → Int
  | .lt => -1
  | .eq => 0
  | .gt => 1

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

/-- A code of a storage key, injective, which orders keys. -/
def StorageKey.rank : StorageKey → List Nat
  | .unit => [0]
  | .bool value => [1, if value then 1 else 0]
  | .character value => [2, value]
  | .integer value => [3, if 0 ≤ value then 2 * value.toNat else 2 * (-value).toNat - 1]
  | .address value => 4 :: value.toList.map Char.toNat
  | .signer value => 5 :: value.toList.map Char.toNat
  | .string value => 6 :: value.toList.map Char.toNat
  | .bytes value => 7 :: value.toList.map UInt8.toNat

/-- A code of a global key, injective, which orders global memory. -/
def GlobalKey.rank (key : GlobalKey) : List Nat :=
  key.namespaceId.index :: key.typeId.index :: key.key.rank

private theorem chars_rank_inj {left right : String}
    (equal : left.toList.map Char.toNat = right.toList.map Char.toNat) : left = right :=
  String.ext ((List.map_inj_right fun _ _ same => Char.toNat_inj.mp same).mp equal)

private theorem bytes_rank_inj {left right : Array UInt8}
    (equal : left.toList.map UInt8.toNat = right.toList.map UInt8.toNat) : left = right :=
  Array.toList_inj.mp ((List.map_inj_right fun _ _ same => UInt8.toNat_inj.mp same).mp equal)

theorem StorageKey.rank_inj {left right : StorageKey} (equal : left.rank = right.rank) :
    left = right := by
  cases left <;> cases right <;>
    simp only [StorageKey.rank, List.cons.injEq, reduceCtorEq, false_and, and_false,
      and_true, true_and, Nat.reduceEqDiff] at equal
  case unit.unit => rfl
  case bool.bool left right => cases left <;> cases right <;> simp_all
  case character.character => rw [equal]
  case integer.integer left right =>
    congr 1
    split at equal <;> split at equal <;> omega
  case address.address => rw [chars_rank_inj equal]
  case signer.signer => rw [chars_rank_inj equal]
  case string.string => rw [chars_rank_inj equal]
  case bytes.bytes => rw [bytes_rank_inj equal]

theorem GlobalKey.rank_inj {left right : GlobalKey} (equal : left.rank = right.rank) :
    left = right := by
  obtain ⟨⟨leftNs⟩, ⟨leftType⟩, leftKey⟩ := left
  obtain ⟨⟨rightNs⟩, ⟨rightType⟩, rightKey⟩ := right
  simp only [GlobalKey.rank, List.cons.injEq] at equal
  obtain ⟨rfl, rfl, keys⟩ := equal
  rw [StorageKey.rank_inj keys]

/-- Codes are ordered totally. -/
theorem rank_trichotomy (left right : List Nat) : left < right ∨ left = right ∨ right < left := by
  by_cases less : left < right
  · exact .inl less
  by_cases greater : right < left
  · exact .inr (.inr greater)
  exact .inr (.inl (List.le_antisymm greater less))

/-- Global memory: at most one resource per key, in the order of the keys'
codes, so that two maps with the same lookups are equal (`ext`). -/
structure GlobalMap where
  entries : Array GlobalSlot := #[]
  deriving Repr, BEq, Inhabited

namespace GlobalMap

/-- The slot placed before the first slot of a higher key. -/
def insertSlot (slot : GlobalSlot) : List GlobalSlot → List GlobalSlot
  | [] => [slot]
  | head :: rest =>
      if slot.key.rank < head.key.rank then slot :: head :: rest
      else head :: insertSlot slot rest

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
  ⟨(insertSlot ⟨key, value⟩ (globals.erase key).entries.toList).toArray⟩

/-- Entries in strictly increasing order of their keys' codes. -/
def Sorted (globals : GlobalMap) : Prop :=
  (globals.entries.toList.map (·.key.rank)).Pairwise (· < ·)

theorem mem_insertSlot {slot : GlobalSlot} {entries : List GlobalSlot} {member : GlobalSlot} :
    member ∈ insertSlot slot entries ↔ member = slot ∨ member ∈ entries := by
  induction entries with
  | nil => simp [insertSlot]
  | cons head rest ih =>
      unfold insertSlot
      split <;> simp only [List.mem_cons, ih, or_left_comm]

theorem find?_insertSlot_other {slot : GlobalSlot} {p : GlobalSlot → Bool}
    (missed : p slot = false) (entries : List GlobalSlot) :
    (insertSlot slot entries).find? p = entries.find? p := by
  induction entries with
  | nil => simp [insertSlot, missed]
  | cons head rest ih =>
      unfold insertSlot
      split
      · simp [missed]
      · by_cases hit : p head = true
        · simp [hit]
        · simp [hit, ih]

theorem find?_insertSlot_self {slot : GlobalSlot} {p : GlobalSlot → Bool} (hit : p slot = true)
    {entries : List GlobalSlot} (none : ∀ member ∈ entries, p member = false) :
    (insertSlot slot entries).find? p = some slot := by
  induction entries with
  | nil => simp [insertSlot, hit]
  | cons head rest ih =>
      unfold insertSlot
      split
      · simp [hit]
      · simp [none head List.mem_cons_self,
          ih fun member member_in => none member (List.mem_cons_of_mem _ member_in)]

theorem insertSlot_sorted {slot : GlobalSlot} :
    {entries : List GlobalSlot} → (entries.map (·.key.rank)).Pairwise (· < ·) →
    (∀ member ∈ entries, member.key ≠ slot.key) →
    ((insertSlot slot entries).map (·.key.rank)).Pairwise (· < ·)
  | [], _, _ => by simp [insertSlot]
  | head :: rest, sorted, distinct => by
      simp only [List.map_cons, List.pairwise_cons, List.mem_map] at sorted
      unfold insertSlot
      split
      · rename_i less
        simp only [List.map_cons, List.pairwise_cons, List.mem_cons, List.mem_map]
        refine ⟨?_, sorted.1, sorted.2⟩
        rintro rank (rfl | ⟨member, member_in, rfl⟩)
        · exact less
        · exact List.lt_trans less (sorted.1 _ ⟨member, member_in, rfl⟩)
      · rename_i notLess
        have greater : head.key.rank < slot.key.rank := by
          rcases rank_trichotomy slot.key.rank head.key.rank with less | same | greater
          · exact absurd less notLess
          · exact absurd (GlobalKey.rank_inj same).symm (distinct head List.mem_cons_self)
          · exact greater
        simp only [List.map_cons, List.pairwise_cons]
        refine ⟨?_, insertSlot_sorted sorted.2
          (fun member member_in => distinct member (List.mem_cons_of_mem _ member_in))⟩
        intro rank rank_in
        obtain ⟨member, member_in, rfl⟩ := List.mem_map.mp rank_in
        rcases mem_insertSlot.mp member_in with rfl | member_in
        · exact greater
        · exact sorted.1 _ ⟨member, member_in, rfl⟩

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
  simp only [lookup, insert, erase, ← Array.find?_toList]
  rw [find?_insertSlot_self (by simp)]
  · rfl
  · intro member member_in
    simp only [Array.toList_filter, List.mem_filter, decide_eq_true_eq] at member_in
    simpa using member_in.2

@[simp] theorem lookup_insert_other (globals : GlobalMap) (written query : GlobalKey)
    (value : RuntimeValue) (distinct : query ≠ written) :
    (globals.insert written value).lookup query = globals.lookup query := by
  simp only [lookup, insert, erase, ← Array.find?_toList]
  rw [find?_insertSlot_other (by simpa using fun equal : written = query => distinct equal.symm),
    Array.toList_filter, listFind?_filter_ne distinct]

/-- What an inserted map holds: the inserted slot, and every other key's. -/
theorem mem_insert {globals : GlobalMap} {key : GlobalKey} {value : RuntimeValue}
    {slot : GlobalSlot} :
    slot ∈ (globals.insert key value).entries ↔
      (slot ∈ globals.entries ∧ slot.key ≠ key) ∨ slot = ⟨key, value⟩ := by
  simp only [insert, erase, List.mem_toArray, mem_insertSlot, Array.mem_toList_iff,
    Array.mem_filter, decide_eq_true_eq]
  exact or_comm

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

theorem sorted_empty : Sorted {} := by simp [Sorted]

theorem Sorted.erase {globals : GlobalMap} (sorted : globals.Sorted) (key : GlobalKey) :
    (globals.erase key).Sorted := by
  unfold Sorted GlobalMap.erase at *
  simp only [Array.toList_filter]
  exact List.Pairwise.sublist (List.Sublist.map _ List.filter_sublist) sorted

theorem Sorted.insert {globals : GlobalMap} (sorted : globals.Sorted) (key : GlobalKey)
    (value : RuntimeValue) : (globals.insert key value).Sorted := by
  unfold Sorted GlobalMap.insert
  apply insertSlot_sorted (Sorted.erase sorted key)
  intro member member_in
  simp only [GlobalMap.erase, Array.toList_filter, List.mem_filter, decide_eq_true_eq] at member_in
  exact member_in.2

private theorem find?_sorted_absent {key : GlobalKey} :
    {entries : List GlobalSlot} → (∀ member ∈ entries, key.rank < member.key.rank) →
    entries.find? (·.key = key) = none
  | [], _ => rfl
  | head :: rest, above => by
      have ne : head.key ≠ key := fun same =>
        List.lt_irrefl _ (same ▸ above head List.mem_cons_self)
      simp only [List.find?_cons, ne, decide_false]
      exact find?_sorted_absent fun member member_in => above member (List.mem_cons_of_mem _ member_in)

private theorem list_ext :
    {left right : List GlobalSlot} → (left.map (·.key.rank)).Pairwise (· < ·) →
    (right.map (·.key.rank)).Pairwise (· < ·) →
    (∀ key, (left.find? (·.key = key)).map (·.value) = (right.find? (·.key = key)).map (·.value)) →
    left = right
  | [], [], _, _, _ => rfl
  | [], head :: _, _, _, same => by
      have := same head.key
      simp at this
  | head :: _, [], _, _, same => by
      have := same head.key
      simp at this
  | head :: rest, head' :: rest', sorted, sorted', same => by
      simp only [List.map_cons, List.pairwise_cons, List.mem_map] at sorted sorted'
      have above : ∀ member ∈ rest, head.key.rank < member.key.rank :=
        fun member member_in => sorted.1 _ ⟨member, member_in, rfl⟩
      have above' : ∀ member ∈ rest', head'.key.rank < member.key.rank :=
        fun member member_in => sorted'.1 _ ⟨member, member_in, rfl⟩
      have keys : head.key = head'.key := by
        rcases rank_trichotomy head.key.rank head'.key.rank with less | equal | greater
        · have := same head.key
          simp only [List.find?_cons, decide_true] at this
          have missing : (head'.key = head.key) = False :=
            eq_false fun eq => List.lt_irrefl _ (eq ▸ less)
          simp only [missing, decide_false,
            find?_sorted_absent fun member member_in =>
              List.lt_trans less (above' member member_in)] at this
          simp at this
        · exact GlobalKey.rank_inj equal
        · have := same head'.key
          simp only [List.find?_cons, decide_true] at this
          have missing : (head.key = head'.key) = False :=
            eq_false fun eq => List.lt_irrefl _ (eq ▸ greater)
          simp only [missing, decide_false,
            find?_sorted_absent fun member member_in =>
              List.lt_trans greater (above member member_in)] at this
          simp at this
      have values : head.value = head'.value := by
        have := same head.key
        simp only [List.find?_cons, decide_true, keys] at this
        simpa using this
      have heads : head = head' := by
        cases head; cases head'; simp only at keys values; rw [keys, values]
      subst heads
      rw [list_ext sorted.2 sorted'.2 fun key => ?_]
      by_cases at_head : head.key = key
      · subst at_head
        rw [find?_sorted_absent above, find?_sorted_absent above']
      · have := same key
        simpa [List.find?_cons, at_head] using this

private theorem find?_sorted_mem {slot : GlobalSlot} :
    {entries : List GlobalSlot} → (entries.map (·.key.rank)).Pairwise (· < ·) →
    slot ∈ entries → entries.find? (·.key = slot.key) = some slot
  | [], _, member => nomatch member
  | head :: rest, sorted, member => by
      simp only [List.map_cons, List.pairwise_cons, List.mem_map] at sorted
      rcases List.mem_cons.mp member with rfl | member
      · simp
      · have ne : head.key ≠ slot.key := fun same =>
          List.lt_irrefl _ (same ▸ sorted.1 _ ⟨slot, member, rfl⟩)
        simp only [List.find?_cons, ne, decide_false]
        exact find?_sorted_mem sorted.2 member

/-- In a sorted map, a slot is what its key looks up. -/
theorem Sorted.lookup_of_mem {globals : GlobalMap} (sorted : globals.Sorted) {slot : GlobalSlot}
    (member : slot ∈ globals.entries) : globals.lookup slot.key = some slot.value := by
  simp only [lookup, ← Array.find?_toList,
    find?_sorted_mem sorted (Array.mem_toList_iff.mpr member), Option.map_some]

/-- Sorted maps with the same lookups are equal. -/
theorem ext {left right : GlobalMap} (sorted : left.Sorted) (sorted' : right.Sorted)
    (same : ∀ key, left.lookup key = right.lookup key) : left = right := by
  obtain ⟨left⟩ := left
  obtain ⟨right⟩ := right
  congr 1
  apply Array.toList_inj.mp
  exact list_ext sorted sorted' fun key => by
    simpa only [lookup, ← Array.find?_toList] using same key

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
the site's recorded death finds the loan it ends; a site borrowed again
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

/-- The throw a pattern mismatch makes (`patternMismatch?`), its arguments
as runtime values. -/
def patternMismatchThrow? (profile : Option Profile) : Option (ThrowKind × Array RuntimeValue) :=
  (patternMismatch? profile).map fun (kind, codes) => (kind, codes.map .integer)

/-- Observable completion of a function.  A language-level throw is an
ordinary semantic result, not an interpreter error. -/
inductive Outcome where
  | returned (values : Array RuntimeValue)
  | threw (kind : ThrowKind) (arguments : Array RuntimeValue)
  deriving Repr, BEq, Inhabited

abbrev LocatedOutcome := Located Outcome

mutual
/-- Whether a value holds no loan hole outside a borrow. A borrow's current
belongs to its lender. -/
def RuntimeValue.holeFree? : RuntimeValue → Bool
  | .vector elements | .tuple elements | .nominal _ _ elements | .closure _ _ _ elements =>
      RuntimeValue.holeFreeList? elements.toList
  | .loanHole _ => false
  | .borrow .. | .unit | .bool _ | .character _ | .integer _ | .address _ | .signer _
  | .string _ | .bytes _ => true
termination_by value => sizeOf value
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); omega)

/-- Whether no value of a list holds a loan hole outside a borrow. -/
def RuntimeValue.holeFreeList? : List RuntimeValue → Bool
  | [] => true
  | value :: values => value.holeFree? && RuntimeValue.holeFreeList? values
termination_by values => sizeOf values
decreasing_by all_goals (simp_wf; omega)
end

/-- Whether an outcome is one a function may return: results that hold no
loan hole outside a borrow. Reading a mutably borrowed place yields its
hole, which only the borrow checker forbids, so a run returning one is
stuck. -/
def Outcome.holeFree : Outcome → Bool
  | .returned values => values.all (·.holeFree?)
  | .threw _ _ => true

/-- Stable reasons why execution cannot derive a language result. -/
inductive InterpreterError where
  | outOfFuel
  | unknownFunction (reference : QualifiedRef)
  | unknownConstant (reference : QualifiedRef)
  | functionHasNoBody (function : FunctionHandle)
  | argumentArity (expected actual : Nat)
  | resultArity (expected actual : Nat)
  /-- A function's results hold a loan hole outside a borrow. -/
  | returnedLoanHole
  | uninitializedLocal (localId : LocalId)
  | expectedBoolean (actual : RuntimeValue)
  | expectedTuple (actual : RuntimeValue)
  | expectedClosure (actual : RuntimeValue)
  /-- The supplied arguments do not fill the parameters a closure's mask
  leaves open. -/
  | closureArguments (mask captured supplied : Nat)
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
  | .returnedLoanHole => "LIR-EXEC-RETURNED-HOLE"
  | .uninitializedLocal _ => "LIR-EXEC-UNINITIALIZED-LOCAL"
  | .expectedBoolean _ => "LIR-EXEC-EXPECTED-BOOL"
  | .expectedTuple _ => "LIR-EXEC-EXPECTED-TUPLE"
  | .expectedClosure _ => "LIR-EXEC-EXPECTED-CLOSURE"
  | .closureArguments .. => "LIR-EXEC-CLOSURE-ARGUMENTS"
  | .invalidConstructor .. => "LIR-EXEC-CONSTRUCTOR"
  | .invalidDataOperation _ => "LIR-EXEC-DATA-OPERATION"
  | .invalidProfileOperation _ => "LIR-EXEC-PROFILE-OPERATION"
  | .patternMismatch => "LIR-EXEC-PATTERN"
  | .nonExhaustiveMatch => "LIR-EXEC-NONEXHAUSTIVE-MATCH"
  | .invalidPlace _ => "LIR-EXEC-PLACE"
  | .escapedLoopControl => "LIR-EXEC-LOOP-CONTROL"
  | .unsupportedPreparedNode => "LIR-EXEC-INTERNAL-CAPABILITY"

end LeanerIR
