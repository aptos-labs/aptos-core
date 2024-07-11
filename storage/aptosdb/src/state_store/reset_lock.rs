use super::buffered_state::BufferedState;
use super::{StateDb, StateStore};
use aptos_infallible::{Mutex, MutexGuard};
use aptos_scratchpad::SmtAncestors;
use aptos_types::state_store::state_value::StateValue;

use std::sync::Arc;

/// Locks the buffered state. This can be used to implement atomic
/// reset together with a ledger database commit.
pub struct ResetLock<'a> {
    buffered_state: MutexGuard<'a, BufferedState>,
    smt_ancestors: MutexGuard<'a, SmtAncestors<StateValue>>,
    state_db: &'a Arc<StateDb>,
    buffered_state_target_items: usize,
}

impl<'a> ResetLock<'a> {
    pub(super) fn new(
        buffered_state: &'a Mutex<BufferedState>,
        smt_ancestors: &'a Mutex<SmtAncestors<StateValue>>,
        state_db: &'a Arc<StateDb>,
        buffered_state_target_items: usize,
    ) -> Self {
        Self {
            buffered_state: buffered_state.lock(),
            smt_ancestors: smt_ancestors.lock(),
            state_db,
            buffered_state_target_items,
        }
    }

    pub fn reset(mut self) {
        let (buffered_state, smt_ancestors) =
            StateStore::create_buffered_state_from_latest_snapshot(
                self.state_db,
                self.buffered_state_target_items,
                false,
                true,
            )
            .expect("buffered state creation failed.");
        *self.buffered_state = buffered_state;
        *self.smt_ancestors = smt_ancestors;
    }
}
