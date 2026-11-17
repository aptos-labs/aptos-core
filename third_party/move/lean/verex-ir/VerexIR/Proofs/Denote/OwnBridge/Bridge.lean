-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.OwnBridge.Operators

/-!
# A declaration's own invariant at the caller's frame

A unit installs each declaration's data invariant as a predicate of an
encoding at the base function values (`OwnAtArguments`), reading the clauses
at the frame the type arguments induce over the runtime frame. A caller holds
a native value of the declaration at its own frame, at an instance row.

The bridge states the installed predicate of the caller's value's encoding as
the declaration's clauses read at the caller's frame over the caller's value:

    installed (params.map Θ.resolve) (encode[Θ] x)  ↔  caller x

for a symbolic caller frame `Θ` and instance row `params`. Its one proof
obligation is the agreement of the two readings of the clauses across the move
(`Skolems.reframeAtInstance`). That agreement is not a consequence of the
equality of the encodings for an arbitrary predicate; it is proved per
declaration from the operator laws
(`VerexIR.Proofs.Denote.OwnBridge.Operators`) and the binder laws below, and a
clause outside them has no agreement and no bridge.

The equivalence serves both directions: its forward half opens a fact about a
value in hand, its backward half certifies a constructed or written value.
The agreement premise speaks of the clauses alone, so the backward half
assumes nothing about the validity of the value it certifies.
-/

namespace VerexIR.Proofs.Denote

section Bridge
variable {unit : Validation.ValidatedUnit} {functions base : FunctionCarriers unit}
  (sameCarrier : functions.carrier = base.carrier)
  (sameFunctions : HEq functions.functions base.functions)
  (sameNominals : functions.nominals = base.nominals)
  (Θ : @Skolems unit functions) (params : NRow)
  (inhabitable : (params.map Θ.resolve).inhabitable = true)

/-- The bridge over the installed predicate's own form. `holds` is the
declaration's clauses at the frame a row of type arguments induces at the base
function values, as the unit installs them; `caller` is the clauses read at
the caller's frame over a value at the instance row; `agree` is the agreement
of the two readings across the move, owed for the value at hand, so that a
declaration whose agreement needs a fact about the value states that fact as a
hypothesis of its own theorem. -/
theorem OwnAtArguments.bridge {σ : NTy}
    (bounded : σ.paramsAll (fun index => decide (index < params.length)) = true)
    (holds : (θ : TypeArgs) →
      @NTy.carrier (@Skolems.instantiate unit base θ (@Skolems.runtime unit base)).toCarriers σ →
        Prop)
    (caller : @NTy.carrier Θ.toCarriers (σ.subst params) → Prop)
    (value : @NTy.carrier Θ.toCarriers (σ.subst params))
    (agree :
      holds ⟨params.map Θ.resolve, inhabitable⟩
          (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
            bounded value) ↔
        caller value) :
    OwnAtArguments unit base σ holds (params.map Θ.resolve)
        (@NTy.encode Θ.toCarriers (σ.subst params) value) ↔
      caller value := by
  rw [← Skolems.encode_reframeAtInstance sameCarrier sameFunctions sameNominals Θ params
    inhabitable bounded value]
  exact (OwnAtArguments.encode unit base σ holds (params.map Θ.resolve) inhabitable _).trans
    agree

/-- The bridge over what a frame states of a declaration (`Carriers.own`):
where the unit's function values install the declaration's clauses as
`OwnAtArguments` at the base function values, the caller's frame states of the
encoding of its value exactly the clauses read at the caller's frame. -/
theorem Carriers.own_bridge {source : StructHandle} {σ : NTy}
    (bounded : σ.paramsAll (fun index => decide (index < params.length)) = true)
    (holds : (θ : TypeArgs) →
      @NTy.carrier (@Skolems.instantiate unit base θ (@Skolems.runtime unit base)).toCarriers σ →
        Prop)
    (installed : ∀ arguments value,
      @FunctionCarriers.own unit functions source arguments value =
        OwnAtArguments unit base σ holds arguments value)
    (caller : @NTy.carrier Θ.toCarriers (σ.subst params) → Prop)
    (value : @NTy.carrier Θ.toCarriers (σ.subst params))
    (agree :
      holds ⟨params.map Θ.resolve, inhabitable⟩
          (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
            bounded value) ↔
        caller value) :
    @Carriers.own Θ.toCarriers source params
        (@NTy.encode Θ.toCarriers (σ.subst params) value) ↔
      caller value := by
  rw [@Carriers.own_frame unit functions Θ source params, installed]
  exact OwnAtArguments.bridge sameCarrier sameFunctions sameNominals Θ params inhabitable bounded
    holds caller value agree

