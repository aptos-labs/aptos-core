// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Greedy slot allocation.
//!
//! Consumes `BlockAnalysis` from `analysis` and maps SSA `ValueId`s to
//! `Home`/`Transfer` slots using liveness-driven type-keyed reuse.
//! This is the pipeline's only conversion from `Instr<SsaSlot>` to
//! `Instr<NamedSlot>`.

use super::{analysis::BlockAnalysis, ssa_function::SSAFunction};
use crate::stackless_exec_ir::{
    instr_utils::{collect_defs_and_uses, try_map_slots},
    BasicBlock, HomeIndex, NamedSlot, SsaSlot,
};
use mono_move_core::{
    types::InternedType, ExecutionErrorKind, IntoExecutionError, PreparedModule, VMInternalError,
    VMResult,
};
use shared_dsa::UnorderedMap;
use thiserror::Error;

#[derive(Debug, Error)]
enum SlotAllocError {
    #[error("ValueId type not found during SSA allocation")]
    ValueIdTypeNotFound,

    #[error("ValueId {value_id} has no binding at its use site")]
    UnboundValueId { value_id: u16 },
}

impl IntoExecutionError for SlotAllocError {
    fn kind(&self) -> ExecutionErrorKind {
        use SlotAllocError::*;
        match self {
            ValueIdTypeNotFound | UnboundValueId { .. } => ExecutionErrorKind::InvariantViolation,
        }
    }
}

/// Output of slot allocation for a single function.
pub(crate) struct AllocatedFunction {
    pub blocks: Vec<BasicBlock<NamedSlot>>,
    pub num_home_slots: u16,
    pub num_transfer_positions: u16,
    pub home_slot_types: Vec<InternedType>,
}

/// SSA `ValueId` → named-slot mapping with per-slot type tracking.
///
/// Backed by two dense vectors indexed by ordinal:
///   - `home_slot_types[i]` — type of `NamedSlot::Home(i)`.
///   - `value_id_bindings[i]` — binding of `SsaSlot::ValueId(i)` (`None` if
///     unbound). Preallocated to the function's value-id count and never
///     cleared: value ids are globally unique and block-local, so each
///     index is written at most once and stale bindings are never read
///     (debug-asserted via `block_floor`).
///
/// Invariant: every Home slot has a recorded type. Enforced by
/// [`Self::mint_fresh`] being the only slot-introduction path. Pinned-local
/// identity (`Home(i) → Home(i)`, always with `i < num_pinned`) is inferred,
/// not stored.
struct SlotTable {
    /// Pinned local count. Occupies `0..num_pinned` in `home_slot_types`.
    num_pinned: u16,
    /// Type per Home slot, indexed by ordinal. Grows only via `mint_fresh`.
    home_slot_types: Vec<InternedType>,
    /// Binding per ValueId, indexed by ordinal.
    value_id_bindings: Vec<Option<NamedSlot>>,
    /// Lowest value id admissible in the current block. Value ids are
    /// allocated monotonically across blocks, so a cross-block id — which
    /// would otherwise silently resolve to a stale binding — trips the
    /// debug assertions in [`Self::bind`] and [`Self::resolve`].
    #[cfg(debug_assertions)]
    block_floor: u16,
    /// What `block_floor` becomes at the next block boundary: one past the
    /// highest value id bound so far.
    #[cfg(debug_assertions)]
    next_block_floor: u16,
}

impl SlotTable {
    fn new(local_types: &[InternedType], num_value_ids: usize) -> Self {
        Self {
            num_pinned: local_types.len() as u16,
            home_slot_types: local_types.to_vec(),
            value_id_bindings: vec![None; num_value_ids],
            #[cfg(debug_assertions)]
            block_floor: 0,
            #[cfg(debug_assertions)]
            next_block_floor: 0,
        }
    }

    /// Marks a block boundary for the block-locality tripwire; bindings
    /// themselves persist (see the struct doc).
    fn start_block(&mut self) {
        #[cfg(debug_assertions)]
        {
            self.block_floor = self.next_block_floor;
        }
    }

