-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Values

/-!
# What the table rules give

UNTRUSTED (checked by the kernel). A table holds entries only in a program
whose module `0x1::table` keeps the table rules (`Program.tablesHoldEntries`,
`Value.wellFormed`), and that module unpacks no table (rule 2,
`CompiledModule.followsTableRules`). Code unpacks only structs its own module
defines, under a name that resolves back to the definition
(`Program.definedStruct?`), so the module unpacking a table type is the
program's module `0x1::table`: in a program whose tables hold entries, no
`Unpack` of a table runs (`unpack_noEntries`), and a table that is unpacked
has none.
-/

namespace MoveBytecode.Proofs

variable {P : Program}

/-- **The table rules forbid unpacking a table** (rule 2): no instruction of a
function with code of a module keeping them unpacks a struct named `Table`. -/
theorem followsTableRules_unpack {m : CompiledModule} (h : m.followsTableRules = true)
    {d : FunctionDefinition} (hd : d ∈ m.functionDefs.toList) {code : CodeUnit}
    (hc : d.code = some code) {fh : FunctionHandle} (hfh : m.functionDefHandle d = some fh)
    {b : Bytecode} (hb : b ∈ code.code.toList) :
    (∀ sd, b = .unpack sd → ((m.structDefs[sd]?).bind m.structDefName) ≠ some "Table") ∧
    (∀ k x, b = .unpackGeneric k → m.structDefInstantiations[k]? = some x →
      ((m.structDefs[x.def_]?).bind m.structDefName) ≠ some "Table") := by
  unfold CompiledModule.followsTableRules at h
  simp only [Bool.and_eq_true, List.all_eq_true] at h
  obtain ⟨-, hall⟩ := h
  have hd' := hall d hd
  rw [hc, hfh] at hd'
  simp only [List.all_eq_true] at hd'
  obtain ⟨i, hi⟩ : ∃ i, (b, i) ∈ code.code.toList.zipIdx := by
    obtain ⟨i, hi, rfl⟩ := List.getElem_of_mem hb
    exact ⟨i, List.mem_zipIdx_iff_getElem?.2 (by simp)⟩
  have hbi := hd' (b, i) hi
  constructor
  · intro sd hsd
    subst hsd
    simp only [Bytecode.tableRuleUse, Bool.not_eq_true', beq_eq_false_iff_ne] at hbi
    exact hbi
  · intro k x hk hx
    subst hk
    simp only [Bytecode.tableRuleUse, hx, Option.all_some, Bool.not_eq_true',
      beq_eq_false_iff_ne] at hbi
    exact hbi

/-- **No table is unpacked where tables hold entries.** An `Unpack` in the
code of function `info`, of a struct definition `d` of its module that
`definedStruct?` resolves to a table type, runs only in a program whose
tables hold no entries. -/
theorem unpack_noEntries {fref : FunctionRef} {info : FunctionInfo} {code : CodeUnit}
    (hf : P.function? fref = some info) (hcode : info.definition.code = some code)
    {instr : Bytecode} (hin : instr ∈ code.code.toList) {d : StructDefinitionIndex}
    (hi : instr = .unpack d ∨ ∃ i x, instr = .unpackGeneric i ∧
      info.module.structDefInstantiations[i]? = some x ∧ x.def_ = d)
    {sid₀ : StructId} (hd : P.definedStruct? fref.module d = some sid₀)
    (ht : sid₀.isTable = true) : P.tablesHoldEntries = false := by
  obtain ⟨hm, hfd, hfh⟩ := function?_lookup hf
  -- The definition's name, and its module's resolution.
  simp only [Program.definedStruct?, Program.struct?, hm, bind, Option.bind_eq_some_iff,
    pure] at hd
  obtain ⟨sinfo, ⟨m', hm', id, hid, sd, hsd, sh, hsh, hsinfo⟩, name, hname, hres⟩ := hd
  simp only [Option.some.injEq] at hm' hsinfo
  subst hm' hsinfo
  split at hres
  · next hrs =>
    simp only [Option.some.injEq] at hres
    subst hres
    simp only at hname hrs
    simp only [StructIdOf.isTable, Bool.and_eq_true, beq_iff_eq] at ht
    obtain ⟨hmod, hnm⟩ := ht
    subst hmod hnm
    simp only [Program.resolveStruct, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at hrs
    obtain ⟨mi, hfind, _, -, _, -, hmi⟩ := hrs
    simp only [StructRef.mk.injEq] at hmi
    obtain ⟨rfl, -⟩ := hmi
    cases htab : P.tablesHoldEntries with
    | false => rfl
    | true =>
      exfalso
      simp only [Program.tablesHoldEntries, hfind, hm, Option.any_some] at htab
      obtain ⟨hun, hgen⟩ := followsTableRules_unpack htab
        (Array.mem_toList_iff.2 (Array.mem_of_getElem? hfd)) hcode
        (fh := info.handle) (by simp [CompiledModule.functionDefHandle, hfh]) hin
      have hdn : ((info.module.structDefs[d]?).bind info.module.structDefName) = some "Table" := by
        simp [hsd, hname]
      rcases hi with rfl | ⟨i, x, rfl, hx, rfl⟩
      · exact hun _ rfl hdn
      · exact hgen i x rfl hx hdn
  · simp at hres

end MoveBytecode.Proofs
