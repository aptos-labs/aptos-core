// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The cells a call to each function may borrow, which is the third pass.

use super::{
    body::{BodyInfo, Call, Callee},
    cells::{CellIdx, Cells},
};
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::def_id::LocalDefId;
use std::collections::BTreeSet;

/// For every body, the cells a call to it may borrow: the ones it holds, and
/// the ones its closures and the functions of this crate it calls may borrow.
/// Every cell, if on the way it hands a value that reaches the struct to
/// opaque code, or owns a value whose destructor reaches the struct.
/// Computed as a fixpoint over the call graph of this crate.
pub(super) fn compute<'tcx>(
    cells: &Cells<'tcx>,
    bodies: &FxHashMap<LocalDefId, BodyInfo<'tcx>>,
) -> FxHashMap<LocalDefId, BTreeSet<CellIdx>> {
    let mut may_borrow: FxHashMap<LocalDefId, BTreeSet<CellIdx>> = FxHashMap::default();
    for (owner, info) in bodies {
        let own = if info.drops_struct || info.calls.iter().any(Call::leaks) {
            (0..cells.list.len()).collect()
        } else {
            info.sites.iter().map(|(cell, _)| *cell).collect()
        };
        may_borrow.insert(*owner, own);
    }
    loop {
        let mut changed = false;
        for (owner, info) in bodies {
            let mut set = may_borrow[owner].clone();
            let callees = info.calls.iter().filter_map(|c| match c.callee {
                Callee::Local(def_id) => Some(def_id),
                Callee::External(_) | Callee::Dynamic(_) | Callee::Value => None,
            });
            for callee in info.closures.iter().copied().chain(callees) {
                if let Some(theirs) = may_borrow.get(&callee) {
                    set.extend(theirs.iter().copied());
                }
            }
            if set.len() != may_borrow[owner].len() {
                may_borrow.insert(*owner, set);
                changed = true;
            }
        }
        if !changed {
            return may_borrow;
        }
    }
}
