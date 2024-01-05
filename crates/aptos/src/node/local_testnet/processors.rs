// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{health_checker::HealthChecker, traits::ServiceManager, RunLocalTestnet};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use clap::Parser;
use diesel_async::{pg::AsyncPgConnection, AsyncConnection};
use maplit::hashset;
use processor::{
    processors::{token_processor::TokenProcessorConfig, ProcessorConfig, ProcessorName},
    utils::database::run_pending_migrations,
    IndexerGrpcProcessorConfig,
};
use reqwest::Url;
use server_framework::RunnableConfig;
use std::collections::HashSet;
use tokio::sync::OnceCell;
use tracing::info;

static RUN_MIGRATIONS_ONCE: OnceCell<bool> = OnceCell::const_new();

/// This struct is used to parse the command line arguments for the processors.
#[derive(Debug, Parser)]
pub struct ProcessorArgs {
    /// The value of this flag determines which processors we will run if
    /// --with-indexer-api is set. Note that some processors are not supported in the
    /// local testnet (e.g. ANS). If you try to set those an error will be thrown
    /// immediately.
    #[clap(
        long,
        value_enum,
        default_values_t = vec![
            ProcessorName::AccountTransactionsProcessor,
            ProcessorName::CoinProcessor,
            ProcessorName::DefaultProcessor,
            ProcessorName::EventsProcessor,
            ProcessorName::FungibleAssetProcessor,
            ProcessorName::StakeProcessor,
            ProcessorName::TokenProcessor,
            ProcessorName::TokenV2Processor,
            ProcessorName::UserTransactionProcessor,
        ],
        requires = "with_indexer_api"
    )]
    processors: Vec<ProcessorName>,
}

#[derive(Debug)]
pub struct ProcessorManager {
    config: IndexerGrpcProcessorConfig,
    prerequisite_health_checkers: HashSet<HealthChecker>,
}

impl ProcessorManager {
    fn new(
        processor_name: &ProcessorName,
        prerequisite_health_checkers: HashSet<HealthChecker>,
        data_service_url: Url,
        postgres_connection_string: String,
    ) -> Result<Self> {
        let processor_config = match processor_name {
            ProcessorName::AccountTransactionsProcessor => {
                ProcessorConfig::AccountTransactionsProcessor
            },
            ProcessorName::AnsProcessor => {
                bail!("ANS processor is not supported in the local testnet")
            },
            ProcessorName::CoinProcessor => ProcessorConfig::CoinProcessor,
            ProcessorName::DefaultProcessor => ProcessorConfig::DefaultProcessor,
            ProcessorName::EventsProcessor => ProcessorConfig::EventsProcessor,
            ProcessorName::FungibleAssetProcessor => ProcessorConfig::FungibleAssetProcessor,
            ProcessorName::NftMetadataProcessor => {
                bail!("NFT Metadata processor is not supported in the local testnet")
            },
            ProcessorName::StakeProcessor => ProcessorConfig::StakeProcessor,
            ProcessorName::TokenProcessor => {
                ProcessorConfig::TokenProcessor(TokenProcessorConfig {
                    // This NFT points contract doesn't exist on local testnets.
                    nft_points_contract: None,
                })
            },
            ProcessorName::TokenV2Processor => ProcessorConfig::TokenV2Processor,
            ProcessorName::UserTransactionProcessor => ProcessorConfig::UserTransactionProcessor,
        };
        let config = IndexerGrpcProcessorConfig {
            processor_config,
            postgres_connection_string,
            indexer_grpc_data_service_address: data_service_url,
            auth_token: "notused".to_string(),
            grpc_http2_config: Default::default(),
            starting_version: None,
            ending_version: None,
            number_concurrent_processing_tasks: None,
            enable_verbose_logging: None,
        };
        let manager = Self {
            config,
            prerequisite_health_checkers,
        };
        Ok(manager)
    }

    /// This function returns many new ProcessorManagers, one for each processor.
    pub fn many_new(
        args: &RunLocalTestnet,
        prerequisite_health_checkers: HashSet<HealthChecker>,
        data_service_url: Url,
        postgres_connection_string: String,
    ) -> Result<Vec<Self>> {
        if args.processor_args.processors.is_empty() {
            bail!("Must specify at least one processor to run");
        }
        let mut managers = Vec::new();
        for processor_name in &args.processor_args.processors {
            managers.push(Self::new(
                processor_name,
                prerequisite_health_checkers.clone(),
                data_service_url.clone(),
                postgres_connection_string.clone(),
            )?);
        }
        Ok(managers)
    }

    /// Create the necessary tables in the DB for the processors to work.
    async fn run_migrations(&self) -> Result<()> {
        let connection_string = &self.config.postgres_connection_string;
        let mut connection = AsyncPgConnection::establish(connection_string)
            .await
            .with_context(|| format!("Failed to connect to postgres at {}", connection_string))?;
        run_pending_migrations(&mut connection).await;
        Ok(())
    }
}

#[async_trait]
impl ServiceManager for ProcessorManager {
    fn get_name(&self) -> String {
        format!("processor_{}", self.config.processor_config.name())
    }

    fn get_health_checkers(&self) -> HashSet<HealthChecker> {
        hashset! {HealthChecker::Processor(
            self.config.postgres_connection_string.to_string(),
            self.config.processor_config.name().to_string(),
        ) }
    }

    fn get_prerequisite_health_checkers(&self) -> HashSet<&HealthChecker> {
        self.prerequisite_health_checkers.iter().collect()
    }

    async fn run_service(self: Box<Self>) -> Result<()> {
        // By default, when a processor starts up (specifically in Worker.run) it runs
        // any pending migrations. Unfortunately, if you start multiple processors at
        // the same time, they can sometimes clash with errors like this:
        //
        // https://stackoverflow.com/q/54351783/3846032
        //
        // To fix this, we run the migrations ourselves here in the CLI first. We use
        // OnceCell to make sure we only run the migration once. When all the processor
        // ServiceManagers reach this point, one of them will run the code and the rest
        // will wait. Doing it at this point in the code is safer than relying on
        // coordiation outside of this manager.
        RUN_MIGRATIONS_ONCE
            .get_or_init(|| async {
                info!("Running DB migrations for the indexer processors");
                self.run_migrations()
                    .await
                    .expect("Failed to run DB migrations");
                info!("Ran DB migrations for the indexer processors");
                true
            })
            .await;

        // Run the processor.
        self.config.run().await
    }
}
