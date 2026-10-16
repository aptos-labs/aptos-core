// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! End-to-end validator observation using a loopback HTTPS server and a fixture
//! CA installed only into the local swarm's trust observation clients.

use crate::smoke_test_environment::SwarmBuilder;
use aptos::test::CliTestFramework;
use aptos_api_types::{transaction::ValidatorTransaction as ApiValidatorTransaction, Transaction};
use aptos_forge::{NodeExt, Swarm};
use aptos_rest_client::Client;
use aptos_types::{
    on_chain_config::{OnChainChunkyDKGConfig, OnChainRandomnessConfig},
    trust_observation::{
        Config, EffectiveTrustData, ObservedTrustData, ObservedValue, Patches,
        FORMAT_DER_CERTIFICATE, PATCH_REMOVE, PATCH_REPLACE,
    },
};
use hyper::{service::service_fn, Body, Request, Response};
use move_core_types::account_address::AccountAddress;
use openssl::{
    asn1::Asn1Time,
    bn::BigNum,
    hash::MessageDigest,
    pkey::PKey,
    rsa::Rsa,
    x509::{
        extension::{BasicConstraints, ExtendedKeyUsage, KeyUsage, SubjectAlternativeName},
        X509NameBuilder, X509,
    },
};
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio::{
    net::TcpListener,
    sync::{oneshot, RwLock},
    task::JoinHandle,
    time::{sleep, timeout},
};
use tokio_rustls::{
    rustls::{Certificate, PrivateKey, ServerConfig},
    TlsAcceptor,
};

const TARGET_ID: &[u8] = b"smoke-intel-root";
const RESOURCE_PREFIX: &str = "0x1::trust_observation::";

/// Generate the fixture at test time: platform TLS libraries reject overly
/// long certificate lifetimes, and checked-in short-lived certs expire in CI.
struct FixtureCertificates {
    ca_pem: String,
    ca_der: Vec<u8>,
    server_der: Vec<u8>,
    server_pk8: Vec<u8>,
}

