-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Types

/-!
# The arguments a declaration's inhabitant demands

Whether a type has values (`NTy.inhabitable`) reads a type parameter in some
positions only: not under a vector, a slice or a table, whose empty value
needs none, not in a variant of an enum after the first, not at an argument a
declared type's kind does not demand. `NTy.demands` lists the parameters it
does read, and the test of a substituted type is the test of the type and of
the demanded parameters' types (`NTy.inhabitable_substWith_eq`).

So a declared type whose kind states its declaration's test and demands has
values exactly where the structural type of the declaration at the same
arguments has (`NTy.inhabitable_nominal_struct`,
`NTy.inhabitable_nominal_enum`), at every argument row: at a function type,
which has no value of its own, too. The kinds there are written out, with the
view and the cleanliness of their values arbitrary, because the test reads
neither: this is the independence the demand rule needs, not a choice of what
a declaration's view or cleanliness is.

A kind that demands every position of its argument row tests the whole row
(`NTy.inhabitable_nominal_all`), the rule a declared type carried before it
stated its demands.
-/

namespace VerexIR.Proofs.Denote

mutual
/-- The type parameters whose types' values a type's values need. -/
def NTy.demands : NTy → List Nat
  | .param index => [index]
  | .tuple elements => elements.demands
  | .struct _ _ fields => fields.demands
  | .enum _ _ names rows _ => NRows.firstDemands names rows
  | .ref referent => referent.demands
  | .ghost slot => slot.demands
  | .observed referent => referent.demands
  | .array element _ => element.demands
  | .nominal _ arguments kind => arguments.demandsAt kind.demands 0
  | .function _ _ _ | .int _ _ | .unit | .bool | .num | .address | .signer | .string | .bytes
  | .char | .slice _ | .vector _ | .table _ _ | .sliceRef _ => []

def NRow.demands : NRow → List Nat
  | .nil => []
  | .cons τ rest => τ.demands ++ rest.demands

/-- The parameters the entries of a row at the demanded positions need,
counting positions from `index`. -/
def NRow.demandsAt (demanded : List Nat) : Nat → NRow → List Nat
  | _, .nil => []
  | index, .cons τ rest =>
      (bif demanded.contains index then τ.demands else []) ++ rest.demandsAt demanded (index + 1)

def NRows.firstDemands : List String → NRows → List Nat
  | _ :: _, .cons fields _ => fields.demands
  | _, _ => []
end

private theorem and_and_and (a b c d : Bool) : ((a && b) && (c && d)) = ((a && c) && (b && d)) := by
  cases a <;> cases b <;> cases c <;> cases d <;> rfl

section Substituted
variable (types : Nat → NTy)

mutual
/-- A substituted type has values where the type has and the types of the
parameters it demands have. -/
theorem NTy.inhabitable_substWith_eq : (τ : NTy) →
    (τ.substWith types).inhabitable =
      (τ.inhabitable && τ.demands.all fun index => (types index).inhabitable)
  | .param index => by
      simp [NTy.substWith, NTy.inhabitable, NTy.demands]
  | .tuple elements => by
      simpa [NTy.substWith, NTy.inhabitable, NTy.demands] using
        NRow.inhabitable_substWith_eq elements
  | .struct _ _ fields => by
      simpa [NTy.substWith, NTy.inhabitable, NTy.demands] using
        NRow.inhabitable_substWith_eq fields
  | .enum _ _ names rows _ => by
      simpa [NTy.substWith, NTy.inhabitable, NTy.demands] using
        NRows.firstInhabitable_substWith_eq names rows
  | .ref referent => by
      simpa [NTy.substWith, NTy.inhabitable, NTy.demands] using
        NTy.inhabitable_substWith_eq referent
  | .ghost slot => by
      simpa [NTy.substWith, NTy.inhabitable, NTy.demands] using
        NTy.inhabitable_substWith_eq slot
  | .observed referent => by
      simpa [NTy.substWith, NTy.inhabitable, NTy.demands] using
        NTy.inhabitable_substWith_eq referent
  | .array element length => by
      simp [NTy.substWith, NTy.inhabitable, NTy.demands, NTy.inhabitable_substWith_eq element,
        Bool.and_assoc]
  | .nominal _ arguments kind => by
      simp [NTy.substWith, NTy.inhabitable, NTy.demands,
        NRow.inhabitableAt_substWith_eq kind.demands arguments 0, Bool.and_assoc]
  | .function _ _ _ | .int _ _ | .unit | .bool | .num | .address | .signer | .string | .bytes
  | .char | .slice _ | .vector _ | .table _ _ | .sliceRef _ => by
      simp [NTy.substWith, NTy.inhabitable, NTy.demands]

