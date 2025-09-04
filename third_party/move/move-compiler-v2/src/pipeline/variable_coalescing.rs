// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements a transformation that reuses locals of the same type when
//! possible.
//!
//! prerequisites:
//! - livevar annotation is available by performing liveness analysis.
//! - copy inference is already performed.
//! - it is preferable, but not required, to have run dead store elimination (so that
//!   dead stores do not consume and retain variable slots).
//! side effect: this transformation removes all pre-existing annotations.
//!
//! This transformation is closely related to the register allocation problem in
//! compilers. As such, an optimal solution to reusing locals is NP-complete, as we
//! can show its equivalence to the graph coloring problem.
//!
//! Our solution here is inspired by the paper "Linear Scan Register Allocation"
//! by Poletto and Sarkar, which proposes a fast and greedy register allocation
//! algorithm. While potentially suboptimal, it is simple and fast, and is known to
//! produce good results in practice. Our solution uses some key ideas from that
//! paper, and performs a linear scan for deciding which locals to reuse.
//!
//! A key concept in this transformation is the "live interval" of a local, as opposed
//! to the more precise "live range" (the set of code offsets where a local is live).
//! The live interval of a local `t` is a consecutive range of code offsets `[i, j]`
//! such that there is no code offset `j' > j` where `t` is live at `j'`, and there is
//! no code offset `i' < i` where `t` is live at `i'`. A trivial live interval for any
//! local is `[0, MAX_CODE_OFFSET]`, but we can often compute more precise live intervals.
//!
//! The transformation greedily reuses (same-typed) locals outside their live intervals.
//! Note that this transformation could potentially create several dead stores, which
//! can be removed by running the dead store elimination transformation afterwards.

use crate::pipeline::livevar_analysis_processor::LiveVarAnnotation;
use move_binary_format::file_format::CodeOffset;
use move_model::{ast::TempIndex, model::FunctionEnv, ty::Type};
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::Bytecode,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::RangeInclusive,
};

/// The live interval of a local.
/// Note that two live intervals i1: b1..=x and i2: x..=e2 are not considered to overlap
/// even though the code offset `x` is included in both intervals.
struct LiveInterval(RangeInclusive<CodeOffset>);

impl LiveInterval {
    /// Create a new live interval that only has the given offset.
    fn new(offset: CodeOffset) -> Self {
        Self(offset..=offset)
    }

    /// Include the given offset in the live interval, expanding the interval as necessary.
    fn include(&mut self, offset: CodeOffset) {
        use std::cmp::{max, min};
        self.0 = min(*self.0.start(), offset)..=max(*self.0.end(), offset);
    }
}

/// Live interval event of a local, used for sorting.
#[derive(Clone)]
enum LiveIntervalEvent {
    /// `Begin(local, start, len)` indicates `local` is first defined at code offset `start`
    /// and has a live interval of length `len` (used for tie-breaking).
    Begin(TempIndex, CodeOffset, usize),
    /// `End(local, end)` indicates `local` is last used at code offset `end`.
    End(TempIndex, CodeOffset),
}

impl LiveIntervalEvent {
    /// Get the code offset at which the event occurs.
    fn offset(&self) -> CodeOffset {
        match self {
            LiveIntervalEvent::Begin(_, offset, _) => *offset,
            LiveIntervalEvent::End(_, offset) => *offset,
        }
    }

    /// Get the local associated with the event.
    fn local(&self) -> TempIndex {
        match self {
            LiveIntervalEvent::Begin(local, _, _) => *local,
            LiveIntervalEvent::End(local, _) => *local,
        }
    }
}

/// Annotate each code offset with its associated live interval events.
#[derive(Clone)]
struct LiveIntervalAnnotation(BTreeMap<CodeOffset, Vec<LiveIntervalEvent>>);

pub struct VariableCoalescing {
    /// If true: only add live interval event annotations, do not perform the transformation.
    /// If false: only perform the transformation, do not add any annotations.
    annotate: bool,
}

impl VariableCoalescing {
    /// Create an instance for performing the variable coalescing transformation.
    /// No annotations are added.
    pub fn transform_only() -> Self {
        Self { annotate: false }
    }

    /// Create an instance for annotating the live interval events associated with variable
    /// coalescing, but do not perform any transformation.
    /// This is useful for testing and debugging.
    pub fn annotate_only() -> Self {
        Self { annotate: true }
    }