impl FixtureCertificates {
    fn generate() -> Self {
        let ca_key = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
        let mut ca_name = X509NameBuilder::new().unwrap();
        ca_name
            .append_entry_by_text("CN", "Trust Observation Smoke CA")
            .unwrap();
        let ca_name = ca_name.build();
        let mut ca = X509::builder().unwrap();
        ca.set_version(2).unwrap();
        ca.set_serial_number(&BigNum::from_u32(1).unwrap().to_asn1_integer().unwrap())
            .unwrap();
        ca.set_subject_name(&ca_name).unwrap();
        ca.set_issuer_name(&ca_name).unwrap();
        ca.set_pubkey(&ca_key).unwrap();
        ca.set_not_before(&Asn1Time::days_from_now(0).unwrap())
            .unwrap();
        ca.set_not_after(&Asn1Time::days_from_now(30).unwrap())
            .unwrap();
        ca.append_extension(BasicConstraints::new().critical().ca().build().unwrap())
            .unwrap();
        ca.append_extension(
            KeyUsage::new()
                .critical()
                .key_cert_sign()
                .crl_sign()
                .build()
                .unwrap(),
        )
        .unwrap();
        ca.sign(&ca_key, MessageDigest::sha256()).unwrap();
        let ca = ca.build();

        let server_key = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
        let mut server_name = X509NameBuilder::new().unwrap();
        server_name.append_entry_by_text("CN", "localhost").unwrap();
        let server_name = server_name.build();
        let mut server = X509::builder().unwrap();
        server.set_version(2).unwrap();
        server
            .set_serial_number(&BigNum::from_u32(2).unwrap().to_asn1_integer().unwrap())
            .unwrap();
        server.set_subject_name(&server_name).unwrap();
        server.set_issuer_name(ca.subject_name()).unwrap();
        server.set_pubkey(&server_key).unwrap();
        server
            .set_not_before(&Asn1Time::days_from_now(0).unwrap())
            .unwrap();
        server
            .set_not_after(&Asn1Time::days_from_now(30).unwrap())
            .unwrap();
        server
            .append_extension(BasicConstraints::new().critical().build().unwrap())
            .unwrap();
        server
            .append_extension(
                SubjectAlternativeName::new()
                    .dns("localhost")
                    .ip("127.0.0.1")
                    .build(&server.x509v3_context(Some(&ca), None))
                    .unwrap(),
            )
            .unwrap();
        server
            .append_extension(ExtendedKeyUsage::new().server_auth().build().unwrap())
            .unwrap();
        server
            .append_extension(
                KeyUsage::new()
                    .digital_signature()
                    .key_encipherment()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        server.sign(&ca_key, MessageDigest::sha256()).unwrap();
        let server = server.build();

        Self {
            ca_pem: String::from_utf8(ca.to_pem().unwrap()).unwrap(),
            ca_der: ca.to_der().unwrap(),
            server_der: server.to_der().unwrap(),
            server_pk8: server_key.private_key_to_pkcs8().unwrap(),
        }
    }
}

struct FixtureHttpsServer {
    url: String,
    content: Arc<RwLock<Vec<u8>>>,
    close_tx: oneshot::Sender<()>,
    join_handle: JoinHandle<()>,
}

impl FixtureHttpsServer {
    async fn spawn(certificates: &FixtureCertificates) -> Self {
        let tls = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(
                vec![Certificate(certificates.server_der.clone())],
                PrivateKey(certificates.server_pk8.clone()),
            )
            .unwrap();
        let acceptor = TlsAcceptor::from(Arc::new(tls));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!(
            "https://127.0.0.1:{}",
            listener.local_addr().unwrap().port()
        );
        let content = Arc::new(RwLock::new(certificates.ca_der.clone()));
        let shared_content = content.clone();
        let (close_tx, mut close_rx) = oneshot::channel();
        let join_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut close_rx => break,
                    incoming = listener.accept() => {
                        let (socket, _) = incoming.unwrap();
                        let acceptor = acceptor.clone();
                        let content = shared_content.clone();
                        tokio::spawn(async move {
                            if let Ok(stream) = acceptor.accept(socket).await {
                                let service = service_fn(move |_request: Request<Body>| {
                                    let content = content.clone();
                                    async move {
                                        let bytes = content.read().await.clone();
                                        Ok::<_, Infallible>(Response::new(Body::from(bytes)))
                                    }
                                });
                                let _ = hyper::server::conn::Http::new()
                                    .serve_connection(stream, service)
                                    .await;
                            }
                        });
                    },
                }
            }
        });
        Self {
            url,
            content,
            close_tx,
            join_handle,
        }
    }

    async fn serve(&self, bytes: &[u8]) {
        *self.content.write().await = bytes.to_vec();
    }

    async fn shutdown(self) {
        let _ = self.close_tx.send(());
        self.join_handle.await.unwrap();
    }
}

async fn submit_framework_script(cli: &CliTestFramework, root_idx: usize, operation: &str) {
    let script = format!(
        r#"
script {{
    use aptos_framework::aptos_governance;
    use aptos_framework::trust_observation;
    use std::string::utf8;

    fun main(core_resources: &signer) {{
        let framework = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        {operation}
    }}
}}
"#
    );
    cli.run_script(root_idx, &script).await.unwrap();
}

async fn config(client: &Client) -> Config {
    client
        .get_account_resource_bcs::<Config>(
            AccountAddress::ONE,
            &format!("{RESOURCE_PREFIX}Config"),
        )
        .await
        .unwrap()
        .into_inner()
}

async fn observed(client: &Client) -> ObservedTrustData {
    client
        .get_account_resource_bcs::<ObservedTrustData>(
            AccountAddress::ONE,
            &format!("{RESOURCE_PREFIX}ObservedTrustData"),
        )
        .await
        .unwrap()
        .into_inner()
}

async fn effective(client: &Client) -> EffectiveTrustData {
    client
        .get_account_resource_bcs::<EffectiveTrustData>(
            AccountAddress::ONE,
            &format!("{RESOURCE_PREFIX}EffectiveTrustData"),
        )
        .await
        .unwrap()
        .into_inner()
}

async fn patches(client: &Client) -> Patches {
    client
        .get_account_resource_bcs::<Patches>(
            AccountAddress::ONE,
            &format!("{RESOURCE_PREFIX}Patches"),
        )
        .await
        .unwrap()
        .into_inner()
}

