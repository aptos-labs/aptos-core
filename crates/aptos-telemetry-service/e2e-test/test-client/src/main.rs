// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Telemetry Service Test Client
//!
//! A CLI tool to test the telemetry service custom contract endpoints.
//! Sends logs, metrics, and custom events to a telemetry service instance.

use anyhow::{anyhow, Result};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey, Ed25519Signature},
    traits::{SigningKey, Uniform},
};
use aptos_telemetry_service::types::telemetry::{TelemetryDump, TelemetryEvent};
use aptos_types::{account_address::AccountAddress, chain_id::ChainId};
use clap::{Parser, Subcommand};
use flate2::{write::GzEncoder, Compression};
use reqwest::{header::CONTENT_ENCODING, Client};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::{debug, info, warn};

/// Telemetry Service Test Client
#[derive(Parser, Debug)]
#[clap(name = "telemetry-test-client", version, about)]
struct Args {
    /// Base URL of the telemetry service
    #[clap(
        short,
        long,
        env = "TELEMETRY_SERVICE_URL",
        default_value = "http://localhost:8082"
    )]
    url: String,

    /// Contract name to use for custom contract endpoints
    #[clap(
        short,
        long,
        env = "CONTRACT_NAME",
        default_value = "e2e_test_contract"
    )]
    contract_name: String,

    /// Private key hex (without 0x prefix) for signing auth requests
    /// If not provided, generates a random key
    #[clap(short, long, env = "PRIVATE_KEY_HEX")]
    private_key: Option<String>,

    /// Chain ID
    #[clap(long, env = "CHAIN_ID", default_value = "4")]
    chain_id: u8,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Authenticate and get a JWT token
    Auth,

    /// Send metrics to the telemetry service
    Metrics {
        /// Path to a file containing Prometheus-format metrics
        /// If not provided, sends sample metrics
        #[clap(short, long)]
        file: Option<PathBuf>,

        /// Sample metric name (used when no file provided)
        #[clap(long, default_value = "telemetry_e2e_test_metric")]
        metric_name: String,

        /// Sample metric value (used when no file provided)
        #[clap(long, default_value = "42")]
        metric_value: f64,
    },

    /// Send logs to the telemetry service
    Logs {
        /// Path to a file containing JSON logs (array of strings)
        /// If not provided, sends sample logs
        #[clap(short, long)]
        file: Option<PathBuf>,

        /// Sample log message (used when no file provided)
        #[clap(
            long,
            default_value = "E2E test log message from telemetry-test-client"
        )]
        message: String,

        /// Number of sample logs to send (used when no file provided)
        #[clap(long, default_value = "3")]
        count: usize,
    },

    /// Send custom events to the telemetry service
    Events {
        /// Path to a file containing JSON events (TelemetryDump format)
        /// If not provided, sends sample events
        #[clap(short, long)]
        file: Option<PathBuf>,

        /// Sample event name (used when no file provided)
        #[clap(long, default_value = "TELEMETRY_E2E_TEST_EVENT")]
        event_name: String,
    },

    /// Send all types of data (auth + metrics + logs + events)
    All {
        /// Number of iterations
        #[clap(short, long, default_value = "1")]
        iterations: usize,

        /// Delay between iterations in seconds
        #[clap(short, long, default_value = "1")]
        delay: u64,
    },
}

/// Challenge request for custom contracts
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChallengeRequest {
    pub address: AccountAddress,
    pub chain_id: ChainId,
}

/// Challenge response from server
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChallengeResponse {
    pub challenge: String,
    pub expires_at: u64,
}

/// Authentication request for custom contracts
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CustomAuthRequest {
    pub address: AccountAddress,
    pub chain_id: ChainId,
    pub challenge: String,
    pub signature: Vec<u8>,
    pub public_key: Vec<u8>,
}

/// Authentication response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CustomAuthResponse {
    pub token: String,
}

/// Test client for telemetry service
struct TelemetryTestClient {
    client: Client,
    base_url: String,
    contract_name: String,
    private_key: Ed25519PrivateKey,
    address: AccountAddress,
    chain_id: ChainId,
    token: Option<String>,
}

impl TelemetryTestClient {
    fn new(
        base_url: String,
        contract_name: String,
        private_key_hex: Option<String>,
        chain_id: u8,
    ) -> Result<Self> {
        // Parse or generate private key using aptos-crypto
        let private_key = match private_key_hex {
            Some(hex_str) => {
                let hex_str = hex_str.strip_prefix("0x").unwrap_or(&hex_str);
                let bytes = hex::decode(hex_str)?;
                Ed25519PrivateKey::try_from(&bytes[..])
                    .map_err(|e| anyhow!("Invalid private key: {:?}", e))?
            },
            None => {
                info!("No private key provided, generating random key");
                let mut rng = rand::rngs::OsRng;
                Ed25519PrivateKey::generate(&mut rng)
            },
        };

        // Derive address from public key
        let public_key: Ed25519PublicKey = (&private_key).into();
        let address = aptos_types::account_address::from_public_key(&public_key);

        info!("Using address: {}", address);
        info!("Public key: {}", hex::encode(public_key.to_bytes()));

        Ok(Self {
            client: Client::new(),
            base_url,
            contract_name,
            private_key,
            address,
            chain_id: ChainId::new(chain_id),
            token: None,
        })
    }

