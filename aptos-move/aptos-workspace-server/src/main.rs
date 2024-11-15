// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use common::make_shared;
use futures::TryFutureExt;
use services::{
    docker_common::create_docker_network, indexer_api::start_indexer_api,
    processors::start_all_processors,
};
use std::path::Path;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

mod common;
mod services;

async fn run_all_services(test_dir: &Path) -> Result<()> {
    let instance_id = Uuid::new_v4();

    // Step 0: register the signal handler for ctrl-c.
    let shutdown = CancellationToken::new();
    {
        let shutdown = shutdown.clone();
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.unwrap();

            println!("\nCtrl-C received. Shutting down services. This may take a while.\n");

            shutdown.cancel();
        });
    }

    // Step 1: spawn all services.
    // Node
    let (fut_node_api, fut_indexer_grpc, fut_node_finish) = services::node::start_node(test_dir)?;

    let fut_node_api = make_shared(fut_node_api);
    let fut_indexer_grpc = make_shared(fut_indexer_grpc);

    // Faucet
    let (fut_faucet, fut_faucet_finish) = services::faucet::start_faucet(
        test_dir.to_owned(),
        fut_node_api.clone(),
        fut_indexer_grpc.clone(),
    );

    // Docker Network
    let docker_network_name = format!("aptos-workspace-{}-postgres", instance_id);
    let (fut_docker_network, fut_docker_network_clean_up) =
        create_docker_network(shutdown.clone(), docker_network_name);

    // Postgres
    let (fut_postgres, fut_postgres_finish, fut_postgres_clean_up) =
        services::postgres::start_postgres(
            shutdown.clone(),
            fut_docker_network.clone(),
            instance_id,
        );
    let fut_postgres = make_shared(fut_postgres);

    // Processors
    let (fut_all_processors_ready, fut_any_processor_finish) = start_all_processors(
        fut_node_api.clone(),
        fut_indexer_grpc.clone(),
        fut_postgres.clone(),
    );
    let fut_all_processors_ready = make_shared(fut_all_processors_ready);

    // Indexer API
    let (fut_indexer_api, fut_indexer_api_finish, fut_indexer_api_clean_up) = start_indexer_api(
        instance_id,
        shutdown.clone(),
        fut_docker_network.clone(),
        fut_postgres.clone(),
        fut_all_processors_ready.clone(),
    );

    // Step 2: wait for all services to be up.
    let all_services_up = async move {
        tokio::try_join!(
            fut_node_api.map_err(anyhow::Error::msg),
            fut_indexer_grpc.map_err(anyhow::Error::msg),
            fut_faucet,
            fut_postgres.map_err(anyhow::Error::msg),
            fut_all_processors_ready.map_err(anyhow::Error::msg),
            fut_indexer_api,
        )
    };
    let clean_up_all = async move {
        fut_indexer_api_clean_up.await;
        fut_postgres_clean_up.await;
        fut_docker_network_clean_up.await;
    };
    tokio::select! {
        _ = shutdown.cancelled() => {
            eprintln!("Running shutdown steps");
            clean_up_all.await;

            return Ok(())
        }
        res = all_services_up => {
            match res.context("one or more services failed to start") {
                Ok(_) => println!("ALL SERVICES UP"),
                Err(err) => {
                    eprintln!("\nOne or more services failed to start, running shutdown steps\n");
                    clean_up_all.await;

                    return Err(err)
                }
            }
        }
    }

    // Step 3: wait for services to stop.
    tokio::select! {
        _ = shutdown.cancelled() => (),
        res = fut_node_finish => {
            eprintln!("Node exited unexpectedly");
            if let Err(err) = res {
                eprintln!("Error: {}", err);
            }
        }
        res = fut_faucet_finish => {
            eprintln!("Faucet exited unexpectedly");
            if let Err(err) = res {
                eprintln!("Error: {}", err);
            }
        }
        res = fut_postgres_finish => {
            eprintln!("Postgres exited unexpectedly");
            if let Err(err) = res {
                eprintln!("Error: {}", err);
            }
        }
        res = fut_any_processor_finish => {
            eprintln!("One of the processors exited unexpectedly");
            if let Err(err) = res {
                eprintln!("Error: {}", err);
            }
        }
        res = fut_indexer_api_finish => {
            eprintln!("Indexer API exited unexpectedly");
            if let Err(err) = res {
                eprintln!("Error: {}", err);
            }
        }
    }

    eprintln!("Running shutdown steps");
    clean_up_all.await;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let test_dir = tempfile::tempdir()?;

    println!("Test directory: {}", test_dir.path().display());

    run_all_services(test_dir.path()).await?;

    println!("Finished running all services");

    Ok(())
}
