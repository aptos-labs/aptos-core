// Copyright © Aptos Foundation

use ::futures::future;
use diesel::{
    r2d2::{ConnectionManager, Pool},
    PgConnection, QueryDsl, RunQueryDsl,
};
use google_cloud_auth::{
    project::{create_token_source, Config},
    token_source::TokenSource,
};
use google_cloud_googleapis::pubsub::v1::{subscriber_client::SubscriberClient, PullRequest};
use nft_metadata_crawler_parser::{
    db::upsert_entry, establish_connection_pool, models::NFTMetadataCrawlerEntry, parser::Parser,
    schema::nft_metadata_crawler_entry,
};
use nft_metadata_crawler_utils::pubsub::send_ack;
use reqwest::Client;
use std::{env, error::Error, time::Duration};
use tokio::task::JoinHandle;
use tonic::{
    metadata::MetadataValue,
    transport::{Channel, ClientTlsConfig},
    Request,
};

async fn process_response(
    res: Vec<String>,
    acks: &[String],
    ts: &dyn TokenSource,
    subscription_name: &String,
    pool: &Pool<ConnectionManager<PgConnection>>,
) -> Result<Vec<(NFTMetadataCrawlerEntry, bool)>, Box<dyn Error + Send + Sync>> {
    let mut uris: Vec<(NFTMetadataCrawlerEntry, bool)> = Vec::new();
    for (entry, ack) in res.into_iter().zip(acks.iter()) {
        let (entry_struct, force) = NFTMetadataCrawlerEntry::new(entry)?;
        let mut conn = pool.get()?;
        if nft_metadata_crawler_entry::table
            .find(&entry_struct.token_data_id)
            .first::<NFTMetadataCrawlerEntry>(&mut conn)
            .is_ok()
        {
            if force {
                println!(
                    "Transaction Version {}: Found NFT entry but forcing parse",
                    entry_struct.last_transaction_version
                );
            } else {
                println!(
                    "Transaction Version {}: Skipping parse",
                    entry_struct.last_transaction_version
                );
                let client = Client::new();
                match send_ack(&client, ts, subscription_name, ack).await {
                    Ok(_) => println!(
                        "Transaction Version {}: Successfully acked",
                        entry_struct.last_transaction_version
                    ),
                    Err(e) => println!(
                        "Transaction Version {}: Error acking - {}",
                        entry_struct.last_transaction_version, e
                    ),
                }
                continue;
            }
        }
        uris.push((upsert_entry(&mut pool.get()?, entry_struct)?, force))
    }
    Ok(uris)
}

fn spawn_parser(
    uri: NFTMetadataCrawlerEntry,
    pool: &Pool<ConnectionManager<PgConnection>>,
    subscription_name: String,
    ack: String,
    bucket: String,
    force: bool,
) -> JoinHandle<()> {
    match pool.get() {
        Ok(mut conn) => tokio::spawn(async move {
            let ts = create_token_source(Config {
                audience: None,
                scopes: Some(&["https://www.googleapis.com/auth/cloud-platform"]),
                sub: None,
            })
            .await
            .expect("No token source");

            let mut parser = Parser::new(uri, Some((400, 400)), bucket, force, ts.as_ref());

            match parser.parse(&mut conn).await {
                Ok(()) => {
                    let client = Client::builder()
                        .timeout(Duration::from_secs(10))
                        .build()
                        .expect("Unable to create client");
                    match send_ack(&client, ts.as_ref(), &subscription_name, &ack).await {
                        Ok(_) => {
                            println!(
                                "Transaction Version {}: Successfully acked",
                                parser.entry.last_transaction_version
                            )
                        },
                        Err(e) => println!(
                            "Transaction Version {}: Error acking - {}",
                            parser.entry.last_transaction_version, e
                        ),
                    }
                },
                Err(e) => println!(
                    "Transaction Version {}: Error parsing - {}",
                    parser.entry.last_transaction_version, e
                ),
            }
        }),
        Err(_) => tokio::spawn(async move { println!("Error getting connection from pool") }),
    }
}

#[allow(deprecated)]
#[tokio::main]
async fn main() {
    println!("Starting parser");
    let pool = establish_connection_pool();

    let subscription_name = env::var("SUBSCRIPTION_NAME").expect("No SUBSCRIPTION NAME");
    let bucket = env::var("BUCKET").expect("No BUCKET");
    let ts = create_token_source(Config {
        audience: None,
        scopes: Some(&["https://www.googleapis.com/auth/cloud-platform"]),
        sub: None,
    })
    .await
    .expect("No token source");

    let channel = Channel::from_static("https://pubsub.googleapis.com")
        .tls_config(ClientTlsConfig::new().domain_name("pubsub.googleapis.com"))
        .expect("Unable to create channel")
        .connect()
        .await
        .expect("Unable to connect to pubsub");

    let mut grpc_client = SubscriberClient::new(channel);

    let make_request = || async {
        let mut request = Request::new(PullRequest {
            subscription: subscription_name.clone(),
            max_messages: 10,
            return_immediately: false,
        });

        request.metadata_mut().insert(
            "authorization",
            MetadataValue::from_str(
                format!(
                    "Bearer {}",
                    ts.token().await.expect("No token").access_token
                )
                .as_str(),
            )
            .expect("Unable to create metadata"),
        );

        request
    };

    while let Ok(response) = grpc_client.pull(make_request().await).await {
        let res = response.into_inner();
        let mut links = Vec::new();
        for pubsub_msg in res.received_messages {
            let message = pubsub_msg.message;
            if let Some(msg) = message {
                links.push((
                    String::from_utf8(msg.data).expect("Unable to parse message"),
                    String::from(pubsub_msg.ack_id),
                ));
            }
        }

        let (res, acks): (Vec<String>, Vec<String>) = links.into_iter().unzip();
        match process_response(res, &acks, ts.as_ref(), &subscription_name, &pool).await {
            Ok(uris) => {
                let handles: Vec<_> = uris
                    .into_iter()
                    .zip(acks.into_iter())
                    .map(|((uri, force), ack)| {
                        spawn_parser(
                            uri,
                            &pool,
                            subscription_name.clone(),
                            ack,
                            bucket.clone(),
                            force,
                        )
                    })
                    .collect();
                if (future::try_join_all(handles).await).is_ok() {
                    println!("SUCCESS");
                }
            },
            Err(e) => println!("Error processing response: {}", e),
        };
    }
}