theorem NRow.inhabitable_substWith_eq : (row : NRow) →
    (row.substWith types).inhabitable =
      (row.inhabitable && row.demands.all fun index => (types index).inhabitable)
  | .nil => rfl
  | .cons τ rest => by
      simp only [NRow.substWith, NRow.inhabitable, NRow.demands, List.all_append,
        NTy.inhabitable_substWith_eq τ, NRow.inhabitable_substWith_eq rest]
      exact and_and_and _ _ _ _

theorem NRow.inhabitableAt_substWith_eq (demanded : List Nat) : (row : NRow) → (index : Nat) →
    (row.substWith types).inhabitableAt demanded index =
      (row.inhabitableAt demanded index &&
        (row.demandsAt demanded index).all fun index => (types index).inhabitable)
  | .nil, _ => rfl
  | .cons τ rest, index => by
      simp only [NRow.substWith, NRow.inhabitableAt, NRow.demandsAt, List.all_append,
        NTy.inhabitable_substWith_eq τ, NRow.inhabitableAt_substWith_eq demanded rest (index + 1)]
      cases demanded.contains index
      · simp
      · simp only [Bool.not_true, Bool.false_or, cond_true]
        exact and_and_and _ _ _ _

theorem NRows.firstInhabitable_substWith_eq : (names : List String) → (rows : NRows) →
    NRows.firstInhabitable names (rows.substWith types) =
      (NRows.firstInhabitable names rows &&
        (NRows.firstDemands names rows).all fun index => (types index).inhabitable)
  | _ :: _, .cons fields _ => NRow.inhabitable_substWith_eq fields
  | [], .nil => rfl
  | [], .cons _ _ => rfl
  | _ :: _, .nil => rfl
end

end Substituted

/-- The demanded positions of a row have values where the row's entries at
the demanded positions have, a position beyond the row being the unit type's. -/
theorem NRow.inhabitableAt_eq_all (demanded : List Nat) : (row : NRow) → (start : Nat) →
    row.inhabitableAt demanded start =
      demanded.all fun index =>
        decide (index < start) || (row.getD (index - start) .unit).inhabitable
  | .nil, start => by
      simp [NRow.inhabitableAt, NTy.inhabitable]
  | .cons τ rest, start => by
      rw [NRow.inhabitableAt, NRow.inhabitableAt_eq_all demanded rest (start + 1), Bool.eq_iff_iff]
      simp only [Bool.and_eq_true, Bool.or_eq_true, Bool.not_eq_true', List.all_eq_true,
        decide_eq_true_eq, List.contains_eq_mem, decide_eq_false_iff_not]
      constructor
      · rintro ⟨head, tail⟩ index member
        rcases Nat.lt_trichotomy index start with below | same | above
        · exact .inl below
        · subst same
          refine .inr ?_
          rw [Nat.sub_self]
          exact head.resolve_left fun absent => absent member
        · refine .inr ?_
          have step : index - start = (index - (start + 1)) + 1 := by omega
          rw [step]
          exact (tail index member).resolve_left (by omega)
      · intro all
        refine ⟨?_, fun index member => ?_⟩
        · by_cases member : start ∈ demanded
          · have entry := (all start member).resolve_left (Nat.lt_irrefl _)
            rw [Nat.sub_self] at entry
            exact .inr entry
          · exact .inl member
        · by_cases below : index < start + 1
          · exact .inl below
          · have entry := (all index member).resolve_left (by omega)
            have step : index - start = (index - (start + 1)) + 1 := by omega
            rw [step] at entry
            exact .inr entry