/-- The declaration's own clause reads the instance row as it reads the
declaration's parameters, which that row substitutes to themselves. -/
private theorem ownAtInstanceRow {source : StructHandle} {arguments : NRow}
    (ownRow : NRow.subst params arguments = params) : ∀ encoded : RuntimeValue,
    @Carriers.own Θ.toCarriers source (NRow.subst params arguments) encoded =
      @Carriers.own Θ.toCarriers source params encoded := fun encoded => by rw [ownRow]

/-- The bridge over a struct's fact: a value of a declared struct at an
instance row is valid exactly where the declaration's clauses read at the
caller's frame hold of it and its fields are valid. The declaration's own
arguments are its parameters, which the row substitutes to itself (`ownRow`);
the fields' validity stays the caller's own fact about the fields and is
neither used nor produced by the bridge. -/
theorem NTy.holds_struct_bridge {source : StructHandle} {arguments fields : NRow}
    (bounded : (NTy.struct source arguments fields).paramsAll
      (fun index => decide (index < params.length)) = true)
    (ownRow : NRow.subst params arguments = params)
    (holds : (θ : TypeArgs) →
      @NTy.carrier (@Skolems.instantiate unit base θ (@Skolems.runtime unit base)).toCarriers
        (.struct source arguments fields) → Prop)
    (installed : ∀ arguments' value,
      @FunctionCarriers.own unit functions source arguments' value =
        OwnAtArguments unit base (.struct source arguments fields) holds arguments' value)
    (caller : @NTy.carrier Θ.toCarriers
      (.struct source (NRow.subst params arguments) (NRow.subst params fields)) → Prop)
    (value : @NTy.carrier Θ.toCarriers
      (.struct source (NRow.subst params arguments) (NRow.subst params fields)))
    (agree :
      holds ⟨params.map Θ.resolve, inhabitable⟩
          (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
            bounded value) ↔
        caller value) :
    @NTy.Holds Θ.toCarriers
        (.struct source (NRow.subst params arguments) (NRow.subst params fields)) value ↔
      caller value ∧ @HList.Holds Θ.toCarriers (NRow.subst params fields) value := by
  rw [@NTy.holds_struct Θ.toCarriers, ownAtInstanceRow Θ params (source := source) ownRow]
  exact and_congr_left' (Carriers.own_bridge sameCarrier sameFunctions sameNominals Θ params
    inhabitable bounded holds installed caller value agree)

/-- The bridge over an enum's fact, as `NTy.holds_struct_bridge` is over a
struct's: a value of a declared enum at an instance row is valid exactly where
the declaration's clauses read at the caller's frame hold of it and the fields
of its variant are valid. -/
theorem NTy.holds_enum_bridge {source : StructHandle} {arguments : NRow} {names : List String}
    {rows : NRows} {distinct : names.Nodup}
    (bounded : (NTy.enum source arguments names rows distinct).paramsAll
      (fun index => decide (index < params.length)) = true)
    (ownRow : NRow.subst params arguments = params)
    (holds : (θ : TypeArgs) →
      @NTy.carrier (@Skolems.instantiate unit base θ (@Skolems.runtime unit base)).toCarriers
        (.enum source arguments names rows distinct) → Prop)
    (installed : ∀ arguments' value,
      @FunctionCarriers.own unit functions source arguments' value =
        OwnAtArguments unit base (.enum source arguments names rows distinct) holds arguments'
          value)
    (caller : @NTy.carrier Θ.toCarriers
      (.enum source (NRow.subst params arguments) names (NRows.subst params rows) distinct) → Prop)
    (value : @NTy.carrier Θ.toCarriers
      (.enum source (NRow.subst params arguments) names (NRows.subst params rows) distinct))
    (agree :
      holds ⟨params.map Θ.resolve, inhabitable⟩
          (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
            bounded value) ↔
        caller value) :
    @NTy.Holds Θ.toCarriers
        (.enum source (NRow.subst params arguments) names (NRows.subst params rows) distinct)
        value ↔
      caller value ∧ @variantCarrier.Holds Θ.toCarriers names (NRows.subst params rows) value := by
  rw [@NTy.holds_enum Θ.toCarriers, ownAtInstanceRow Θ params (source := source) ownRow]
  exact and_congr_left' (Carriers.own_bridge sameCarrier sameFunctions sameNominals Θ params
    inhabitable bounded holds installed caller value agree)

