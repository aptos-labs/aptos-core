-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Term

/-! Calls through contracts whose failure conditions coincide. -/

namespace VerexIR.Proofs.Denote

variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit]

/-- The weakest precondition through a verified callee whose contract
excuses its postcondition exactly where it must fail, as every generated
contract's does: a successful run refutes the condition, so the
postcondition holds outright. -/
theorem wp_call_strict {Args Result : Type} {function : Args → Comp unit Result}
    {contract : Contract (Memory unit) Failure Args Result} {args : Args}
    {ensures : Result → Memory unit → Prop} {aborts : Failure → Prop} {initial : Memory unit}
    (verified : Satisfies function contract)
    (strict : contract.mayAbort = contract.mustAbort)
    (assumed : contract.assumes args initial)
    (permitted : contract.requires args initial)
    (post : ∀ result final, ¬contract.mustAbort args initial →
      contract.ensures args initial result final → contract.frame args initial result final →
      ensures result final)
    (failing : ∀ error, contract.aborts args initial error → aborts error) :
    wp (function args) ensures aborts initial :=
  wp_call verified assumed permitted
    (fun result final established frame refuted =>
      post result final refuted (established (by rw [strict]; exact refuted)) frame)
    failing

/-- The weakest precondition through a verified callee whose contract's
conditions at the call are given by equations: a contract normalized once
per callee and instance, its normal forms instantiated at each call. -/
theorem wp_call_normal {Args Result : Type} {function : Args → Comp unit Result}
    {contract : Contract (Memory unit) Failure Args Result} {args : Args}
    {ensures : Result → Memory unit → Prop} {aborts : Failure → Prop} {initial : Memory unit}
    {assumes requires mustAbort : Prop} {established framed : Result → Memory unit → Prop}
    {failed : Failure → Prop}
    (verified : Satisfies function contract)
    (strict : contract.mayAbort = contract.mustAbort)
    (assumes_eq : contract.assumes args initial = assumes)
    (requires_eq : contract.requires args initial = requires)
    (mustAbort_eq : contract.mustAbort args initial = mustAbort)
    (ensures_eq : ∀ result final, contract.ensures args initial result final = established result final)
    (frame_eq : ∀ result final, contract.frame args initial result final = framed result final)
    (aborts_eq : ∀ error, contract.aborts args initial error = failed error)
    (assumed : assumes) (permitted : requires)
    (post : ∀ result final, ¬mustAbort → established result final → framed result final →
      ensures result final)
    (failing : ∀ error, failed error → aborts error) :
    wp (function args) ensures aborts initial :=
  wp_call_strict verified strict (assumes_eq ▸ assumed) (requires_eq ▸ permitted)
    (fun result final refuted holds frame =>
      post result final (mustAbort_eq ▸ refuted) (ensures_eq result final ▸ holds)
        (frame_eq result final ▸ frame))
    (fun error failure => failing error (aborts_eq error ▸ failure))

/-- `wp_call` with the contract's conditions at the call given by equations:
the postcondition still excused where the contract permits a failure. -/
theorem wp_call_equations {Args Result : Type} {function : Args → Comp unit Result}
    {contract : Contract (Memory unit) Failure Args Result} {args : Args}
    {ensures : Result → Memory unit → Prop} {aborts : Failure → Prop} {initial : Memory unit}
    {assumes requires mayAbort mustAbort : Prop} {established framed : Result → Memory unit → Prop}
    {failed : Failure → Prop}
    (verified : Satisfies function contract)
    (assumes_eq : contract.assumes args initial = assumes)
    (requires_eq : contract.requires args initial = requires)
    (mayAbort_eq : contract.mayAbort args initial = mayAbort)
    (mustAbort_eq : contract.mustAbort args initial = mustAbort)
    (ensures_eq : ∀ result final, contract.ensures args initial result final = established result final)
    (frame_eq : ∀ result final, contract.frame args initial result final = framed result final)
    (aborts_eq : ∀ error, contract.aborts args initial error = failed error)
    (assumed : assumes) (permitted : requires)
    (post : ∀ result final, (¬mayAbort → established result final) → framed result final →
      ¬mustAbort → ensures result final)
    (failing : ∀ error, failed error → aborts error) :
    wp (function args) ensures aborts initial :=
  wp_call verified (assumes_eq ▸ assumed) (requires_eq ▸ permitted)
    (fun result final holds frame refuted =>
      post result final
        (fun permittedRefuted => ensures_eq result final ▸ holds (mayAbort_eq ▸ permittedRefuted))
        (frame_eq result final ▸ frame) (mustAbort_eq ▸ refuted))
    (fun error failure => failing error (aborts_eq error ▸ failure))

end VerexIR.Proofs.Denote
