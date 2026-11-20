// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Run with `RUST_MIN_STACK=67108864 RUST_LOG=info`: the CLI compiles Move packages,
//! and the test checks node logs to distinguish certificate recovery from proactive delivery.

use crate::{
    randomness::{
        decrypt_key_map, e2e_basic_consumption::publish_on_chain_dice_module, get_current_version,
        verify_randomness,
    },
    smoke_test_environment::SwarmBuilder,
    utils::get_on_chain_resource,
};
use aptos::common::types::GasOptions;
use aptos_forge::{LocalNode, NodeExt, Swarm, SwarmExt};
use aptos_logger::info;
use aptos_move_cli::MemberId;
use aptos_schemadb::{
    schema::{KeyCodec, Schema, ValueCodec},
    Options, DB,
};
use aptos_types::{
    account_address::AccountAddress,
    dkg::{DKGState, DKGTrait, DefaultDKG},
    on_chain_config::{OnChainChunkyDKGConfig, OnChainRandomnessConfig},
};
use std::{collections::HashSet, fs, path::Path, str::FromStr, sync::Arc, time::Duration};
use tokio::time::Instant;

const NUM_VALIDATORS: usize = 5;
const RECOVERY_TIMEOUT: Duration = Duration::from_secs(90);

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

fn read_rand_db(path: &Path, epoch: u64) -> (Vec<u8>, HashSet<AccountAddress>) {
    let db = DB::open_readonly(
        path,
        "rand_db_loss_smoke",
        vec!["key_pair", "aug_data", "certified_aug_data"],
        Options::default(),
    )
    .unwrap();
    let (key_epoch, key_pair) = db.get::<KeyPairSnapshot>(&()).unwrap().unwrap();
    assert_eq!(key_epoch, epoch);
    let mut iter = db.iter::<CertifiedDataSnapshot>().unwrap();
    iter.seek_to_first();
    let certified_authors = iter
        .map(|entry| {
            let (key, _) = entry.unwrap();
            let (cert_epoch, author): (u64, AccountAddress) = bcs::from_bytes(&key).unwrap();
            assert_eq!(cert_epoch, epoch);
            author
        })
        .collect();
    (key_pair, certified_authors)
}

