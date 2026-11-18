-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.Rendered.Support

/-!
# `split_pair_frame` as a frontend renders it

The module below is the rendering of
`VerexE2ETests/SourceVerify/split_pair_frame.move` that the
source-verification driver writes to `.verex/split_pair_frame.move.lean`, its
`verex sources at` command dropped; the diagnostics are those of
`split_pair_frame.exp`. Neither is edited here:
`scripts/prover-smoke-renderings.sh --write` writes this file again, and
without `--write` tells whether it is what the frontend renders.
-/

/--
error: verification failed with a residual obligation:
case right.right.right.left.isTrue
registry✝ : VerexIR.Validation.SemanticsRegistry
executable✝ : VerexIR.Validation.ExecutableUnit unit
prepared✝ : VerexIR.Validation.prepareExecution registry✝ unit = Except.ok executable✝
nativesTyped : VerexIR.NativesTyped executable✝
nativesShift : VerexIR.NativesShift executable✝
runsAgree : VerexIR.Proofs.RunsAgree executable✝
ownersRepresentable : VerexIR.Proofs.OwnersRepresentable unit
verexAgree : VerexIR.Proofs.TypesAgree unit
verexResources : VerexIR.Proofs.ResourcesTyped unit
s : VerexIR.Proofs.Denote.NTy.signer.carrier
input : 0x43.split_pair_frame.unit.nominal.namespace.0.declaration✝⁻⁰.carrier
initialState✝ : VerexIR.Proofs.Denote.Memory unit
permitted : VerexIR.Proofs.Denote.DataInvariants (storedInvariant unit executable✝) initialState✝
val✝ : 0x43.split_pair_frame.unit.nominal.namespace.0.declaration✝¹⁻⁰.carrier
verexContinuation : VerexIR.Proofs.Denote.Flow VerexIR.Proofs.Denote.ResultShape.none
    (VerexIR.Proofs.Denote.NRow.cons VerexIR.Proofs.Denote.NTy.signer
      (VerexIR.Proofs.Denote.NRow.cons 0x43.split_pair_frame.unit.nominal.namespace.0.declaration✝⁻⁰
        (VerexIR.Proofs.Denote.NRow.cons VerexIR.Proofs.Denote.NTy.address
          (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false)
            (VerexIR.Proofs.Denote.NRow.cons
              (VerexIR.Proofs.Denote.NTy.function
                (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false)
                  VerexIR.Proofs.Denote.NRow.nil)
                { parameters := [false], results := [false] }
                (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false)
                  VerexIR.Proofs.Denote.NRow.nil))
              (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false)
                (VerexIR.Proofs.Denote.NRow.cons VerexIR.Proofs.Denote.NTy.address
                  (VerexIR.Proofs.Denote.NRow.cons VerexIR.Proofs.Denote.NTy.address
                    VerexIR.Proofs.Denote.NRow.nil))))))))
    VerexIR.Proofs.Denote.NTy.unit.carrier →
  VerexIR.Proofs.Denote.Memory unit → Prop :=
  attempt_1._verexContinuation_1_8 s input initialState✝
entry✝² : Option VerexIR.Proofs.Denote.NTy.signer.carrier
entry✝¹ : Option 0x43.split_pair_frame.unit.nominal.namespace.0.declaration✝⁻⁰.carrier
entry✝ : Option (VerexIR.Proofs.Denote.NTy.int 64 false).carrier
entry : Option VerexIR.Proofs.Denote.NTy.address.carrier
x✝¹ : VerexIR.Proofs.Denote.Slot.Holds 0x43.split_pair_frame.unit.nominal.namespace.0.declaration✝⁻⁰ entry✝¹
w✝²⁶ : Option (VerexIR.Proofs.Denote.NTy.int 64 false).carrier
w✝²⁵ : Option VerexIR.Proofs.Denote.NTy.address.carrier
w✝²⁴ : Option VerexIR.Proofs.Denote.NTy.signer.carrier
w✝²³ : Option 0x43.split_pair_frame.unit.nominal.namespace.0.declaration✝⁻⁰.carrier
w✝²² : Option VerexIR.Proofs.Denote.NTy.address.carrier
w✝²¹ : Option (VerexIR.Proofs.Denote.NTy.int 64 false).carrier
w✝²⁰ :
  Option
    (VerexIR.Proofs.Denote.NTy.function
        (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false) VerexIR.Proofs.Denote.NRow.nil)
        { parameters := [false], results := [false] }
        (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false)
          VerexIR.Proofs.Denote.NRow.nil)).carrier
w✝¹⁹ w✝¹⁸ : Option VerexIR.Proofs.Denote.NTy.address.carrier
w✝¹⁷ :
  (VerexIR.Proofs.Denote.NTy.function
      (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false) VerexIR.Proofs.Denote.NRow.nil)
      { parameters := [false], results := [false] }
      (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false) VerexIR.Proofs.Denote.NRow.nil)).carrier
