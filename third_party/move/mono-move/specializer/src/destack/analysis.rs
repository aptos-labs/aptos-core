// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Block analysis phase for the slot allocator.
//!
//! # Borrow safety
//!
//! The analysis tracks direct reads/writes of pinned locals but does not track
//! indirect mutation through references (e.g. `MutBorrowLoc` + `WriteRef`).
//! This is sound because Move's borrow checker guarantees that a local cannot
//! be directly read or written while a mutable reference to it is outstanding.

use super::instr_utils::get_defs_uses;
use crate::stackless_exec_ir::{Instr, Slot};
use std::collections::BTreeMap;

/// Analysis results for a single basic block.
/// All Slot keys are `Vid`s.
pub(crate) struct BlockAnalysis {
    /// `Vid` -> last instruction index where it is used or defined.
    pub last_use: BTreeMap<Slot, usize>,
    /// `Vid` -> pinned local it is stored into, when the local is untouched in between.
    pub stloc_targets: BTreeMap<Slot, Slot>,
    /// `Vid` -> pinned local it was loaded from, when the local is not redefined before last use.
    pub coalesce_to_local: BTreeMap<Slot, Slot>,
    /// `Vid` -> `Xfer` for call args/rets whose live range doesn't cross a call.
    pub xfer_precolor: BTreeMap<Slot, Slot>,
    /// Maximum number of transfer slots needed across all calls in the block.
    pub max_xfer_width: u16,
}

