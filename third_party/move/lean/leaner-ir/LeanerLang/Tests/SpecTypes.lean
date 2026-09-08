-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/- A literal vector length is not a multiplication goal. The generic
arithmetic closer must not unify it with a product-range certificate. -/
set_option maxHeartbeats 1000 in
example (value : Int) : LeanerLang.Contract.lengthVector
    (.vector #[.integer value]) < 18446744073709551615 := by
  leaner_certified_close!

set_option maxHeartbeats 1000 in
example (left right upper : Int) (leftNonnegative : 0 ≤ left)
    (rightNonnegative : 0 ≤ right)
    (failed : 0 ≤ left * right → upper < left * right) :
    upper < left * right := by
  leaner_product_upper

/- Decoded vector preconditions expose natural bounds before body
normalization, including the explicit `Int.ofNat` used by runtime lengths. -/
set_option maxHeartbeats 1000 in
example (xs : Array LeanerIR.RuntimeValue)
    (nonempty : 0 < LeanerLang.Contract.lengthVector (.vector xs)) :
    0 < xs.size := by
  leaner_normalize_vector_lengths
  exact nonempty

set_option maxHeartbeats 1000 in
example {α : Type} (encode : α → LeanerIR.RuntimeValue) (xs ys : Array α)
    (ordered : LeanerLang.Contract.lengthVector (.vector (xs.map encode)) ≤
      LeanerLang.Contract.lengthVector (.vector (ys.map encode))) :
    xs.size ≤ ys.size := by
  leaner_normalize_vector_lengths
  exact ordered

set_option maxHeartbeats 1000 in
example (xs : Array Int) (index : Nat) (bound : index < xs.size) :
    ((LeanerIR.RuntimeValue.vector (xs.map LeanerIR.RuntimeValue.integer)).field index).asInt =
      xs[index] := by
  leaner_spec_vector_read

set_option maxHeartbeats 1000 in
example (xs : Array LeanerIR.RuntimeValue) :
    ((LeanerIR.RuntimeValue.vector xs).field xs.size).asInt = 0 := by
  leaner_spec_vector_read

set_option maxHeartbeats 1000 in
example (xs : Array Int) (index : Nat) (replacement : Int) (bound : index < xs.size) :
    ((LeanerIR.RuntimeValue.vector ((xs.map LeanerIR.RuntimeValue.integer).setIfInBounds
      index (.integer replacement))).field index).asInt = replacement := by
  leaner_spec_vector_read

/-!
# Generated typed-twin tests

`#leaner_unit` materializes one typed twin per supported struct declaration
and keyed accessors per storable family.  These assertions pin the generated
vocabulary: the erasure/decoding roundtrip, certified field bounds, the
defaults, and the agreement of the family accessors with the runtime map.
-/

namespace LeanerLang.Tests.SpecTypes

leaner module 0x42::twins where
  struct Amount has Copy, Drop, Store where
    value : u64
  struct Coin has Key where
    amount : Amount
    flag : Bool
    owner : Address
  struct Buffer has Copy, Drop, Store where
    bytes : Vector<u8>
    flags : Vector<Bool>
    amounts : Vector<Amount>
    rows : Vector<Vector<u64> >
  struct LoopBits has Copy, Drop, Store where
    length : u64
    flags : Vector<Bool>
  public fun balance_of(addr : Address) -> u64 := Coin[addr].amount.value

#leaner_unit 0x42::twins

open LeanerIR
open «0x42»

/-- The roundtrip of a twin with a nested twin and mixed scalar fields is
one `simp` through the tagged codec lemmas. -/
example (coin : twins.Coin) :
    twins.Coin.decode? (twins.Coin.erase coin) = some coin := by simp

/-- Decoding through the erasure recovers typed contents wholesale. -/
example (contents : Option twins.Coin) :
    (contents.map twins.Coin.erase).bind twins.Coin.decode? = contents := by
  simp

/-- Vector fields retain their element's native representation, including
nested vectors and nominal elements, and roundtrip through their codecs. -/
example (buffer : twins.Buffer) :
    twins.Buffer.decode? (twins.Buffer.erase buffer) = some buffer := by simp

example (buffer : twins.Buffer) :
    SemanticOperations.Plain (twins.Buffer.erase buffer) := by leaner_plain

example (buffer : twins.Buffer) :
    SemanticOperations.outermostBorrows (twins.Buffer.erase buffer) = #[] := by simp

set_option maxHeartbeats 1000 in
/-- A loop header decodes the actual runtime aggregate, not an already
packed twin. The proof cost must not depend on the symbolic vector length. -/
example (length : Int) (fits : IntegerValueFits (.bits 64) false length)
    (flags : SpecVector Bool) :
    ∃ decoded : twins.LoopBits,
      twins.LoopBits.decode?
        (.nominal ⟨⟨0⟩, 3⟩ none #[.integer length, .vector (flags.values.map .bool)]) =
        some decoded := by
  leaner_certified_close!

set_option maxHeartbeats 1000 in
example (initial current : SpecVector Bool)
    (_lengths : (current.values.map LeanerIR.Proofs.Codec.bool.encode).size =
      initial.values.size % Int.natAbs (18446744073709551616 : Int)) :
    (current.values.size : Int) % 18446744073709551616 =
      (initial.values.size : Int) % 18446744073709551616 := by
  leaner_certified_close!

set_option maxHeartbeats 1000 in
example (flags : SpecVector Bool) :
    LeanerIR.Proofs.Codec.bool.boundedVector.decode?
      (.vector (flags.values.map .bool)) = some flags := by
  simp only [lir_data_norm]

set_option maxHeartbeats 1000 in
example (initial saved : SpecVector Bool)
    (_equal : initial.values.map RuntimeValue.bool =
      saved.values.map LeanerIR.Proofs.Codec.bool.encode) :
    RuntimeValue.vector (initial.values.map RuntimeValue.bool) =
      .vector (({ values := saved.values.toList.toArray, bounded := saved.bounded } :
        SpecVector Bool).values.map RuntimeValue.bool) := by
  leaner_certified_close!

set_option maxHeartbeats 1000 in
example (flags : SpecVector Bool)
    (_unrelated : (List.replicate 100000 ()).length = 100000) :
    Int.ofNat (flags.values.map RuntimeValue.bool).size =
      (flags.values.size : Int) % 18446744073709551616 := by
  leaner_certified_close!

set_option maxHeartbeats 1000 in
example (flags : SpecVector Bool) :
    LeanerLang.Contract.lengthVector (.vector (flags.values.map RuntimeValue.bool)) =
      (flags.values.size : Int) % 18446744073709551616 := by
  leaner_certified_close!

set_option maxHeartbeats 1000 in
example (flags : SpecVector Bool) (index : Nat) :
    ∃ updated : SpecVector Bool,
      LeanerIR.Proofs.Codec.bool.boundedVector.decode?
        (.vector ((flags.values.toList.map RuntimeValue.bool).set index (.bool false)).toArray) =
          some updated ∧ updated.values.size = flags.values.size := by
  simp only [lir_data_norm]
  leaner_certified_close!

set_option maxHeartbeats 1000 in
example (flags : SpecVector Bool) (index : Nat) :
    ∃ updated : SpecVector Bool,
      LeanerIR.Proofs.Codec.bool.boundedVector.decode?
        (.vector ((flags.values.toList.map RuntimeValue.bool).set index (.bool false)).toArray) =
          some updated ∧
      .vector ((flags.values.toList.map RuntimeValue.bool).set index (.bool false)).toArray =
        LeanerIR.Proofs.Codec.bool.boundedVector.encode updated := by
  simp only [lir_data_norm]
  leaner_certified_close!

example : (default : twins.Buffer).rows.values = #[] := rfl

/-- Each Move vector field, including a nested row, carries its own bound. -/
example (buffer : twins.Buffer) : buffer.rows.values.size < 2 ^ 64 :=
  buffer.rows.bounded

example (row : LeanerIR.SpecVector (LeanerIR.SpecInt (.bits 64) false)) :
    row.values.size < 2 ^ 64 := row.bounded

/-- The generated default inhabits every field. -/
example : (default : twins.Coin).flag = false := rfl
example : (default : twins.Coin).amount.value.val = 0 := rfl

/-- Certified integer fields carry their bounds in `omega` form. -/
example (amount : twins.Amount) : amount.value.val ≤ 18446744073709551615 := by
  have bounds := amount.value.unsigned_bounds
  omega

/-- The existence test is the raw runtime lookup — the same term a program's
`exists<Coin>` evaluates, with nothing between them. -/
example (globals : GlobalMap) (key : RuntimeValue) :
    twins.Coin.contains globals key
      = (globals.lookup (twins.Coin.key key)).isSome := rfl

/-- The typed read decodes the same lookup. -/
example (globals : GlobalMap) (key : RuntimeValue) :
    twins.Coin.read globals key
      = (globals.lookup (twins.Coin.key key)).bind twins.Coin.decode? := rfl

/-- Publishing an erasure is read back as its typed value. -/
example (globals : GlobalMap) (coin : twins.Coin) (key : RuntimeValue) :
    twins.Coin.read
      (globals.insert (twins.Coin.key key) (twins.Coin.erase coin)) key
      = some coin := by
  simp [twins.Coin.read, twins.Coin.readAt, twins.Coin.key]

end LeanerLang.Tests.SpecTypes

namespace LeanerLang.Tests.EnumCodecs

leaner module 0x42::enum_codecs where
  enum Choice has Copy, Drop, Store where
    | Empty
    | Number (value : u8)
    | OtherNumber (value : u8)
    | Flag (value : Bool)

#leaner_unit 0x42::enum_codecs

open LeanerIR «0x42».enum_codecs

-- Raw payload equations select the tag without requiring an erased
-- SpecInt metavariable to unify with a scalar integer expression.
example (value : RuntimeValue) :
    Choice.decode? (.nominal ⟨⟨0⟩, 0⟩ (some "Number") #[value]) =
      (decodeInt? (.bits 8) false value).map Choice.Number := by
  rw [Choice.decode?_Number_literal]
  cases decodeInt? (.bits 8) false value <;> rfl

example : Choice.decode? (.nominal ⟨⟨0⟩, 0⟩ (some "Number") #[.integer 256]) = none := by
  simp [decodeInt?, IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]

example : Choice.decode? (.nominal ⟨⟨0⟩, 0⟩ (some "Number") #[.bool true]) = none := by
  simp [decodeInt?]

example : Choice.decode? (.nominal ⟨⟨0⟩, 0⟩ (some "Number") #[]) = none := by
  simp [Choice.decode?]

example : Choice.decode? (.nominal ⟨⟨0⟩, 0⟩ (some "Missing") #[]) = none := by
  simp [Choice.decode?]

example : Choice.decode? (.nominal ⟨⟨0⟩, 1⟩ (some "Empty") #[]) = none := by
  simp [Choice.decode?]

set_option maxHeartbeats 10000 in
/-- A returned native enum determines its runtime representation. Only
the generated decoder is opened; no callee implementation is involved. -/
example (runtime : RuntimeValue) (value : Choice)
    (decoded : Choice.decode? runtime = some value) : runtime = Choice.erase value := by
  leaner_call_results
  all_goals simp_all [Choice.erase]

set_option maxHeartbeats 10000 in
/-- A callee's variant guarantee removes incompatible decoder branches
before the caller continues. -/
example (runtime : RuntimeValue) (value : Choice)
    (decoded : Choice.decode? runtime = some value)
    (post : Choice.erase value = .nominal ⟨⟨0⟩, 0⟩ (some "Number") #[.integer 7]) :
    runtime = .nominal ⟨⟨0⟩, 0⟩ (some "Number") #[.integer 7] := by
  leaner_call_results
  run_tac
    unless (← Lean.Elab.Tactic.getGoals).length ≤ 1 do
      throwError "impossible enum variants were not pruned"
  all_goals simp_all

end LeanerLang.Tests.EnumCodecs

namespace LeanerLang.Tests.VectorProfiles

leaner namespace rust_vector_twins where
  struct Buffer where
    values : Vector<u64>

#leaner_unit rust_vector_twins

/-- The Move-specific bound must not change Rust's existing native array. -/
example (buffer : rust_vector_twins.Buffer) : Array (LeanerIR.SpecInt (.bits 64) false) :=
  buffer.values

example (buffer : rust_vector_twins.Buffer) :
    rust_vector_twins.Buffer.decode? (rust_vector_twins.Buffer.erase buffer) = some buffer := by
  simp

end LeanerLang.Tests.VectorProfiles

namespace LeanerLang.Tests.GenericEnumCodecs

leaner module 0x42::generic_enum_codecs where
  enum Choice {T has Copy, Drop, Store} {U has Copy, Drop, Store} has Copy, Drop, Store where
    | Empty
    | First (value : T)
    | Second (value : U)
    | Batch (values : Vector<T>)

#leaner_unit 0x42::generic_enum_codecs

open LeanerIR «0x42».generic_enum_codecs

set_option maxHeartbeats 1000 in
example {α β : Type} (left : Proofs.Codec α RuntimeValue)
    (right : Proofs.Codec β RuntimeValue) (value : Choice α β) :
    Choice.decode? left right (Choice.erase left right value) = some value := by
  simp

set_option maxHeartbeats 1000 in
example {α β : Type} (left : Proofs.Codec α RuntimeValue)
    (right : Proofs.Codec β RuntimeValue) (contents : Option (Choice α β)) :
    (contents.map (Choice.erase left right)).bind (Choice.decode? left right) = contents := by
  simp

set_option maxHeartbeats 1000 in
example {α β : Type} (left : Proofs.Codec α RuntimeValue)
    (right : Proofs.Codec β RuntimeValue) (value : RuntimeValue) :
    Choice.decode? left right (.nominal ⟨⟨0⟩, 0⟩ (some "Second") #[value]) =
      (right.decode? value).map Choice.Second := by
  rw [Choice.decode?_Second_literal]
  cases right.decode? value <;> rfl

example : Choice.decode? Proofs.Codec.bool Proofs.Codec.bool
    (.nominal ⟨⟨0⟩, 0⟩ (some "First") #[.integer 7]) = none := by
  simp [Proofs.Codec.bool, decodeBool?]

example : Choice.decode? Proofs.Codec.bool Proofs.Codec.bool
    (.nominal ⟨⟨0⟩, 0⟩ (some "Missing") #[]) = none := by
  simp [Choice.decode?]

open Lean Elab Command in
run_cmd do
  for name in [``Choice.decode?_erase, ``Choice.decode?_map_erase,
      ``Choice.decode?_First_literal, ``Choice.decode?_Second_literal,
      ``Choice.decode?_Batch_literal] do
    if (← collectAxioms name).contains ``sorryAx then
      throwError "generic enum helper contains an admission: {name}"
  if (← getEnv).contains `«0x42».generic_enum_codecs.Choice.plain_erase then
    throwError "arbitrary parameter codecs were assumed loan-free"

end LeanerLang.Tests.GenericEnumCodecs

namespace LeanerLang.Tests.GenericDataCertificates

open LeanerIR LeanerIR.Proofs LeanerIR.SemanticOperations

-- These rules use a certificate for the actual encoded argument, not an
-- assumption that every inhabitant of an arbitrary codec is loan-free.
set_option maxHeartbeats 1000 in
example {α : Type} (codec : Codec α RuntimeValue) (value : α)
    (plain : Plain (codec.encode value)) :
    collectPruned borrowEntry? (codec.encode value) = #[] := by
  simp only [lir_eval]

set_option maxHeartbeats 1000 in
example {α : Type} (codec : Codec α RuntimeValue) (value : α) (loan : Nat)
    (plain : Plain (codec.encode value)) :
    findFirst (holeMark? loan) (codec.encode value) = none := by
  simp only [lir_eval]

set_option maxHeartbeats 1000 in
example {α : Type} (codec : Codec α RuntimeValue) (left right : α)
    (equal : (codec.encode left == codec.encode right) = true) : left = right := by
  leaner_certified_close!

set_option maxHeartbeats 1000 in
example {α : Type} (codec : Codec α RuntimeValue) (left right : α)
    (equal : codec.encode left = codec.encode right)
    (plain : Plain (codec.encode left)) : Plain (codec.encode right) := by
  leaner_certified_close!

example : ¬ Plain ((Codec.identity RuntimeValue).encode (.borrow 0 .unit)) := by
  simp only [Codec.identity, id_eq, Plain.borrow_iff, not_false_eq_true]

set_option maxHeartbeats 1000 in
example (left right : Int) (answer : Bool)
    (summary : answer = true ↔
      RuntimeValue.nominal ⟨⟨0⟩, 0⟩ none #[.integer left] =
        RuntimeValue.nominal ⟨⟨0⟩, 0⟩ none #[.integer right]) :
    answer = true ↔ left = right := by
  leaner_certified_close!

set_option maxHeartbeats 1000 in
example (input result : RuntimeValue) (value : Int)
    (decoded : RuntimeValue.integer value = input)
    (summary : result = input) : result.asInt = value := by
  leaner_call_normalize []

-- The forward equation expands into itself, but its reverse safely picks
-- the known variant. The cycle guard must check the selected direction.
set_option maxHeartbeats 1000 in
example (flag : Bool)
    (decoded : RuntimeValue.integer 0 =
      if flag then RuntimeValue.integer 0 else RuntimeValue.integer 1) :
    (if flag then RuntimeValue.integer 0 else RuntimeValue.integer 1).asInt = 0 := by
  leaner_call_normalize []

set_option maxHeartbeats 1000 in
example (runtime : RuntimeValue) (value : SpecVector RuntimeValue)
    (decoded : (Codec.identity RuntimeValue).boundedVector.decode? runtime = some value) :
    runtime = .vector value.values := by
  leaner_call_results
  rfl

set_option maxHeartbeats 1000 in
example : True := by
  fail_if_success
    have : ∃ value, LeanerIR.decodeInt? (.bits 64) false
        (.integer (-1)) = some value := by
      leaner_certified_close!
  trivial

end LeanerLang.Tests.GenericDataCertificates

namespace LeanerLang.Tests.VectorCertificates

open LeanerIR LeanerIR.Proofs.Denotation

set_option maxHeartbeats 1000 in
example (a b c : RuntimeValue) :
    (#[a, b, c].swapIfInBounds 0 2) = #[c, b, a] := by
  simp only [lir_eval]

set_option maxHeartbeats 1000 in
example (a : RuntimeValue) : (#[a].swapIfInBounds 0 1) = #[a] := by
  simp only [lir_eval]

-- Symbolic elements must not force a fallback through the source arena.
set_option maxHeartbeats 1000 in
example (a b c : RuntimeValue) (frame : RuntimeFrame) (state : RuntimeState) :
    PrimitiveLocationOperation.swapVector.evaluate?
        #[.vector #[a, b, c], .integer 0, .integer 2] frame state =
      some (.value frame state (.vector #[c, b, a])) := by
  rfl

set_option maxHeartbeats 1000 in
example (a : RuntimeValue) (frame : RuntimeFrame) (state : RuntimeState) :
    PrimitiveLocationOperation.swapVector.evaluate?
        #[.vector #[a], .integer 0, .integer 0] frame state =
      some (.value frame state (.vector #[a])) := by
  rfl

set_option maxHeartbeats 1000 in
example (a b c : RuntimeValue) (frame : RuntimeFrame) (state : RuntimeState) :
    PrimitiveLocationOperation.concatVector.evaluate?
        #[.vector #[a], .vector #[b, c]] frame state =
      some (.value frame state (.vector #[a, b, c])) := by
  rfl

set_option maxHeartbeats 1000 in
example (a b c : RuntimeValue) (frame : RuntimeFrame) (state : RuntimeState) :
    PrimitiveLocationOperation.slice.evaluate?
        #[.vector #[a, b, c], .integer 1, .integer 3] frame state =
      some (.value frame state (.vector #[b, c])) := by
  rfl

-- Raw slicing rejects, rather than clamps, a beyond-end range.
set_option maxHeartbeats 1000 in
example (a : RuntimeValue) (frame : RuntimeFrame) (state : RuntimeState) :
    PrimitiveLocationOperation.slice.evaluate?
        #[.vector #[a], .integer 0, .integer 2] frame state =
      some (.throw_ frame state .abort #[.integer 0, .integer 2]) := by
  rfl

set_option maxHeartbeats 1000 in
example (a b c d e : RuntimeValue) (frame : RuntimeFrame) (state : RuntimeState) :
    PrimitiveLocationOperation.reverseSliceVector.evaluate?
        #[.vector #[a, b, c, d, e], .integer 1, .integer 4] frame state =
      some (.value frame state (.vector #[a, d, c, b, e])) := by
  rfl

set_option maxHeartbeats 1000 in
example (frame : RuntimeFrame) (state : RuntimeState) :
    PrimitiveLocationOperation.containsVector.evaluate?
        #[.vector #[.integer 4, .integer 5, .integer 4], .integer 4] frame state =
      some (.value frame state (.bool true)) := by
  simp only [lir_eval]
  simp [Option.any]

set_option maxHeartbeats 1000 in
example (frame : RuntimeFrame) (state : RuntimeState) :
    (PrimitiveLocationOperation.indexOfVector (.integer (.bits 64) false)).evaluate?
        #[.vector #[.integer 4, .integer 5, .integer 4], .integer 4] frame state =
      some (.value frame state (.tuple #[.bool true, .integer 0])) := by
  simp only [lir_eval]
  simp [Option.any]

set_option maxHeartbeats 1000 in
example (frame : RuntimeFrame) (state : RuntimeState) :
    (PrimitiveLocationOperation.indexOfVector (.integer (.bits 64) false)).evaluate?
        #[.vector #[.integer 4], .integer 5] frame state =
      some (.value frame state (.tuple #[.bool false, .integer 0])) := by
  simp only [lir_eval]
  simp [Option.any]

set_option maxHeartbeats 1000 in
example (frame : RuntimeFrame) (state : RuntimeState) :
    PrimitiveLocationOperation.destroyEmptyVector.evaluate? #[.vector #[]] frame state =
      some (.value frame state .unit) := by
  rfl

set_option maxHeartbeats 1000 in
example (a : RuntimeValue) (frame : RuntimeFrame) (state : RuntimeState) :
    PrimitiveLocationOperation.destroyEmptyVector.evaluate? #[.vector #[a]] frame state =
      some (.throw_ frame state .abort #[]) := by
  rfl

set_option maxHeartbeats 1000 in
example (a : RuntimeValue) (failure : LeanerIR.ThrowKind)
    (frame : RuntimeFrame) (state : RuntimeState) :
    (PrimitiveLocationOperation.checkVectorIndex failure).evaluate?
        #[.vector #[a], .integer 1] frame state =
      some (.throw_ frame state failure #[.integer 1]) := by
  rfl

set_option maxHeartbeats 1000 in
example (a : RuntimeValue) (failure : LeanerIR.ThrowKind)
    (frame : RuntimeFrame) (state : RuntimeState) :
    (PrimitiveLocationOperation.checkVectorIndex failure).evaluate?
        #[.vector #[a], .integer 0] frame state =
      some (.value frame state .unit) := by
  rfl

end LeanerLang.Tests.VectorCertificates