async fn wait_for_target_version(client: &Client, expected: Option<u64>) {
    timeout(Duration::from_secs(90), async {
        loop {
            let current = config(client).await;
            if current.get(TARGET_ID).map(|target| target.config_version) == expected {
                return;
            }
            sleep(Duration::from_millis(500)).await;
        }
    })
    .await
    .expect("trust target configuration did not activate in the next epoch");
}

async fn wait_for_observation(client: &Client, version: u64, value: &[u8]) -> ObservedValue {
    timeout(Duration::from_secs(120), async {
        loop {
            let state = observed(client).await;
            if let Some(entry) = state.get(TARGET_ID) {
                if entry.version == version && entry.value == value {
                    return entry.clone();
                }
            }
            sleep(Duration::from_millis(500)).await;
        }
    })
    .await
    .expect("validators did not commit the expected trust observation")
}

async fn assert_committed_observation_txn(
    client: &Client,
    start_version: u64,
    expected_version: u64,
    expected_value: &[u8],
) {
    let end_version = client
        .get_ledger_information()
        .await
        .unwrap()
        .inner()
        .version;
    let mut next_version = start_version;
    while next_version <= end_version {
        let transactions = client
            .get_transactions(Some(next_version), Some(100))
            .await
            .unwrap()
            .into_inner();
        assert!(
            !transactions.is_empty(),
            "missing transaction at {next_version}"
        );
        next_version += transactions.len() as u64;
        for transaction in transactions {
            if let Transaction::ValidatorTransaction(
                ApiValidatorTransaction::ObservedTrustUpdate(update),
            ) = transaction
            {
                let observed = &update.quorum_certified_update.update;
                if observed.target_id.0 == TARGET_ID && observed.version.0 == expected_version {
                    assert!(update.info.success);
                    assert_eq!(observed.value.0, expected_value);
                    assert!(
                        update
                            .quorum_certified_update
                            .multi_sig
                            .signer_indices
                            .len()
                            >= 3,
                        "expected a quorum of four equal-stake validators"
                    );
                    assert!(!update.events.is_empty());
                    return;
                }
            }
        }
    }
    panic!("no committed ObservedTrustUpdate validator transaction found");
}

/// Validators observe a loopback HTTPS root, quorum certify two versions,
/// and commit them as validator transactions. Governance patches alter the
/// verifier-facing value without changing the observation and are invalidated
/// when the target configuration changes or is removed.
#[test]
fn trust_observation_consensus_smoke() {
    // Aptos CLI's Move script compilation needs more than libtest's 2 MiB
    // default stack on this platform; keep the test runnable without env vars.
    std::thread::Builder::new()
        .name("trust-observation-smoke".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .unwrap()
                .block_on(trust_observation_consensus_smoke_inner());
        })
        .unwrap()
        .join()
        .unwrap();
}