    /// Mints a fresh Home slot with the given type and binds `value_id` to it.
    /// The only path that introduces a new Home slot.
    fn mint_fresh(&mut self, value_id: u16, ty: InternedType) -> NamedSlot {
        let named_slot = NamedSlot::Home(HomeIndex(self.home_slot_types.len() as u16));
        self.home_slot_types.push(ty);
        self.bind(value_id, named_slot);
        named_slot
    }

    /// Binds `value_id` to an existing named slot.
    fn bind(&mut self, value_id: u16, named_slot: NamedSlot) {
        #[cfg(debug_assertions)]
        {
            debug_assert!(
                value_id >= self.block_floor,
                "value id {value_id} crosses a block boundary (block floor {})",
                self.block_floor
            );
            self.next_block_floor = self.next_block_floor.max(value_id.saturating_add(1));
        }
        // An id beyond the preallocated range cannot come out of SSA
        // conversion; leaving it unbound surfaces as `UnboundValueId`
        // at `resolve`.
        if let Some(binding) = self.value_id_bindings.get_mut(value_id as usize) {
            *binding = Some(named_slot);
        }
    }

    /// Returns `(named_slot, type)` for a value ID bound to a Home slot, or
    /// `None` if unbound or bound to a Transfer slot. Returning both pieces
    /// atomically prevents callers from observing a slot without its type.
    fn lookup(&self, value_id: u16) -> Option<(NamedSlot, InternedType)> {
        let named_slot = self.binding_of(value_id)?;
        let NamedSlot::Home(i) = named_slot else {
            return None;
        };
        let ty = *self.home_slot_types.get(i.0 as usize)?;
        Some((named_slot, ty))
    }

    /// Whether `value_id` is bound.
    fn contains(&self, value_id: u16) -> bool {
        self.binding_of(value_id).is_some()
    }

    /// Resolves an SSA slot to its named slot: pinned identity for `Home`,
    /// the recorded binding for `ValueId`. An unbound `ValueId` is a hard
    /// error — SSA is intra-block, so every use is bound by its earlier def.
    fn resolve(&self, slot: SsaSlot) -> VMResult<NamedSlot> {
        match slot {
            SsaSlot::Home(i) => {
                debug_assert!(i.0 < self.num_pinned, "SSA Home slot beyond pinned locals");
                Ok(NamedSlot::Home(i))
            },
            SsaSlot::ValueId(i) => {
                #[cfg(debug_assertions)]
                debug_assert!(
                    i >= self.block_floor,
                    "value id {i} crosses a block boundary (block floor {})",
                    self.block_floor
                );
                self.binding_of(i).ok_or_else(|| {
                    VMInternalError::new(SlotAllocError::UnboundValueId { value_id: i })
                })
            },
        }
    }

    /// Whether `named_slot` is poolable — a non-pinned Home slot.
    fn is_poolable(&self, named_slot: NamedSlot) -> bool {
        matches!(named_slot, NamedSlot::Home(i) if i.0 >= self.num_pinned)
    }

    fn next_slot(&self) -> u16 {
        self.home_slot_types.len() as u16
    }

    /// Consumes the table and returns the per-Home-slot type vector
    /// indexed by ordinal.
    fn into_home_slot_types(self) -> Vec<InternedType> {
        self.home_slot_types
    }

    fn binding_of(&self, value_id: u16) -> Option<NamedSlot> {
        self.value_id_bindings
            .get(value_id as usize)
            .copied()
            .flatten()
    }
}

/// Map SSA `ValueId`s to named slots across all blocks.
///
/// Consumes the SSAFunction and produces named-slot blocks.
///
/// Pre: SSA blocks after fusion passes; value_id_types maps each
///      `ValueId(i)` to its type at index `i`.
/// Post: all `ValueId`s replaced with `Home`/`Transfer` slots
///       (guaranteed by the output type).
pub(crate) fn allocate_slots(
    ssa: SSAFunction,
    module: &PreparedModule,
) -> VMResult<AllocatedFunction> {
    let mut table = SlotTable::new(&ssa.local_types, ssa.value_id_types.len());
    let mut result_blocks = Vec::with_capacity(ssa.blocks.len());
    let mut num_transfer_positions: u16 = 0;
    let mut free_pool: UnorderedMap<InternedType, Vec<NamedSlot>> = UnorderedMap::new();

    let is_bitwise_copy_value = |id: u16| -> bool {
        ssa.value_id_types
            .get(id as usize)
            .is_some_and(|ty| module.is_bitwise_copy_type(*ty))
    };

    for block in ssa.blocks {
        let analysis = BlockAnalysis::analyze(&block.instrs, is_bitwise_copy_value);
        num_transfer_positions = num_transfer_positions.max(analysis.max_transfer_positions);
        result_blocks.push(allocate_block(
            block,
            &mut table,
            &mut free_pool,
            &ssa.value_id_types,
            &analysis,
        )?);
    }

    let num_home_slots = table.next_slot();
    let home_slot_types = table.into_home_slot_types();

    Ok(AllocatedFunction {
        blocks: result_blocks,
        num_home_slots,
        num_transfer_positions,
        home_slot_types,
    })
}

