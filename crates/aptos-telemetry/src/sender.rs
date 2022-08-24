// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::metrics;
use crate::metrics::{increment_log_ingest_failures_by, increment_log_ingest_successes_by};
use anyhow::{anyhow, Error};
use aptos_config::config::{NodeConfig, RoleType};
use aptos_crypto::{
    noise::{self, NoiseConfig},
    x25519,
};
use aptos_infallible::{Mutex, RwLock};
use aptos_logger::debug;
use aptos_telemetry_service::types::{
    auth::{AuthRequest, AuthResponse},
    telemetry::TelemetryDump,
};
use aptos_types::{chain_id::ChainId, PeerId};
use flate2::{write::GzEncoder, Compression};
use reqwest::{RequestBuilder, Response, StatusCode};
use std::{io::Write, sync::Arc};
use tokio_retry::{
    strategy::{jitter, ExponentialBackoff},
    Retry,
};

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
    base_url: String,
    chain_id: ChainId,
    peer_id: PeerId,
    role_type: RoleType,
    client: reqwest::Client,
    auth_context: Arc<AuthContext>,
}

impl TelemetrySender {
    pub fn new(base_url: String, chain_id: ChainId, node_config: &NodeConfig) -> Self {
        Self {
            base_url,
            chain_id,
            peer_id: node_config.peer_id().unwrap_or(PeerId::ZERO),
            role_type: node_config.base.role,
            client: reqwest::Client::new(),
            auth_context: Arc::new(AuthContext::new(node_config)),
        }
    }

    // sends an authenticated request to the telemetry service, automatically adding an auth token
    // This function does not work with streaming bodies at the moment and will panic if you try so.
    pub async fn send_authenticated_request(
        &self,
        request_builder: RequestBuilder,
    ) -> Result<Response, anyhow::Error> {
        let token = self.get_auth_token().await?;

        let mut response = request_builder
            .try_clone()
            .expect("Could not clone request_builder")
            .bearer_auth(token)
            .send()
            .await?;
        // do 1 retry if the first attempt failed
        if response.status() == StatusCode::UNAUTHORIZED {
            // looks like request failed due to auth error. Let's get a new a fresh token. If this fails again we'll just return the error.
            self.reset_token();
            let token = self.get_auth_token().await?;
            response = request_builder.bearer_auth(token).send().await?;
        }
        Ok(response)
    }

    pub(crate) async fn push_prometheus_metrics(&self) -> Result<(), anyhow::Error> {
        debug!("Sending Prometheus Metrics");

        let token = self.get_auth_token().await?;

        let scraped_metrics =
            prometheus::TextEncoder::new().encode_to_string(&aptos_metrics_core::gather())?;

        let mut gzip_encoder = GzEncoder::new(Vec::new(), Compression::default());
        gzip_encoder.write_all(scraped_metrics.as_bytes())?;
        let compressed_bytes = gzip_encoder.finish()?;

        let response = self
            .send_authenticated_request(
                self.client
                    .post(format!("{}/push-metrics", self.base_url))
                    .header("Content-Encoding", "gzip")
                    .bearer_auth(token)
                    .body(compressed_bytes),
            )
            .await;

        match response {
            Err(e) => Err(anyhow!("Prometheus Metrics push failed: {}", e)),
            Ok(response) => {
                if response.status().is_success() {
                    Ok(())
                } else {
                    Err(anyhow!(
                        "Prometheus Metrics push failed with response: {}, body: {}",
                        response.status(),
                        response
                            .text()
                            .await
                            .unwrap_or_else(|_| "empty body".to_string()),
                    ))
                }
            }
        }
    }

    pub(crate) async fn try_push_prometheus_metrics(&self) {
        self.push_prometheus_metrics().await.map_or_else(
            |e| debug!("Failed to push Prometheus Metrics: {}", e),
            |_| debug!("Prometheus Metrics pushed successfully."),
        );
    }

    pub async fn send_logs(&self, batch: Vec<String>) {
        if let Ok(json) = serde_json::to_string(&batch) {
            let len = json.len();

            let retry_strategy = ExponentialBackoff::from_millis(10)
                .map(jitter) // add jitter to delays
                .take(4); // limit to 4 retries

            let result = Retry::spawn(retry_strategy, || async {
                self.post_logs(json.as_bytes()).await
            })
            .await;
            match result {
                Ok(_) => {
                    increment_log_ingest_successes_by(batch.len() as u64);
                    debug!("Sent log of length: {}", len);
                }
                Err(error) => {
                    increment_log_ingest_failures_by(batch.len() as u64);
                    debug!("Failed send log of length: {} with error: {}", len, error);
                }
            }
        } else {
            debug!("Failed json serde of batch: {:?}", batch);
        }
    }

