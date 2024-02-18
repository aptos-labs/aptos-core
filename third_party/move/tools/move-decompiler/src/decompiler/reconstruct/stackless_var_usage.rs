// Copyright (c) Verichains
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::{cell::RefCell, collections::HashMap};

use super::{
    super::cfg::{
        algo::blocks_stackless::AnnotatedBytecode,
        datastructs::{BasicBlock, CodeUnitBlock, HyperBlock},
        metadata::WithMetadata,
        StacklessBlockContent,
    },
    var_pipeline::{
        BranchMergeableVar, TimeDeltableVar, VarPipelineRunner, VarPipelineState,
        VarPipelineStateRef,
    },
};

pub trait VarUsageSnapshotWithDelta {
    type Delta;
}

#[derive(Clone, Debug)]
pub struct VarUsage {
    pub read_cnt: usize,
    pub write_cnt: usize,

    pub first_read: usize,
    pub first_write: usize,
    pub last_read: usize,
    pub last_write: usize,

    last_set_read_time_container: usize,
    last_set_write_time_container: usize,

    pub max_read_cnt_max_from_cfg: usize,
    pub should_keep_as_variable: bool,
}

impl Default for VarUsage {
    fn default() -> Self {
        Self {
            read_cnt: 0,
            write_cnt: 0,
            first_read: usize::MAX,
            first_write: usize::MAX,
            last_read: usize::MIN,
            last_write: usize::MIN,

            last_set_read_time_container: usize::MAX,
            last_set_write_time_container: usize::MAX,

            max_read_cnt_max_from_cfg: Default::default(),
            should_keep_as_variable: Default::default(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct VarUsageDelta {
    pub read_cnt: isize,
    pub write_cnt: isize,
}

impl VarUsageSnapshotWithDelta for VarUsage {
    type Delta = VarUsageDelta;
}

impl TimeDeltableVar for VarUsage {
    type VarDelta = VarUsageDelta;
    fn delta(&self, base: &Option<&Self>) -> Option<Self::VarDelta> {
        match base {
            Some(base) => Some(Self::VarDelta {
                read_cnt: self.read_cnt as isize - base.read_cnt as isize,
                write_cnt: self.write_cnt as isize - base.write_cnt as isize,
            }),

            None => Some(Self::VarDelta {
                read_cnt: self.read_cnt as isize,
                write_cnt: self.write_cnt as isize,
            }),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct VarUsageSnapshot<T: Default + Clone + VarUsageSnapshotWithDelta> {
    // snapshots usage excluding current block
    pub backward_run_pre: (usize, HashMap<usize, T>),
    // snapshots usage excluding current block
    pub forward_run_pre: (usize, HashMap<usize, T>),

    pub forward_run_post: HashMap<usize, T::Delta>,
}

impl VarUsage {
    fn add_read(&mut self, time: usize, time_container: usize) {
        self.read_cnt += 1;
        self.max_read_cnt_max_from_cfg += 1;
        self.last_read = self.last_read.max(time);
        self.first_read = self.first_read.min(time);
        self.last_set_read_time_container = time_container;
    }

    fn add_write(&mut self, time: usize, time_container: usize) {
        self.write_cnt += 1;
        self.last_write = self.last_write.max(time);
        self.first_write = self.first_write.min(time);
        self.last_set_write_time_container = time_container;
    }
}

impl BranchMergeableVar for VarUsage {
    fn merge_branch(&self, other: &Self, base: &Option<&Self>) -> Option<Self> {
        let (base_read_cnt, base_write_cnt, base_last_set_read, base_last_set_write) = match base {
            Some(base) => (
                base.read_cnt,
                base.write_cnt,
                base.last_set_read_time_container,
                base.last_set_write_time_container,
            ),

            None => (0, 0, usize::MAX, usize::MAX),
        };

        Some(Self {
            read_cnt: self.read_cnt + other.read_cnt - base_read_cnt,
            write_cnt: self.write_cnt + other.write_cnt - base_write_cnt,

            // these cannot be merged, each branch has its own time range
            first_read: Default::default(),
            last_read: Default::default(),
            first_write: Default::default(),
            last_write: Default::default(),

            last_set_read_time_container: base_last_set_read,
            last_set_write_time_container: base_last_set_write,

            should_keep_as_variable: self.should_keep_as_variable || other.should_keep_as_variable,
            max_read_cnt_max_from_cfg: self
                .max_read_cnt_max_from_cfg
                .max(other.max_read_cnt_max_from_cfg),
        })
    }
}

pub struct StacklessVarUsagePipeline {}

struct ForwardVisitorConfig {
    t: RefCell<usize>,
}
struct BackwardVisitorConfig {
    t: RefCell<usize>,
}

impl StacklessVarUsagePipeline {
    pub fn new() -> Self {
        Self {}
    }

    pub fn run(
        &self,
        unit: &mut WithMetadata<CodeUnitBlock<usize, StacklessBlockContent>>,
    ) -> Result<VarPipelineStateRef<VarUsage>, anyhow::Error> {
        self.run_unit(
            &BackwardVisitorConfig { t: RefCell::new(0) },
            &VarPipelineState::new().boxed(),
            unit,
        )?;

        self.run_unit(
            &ForwardVisitorConfig { t: RefCell::new(0) },
            &VarPipelineState::new().boxed(),
            unit,
        )
    }
}

impl VarPipelineRunner<BackwardVisitorConfig, VarUsage> for StacklessVarUsagePipeline {
    fn run_unit(
        &self,
        config: &BackwardVisitorConfig,
        state: &VarPipelineStateRef<VarUsage>,
        unit_block: &mut WithMetadata<CodeUnitBlock<usize, StacklessBlockContent>>,
    ) -> Result<VarPipelineStateRef<VarUsage>, anyhow::Error> {
        let original_t = *config.t.borrow();

        let mut state = state.copy_with_new_time();
        *config.t.borrow_mut() = 0;

        unit_block
            .meta_mut()
            .get_or_default::<VarUsageSnapshot<VarUsage>>()
            .backward_run_pre = (*config.t.borrow(), state.snapshot());

        for block in &mut unit_block.inner_mut().blocks.iter_mut().rev() {
            state = self.run_hyperblock(config, &state, block)?;
        }

        *config.t.borrow_mut() = original_t;

        Ok(state)
    }

    fn run_hyperblock(
        &self,
        config: &BackwardVisitorConfig,
        state: &VarPipelineStateRef<VarUsage>,
        hyper_block: &mut WithMetadata<HyperBlock<usize, StacklessBlockContent>>,
    ) -> Result<VarPipelineStateRef<VarUsage>, anyhow::Error> {
        let mut state = state.copy_as_ref();

        hyper_block
            .meta_mut()
            .get_or_default::<VarUsageSnapshot<VarUsage>>()
            .backward_run_pre = (*config.t.borrow(), state.snapshot());

        match hyper_block.inner_mut() {
            HyperBlock::ConnectedBlocks(blocks) => {
                for block in blocks.iter_mut().rev() {
                    state = self.run_basicblock(config, &state, block)?;
                }
            }

            HyperBlock::IfElseBlocks { if_unit, else_unit } => {
                let state_time_id = state.time_id();
                let t = *config.t.borrow();

                let state_t = self.run_unit(config, &state, if_unit.as_mut())?;
                let state_f = self.run_unit(config, &state, else_unit.as_mut())?;
                state = state
                    .merge_branches(vec![&state_t, &state_f], |s| {
                        update_rw_with_time_id_check(s, state_time_id, t)
                    })
                    .boxed();
            }

            HyperBlock::WhileBlocks { inner, outer, .. } => {
                let state_time_id = state.time_id();
                let t = *config.t.borrow();

                state = self.run_unit(config, &state, outer.as_mut())?;
                let state_i = self.run_unit(config, &state, inner.as_mut())?;
                state = state
                    .merge_branches(vec![&state_i], |s| {
                        update_rw_with_time_id_check(s, state_time_id, t)
                    })
                    .boxed();
            }
        };
        Ok(state)
    }

    fn run_basicblock(
        &self,
        config: &BackwardVisitorConfig,
        state: &VarPipelineStateRef<VarUsage>,
        basic_block: &mut WithMetadata<BasicBlock<usize, StacklessBlockContent>>,
    ) -> Result<VarPipelineStateRef<VarUsage>, anyhow::Error> {
        let mut state = if basic_block.inner().next.is_terminated() {
            state.new_initial()
        } else {
            state.copy_as_ref()
        };

        basic_block
            .meta_mut()
            .get_or_default::<VarUsageSnapshot<VarUsage>>()
            .backward_run_pre = (*config.t.borrow(), state.snapshot());

        for inst in basic_block.inner_mut().content.code.iter_mut().rev() {
            *config.t.borrow_mut() += 1;
            inst.meta_mut()
                .get_or_default::<VarUsageSnapshot<VarUsage>>()
                .backward_run_pre = (*config.t.borrow(), state.snapshot());
            update_state(inst, &mut state, false, *config.t.borrow());
        }

        Ok(state)
    }
}

impl VarPipelineRunner<ForwardVisitorConfig, VarUsage> for StacklessVarUsagePipeline {
    fn run_unit(
        &self,
        config: &ForwardVisitorConfig,
        state: &VarPipelineStateRef<VarUsage>,
        unit_block: &mut WithMetadata<CodeUnitBlock<usize, StacklessBlockContent>>,
    ) -> Result<VarPipelineStateRef<VarUsage>, anyhow::Error> {
        let original_t = *config.t.borrow();

        let mut state = state.copy_with_new_time();
        let original_state = state.copy_as_ref();
        *config.t.borrow_mut() = 0;

        unit_block
            .meta_mut()
            .get_or_default::<VarUsageSnapshot<VarUsage>>()
            .forward_run_pre = (*config.t.borrow(), state.snapshot());

        for block in &mut unit_block.inner_mut().blocks {
            state = self.run_hyperblock(config, &state, block)?;
        }

        unit_block
            .meta_mut()
            .get_or_default::<VarUsageSnapshot<VarUsage>>()
            .forward_run_post = state.delta(original_state.as_ref());

        *config.t.borrow_mut() = original_t;

        Ok(state)
    }

    fn run_hyperblock(
        &self,
        config: &ForwardVisitorConfig,
        state: &VarPipelineStateRef<VarUsage>,
        hyper_block: &mut WithMetadata<HyperBlock<usize, StacklessBlockContent>>,
    ) -> Result<VarPipelineStateRef<VarUsage>, anyhow::Error> {
        let original_state = state.copy_as_ref();
        let mut state = state.copy_as_ref();
        hyper_block
            .meta_mut()
            .get_or_default::<VarUsageSnapshot<VarUsage>>()
            .forward_run_pre = (*config.t.borrow(), state.snapshot());
        match hyper_block.inner_mut() {
            HyperBlock::ConnectedBlocks(blocks) => {
                for block in blocks.iter_mut() {
                    state = self.run_basicblock(config, &state, block)?;
                }
            }

            HyperBlock::IfElseBlocks { if_unit, else_unit } => {
                let state_time_id = state.time_id();
                let t = *config.t.borrow();
                let state_t = self.run_unit(config, &state, if_unit.as_mut())?;
                let state_f = self.run_unit(config, &state, else_unit.as_mut())?;
                state = state
                    .merge_branches(vec![&state_t, &state_f], |x| {
                        update_rw_with_time_id_check(x, state_time_id, t)
                    })
                    .boxed();
            }

            HyperBlock::WhileBlocks { inner, outer, .. } => {
                let state_time_id = state.time_id();
                let t = *config.t.borrow();

                let state_i = self.run_unit(config, &state, inner.as_mut())?;
                state = state
                    .merge_branches(vec![&state_i], |x| {
                        update_rw_with_time_id_check(x, state_time_id, t)
                    })
                    .boxed();

                state = self.run_unit(config, &state, outer.as_mut())?;
            }
        }

        hyper_block
            .meta_mut()
            .get_or_default::<VarUsageSnapshot<VarUsage>>()
            .forward_run_post = state.delta(original_state.as_ref());

        Ok(state)
    }

    fn run_basicblock(
        &self,
        config: &ForwardVisitorConfig,
        state: &VarPipelineStateRef<VarUsage>,
        basic_block: &mut WithMetadata<BasicBlock<usize, StacklessBlockContent>>,
    ) -> Result<VarPipelineStateRef<VarUsage>, anyhow::Error> {
        let original_state = state.copy_as_ref();
        let mut state: Box<VarPipelineState<VarUsage>> = state.copy_as_ref();

        basic_block
            .meta_mut()
            .get_or_default::<VarUsageSnapshot<VarUsage>>()
            .forward_run_pre = (*config.t.borrow(), state.snapshot());

        for inst in basic_block.inner_mut().content.code.iter_mut() {
            *config.t.borrow_mut() += 1;
            let prev_state = state.copy_as_ref();
            inst.meta_mut()
                .get_or_default::<VarUsageSnapshot<VarUsage>>()
                .forward_run_pre = (*config.t.borrow(), state.snapshot());
            update_state(inst, &mut state, true, *config.t.borrow());
            inst.meta_mut()
                .get_or_default::<VarUsageSnapshot<VarUsage>>()
                .forward_run_post = state.delta(prev_state.as_ref());
        }

        basic_block
            .meta_mut()
            .get_or_default::<VarUsageSnapshot<VarUsage>>()
            .forward_run_post = state.delta(original_state.as_ref());

        Ok(state)
    }
}

fn update_rw_with_time_id_check(s: &VarUsage, state_time_id: usize, t: usize) -> VarUsage {
    let mut s = s.clone();

    if s.last_set_read_time_container != state_time_id {
        s.first_read = t;
        s.last_read = t;
    } else {
        s.first_read = usize::MAX;
        s.last_read = usize::MIN;
    }

    if s.last_set_write_time_container != state_time_id {
        s.first_write = t;
        s.last_write = t;
    } else {
        s.first_write = usize::MAX;
        s.last_write = usize::MIN;
    }

    s
}
fn update_state(
    inst: &AnnotatedBytecode,
    state: &mut Box<VarPipelineState<VarUsage>>,
    running_forward: bool,
    t: usize,
) {
    use move_stackless_bytecode::stackless_bytecode::Bytecode::*;
    let state_time_id = state.time_id();
    match &inst.bytecode {
        Assign(_, dst, src, _) => {
            let svar = state.get_or_default(src);
            svar.add_read(t, state_time_id);
            let dvar = state.get_or_default(dst);
            dvar.add_write(t, state_time_id);
        }

        Call(_, dsts, op, srcs, _) => {
            use move_stackless_bytecode::stackless_bytecode::Operation;
            if matches!(op, Operation::Drop | Operation::Release) {
                for s in srcs {
                    let svar = state.get_or_default(s);
                    svar.should_keep_as_variable = true;
                }
                return;
            }
            for dst in dsts {
                let dvar = state.get_or_default(dst);
                dvar.add_write(t, state_time_id);
            }
            for &src in srcs {
                let svar = state.get_or_default(&src);
                svar.add_read(t, state_time_id);
            }
            if running_forward {
                if matches!(op, Operation::Pack(..) | Operation::Unpack(..)) {
                    for dst in dsts {
                        let dvar = state.get_or_default(dst);
                        dvar.should_keep_as_variable = true;
                    }
                }
                if matches!(op, Operation::BorrowLoc) {
                    for src in srcs {
                        let svar = state.get_or_default(src);
                        svar.should_keep_as_variable = true;
                    }
                }
            }
        }

        Ret(_, srcs) => {
            for &src in srcs {
                let svar = state.get_or_default(&src);
                svar.add_read(t, state_time_id);
            }
        }

        Branch(_, _, _, src) | Abort(_, src) => {
            let svar = state.get_or_default(src);
            svar.add_read(t, state_time_id);
        }

        Load(_, dst, _) => {
            let dvar = state.get_or_default(dst);
            dvar.add_write(t, state_time_id);
        }

        Jump(..) | Label(..) | Nop(..) | SaveMem(..) | SaveSpecVar(..) | Prop(..) => {}
    }
}
