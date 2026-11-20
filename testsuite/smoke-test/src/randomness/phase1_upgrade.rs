// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Phase-1 upgrades retain RandDB. Set RANDDB_UPGRADE_OLD_BIN and RANDDB_UPGRADE_NEW_BIN
//! to use prebuilt binaries; otherwise Forge builds the pinned pre-PR revision and workspace.

use crate::{
    randomness::{
        decrypt_key_map, e2e_basic_consumption::publish_on_chain_dice_module, get_current_version,
        script_to_enable_main_logic, verify_randomness,
    },
    utils::get_on_chain_resource,
};
use aptos::{common::types::GasOptions, test::CliTestFramework};
use aptos_crypto::HashValue;
use aptos_dkg::weighted_vuf::traits::WeightedVUF;
use aptos_forge::{
    ActiveNodesGuard, Factory, LocalFactory, LocalNode, LocalSwarm, LocalVersion, Node, NodeExt,
    Swarm, SwarmExt, Version,
};
use aptos_infallible::Mutex;
use aptos_logger::info;
use aptos_move_cli::MemberId;
use aptos_schemadb::{
    schema::{KeyCodec, Schema, ValueCodec},
    Options, DB,
};
use aptos_types::{
    account_address::AccountAddress,
    dkg::{DKGSessionState, DKGState, DKGTrait, DefaultDKG},
    on_chain_config::{OnChainChunkyDKGConfig, OnChainRandomnessConfig},
    randomness::{WvufPP, APK, ASK, WVUF},
};
use once_cell::sync::Lazy;
use rand::{
    rngs::{OsRng, StdRng},
    SeedableRng,
};
use std::{
    collections::{HashMap, HashSet},
    env,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use tokio::time::Instant;

const NUM_VALIDATORS: usize = 5;
const TIMEOUT: Duration = Duration::from_secs(90);
const LEGACY_REVISION: &str = "7b23ba0a947166c6c2fccc32608a345942f209a3";

static FACTORY: Lazy<LocalFactory> = Lazy::new(|| {
    match (
        env::var_os("RANDDB_UPGRADE_OLD_BIN"),
        env::var_os("RANDDB_UPGRADE_NEW_BIN"),
    ) {
        (Some(old), Some(new)) => {
            let versions = [
                (0, LEGACY_REVISION, PathBuf::from(old)),
                (1, "phase1", PathBuf::from(new)),
            ]
            .into_iter()
            .map(|(order, name, path)| {
                assert!(
                    path.is_file(),
                    "missing validator binary: {}",
                    path.display()
                );
                let version = Version::new(order, name.to_owned());
                (version.clone(), LocalVersion::new(path, version))
            })
            .collect();
            LocalFactory::new(versions, None)
        },
        (None, None) => LocalFactory::with_revision_and_workspace(LEGACY_REVISION).unwrap(),
        _ => panic!("set both RANDDB_UPGRADE_OLD_BIN and RANDDB_UPGRADE_NEW_BIN"),
    }
});

// Read the on-disk format without exposing consensus's private RandDB module.
// These readers are only used while the validator process is stopped.
#[derive(Debug)]
struct KeyPairSnapshot;

impl Schema for KeyPairSnapshot {
    type Key = ();
    type Value = (u64, Vec<u8>);

    const COLUMN_FAMILY_NAME: &'static str = "key_pair";
}

impl KeyCodec<KeyPairSnapshot> for () {
    fn encode_key(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_key(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl ValueCodec<KeyPairSnapshot> for (u64, Vec<u8>) {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

#[derive(Debug)]
struct CertifiedDataSnapshot;

impl Schema for CertifiedDataSnapshot {
    type Key = Vec<u8>;
    type Value = Vec<u8>;

    const COLUMN_FAMILY_NAME: &'static str = "certified_aug_data";
}

impl KeyCodec<CertifiedDataSnapshot> for Vec<u8> {
    fn encode_key(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self.clone())
    }

    fn decode_key(data: &[u8]) -> anyhow::Result<Self> {
        Ok(data.to_vec())
    }
}

impl ValueCodec<CertifiedDataSnapshot> for Vec<u8> {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self.clone())
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        Ok(data.to_vec())
    }
}

#[derive(PartialEq)]
struct RandDbSnapshot {
    key: Vec<u8>,
    certificates: HashMap<AccountAddress, Vec<u8>>,
}

fn read_rand_db(path: &Path, epoch: u64) -> RandDbSnapshot {
    let db = DB::open_readonly(
        path,
        "randdb_phase1_upgrade",
        vec!["key_pair", "aug_data", "certified_aug_data"],
        Options::default(),
    )
    .unwrap();
    let (key_epoch, key) = db.get::<KeyPairSnapshot>(&()).unwrap().unwrap();
    assert_eq!(key_epoch, epoch);
    let mut iter = db.iter::<CertifiedDataSnapshot>().unwrap();
    iter.seek_to_first();
    let certificates = iter
        .map(|entry| {
            let (key, value) = entry.unwrap();
            let (cert_epoch, author): (u64, AccountAddress) = bcs::from_bytes(&key).unwrap();
            assert_eq!(cert_epoch, epoch);
            (author, value)
        })
        .collect();
    RandDbSnapshot { key, certificates }
}

async fn wait_for_log(node: &LocalNode, offset: usize, needle: &str) {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let log = node.get_log_contents().unwrap();
        if log.get(offset..).unwrap_or_default().contains(needle) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "{} never logged {:?}; logs: {:?}",
            node.name(),
            needle,
            node.log_path()
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

struct UpgradeSwarm {
    swarm: LocalSwarm,
    cli: CliTestFramework,
    peers: Vec<AccountAddress>,
    upgraded: HashSet<AccountAddress>,
    deterministic: HashSet<AccountAddress>,
    version: Version,
    epoch: u64,
    roll: MemberId,
}

impl UpgradeSwarm {
    async fn new() -> Self {
        aptos_logger::Logger::new().init();
        let old = FACTORY.versions().min().unwrap();
        let version = FACTORY.versions().max().unwrap();
        assert_ne!(old, version);
        static ACTIVE_NODES: Lazy<Arc<Mutex<usize>>> = Lazy::new(|| Arc::new(Mutex::new(0)));
        let guard = ActiveNodesGuard::grab(NUM_VALIDATORS * 2, ACTIVE_NODES.clone()).await;
        let swarm = FACTORY
            .new_swarm_with_version(
                OsRng,
                NonZeroUsize::new(NUM_VALIDATORS).unwrap(),
                0,
                &old,
                Some(aptos_cached_packages::head_release_bundle().clone()),
                None,
                None,
                Some(Arc::new(|_, stake| {
                    *stake = 100_000_000_000_000;
                })),
                Some(Arc::new(|conf| {
                    conf.epoch_duration_secs = 3600;
                    conf.consensus_config.enable_validator_txns();
                    conf.consensus_config.disable_rand_check();
                    conf.randomness_config_override =
                        Some(OnChainRandomnessConfig::default_enabled());
                    conf.chunky_dkg_config_override =
                        Some(OnChainChunkyDKGConfig::default_disabled());
                })),
                guard,
            )
            .await
            .unwrap();
        swarm
            .wait_for_all_nodes_to_catchup_to_epoch(2, TIMEOUT)
            .await
            .unwrap();
        let peers: Vec<_> = swarm.validators().map(|node| node.peer_id()).collect();
        assert!(swarm.validators().all(|node| node.version() == old));
        let endpoint = swarm.validator(peers[0]).unwrap().rest_api_endpoint();
        let mut cli = CliTestFramework::new(endpoint.clone(), endpoint, 0).await;
        let root_address = swarm.chain_info().root_account().address();
        assert_eq!(
            cli.add_account_with_address_to_cli(swarm.root_key(), root_address),
            0
        );
        let roll =
            MemberId::from_str(&format!("{}::dice::roll", root_address.to_hex_literal())).unwrap();
        let mut test = Self {
            swarm,
            cli,
            peers,
            upgraded: HashSet::new(),
            deterministic: HashSet::new(),
            version,
            epoch: 2,
            roll,
        };
        test.wait_for_certificates(&[0; NUM_VALIDATORS]).await;
        publish_on_chain_dice_module(&mut test.cli, 0).await;
        test.assert_key_modes().await;
        test.assert_randomness_commits(0).await;
        test
    }

    async fn session(&self) -> DKGSessionState {
        get_on_chain_resource::<DKGState>(
            &self.swarm.validator(self.peers[0]).unwrap().rest_client(),
        )
        .await
        .last_completed
        .unwrap()
    }

    async fn wait_for_certificates(&self, offsets: &[usize]) {
        for (node, offset) in self.swarm.validators().zip(offsets) {
            wait_for_log(
                node,
                *offset,
                "[RandManager] Finish broadcasting certified aug data",
            )
            .await;
        }
    }

    async fn snapshot(&mut self, index: usize) -> RandDbSnapshot {
        let node = self.swarm.validator(self.peers[index]).unwrap();
        node.stop();
        let snapshot = read_rand_db(&node.config().storage.dir().join("rand_db"), self.epoch);
        node.start().unwrap();
        self.swarm.wait_all_alive(TIMEOUT).await.unwrap();
        self.swarm
            .wait_for_all_nodes_to_catchup(TIMEOUT)
            .await
            .unwrap();
        snapshot
    }

    // Independently derive the expected new-format key from the current transcript.
    fn deterministic_key(&self, index: usize, session: &DKGSessionState) -> Vec<u8> {
        let params = DefaultDKG::new_public_params(&session.metadata);
        assert_eq!(params.pvss_config.wconfig.get_threshold_weight(), 4);
        let transcript =
            bcs::from_bytes::<<DefaultDKG as DKGTrait>::Transcript>(&session.transcript).unwrap();
        let keys = decrypt_key_map(&self.swarm);
        let player_index = session
            .metadata
            .target_validator_set
            .iter()
            .position(|info| info.addr == self.peers[index])
            .unwrap();
        let (sk, pk) = DefaultDKG::decrypt_secret_share_from_transcript(
            &params,
            &transcript,
            player_index as u64,
            &keys[&self.peers[index]],
        )
        .unwrap();
        let seed_bytes = bcs::to_bytes(&(
            "APTOS_RANDOMNESS_AUG_KEY_SEED_V1",
            &sk.main,
            self.epoch,
            &session.metadata,
        ))
        .unwrap();
        let mut rng = StdRng::from_seed(*HashValue::sha3_256_of(&seed_bytes).as_ref());
        let key = WVUF::augment_key_pair(
            &WvufPP::from(&params.pvss_config.pp),
            sk.main,
            pk.main,
            &mut rng,
        );
        bcs::to_bytes(&(key, None::<(ASK, APK)>)).unwrap()
    }

    async fn assert_key_modes(&mut self) {
        let session = self.session().await;
        assert_eq!(session.metadata.dealer_epoch + 1, self.epoch);
        for index in 0..NUM_VALIDATORS {
            let snapshot = self.snapshot(index).await;
            assert_eq!(snapshot.certificates.len(), NUM_VALIDATORS);
            // Never print private key bytes on assertion failures.
            assert!(
                (snapshot.key == self.deterministic_key(index, &session))
                    == self.deterministic.contains(&self.peers[index]),
                "unexpected augmentation mode for validator {} in epoch {}",
                index,
                self.epoch
            );
        }
        info!(
            "Phase1 upgrade: epoch {}, {} deterministic and {} legacy keys",
            self.epoch,
            self.deterministic.len(),
            NUM_VALIDATORS - self.deterministic.len()
        );
    }

    async fn upgrade(&mut self, index: usize) {
        let session = self.session().await;
        let peer = self.peers[index];
        let node = self.swarm.validator(peer).unwrap();
        let offset = node.get_log_contents().unwrap().len();
        node.stop();
        let before = read_rand_db(&node.config().storage.dir().join("rand_db"), self.epoch);
        self.swarm
            .upgrade_validator(peer, &self.version)
            .await
            .unwrap();
        wait_for_log(
            self.swarm.validator(peer).unwrap(),
            offset,
            "Recovering existing augmented key",
        )
        .await;
        assert_eq!(self.swarm.validator(peer).unwrap().version(), self.version);
        let after = self.snapshot(index).await;
        assert!(
            before == after,
            "upgrade changed the current-epoch RandDB for validator {}",
            index
        );
        let after_session = self.session().await;
        assert_eq!(after_session.metadata, session.metadata);
        assert_eq!(after_session.transcript, session.transcript);
        self.upgraded.insert(peer);
        info!(
            "Phase1 upgrade: validator {} replaced its binary in epoch {}; RandDB unchanged",
            index, self.epoch
        );
        self.assert_randomness_commits(index).await;
    }

    async fn assert_randomness_commits(&mut self, required: usize) {
        // Keep exactly the reconstruction quorum online, including the validator under test.
        let offline = if required == NUM_VALIDATORS - 1 {
            NUM_VALIDATORS - 2
        } else {
            NUM_VALIDATORS - 1
        };
        self.swarm.validator(self.peers[offline]).unwrap().stop();
        let client = self
            .swarm
            .validator(self.peers[required])
            .unwrap()
            .rest_client();
        let before = get_current_version(&client).await;
        let txn = self
            .cli
            .run_function(
                0,
                Some(GasOptions {
                    gas_unit_price: Some(100),
                    max_gas: Some(10_000),
                    expiration_secs: 120,
                }),
                self.roll.clone(),
                vec![],
                vec![],
            )
            .await
            .unwrap();
        assert_eq!(txn.success, Some(true));
        let version = get_current_version(&client).await;
        assert!(version > before);
        assert_eq!(
            client.get_ledger_information().await.unwrap().inner().epoch,
            self.epoch
        );
        verify_randomness(&decrypt_key_map(&self.swarm), &client, version)
            .await
            .unwrap();
        self.swarm
            .validator(self.peers[offline])
            .unwrap()
            .start()
            .unwrap();
        self.swarm.wait_all_alive(TIMEOUT).await.unwrap();
        self.swarm
            .wait_for_all_nodes_to_catchup(TIMEOUT)
            .await
            .unwrap();
        info!(
            "Phase1 upgrade: epoch {} randomness commit requires validator {}, version {} -> {}",
            self.epoch, required, before, version
        );
    }

    async fn next_epoch(&mut self) {
        let previous = self.session().await;
        let offsets: Vec<_> = self
            .swarm
            .validators()
            .map(|node| node.get_log_contents().unwrap().len())
            .collect();
        self.cli
            .run_script(0, &script_to_enable_main_logic())
            .await
            .unwrap();
        self.epoch += 1;
        self.swarm
            .wait_for_all_nodes_to_catchup_to_epoch(self.epoch, TIMEOUT)
            .await
            .unwrap();
        self.wait_for_certificates(&offsets).await;
        let current = self.session().await;
        assert_eq!(current.metadata.dealer_epoch + 1, self.epoch);
        assert_ne!(current.transcript, previous.transcript);
        self.deterministic = self.upgraded.clone();
        self.assert_key_modes().await;
        // Require every validator's contribution, including those still on the old binary.
        for index in 0..NUM_VALIDATORS {
            self.assert_randomness_commits(index).await;
        }
    }
}

#[tokio::test]
async fn phase1_upgrade_one_validator_per_epoch_preserves_rand_db() {
    let mut test = UpgradeSwarm::new().await;
    for index in 0..NUM_VALIDATORS {
        test.upgrade(index).await;
        test.assert_key_modes().await;
        test.next_epoch().await;
    }
}

#[tokio::test]
async fn phase1_upgrade_majority_in_one_epoch_preserves_rand_db() {
    let mut test = UpgradeSwarm::new().await;
    // Four of five binaries change in E; all five retain their legacy epoch-E keys.
    for index in 0..4 {
        test.upgrade(index).await;
    }
    assert_eq!(test.epoch, 2);
    test.assert_key_modes().await;
    test.next_epoch().await;
    // E+1 has four deterministic keys and one legacy key; finish the first release.
    test.upgrade(4).await;
    test.next_epoch().await;
}
