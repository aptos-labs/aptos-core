// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::metrics;
use anyhow::{anyhow, Error};
use aptos_config::config::NodeConfig;
use aptos_crypto::{
    noise::{self, NoiseConfig},
    x25519,
};
use aptos_infallible::{Mutex, RwLock};
use aptos_logger::{debug, error};
use aptos_telemetry_service::types::{
    auth::{AuthRequest, AuthResponse},
    telemetry::TelemetryDump,
};
use aptos_types::{chain_id::ChainId, PeerId};
use reqwest::{StatusCode, Url};
use std::sync::Arc;

// const METRICS_ENDPOINT: &str = "https://aptos.dev/telemetry/";
// const AUTH_URL: &str = "https://aptos.dev/telemetry/auth";
// const METRICS_URL: &str = "https://aptos.dev/telemetry/telemetry";

const METRICS_ENDPOINT: &str = "http://localhost:8081/";
const AUTH_URL: &str = "http://localhost:8081/auth";
const METRICS_URL: &str = "http://localhost:8081/telemetry";
const MAX_RETRIES: i32 = 5;

struct AuthContext {
    noise_config: Option<NoiseConfig>,
    token: RwLock<Option<String>>,
    server_public_key: Mutex<Option<x25519::PublicKey>>,
}

impl AuthContext {
    fn new(node_config: &NodeConfig) -> Self {
        Self {
            noise_config: node_config.identity_key().map(NoiseConfig::new),
            token: RwLock::new(None),
            server_public_key: Mutex::new(None),
        }
    }
}

#[derive(Clone)]
pub(crate) struct TelemetrySender {
    chain_id: ChainId,
    peer_id: PeerId,
    client: reqwest::Client,
    auth_context: Arc<AuthContext>,
}

impl TelemetrySender {
    pub fn new(chain_id: ChainId, node_config: &NodeConfig) -> Self {
        Self {
            chain_id,
            peer_id: node_config.peer_id().unwrap_or(PeerId::ZERO),
            client: reqwest::Client::new(),
            auth_context: Arc::new(AuthContext::new(node_config)),
        }
    }

    pub async fn send_metrics(&self, event_name: String, telemetry_dump: TelemetryDump) {
        for i in 1..MAX_RETRIES {
            match self.post_metrics(&telemetry_dump).await {
                Ok(_) => {
                    debug!(
                        "Sent telemetry event {}, data: {:?}",
                        &event_name, &telemetry_dump
                    );
                    metrics::increment_telemetry_service_successes(&event_name);
                    return;
                }
                Err(error) => {
                    if error.to_string().contains("Unauthorized") && i < MAX_RETRIES {
                        debug!("Retrying failed send telemetry event: Error: {}.", error);
                        continue;
                    }
                    error!("Failed to send telemetry event: Error: {}", error);
                    metrics::increment_telemetry_service_failures(&event_name);
                    return;
                }
            }
        }
    }

    async fn post_metrics(&self, telemetry_dump: &TelemetryDump) -> Result<(), anyhow::Error> {
        let token = self.get_token().await?;

        // Send the request and wait for a response
        let send_result = self
            .client
            .post(METRICS_URL)
            .json::<TelemetryDump>(telemetry_dump)
            .bearer_auth(token)
            .send()
            .await;

        // Process the response
        match send_result {
            Ok(response) => {
                let status_code = response.status();
                if status_code.is_success() {
                    Ok(())
                } else if status_code == StatusCode::UNAUTHORIZED {
                    self.reset_token();
                    Err(anyhow!("Unauthorized"))
                } else {
                    Err(anyhow!("Error status received: {}", status_code))
                }
            }
            Err(error) => Err(anyhow!("Error sending metrics. Err: {}", error)),
        }
    }

    async fn get_token(&self) -> Result<String, Error> {
        // Try to read the token holding a read lock
        let token = { self.auth_context.token.read().as_ref().cloned() };
        match token {
            Some(token) => Ok(token),
            None => {
                let token = self.authenticate().await?;
                *self.auth_context.token.write() = Some(token.clone());
                Ok(token)
            }
        }
    }

    async fn get_public_key_from_server(&self) -> Result<x25519::PublicKey, anyhow::Error> {
        let url = Url::parse(METRICS_ENDPOINT)?;
        let response = self.client.get(url).send().await?;

        match response.error_for_status() {
            Ok(response) => {
                let public_key = response.json::<x25519::PublicKey>().await?;
                Ok(public_key)
            }
            Err(err) => Err(anyhow!("Error getting server public key. {}", err)),
        }
    }

    async fn server_public_key(&self) -> Result<x25519::PublicKey, anyhow::Error> {
        let server_public_key = { *self.auth_context.server_public_key.lock() };
        match server_public_key {
            Some(key) => Ok(key),
            None => {
                let public_key = self.get_public_key_from_server().await?;
                *self.auth_context.server_public_key.lock() = Some(public_key);
                Ok(public_key)
            }
        }
    }

    fn reset_token(&self) {
        *self.auth_context.token.write() = None
    }

    pub async fn authenticate(&self) -> Result<String, anyhow::Error> {
        let noise_config = match &self.auth_context.noise_config {
            Some(config) => config,
            None => return Err(anyhow!("Cannot send telemetry without private key")),
        };
        let server_public_key = self.server_public_key().await?;

        // buffer to first noise handshake message
        let mut client_noise_msg = vec![0; noise::handshake_init_msg_len(0)];

        // build the prologue (chain_id | peer_id | server_public_key)
        const CHAIN_ID_LENGTH: usize = 1;
        const ID_SIZE: usize = CHAIN_ID_LENGTH + PeerId::LENGTH;
        const PROLOGUE_SIZE: usize = CHAIN_ID_LENGTH + PeerId::LENGTH + x25519::PUBLIC_KEY_SIZE;
        let mut prologue = [0; PROLOGUE_SIZE];
        prologue[..CHAIN_ID_LENGTH].copy_from_slice(&[self.chain_id.id()]);
        prologue[CHAIN_ID_LENGTH..ID_SIZE].copy_from_slice(self.peer_id.as_ref());
        prologue[ID_SIZE..PROLOGUE_SIZE].copy_from_slice(server_public_key.as_slice());

        let mut rng = rand::rngs::OsRng;

        // craft first handshake message  (-> e, es, s, ss)
        let initiator_state = noise_config
            .initiate_connection(
                &mut rng,
                &prologue,
                server_public_key,
                None,
                &mut client_noise_msg,
            )
            .unwrap();

        let auth_request = AuthRequest {
            chain_id: self.chain_id,
            peer_id: self.peer_id,
            server_public_key,
            handshake_msg: client_noise_msg,
        };

        let response = self
            .client
            .post(AUTH_URL)
            .json::<AuthRequest>(&auth_request)
            .send()
            .await?;

        let resp = match response.error_for_status() {
            Ok(response) => Ok(response.json::<AuthResponse>().await?),
            Err(err) => {
                error!(
                    "[telemetry-client] Error sending authentication request: {}",
                    err
                );
                Err(anyhow!("error {}", err))
            }
        }?;

        let (response_payload, _) = noise_config
            .finalize_connection(initiator_state, resp.handshake_msg.as_slice())
            .unwrap();

        let jwt = String::from_utf8(response_payload)?;

        Ok(jwt)
    }
}
