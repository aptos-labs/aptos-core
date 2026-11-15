-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
The literal UTF-8 normalizer accepts at most 64 bytes. These boundary cases
measure both successful validation branches and rejection after traversing
an otherwise valid prefix. The kernel checks the reduction certificates;
no native decision axiom is used. Existing performance tolerances apply.
-/

namespace VerexSL.Tests.Utf8Performance
open VerexIR VerexIR.Proofs.Denote

#verex_measure

open Lean Elab Command in
elab "#measure_utf8 " name:ident " : " goal:term " := " proof:term : command => do
  VerexSL.Perf.measure s!"utf8.{name.getId}" ((← getCurrNamespace) ++ name.getId) do
    elabCommand (← `(theorem $name : $goal := $proof))

private abbrev byte65 : SpecInt (.bits 8) false := ⟨65, by decide⟩
private abbrev byte128 : SpecInt (.bits 8) false := ⟨128, by decide⟩
private abbrev byte152 : SpecInt (.bits 8) false := ⟨152, by decide⟩
private abbrev byte159 : SpecInt (.bits 8) false := ⟨159, by decide⟩
private abbrev byte240 : SpecInt (.bits 8) false := ⟨240, by decide⟩

private abbrev ascii64Bytes : SpecVector (SpecInt (.bits 8) false) :=
  ⟨#[
    byte65, byte65, byte65, byte65, byte65, byte65, byte65, byte65,
    byte65, byte65, byte65, byte65, byte65, byte65, byte65, byte65,
    byte65, byte65, byte65, byte65, byte65, byte65, byte65, byte65,
    byte65, byte65, byte65, byte65, byte65, byte65, byte65, byte65,
    byte65, byte65, byte65, byte65, byte65, byte65, byte65, byte65,
    byte65, byte65, byte65, byte65, byte65, byte65, byte65, byte65,
    byte65, byte65, byte65, byte65, byte65, byte65, byte65, byte65,
    byte65, byte65, byte65, byte65, byte65, byte65, byte65, byte65
  ], by decide⟩

#measure_utf8 ascii64 : (∃ text : String,
    wrapInt 64 false (text.utf8ByteSize : Int) = (ascii64Bytes.values.size : Int) ∧
      stringBytes text = ascii64Bytes) := by
  unfold ascii64Bytes
  verex_normalize

private abbrev fourByte64Bytes : SpecVector (SpecInt (.bits 8) false) :=
  ⟨#[
    byte240, byte159, byte152, byte128, byte240, byte159, byte152, byte128,
    byte240, byte159, byte152, byte128, byte240, byte159, byte152, byte128,
    byte240, byte159, byte152, byte128, byte240, byte159, byte152, byte128,
    byte240, byte159, byte152, byte128, byte240, byte159, byte152, byte128,
    byte240, byte159, byte152, byte128, byte240, byte159, byte152, byte128,
    byte240, byte159, byte152, byte128, byte240, byte159, byte152, byte128,
    byte240, byte159, byte152, byte128, byte240, byte159, byte152, byte128,
    byte240, byte159, byte152, byte128, byte240, byte159, byte152, byte128
  ], by decide⟩

#measure_utf8 fourByte64 : (∃ text : String,
    wrapInt 64 false (text.utf8ByteSize : Int) = (fourByte64Bytes.values.size : Int) ∧
      stringBytes text = fourByte64Bytes) := by
  unfold fourByte64Bytes
  verex_normalize

private abbrev invalidLast64Bytes : SpecVector (SpecInt (.bits 8) false) :=
  ⟨#[
    byte65, byte65, byte65, byte65, byte65, byte65, byte65, byte65,
    byte65, byte65, byte65, byte65, byte65, byte65, byte65, byte65,
    byte65, byte65, byte65, byte65, byte65, byte65, byte65, byte65,
    byte65, byte65, byte65, byte65, byte65, byte65, byte65, byte65,
    byte65, byte65, byte65, byte65, byte65, byte65, byte65, byte65,
    byte65, byte65, byte65, byte65, byte65, byte65, byte65, byte65,
    byte65, byte65, byte65, byte65, byte65, byte65, byte65, byte65,
    byte65, byte65, byte65, byte65, byte65, byte65, byte65, byte128
  ], by decide⟩

#measure_utf8 invalidLast64 : ¬ (∃ text : String,
    wrapInt 64 false (text.utf8ByteSize : Int) = (invalidLast64Bytes.values.size : Int) ∧
      stringBytes text = invalidLast64Bytes) := by
  unfold invalidLast64Bytes
  verex_normalize

#verex_perf "VerexSL/Tests/Utf8Performance.exp"

end VerexSL.Tests.Utf8Performance
