// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    docker::{
        create_network, create_volume, delete_container, delete_volume, get_docker,
        pull_docker_image, setup_docker_logging, StopContainerShutdownStep, CONTAINER_NETWORK_NAME,
    },
    health_checker::HealthChecker,
    traits::{ServiceManager, ShutdownStep},
    RunLocalnet,
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

pub const POSTGRES_CONTAINER_NAME: &str = "local-testnet-postgres";
const POSTGRES_VOLUME_NAME: &str = "local-testnet-postgres-data";
const POSTGRES_IMAGE: &str = "postgres:14.11";
const DATA_PATH_IN_CONTAINER: &str = "/var/lib/mydata";
const POSTGRES_DEFAULT_PORT: u16 = 5432;

/// Args related to running postgres in the localnet.
#[derive(Clone, Debug, Parser)]
pub struct PostgresArgs {
    /// This is the database to connect to, both when --use-host-postgres is set
    /// and when it is not (when postgres is running in a container).
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
    /// `postgres_args` (e.g. --host-postgres-port) rather than running an instance
    /// with Docker. This can be used to connect to an existing postgres instance
    /// running on the host system.
    ///
    /// WARNING: Any existing database it finds (based on --postgres-database) will be
    /// dropped and recreated.
    #[clap(long, requires = "with_indexer_api")]
    pub use_host_postgres: bool,

    /// If --use-host-postgres is set, you can use this to change the host we try to
    /// connect to.
    #[clap(long, default_value = "127.0.0.1")]
    pub host_postgres_host: String,

    /// When --use-host-postgres is set, this is the port to connect to.
    #[clap(long, default_value_t = 5432)]
    pub host_postgres_port: u16,

    /// When --use-host-postgres is set, this is the password to connect with.
    #[clap(long)]
    pub host_postgres_password: Option<String>,
}

impl PostgresArgs {
    pub fn get_postgres_port(&self, external: bool) -> u16 {
        match external {
            true => match self.use_host_postgres {
                true => self.host_postgres_port,
                false => self.postgres_port,
            },
            // If connecting from inside the container network, just use the default
            // postgres port, since we run postgres on 5432 inside the container.
            false => POSTGRES_DEFAULT_PORT,
        }
    }

