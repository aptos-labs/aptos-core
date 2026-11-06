-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Properties.References

/-!
# Validity of locations, and the events that end it

UNTRUSTED (checked by the kernel). The vocabulary of the proof that D4b′,
D4c and D4e imply D4a (`Proofs.Dangling`): a location is valid for a
reference when it resolves and no invalidation recorded since the
reference's creation covers it (`Valid`); a step keeps every valid location
valid unless one of its events hits it (`Hits`): a direct write overlapping
it, a write through a reference strictly enclosing it, or the end of its
frame. Those are exactly what D4b′, D4c and D4e exclude before a later use.
-/

namespace MoveBytecode.Proofs

/-- Location `l` is valid in `s` for a reference created when `n`
invalidations had been recorded. -/
def Valid (s : State) (l : Location) (n : Nat) : Prop :=
  (s.deref? l n).isSome = true ∧ n ≤ s.invalidations.length

/-- Event `e` may end the validity of location `l`: a direct write that
overlaps it, a write through a reference of a location strictly enclosing it,
or the end of the frame whose local it is in. -/
def Hits (l : Location) (e : Event) : Prop :=
  (∃ m, e = .accessed .write m none ∧ Access.write.overlaps m l = true) ∨
  (∃ m t, e = .accessed .write m (some t) ∧ l.strictlyInside m = true) ∨
  (∃ f rs j, e = .frameEnded f rs ∧ l.root = .local_ f j)

end MoveBytecode.Proofs