impl BlockAnalysis {
    /// Analyze a basic block to produce data for named slot allocation.
    ///
    /// Pre: `instrs` is an intra-block SSA slice.
    ///      `Vid`s are defined exactly once.
    /// Post: each table entry is sound:
    ///   - stloc_targets: the local is not touched between the `Vid`'s def and the store.
    ///   - coalesce_to_local: the local is not redefined between the load and the `Vid`'s last use.
    ///   - xfer_precolor: the `Vid`'s live range does not cross another call.
    ///   - stloc_targets and coalesce_to_local are disjoint from xfer_precolor.
    pub(crate) fn analyze(instrs: &[Instr]) -> Self {
        // Forward scan: build per-`Vid` and per-`Home` position indices.
        // `Vid` -> last instruction index where it appears as def or use.
        let mut last_use: BTreeMap<Slot, usize> = BTreeMap::new();
        // `Vid` -> instruction index where it is defined.
        let mut def_pos: BTreeMap<Slot, usize> = BTreeMap::new();
        // `Home` -> sorted positions where the local appears as def or use.
        let mut local_touch_pos: BTreeMap<Slot, Vec<usize>> = BTreeMap::new();
        // `Home` -> sorted positions where the local appears as def only.
        let mut local_def_pos: BTreeMap<Slot, Vec<usize>> = BTreeMap::new();

        for (i, instr) in instrs.iter().enumerate() {
            let (defs, uses) = get_defs_uses(instr);
            for r in &uses {
                if r.is_vid() {
                    last_use.insert(*r, i);
                }
                if r.is_home() {
                    local_touch_pos.entry(*r).or_default().push(i);
                }
            }
            for r in &defs {
                if r.is_vid() {
                    last_use.entry(*r).or_insert(i);
                    def_pos.entry(*r).or_insert(i);
                }
                if r.is_home() {
                    local_touch_pos.entry(*r).or_default().push(i);
                    local_def_pos.entry(*r).or_default().push(i);
                }
            }
        }

        // Call positions — any instruction that clobbers xfer slots.
        // Already sorted since we iterate in order.
        let call_positions: Vec<usize> = instrs
            .iter()
            .enumerate()
            .filter(|(_, ins)| {
                matches!(
                    ins,
                    Instr::Call(..) | Instr::CallGeneric(..) | Instr::CallClosure(..)
                )
            })
            .map(|(i, _)| i)
            .collect();

        // StLoc look-ahead: map `Vid` → local when `Vid` is produced and later stored
        // to that local, and the local is not accessed in between.
        // Check uses binary search on local_touch_pos: O(log n) per candidate.
        let mut stloc_targets: BTreeMap<Slot, Slot> = BTreeMap::new();
        for (i, instr) in instrs.iter().enumerate() {
            if let Instr::Move(dst, src) = instr
                && dst.is_home()
                && src.is_vid()
                && !stloc_targets.contains_key(src)
            {
                let dp = def_pos.get(src).copied().unwrap_or(0);
                let touches = local_touch_pos
                    .get(dst)
                    .map(|v| v.as_slice())
                    .unwrap_or(&[]);
                let local_touched = has_any_in_range(touches, dp + 1, i);
                if !local_touched {
                    stloc_targets.insert(*src, *dst);
                }
            }
        }

        // CopyLoc/MoveLoc coalescing.
        // Check uses binary search on local_def_pos: O(log n) per candidate.
        let mut coalesce_to_local: BTreeMap<Slot, Slot> = BTreeMap::new();
        for (i, instr) in instrs.iter().enumerate() {
            match instr {
                Instr::Copy(dst, src) | Instr::Move(dst, src) if dst.is_vid() && src.is_home() => {
                    let vid = *dst;
                    if let Some(&lu) = last_use.get(&vid)
                        && lu > i
                        && !stloc_targets.contains_key(&vid)
                    {
                        let local = *src;
                        let defs = local_def_pos
                            .get(&local)
                            .map(|v| v.as_slice())
                            .unwrap_or(&[]);
                        let local_redefined = has_any_in_range(defs, i + 1, lu);
                        if !local_redefined {
                            coalesce_to_local.insert(vid, local);
                        }
                    }
                },
                _ => {},
            }
        }

        // Xfer slot precoloring.
        // has_call_between uses binary search on call_positions: O(log c) per query.
        let mut xfer_precolor: BTreeMap<Slot, Slot> = BTreeMap::new();
        let mut max_xfer_width: u16 = 0;

        for (ci_idx, &ci) in call_positions.iter().enumerate() {
            let (rets, args) = match &instrs[ci] {
                Instr::Call(rets, _, args) | Instr::CallGeneric(rets, _, args) => {
                    (rets.clone(), args.clone())
                },
                _ => continue,
            };

            let next_call = call_positions
                .get(ci_idx + 1)
                .copied()
                .unwrap_or(instrs.len());

            let call_width = std::cmp::max(args.len(), rets.len()) as u16;
            if call_width > max_xfer_width {
                max_xfer_width = call_width;
            }

            for (j, vid) in args.iter().enumerate() {
                if !vid.is_vid() {
                    continue;
                }
                if stloc_targets.contains_key(vid) {
                    continue;
                }
                // Skip `Vid`s coalesced to a pinned local (e.g. Copy(vid, param)).
                // Copy propagation will replace the `Vid` with the local, so the
                // call ends up passing the local directly — no xfer copy needed.
                if coalesce_to_local.contains_key(vid) {
                    continue;
                }
                if xfer_precolor.contains_key(vid) {
                    continue;
                }
                let dp = match def_pos.get(vid) {
                    Some(&p) => p,
                    None => continue,
                };
                // Live range crosses an earlier call — xfer slot would be clobbered.
                if has_any_in_range(&call_positions, dp + 1, ci) {
                    continue;
                }
                // Used after this call — xfer slot won't survive.
                if last_use.get(vid) != Some(&ci) {
                    continue;
                }
                xfer_precolor.insert(*vid, Slot::Xfer(j as u16));
            }

            for (k, vid) in rets.iter().enumerate() {
                if !vid.is_vid() {
                    continue;
                }
                if stloc_targets.contains_key(vid) {
                    continue;
                }
                if xfer_precolor.contains_key(vid) {
                    continue;
                }
                // Used at or after the next call — xfer slot would be clobbered.
                if let Some(&lu) = last_use.get(vid)
                    && lu >= next_call
                {
                    continue;
                }
                xfer_precolor.insert(*vid, Slot::Xfer(k as u16));
            }
        }

        Self {
            last_use,
            stloc_targets,
            coalesce_to_local,
            xfer_precolor,
            max_xfer_width,
        }
    }
}

/// Returns true if `sorted` contains any element in the half-open range [lo, hi).
fn has_any_in_range(sorted: &[usize], lo: usize, hi: usize) -> bool {
    if lo >= hi {
        return false;
    }
    // Find the first element >= lo.
    let idx = sorted.partition_point(|&x| x < lo);
    idx < sorted.len() && sorted[idx] < hi
}
