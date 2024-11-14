// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use common::{make_shared, IP_LOCAL_HOST};
use futures::TryFutureExt;
use services::processors::start_all_processors;
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

    // Postgres
    let (fut_postgres, fut_postgres_finish, fut_postgres_cancel) =
        services::postgres::start_postgres(instance_id)?;
    let fut_postgres = make_shared(fut_postgres);

    // Processors
    let (fut_all_processors_ready, fut_any_processor_finish) = start_all_processors(
        fut_node_api.clone(),
        fut_indexer_grpc.clone(),
        fut_postgres.clone(),
    );

    // Step 2: wait for all services to be up.
    let all_services_up = async move {
        tokio::try_join!(
            fut_node_api.map_err(anyhow::Error::msg),
            fut_indexer_grpc.map_err(anyhow::Error::msg),
            fut_faucet,
            fut_postgres.map_err(anyhow::Error::msg),
            fut_all_processors_ready,
        )
    };
    tokio::select! {
        _ = shutdown.cancelled() => {
            eprintln!("Running shutdown steps");
            fut_postgres_cancel.await?;

            return Ok(())
        }
        res = all_services_up => {
            res.context("one or more services failed to start")?;

            println!(
                "Indexer API is ready. Endpoint: http://{}:0/",
                IP_LOCAL_HOST
            );

            println!("ALL SERVICES UP");
        }
    }

    // Step 3: wait for services to stop.
    tokio::select! {
        _ = shutdown.cancelled() => (),
        res = fut_node_finish => {
            eprintln!("Node existed unexpectedly");
            if let Err(err) = res {
                eprintln!("Error: {}", err);
            }
        }
        res = fut_faucet_finish => {
            eprintln!("Faucet existed unexpectedly");
            if let Err(err) = res {
                eprintln!("Error: {}", err);
            }
        }
        res = fut_postgres_finish => {
            eprintln!("Postgres existed unexpectedly");
            if let Err(err) = res {
                eprintln!("Error: {}", err);
            }
        }
        res = fut_any_processor_finish => {
            eprintln!("One of the processors existed unexpectedly");
            if let Err(err) = res {
                eprintln!("Error: {}", err);
            }
        }
    }

    eprintln!("Running shutdown steps");
    fut_postgres_cancel.await?;

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
