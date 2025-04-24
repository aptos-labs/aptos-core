use crate::{framework::NodeId, Slot};

pub trait LeaderSchedule: Fn(Slot) -> NodeId + Clone + Send + Sync + 'static {}

impl<S> LeaderSchedule for S where S: Fn(Slot) -> NodeId + Clone + Send + Sync + 'static {}

pub fn round_robin(n_nodes: usize) -> impl LeaderSchedule {
    move |slot| slot as usize % n_nodes
}
