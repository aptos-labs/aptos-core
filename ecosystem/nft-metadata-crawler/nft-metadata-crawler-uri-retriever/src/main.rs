// Copyright © Aptos Foundation

use anyhow::Context;
use aptos_indexer_grpc_server_framework::{RunnableConfig, ServerArgs};
use clap::Parser;
use google_cloud_googleapis::pubsub::v1::publisher_client::PublisherClient;
use nft_metadata_crawler_utils::{get_token_source, pubsub::publish_uris};
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

/// Function to process CSV file
/// TODO: Remove for production, integrate with DB
fn process_file() -> anyhow::Result<Vec<String>> {
    let file = File::open("./test.csv").context("Failed to open file")?;
    let reader = BufReader::new(file);

    reader
        .lines()
        .map(|line| line.context("Failed to read line"))
        .collect()
}

/// Publishes URIs from CSV to PubSub
/// TODO: Remove for production, integrate with DB
async fn send_publish_uris(
    start: i32,
    end: i32,
    force: bool,
    grpc_client: &mut PublisherClient<Channel>,
    topic_name: String,
    token: String,
) -> anyhow::Result<String> {
    let links = process_file()?;
    info!("Publishing {} links to PubSub", links.len());
    publish_uris(links, force, grpc_client, topic_name, token).await?;

    Ok(format!("Successfully published {} to {}", start, end))
}

#[async_trait::async_trait]
impl RunnableConfig for URIRetrieverConfig {
    async fn run(&self) -> anyhow::Result<()> {
        env::set_var(
            "GOOGLE_APPLICATION_CREDENTIALS",
            self.google_application_credentials.clone(),
        );

        let ts = get_token_source().await;

        // Establish gRPC client
        let channel = Channel::from_static("https://pubsub.googleapis.com")
            .tls_config(ClientTlsConfig::new().domain_name("pubsub.googleapis.com"))?
            .connect()
            .await?;

        let mut grpc_client = PublisherClient::new(channel);

        // Temporarily stub the parsing of start and end transaction versions and force flag from request
        // TODO: Handle parsing of request/trigger for start and end transaction versions and force flag
        let start = 1;
        let end = 2;
        let force = false;

        // Query URIs from database and publish to PubSub
        match send_publish_uris(
            start,
            end,
            force,
            &mut grpc_client,
            self.topic_name.clone(),
            ts.token().await?.access_token,
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
        "uriretriever".to_string()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = ServerArgs::parse();
    args.run::<URIRetrieverConfig>().await
}