    async fn post_logs(&self, json: &[u8]) -> Result<(), anyhow::Error> {
        let token = self.get_auth_token().await?;

        let mut gzip_encoder = GzEncoder::new(Vec::new(), Compression::default());
        gzip_encoder.write_all(json)?;
        let compressed_bytes = gzip_encoder.finish()?;

        // Send the request and wait for a response
        let send_result = self
            .client
            .post(format!("{}/log_ingest", self.base_url))
            .header("Content-Encoding", "gzip")
            .bearer_auth(token)
            .body(compressed_bytes)
            .send()
            .await;

        // Process the result
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
            Err(error) => Err(anyhow!("Error sending log. Err: {}", error)),
        }
    }

    pub async fn send_metrics(&self, event_name: String, telemetry_dump: TelemetryDump) {
        let retry_strategy = ExponentialBackoff::from_millis(10)
            .map(jitter) // add jitter to delays
            .take(4); // limit to 4 retries

        let result = Retry::spawn(retry_strategy, || async {
            self.post_metrics(&telemetry_dump.clone()).await
        })
        .await;

        match result {
            Ok(_) => {
                debug!(
                    "Sent telemetry event {}, data: {:?}",
                    &event_name, &telemetry_dump
                );
                metrics::increment_telemetry_service_successes(&event_name);
            }
            Err(error) => {
                debug!("Failed to send telemetry event: Error: {}", error);
                metrics::increment_telemetry_service_failures(&event_name);
            }
        }
    }

    async fn post_metrics(&self, telemetry_dump: &TelemetryDump) -> Result<(), anyhow::Error> {
        let token = self.get_auth_token().await?;

        // Send the request and wait for a response
        let send_result = self
            .client
            .post(format!("{}/custom_event", self.base_url))
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
                    Err(anyhow!(
                        "Error status received {}: {}",
                        status_code,
                        response.text().await?,
                    ))
                }
            }
            Err(error) => Err(anyhow!("Error sending metrics. Err: {}", error)),
        }
    }

    async fn get_auth_token(&self) -> Result<String, Error> {
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
        let response = self.client.get(self.base_url.to_string()).send().await?;

        match error_for_status_with_body(response).await {
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
        *self.auth_context.token.write() = None;
        *self.auth_context.server_public_key.lock() = None;
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
            role_type: self.role_type,
            server_public_key,
            handshake_msg: client_noise_msg,
        };

        let response = self
            .client
            .post(format!("{}/auth", self.base_url))
            .json::<AuthRequest>(&auth_request)
            .send()
            .await?;

        let resp = match error_for_status_with_body(response).await {
            Ok(response) => Ok(response.json::<AuthResponse>().await?),
            Err(err) => {
                debug!(
                    "[telemetry-client] Error sending authentication request: {}",
                    err,
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

#[cfg(test)]
mod tests {

    use std::{
        collections::BTreeMap,
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::metrics::{APTOS_TELEMETRY_SERVICE_FAILURE, APTOS_TELEMETRY_SERVICE_SUCCESS};

    use super::*;
    use aptos_crypto::Uniform;
    use aptos_telemetry_service::types::telemetry::TelemetryEvent;
    use httpmock::MockServer;

    #[tokio::test]
    async fn test_server_public_key() {
        let mut rng = rand::thread_rng();
        let private_key = x25519::PrivateKey::generate(&mut rng);

        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method("GET").path("/");
            then.status(200).json_body_obj(&private_key.public_key());
        });

        let node_config = NodeConfig::default();
        let client = TelemetrySender::new(server.base_url(), ChainId::default(), &node_config);

        let result1 = client.server_public_key().await;
        let result2 = client.server_public_key().await;

        println!("{:?}", result1);

        // Should call the server once and cache the key
        assert_eq!(mock.hits(), 1);
        assert_eq!(result1.is_ok(), true);
        assert_eq!(result1.unwrap(), private_key.public_key());
        assert_eq!(result2.is_ok(), true);
        assert_eq!(result2.unwrap(), private_key.public_key());

        client.reset_token();

        let result3 = client.server_public_key().await;
        assert_eq!(mock.hits(), 2);
        assert_eq!(result3.is_ok(), true);
        assert_eq!(result3.unwrap(), private_key.public_key());
    }

    #[tokio::test]
    async fn test_post_metrics() {
        let mut telemetry_event = TelemetryEvent {
            name: "sample-event".into(),
            params: BTreeMap::new(),
        };
        telemetry_event
            .params
            .insert("key-1".into(), "value-1".into());
        let telemetry_dump = TelemetryDump {
            client_id: "client-1".into(),
            user_id: "user-1".into(),
            timestamp_micros: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros()
                .to_string(),
            events: vec![],
        };

        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method("POST")
                .header("Authorization", "Bearer SECRET_JWT_TOKEN")
                .path("/custom_event")
                .json_body_obj(&telemetry_dump);
            then.status(200);
        });

        let node_config = NodeConfig::default();
        let client = TelemetrySender::new(server.base_url(), ChainId::default(), &node_config);
        {
            *client.auth_context.token.write() = Some("SECRET_JWT_TOKEN".into());
        }

        let result = client.post_metrics(&telemetry_dump).await;

        mock.assert();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_metrics_retry() {
        let event_name = "sample-event";
        let mut telemetry_event = TelemetryEvent {
            name: event_name.into(),
            params: BTreeMap::new(),
        };
        telemetry_event
            .params
            .insert("key-1".into(), "value-1".into());
        let telemetry_dump = TelemetryDump {
            client_id: "client-1".into(),
            user_id: "user-1".into(),
            timestamp_micros: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros()
                .to_string(),
            events: vec![],
        };

        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method("POST").path("/custom_event");
            then.status(400);
        });

        let node_config = NodeConfig::default();
        let client = TelemetrySender::new(server.base_url(), ChainId::default(), &node_config);
        {
            *client.auth_context.token.write() = Some("SECRET_JWT_TOKEN".into());
        }

        client.send_metrics(event_name.into(), telemetry_dump).await;

        mock.assert_hits(5);
        assert_eq!(
            APTOS_TELEMETRY_SERVICE_SUCCESS
                .with_label_values(&[event_name])
                .get(),
            0
        );
        assert_eq!(
            APTOS_TELEMETRY_SERVICE_FAILURE
                .with_label_values(&[event_name])
                .get(),
            1
        );
    }

    #[tokio::test]
    async fn test_push_prometheus_metrics() {
        metrics::increment_telemetry_service_successes("test-event");

        let scraped_metrics = prometheus::TextEncoder::new()
            .encode_to_string(&aptos_metrics_core::gather())
            .unwrap();

        let mut gzip_encoder = GzEncoder::new(Vec::new(), Compression::default());
        gzip_encoder.write_all(scraped_metrics.as_bytes()).unwrap();
        let expected_compressed_bytes = gzip_encoder.finish().unwrap();

        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method("POST")
                .header("Authorization", "Bearer SECRET_JWT_TOKEN")
                .path("/push-metrics")
                .body(String::from_utf8_lossy(&expected_compressed_bytes));
            then.status(200);
        });

        let node_config = NodeConfig::default();
        let client = TelemetrySender::new(server.base_url(), ChainId::default(), &node_config);
        {
            *client.auth_context.token.write() = Some("SECRET_JWT_TOKEN".into());
        }

        let result = client.push_prometheus_metrics().await;

        mock.assert();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_post_logs() {
        let batch = vec!["log1".to_string(), "log2".to_string()];
        let json = serde_json::to_string(&batch);
        assert!(json.is_ok());

        let mut gzip_encoder = GzEncoder::new(Vec::new(), Compression::default());
        gzip_encoder.write_all(json.unwrap().as_bytes()).unwrap();
        let expected_compressed_bytes = gzip_encoder.finish().unwrap();

        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method("POST")
                .header("Authorization", "Bearer SECRET_JWT_TOKEN")
                .path("/log_ingest")
                .body(String::from_utf8_lossy(&expected_compressed_bytes));
            then.status(200);
        });

        let node_config = NodeConfig::default();
        let client = TelemetrySender::new(server.base_url(), ChainId::default(), &node_config);
        {
            *client.auth_context.token.write() = Some("SECRET_JWT_TOKEN".into());
        }

        client.send_logs(batch).await;

        mock.assert();
    }
}

async fn error_for_status_with_body(response: Response) -> Result<Response, anyhow::Error> {
    if response.status().is_client_error() || response.status().is_server_error() {
        Err(anyhow!(
            "HTTP status error ({}) for url ({}): {}",
            response.status(),
            response.url().clone(),
            response.text().await?,
        ))
    } else {
        Ok(response)
    }
}
