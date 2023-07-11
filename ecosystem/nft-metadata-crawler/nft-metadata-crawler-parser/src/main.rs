// Copyright © Aptos Foundation

use std::{env, error::Error};

use ::futures::future;
use diesel::{
    r2d2::{ConnectionManager, Pool},
    PgConnection, QueryDsl, RunQueryDsl,
};
use google_cloud_auth::{
    project::{create_token_source, Config},
    token_source::TokenSource,
};
use nft_metadata_crawler_parser::{
    db::upsert_entry, establish_connection_pool, models::NFTMetadataCrawlerEntry, parser::Parser,
    schema::nft_metadata_crawler_entry,
};
use nft_metadata_crawler_utils::pubsub::{consume_from_queue, send_ack};
use reqwest::Client;
use tokio::task::JoinHandle;

async fn process_response(
    res: Vec<String>,
    acks: &Vec<String>,
    ts: &Box<dyn TokenSource>,
    subscription_name: &String,
    pool: &Pool<ConnectionManager<PgConnection>>,
) -> Result<Vec<(NFTMetadataCrawlerEntry, bool)>, Box<dyn Error + Send + Sync>> {
    let mut uris: Vec<(NFTMetadataCrawlerEntry, bool)> = Vec::new();
    for (entry, ack) in res.into_iter().zip(acks.into_iter()) {
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
                match send_ack(&client, ts.as_ref(), &subscription_name, &ack).await {
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
                    let client = Client::new();
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

#[tokio::main]
async fn main() {
    println!("Starting parser");
    let pool = establish_connection_pool();
    let client = Client::new();
    let subscription_name = env::var("SUBSCRIPTION_NAME").expect("No SUBSCRIPTION NAME");
    let bucket = env::var("BUCKET").expect("No BUCKET");
    let ts = create_token_source(Config {
        audience: None,
        scopes: Some(&["https://www.googleapis.com/auth/cloud-platform"]),
        sub: None,
    })
    .await
    .expect("No token source");

    while let Ok(r) = consume_from_queue(&client, ts.as_ref(), &subscription_name).await {
        let (res, acks): (Vec<String>, Vec<String>) = r.into_iter().unzip();
        match process_response(res, &acks, &ts, &subscription_name, &pool).await {
            Ok(uris) => {
                let handles: Vec<_> = uris
                    .into_iter()
                    .zip(acks.into_iter())
                    .into_iter()
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
                if let Ok(_) = future::try_join_all(handles).await {
                    println!("SUCCESS");
                }
            },
            Err(e) => println!("Error processing response: {}", e),
        };
    }
}
