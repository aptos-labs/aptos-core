// Copyright © Aptos Foundation

use aptos_indexer_grpc_server_framework::{RunnableConfig, ServerArgs};
use crossbeam_channel::{bounded, Receiver, Sender};
use diesel::{
    r2d2::{ConnectionManager, Pool},
    PgConnection,
};
use google_cloud_googleapis::pubsub::v1::subscriber_client::SubscriberClient;
use nft_metadata_crawler_parser::{establish_connection_pool, parser::Parser};
use nft_metadata_crawler_utils::{
    get_token_source,
    pubsub::{consume_uris, send_acks},
    NFTMetadataCrawlerEntry,
};
use serde::{Deserialize, Serialize};
use std::{env, sync::Arc};
use tokio::{
    sync::{Mutex, Semaphore},
    task::JoinHandle,
};
use tonic::transport::{Channel, ClientTlsConfig};
use tracing::{error, info};

/**
 * Structs to hold config from YAML
 */
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParserConfig {
    pub google_application_credentials: String,
    pub bucket: String,
    pub subscription_name: String,
    pub database_url: String,
    pub cdn_prefix: String,
    pub ipfs_prefix: String,
}

/**
 * Subscribes to PubSub and sends URIs to Channel
 */
async fn process_response(
    sender: Sender<(NFTMetadataCrawlerEntry, String)>,
    mut grpc_client: SubscriberClient<Channel>,
    subscription_name: String,
) -> anyhow::Result<()> {
    let ts = get_token_source().await;
    loop {
        // Pulls an entry from PubSub
        let entries = consume_uris(
            10,
            &mut grpc_client,
            subscription_name.clone(),
            ts.token().await?.access_token,
        )
        .await?;

        // Parses entry and sends to Channel
        for msg in entries.into_inner().received_messages {
            if let Some(entry_option) = msg.message {
                let entry = entry_option.data;
                let ack = msg.ack_id;
                let entry = NFTMetadataCrawlerEntry::new(String::from_utf8(entry)?)?;
                sender.send((entry.clone(), ack))?;
            }
        }
    }
}

/**
 * Spawns a worker to pull from Channel and perform parsing operations
 */
async fn spawn_parser(
    id: usize,
    semaphore: Arc<Semaphore>,
    receiver: Arc<Mutex<Receiver<(NFTMetadataCrawlerEntry, String)>>>,
    conn: Pool<ConnectionManager<PgConnection>>,
    bucket: String,
    mut grpc_client: SubscriberClient<Channel>,
    subscription_name: String,
    cdn_prefix: String,
) -> anyhow::Result<()> {
    let ts = get_token_source().await;
    loop {
        let _ = semaphore.acquire().await?;

        // Pulls entry from Channel
        let (entry, ack) = receiver.lock().await.recv()?;
        let token = ts.token().await?.access_token;

        // Parses entry
        info!(worker_id = id, "Received entry");
        let mut parser = Parser::new(
            entry,
            bucket.clone(),
            token.clone(),
            conn.get()?,
            cdn_prefix.clone(),
        );
        parser.parse().await?;

        // Sends ack to PubSub
        info!(worker_id = id, "Finished parsing");
        send_acks(
            vec![ack],
            &mut grpc_client,
            subscription_name.clone(),
            token,
        )
        .await?;

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
}

/**
 * Main driver function
 */
#[async_trait::async_trait]
impl RunnableConfig for ParserConfig {
    async fn run(&self) -> anyhow::Result<()> {
        let pool = establish_connection_pool(self.database_url.clone());

        env::set_var("IPFS_PREFIX", self.ipfs_prefix.clone());
        env::set_var(
            "GOOGLE_APPLICATION_CREDENTIALS",
            self.google_application_credentials.clone(),
        );

        // Establish gRPC client
        let channel = Channel::from_static("https://pubsub.googleapis.com")
            .tls_config(ClientTlsConfig::new().domain_name("pubsub.googleapis.com"))?
            .connect()
            .await?;
        let grpc_client = SubscriberClient::new(channel);

        // Create workers
        let num_workers = 10;
        let (sender, receiver) = bounded::<(NFTMetadataCrawlerEntry, String)>(20);
        let receiver = Arc::new(Mutex::new(receiver));
        let semaphore = Arc::new(Semaphore::new(num_workers));

        // Spawn producer
        let producer = tokio::spawn(process_response(
            sender,
            grpc_client.clone(),
            self.subscription_name.clone(),
        ));

        // Spawns workers
        let mut workers: Vec<JoinHandle<anyhow::Result<()>>> = Vec::new();
        for id in 0..num_workers {
            let worker = tokio::spawn(spawn_parser(
                id,
                Arc::clone(&semaphore),
                Arc::clone(&receiver),
                pool.clone(),
                self.bucket.clone(),
                grpc_client.clone(),
                self.subscription_name.clone(),
                self.cdn_prefix.clone(),
            ));

            workers.push(worker);
        }

        match producer.await {
            Ok(_) => (),
            Err(e) => error!("Producer error: {:?}", e),
        }

        for worker in workers {
            match worker.await {
                Ok(_) => (),
                Err(e) => error!("Worker error: {:?}", e),
            }
        }
        Ok(())
    }

    fn get_server_name(&self) -> String {
        "parser".to_string()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = <ServerArgs as clap::Parser>::parse();
    args.run::<ParserConfig>().await
}
