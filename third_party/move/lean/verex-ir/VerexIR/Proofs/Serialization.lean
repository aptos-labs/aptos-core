-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Representation

/-! # Serialization

The meaning of a serialization function the Move Prover's prelude gives
(`bcs::serialize`): one byte-vector function of a value and the types it is
taken at, of which only that no value's bytes are empty is known. The
prelude also states the serialization injective and every address of one
length; no function into byte vectors of bounded length is either of every
value, so a function assumes those of its parameters instead. -/

namespace VerexIR.Proofs

/-- Some byte-vector function of type arguments and a value has no empty
value. -/
theorem exists_nonempty_bytes : ∃ bytes : (Types : Type) → Types → (Arguments : Type) → Arguments →
    SpecVector (SpecInt (.bits 8) false),
    ∀ Types typeArguments Arguments bundle,
      0 < (bytes Types typeArguments Arguments bundle).values.size :=
  ⟨fun _ _ _ _ => ⟨#[⟨0, by decide⟩], by decide⟩, fun _ _ _ _ => Nat.one_pos⟩

/-- A value's serialization at type arguments: chosen among the functions
with no empty value, so that nothing about it but that is known. -/
noncomputable def serializedBytes {Types : Type} (typeArguments : Types) {Arguments : Type}
    (bundle : Arguments) : SpecVector (SpecInt (.bits 8) false) :=
  Classical.choose exists_nonempty_bytes Types typeArguments Arguments bundle

/-- No value's serialization is empty. -/
theorem serializedBytes_nonempty {Types : Type} (typeArguments : Types) {Arguments : Type}
    (bundle : Arguments) : 0 < (serializedBytes typeArguments bundle).values.size :=
  Classical.choose_spec exists_nonempty_bytes Types typeArguments Arguments bundle

end VerexIR.Proofs
