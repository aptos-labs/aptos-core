// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    health_checker::HealthChecker,
    traits::{ServiceManager, ShutdownStep},
    utils::{confirm_docker_available, delete_container, pull_docker_image},
    RunLocalTestnet,
};
use crate::node::local_testnet::utils::{
    get_docker, setup_docker_logging, KillContainerShutdownStep,
};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use bollard::{
    container::{Config, CreateContainerOptions, StartContainerOptions, WaitContainerOptions},
    models::{HostConfig, PortBinding},
};
use clap::Parser;
use diesel_async::{pg::AsyncPgConnection, AsyncConnection, RunQueryDsl};
use futures::TryStreamExt;
use maplit::{hashmap, hashset};
use std::{collections::HashSet, path::PathBuf};
use tracing::{info, warn};

const POSTGRES_CONTAINER_NAME: &str = "local-testnet-postgres";
const POSTGRES_IMAGE: &str = "postgres:14.9";
const POSTGRES_DEFAULT_PORT: u16 = 5432;

/// Args related to running postgres in the local testnet.
#[derive(Clone, Debug, Parser)]
pub struct PostgresArgs {
    /// This is the database to connect to, both when --use-host-postgres is set
    /// and when it is not (i.e. when postgres is running in a container).
    #[clap(long, default_value = "local_testnet")]
    pub postgres_database: String,

    /// The user to connect as. If --use-host-postgres is set, we expect this user to
    /// exist already.
    #[clap(long, default_value = "postgres")]
    pub postgres_user: String,

    /// This is the port to use for the postgres instance when --use-host-postgres
    /// is not set (i.e. we are running a postgres instance in a container).
    #[clap(long, default_value_t = 5433)]
    pub postgres_port: u16,

    /// If set, connect to the postgres instance specified by the rest of the
    /// `postgres_args` (e.g. --host-postgres-host, --host-postgres-user, etc) rather
    /// than running a new one with Docker. This can be used to connect to an existing
    /// postgres instance running on the host system. Do not include the database.
    ///
    /// WARNING: Any existing database it finds (based on --postgres-database) will be
    /// dropped.
    #[clap(long, requires = "with_indexer_api")]
    pub use_host_postgres: bool,

    /// When --use-host-postgres is set, this is the port to connect to.
    #[clap(long, default_value_t = 5432)]
    pub host_postgres_port: u16,

    /// When --use-host-postgres is set, this is the password to connect with.
    #[clap(long)]
    pub host_postgres_password: Option<String>,
}

impl PostgresArgs {
    pub fn get_postgres_port(&self) -> u16 {
        match self.use_host_postgres {
            true => self.host_postgres_port,
            false => self.postgres_port,
        }
    }

    /// Get the connection string for the postgres database. If `database` is specified
    /// we will use that rather than `postgres_database`.
    pub fn get_connection_string(&self, database: Option<&str>) -> String {
        let password = match self.use_host_postgres {
            true => match &self.host_postgres_password {
                Some(password) => format!(":{}", password),
                None => "".to_string(),
            },
            false => "".to_string(),
        };
        let port = self.get_postgres_port();
        let database = match database {
            Some(database) => database,
            None => &self.postgres_database,
        };
        format!(
            "postgres://{}{}@127.0.0.1:{}/{}",
            self.postgres_user, password, port, database,
        )
    }
}

#[derive(Clone, Debug)]
pub struct PostgresManager {
    args: PostgresArgs,
    test_dir: PathBuf,
    force_restart: bool,
}

impl PostgresManager {
    pub fn new(args: &RunLocalTestnet, test_dir: PathBuf) -> Result<Self> {
        if args.postgres_args.use_host_postgres
            && args.postgres_args.postgres_database == "postgres"
        {
            bail!("The postgres database cannot be named postgres if --use-host-postgres is set");
        }
        Ok(Self {
            args: args.postgres_args.clone(),
            test_dir,
            force_restart: args.force_restart,
        })
    }

