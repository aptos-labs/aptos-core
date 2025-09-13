// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    function_target::{FunctionData, FunctionTarget},
    print_targets_with_annotations_for_test,
    stackless_bytecode_generator::StacklessBytecodeGenerator,
    stackless_control_flow_graph::generate_cfg_in_dot_format,
};
use core::fmt;
use itertools::{Either, Itertools};
use log::debug;
use move_model::model::{FunId, FunctionEnv, GlobalEnv, QualifiedId};
use petgraph::graph::DiGraph;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Formatter,
    fs,
};

/// A data structure which holds data for multiple function targets, and allows to
/// manipulate them as part of a transformation pipeline.
#[derive(Debug, Default)]
pub struct FunctionTargetsHolder {
    targets: BTreeMap<QualifiedId<FunId>, BTreeMap<FunctionVariant, FunctionData>>,
}

/// Describes a function verification flavor.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum VerificationFlavor {
    Regular,
    Instantiated(usize),
    Inconsistency(Box<VerificationFlavor>),
}

impl std::fmt::Display for VerificationFlavor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VerificationFlavor::Regular => write!(f, ""),
            VerificationFlavor::Instantiated(index) => {
                write!(f, "instantiated_{}", index)
            },
            VerificationFlavor::Inconsistency(flavor) => write!(f, "inconsistency_{}", flavor),
        }
    }
}

/// Describes a function target variant.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FunctionVariant {
    /// The baseline variant which was created from the original Move bytecode and is then
    /// subject of multiple transformations.
    Baseline,
    /// A variant which is instrumented for verification. Only functions which are target
    /// of verification have one of those. There can be multiple verification variants,
    /// each identified by a unique flavor.
    Verification(VerificationFlavor),
}

impl FunctionVariant {
    pub fn is_verified(&self) -> bool {
        matches!(self, FunctionVariant::Verification(..))
    }
}

impl std::fmt::Display for FunctionVariant {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use FunctionVariant::*;
        match self {
            Baseline => write!(f, "baseline"),
            Verification(VerificationFlavor::Regular) => write!(f, "verification"),
            Verification(v) => write!(f, "verification[{}]", v),
        }
    }
}

/// A trait describing a function target processor.
pub trait FunctionTargetProcessor {
    /// Processes a function variant. Takes as parameter a target holder which can be mutated, the
    /// env of the function being processed, and the target data. During the time the processor is
    /// called, the target data is removed from the holder, and added back once transformation
    /// has finished. This allows the processor to take ownership on the target data.
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        _fun_env: &FunctionEnv,
        _data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        unimplemented!()
    }

    /// Same as `process` but can return None to indicate that the function variant is
    /// removed. By default, this maps to `Some(self.process(..))`. One needs to implement
    /// either this function or `process`.
    fn process_and_maybe_remove(
        &self,
        targets: &mut FunctionTargetsHolder,
        func_env: &FunctionEnv,
        data: FunctionData,
        scc_opt: Option<&[FunctionEnv]>,
    ) -> Option<FunctionData> {
        Some(self.process(targets, func_env, data, scc_opt))
    }

    /// Returns a name for this processor. This should be suitable as a file suffix.
    fn name(&self) -> String;

    /// A function which is called once before any `process` call is issued.
    fn initialize(&self, _env: &GlobalEnv, _targets: &mut FunctionTargetsHolder) {}

    /// A function which is called once after the last `process` call.
    fn finalize(&self, _env: &GlobalEnv, _targets: &mut FunctionTargetsHolder) {}

    /// A function which can be implemented to indicate that instead of a sequence of initialize,
    /// process, and finalize, this processor has a single `run` function for the analysis of the
    /// whole set of functions.
    fn is_single_run(&self) -> bool {
        false
    }

    /// To be implemented if `is_single_run()` is true.
    fn run(&self, _env: &GlobalEnv, _targets: &mut FunctionTargetsHolder) {
        unimplemented!()
    }

    /// A function which creates a dump of the processors results, for debugging.
    fn dump_result(
        &self,
        _f: &mut Formatter<'_>,
        _env: &GlobalEnv,
        _targets: &FunctionTargetsHolder,
    ) -> fmt::Result {
        Ok(())
    }
}

