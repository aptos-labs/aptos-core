-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

open VerexIR VerexIR.Proofs.Denote

-- These are the wrapped-length and exact-byte relations used by source
-- contracts. Automatic normalization must decide literal input, rather than
-- require the user to manufacture a String witness or unfold the decoder.

private abbrev bytes0 : SpecVector (SpecInt (.bits 8) false) :=
  ⟨#[], by decide⟩

example : (∃ text : String, wrapInt 64 false (text.utf8ByteSize : Int) =
    (bytes0.values.size : Int) ∧ stringBytes text = bytes0) := by
  unfold bytes0
  verex_normalize

private abbrev bytes1 : SpecVector (SpecInt (.bits 8) false) :=
  ⟨#[⟨0, by decide⟩, ⟨65, by decide⟩, ⟨127, by decide⟩], by decide⟩

example : (∃ text : String, wrapInt 64 false (text.utf8ByteSize : Int) =
    (bytes1.values.size : Int) ∧ stringBytes text = bytes1) := by
  unfold bytes1
  verex_normalize

private abbrev bytes2 : SpecVector (SpecInt (.bits 8) false) :=
  ⟨#[⟨194, by decide⟩, ⟨128, by decide⟩], by decide⟩

example : (∃ text : String, wrapInt 64 false (text.utf8ByteSize : Int) =
    (bytes2.values.size : Int) ∧ stringBytes text = bytes2) := by
  unfold bytes2
  verex_normalize

private abbrev bytes3 : SpecVector (SpecInt (.bits 8) false) :=
  ⟨#[⟨224, by decide⟩, ⟨160, by decide⟩, ⟨128, by decide⟩], by decide⟩

example : (∃ text : String, wrapInt 64 false (text.utf8ByteSize : Int) =
    (bytes3.values.size : Int) ∧ stringBytes text = bytes3) := by
  unfold bytes3
  verex_normalize

private abbrev bytes4 : SpecVector (SpecInt (.bits 8) false) :=
  ⟨#[⟨244, by decide⟩, ⟨143, by decide⟩, ⟨191, by decide⟩, ⟨191, by decide⟩], by decide⟩

example : (∃ text : String, wrapInt 64 false (text.utf8ByteSize : Int) =
    (bytes4.values.size : Int) ∧ stringBytes text = bytes4) := by
  unfold bytes4
  verex_normalize

private abbrev bytes5 : SpecVector (SpecInt (.bits 8) false) :=
  ⟨#[⟨128, by decide⟩], by decide⟩

example : ¬ (∃ text : String, wrapInt 64 false (text.utf8ByteSize : Int) =
    (bytes5.values.size : Int) ∧ stringBytes text = bytes5) := by
  unfold bytes5
  verex_normalize

private abbrev bytes6 : SpecVector (SpecInt (.bits 8) false) :=
  ⟨#[⟨192, by decide⟩, ⟨128, by decide⟩], by decide⟩

example : ¬ (∃ text : String, wrapInt 64 false (text.utf8ByteSize : Int) =
    (bytes6.values.size : Int) ∧ stringBytes text = bytes6) := by
  unfold bytes6
  verex_normalize

private abbrev bytes7 : SpecVector (SpecInt (.bits 8) false) :=
  ⟨#[⟨237, by decide⟩, ⟨160, by decide⟩, ⟨128, by decide⟩], by decide⟩

example : ¬ (∃ text : String, wrapInt 64 false (text.utf8ByteSize : Int) =
    (bytes7.values.size : Int) ∧ stringBytes text = bytes7) := by
  unfold bytes7
  verex_normalize

private abbrev bytes8 : SpecVector (SpecInt (.bits 8) false) :=
  ⟨#[⟨244, by decide⟩, ⟨144, by decide⟩, ⟨128, by decide⟩, ⟨128, by decide⟩], by decide⟩

example : ¬ (∃ text : String, wrapInt 64 false (text.utf8ByteSize : Int) =
    (bytes8.values.size : Int) ∧ stringBytes text = bytes8) := by
  unfold bytes8
  verex_normalize

private abbrev bytes9 : SpecVector (SpecInt (.bits 8) false) :=
  ⟨#[⟨226, by decide⟩, ⟨130, by decide⟩], by decide⟩

example : ¬ (∃ text : String, wrapInt 64 false (text.utf8ByteSize : Int) =
    (bytes9.values.size : Int) ∧ stringBytes text = bytes9) := by
  unfold bytes9
  verex_normalize

-- Symbolic encodings remain valid without recursive decoder expansion.
example (text : String) :
    (∃ decoded : String, wrapInt 64 false (decoded.utf8ByteSize : Int) =
      ((stringBytes text).values.size : Int) ∧ stringBytes decoded = stringBytes text) := by
  verex_normalize

-- An unknown input keeps one opaque validator call in the normal form.
example (bytes : SpecVector (SpecInt (.bits 8) false)) :
    (∃ text : String, wrapInt 64 false (text.utf8ByteSize : Int) =
      (bytes.values.size : Int) ∧ stringBytes text = bytes) ↔
      (Utf8.toByteArray bytes).validateUTF8 = true := by
  verex_normalize