    /// Drop and recreate the database specified by `self.args.postgres_database`.
    /// This is only necessary when --force-restart and --use-host-postgres are set.
    /// For this we connect to the `postgres` database so we can drop the database
    /// we'll actually use (since you can't drop a database you're connected to).
    async fn recreate_host_database(&self) -> Result<()> {
        info!("Dropping database {}", self.args.postgres_database);
        let connection_string = self.args.get_connection_string(Some("postgres"));

        // Open a connection to the DB.
        let mut connection = AsyncPgConnection::establish(&connection_string)
            .await
            .with_context(|| format!("Failed to connect to postgres at {}", connection_string))?;

        // Drop the DB.
        diesel::sql_query(format!(
            "DROP DATABASE IF EXISTS {}",
            self.args.postgres_database
        ))
        .execute(&mut connection)
        .await?;
        info!("Dropped database {}", self.args.postgres_database);

        // Create DB again.
        diesel::sql_query(format!("CREATE DATABASE {}", self.args.postgres_database))
            .execute(&mut connection)
            .await?;
        info!("Created database {}", self.args.postgres_database);

        Ok(())
    }
}

#[async_trait]
impl ServiceManager for PostgresManager {
    fn get_name(&self) -> String {
        "Postgres".to_string()
    }

    async fn pre_run(&self) -> Result<()> {
        if self.args.use_host_postgres && self.force_restart {
            // If we're using a DB outside of Docker, drop and recreate the database.
            self.recreate_host_database().await?;
        } else {
            // Confirm Docker is available.
            confirm_docker_available().await?;

            // Pull the image here so it is not subject to the 30 second startup timeout.
            pull_docker_image(POSTGRES_IMAGE).await?;

            // Kill any existing container we find.
            delete_container(POSTGRES_CONTAINER_NAME).await?;
        }

        Ok(())
    }

    fn get_healthchecks(&self) -> HashSet<HealthChecker> {
        hashset! {HealthChecker::Postgres(
            self.args.get_connection_string(None),
        )}
    }

    fn get_prerequisite_health_checkers(&self) -> HashSet<&HealthChecker> {
        hashset! {}
    }

    async fn run_service(self: Box<Self>) -> Result<()> {
        // If we're using postgres on the host just do nothing forever.
        if self.args.use_host_postgres {
            return std::future::pending().await;
        }

        // Let the user know where to go to see logs for postgres.
        setup_docker_logging(&self.test_dir, "postgres", POSTGRES_CONTAINER_NAME)?;

        let options = Some(CreateContainerOptions {
            name: POSTGRES_CONTAINER_NAME,
            ..Default::default()
        });

        let port = self.args.get_postgres_port().to_string();
        let exposed_ports = Some(hashmap! {POSTGRES_DEFAULT_PORT.to_string() => hashmap!{}});
        let host_config = Some(HostConfig {
            port_bindings: Some(hashmap! {
                POSTGRES_DEFAULT_PORT.to_string() => Some(vec![PortBinding {
                    host_ip: Some("127.0.0.1".to_string()),
                    host_port: Some(port),
                }]),
            }),
            ..Default::default()
        });

        let config = Config {
            image: Some(POSTGRES_IMAGE.to_string()),
            tty: Some(true),
            exposed_ports,
            host_config,
            env: Some(vec![
                "POSTGRES_HOST_AUTH_METHOD=trust".to_string(),
                format!("POSTGRES_USER={}", self.args.postgres_user),
                format!("POSTGRES_DB={}", self.args.postgres_database),
            ]),
            ..Default::default()
        };

        let docker = get_docker()?;

        let id = docker.create_container(options, config).await?.id;

        docker
            .start_container(&id, None::<StartContainerOptions<&str>>)
            .await
            .context("Failed to start postgres container")?;

        // Wait for the container to stop, which it never should unless we receive
        // ctrl-c.
        let wait = docker
            .wait_container(
                &id,
                Some(WaitContainerOptions {
                    condition: "not-running",
                }),
            )
            .try_collect::<Vec<_>>()
            .await
            .context("Failed to wait on postgres container")?;

        warn!("Postgres container stopped: {:?}", wait.last());

        Ok(())
    }

    fn get_shutdown_steps(&self) -> Vec<Box<dyn ShutdownStep>> {
        // Shutdown the postgres container, if any.
        vec![Box::new(KillContainerShutdownStep::new(
            POSTGRES_CONTAINER_NAME,
        ))]
    }
}