pub struct ProcessorResultDisplay<'a> {
    pub env: &'a GlobalEnv,
    pub targets: &'a FunctionTargetsHolder,
    pub processor: &'a dyn FunctionTargetProcessor,
}

impl fmt::Display for ProcessorResultDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.processor.dump_result(f, self.env, self.targets)
    }
}

/// A processing pipeline for function targets.
#[derive(Default)]
pub struct FunctionTargetPipeline {
    processors: Vec<Box<dyn FunctionTargetProcessor>>,
    /// Indices of processors which have been marked to not dump their target annotations.
    no_annotation_dump_indices: BTreeSet<usize>,
}

impl FunctionTargetsHolder {
    /// Get an iterator for all functions this holder.
    pub fn get_funs(&self) -> impl Iterator<Item = QualifiedId<FunId>> + '_ {
        self.targets.keys().cloned()
    }

    /// Gets an iterator for all functions and variants in this holder.
    pub fn get_funs_and_variants(
        &self,
    ) -> impl Iterator<Item = (QualifiedId<FunId>, FunctionVariant)> + '_ {
        self.targets
            .iter()
            .flat_map(|(id, vs)| vs.keys().map(move |v| (*id, v.clone())))
    }

    /// Adds a new function target. The target will be initialized from the Move byte code.
    pub fn add_target(&mut self, func_env: &FunctionEnv<'_>) {
        // Skip inlined functions, they do not have associated bytecode.
        if func_env.is_inline() {
            return;
        }
        let generator = StacklessBytecodeGenerator::new(func_env);
        let data = generator.generate_function();
        self.targets
            .entry(func_env.get_qualified_id())
            .or_default()
            .insert(FunctionVariant::Baseline, data);
    }

    /// Gets a function target for read-only consumption, for the given variant.
    pub fn get_target<'env>(
        &'env self,
        func_env: &'env FunctionEnv<'env>,
        variant: &FunctionVariant,
    ) -> FunctionTarget<'env> {
        let data = self
            .get_data(&func_env.get_qualified_id(), variant)
            .unwrap_or_else(|| {
                panic!(
                    "expected function target: {} ({:?})",
                    func_env.get_full_name_str(),
                    variant
                )
            });
        FunctionTarget::new(func_env, data)
    }

    pub fn has_target(&self, func_env: &FunctionEnv<'_>, variant: &FunctionVariant) -> bool {
        self.get_data(&func_env.get_qualified_id(), variant)
            .is_some()
    }

    /// Gets all available variants for function.
    pub fn get_target_variants(&self, func_env: &FunctionEnv<'_>) -> Vec<FunctionVariant> {
        self.targets
            .get(&func_env.get_qualified_id())
            .expect("function targets exist")
            .keys()
            .cloned()
            .collect_vec()
    }

    /// Gets targets for all available variants.
    pub fn get_targets<'env>(
        &'env self,
        func_env: &'env FunctionEnv<'env>,
    ) -> Vec<(FunctionVariant, FunctionTarget<'env>)> {
        assert!(
            !func_env.is_inline(),
            "attempt to get bytecode function target for inline function"
        );
        self.targets
            .get(&func_env.get_qualified_id())
            .expect("function targets exist")
            .iter()
            .map(|(v, d)| (v.clone(), FunctionTarget::new(func_env, d)))
            .collect_vec()
    }

    /// Gets function data for a variant.
    pub fn get_data(
        &self,
        id: &QualifiedId<FunId>,
        variant: &FunctionVariant,
    ) -> Option<&FunctionData> {
        self.targets.get(id).and_then(|vs| vs.get(variant))
    }

    /// Gets mutable function data for a variant.
    pub fn get_data_mut(
        &mut self,
        id: &QualifiedId<FunId>,
        variant: &FunctionVariant,
    ) -> Option<&mut FunctionData> {
        self.targets.get_mut(id).and_then(|vs| vs.get_mut(variant))
    }

    /// Removes function data for a variant.
    pub fn remove_target_data(
        &mut self,
        id: &QualifiedId<FunId>,
        variant: &FunctionVariant,
    ) -> FunctionData {
        self.targets
            .get_mut(id)
            .expect("function target exists")
            .remove(variant)
            .expect("variant exists")
    }

    /// Sets function data for a function's variant.
    pub fn insert_target_data(
        &mut self,
        id: &QualifiedId<FunId>,
        variant: FunctionVariant,
        data: FunctionData,
    ) {
        self.targets.entry(*id).or_default().insert(variant, data);
    }

    /// Processes the function target data for given function.
    fn process(
        &mut self,
        func_env: &FunctionEnv,
        processor: &dyn FunctionTargetProcessor,
        scc_opt: Option<&[FunctionEnv]>,
    ) {
        let id = func_env.get_qualified_id();
        for variant in self.get_target_variants(func_env) {
            // Remove data so we can own it.
            let data = self.remove_target_data(&id, &variant);
            if let Some(processed_data) =
                processor.process_and_maybe_remove(self, func_env, data, scc_opt)
            {
                // Put back processed data.
                self.insert_target_data(&id, variant, processed_data);
            }
        }
    }

    pub fn compute_move_functions_size(&self) -> BTreeMap<QualifiedId<FunId>, (usize, usize)> {
        self.targets
            .iter()
            .filter_map(|(fid, variants)| {
                let baseline_function = variants.get(&FunctionVariant::Baseline)?;
                Some((
                    *fid,
                    (
                        baseline_function.code.len(),
                        baseline_function.local_types.len(),
                    ),
                ))
            })
            .collect()
    }
}