/-! ## Binders

A binder over a domain no frame reads (an integer index, a boolean) is the
same binder on both sides. A binder over a carrier ranges, at the
declaration's frame, over exactly the moved values of the caller's carrier
(`Skolems.reframeAtInstance_surjective`), and a binder over the elements of a
vector over exactly the moved elements. A binder restricted to the valid
values of a carrier needs more: that the move preserves and reflects validity
at that type, which is a premise of the law, since the declaration's frame
reads validity at the base function values. -/

/-- A universal clause over a carrier at the declaration's frame is the
universal clause over the caller's carrier, where the bodies agree across the
move. -/
theorem Skolems.forall_reframeAtInstance {τ : NTy}
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    {body : @NTy.carrier
      (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers τ → Prop}
    {body' : @NTy.carrier Θ.toCarriers (τ.subst params) → Prop}
    (agree : ∀ value,
      body (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        bounded value) ↔ body' value) :
    (∀ target, body target) ↔ ∀ value, body' value := by
  constructor
  · intro all value
    exact (agree value).mp (all _)
  · intro all target
    obtain ⟨value, moved⟩ := Skolems.reframeAtInstance_surjective sameCarrier sameFunctions
      sameNominals Θ params inhabitable bounded target
    exact moved ▸ (agree value).mpr (all value)

/-- An existential clause over a carrier, likewise. -/
theorem Skolems.exists_reframeAtInstance {τ : NTy}
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    {body : @NTy.carrier
      (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers τ → Prop}
    {body' : @NTy.carrier Θ.toCarriers (τ.subst params) → Prop}
    (agree : ∀ value,
      body (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        bounded value) ↔ body' value) :
    (∃ target, body target) ↔ ∃ value, body' value := by
  constructor
  · rintro ⟨target, held⟩
    obtain ⟨value, moved⟩ := Skolems.reframeAtInstance_surjective sameCarrier sameFunctions
      sameNominals Θ params inhabitable bounded target
    exact ⟨value, (agree value).mp (moved ▸ held)⟩
  · rintro ⟨value, held⟩
    exact ⟨_, (agree value).mpr held⟩

/-- A universal clause over the elements of a vector at the declaration's
frame is the universal clause over the elements of the caller's vector, where
the bodies agree across the move for the caller's elements. -/
theorem Skolems.forall_mem_reframeAtInstance {element : NTy}
    (bounded : (NTy.vector element).paramsAll
      (fun index => decide (index < params.length)) = true)
    (vector : @NTy.carrier Θ.toCarriers (.vector (element.subst params)))
    {body : @NTy.carrier
      (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers element → Prop}
    {body' : @NTy.carrier Θ.toCarriers (element.subst params) → Prop}
    (agree : ∀ value, value ∈ vector.values →
      (body (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (τ := element) (by simpa only [NTy.paramsAll] using bounded) value) ↔ body' value)) :
    (∀ target,
        target ∈ (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params
          inhabitable (τ := .vector element) bounded vector).values → body target) ↔
      ∀ value, value ∈ vector.values → body' value := by
  rw [Skolems.reframeAtInstance_vector_values]
  constructor
  · intro all value member
    exact (agree value member).mp (all _ (Array.mem_map.mpr ⟨value, member, rfl⟩))
  · intro all target member
    obtain ⟨value, member, moved⟩ := Array.mem_map.mp member
    exact moved ▸ (agree value member).mpr (all value member)

/-- An existential clause over the elements of a vector, likewise. -/
theorem Skolems.exists_mem_reframeAtInstance {element : NTy}
    (bounded : (NTy.vector element).paramsAll
      (fun index => decide (index < params.length)) = true)
    (vector : @NTy.carrier Θ.toCarriers (.vector (element.subst params)))
    {body : @NTy.carrier
      (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers element → Prop}
    {body' : @NTy.carrier Θ.toCarriers (element.subst params) → Prop}
    (agree : ∀ value, value ∈ vector.values →
      (body (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (τ := element) (by simpa only [NTy.paramsAll] using bounded) value) ↔ body' value)) :
    (∃ target,
        target ∈ (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params
          inhabitable (τ := .vector element) bounded vector).values ∧ body target) ↔
      ∃ value, value ∈ vector.values ∧ body' value := by
  rw [Skolems.reframeAtInstance_vector_values]
  constructor
  · rintro ⟨target, member, held⟩
    obtain ⟨value, member, moved⟩ := Array.mem_map.mp member
    exact ⟨value, member, (agree value member).mp (moved ▸ held)⟩
  · rintro ⟨value, member, held⟩
    exact ⟨_, Array.mem_map.mpr ⟨value, member, rfl⟩, (agree value member).mpr held⟩

/-- A universal clause over the valid values of a carrier is the caller's,
where the move preserves and reflects validity at the carrier's type
(`valid`) and the bodies agree. The premise is stated, not assumed: at a type
none of whose declarations states an invariant the two frames read validity
alike; at a type that mentions one, the declaration's frame reads that
declaration's invariant at the base function values and only preservation
holds (`Skolems.forall_holds_reframeAtInstance_mp`). -/
theorem Skolems.forall_holds_reframeAtInstance {τ : NTy}
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    {body : @NTy.carrier
      (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers τ → Prop}
    {body' : @NTy.carrier Θ.toCarriers (τ.subst params) → Prop}
    (valid : ∀ value,
      @NTy.Holds (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers τ
          (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
            bounded value) ↔
        @NTy.Holds Θ.toCarriers (τ.subst params) value)
    (agree : ∀ value,
      body (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        bounded value) ↔ body' value) :
    (∀ target,
        @NTy.Holds (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers τ
          target → body target) ↔
      ∀ value, @NTy.Holds Θ.toCarriers (τ.subst params) value → body' value :=
  Skolems.forall_reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
    bounded fun value => imp_congr (valid value) (agree value)

/-- The opening half of a validity-restricted universal clause needs only that
the move preserves validity: a caller's valid value is valid at the
declaration's frame. The certifying half is not provable from preservation. -/
theorem Skolems.forall_holds_reframeAtInstance_mp {τ : NTy}
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    {body : @NTy.carrier
      (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers τ → Prop}
    {body' : @NTy.carrier Θ.toCarriers (τ.subst params) → Prop}
    (preserved : ∀ value, @NTy.Holds Θ.toCarriers (τ.subst params) value →
      @NTy.Holds (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers τ
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
          bounded value))
    (agree : ∀ value,
      body (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        bounded value) → body' value)
    (all : ∀ target,
      @NTy.Holds (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers τ
        target → body target) :
    ∀ value, @NTy.Holds Θ.toCarriers (τ.subst params) value → body' value :=
  fun value held => agree value (all _ (preserved value held))

end Bridge

/-- An obligation's marker does not read its proposition: two readings of a
clause agree under the same marker where they agree. -/
theorem _root_.VerexIR.Proofs.Obligation.congr {file : String} {startByte endByte : Nat}
    {p q : Prop} (same : p ↔ q) :
    VerexIR.Proofs.Obligation file startByte endByte p ↔
      VerexIR.Proofs.Obligation file startByte endByte q :=
  VerexIR.Proofs.Obligation_iff.trans (same.trans VerexIR.Proofs.Obligation_iff.symm)

end VerexIR.Proofs.Denote