    /// Build API path
    fn build_path(&self, path: &str) -> String {
        format!("{}/api/v1/{}", self.base_url, path)
    }

    /// Authenticate and get JWT token using 2-step challenge-response flow
    async fn authenticate(&mut self) -> Result<String> {
        info!("Authenticating with telemetry service...");

        // Step 1: Request challenge from server
        info!("Requesting challenge from server...");
        let challenge_request = ChallengeRequest {
            address: self.address,
            chain_id: self.chain_id,
        };

        let challenge_url = self.build_path(&format!(
            "custom-contract/{}/auth-challenge",
            self.contract_name
        ));
        let challenge_response = self
            .client
            .post(&challenge_url)
            .json(&challenge_request)
            .send()
            .await?;

        if !challenge_response.status().is_success() {
            let status = challenge_response.status();
            let body = challenge_response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Challenge request failed with status {}: {}",
                status,
                body
            ));
        }

        let challenge_resp: ChallengeResponse = challenge_response.json().await?;
        debug!(
            "Received challenge: {} (expires at {})",
            challenge_resp.challenge, challenge_resp.expires_at
        );

        // Step 2: Sign the server-issued challenge using aptos-crypto
        let signature: Ed25519Signature = self
            .private_key
            .sign_arbitrary_message(challenge_resp.challenge.as_bytes());

        // Step 3: Send auth request with signed challenge
        let public_key: Ed25519PublicKey = (&self.private_key).into();
        let auth_request = CustomAuthRequest {
            address: self.address,
            chain_id: self.chain_id,
            challenge: challenge_resp.challenge,
            signature: signature.to_bytes().to_vec(),
            public_key: public_key.to_bytes().to_vec(),
        };

        debug!("Auth request: {:?}", auth_request);

        let auth_url = self.build_path(&format!("custom-contract/{}/auth", self.contract_name));
        let response = self
            .client
            .post(&auth_url)
            .json(&auth_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Auth failed with status {}: {}", status, body));
        }

        let auth_response: CustomAuthResponse = response.json().await?;
        self.token = Some(auth_response.token.clone());

        info!("Authentication successful!");
        debug!("JWT token: {}", auth_response.token);

        Ok(auth_response.token)
    }

    /// Ensure we have a valid token
    async fn ensure_authenticated(&mut self) -> Result<String> {
        match &self.token {
            Some(token) => Ok(token.clone()),
            None => self.authenticate().await,
        }
    }

    /// Send metrics to the telemetry service
    async fn send_metrics(&mut self, metrics: &str) -> Result<()> {
        let token = self.ensure_authenticated().await?;

        info!("Sending metrics ({} bytes)...", metrics.len());
        debug!("Metrics content:\n{}", metrics);

        // Gzip compress the metrics
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(metrics.as_bytes())?;
        let compressed = encoder.finish()?;

        let url = self.build_path(&format!(
            "custom-contract/{}/ingest/metrics",
            self.contract_name
        ));
        let response = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .header(CONTENT_ENCODING, "gzip")
            .body(compressed)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Metrics send failed with status {}: {}",
                status,
                body
            ));
        }

        info!("Metrics sent successfully!");
        Ok(())
    }

    /// Send logs to the telemetry service
    async fn send_logs(&mut self, logs: Vec<String>) -> Result<()> {
        let token = self.ensure_authenticated().await?;

        info!("Sending {} logs...", logs.len());
        debug!("Logs: {:?}", logs);

        // Serialize to JSON and gzip compress
        let json = serde_json::to_string(&logs)?;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(json.as_bytes())?;
        let compressed = encoder.finish()?;

        let url = self.build_path(&format!(
            "custom-contract/{}/ingest/logs",
            self.contract_name
        ));
        let response = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .header(CONTENT_ENCODING, "gzip")
            .body(compressed)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Logs send failed with status {}: {}", status, body));
        }

        info!("Logs sent successfully!");
        Ok(())
    }

    /// Send custom events to the telemetry service
    async fn send_events(&mut self, events: Vec<TelemetryEvent>) -> Result<()> {
        let token = self.ensure_authenticated().await?;

        info!("Sending {} custom events...", events.len());
        debug!("Events: {:?}", events);

        let telemetry_dump = TelemetryDump {
            client_id: format!("test-client-{}", self.address),
            user_id: self.address.to_string(),
            timestamp_micros: SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_micros()
                .to_string(),
            events,
        };

        let url = self.build_path(&format!(
            "custom-contract/{}/ingest/custom-event",
            self.contract_name
        ));
        let response = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(&telemetry_dump)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Events send failed with status {}: {}",
                status,
                body
            ));
        }

        info!("Custom events sent successfully!");
        Ok(())
    }
}

