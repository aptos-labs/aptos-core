-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
import VerexIR.Proofs.Denote.Types

/-!
# Certification of a nominal's values

What the basis of a native nominal type gives at an arbitrary family: the
plain layer reads nothing of the certified one, a certified value carries
its declaration's invariant, forgetting and certifying are inverse, and
substitution reaches a nominal through its type arguments.
-/

namespace VerexIR.Proofs.Denote
namespace NominalCertification

open VerexIR.SemanticOperations

variable {source : StructHandle} {arguments : NRow} {productive : Bool}

/-- The plain type of a nominal does not depend on what a declaration states
of an encoding (`Carriers.own`). -/
example (base : Carriers) :
    @NTy.plain { base with own := fun _ _ _ => True } (NTy.nominal source arguments productive) =
      @NTy.plain { base with own := fun _ _ _ => False }
        (NTy.nominal source arguments productive) := rfl

/-- Nor on what it states of a native value (`Carriers.nominalOwn`). -/
example (base : Carriers) :
    @NTy.plain { base with nominalOwn := fun _ _ _ _ _ => True }
        (NTy.nominal source arguments productive) =
      @NTy.plain { base with nominalOwn := fun _ _ _ _ _ => False }
        (NTy.nominal source arguments productive) := rfl

/-- A certified value of a nominal satisfies its declaration's invariant. -/
example [Carriers] (value : (NTy.nominal source arguments productive).carrier) :
    Carriers.nominalOwn source productive arguments
      (NRow.plainSortsOf Carriers.toPlain arguments) value.val :=
  Carriers.nominals.nominalHolds_own _ _ _ _ _ _ _ value.property

/-- Certifying a forgotten value gives the value back. -/
example [Carriers] (value : (NTy.nominal source arguments productive).carrier) :
    NTy.ofDeepHolds (NTy.nominal source arguments productive)
        (NTy.forget (NTy.nominal source arguments productive) value)
        (NTy.deepHolds_forget (NTy.nominal source arguments productive) value) = value :=
  NTy.ofDeepHolds_forget (NTy.nominal source arguments productive) value

/-- Forgetting a certified value gives the plain value back. -/
example [Carriers] (value : (NTy.nominal source arguments productive).plain)
    (held : NTy.deepHolds (NTy.nominal source arguments productive) value) :
    NTy.forget (NTy.nominal source arguments productive)
        (NTy.ofDeepHolds (NTy.nominal source arguments productive) value held) = value :=
  NTy.forget_ofDeepHolds (NTy.nominal source arguments productive) value held

/-- Substitution reaches a nominal through its type arguments alone. -/
example (θ : NRow) :
    NTy.subst θ (NTy.nominal source arguments productive) =
      NTy.nominal source (NRow.subst θ arguments) productive := rfl

end NominalCertification
end VerexIR.Proofs.Denote