fn value_id_type(value_id: u16, value_id_types: &[InternedType]) -> VMResult<InternedType> {
    value_id_types
        .get(value_id as usize)
        .copied()
        .ok_or_else(|| VMInternalError::new(SlotAllocError::ValueIdTypeNotFound))
}

/// Allocate named slots for a single basic block, consuming its SSA
/// instructions and producing their named-slot counterparts.
///
/// For each instruction, in order: free last-use sources, allocate defs, map
/// to the named-slot form, free dead defs.
/// Allocation priority: transfer_precolor > stloc_targets > coalesce_to_local
/// > type-keyed reuse > fresh.
fn allocate_block(
    block: BasicBlock<SsaSlot>,
    table: &mut SlotTable,
    free_pool: &mut UnorderedMap<InternedType, Vec<NamedSlot>>,
    value_id_types: &[InternedType],
    analysis: &BlockAnalysis,
) -> VMResult<BasicBlock<NamedSlot>> {
    table.start_block();

    let instrs = block.instrs.try_map(|index, instr| -> VMResult<_> {
        let (defs, uses) = collect_defs_and_uses(&instr);

        // Phase 1: Free use-slots whose last use is this instruction.
        // Done BEFORE def allocation so the freed slot can be immediately
        // reused. Safe because sources are read before destinations are
        // written.
        for use_slot in &uses {
            if let SsaSlot::ValueId(id) = use_slot
                && analysis.live_end.get(use_slot) == Some(&index)
                && !defs.contains(use_slot)
                && let Some((named_slot, ty)) = table.lookup(*id)
                && table.is_poolable(named_slot)
            {
                free_pool.entry(ty).or_default().push(named_slot);
            }
        }

        // Phase 2: Allocate named slots for destination `ValueId`s.
        for def_slot in &defs {
            if let SsaSlot::ValueId(id) = def_slot
                && !table.contains(*id)
            {
                if let Some(&position) = analysis.transfer_precolor.get(def_slot) {
                    table.bind(*id, NamedSlot::Transfer(position));
                } else if let Some(&home_idx) = analysis.stloc_targets.get(def_slot) {
                    table.bind(*id, NamedSlot::Home(home_idx));
                } else if let Some(&home_idx) = analysis.coalesce_to_local.get(def_slot) {
                    table.bind(*id, NamedSlot::Home(home_idx));
                } else {
                    // General case: reuse a same-typed slot from the pool,
                    // or mint a fresh one.
                    let ty = value_id_type(*id, value_id_types)?;
                    if let Some(named_slot) = free_pool.get_mut(&ty).and_then(|slots| slots.pop()) {
                        table.bind(*id, named_slot);
                    } else {
                        table.mint_fresh(*id, ty);
                    }
                }
            }
        }

        // Phase 3: Convert the instruction into the named-slot form.
        let converted = try_map_slots(instr, |slot| table.resolve(slot))?;

        // Phase 4: Free slots for defs that are never used
        // (live_end == def site).
        for def_slot in &defs {
            if let SsaSlot::ValueId(id) = def_slot
                && analysis.live_end.get(def_slot) == Some(&index)
                && !uses.contains(def_slot)
                && let Some((named_slot, ty)) = table.lookup(*id)
                && table.is_poolable(named_slot)
            {
                free_pool.entry(ty).or_default().push(named_slot);
            }
        }

        Ok(converted)
    })?;

    Ok(BasicBlock {
        label: block.label,
        instrs,
    })
}
