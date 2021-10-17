// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    experimental::{
        buffer_manager::{create_channel, Sender},
        persisting_phase::{PersistingPhase, PersistingRequest},
        pipeline_phase::PipelinePhase,
    },
    state_replication::StateComputer,
    test_utils::EmptyStateComputer,
};
use std::sync::Arc;

pub fn prepare_persisting_phase() -> PersistingPhase {
    let execution_proxy = Arc::new(EmptyStateComputer);
    PersistingPhase::new(execution_proxy)
}

pub fn prepare_persisting_pipeline_with_state_computer(
    state_computer: Arc<dyn StateComputer>,
) -> (Sender<PersistingRequest>, PipelinePhase<PersistingPhase>) {
    let (in_channel_tx, in_channel_rx) = create_channel::<PersistingRequest>();
    let persisting_phase = PersistingPhase::new(state_computer);

    let persisting_phase_pipeline =
        PipelinePhase::new(in_channel_rx, None, Box::new(persisting_phase));

    (in_channel_tx, persisting_phase_pipeline)
}

pub fn prepare_persisting_pipeline() -> (Sender<PersistingRequest>, PipelinePhase<PersistingPhase>)
{
    let state_computer = Arc::new(EmptyStateComputer);
    prepare_persisting_pipeline_with_state_computer(state_computer)
}