/-- A declared type at the kind of a struct declaration — the row's test, the
parameters it demands, and any view and cleanliness — has values exactly where
the declaration's structural type at the same arguments has. -/
theorem NTy.inhabitable_nominal_struct (source : StructHandle) (arguments fields : NRow)
    (view : View) (clean : Bool) :
    (NTy.nominal source arguments ⟨fields.inhabitable, fields.demands, view, clean⟩).inhabitable =
      (NTy.struct source arguments
        (fields.substWith (NominalBasis.slot arguments))).inhabitable := by
  show (fields.inhabitable && arguments.inhabitableAt fields.demands 0) =
    (fields.substWith (NominalBasis.slot arguments)).inhabitable
  rw [NRow.inhabitable_substWith_eq, NRow.inhabitableAt_eq_all]
  simp

/-- The same for an enum declaration, whose test reads its first variant. -/
theorem NTy.inhabitable_nominal_enum (source : StructHandle) (arguments : NRow)
    (names : List String) (rows : NRows) (distinct : names.Nodup)
    (view : View) (clean : Bool) :
    (NTy.nominal source arguments
        ⟨NRows.firstInhabitable names rows, NRows.firstDemands names rows, view, clean⟩).inhabitable =
      (NTy.enum source arguments names (rows.substWith (NominalBasis.slot arguments))
        distinct).inhabitable := by
  show (NRows.firstInhabitable names rows &&
      arguments.inhabitableAt (NRows.firstDemands names rows) 0) =
    NRows.firstInhabitable names (rows.substWith (NominalBasis.slot arguments))
  rw [NRows.firstInhabitable_substWith_eq, NRow.inhabitableAt_eq_all]
  simp

/-- A row all of whose positions are demanded has values where the row has. -/
theorem NRow.inhabitableAt_range : (row : NRow) → (start : Nat) →
    row.inhabitableAt (List.range (start + row.length)) start = row.inhabitable
  | .nil, _ => rfl
  | .cons τ rest, start => by
      have step : start + (NRow.cons τ rest).length = (start + 1) + rest.length := by
        simp [NRow.length]
        omega
      rw [NRow.inhabitableAt, step, NRow.inhabitableAt_range rest (start + 1), NRow.inhabitable]
      have inside : (List.range ((start + 1) + rest.length)).contains start = true := by
        simp [List.contains_eq_mem, List.mem_range]
        omega
      rw [inside]
      simp

theorem NRow.inhabitableAt_all (row : NRow) :
    row.inhabitableAt (List.range row.length) 0 = row.inhabitable := by
  simpa using NRow.inhabitableAt_range row 0

/-- A declared type whose kind demands every position of its argument row has
values where it is productive and the whole row has: the rule a declared type
carried before its kind stated which arguments it demands. -/
theorem NTy.inhabitable_nominal_all (source : StructHandle) (arguments : NRow)
    (productive : Bool) (view : View) (clean : Bool) :
    (NTy.nominal source arguments
        ⟨productive, List.range arguments.length, view, clean⟩).inhabitable =
      (productive && arguments.inhabitable) := by
  show (productive && arguments.inhabitableAt (List.range arguments.length) 0) = _
  rw [NRow.inhabitableAt_all]

/-! ## Controls -/

section Controls

/-- A type without a value of its own: a function type. -/
private def closure : NTy := .function .nil ⟨[], []⟩ .nil

private def handle : StructHandle := ⟨⟨0⟩, 30⟩
private def inner : StructHandle := ⟨⟨0⟩, 31⟩

private def row (τ : NTy) : NRow := .cons τ .nil

example : closure.inhabitable = false := rfl

/-- A phantom parameter is not demanded: the declared type has values at a
function type, as the structural type has. -/
example : (row .bool).inhabitable = true := rfl
example : (row .bool).demands = [] := rfl
example (view : View) (clean : Bool) :
    (NTy.nominal handle (row closure) ⟨true, [], view, clean⟩).inhabitable = true := rfl
example : (NTy.struct handle (row closure) (row .bool)).inhabitable = true := rfl

/-- A field of the parameter demands it: the declared type has no values at a
function-typed argument it demands, and has them at one it does not. -/
example : (row (.param 0)).demands = [0] := rfl
example (view : View) (clean : Bool) :
    (NTy.nominal handle (row closure) ⟨true, [0], view, clean⟩).inhabitable = false := rfl
