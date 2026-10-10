-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Transpiler.Tests.Programs.MoveStdlib.Std.Option

/-! The generated `std::option::borrow_mut` contract is proved against its
payload-returning body, rather than merely elaborated or assumed opaque. -/

open Move
open scoped Move Move.Spec

verify Std.option.borrow_mut by
  contract_intro
  unfold Std.option.borrow_mut.mutationSpec
  unfold Std.option.borrow.specFun Std.option.is_none.specFun
  simp [wp_norm]
  intro future
  cases args <;> simp_all [Move.Semantics.Mutation.Finished]
  all_goals simp [wp_norm]
  intro finalOwner
  subst future
  simp