    /// Compute the live intervals of locals in the given function target.
    /// The result is a vector of live intervals, where the index of the vector is the local.
    /// If a local has `None` as its live interval, we can ignore the local for the coalescing
    /// transformation (eg., because it is borrowed or because it is never used): it implies
    /// that it is the trivial live interval.
    fn live_intervals(target: &FunctionTarget) -> Vec<Option<LiveInterval>> {
        let LiveVarAnnotation(live_var_infos) = target
            .get_annotations()
            .get::<LiveVarAnnotation>()
            .expect("live var annotation is a prerequisite");
        // Note: we currently exclude all the variables that are borrowed or appear in spec blocks
        // from participating in this transformation, which is safe. However, we could be more
        // precise in this regard.
        let pinned_locals = target.get_pinned_temps(false);
        // Initially, all locals have trivial live intervals.
        // They are made more precise using live variable analysis.
        let mut live_intervals = std::iter::repeat_with(|| None)
            .take(target.get_local_count())
            .collect::<Vec<_>>();
        let code = target.get_bytecode();
        for (offset, live_var_info) in live_var_infos.iter() {
            live_var_info
                .after
                .keys()
                .chain(live_var_info.before.keys())
                .chain(code[*offset as usize].dests().iter())
                .filter(|local| !pinned_locals.contains(local))
                .for_each(|local| {
                    // non-pinned local that is:
                    // - live before and/or after the code offset.
                    // - written to at the code offset (but may never be live).
                    let interval =
                        live_intervals[*local].get_or_insert_with(|| LiveInterval::new(*offset));
                    interval.include(*offset);
                });
        }
        live_intervals
    }

    /// Compute the sorted live interval events of locals in the given function target.
    ///
    /// The sort order is as follows (in order of precedence):
    /// 1. `Begin` events of parameters come first, in the order of their local index.
    /// 2. events with lower code offsets come earlier.
    /// 3. `End` comes before `Begin`.
    /// 4. two `End` events are ordered by local index (arbitrary but deterministic).
    /// 5. two `Begin` events are ordered by the length[**] of their live intervals (shorter comes earlier),
    ///    and then by local index (arbitrary but deterministic).
    ///
    /// [**] The intention behind ordering by lengths is to remap a local with shorter interval first,
    /// so that it can be reused by other locals sooner.
    fn sorted_live_interval_events(target: &FunctionTarget) -> Vec<LiveIntervalEvent> {
        let live_intervals = Self::live_intervals(target);
        let mut sorted_events = vec![];
        let mut other_events = vec![];
        let param_count = target.get_parameter_count();
        let is_param = |t: TempIndex| t < param_count;
        // Initially, `sorted_events` contain `Begin` events of parameters, `other_events` contain
        // all other events.
        for (local, interval) in live_intervals.into_iter().enumerate() {
            if let Some(LiveInterval(range)) = interval {
                let (start, end) = (*range.start(), *range.end());
                let begin_event = LiveIntervalEvent::Begin(local, start, range.count());
                if is_param(local) {
                    sorted_events.push(begin_event);
                } else {
                    other_events.push(begin_event);
                }
                other_events.push(LiveIntervalEvent::End(local, end));
            }
        }
        // Sort `other_events` based on ordering notes 2-5 from the function documentation.
        other_events.sort_by(|a, b| {
            use LiveIntervalEvent::*;
            a.offset().cmp(&b.offset()).then_with(|| match (a, b) {
                (End(..), Begin(..)) => std::cmp::Ordering::Less,
                (Begin(..), End(..)) => std::cmp::Ordering::Greater,
                (End(local_a, _), End(local_b, _)) => local_a.cmp(local_b),
                (Begin(local_a, _, length_a), Begin(local_b, _, length_b)) => {
                    length_a.cmp(length_b).then_with(|| local_a.cmp(local_b))
                },
            })
        });
        sorted_events.append(&mut other_events); // `other_events` are now sorted.
        sorted_events
    }