example (view : View) (clean : Bool) :
    (NTy.nominal handle (row .bool) ⟨true, [0], view, clean⟩).inhabitable = true := rfl

/-- A vector, a slice and a table of the parameter have the empty value. -/
example : (row (.vector (.param 0))).demands = [] := rfl
example : (row (.slice (.param 0))).demands = [] := rfl
example : (row (.table (.param 0) (.param 0))).demands = [] := rfl
example (view : View) (clean : Bool) :
    (NTy.nominal handle (row closure) ⟨true, [], view, clean⟩).inhabitable = true := rfl
example : (NTy.struct handle (row closure) (row (.vector closure))).inhabitable = true := rfl

/-- A fixed array demands its element, and has no value beyond the length
bound: such a declaration is not productive. -/
example : (row (.array (.param 0) 2)).demands = [0] := rfl
example : (row (.array (.param 0) 2)).inhabitable = true := rfl
example : (row (.array (.param 0) (2 ^ 64))).inhabitable = false := by decide

/-- An enum reads its first variant only. -/
example : NRows.firstInhabitable ["A", "B"] (.cons .nil (.cons (row (.param 0)) .nil)) = true := rfl
example : NRows.firstDemands ["A", "B"] (.cons .nil (.cons (row (.param 0)) .nil)) = [] := rfl
example : NRows.firstDemands ["A", "B"] (.cons (row (.param 0)) (.cons .nil .nil)) = [0] := rfl
example (view : View) (clean : Bool) : (NTy.nominal handle (row closure)
    ⟨NRows.firstInhabitable ["A", "B"] (.cons .nil (.cons (row (.param 0)) .nil)),
      NRows.firstDemands ["A", "B"] (.cons .nil (.cons (row (.param 0)) .nil)),
      view, clean⟩).inhabitable = true := rfl
example (view : View) (clean : Bool) : (NTy.nominal handle (row closure)
    ⟨NRows.firstInhabitable ["A", "B"] (.cons (row (.param 0)) (.cons .nil .nil)),
      NRows.firstDemands ["A", "B"] (.cons (row (.param 0)) (.cons .nil .nil)),
      view, clean⟩).inhabitable = false := rfl

/-- A nested generic demands what its own kind demands of the written
argument, whatever its view and cleanliness. -/
example (view : View) (clean : Bool) :
    (row (.nominal inner (row (.param 0)) ⟨true, [], view, clean⟩)).demands = [] := rfl
example (view : View) (clean : Bool) :
    (row (.nominal inner (row (.param 0)) ⟨true, [0], view, clean⟩)).demands = [0] := rfl

/-- A declaration without values has none at any argument. -/
example (arguments : NRow) (demands : List Nat) (view : View) (clean : Bool) :
    (NTy.nominal handle arguments ⟨false, demands, view, clean⟩).inhabitable = false := rfl

/-- A function-typed field has no value: the declaration is not productive. -/
example : (row closure).inhabitable = false := rfl
example (arguments : NRow) (view : View) (clean : Bool) :
    (NTy.nominal handle arguments
      ⟨(row closure).inhabitable, (row closure).demands, view, clean⟩).inhabitable = false := rfl

/-- A demanded position beyond the argument row reads the unit type, which has
values. -/
example (view : View) (clean : Bool) :
    (NTy.nominal handle .nil ⟨true, [3], view, clean⟩).inhabitable = true := rfl
example (view : View) (clean : Bool) :
    (NTy.nominal handle (row closure) ⟨true, [1], view, clean⟩).inhabitable = true := rfl

/-- The all-demanded kind tests every argument. -/
example (view : View) (clean : Bool) :
    (NTy.nominal handle (row closure) ⟨true, List.range (row closure).length, view,
      clean⟩).inhabitable = false := rfl
example (view : View) (clean : Bool) :
    (NTy.nominal handle (row .bool) ⟨true, List.range (row .bool).length, view,
      clean⟩).inhabitable = true := rfl

end Controls

end VerexIR.Proofs.Denote
