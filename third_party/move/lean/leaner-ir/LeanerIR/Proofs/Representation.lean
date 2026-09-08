-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.Typing

/-!
# Typed spec-level view of global storage

Contracts speak about global memory through a typed view: per resource
family, a `StorageKey`-indexed map of typed values whose *erasure* is what
the runtime map holds.  A value read out of storage is then literally the
erasure of a typed value, so its constructor shape and its range facts hold
by reduction rather than by inversion of a typing judgment.

The frontends generate one typed twin per struct declaration (with an
`erase`/`decode?` pair and their roundtrip) and one `FamilyRepresentation`
conjunct per storable family; this file owns the generic vocabulary those
generated declarations share.  Cross-family independence is a theorem of the
keyed map — distinct families never collide because their keys differ — not
an assumption.
-/

namespace LeanerIR

/-! ## Certified integers

The specification-level representation of an integer field: the value
together with the fact that it inhabits its declared type.  Widths without a
neutral range decision (`pointer` before target selection, zero bits) make
the certificate unsatisfiable, so a `SpecInt` at such a type has no
inhabitants rather than junk bounds. -/

instance (width : IntWidth) (signed : Bool) (value : Int) :
    Decidable (IntegerValueFits width signed value) :=
  inferInstanceAs (Decidable (_ = _))

/-- A certified integer: the typed-twin field representation of a fixed-width
integer, carrying its range certificate. -/
structure SpecInt (width : IntWidth) (signed : Bool) where
  val : Int
  fits : IntegerValueFits width signed val

/-- Certified integers are their values; the certificate is proof-irrelevant. -/
theorem SpecInt.ext {width : IntWidth} {signed : Bool} :
    ∀ {a b : SpecInt width signed}, a.val = b.val → a = b
  | ⟨_, _⟩, ⟨_, _⟩, rfl => rfl

/-- The bounds an unsigned certificate carries, in the form `omega` reads.
The zero-width case is vacuous: no certificate exists there. -/
theorem IntegerValueFits.unsigned_bounds {width : Nat} {value : Int}
    (fits : IntegerValueFits (.bits width) false value) :
    0 ≤ value ∧ value ≤ 2 ^ width - 1 := by
  by_cases zero : width = 0
  · simp [IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?, zero] at fits
  · simpa [IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?, zero] using fits

/-- The bounds a signed certificate carries, in the form `omega` reads. -/
theorem IntegerValueFits.signed_bounds {width : Nat} {value : Int}
    (fits : IntegerValueFits (.bits width) true value) :
    -(2 ^ (width - 1)) ≤ value ∧ value ≤ 2 ^ (width - 1) - 1 := by
  by_cases zero : width = 0
  · simp [IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?, zero] at fits
  · simpa [IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?, zero] using fits

/-- The bounds of an unsigned certified integer, in the form `omega` reads. -/
theorem SpecInt.unsigned_bounds {width : Nat} (n : SpecInt (.bits width) false) :
    0 ≤ n.val ∧ n.val ≤ 2 ^ width - 1 :=
  n.fits.unsigned_bounds

/-- The bounds of a signed certified integer, in the form `omega` reads. -/
theorem SpecInt.signed_bounds {width : Nat} (n : SpecInt (.bits width) true) :
    -(2 ^ (width - 1)) ≤ n.val ∧ n.val ≤ 2 ^ (width - 1) - 1 :=
  n.fits.signed_bounds

/-! ## Field codecs

A generated twin's `decode?` composes these per-primitive decoders, one per
scalar field kind; the simp-tagged roundtrips make every generated
`decode?_erase` one uniform `simp`.  Array-literal patterns get no `match`
equations, so nothing here (or generated from here) matches a field row as
an array literal — rows go through `toList`. -/

/-- Decode a certified integer field. -/
def decodeInt? (width : IntWidth) (signed : Bool) :
    RuntimeValue → Option (SpecInt width signed)
  | .integer value =>
      if fits : IntegerValueFits width signed value then some ⟨value, fits⟩
      else none
  | _ => none

@[simp] theorem decodeInt?_val {width : IntWidth} {signed : Bool}
    (n : SpecInt width signed) :
    decodeInt? width signed (.integer n.val) = some n := by
  simp [decodeInt?, n.fits]

/-- Decode a boolean field. -/
def decodeBool? : RuntimeValue → Option Bool
  | .bool value => some value
  | _ => none

@[simp] theorem decodeBool?_bool (value : Bool) :
    decodeBool? (.bool value) = some value := rfl

/-- Decode a string field. -/
def decodeString? : RuntimeValue → Option String
  | .string value => some value
  | _ => none

@[simp] theorem decodeString?_string (value : String) :
    decodeString? (.string value) = some value := rfl

/-- Decode an address field. -/
def decodeAddress? : RuntimeValue → Option String
  | .address value => some value
  | _ => none

