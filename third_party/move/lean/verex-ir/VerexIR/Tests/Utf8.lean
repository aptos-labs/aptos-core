-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Utf8

namespace VerexIR.Tests.Utf8

open VerexIR.Proofs.Denote

private def valid (bytes : SpecVector (SpecInt (.bits 8) false)) : Prop :=
  ∃ text : String, wrapInt 64 false (text.utf8ByteSize : Int) =
    (bytes.values.size : Int) ∧ stringBytes text = bytes

private def invalidFF : SpecVector (SpecInt (.bits 8) false) :=
  ⟨#[Utf8.ofByte 0xff], by decide⟩
private def overlong : SpecVector (SpecInt (.bits 8) false) :=
  ⟨#[Utf8.ofByte 0xc0, Utf8.ofByte 0xaf], by decide⟩
private def truncated : SpecVector (SpecInt (.bits 8) false) :=
  ⟨#[Utf8.ofByte 0xe2, Utf8.ofByte 0x82], by decide⟩
private def surrogate : SpecVector (SpecInt (.bits 8) false) :=
  ⟨#[Utf8.ofByte 0xed, Utf8.ofByte 0xa0, Utf8.ofByte 0x80], by decide⟩
private def beyondUnicode : SpecVector (SpecInt (.bits 8) false) :=
  ⟨#[Utf8.ofByte 0xf4, Utf8.ofByte 0x90, Utf8.ofByte 0x80, Utf8.ofByte 0x80], by decide⟩
private def empty : SpecVector (SpecInt (.bits 8) false) := ⟨#[], by decide⟩
private def asciiNul : SpecVector (SpecInt (.bits 8) false) :=
  ⟨#[Utf8.ofByte 0x41, Utf8.ofByte 0, Utf8.ofByte 0x7f], by decide⟩
private def multibyte : SpecVector (SpecInt (.bits 8) false) :=
  ⟨#[Utf8.ofByte 0xc3, Utf8.ofByte 0xa9, -- é
     Utf8.ofByte 0xe4, Utf8.ofByte 0xb8, Utf8.ofByte 0xad, -- 中
     Utf8.ofByte 0xf0, Utf8.ofByte 0x9f, Utf8.ofByte 0xa6, Utf8.ofByte 0x80], by decide⟩ -- 🦀

example : ¬ valid invalidFF := by unfold valid; rw [Utf8.valid_iff]; decide +kernel
example : ¬ valid overlong := by unfold valid; rw [Utf8.valid_iff]; decide +kernel
example : ¬ valid truncated := by unfold valid; rw [Utf8.valid_iff]; decide +kernel
example : ¬ valid surrogate := by unfold valid; rw [Utf8.valid_iff]; decide +kernel
example : ¬ valid beyondUnicode := by unfold valid; rw [Utf8.valid_iff]; decide +kernel
example : valid empty := by unfold valid; rw [Utf8.valid_iff]; decide +kernel
example : valid asciiNul := by unfold valid; rw [Utf8.valid_iff]; decide +kernel
example : valid multibyte := by unfold valid; rw [Utf8.valid_iff]; decide +kernel

end VerexIR.Tests.Utf8