    /// Compute the coalesceable locals of the given function target.
    /// The result is a map, where for each mapping from local `t` to its coalesceable local `u`,
    /// we can safely replace all occurrences of `t` with `u`.
    ///
    /// This safety property follows from:
    ///   - `t` and `u` are of the same type
    ///   - the live intervals of `t` and `u` do not overlap, in which case, they do not interfere
    ///     with each others computations,
    fn coalesceable_locals(target: &FunctionTarget) -> BTreeMap<TempIndex, TempIndex> {
        let sorted_events = Self::sorted_live_interval_events(target);
        // Map local `t` to its coalesceable local `u`, where the replacement `t` -> `u` is safe.
        let mut coalesceable_locals = BTreeMap::new();
        // For each type in the function, keep track of the available locals (not alive) of that type.
        let mut avail_map: BTreeMap<&Type, BTreeSet<TempIndex>> = BTreeMap::new();
        let bytecode = target.get_bytecode();
        for event in sorted_events {
            use LiveIntervalEvent::*;
            match event {
                Begin(local, offset, _) => {
                    let local_type = target.get_local_type(local);
                    if let Some(avail_locals) = avail_map.get_mut(local_type) {
                        if !avail_locals.is_empty() {
                            // There are dead locals available with matching types.
                            // Let's use one of them to replace occurrences of `local`.
                            if let Bytecode::Assign(_, _, src, _) = bytecode[offset as usize] {
                                let src_coalesced = coalesceable_locals.get(&src).unwrap_or(&src);
                                // Special case: `local := src_coalesced`, where `src_coalesced` is available.
                                // Let's reuse the RHS to create a self-assignment, which can be optimized away.
                                if let Some(avail) = avail_locals.take(src_coalesced) {
                                    coalesceable_locals.insert(local, avail);
                                    continue;
                                }
                            }
                            // If none of the above special cases apply, let's reuse any available local.
                            let avail = avail_locals.pop_first().expect("non-empty");
                            coalesceable_locals.insert(local, avail);
                        }
                    }
                },
                End(local, _) => {
                    let local_type = target.get_local_type(local);
                    let avail_local = *coalesceable_locals.get(&local).unwrap_or(&local);
                    // `local` is no longer alive, so it can be reused.
                    avail_map.entry(local_type).or_default().insert(avail_local);
                },
            }
        }
        coalesceable_locals
    }

    /// Annotate the given function target with live interval events.
    fn annotate(target: &FunctionTarget) -> LiveIntervalAnnotation {
        let sorted_events = Self::sorted_live_interval_events(target);
        let mut mapping = BTreeMap::new();
        for event in sorted_events.into_iter() {
            let offset = event.offset();
            mapping.entry(offset).or_insert_with(Vec::new).push(event);
        }
        LiveIntervalAnnotation(mapping)
    }

    /// Obtain the transformed code of the given function target by reusing coalesceable locals.
    /// The resulting code can potentially leave several locals unused.
    fn transform(target: &FunctionTarget) -> Vec<Bytecode> {
        let coalesceable_locals = Self::coalesceable_locals(target);
        let mut new_code = vec![];
        let mut remapping_locals =
            |local: TempIndex| *coalesceable_locals.get(&local).unwrap_or(&local);
        for instr in target.get_bytecode() {
            let remapped_instr = instr.clone().remap_all_vars(target, &mut remapping_locals);
            new_code.push(remapped_instr);
        }
        new_code
    }

    /// Registers annotation formatter at the given function target.
    /// Helps with testing and debugging.
    pub fn register_formatters(target: &FunctionTarget) {
        target.register_annotation_formatter(Box::new(format_live_interval_annotation));
    }
}

impl FunctionTargetProcessor for VariableCoalescing {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        func_env: &FunctionEnv,
        mut data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if func_env.is_native() {
            return data;
        }
        let target = FunctionTarget::new(func_env, &data);
        if self.annotate {
            let annotation = Self::annotate(&target);
            data.annotations.set(annotation, true);
        } else {
            data.code = Self::transform(&target);
            // Annotations may no longer be valid after this transformation.
            // So remove them.
            data.annotations.clear();
        }
        data
    }

    fn name(&self) -> String {
        if self.annotate {
            "VariableCoalescingAnnotator".to_string()
        } else {
            "VariableCoalescingTransformer".to_string()
        }
    }
}

// ====================================================================
// Formatting functionality for live interval annotations

pub fn format_live_interval_annotation(
    target: &FunctionTarget,
    code_offset: CodeOffset,
) -> Option<String> {
    let LiveIntervalAnnotation(map) = target.get_annotations().get::<LiveIntervalAnnotation>()?;
    let events = map.get(&code_offset)?;
    let mut res = "events: ".to_string();
    res.push_str(
        &events
            .iter()
            .map(|event| {
                let local = {
                    let l = event.local();
                    let name = target.get_local_raw_name(l);
                    name.display(target.symbol_pool()).to_string()
                };
                let prefix = if matches!(event, LiveIntervalEvent::Begin(..)) {
                    "b:"
                } else {
                    "e:"
                };
                format!("{}{}", prefix, local)
            })
            .collect::<Vec<_>>()
            .join(", "),
    );
    Some(res)
}