w✝¹⁶ : (VerexIR.Proofs.Denote.NTy.int 64 false).carrier
w✝¹⁵ : VerexIR.Proofs.Denote.NTy.signer.carrier
w✝¹⁴ :
  (VerexIR.Proofs.Denote.NTy.function
      (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false) VerexIR.Proofs.Denote.NRow.nil)
      { parameters := [false], results := [false] }
      (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false) VerexIR.Proofs.Denote.NRow.nil)).carrier
w✝¹³ : (VerexIR.Proofs.Denote.NTy.int 64 false).carrier
w✝¹² : 0x43.split_pair_frame.unit.nominal.namespace.0.declaration✝⁻⁰.Holds (Sum.inl (w✝¹³, ()))
w✝¹¹ :
  VerexIR.Proofs.Denote.Skolems.plainAt 0x43.split_pair_frame.unit.nominal.namespace.0.declaration✝¹⁻⁰
      VerexIR.Proofs.Denote.NRow.nil (VerexIR.StorageKey.address w✝¹⁵) initialState✝ =
    some (Sum.inr (Sum.inr (Sum.inl (w✝¹⁴, ()))))
w✝¹⁰ : 0x43.split_pair_frame.unit.nominal.namespace.0.declaration✝¹⁻⁰.Holds (Sum.inr (Sum.inr (Sum.inl (w✝¹⁴, ()))))
w✝⁹ :
  VerexIR.Proofs.KeepsMemoryAt executable✝
    (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false) VerexIR.Proofs.Denote.NRow.nil)
    { parameters := [false], results := [false] }
    (VerexIR.Proofs.Denote.ResultShape.one (VerexIR.Proofs.Denote.NTy.int 64 false)) w✝¹⁴
w✝⁸ : (VerexIR.Proofs.Denote.ResultShape.one (VerexIR.Proofs.Denote.NTy.int 64 false)).carrier
w✝⁷ :
  ¬VerexIR.Proofs.AbortsOfEntries (VerexIR.Proofs.Denote.closureMeaning executable✝)
      (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false) VerexIR.Proofs.Denote.NRow.nil)
      { parameters := [false], results := [false] }
      (VerexIR.Proofs.Denote.ResultShape.one (VerexIR.Proofs.Denote.NTy.int 64 false)) w✝¹⁴ (w✝¹³, ())
      (VerexIR.Proofs.Denote.Skolems.writePlain 0x43.split_pair_frame.unit.nominal.namespace.0.declaration✝¹⁻⁰
        VerexIR.Proofs.Denote.NRow.nil (VerexIR.StorageKey.address w✝¹⁵) none initialState✝)
w✝⁶ :
  ∀ (default : (VerexIR.Proofs.Denote.ResultShape.one (VerexIR.Proofs.Denote.NTy.int 64 false)).carrier),
    VerexIR.Proofs.ResultOf (VerexIR.Proofs.Denote.closureMeaning executable✝)
        (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false) VerexIR.Proofs.Denote.NRow.nil)
        { parameters := [false], results := [false] }
        (VerexIR.Proofs.Denote.ResultShape.one (VerexIR.Proofs.Denote.NTy.int 64 false)) w✝¹⁴ (w✝¹³, ())
        (VerexIR.Proofs.Denote.Skolems.writePlain 0x43.split_pair_frame.unit.nominal.namespace.0.declaration✝¹⁻⁰
          VerexIR.Proofs.Denote.NRow.nil (VerexIR.StorageKey.address w✝¹⁵) none initialState✝)
        default =
      w✝⁸
w✝⁵ :
  VerexIR.Proofs.EnsuresOf (VerexIR.Proofs.Denote.closureMeaning executable✝)
    (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false) VerexIR.Proofs.Denote.NRow.nil)
    { parameters := [false], results := [false] }
    (VerexIR.Proofs.Denote.ResultShape.one (VerexIR.Proofs.Denote.NTy.int 64 false)) w✝¹⁴ (w✝¹³, ()) w✝⁸
    (VerexIR.Proofs.Denote.Skolems.writePlain 0x43.split_pair_frame.unit.nominal.namespace.0.declaration✝¹⁻⁰
      VerexIR.Proofs.Denote.NRow.nil (VerexIR.StorageKey.address w✝¹⁵) none initialState✝)
    (VerexIR.Proofs.Denote.Skolems.writePlain 0x43.split_pair_frame.unit.nominal.namespace.0.declaration✝¹⁻⁰
      VerexIR.Proofs.Denote.NRow.nil (VerexIR.StorageKey.address w✝¹⁵) none initialState✝)
w✝⁴ :
  VerexIR.Proofs.StateOf (VerexIR.Proofs.Denote.closureMeaning executable✝)
      (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false) VerexIR.Proofs.Denote.NRow.nil)
      { parameters := [false], results := [false] }
      (VerexIR.Proofs.Denote.ResultShape.one (VerexIR.Proofs.Denote.NTy.int 64 false)) w✝¹⁴ (w✝¹³, ())
      (VerexIR.Proofs.Denote.Skolems.writePlain 0x43.split_pair_frame.unit.nominal.namespace.0.declaration✝¹⁻⁰
        VerexIR.Proofs.Denote.NRow.nil (VerexIR.StorageKey.address w✝¹⁵) none initialState✝) =
    VerexIR.Proofs.Denote.Skolems.writePlain 0x43.split_pair_frame.unit.nominal.namespace.0.declaration✝¹⁻⁰
      VerexIR.Proofs.Denote.NRow.nil (VerexIR.StorageKey.address w✝¹⁵) none initialState✝
