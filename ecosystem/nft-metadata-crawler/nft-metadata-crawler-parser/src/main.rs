// Copyright © Aptos Foundation

use std::{env, sync::Arc};

use crossbeam_channel::{bounded, Receiver, Sender};
use diesel::{
    r2d2::{ConnectionManager, Pool},
    PgConnection,
};
use google_cloud_googleapis::pubsub::v1::subscriber_client::SubscriberClient;
use nft_metadata_crawler_parser::{establish_connection_pool, parser::Parser};
use nft_metadata_crawler_utils::{
    get_token_source, load_config_from_yaml,
    pubsub::{consume_uris, send_acks},
    NFTMetadataCrawlerEntry,
};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::{Mutex, Semaphore},
    task::JoinHandle,
};
use tonic::transport::{Channel, ClientTlsConfig};
use tracing::{error, info};

#[derive(clap::Parser)]
pub struct ServerArgs {
    #[clap(short, long, value_parser)]
    pub config_path: String,
}

// Structs to hold config from YAML
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParserConfig {
    pub google_application_credentials: String,
    pub bucket: String,
    pub subscription_name: String,
    pub database_url: String,
}

// Subscribes to PubSub and sends URIs to Channel
async fn process_response(
    sender: Sender<(NFTMetadataCrawlerEntry, String)>,
    mut grpc_client: SubscriberClient<Channel>,
    subscription_name: String,
) -> anyhow::Result<()> {
    let ts = get_token_source().await;
    loop {
        let token = ts.token().await.expect("Unable to get token").access_token;

        // Pulls an entry from PubSub
        let entries = consume_uris(1, &mut grpc_client, subscription_name.clone(), token).await?;

        // Parses entry and sends to Channel
        for msg in entries.into_inner().received_messages {
            if let Some(entry_option) = msg.message {
                let entry = entry_option.data;
                let ack = msg.ack_id;
                let entry = NFTMetadataCrawlerEntry::new(String::from_utf8(entry)?)?;
                sender.send((entry.clone(), ack))?;
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

// Spawns a worker to pull from Channel and perform parsing operations
async fn spawn_parser(
    id: usize,
    semaphore: Arc<Semaphore>,
    receiver: Arc<Mutex<Receiver<(NFTMetadataCrawlerEntry, String)>>>,
    conn: Pool<ConnectionManager<PgConnection>>,
    bucket: String,
    mut grpc_client: SubscriberClient<Channel>,
    subscription_name: String,
) -> anyhow::Result<()> {
    let ts = get_token_source().await;
    loop {
        let _ = semaphore.acquire().await?;
        let (entry, ack) = {
            let lock = receiver.lock();
            let data = lock.await.recv()?;
            data
        };

        let token = ts.token().await.expect("Unable to get token").access_token;

        info!("Worker {} got entry", id);
        let mut parser = Parser::new(entry, bucket.clone(), token.clone(), conn.get()?);
        parser.parse().await?;

        info!("Worker {} finished parsing", id);
        send_acks(
            vec![ack],
            &mut grpc_client,
            subscription_name.clone(),
            token,
        )
        .await?;

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

#[tokio::main]
async fn main() {
    info!("Starting Parser");

    // Load configs
    let args = <ServerArgs as clap::Parser>::parse();
    let config =
        load_config_from_yaml::<ParserConfig>(args.config_path).expect("Unable to load config");
    let pool = establish_connection_pool(config.database_url.clone());

    env::set_var(
        "GOOGLE_APPLICATION_CREDENTIALS",
        config.google_application_credentials,
    );

    // Establish gRPC client
    let channel = Channel::from_static("https://pubsub.googleapis.com")
        .tls_config(ClientTlsConfig::new().domain_name("pubsub.googleapis.com"))
        .expect("Unable to create channel")
        .connect()
        .await
        .expect("Unable to connect to pubsub");

    let grpc_client = SubscriberClient::new(channel);

    // Create workers
    let num_workers = 10;
    let (sender, receiver) = bounded::<(NFTMetadataCrawlerEntry, String)>(20);
    let receiver = Arc::new(Mutex::new(receiver));
    let semaphore = Arc::new(Semaphore::new(num_workers));

    // Spawn your producer.
    let producer = tokio::spawn(process_response(
        sender,
        grpc_client.clone(),
        config.subscription_name.clone(),
    ));

    // Spawns workers
    let mut workers: Vec<JoinHandle<anyhow::Result<()>>> = Vec::new();
    for id in 0..num_workers {
        let semaphore_clone = Arc::clone(&semaphore);
        let receiver_clone = Arc::clone(&receiver);

        let worker = tokio::spawn(spawn_parser(
            id,
            semaphore_clone,
            receiver_clone,
            pool.clone(),
            config.bucket.clone(),
            grpc_client.clone(),
            config.subscription_name.clone(),
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
}