    /// Get the connection string for the postgres database. If `database` is specified
    /// we will use that rather than `self.postgres_database`. If `external` is true,
    /// it will give you the string for connecting from the host. If it is false, it
    /// will give you the string for connecting from another container in the network
    /// we create for all containers in the localnet.
    pub fn get_connection_string(&self, database: Option<&str>, external: bool) -> String {
        let password = match self.use_host_postgres {
            true => match &self.host_postgres_password {
                Some(password) => format!(":{}", password),
                None => "".to_string(),
            },
            false => "".to_string(),
        };
        let port = self.get_postgres_port(external);
        let database = match database {
            Some(database) => database,
            None => &self.postgres_database,
        };
        let host = match self.use_host_postgres {
            true => &self.host_postgres_host,
            false => match external {
                true => "127.0.0.1",
                false => POSTGRES_CONTAINER_NAME,
            },
        };
        format!(
            "postgres://{}{}@{}:{}/{}",
            self.postgres_user, password, host, port, database,
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
    pub fn new(args: &RunLocalnet, test_dir: PathBuf) -> Result<Self> {
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
        let connection_string = self.args.get_connection_string(Some("postgres"), true);

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
        if self.args.use_host_postgres {
            if self.force_restart {
                // If we're using a DB outside of Docker, drop and recreate the database.
                self.recreate_host_database().await?;
            }
        } else {
            // Confirm Docker is available.
            get_docker().await?;

            // Kill any existing container we find.
            delete_container(POSTGRES_CONTAINER_NAME).await?;

            // Pull the image here so it is not subject to the startup timeout for
            // `run_service`.
            pull_docker_image(POSTGRES_IMAGE).await?;

            // Create a network for the containers to talk to each other.
            create_network(CONTAINER_NETWORK_NAME).await?;
        }

        Ok(())
    }

    fn get_health_checkers(&self) -> HashSet<HealthChecker> {
        hashset! {HealthChecker::Postgres(
            self.args.get_connection_string(None, true),
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

        let docker = get_docker().await?;

        // If we're starting afresh, delete any existing volume.
        if self.force_restart {
            delete_volume(POSTGRES_VOLUME_NAME)
                .await
                .context("Failed to delete volume for postgres")?;
        }

        // Create a volume for the postgres instance to use.
        create_volume(POSTGRES_VOLUME_NAME)
            .await
            .context("Failed to create volume for postgres")?;

        let options = Some(CreateContainerOptions {
            name: POSTGRES_CONTAINER_NAME,
            ..Default::default()
        });

        let port = self.args.get_postgres_port(true).to_string();
        let exposed_ports = Some(hashmap! {POSTGRES_DEFAULT_PORT.to_string() => hashmap!{}});
        let host_config = Some(HostConfig {
            // Bind the container to the network we created in the pre_run. This does
            // not prevent the binary in the container from exposing itself to the host
            // on 127.0.0.1. See more here: https://stackoverflow.com/a/77432636/3846032.
            network_mode: Some(CONTAINER_NETWORK_NAME.to_string()),
            port_bindings: Some(hashmap! {
                POSTGRES_DEFAULT_PORT.to_string() => Some(vec![PortBinding {
                    host_ip: Some("127.0.0.1".to_string()),
                    host_port: Some(port),
                }]),
            }),
            // Mount the volume in to the container. We use a volume because they are
            // more performant and easier to manage via the Docker API.
            binds: Some(vec![format!(
                "{}:{}",
                POSTGRES_VOLUME_NAME, DATA_PATH_IN_CONTAINER,
            )]),
            ..Default::default()
        });

        let config = Config {
            image: Some(POSTGRES_IMAGE.to_string()),
            // We set this to false so the container keeps running after the CLI
            // shuts down by default. We manually kill the container if applicable,
            // for example if the user set --force-restart.
            tty: Some(false),
            exposed_ports,
            host_config,
            env: Some(vec![
                // We run postgres without any auth + no password.
                "POSTGRES_HOST_AUTH_METHOD=trust".to_string(),
                format!("POSTGRES_USER={}", self.args.postgres_user),
                format!("POSTGRES_DB={}", self.args.postgres_database),
                // This tells where postgres to store the DB data on disk. This is the
                // directory inside the container that is mounted from the host system.
                format!("PGDATA={}", DATA_PATH_IN_CONTAINER),
            ]),
            cmd: Some(
                vec![
                    "postgres",
                    "-c",
                    // The default is 100 as of Postgres 14.11. Given the localnet
                    // can be composed of many different processors all with their own
                    // connection pools, 100 is insufficient.
                    "max_connections=200",
                    "-c",
                    // The default is 128MB as of Postgres 14.11. We 2x that value to
                    // match the fact that we 2x'd max_connections.
                    "shared_buffers=256MB",
                ]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            ),
            ..Default::default()
        };

        info!("Starting postgres with this config: {:?}", config);

        let id = docker
            .create_container(options, config)
            .await
            .context("Failed to create postgres container")?
            .id;

        info!("Created container for postgres with this ID: {}", id);

        docker
            .start_container(&id, None::<StartContainerOptions<&str>>)
            .await
            .context("Failed to start postgres container")?;

        info!("Started postgres container {}", id);

        // Wait for the container to stop (which it shouldn't).
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
        if self.args.use_host_postgres {
            vec![]
        } else {
            // Stop the container. Note, stopping and even deleting the container is
            // fine because we store the data in a directory mounted in from the host
            // system, so it will persist.
            vec![Box::new(StopContainerShutdownStep::new(
                POSTGRES_CONTAINER_NAME,
            ))]
        }
    }
}
