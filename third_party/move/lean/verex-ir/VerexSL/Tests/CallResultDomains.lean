-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# The domain a call binds its result at

The call rule is stated at the domain of the callee's result
(`callResultDomain?`): `Unit` without a result, and with one the carrier of
its native type at the family the call is made at, the callee's type
arguments substituted. Each control reads the binder of a call's result in
the leaf the walk leaves, before any preparation: its type is that domain,
mentions no result shape, substitution or induced family, and is a fixed
point of the family normalization, so nothing after the rule spells it again.
The shapes are no result, one result, a named struct, a tuple, a mutable
reference, a generic call at a type argument, and a generic call inside an
inlined generic callee.
-/

namespace VerexSL.Tests.CallResultDomains

open Lean Meta Elab Tactic
open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

/-- The local `x` is a call's result bound at the domain the call rule
states: `Unit` for `unit`, else the carrier of a native type that is a named
declaration for `nominal` and whose head is the constructor `NTy.<head>`
otherwise. -/
elab "expect_result_domain " x:ident " : " head:ident : tactic => withMainContext do
  let decl ← getLocalDeclFromUserName x.getId
  let type ← instantiateMVars decl.type
  if head.getId == `unit then
    unless type.isConstOf ``Unit do throwError "{x} is bound at {type}"
  else
    unless type.isAppOfArity ``NTy.carrier 2 do throwError "{x} is bound at {type}"
    let τ := (type.getArg! 1).consumeMData
    let wanted ← if head.getId == `nominal then pure (isNominalType (← getEnv) τ)
      else pure (τ.getAppFn.constName? == some (``NTy ++ head.getId))
    unless wanted do throwError "{x} is bound at the carrier of {τ}"
  for spelling in [``ResultShape.carrier, ``ResultShape.subst, ``NTy.subst, ``NRow.subst,
      ``Skolems.instantiate, ``Carriers.instantiate] do
    if (type.find? (·.isConstOf spelling)).isSome then
      throwError "{x} is bound at a type that mentions {spelling}: {type}"
  unless (← canonicalFamilyResult type).expr == type do
    throwError "the family normalization spells the domain of {x} again: {type}"

verex module 0x42::call_result_domains where
  pragma verify = false

  native fun touch(x : u64) -> Unit
  spec touch where
    aborts_if false

  native fun make_u64() -> u64
  spec make_u64 where
    aborts_if false
    ensures result == 7

  struct Percent has Copy, Drop where
    value : u64
  spec Percent where
    invariant value <= 100

  native fun make_percent() -> Percent
  spec make_percent where
    aborts_if false

  native fun make_two() -> (u64, Bool)
  spec make_two where
    aborts_if false
    ensures spec.result[0] == 7

  native fun summarized(slot : &mut u64) -> &mut u64
  spec summarized where
    ensures result == old(slot) && slot == final(result)
    aborts_if false

  native fun same {T has Copy, Drop}(x : T) -> T
  spec same where
    aborts_if false
    ensures result == x

  fun through {T has Copy, Drop}(x : T) -> T := same::<T>(x)

  fun use_none(x : u64) -> u64 := do
    touch(x)
    x
  spec use_none where
    aborts_if false
    ensures result == x

  fun use_one() -> u64 := make_u64()
  spec use_one where
    aborts_if false
    ensures result == 7

  fun use_nominal() -> Percent := make_percent()
  spec use_nominal where
    aborts_if false
    ensures result.value <= 100

  fun use_tuple() -> (u64, Bool) := make_two()
  spec use_tuple where
    aborts_if false
    ensures spec.result[0] == 7

  fun use_reference(slot : &mut u64) -> Unit := do
    let returned := summarized(slot)
    *returned := 37
  spec use_reference where
    aborts_if false
    ensures slot == 37

  fun use_generic() -> u64 := same::<u64>(7)
  spec use_generic where
    aborts_if false
    ensures result == 7

  fun use_nested() -> u64 := through::<u64>(7)
  spec use_nested where
    aborts_if false
    ensures result == 7

verify 0x42::call_result_domains::use_none by lean =>
  verex_obligations
  verex_on
  | ensures_1 => expect_result_domain touch_result : unit; verex_prepare; all_goals verex_decide
  | _ => verex_decide

verify 0x42::call_result_domains::use_one by lean =>
  verex_obligations
  verex_on
  | ensures_1 => expect_result_domain make_u64_result : int; verex_prepare; all_goals verex_decide
  | _ => verex_decide

verify 0x42::call_result_domains::use_nominal by lean =>
  verex_obligations
  verex_on
  | ensures_1 => expect_result_domain make_percent_result : nominal; verex_prepare; all_goals verex_decide
  | _ => verex_decide

verify 0x42::call_result_domains::use_tuple by lean =>
  verex_obligations
  verex_on
  | ensures_1 => expect_result_domain make_two_result : tuple; verex_prepare; all_goals verex_decide
  | _ => verex_decide

verify 0x42::call_result_domains::use_reference by lean =>
  verex_obligations
  verex_on
  | ensures_1 => expect_result_domain summarized_result : ref; verex_prepare; all_goals verex_decide
  | _ => verex_decide

verify 0x42::call_result_domains::use_generic by lean =>
  verex_obligations
  verex_on
  | ensures_1 => expect_result_domain same_result : int; verex_prepare; all_goals verex_decide
  | _ => verex_decide

verify 0x42::call_result_domains::use_nested by lean =>
  verex_obligations
  verex_on
  | ensures_1 => expect_result_domain same_result : int; verex_prepare; all_goals verex_decide
  | _ => verex_decide

end VerexSL.Tests.CallResultDomains
