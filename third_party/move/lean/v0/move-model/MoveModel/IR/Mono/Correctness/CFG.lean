-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Mono.Correctness.Steps

/-!
# CFG instantiation structure

These inversion lemmas recover source blocks and instruction lists from an
instantiated CFG. They are kept separate because both exact-instance
execution and generated-call simulation need them.
-/

namespace MoveModel.IR

@[simp] theorem Cfg.instantiate_entry (cfg : Cfg) (args : List Ty) :
    (cfg.instantiate args).entry = cfg.entry := rfl

@[simp] theorem Cfg.instantiate_size (cfg : Cfg) (args : List Ty) :
    (cfg.instantiate args).size = cfg.size := rfl

@[simp] theorem Cfg.instantiate_blocks (cfg : Cfg) (args : List Ty)
    (blockId : BlockId) :
    (cfg.instantiate args).blocks blockId =
      (cfg.blocks blockId).map (Block.instantiate args) := rfl

theorem Cfg.instantiate_blocks_eq_some_iff {cfg : Cfg} {args : List Ty}
    {blockId : BlockId} {block : Block} :
    (cfg.instantiate args).blocks blockId = some block ↔
      ∃ source, cfg.blocks blockId = some source ∧
        block = source.instantiate args := by
  simp only [Cfg.instantiate_blocks, Option.map_eq_some_iff]
  constructor
  · rintro ⟨source, hsource, hblock⟩
    exact ⟨source, hsource, hblock.symm⟩
  · rintro ⟨source, hsource, hblock⟩
    exact ⟨source, hsource, hblock.symm⟩

theorem Cfg.instantiate_blocks_eq_some {cfg : Cfg} {args : List Ty}
    {blockId : BlockId} {block : Block}
    (h : cfg.blocks blockId = some block) :
    (cfg.instantiate args).blocks blockId = some (block.instantiate args) := by
  simp [h]

@[simp] theorem Block.instantiate_instrs (block : Block) (args : List Ty) :
    (block.instantiate args).instrs =
      block.instrs.map (Instr.instantiate args) := rfl

@[simp] theorem Block.instantiate_term (block : Block) (args : List Ty) :
    (block.instantiate args).term = block.term := rfl

end MoveModel.IR
