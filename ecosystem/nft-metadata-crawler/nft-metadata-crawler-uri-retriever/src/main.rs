// Copyright © Aptos Foundation

use anyhow::Context;
use aptos_indexer_grpc_server_framework::{RunnableConfig, ServerArgs};
use clap::Parser;
use google_cloud_auth::project::{create_token_source, Config};
use google_cloud_googleapis::pubsub::v1::publisher_client::PublisherClient;
use nft_metadata_crawler_utils::pubsub::publish_uris;
use serde::{Deserialize, Serialize};
use std::{
    env,
    fs::File,
    io::{BufRead, BufReader},
};
use tonic::transport::{Channel, ClientTlsConfig};
use tracing::{error, info};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct URIRetrieverConfig {
    pub google_application_credentials: String,
    pub topic_name: String,
}

// Temporary function to process CSV file
fn process_file() -> anyhow::Result<Vec<String>> {
    let file = File::open("./test.csv").context("Failed to open file")?;
    let reader = BufReader::new(file);

    reader
        .lines()
        .map(|line| line.context("Failed to read line"))
        .collect()
}

// Publishes URIs from CSV to PubSub
async fn send_publish_uris(
    start: i32,
    end: i32,
    force: bool,
    grpc_client: &mut PublisherClient<Channel>,
    topic_name: String,
    token: String,
) -> anyhow::Result<String> {
    let links = process_file()?;
    publish_uris(links, force, grpc_client, topic_name, token).await?;

    Ok(format!("{} to {}", start, end))
}

#[async_trait::async_trait]
impl RunnableConfig for URIRetrieverConfig {
    async fn run(&self) -> anyhow::Result<()> {
        env::set_var(
            "GOOGLE_APPLICATION_CREDENTIALS",
            self.google_application_credentials.clone(),
        );

        let ts = create_token_source(Config {
            audience: None,
            scopes: Some(&["https://www.googleapis.com/auth/cloud-platform"]),
            sub: None,
        })
        .await
        .expect("No token source");

        // Establish gRPC client
        let channel = Channel::from_static("https://pubsub.googleapis.com")
            .tls_config(ClientTlsConfig::new().domain_name("pubsub.googleapis.com"))
            .expect("Unable to create channel")
            .connect()
            .await
            .expect("Unable to connect to pubsub");

        let mut grpc_client = PublisherClient::new(channel);

        // Parse start and end transaction_versions
        // let mut parts = req.uri().path().trim_start_matches('/').split('/');
        let start = 1;
        let end = 2;
        let force = false; // matches!(parts.next(), Some("force"));

        // Query URIs from database and publish to PubSub
        match send_publish_uris(
            start,
            end,
            force,
            &mut grpc_client,
            self.topic_name.clone(),
            ts.token().await.expect("No token").access_token,
        )
        .await
        {
            Ok(res) => {
                info!(res);
            },
            Err(err) => {
                error!("{}", err);
            },
        }

        Ok(())
    }

    fn get_server_name(&self) -> String {
        "uri-retriever".to_string()
    }
}

// Main URI Retriever server flow
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = ServerArgs::parse();
    args.run::<URIRetrieverConfig>().await
}