w✝³ w✝² : 0x43.split_pair_frame.unit.nominal.namespace.0.declaration✝¹⁻⁰.Holds (Sum.inr (Sum.inl (w✝⁸, ())))
w✝¹ :
  VerexIR.Proofs.Denote.AnchorSaved 9 (entry✝², entry✝¹, entry✝², entry✝, none, entry✝, entry, entry, ()) initialState✝
w✝ :
  VerexIR.Proofs.Denote.AnchorSaved 135 (entry✝², entry✝¹, entry✝², entry✝, some w✝¹⁴, some w✝¹³, entry, entry, ())
    (VerexIR.Proofs.Denote.Skolems.writePlain 0x43.split_pair_frame.unit.nominal.namespace.0.declaration✝¹⁻⁰
      VerexIR.Proofs.Denote.NRow.nil (VerexIR.StorageKey.address w✝¹⁵) none initialState✝)
key : VerexIR.StorageKey
x✝ : ¬key = VerexIR.StorageKey.address s
symmetric : ¬VerexIR.StorageKey.address s = key
h✝ : key = VerexIR.StorageKey.address w✝¹⁵
⊢ VerexIR.Proofs.Denote.Skolems.writePlain 0x43.split_pair_frame.unit.nominal.namespace.0.declaration✝¹⁻⁰
      VerexIR.Proofs.Denote.NRow.nil (VerexIR.StorageKey.address w✝¹⁵) (some (Sum.inr (Sum.inl (w✝⁸, ()))))
      initialState✝
      { type := 0x43.split_pair_frame.unit.nominal.namespace.0.declaration✝¹⁻⁰,
        arguments := VerexIR.Proofs.Denote.NRow.nil }
      (VerexIR.StorageKey.address w✝¹⁵) =
    initialState✝
      { type := 0x43.split_pair_frame.unit.nominal.namespace.0.declaration✝¹⁻⁰,
        arguments := VerexIR.Proofs.Denote.NRow.nil }
      key
---
error: the data invariant of `fst✝¹⁴` at join 1 is not established
---
error: the specification clause `ensures [inferred] do
        let a := s.address
        (..S1 |~ remove<State>(a))` is not established
---
error: the specification clause `ensures [inferred] old(global<State>(s.address)) is Empty && input is Number
        ==> (do
          let a := s.address
          S1.. |~ publish<State>(a, new State::Value { 0 := input.0 }))` is not established
---
error: verex verification failed: the automatic verification of `process` failed; provide a proof in `VerexE2ETests/SourceVerify/split_pair_frame.proof.lean`:
  verify process by lean =>
    verex_obligations
    verex_on
    | leaf => verex_prepare; sorry
    | data_invariant => verex_prepare; sorry
    | ensures_1 => verex_prepare; sorry
    | ensures_2 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_rendering in
verex module 0x43::split_pair_frame where
  pragma proof_file = "VerexE2ETests/SourceVerify/split_pair_frame.proof.lean"

  -- Copyright © Aptos Foundation
  -- SPDX-License-Identifier: Apache-2.0
  -- A match on a pair of a resource and an argument: the split leaves a pattern
  -- equation that is not in normal form, and the closure invoke of `process`
  -- loses it, so `process` is a parked gap. `plain` is the control: the same
  -- pair match with no closure invoke verifies.
  enum Input where
    | Number (0 : u64)
    | Other

  enum State has Copy, Drop, Key where
    | Empty
    | Value (0 : u64)
    | Continuation (0 : Fn(u64) -> u64 has Copy, Drop, Store)

  fun process(s : &Signer, input : Input) -> Unit := do
    let addr := s.address
    match (move_from<State>(addr), input) with
      | (State::Empty {},
        Input::Number { 0 := x }) => move_to<State>(
        s, new State::Value { 0 := x }
      )
      | (State::Continuation { 0 := f },
        Input::Number { 0 := x }) => move_to<State>(
        s, new State::Value { 0 := invoke(f, x) }
      )
      | (_, _) => abort(1)

  spec process where
    pragma opaque
    pragma aborts_if_is_partial
    ensures [inferred] do
        let a := s.address
        (..S1 |~ remove<State>(a))
    ensures [inferred] old(global<State>(s.address)) is Empty && input is Number
        ==> (do
          let a := s.address
          S1.. |~ publish<State>(a, new State::Value { 0 := input.0 }))
    modifies global<State>(s.address)

  fun plain(s : &Signer, input : Input) -> Unit := do
    let addr := s.address
    match (move_from<State>(addr), input) with
      | (State::Value { 0 := y },
        Input::Number { 0 := x }) => move_to<State>(
        s, new State::Value { 0 := x + y }
      )
      | (_, _) => abort(1)

  spec plain where
    modifies *