async fn wait_for_log(node: &LocalNode, offset: usize, needle: &str) {
    let deadline = Instant::now() + RECOVERY_TIMEOUT;
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

#[tokio::test]
async fn rand_db_loss_recovers_within_same_epoch() {
    run_rand_db_loss_recovery(false).await;
}

#[tokio::test]
async fn rand_db_loss_recovers_with_optimistic_verification() {
    run_rand_db_loss_recovery(true).await;
}

async fn run_rand_db_loss_recovery(optimistic_verification: bool) {
    let (swarm, mut cli, _faucet) = SwarmBuilder::new_local(NUM_VALIDATORS)
        .with_aptos()
        .with_init_config(Arc::new(move |_, conf, _| {
            conf.consensus.optimistic_rand_share_verification = optimistic_verification;
        }))
        .with_init_genesis_stake(Arc::new(|_, stake| {
            *stake = 100_000_000_000_000;
        }))
        .with_init_genesis_config(Arc::new(|conf| {
            // The first block starts the initial DKG; leave ample time in the next epoch
            // to recover without triggering another DKG or changing the randomness config.
            conf.epoch_duration_secs = 3600;
            conf.consensus_config.enable_validator_txns();
            conf.consensus_config.disable_rand_check();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
            // This test isolates RandDB; ChunkyDKG has separate persisted dealer/share state.
            conf.chunky_dkg_config_override = Some(OnChainChunkyDKGConfig::default_disabled());
        }))
        .build_with_cli(0)
        .await;

    let root_address = swarm.chain_info().root_account().address();
    let root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_address);
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, RECOVERY_TIMEOUT)
        .await
        .unwrap();

    let victim_id = swarm.validators().next().unwrap().peer_id();
    let offline_id = swarm
        .validators()
        .find(|node| node.peer_id() != victim_id)
        .unwrap()
        .peer_id();
    let victim_client = swarm.validator(victim_id).unwrap().rest_client();
    let healthy_client = swarm
        .validators()
        .find(|node| node.peer_id() != victim_id && node.peer_id() != offline_id)
        .unwrap()
        .rest_client();
    let decrypt_keys = decrypt_key_map(&swarm);
    let dkg_before = get_on_chain_resource::<DKGState>(&victim_client)
        .await
        .last_completed
        .unwrap();
    let epoch = dkg_before.metadata.dealer_epoch + 1;
    assert_eq!(epoch, 2);
    let params = DefaultDKG::new_public_params(&dkg_before.metadata);
    let threshold = params.pvss_config.wconfig.get_threshold_weight();
    assert_eq!(threshold, 4, "exercise the normal 4-out-of-5 configuration");

    // Every peer must have finished its proactive broadcast before the loss. Otherwise a
    // pending retry could accidentally recover our certificates without exercising the new RPC.
    for node in swarm.validators() {
        wait_for_log(
            node,
            0,
            "[RandManager] Finish broadcasting certified aug data",
        )
        .await;
    }
    publish_on_chain_dice_module(&mut cli, root_idx).await;
    let roll_function =
        MemberId::from_str(&format!("{}::dice::roll", root_address.to_hex_literal())).unwrap();
    let expected_authors: HashSet<_> = swarm.validators().map(|node| node.peer_id()).collect();
    let victim = swarm.validator(victim_id).unwrap();
    let rand_db_path = victim.config().storage.dir().join("rand_db");
    // Keep one other validator offline: committing a new dice transaction now requires
    // the recovered validator, rather than allowing four healthy peers to mask a failure.
    swarm.validator(offline_id).unwrap().stop();
    victim.stop();
    let (original_key, original_certs) = read_rand_db(&rand_db_path, epoch);
    assert_eq!(original_certs, expected_authors);

    for loss in 0..2 {
        let version_before = get_current_version(&healthy_client).await;
        // Move away only RandDB, preserving the ledger, consensus DB, safety rules and identity.
        let removed_db = rand_db_path.with_file_name(format!("rand_db_removed_{loss}"));
        fs::rename(&rand_db_path, &removed_db).unwrap();
        assert!(!rand_db_path.exists());
        let victim = swarm.validator(victim_id).unwrap();
        let log_offset = victim.get_log_contents().unwrap().len();
        victim.start().unwrap();

        wait_for_log(
            victim,
            log_offset,
            "[RandManager] Recovered certified aug data for",
        )
        .await;
        wait_for_log(victim, log_offset, "Processing decisioned randomness.").await;
        wait_for_log(
            victim,
            log_offset,
            "[RandManager] Finish broadcasting aug data",
        )
        .await;

        let transaction = cli
            .run_function(
                root_idx,
                Some(GasOptions {
                    gas_unit_price: Some(100),
                    max_gas: Some(10_000),
                    expiration_secs: 120,
                }),
                roll_function.clone(),
                vec![],
                vec![],
            )
            .await
            .expect("a randomness-consuming transaction must commit after recovery");
        assert_eq!(transaction.success, Some(true));
        let version_after = get_current_version(&victim_client).await;
        assert!(version_after > version_before);
        let ledger = victim_client
            .get_ledger_information()
            .await
            .unwrap()
            .into_inner();
        assert_eq!(
            ledger.epoch, epoch,
            "recovery must not wait for a new epoch"
        );
        let dkg_after = get_on_chain_resource::<DKGState>(&victim_client)
            .await
            .last_completed
            .unwrap();
        assert_eq!(
            bcs::to_bytes(&dkg_after.metadata).unwrap(),
            bcs::to_bytes(&dkg_before.metadata).unwrap()
        );
        assert_eq!(dkg_after.transcript, dkg_before.transcript);
        verify_randomness(&decrypt_keys, &victim_client, version_after)
            .await
            .expect("recovered randomness must match evaluation under the original DKG secret");

        victim.stop();
        let (recovered_key, recovered_certs) = read_rand_db(&rand_db_path, epoch);
        // Do not print the serialized private key material on an assertion failure.
        assert!(
            original_key == recovered_key,
            "the augmented key pair changed after DB loss"
        );
        assert!(recovered_certs.contains(&victim_id));
        assert!(recovered_certs.is_subset(&expected_authors));
        assert!(recovered_certs.len() >= threshold);
        info!(
            "RandDB loss {} recovered in epoch {}, versions {} -> {}, {} certificates, optimistic={}",
            loss, epoch, version_before, version_after, recovered_certs.len(), optimistic_verification
        );
    }

    // A normal restart must reuse the regenerated key pair and the persisted recovered certs.
    let victim = swarm.validator(victim_id).unwrap();
    let log_offset = victim.get_log_contents().unwrap().len();
    victim.start().unwrap();
    wait_for_log(victim, log_offset, "Recovering existing augmented key").await;
    wait_for_log(victim, log_offset, "Processing decisioned randomness.").await;
    assert_eq!(
        victim_client
            .get_ledger_information()
            .await
            .unwrap()
            .inner()
            .epoch,
        epoch
    );
    swarm.validator(offline_id).unwrap().start().unwrap();
    swarm
        .wait_for_all_nodes_to_catchup(RECOVERY_TIMEOUT)
        .await
        .unwrap();
}