impl FunctionTargetPipeline {
    pub fn is_empty(&self) -> bool {
        self.processors.is_empty()
    }

    pub fn processor_count(&self) -> usize {
        self.processors.len()
    }

    /// Cuts down the pipeline to stop after the given named processor
    pub fn stop_after_for_testing(&mut self, name: &str) {
        for i in 0..self.processor_count() {
            if self.processors[i].name() == name {
                for _ in i + 1..self.processor_count() {
                    self.processors.remove(i + 1);
                }
                return;
            }
        }
        panic!("no processor named `{}`", name)
    }

    /// Adds a processor to this pipeline. Processor will be called in the order they have been
    /// added.
    pub fn add_processor(&mut self, processor: Box<dyn FunctionTargetProcessor>) {
        self.processors.push(processor)
    }

    /// Similar to `add_processor`,
    /// but additionally records that we should not dump its target annotations.
    pub fn add_processor_without_annotation_dump(
        &mut self,
        processor: Box<dyn FunctionTargetProcessor>,
    ) {
        self.no_annotation_dump_indices
            .insert(self.processors.len());
        self.processors.push(processor)
    }

    /// Returns true if the processor at `index` should not have its target annotations dumped.
    /// `index` is 1-based, similar to `hook_after_each_processor`.
    pub fn should_dump_target_annotations(&self, index: usize) -> bool {
        !self.no_annotation_dump_indices.contains(&(index - 1))
    }

    /// Gets the last processor in the pipeline, for testing.
    pub fn last_processor(&self) -> &dyn FunctionTargetProcessor {
        self.processors
            .iter()
            .last()
            .expect("pipeline not empty")
            .as_ref()
    }

    /// Build the call graph.
    /// Nodes of this call graph are qualified function ids.
    /// An edge A -> B in the call graph means that function A calls function B.
    fn build_call_graph(
        env: &GlobalEnv,
        targets: &FunctionTargetsHolder,
    ) -> DiGraph<QualifiedId<FunId>, ()> {
        let mut graph = DiGraph::new();
        let mut nodes = BTreeMap::new();
        for fun_id in targets.get_funs() {
            let node_idx = graph.add_node(fun_id);
            nodes.insert(fun_id, node_idx);
        }
        for fun_id in targets.get_funs() {
            let src_idx = nodes.get(&fun_id).unwrap();
            let fun_env = env.get_function(fun_id);
            for callee in fun_env
                .get_used_functions()
                .expect("called functions must be computed")
            {
                let dst_idx = nodes
                    .get(callee)
                    .expect("callee is not in function targets");
                graph.add_edge(*src_idx, *dst_idx, ());
            }
        }
        graph
    }

