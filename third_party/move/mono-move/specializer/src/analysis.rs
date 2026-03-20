// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Block analysis phase for the slot allocator.
//!
//! Pure analysis producing immutable data consumed by `slotalloc`.

use crate::{
    instr_utils::get_defs_uses,
    ir::{Instr, Slot},
};
use std::collections::BTreeMap;

/// Analysis results for a single basic block.
/// All Slot keys are VIDs (`Vid(i)`).
pub(crate) struct BlockAnalysis {
    pub last_use: BTreeMap<Slot, usize>,
    pub stloc_targets: BTreeMap<Slot, Slot>,
    pub coalesce_to_local: BTreeMap<Slot, Slot>,
    pub xfer_precolor: BTreeMap<Slot, Slot>,
    pub max_xfer_width: u16,
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

/// Analyze a basic block to produce data for named slot allocation.
///
/// Pre: `instrs` is an intra-block SSA slice (after Pass 1 + 1.5).
///      Temp VIDs are defined exactly once.
/// Post: all tables are conservative — every entry is safe to use.
///       stloc_targets and coalesce_to_local are disjoint from xfer_precolor.
pub(crate) fn analyze_block(instrs: &[Instr]) -> BlockAnalysis {
    let is_temp_vid = |r: &Slot| -> bool { r.is_vid() };
    let is_pinned = |r: &Slot| -> bool { matches!(r, Slot::Home(_)) };

    // Forward scan: def_pos and last_use for temp vids.
    // Also build per-local position indices for pinned locals:
    //   local_touch_pos — all positions where local appears as def or use
    //   local_def_pos   — positions where local appears as def only
    let mut last_use: BTreeMap<Slot, usize> = BTreeMap::new();
    let mut def_pos: BTreeMap<Slot, usize> = BTreeMap::new();
    let mut local_touch_pos: BTreeMap<Slot, Vec<usize>> = BTreeMap::new();
    let mut local_def_pos: BTreeMap<Slot, Vec<usize>> = BTreeMap::new();

    for (i, instr) in instrs.iter().enumerate() {
        let (defs, uses) = get_defs_uses(instr);
        for r in &uses {
            if is_temp_vid(r) {
                last_use.insert(*r, i);
            }
            if is_pinned(r) {
                local_touch_pos.entry(*r).or_default().push(i);
            }
        }
        for r in &defs {
            if is_temp_vid(r) {
                last_use.entry(*r).or_insert(i);
                def_pos.entry(*r).or_insert(i);
            }
            if is_pinned(r) {
                local_touch_pos.entry(*r).or_default().push(i);
                local_def_pos.entry(*r).or_default().push(i);
            }
        }
    }

    // Call positions (Call/CallGeneric only, not CallClosure).
    // Already sorted since we iterate in order.
    let call_positions: Vec<usize> = instrs
        .iter()
        .enumerate()
        .filter(|(_, ins)| matches!(ins, Instr::Call(..) | Instr::CallGeneric(..)))
        .map(|(i, _)| i)
        .collect();

    // StLoc look-ahead: map VID → local when VID is produced and later stored
    // to that local, and the local is not accessed in between.
    // Check uses binary search on local_touch_pos: O(log n) per candidate.
    let mut stloc_targets: BTreeMap<Slot, Slot> = BTreeMap::new();
    for (i, instr) in instrs.iter().enumerate() {
        if let Instr::Move(dst, src) = instr
            && is_pinned(dst)
            && is_temp_vid(src)
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
            Instr::Copy(dst, src) | Instr::Move(dst, src) if is_temp_vid(dst) && is_pinned(src) => {
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
            if !is_temp_vid(vid) {
                continue;
            }
            if stloc_targets.contains_key(vid) {
                continue;
            }
            // Skip VIDs coalesced to a pinned local (e.g. Copy(vid, param)).
            // Copy propagation will replace the VID with the local, so the
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
            if has_any_in_range(&call_positions, dp + 1, ci) {
                continue;
            }
            if last_use.get(vid) != Some(&ci) {
                continue;
            }
            xfer_precolor.insert(*vid, Slot::Xfer(j as u16));
        }

        for (k, vid) in rets.iter().enumerate() {
            if !is_temp_vid(vid) {
                continue;
            }
            if stloc_targets.contains_key(vid) {
                continue;
            }
            if xfer_precolor.contains_key(vid) {
                continue;
            }
            if let Some(&lu) = last_use.get(vid)
                && lu >= next_call
            {
                continue;
            }
            xfer_precolor.insert(*vid, Slot::Xfer(k as u16));
        }
    }

    BlockAnalysis {
        last_use,
        stloc_targets,
        coalesce_to_local,
        xfer_precolor,
        max_xfer_width,
    }
}
