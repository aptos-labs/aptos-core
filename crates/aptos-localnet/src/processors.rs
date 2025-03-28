// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Result};
use processor::config::processor_config::{ProcessorConfig, ProcessorName};

pub fn get_processor_config(processor_name: &ProcessorName) -> Result<ProcessorConfig> {
    Ok(match processor_name {
        ProcessorName::AccountTransactionsProcessor => {
            ProcessorConfig::AccountTransactionsProcessor
        },
        ProcessorName::AnsProcessor => {
            bail!("ANS processor is not supported in the localnet")
        },
        ProcessorName::DefaultProcessor => ProcessorConfig::DefaultProcessor,
        ProcessorName::EventsProcessor => ProcessorConfig::EventsProcessor,
        ProcessorName::FungibleAssetProcessor => ProcessorConfig::FungibleAssetProcessor,
        ProcessorName::MonitoringProcessor => {
            bail!("Monitoring processor is not supported in the localnet")
        },
        ProcessorName::NftMetadataProcessor => {
            bail!("NFT Metadata processor is not supported in the localnet")
        },
        ProcessorName::ObjectsProcessor => {
            ProcessorConfig::ObjectsProcessor(ObjectsProcessorConfig {
                query_retries: Default::default(),
                query_retry_delay_ms: Default::default(),
            })
        },
        ProcessorName::ParquetDefaultProcessor => {
            bail!("ParquetDefaultProcessor is not supported in the localnet")
        },
        ProcessorName::ParquetFungibleAssetActivitiesProcessor => {
            bail!("ParquetFungibleAssetActivitiesProcessor is not supported in the localnet")
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
        ProcessorName::ParquetUserTransactionsProcessor => {
            bail!("ParquetUserTransactionsProcessor is not supported in the localnet")
        },
        ProcessorName::StakeProcessor => ProcessorConfig::StakeProcessor(StakeProcessorConfig {
            query_retries: Default::default(),
            query_retry_delay_ms: Default::default(),
        }),
        ProcessorName::TokenV2Processor => {
            ProcessorConfig::TokenV2Processor(TokenV2ProcessorConfig {
                query_retries: Default::default(),
                query_retry_delay_ms: Default::default(),
            })
        },
        ProcessorName::TransactionMetadataProcessor => {
            ProcessorConfig::TransactionMetadataProcessor
        },
        ProcessorName::UserTransactionProcessor => ProcessorConfig::UserTransactionProcessor,
    })
}