/// Generate sample Prometheus metrics
fn generate_sample_metrics(name: &str, value: f64) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    format!(
        r#"# HELP {name} A test metric from telemetry-test-client
# TYPE {name} gauge
{name}{{source="test_client",environment="e2e_test"}} {value} {timestamp}
"#,
        name = name,
        value = value,
        timestamp = timestamp
    )
}

/// Generate sample logs
fn generate_sample_logs(message: &str, count: usize) -> Vec<String> {
    (0..count)
        .map(|i| {
            serde_json::json!({
                "level": "INFO",
                "message": format!("{} [{}]", message, i),
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "source": "telemetry-test-client",
                "iteration": i
            })
            .to_string()
        })
        .collect()
}

/// Generate sample events
fn generate_sample_events(event_name: &str) -> Vec<TelemetryEvent> {
    let mut params = BTreeMap::new();
    params.insert("source".to_string(), "telemetry-test-client".to_string());
    params.insert("timestamp".to_string(), chrono::Utc::now().to_rfc3339());
    params.insert("test_key".to_string(), "test_value".to_string());

    vec![TelemetryEvent {
        name: event_name.to_string(),
        params,
    }]
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("telemetry_test_client=info".parse()?),
        )
        .init();

    let args = Args::parse();

    info!("Telemetry Test Client");
    info!("URL: {}", args.url);
    info!("Contract: {}", args.contract_name);
    info!("Chain ID: {}", args.chain_id);

    let mut client = TelemetryTestClient::new(
        args.url,
        args.contract_name,
        args.private_key,
        args.chain_id,
    )?;

    match args.command {
        Commands::Auth => {
            let token = client.authenticate().await?;
            println!("JWT Token: {}", token);
        },

        Commands::Metrics {
            file,
            metric_name,
            metric_value,
        } => {
            let metrics = match file {
                Some(path) => {
                    info!("Reading metrics from file: {:?}", path);
                    fs::read_to_string(path)?
                },
                None => {
                    info!("Generating sample metrics");
                    generate_sample_metrics(&metric_name, metric_value)
                },
            };
            client.send_metrics(&metrics).await?;
        },

        Commands::Logs {
            file,
            message,
            count,
        } => {
            let logs = match file {
                Some(path) => {
                    info!("Reading logs from file: {:?}", path);
                    serde_json::from_str(&fs::read_to_string(path)?)?
                },
                None => {
                    info!("Generating sample logs");
                    generate_sample_logs(&message, count)
                },
            };
            client.send_logs(logs).await?;
        },

        Commands::Events { file, event_name } => {
            let events = match file {
                Some(path) => {
                    info!("Reading events from file: {:?}", path);
                    let dump: TelemetryDump = serde_json::from_str(&fs::read_to_string(path)?)?;
                    dump.events
                },
                None => {
                    info!("Generating sample events");
                    generate_sample_events(&event_name)
                },
            };
            client.send_events(events).await?;
        },

        Commands::All { iterations, delay } => {
            for i in 0..iterations {
                info!("=== Iteration {}/{} ===", i + 1, iterations);

                // Authenticate (will reuse token after first iteration)
                if i == 0 {
                    client.authenticate().await?;
                }

                // Send metrics with telemetry_e2e_test_ prefix
                let metrics =
                    generate_sample_metrics("telemetry_e2e_test_iteration_metric", i as f64);
                if let Err(e) = client.send_metrics(&metrics).await {
                    warn!("Failed to send metrics: {}", e);
                }

                // Send logs
                let logs =
                    generate_sample_logs(&format!("telemetry_e2e_test iteration {} log", i), 2);
                if let Err(e) = client.send_logs(logs).await {
                    warn!("Failed to send logs: {}", e);
                }

                // Send events with telemetry_e2e_test_ prefix
                let events =
                    generate_sample_events(&format!("TELEMETRY_E2E_TEST_EVENT_ITER_{}", i));
                if let Err(e) = client.send_events(events).await {
                    warn!("Failed to send events: {}", e);
                }

                if i < iterations - 1 {
                    info!("Waiting {} seconds before next iteration...", delay);
                    tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
                }
            }
        },
    }

    info!("Done!");
    Ok(())
}
