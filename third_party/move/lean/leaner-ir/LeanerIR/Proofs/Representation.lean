-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.Typing

/-!
# Typed specification representation of runtime values

The frontends generate one typed twin per struct declaration, with an
`erase`/`decode?` pair and their roundtrip; this file owns the generic
vocabulary those generated declarations share: certified integers and the
canonical scalar decoders.
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

/-! ## Certified Move vectors

Move's native vector carries an unsigned-64 length bound.
The neutral runtime and non-Move native arrays remain unrestricted. -/

structure SpecVector (α : Type) where
  values : Array α
  bounded : values.size < 2 ^ 64

@[ext, grind ext] theorem SpecVector.ext {a b : SpecVector α} (h : a.values = b.values) : a = b := by
  cases a
  cases b
  cases h
  rfl

instance : Inhabited (SpecVector α) := ⟨⟨#[], by change 0 < 2 ^ 64; decide⟩⟩

/-- Vectors are equal exactly when their values are: the form a goal about
vectors is decided in. -/
theorem SpecVector.eq_iff_values (left right : SpecVector α) :
    left = right ↔ left.values = right.values :=
  ⟨fun h => h ▸ rfl, SpecVector.ext⟩

/-- The vector of the images of the elements: a vector of twins viewed
natively element by element. -/
def SpecVector.map (f : α → β) (vector : SpecVector α) : SpecVector β :=
  ⟨vector.values.map f, by simpa only [Array.size_map] using vector.bounded⟩

@[simp] theorem SpecVector.map_values (f : α → β) (vector : SpecVector α) :
    (vector.map f).values = vector.values.map f := rfl

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

/-! Successful canonical scalar decoding determines the runtime shape.
These are stronger than an arbitrary codec's `decode_encode` law: only
these concrete decoders reject all noncanonical representations. Modular
callers use the shared lemmas instead of splitting every runtime constructor
again for each call site. -/

theorem decodeInt?_shape {width : IntWidth} {signed : Bool}
    {runtime : RuntimeValue} {value : SpecInt width signed}
    (decoded : decodeInt? width signed runtime = some value) :
    runtime = .integer value.val := by
  cases runtime <;> simp only [decodeInt?] at decoded <;> try contradiction
  split at decoded
  · cases Option.some.inj decoded
    rfl
  · contradiction

theorem decodeBool?_shape {runtime : RuntimeValue} {value : Bool}
    (decoded : decodeBool? runtime = some value) : runtime = .bool value := by
  cases runtime <;> simp_all [decodeBool?]

theorem decodeString?_shape {runtime : RuntimeValue} {value : String}
    (decoded : decodeString? runtime = some value) : runtime = .string value := by
  cases runtime <;> simp_all [decodeString?]

theorem decodeAddress?_shape {runtime : RuntimeValue} {value : String}
    (decoded : decodeAddress? runtime = some value) : runtime = .address value := by
  cases runtime <;> simp_all [decodeAddress?]

theorem decodeSigner?_shape {runtime : RuntimeValue} {value : String}
    (decoded : decodeSigner? runtime = some value) : runtime = .signer value := by
  cases runtime <;> simp_all [decodeSigner?]

theorem decodeBytes?_shape {runtime : RuntimeValue} {value : Array UInt8}
    (decoded : decodeBytes? runtime = some value) : runtime = .bytes value := by
  cases runtime <;> simp_all [decodeBytes?]

theorem decodeUnit?_shape {runtime : RuntimeValue} {value : Unit}
    (decoded : decodeUnit? runtime = some value) : runtime = .unit := by
  cases runtime <;> simp_all [decodeUnit?]

end LeanerIR
