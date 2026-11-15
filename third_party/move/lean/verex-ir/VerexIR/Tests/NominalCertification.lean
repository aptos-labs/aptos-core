-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.GivenOwn

/-!
# Carrier independence and invariant sensitivity of nominals

Under the single-tower plain carrier representation:
- `NTy.Holds` depends on `Carriers.own` (negative control), while `NTy.carrier` is
  independent of both `Carriers.own` and `Carriers.holds` (positives, definitionally on
  concrete argument rows and propositionally across all `arguments : NRow`);
- `NTy.subst` distinguishes distinct nominal flags (negative control) and reaches a
  nominal through its type arguments alone (positive).
-/

namespace VerexIR.Proofs.Denote
namespace NominalCertification

variable {source : StructHandle} {arguments : NRow} {productive : Bool}

set_option linter.unusedVariables false in
/-- Negative control (first): `NTy.Holds` on a declaration DOES depend on `Carriers.own`. -/
example (base : Carriers) (v : @NTy.carrier base (.struct source .nil .nil)) : True := by
  fail_if_success
    have : @NTy.Holds { base with own := fun _ _ _ => True } (.struct source .nil .nil) v =
        @NTy.Holds { base with own := fun _ _ _ => False } (.struct source .nil .nil) v := rfl
  trivial

/-- Positive: On concrete argument rows, the plain carrier type of `NTy.nominal` is
definitionally independent of `Carriers.own`. -/
example (base : Carriers) :
    @NTy.carrier { base with own := fun _ _ _ => True }
        (NTy.nominal source (.cons (.param 0) .nil) productive) =
      @NTy.carrier { base with own := fun _ _ _ => False }
        (NTy.nominal source (.cons (.param 0) .nil) productive) := rfl

/-- Positive: Across every argument row, `NTy.carrier` of `NTy.nominal` is independent
of `Carriers.own`. -/
example (base : Carriers) :
    @NTy.carrier { base with own := fun _ _ _ => True } (NTy.nominal source arguments productive) =
      @NTy.carrier { base with own := fun _ _ _ => False }
        (NTy.nominal source arguments productive) :=
  (NTy.carrier_withOwn base _ (NTy.nominal source arguments productive)).trans
    (NTy.carrier_withOwn base _ (NTy.nominal source arguments productive)).symm

/-- Positive: Nor does the plain carrier type of `NTy.nominal` depend on `Carriers.holds`. -/
example (base : Carriers) :
    @NTy.carrier { base with holds := fun _ _ => True }
        (NTy.nominal source (.cons (.param 0) .nil) productive) =
      @NTy.carrier { base with holds := fun _ _ => False }
        (NTy.nominal source (.cons (.param 0) .nil) productive) := rfl

set_option linter.unusedVariables false in
/-- Negative control (first): Substitution does not conflate distinct `productive` flags. -/
example (θ : NRow) : True := by
  fail_if_success
    have : NTy.subst θ (NTy.nominal source arguments true) =
        NTy.subst θ (NTy.nominal source arguments false) := rfl
  trivial

/-- Positive: Substitution reaches a nominal through its type arguments alone. -/
example (θ : NRow) :
    NTy.subst θ (NTy.nominal source arguments productive) =
      NTy.nominal source (NRow.subst θ arguments) productive := rfl

end NominalCertification
end VerexIR.Proofs.Denote