@[simp] theorem decodeAddress?_address (value : String) :
    decodeAddress? (.address value) = some value := rfl

/-- Decode a signer field. -/
def decodeSigner? : RuntimeValue → Option String
  | .signer value => some value
  | _ => none

@[simp] theorem decodeSigner?_signer (value : String) :
    decodeSigner? (.signer value) = some value := rfl

/-- Decode a byte-array field. -/
def decodeBytes? : RuntimeValue → Option (Array UInt8)
  | .bytes value => some value
  | _ => none

@[simp] theorem decodeBytes?_bytes (value : Array UInt8) :
    decodeBytes? (.bytes value) = some value := rfl

/-- Decode a unit field. -/
def decodeUnit? : RuntimeValue → Option Unit
  | .unit => some ()
  | _ => none

@[simp] theorem decodeUnit?_unit : decodeUnit? .unit = some () := rfl

/-! ## Family representation

One resource family's typed contents, represented in the runtime map through
the family's erasure.  This is the single head every generated `Rep`
conjunct applies and the storage tactic keys on. -/

/-- The runtime map holds exactly the erasures of one family's typed
contents: at every key of the family, the runtime lookup is the image of the
typed lookup. -/
def FamilyRepresentation {T : Type} (erase : T → RuntimeValue)
    (namespaceId : NamespaceId) (typeId : TypeId)
    (contents : StorageKey → Option T) (globals : GlobalMap) : Prop :=
  ∀ key : StorageKey,
    globals.lookup ⟨namespaceId, typeId, key⟩ = (contents key).map erase

/-- Decoding through the erasure recovers the typed contents; with it a
clause's typed read of a key and the program's runtime read are one term. -/
theorem map_erase_bind_decode {T : Type} {erase : T → RuntimeValue}
    {decode? : RuntimeValue → Option T}
    (roundtrip : ∀ value : T, decode? (erase value) = some value)
    (contents : Option T) : (contents.map erase).bind decode? = contents := by
  cases contents <;> simp [roundtrip]

/-- Pointwise update of one family's typed contents. -/
def updateContents {T : Type} (contents : StorageKey → Option T)
    (key : StorageKey) (value : Option T) : StorageKey → Option T :=
  fun query => if query = key then value else contents query

namespace FamilyRepresentation

variable {T : Type} {erase : T → RuntimeValue} {namespaceId : NamespaceId}
  {typeId : TypeId} {contents : StorageKey → Option T} {globals : GlobalMap}

/-- The empty state represents the empty contents: the typed-state
precondition every contract carries is satisfiable, so verification under it
is not vacuous. -/
theorem empty (erase : T → RuntimeValue) (namespaceId : NamespaceId)
    (typeId : TypeId) :
    FamilyRepresentation erase namespaceId typeId (fun _ => none) {} := by
  intro key
  simp

/-- Publishing a typed value updates the represented contents pointwise. -/
theorem insert_self
    (represented : FamilyRepresentation erase namespaceId typeId contents globals)
    (key : StorageKey) (value : T) :
    FamilyRepresentation erase namespaceId typeId
      (updateContents contents key (some value))
      (globals.insert ⟨namespaceId, typeId, key⟩ (erase value)) := by
  intro query
  by_cases same : query = key
  · subst same
    simp [updateContents]
  · have distinct : (⟨namespaceId, typeId, query⟩ : GlobalKey)
        ≠ ⟨namespaceId, typeId, key⟩ := by
      simp [same]
    rw [GlobalMap.lookup_insert_other _ _ _ _ distinct, represented query]
    simp [updateContents, same]

/-- Taking a typed value updates the represented contents pointwise. -/
theorem erase_self
    (represented : FamilyRepresentation erase namespaceId typeId contents globals)
    (key : StorageKey) :
    FamilyRepresentation erase namespaceId typeId
      (updateContents contents key none)
      (globals.erase ⟨namespaceId, typeId, key⟩) := by
  intro query
  by_cases same : query = key
  · subst same
    simp [updateContents]
  · have distinct : (⟨namespaceId, typeId, query⟩ : GlobalKey)
        ≠ ⟨namespaceId, typeId, key⟩ := by
      simp [same]
    rw [GlobalMap.lookup_erase_other _ _ _ distinct, represented query]
    simp [updateContents, same]

/-- Distinct families are disjoint locations: publishing under another
family leaves this family's representation untouched.  The proved analogue
of the frozen stack's `IndependentResourceStores` assumption. -/
theorem insert_other
    (represented : FamilyRepresentation erase namespaceId typeId contents globals)
    (written : GlobalKey) (value : RuntimeValue)
    (distinct : written.namespaceId ≠ namespaceId ∨ written.typeId ≠ typeId) :
    FamilyRepresentation erase namespaceId typeId contents
      (globals.insert written value) := by
  intro query
  have separate : (⟨namespaceId, typeId, query⟩ : GlobalKey) ≠ written := by
    intro equal
    rcases distinct with different | different <;>
      simp [← equal] at different
  rw [GlobalMap.lookup_insert_other _ _ _ _ separate, represented query]

