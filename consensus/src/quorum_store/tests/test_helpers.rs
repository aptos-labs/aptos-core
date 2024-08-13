use crate::liveness::proposer_election::TNextProposersProvider;
use aptos_consensus_types::common::{Author, Round};

pub struct MockProposersProvider {}

impl MockProposersProvider {
    pub fn new() -> Self {
        Self {}
    }
}

impl TNextProposersProvider for MockProposersProvider {
    fn get_next_proposers(&self, _current_round: Round, _count: u64) -> Vec<Author> {
        Vec::new()
    }
}
