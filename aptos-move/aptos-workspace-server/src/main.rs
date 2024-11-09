// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use futures::{future::Shared, FutureExt};
use std::{
    future::Future,
    net::{IpAddr, Ipv4Addr},
    path::Path,
    sync::Arc,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

mod services;

const IP_LOCAL_HOST: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

/// Converts a future into a shared one by putting the error into an Arc.
fn make_shared<F, T, E>(fut: F) -> Shared<impl Future<Output = Result<T, Arc<E>>>>
where
    T: Clone,
    F: Future<Output = Result<T, E>>,
{
    fut.map(|r| r.map_err(|err| Arc::new(err))).shared()
}

async fn run_all_services(test_dir: &Path) -> Result<()> {
    let instance_id = Uuid::new_v4();

    // Step 0: register the signal handler for ctrl-c.
    let shutdown = CancellationToken::new();
    {
        let shutdown = shutdown.clone();
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.unwrap();

            println!("Ctrl-C received. Shutting down services. This may take a while.");

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
    // Step 2: wait for all services to be up.
    let all_services_up =
        async move { tokio::join!(fut_node_api, fut_indexer_grpc, fut_faucet, fut_postgres) };
    tokio::select! {
        _ = shutdown.cancelled() => {
            eprintln!("Running shutdown steps");
            fut_postgres_cancel.await?;

            return Ok(())
        }
        (res_node_api, res_indexer_grpc, res_faucet, res_postgres) = all_services_up => {
            res_node_api
                .map_err(anyhow::Error::msg)
                .context("failed to start node api")?;
            res_indexer_grpc
                .map_err(anyhow::Error::msg)
                .context("failed to start node api")?;
            res_faucet.context("failed to start faucet")?;
            res_postgres.context("failed to start postgres")?;

            println!(
                "Indexer API is ready. Endpoint: http://{}:0/",
                IP_LOCAL_HOST
            );

            println!("ALL SERVICES STARTED SUCCESSFULLY");
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
