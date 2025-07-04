use crate::network_interface::ConsensusMsg;
use crate::metrics_safety_rules::MetricsSafetyRules;
use crate::network::NetworkSender;
use aptos_consensus_types::{block_data::BlockData,common::{Author, Round, Payload},block::{Block}, block_retrieval::{BlockRetrievalRequest,BlockRetrievalResponse,BlockRetrievalStatus}, epoch_retrieval::EpochRetrievalRequest, proposal_msg::ProposalMsg, quorum_cert::QuorumCert, sync_info::SyncInfo, vote::Vote, vote_data::VoteData, vote_msg::VoteMsg};
use std::sync::{Arc, Once, OnceLock};
use aptos_infallible::{checked, Mutex};
use std::thread;
use aptos_logger::info;

pub trait StateModelLike: Send + Sync {
    fn on_new_msg(&mut self, msg: &ConsensusMsg);
    fn current_state(&self) -> String;
}

pub trait FuzzHook: Send + Sync {
    fn start_active_fuzzer(&self, network: NetworkSender, state_model: Arc<Mutex<Box<dyn StateModelLike>>>, safety_rules: Arc<Mutex<MetricsSafetyRules>>, author: Author);
    fn mutate_consensus_msg(&self, msg: crate::network_interface::ConsensusMsg) -> ConsensusMsg;
}

static mut GLOBAL_FUZZ_HOOK: Option<Box<dyn FuzzHook>> = None;

static GLOBAL_STATE_MODEL: OnceLock<Arc<Mutex<Box<dyn StateModelLike>>>> = OnceLock::new();

pub fn register_state_model(model: Box<dyn StateModelLike>) {
    GLOBAL_STATE_MODEL
        .set(Arc::new(Mutex::new(model)))
        .unwrap_or_else(|_| panic!("StateModel already registered"));
}

pub fn register_fuzz_hook(hook: Box<dyn FuzzHook>) {
    unsafe {
        GLOBAL_FUZZ_HOOK = Some(hook);
    }
}

pub fn run_fuzzer(copy_network: NetworkSender,
    state_model: Arc<Mutex<Box<dyn StateModelLike>>>,
    safety_rules_container_new: Arc<Mutex<MetricsSafetyRules>>,
    author: Author) {
    unsafe {
        static INIT: Once = Once::new();
        if let Some(ref h) = GLOBAL_FUZZ_HOOK {
            INIT.call_once(|| {
                thread::spawn(move || {
                    let author_copy = author.clone();
                    info!("\n\n @@@@ RAPTURE enter the EPOCH time! Ready to Start Fuzzing! @@@@ \n\n");
                    h.start_active_fuzzer(copy_network, state_model, safety_rules_container_new, author_copy);
                    info!("\n\n @@@@ Finish RAPTURE Fuzzer! @@@@ \n\n");
                });
            });
        }
    }
}

pub fn consensus_msg_mutate(msg: crate::network_interface::ConsensusMsg) -> ConsensusMsg{
    unsafe {
        if let Some(ref h) = GLOBAL_FUZZ_HOOK {
            return h.mutate_consensus_msg(msg);
        }
        return msg;
    }
}

pub fn get_state_model_arc() -> Option<Arc<Mutex<Box<dyn StateModelLike>>>> {
    GLOBAL_STATE_MODEL.get().cloned()
} 