    /// Sort the call graph formed by the given `targets` in reverse topological order.
    /// The returned vector contains either:
    /// - a function id, if the function is not recursive, or only self-recursive.
    /// - a vector of function ids, if those functions are mutually recursive; this vector
    ///   is guaranteed to have at least two elements.
    pub fn sort_in_reverse_topological_order(
        env: &GlobalEnv,
        targets: &FunctionTargetsHolder,
    ) -> Vec<Either<QualifiedId<FunId>, Vec<QualifiedId<FunId>>>> {
        let graph = Self::build_call_graph(env, targets);
        // Tarjan's algorithm returns SCCs in reverse topological order.
        petgraph::algo::tarjan_scc(&graph)
            .iter()
            .map(|scc| {
                match scc.as_slice() {
                    [] => panic!("ICE: scc entry must not be empty"),
                    [node_idx] => {
                        // If the SCC has only one node, it is not recursive, or is only self-recursive.
                        Either::Left(graph[*node_idx])
                    },
                    _ => Either::Right(scc.iter().map(|node_idx| graph[*node_idx]).collect_vec()),
                }
            })
            .collect_vec()
    }

    /// Runs the pipeline on all functions in the targets holder. Processors are run on each
    /// individual function in breadth-first fashion; i.e. a processor can expect that processors
    /// preceding it in the pipeline have been executed for all functions before it is called.
    /// `hook_before_pipeline` is called before the pipeline is run, and `hook_after_each_processor`
    /// is called after each processor in the pipeline has been run on all functions.
    /// If `hook_after_each_processor` returns false, the pipeline is stopped.
    /// Note that `hook_after_each_processor` is called with index starting at 1.
    pub fn run_with_hook<Before, AfterEach>(
        &self,
        env: &GlobalEnv,
        targets: &mut FunctionTargetsHolder,
        hook_before_pipeline: Before,
        hook_after_each_processor: AfterEach,
    ) where
        Before: Fn(&FunctionTargetsHolder),
        AfterEach: Fn(usize, &dyn FunctionTargetProcessor, &FunctionTargetsHolder) -> bool,
    {
        let rev_topo_order = Self::sort_in_reverse_topological_order(env, targets);
        debug!("transforming bytecode");
        hook_before_pipeline(targets);
        for (step_count, processor) in self.processors.iter().enumerate() {
            if processor.is_single_run() {
                processor.run(env, targets);
            } else {
                processor.initialize(env, targets);
                for item in &rev_topo_order {
                    match item {
                        Either::Left(fid) => {
                            let func_env = env.get_function(*fid);
                            targets.process(&func_env, processor.as_ref(), None);
                        },
                        Either::Right(scc) => 'fixedpoint: loop {
                            let scc_env: Vec<_> =
                                scc.iter().map(|fid| env.get_function(*fid)).collect();
                            for fid in scc {
                                let func_env = env.get_function(*fid);
                                targets.process(&func_env, processor.as_ref(), Some(&scc_env));
                            }

                            // check for fixedpoint in summaries
                            for fid in scc {
                                let func_env = env.get_function(*fid);
                                if func_env.is_inline() {
                                    continue;
                                }
                                for (_, target) in targets.get_targets(&func_env) {
                                    if !target.data.annotations.reached_fixedpoint() {
                                        continue 'fixedpoint;
                                    }
                                }
                            }
                            // fixedpoint reached when execution hits this line
                            break 'fixedpoint;
                        },
                    }
                }
                processor.finalize(env, targets);
            }
            if !hook_after_each_processor(step_count + 1, processor.as_ref(), targets) {
                break;
            }
        }
    }

    /// Run the pipeline on all functions in the targets holder, with no hooks in effect
    pub fn run(&self, env: &GlobalEnv, targets: &mut FunctionTargetsHolder) {
        self.run_with_hook(env, targets, |_| {}, |_, _, _| true)
    }

    /// Runs the pipeline on all functions in the targets holder, and dump the bytecode via `log` before the
    /// pipeline as well as after each processor pass, identifying it by `dump_base_name`. If `dump_cfg` is set,
    /// dump the per-function control-flow graph (in dot format) to a file, using the given base name.
    /// `continue_to_next_processor` determines whether the pipeline should continue to the next processor.
    pub fn run_with_dump(
        &self,
        env: &GlobalEnv,
        targets: &mut FunctionTargetsHolder,
        dump_base_name: &str,
        dump_cfg: bool,
        register_annotations: &impl Fn(&FunctionTarget),
        continue_to_next_processor: impl Fn() -> bool,
    ) {
        self.run_with_hook(
            env,
            targets,
            |holders| {
                Self::debug_dump(
                    dump_base_name,
                    0,
                    "stackless",
                    &Self::get_pre_pipeline_dump(env, holders, /*verbose*/ true),
                )
            },
            |step_count, processor, holders| {
                let suffix = processor.name();
                Self::debug_dump(
                    dump_base_name,
                    step_count,
                    &suffix,
                    &Self::get_per_processor_dump(
                        env,
                        holders,
                        processor,
                        register_annotations,
                        /*verbose*/ true,
                    ),
                );
                if dump_cfg {
                    Self::dump_cfg(env, holders, dump_base_name, step_count, &suffix);
                }
                continue_to_next_processor()
            },
        );
    }

    fn print_targets(
        env: &GlobalEnv,
        name: &str,
        targets: &FunctionTargetsHolder,
        register_annotations: &impl Fn(&FunctionTarget),
        verbose: bool,
    ) -> String {
        print_targets_with_annotations_for_test(
            env,
            &format!("after processor `{}`", name),
            targets,
            register_annotations,
            verbose,
        )
    }

    fn get_pre_pipeline_dump(
        env: &GlobalEnv,
        targets: &FunctionTargetsHolder,
        verbose: bool,
    ) -> String {
        Self::print_targets(env, "stackless", targets, &|_| {}, verbose)
    }

    fn get_per_processor_dump(
        env: &GlobalEnv,
        targets: &FunctionTargetsHolder,
        processor: &dyn FunctionTargetProcessor,
        register_annotations: &impl Fn(&FunctionTarget),
        verbose: bool,
    ) -> String {
        let mut dump = format!("{}", ProcessorResultDisplay {
            env,
            targets,
            processor,
        });
        if !processor.is_single_run() {
            if !dump.is_empty() {
                dump = format!("\n\n{}", dump);
            }
            dump.push_str(&Self::print_targets(
                env,
                &processor.name(),
                targets,
                register_annotations,
                verbose,
            ));
        }
        dump
    }

    fn debug_dump(base_name: &str, step_count: usize, suffix: &str, content: &str) {
        let name = format!("bytecode of {}#step{}_{}", base_name, step_count, suffix);
        debug!("{}:\n{}\n", name, content.trim())
    }

    /// Generate dot files for control-flow graphs.
    fn dump_cfg(
        env: &GlobalEnv,
        targets: &FunctionTargetsHolder,
        base_name: &str,
        step_count: usize,
        suffix: &str,
    ) {
        for (fun_id, variants) in &targets.targets {
            let func_env = env.get_function(*fun_id);
            let func_name = func_env.get_full_name_str();
            let func_name = func_name.replace("::", "__");
            for (variant, data) in variants {
                if !data.code.is_empty() {
                    let dot_file = format!(
                        "{}_{}_{}_{}_{}_cfg.dot",
                        base_name, step_count, suffix, func_name, variant
                    );
                    debug!("generating dot graph for cfg in `{}`", dot_file);
                    let func_target = FunctionTarget::new(&func_env, data);
                    let dot_graph = generate_cfg_in_dot_format(&func_target, true);
                    fs::write(&dot_file, dot_graph).expect("generating dot file for CFG");
                }
            }
        }
    }
}