async fn trust_observation_consensus_smoke_inner() {
    let certificates = FixtureCertificates::generate();
    let ca_pem = certificates.ca_pem.clone();
    let server = FixtureHttpsServer::spawn(&certificates).await;
    let first_url = format!("{}/root.cer", server.url);
    let second_url = format!("{}/rotated-root.cer", server.url);
    let (swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(move |_, config, _| {
            config
                .jwk_consensus
                .trust_observation_extra_root_certificates_pem = vec![ca_pem.clone()];
        }))
        .with_init_genesis_stake(Arc::new(|_, amount| {
            *amount = 100_000_000_000_000;
        }))
        .with_init_genesis_config(Arc::new(|config| {
            config.epoch_duration_secs = 30;
            config.consensus_config.enable_validator_txns();
            config.randomness_config_override = Some(OnChainRandomnessConfig::Off);
            config.chunky_dkg_config_override = Some(OnChainChunkyDKGConfig::default_disabled());
        }))
        .build_with_cli(0)
        .await;
    let client = swarm.validators().next().unwrap().rest_client();
    let root_idx = cli.add_account_with_address_to_cli(
        swarm.root_key(),
        swarm.chain_info().root_account().address(),
    );
    assert!(config(&client).await.targets.is_empty());
    assert!(observed(&client).await.entries.is_empty());
    assert!(effective(&client).await.entries.is_empty());
    let first_transaction_version = client
        .get_ledger_information()
        .await
        .unwrap()
        .inner()
        .version
        + 1;

    submit_framework_script(
        &cli,
        root_idx,
        &format!(
            r#"trust_observation::upsert_target_for_next_epoch(
                &framework, b"smoke-intel-root", utf8(b"{first_url}"),
                {FORMAT_DER_CERTIFICATE}, 1024,
            );
            aptos_governance::reconfigure(&framework);"#
        ),
    )
    .await;
    wait_for_target_version(&client, Some(1)).await;
    let target = config(&client).await.get(TARGET_ID).unwrap().clone();
    assert_eq!(target.url, first_url);
    assert_eq!(target.format, FORMAT_DER_CERTIFICATE);
    assert_eq!(target.max_bytes, 1024);
    let first = wait_for_observation(&client, 1, &certificates.ca_der).await;
    assert_eq!(first.config_version, 1);
    assert_eq!(effective(&client).await.get(TARGET_ID), Some(&first));
    assert_committed_observation_txn(&client, first_transaction_version, 1, &certificates.ca_der)
        .await;

    server.serve(&certificates.server_der).await;
    let second = wait_for_observation(&client, 2, &certificates.server_der).await;
    assert_eq!(second.config_version, 1);
    assert_eq!(effective(&client).await.get(TARGET_ID), Some(&second));

    submit_framework_script(
        &cli,
        root_idx,
        r#"trust_observation::set_replacement_patch(
            &framework, b"smoke-intel-root", b"temporary-root",
        );"#,
    )
    .await;
    assert_eq!(observed(&client).await.get(TARGET_ID), Some(&second));
    let patched = effective(&client).await;
    let entry = patched.get(TARGET_ID).unwrap();
    assert_eq!(entry.config_version, 1);
    assert_eq!(entry.value, b"temporary-root");
    assert_eq!(patches(&client).await.entries[0].kind, PATCH_REPLACE);

    submit_framework_script(
        &cli,
        root_idx,
        r#"trust_observation::set_removal_patch(&framework, b"smoke-intel-root");"#,
    )
    .await;
    assert!(effective(&client).await.get(TARGET_ID).is_none());
    assert_eq!(patches(&client).await.entries[0].kind, PATCH_REMOVE);
    assert_eq!(observed(&client).await.get(TARGET_ID), Some(&second));

    submit_framework_script(
        &cli,
        root_idx,
        r#"trust_observation::clear_patch(&framework, b"smoke-intel-root");"#,
    )
    .await;
    assert!(patches(&client).await.entries.is_empty());
    assert_eq!(effective(&client).await.get(TARGET_ID), Some(&second));

    submit_framework_script(
        &cli,
        root_idx,
        r#"trust_observation::set_replacement_patch(
            &framework, b"smoke-intel-root", b"old-configuration-only",
        );"#,
    )
    .await;
    server.serve(b"invalid DER").await;
    submit_framework_script(
        &cli,
        root_idx,
        &format!(
            r#"trust_observation::upsert_target_for_next_epoch(
                &framework, b"smoke-intel-root", utf8(b"{second_url}"),
                {FORMAT_DER_CERTIFICATE}, 1024,
            );
            aptos_governance::reconfigure(&framework);"#
        ),
    )
    .await;
    wait_for_target_version(&client, Some(2)).await;
    assert_eq!(
        config(&client).await.get(TARGET_ID).unwrap().url,
        second_url
    );
    assert!(patches(&client).await.entries.is_empty());
    assert!(effective(&client).await.get(TARGET_ID).is_none());
    assert!(observed(&client).await.entries.is_empty());

    submit_framework_script(
        &cli,
        root_idx,
        r#"trust_observation::remove_target_for_next_epoch(
            &framework, b"smoke-intel-root",
        );
        aptos_governance::reconfigure(&framework);"#,
    )
    .await;
    wait_for_target_version(&client, None).await;
    assert!(config(&client).await.targets.is_empty());
    assert!(observed(&client).await.entries.is_empty());
    assert!(effective(&client).await.entries.is_empty());
    server.shutdown().await;
}
