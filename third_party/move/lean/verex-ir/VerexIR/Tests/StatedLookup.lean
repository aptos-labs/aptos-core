-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
A hypothesis lookup is not a computation. `verex_assumption` finds the
hypothesis stating the goal up to reducible unfolding, source-location
markers and an equation's orientation. A premise the context states in
another spelling is brought to that spelling by its own named operation
first: a method premise at an instantiated frame by the transport to the
outer frame, whose substituted rows are the stated ones up to reducible
unfolding; a closure's shape at the closure's own function and mask.
-/

namespace VerexIR.Tests.StatedLookup

def doubled (count : Nat) : Nat := count + count

opaque Marked : Nat → Prop

def MarkedTwice (count : Nat) : Prop := Marked (doubled count)

abbrev MarkedFour : Prop := Marked 4

-- The same proposition, up to reducible unfolding.
example (stated : Marked 4) : MarkedFour := by verex_assumption

-- Under a source-location marker, on either side.
example (stated : VerexIR.Proofs.Obligation "source" 0 0 (Marked 4)) : Marked 4 := by
  verex_assumption

-- An equation stated with its sides exchanged.
example (left right : Nat) (stated : left = right) : right = left := by verex_assumption

-- A hypothesis that only evaluation makes the goal is not looked up.
example (stated : Marked (doubled 2)) : True := by
  fail_if_success (have : Marked 4 := by verex_assumption)
  trivial

section Frames

open VerexIR.Proofs VerexIR.Proofs.Denote

variable {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit] [Θ : Skolems unit]

-- A method premise at the frame instantiated by the caller's own type
-- arguments is the premise the context states at the outer frame.
example
    (function : @NTy.carrier ((Skolems.instantiate
        ⟨.cons (.param 0) (.cons (.param 1) .nil), rfl⟩ Θ).toCarriers).toSorts
      (.function (.cons (.param 0) (.cons (.int 64 false) .nil))
        { parameters := [true, false], results := [false] } (ResultShape.one (.param 1)).row))
    (stated : ReceiverScopedMethod (.cons (.param 0) (.cons (.int 64 false) .nil))
      { parameters := [true, false], results := [false] } (.one (.param 1))
      ((ResultShape.one (.param 1)).functionOfSkolem ⟨.cons (.param 0) (.cons (.param 1) .nil), rfl⟩
        (.cons (.param 0) (.cons (.int 64 false) .nil))
        { parameters := [true, false], results := [false] } function)) :
    @ReceiverScopedMethod unit _
      (Skolems.instantiate ⟨.cons (.param 0) (.cons (.param 1) .nil), rfl⟩ Θ)
      (.cons (.param 0) (.cons (.int 64 false) .nil))
      { parameters := [true, false], results := [false] } (.one (.param 1)) function := by
  apply ReceiverScopedMethod.of_instantiate _
  simp only [vir_denote_norm]
  verex_assumption

-- At exchanged type arguments the instantiated rows are other rows: the
-- premise at the outer frame is not the one owed.
example
    (function : @NTy.carrier ((Skolems.instantiate
        ⟨.cons (.param 1) (.cons (.param 0) .nil), rfl⟩ Θ).toCarriers).toSorts
      (.function (.cons (.param 0) (.cons (.int 64 false) .nil))
        { parameters := [true, false], results := [false] } (ResultShape.one (.param 1)).row))
    (other : (NTy.function (.cons (.param 0) (.cons (.int 64 false) .nil))
      { parameters := [true, false], results := [false] } (ResultShape.one (.param 1)).row).carrier)
    (stated : ReceiverScopedMethod (.cons (.param 0) (.cons (.int 64 false) .nil))
      { parameters := [true, false], results := [false] } (.one (.param 1)) other) : True := by
  fail_if_success
    have : @ReceiverScopedMethod unit _
        (Skolems.instantiate ⟨.cons (.param 1) (.cons (.param 0) .nil), rfl⟩ Θ)
        (.cons (.param 0) (.cons (.int 64 false) .nil))
        { parameters := [true, false], results := [false] } (.one (.param 1)) function := by
      apply ReceiverScopedMethod.of_instantiate _
      simp only [vir_denote_norm]
      verex_assumption
  trivial

-- A closure's shape is looked up at that closure's own function and mask:
-- the shape of a closure with the same function and mask and other captures
-- is another fact.
example (handle : VerexIR.FunctionHandle) (instantiation : Array (VerexIR.TypeId × VerexIR.TypeId))
    {σs : NRow} (captures others : HList σs)
    (stated : receiverScopedShape unit (closureOf handle 0 instantiation others).function
      (closureOf handle 0 instantiation others).mask
      (closureOf handle 0 instantiation others).typeInstantiation
      (closureOf handle 0 instantiation others).captures = true) : True := by
  fail_if_success
    have : receiverScopedShape unit (closureOf handle 0 instantiation captures).function
        (closureOf handle 0 instantiation captures).mask
        (closureOf handle 0 instantiation captures).typeInstantiation
        (closureOf handle 0 instantiation captures).captures = true := by
      verex_assumption
  trivial

end Frames

end VerexIR.Tests.StatedLookup