/-- Distinct families are disjoint locations: taking under another family
leaves this family's representation untouched. -/
theorem erase_other
    (represented : FamilyRepresentation erase namespaceId typeId contents globals)
    (written : GlobalKey)
    (distinct : written.namespaceId ≠ namespaceId ∨ written.typeId ≠ typeId) :
    FamilyRepresentation erase namespaceId typeId contents
      (globals.erase written) := by
  intro query
  have separate : (⟨namespaceId, typeId, query⟩ : GlobalKey) ≠ written := by
    intro equal
    rcases distinct with different | different <;>
      simp [← equal] at different
  rw [GlobalMap.lookup_erase_other _ _ _ separate, represented query]

/-- Writing a resource back over the hole its borrow left updates the
represented contents pointwise, exactly as a direct publish would: the
hole is not an erasure of anything typed, but it is overwritten at the
same key before the map is read again. -/
theorem insert_over_hole
    (represented : FamilyRepresentation erase namespaceId typeId contents globals)
    (key : StorageKey) (hole : RuntimeValue) (value : T) :
    FamilyRepresentation erase namespaceId typeId
      (updateContents contents key (some value))
      ((globals.insert ⟨namespaceId, typeId, key⟩ hole).insert
        ⟨namespaceId, typeId, key⟩ (erase value)) := by
  intro query
  by_cases same : query = key
  · subst same
    simp [updateContents]
  · have distinct : (⟨namespaceId, typeId, query⟩ : GlobalKey)
        ≠ ⟨namespaceId, typeId, key⟩ := by
      simp [same]
    rw [GlobalMap.lookup_insert_other _ _ _ _ distinct,
      GlobalMap.lookup_insert_other _ _ _ _ distinct, represented query]
    simp [updateContents, same]

/-- Writing a resource back over two intermediate values at its key — the
outer borrow's hole, then the export carrying a returned loan's hole —
updates the represented contents pointwise, as `insert_over_hole` does
over one. -/
theorem insert_over_two
    (represented : FamilyRepresentation erase namespaceId typeId contents globals)
    (key : StorageKey) (first second : RuntimeValue) (value : T) :
    FamilyRepresentation erase namespaceId typeId
      (updateContents contents key (some value))
      (((globals.insert ⟨namespaceId, typeId, key⟩ first).insert
        ⟨namespaceId, typeId, key⟩ second).insert
        ⟨namespaceId, typeId, key⟩ (erase value)) := by
  intro query
  by_cases same : query = key
  · subst same
    simp [updateContents]
  · have distinct : (⟨namespaceId, typeId, query⟩ : GlobalKey)
        ≠ ⟨namespaceId, typeId, key⟩ := by
      simp [same]
    rw [GlobalMap.lookup_insert_other _ _ _ _ distinct,
      GlobalMap.lookup_insert_other _ _ _ _ distinct,
      GlobalMap.lookup_insert_other _ _ _ _ distinct, represented query]
    simp [updateContents, same]

/-- Writing a typed value at a key into a map that agrees with a
represented map everywhere else updates the represented contents
pointwise: what a caller knows of a callee's final map — its frame
clause — is enough to represent its own write over it. -/
theorem insert_agreeing
    (represented : FamilyRepresentation erase namespaceId typeId contents globals)
    {other : GlobalMap} (key : StorageKey)
    (agreeing : ∀ query : GlobalKey, query ≠ ⟨namespaceId, typeId, key⟩ →
      other.lookup query = globals.lookup query)
    (value : T) :
    FamilyRepresentation erase namespaceId typeId
      (updateContents contents key (some value))
      (other.insert ⟨namespaceId, typeId, key⟩ (erase value)) := by
  intro query
  by_cases same : query = key
  · subst same
    simp [updateContents]
  · have distinct : (⟨namespaceId, typeId, query⟩ : GlobalKey)
        ≠ ⟨namespaceId, typeId, key⟩ := by
      simp [same]
    rw [GlobalMap.lookup_insert_other _ _ _ _ distinct, agreeing _ distinct,
      represented query]
    simp [updateContents, same]

/-- A typed read through the representation is the typed contents: this
is how a clause's `get` of a key leaves the map behind. -/
theorem read_eq
    (represented : FamilyRepresentation erase namespaceId typeId contents globals)
    {decode? : RuntimeValue → Option T}
    (roundtrip : ∀ value : T, decode? (erase value) = some value)
    (key : StorageKey) :
    (globals.lookup ⟨namespaceId, typeId, key⟩).bind decode? = contents key := by
  rw [represented key]
  exact map_erase_bind_decode roundtrip (contents key)

end FamilyRepresentation

end LeanerIR
