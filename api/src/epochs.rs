// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    accept_type::AcceptType,
    context::{api_spawn_blocking, Context},
    response::{
        BadRequestError, BasicError, BasicResponse, BasicResponseStatus, BasicResult, InternalError,
    },
    ApiTags,
};
use anyhow::Context as AnyhowContext;
use aptos_api_types::{AptosErrorCode, Epoch, LedgerInfo};
use poem_openapi::{param::Path, OpenApi};
use std::sync::Arc;

/// API for epoch version lookups
#[derive(Clone)]
pub struct EpochsApi {
    pub context: Arc<Context>,
}

#[OpenApi]
impl EpochsApi {
    /// Get the version range for the current epoch
    ///
    /// Returns the inclusive first ledger version for the current open epoch and
    /// `last_version = null`.
    #[oai(
        path = "/epochs",
        method = "get",
        operation_id = "get_current_epoch",
        tag = "ApiTags::Epochs"
    )]
    async fn get_current_epoch(&self, accept_type: AcceptType) -> BasicResult<Epoch> {
        self.context
            .check_api_output_enabled("Get current epoch", &accept_type)?;
        let api = self.clone();
        api_spawn_blocking(move || api.get_current_epoch_inner(accept_type)).await
    }

    /// Get the version range for an epoch
    ///
    /// Returns the inclusive first ledger version for an epoch and, when available,
    /// the inclusive last ledger version.
    ///
    /// Sealed epochs return both `first_version` and `last_version`. The current
    /// open epoch returns `first_version` and `last_version = null`.
    #[oai(
        path = "/epochs/:epoch",
        method = "get",
        operation_id = "get_epoch",
        tag = "ApiTags::Epochs"
    )]
    async fn get_epoch(&self, accept_type: AcceptType, epoch: Path<u64>) -> BasicResult<Epoch> {
        self.context
            .check_api_output_enabled("Get epoch", &accept_type)?;
        let api = self.clone();
        api_spawn_blocking(move || api.get_epoch_inner(accept_type, epoch.0)).await
    }
}

impl EpochsApi {
    fn get_current_epoch_inner(&self, accept_type: AcceptType) -> BasicResult<Epoch> {
        let latest_ledger_info = self.context.get_latest_storage_ledger_info()?;
        let current_open_epoch = self.get_current_open_epoch(&latest_ledger_info)?;
        self.get_epoch_response(
            accept_type,
            current_open_epoch,
            current_open_epoch,
            &latest_ledger_info,
        )
    }

    fn get_epoch_inner(&self, accept_type: AcceptType, epoch: u64) -> BasicResult<Epoch> {
        let latest_ledger_info = self.context.get_latest_storage_ledger_info()?;
        let current_open_epoch = self.get_current_open_epoch(&latest_ledger_info)?;

        if epoch > current_open_epoch {
            return Err(BasicError::bad_request_with_code(
                format!(
                    "Epoch {} has not started yet. Current open epoch: {}",
                    epoch, current_open_epoch
                ),
                AptosErrorCode::InvalidInput,
                &latest_ledger_info,
            ));
        }

        self.get_epoch_response(accept_type, epoch, current_open_epoch, &latest_ledger_info)
    }

    fn get_current_open_epoch(&self, latest_ledger_info: &LedgerInfo) -> Result<u64, BasicError> {
        let latest_ledger_info_with_sigs = self
            .context
            .get_latest_ledger_info_with_signatures()
            .context("Failed to retrieve latest ledger info")
            .map_err(|err| {
                BasicError::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    latest_ledger_info,
                )
            })?;

        Ok(latest_ledger_info_with_sigs.ledger_info().next_block_epoch())
    }

    fn get_epoch_response(
        &self,
        accept_type: AcceptType,
        epoch: u64,
        current_open_epoch: u64,
        latest_ledger_info: &LedgerInfo,
    ) -> BasicResult<Epoch> {
        let (first_version, last_version) = if epoch == 0 {
            (0, Some(0))
        } else {
            let previous_last_version =
                self.get_epoch_ending_version(epoch - 1, &latest_ledger_info)?;
            let first_version = previous_last_version
                .checked_add(1)
                .context("Epoch version overflow while computing first version")
                .map_err(|err| {
                    BasicError::internal_with_code(
                        err,
                        AptosErrorCode::InternalError,
                        &latest_ledger_info,
                    )
                })?;
            let last_version = if epoch == current_open_epoch {
                None
            } else {
                Some(self.get_epoch_ending_version(epoch, &latest_ledger_info)?)
            };
            (first_version, last_version)
        };

        BasicResponse::try_from_rust_value((
            Epoch::new(epoch, first_version, last_version),
            &latest_ledger_info,
            BasicResponseStatus::Ok,
            &accept_type,
        ))
    }

    fn get_epoch_ending_version(
        &self,
        epoch: u64,
        latest_ledger_info: &LedgerInfo,
    ) -> Result<u64, BasicError> {
        let end_epoch = epoch
            .checked_add(1)
            .context("Epoch overflow while resolving ending ledger info")
            .map_err(|err| {
                BasicError::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    latest_ledger_info,
                )
            })?;
        let proof = self
            .context
            .db
            .get_epoch_ending_ledger_infos(epoch, end_epoch)
            .context(format!(
                "Failed to retrieve epoch ending ledger info for epoch {}",
                epoch
            ))
            .map_err(|err| {
                BasicError::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    latest_ledger_info,
                )
            })?;

        if proof.more {
            return Err(BasicError::internal_with_code(
                format!(
                    "Unexpected paginated epoch ending ledger info for epoch {}",
                    epoch
                ),
                AptosErrorCode::InternalError,
                latest_ledger_info,
            ));
        }
        if proof.ledger_info_with_sigs.len() != 1 {
            return Err(BasicError::internal_with_code(
                format!(
                    "Expected exactly one epoch ending ledger info for epoch {}, found {}",
                    epoch,
                    proof.ledger_info_with_sigs.len()
                ),
                AptosErrorCode::InternalError,
                latest_ledger_info,
            ));
        }

        Ok(proof.ledger_info_with_sigs[0].ledger_info().version())
    }
}
