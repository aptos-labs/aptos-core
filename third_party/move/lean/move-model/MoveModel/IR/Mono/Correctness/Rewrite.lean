-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Mono.Correctness.Plan

/-!
# Call-rewriting correctness

The materializer changes only function-call targets. This file records that
fact independently of both the execution simulation and tag coverage.
-/

namespace MoveModel.IR

/-- Instantiation substitutes the type arguments of a generic call target. -/
theorem Oper.calledInstance?_instantiate (op : Oper) (args : List Ty) :
    (op.instantiate args).calledInstance? =
      (op.calledInstance?.map fun key =>
        ⟨key.funId, instantiateTypes args key.typeArgs⟩) := by
  cases op <;> rfl

/-- Resolve an instantiated call to the generated function selected by the
plan. -/
@[simp] theorem MonoPlan.rewriteOper_functionInst {plan : MonoPlan}
    {f : FunId} {typeArgs : List Ty} {generated : FunId}
    (h : plan.generatedFunId? ⟨f, typeArgs⟩ = some generated) :
    plan.rewriteOper (.functionInst f typeArgs) = .function generated := by
  simp [MonoPlan.rewriteOper, h]

/-- Resolve a nongeneric call to the generated empty-instantiation entry. -/
@[simp] theorem MonoPlan.rewriteOper_function {plan : MonoPlan}
    {f generated : FunId}
    (h : plan.generatedFunId? ⟨f, []⟩ = some generated) :
    plan.rewriteOper (.function f) = .function generated := by
  simp [MonoPlan.rewriteOper, h]

/-- Rewriting an instruction preserves its operands. -/
@[simp] theorem MonoPlan.rewriteInstr_call (plan : MonoPlan)
    (dsts : List LocalIndex) (op : Oper) (srcs : List LocalIndex) :
    plan.rewriteInstr (.call dsts op srcs) =
      .call dsts (plan.rewriteOper op) srcs := rfl

/-- Rewriting preserves a CFG's entry block. -/
@[simp] theorem MonoPlan.rewriteCfg_entry (plan : MonoPlan) (cfg : Cfg) :
    (plan.rewriteCfg cfg).entry = cfg.entry := rfl

/-- Rewriting preserves a CFG's declared block bound. -/
@[simp] theorem MonoPlan.rewriteCfg_size (plan : MonoPlan) (cfg : Cfg) :
    (plan.rewriteCfg cfg).size = cfg.size := rfl

/-- Rewriting a block lookup maps precisely the instructions in that block. -/
@[simp] theorem MonoPlan.rewriteCfg_blocks (plan : MonoPlan) (cfg : Cfg)
    (blockId : BlockId) :
    (plan.rewriteCfg cfg).blocks blockId =
      (cfg.blocks blockId).map fun block =>
        { block with instrs := block.instrs.map plan.rewriteInstr } := rfl

/-- A successful source block lookup yields the corresponding rewritten block. -/
theorem MonoPlan.rewriteCfg_blocks_eq_some {plan : MonoPlan} {cfg : Cfg}
    {blockId : BlockId} {block : Block}
    (h : cfg.blocks blockId = some block) :
    (plan.rewriteCfg cfg).blocks blockId =
      some { block with instrs := block.instrs.map plan.rewriteInstr } := by
  simp [h]

end MoveModel.IR
