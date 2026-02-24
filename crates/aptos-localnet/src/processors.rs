// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{bail, Result};
use processor::{
    config::processor_config::{ProcessorConfig, ProcessorName},
    processors::{
        objects::objects_processor::ObjectsProcessorConfig,
        stake::stake_processor::StakeProcessorConfig,
        token_v2::token_v2_processor::TokenV2ProcessorConfig,
    },
};

pub fn get_processor_config(processor_name: &ProcessorName) -> Result<ProcessorConfig> {
    Ok(match processor_name {
        ProcessorName::AccountTransactionsProcessor => {
            ProcessorConfig::AccountTransactionsProcessor(Default::default())
        },
        ProcessorName::AccountRestorationProcessor => {
            ProcessorConfig::AccountRestorationProcessor(Default::default())
        },
        ProcessorName::AnsProcessor => {
            bail!("ANS processor is not supported in the localnet")
        },
        ProcessorName::DefaultProcessor => ProcessorConfig::DefaultProcessor(Default::default()),
        ProcessorName::FungibleAssetProcessor => {
            ProcessorConfig::FungibleAssetProcessor(Default::default())
        },
        ProcessorName::GasFeeProcessor => {
            bail!("GasFeeProcessor is not supported in the localnet")
        },
        ProcessorName::MonitoringProcessor => {
            bail!("Monitoring processor is not supported in the localnet")
        },
        ProcessorName::ObjectsProcessor => {
            ProcessorConfig::ObjectsProcessor(ObjectsProcessorConfig {
                default_config: Default::default(),
                query_retries: Default::default(),
                query_retry_delay_ms: Default::default(),
            })
        },
        ProcessorName::ParquetDefaultProcessor => {
            bail!("ParquetDefaultProcessor is not supported in the localnet")
        },
        ProcessorName::ParquetFungibleAssetProcessor => {
            bail!("ParquetFungibleAssetProcessor is not supported in the localnet")
        },
        ProcessorName::ParquetTransactionMetadataProcessor => {
            bail!("ParquetTransactionMetadataProcessor is not supported in the localnet")
        },
        ProcessorName::ParquetAnsProcessor => {
            bail!("ParquetAnsProcessor is not supported in the localnet")
        },
        ProcessorName::ParquetEventsProcessor => {
            bail!("ParquetEventsProcessor is not supported in the localnet")
        },
        ProcessorName::ParquetTokenV2Processor => {
            bail!("ParquetTokenV2Processor is not supported in the localnet")
        },
        ProcessorName::ParquetUserTransactionProcessor => {
            bail!("ParquetUserTransactionProcessor is not supported in the localnet")
        },
        ProcessorName::ParquetObjectsProcessor => {
            bail!("ParquetObjectsProcessor is not supported in the localnet")
        },
        ProcessorName::ParquetAccountTransactionsProcessor => {
            bail!("ParquetAccountTransactionsProcessor is not supported in the localnet")
        },
        ProcessorName::ParquetStakeProcessor => {
            bail!("ParquetStakeProcessor is not supported in the localnet")
        },
        ProcessorName::StakeProcessor => ProcessorConfig::StakeProcessor(StakeProcessorConfig {
            default_config: Default::default(),
            query_retries: Default::default(),
            query_retry_delay_ms: Default::default(),
        }),
        ProcessorName::TokenV2Processor => {
            ProcessorConfig::TokenV2Processor(TokenV2ProcessorConfig {
                default_config: Default::default(),
                query_retries: Default::default(),
                query_retry_delay_ms: Default::default(),
            })
        },
        ProcessorName::UserTransactionProcessor => {
            ProcessorConfig::UserTransactionProcessor(Default::default())
        },
    })
}
