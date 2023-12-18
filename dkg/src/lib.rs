use aptos_config::network_id::{NetworkId, PeerNetworkId};
use aptos_network::application::{
    error::Error,
    interface::{NetworkClient, NetworkClientInterface, NetworkServiceEvents},
};
use aptos_types::PeerId;
use aptos_validator_transaction_pool as vtxn_pool;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::runtime::Runtime;

#[allow(clippy::let_and_return)]
pub fn start_dkg_runtime(
    _network_client: NetworkClient<DKGMsg>,
    _network_service_events: NetworkServiceEvents<DKGMsg>,
    _vtxn_pool_writer: vtxn_pool::SingleTopicWriteClient,
    _vtxn_pulled_rx: vtxn_pool::PullNotificationReceiver,
) -> Runtime {
    let runtime = aptos_runtimes::spawn_named_runtime("dkg".into(), Some(4));
    runtime
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DKGMsg {}

#[derive(Clone)]
pub struct DKGNetworkClient<NetworkClient> {
    network_client: NetworkClient,
}

impl<NetworkClient: NetworkClientInterface<DKGMsg>> DKGNetworkClient<NetworkClient> {
    pub fn new(network_client: NetworkClient) -> Self {
        Self { network_client }
    }

    pub async fn send_rpc(
        &self,
        peer: PeerId,
        message: DKGMsg,
        rpc_timeout: Duration,
    ) -> Result<DKGMsg, Error> {
        let peer_network_id = PeerNetworkId::new(NetworkId::Validator, peer);
        self.network_client
            .send_to_peer_rpc(message, rpc_timeout, peer_network_id)
            .await
    }
}
