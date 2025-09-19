// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    errors::{AptosTapError, AptosTapErrorResponse},
    ApiTags,
};
use crate::{
    bypasser::{Bypasser, BypasserTrait},
    checkers::{Checker, CheckerData, CheckerTrait, CompleteData},
    endpoints::AptosTapErrorCode,
    firebase_jwt::jwt_sub,
    funder::{Funder, FunderTrait},
    helpers::{get_current_time_secs, transaction_hashes},
};
use aptos_logger::info;
use aptos_sdk::{
    crypto::{ed25519::Ed25519PublicKey, ValidCryptoMaterialStringExt},
    types::{
        account_address::AccountAddress,
        transaction::{authenticator::AuthenticationKey, SignedTransaction},
    },
};
use poem::{http::HeaderMap, web::RealIp};
use poem_openapi::{
    payload::{Json, PlainText},
    ApiResponse, Object, OpenApi,
};
use std::sync::Arc;
use tokio::sync::{Semaphore, SemaphorePermit};

#[derive(Clone, Debug, Default, Object)]
pub struct FundRequest {
    /// If not set, the default is the preconfigured max funding amount. If set,
    /// we will use this amount instead assuming it is < than the maximum,
    /// otherwise we'll just use the maximum.
    pub amount: Option<u64>,

    /// Either this or `address` / `pub_key` must be provided.
    pub auth_key: Option<String>,

    /// Either this or `auth_key` / `pub_key` must be provided.
    pub address: Option<String>,

    /// Either this or `auth_key` / `address` must be provided.
    pub pub_key: Option<String>,
}

#[derive(Clone, Debug, Object)]
pub struct FundResponse {
    pub txn_hashes: Vec<String>,
}

impl FundRequest {
    pub fn receiver(&self) -> Option<AccountAddress> {
        if let Some(auth_key) = self.auth_key.as_ref() {
            return match AccountAddress::from_hex_literal(auth_key) {
                Ok(auth_key) => Some(auth_key),
                Err(_) => match AccountAddress::from_hex(auth_key) {
                    Ok(auth_key) => Some(auth_key),
                    Err(_) => None,
                },
            };
        }
        if let Some(address) = self.address.as_ref() {
            return match AccountAddress::from_hex_literal(address) {
                Ok(address) => Some(address),
                Err(_) => match AccountAddress::from_hex(address) {
                    Ok(address) => Some(address),
                    Err(_) => None,
                },
            };
        }
        if let Some(pub_key) = self.pub_key.as_ref() {
            return match Ed25519PublicKey::from_encoded_string(pub_key) {
                Ok(pub_key) => Some(AuthenticationKey::ed25519(&pub_key).account_address()),
                Err(_) => None,
            };
        }
        None
    }
}

impl std::fmt::Display for FundRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "<Fund {:?} to {:?}>", self.amount, self.receiver())
    }
}

pub struct FundApi {
    pub components: Arc<FundApiComponents>,
}

#[OpenApi]
impl FundApi {
    /// Funds an account
    ///
    /// With this endpoint a user can create and fund an account. Depending on
    /// the configured funding backend, this may do different things under the
    /// hood (e.g. minting vs transferring) and have different funding semantics
    /// (e.g. whether it will fund an account if it already exists).
    #[oai(
        path = "/fund",
        method = "post",
        operation_id = "fund",
        tag = "ApiTags::Fund"
    )]
    async fn fund(
        &self,
        fund_request: Json<FundRequest>,
        // This automagically uses FromRequest to get this data from the request.
        // It takes into things like X-Forwarded-IP and X-Real-IP.
        source_ip: RealIp,
        // Same thing, this uses FromRequest.
        header_map: &HeaderMap,
    ) -> poem::Result<Json<FundResponse>, AptosTapErrorResponse> {
        let txns = self
            .components
            .fund_inner(fund_request.0, source_ip, header_map, false)
            .await?;
        Ok(Json(FundResponse {
            txn_hashes: get_hashes(&txns),
        }))
    }

    /// Check whether a given requester is eligible to be funded
    ///
    /// This function runs only the various eligibility checks that we perform
    /// in `fund` without actually funding the account or writing anything to
    /// storage. If the request is valid it returns an empty 200. If it is invalid
    /// it returns a 400 or 403 with an explanation in the response body.
    #[oai(
        path = "/is_eligible",
        method = "post",
        operation_id = "is_eligible",
        tag = "ApiTags::Fund"
    )]
    async fn is_eligible(
        &self,
        fund_request: Json<FundRequest>,
        // This automagically uses FromRequest to get this data from the request.
        // It takes into things like X-Forwarded-IP and X-Real-IP.
        source_ip: RealIp,
        // Same thing, this uses FromRequest.
        header_map: &HeaderMap,
    ) -> poem::Result<(), AptosTapErrorResponse> {
        let (checker_data, bypass, _semaphore_permit) = self
            .components
            .preprocess_request(&fund_request.0, source_ip, header_map, true)
            .await?;

        if bypass {
            return Ok(());
        }

        // Call Funder.fund with `check_only` set, meaning it only does the
        // initial set of checks without actually submitting any transactions
        // to fund the account.
        self.components
            .funder
            .fund(fund_request.amount, checker_data.receiver, true, bypass)
            .await?;

        Ok(())
    }
}

pub struct FundApiComponents {
    /// If any of the allowers say yes, the request is allowed unconditionally
    /// and we never write anything to storage.
    pub bypassers: Vec<Bypasser>,

    /// If any of the checkers say no, the request is rejected.
    pub checkers: Vec<Checker>,

    /// The component that funds accounts.
    pub funder: Arc<Funder>,

    /// See the comment in `RunConfig`.
    pub return_rejections_early: bool,

    /// This semaphore is used to ensure we only process a certain number of
    /// requests concurrently.
    pub concurrent_requests_semaphore: Option<Arc<Semaphore>>,
}

impl FundApiComponents {
    /// Preprocesses the request to return the source IP, receiver account
    /// address and requested amount taking into account Funder configuration
    /// (i.e. max amount). It also ensures the request passes checkers.
    /// This function mostly exists to reduce duplication between the `fund`
    /// and `is_eligible` endpoints. This function also runs the Bypassers.
    /// If any of them said yes, this will return true as the last element
    /// of the output of this function.
    async fn preprocess_request(
        &self,
        fund_request: &FundRequest,
        source_ip: RealIp,
        header_map: &HeaderMap,
        dry_run: bool,
    ) -> poem::Result<(CheckerData, bool, Option<SemaphorePermit<'_>>), AptosTapError> {
        let permit = match &self.concurrent_requests_semaphore {
            Some(semaphore) => match semaphore.try_acquire() {
                Ok(permit) => Some(permit),
                Err(_) => {
                    return Err(AptosTapError::new(
                        "Server overloaded, please try again later".to_string(),
                        AptosTapErrorCode::ServerOverloaded,
                    ))
                },
            },
            None => None,
        };

        let source_ip = match source_ip.0 {
            Some(ip) => ip,
            None => {
                return Err(AptosTapError::new(
                    "No source IP found in the request".to_string(),
                    AptosTapErrorCode::SourceIpMissing,
                ))
            },
        };

        let receiver = match fund_request.receiver() {
            Some(receiver) => receiver,
            None => {
                return Err(AptosTapError::new(
                    "Account address, auth key, or pub key must be provided and valid".to_string(),
                    AptosTapErrorCode::InvalidRequest,
                ))
            },
        };

        let checker_data = CheckerData {
            receiver,
            source_ip,
            headers: Arc::new(header_map.clone()),
            time_request_received_secs: get_current_time_secs(),
        };

        // See if this request meets the criteria to bypass checkers / storage.
        for bypasser in &self.bypassers {
            if bypasser
                .request_can_bypass(checker_data.clone())
                .await
                .map_err(|e| {
                    AptosTapError::new_with_error_code(e, AptosTapErrorCode::BypasserError)
                })?
            {
                info!(
                    "Allowing request from {} to bypass checks / storage",
                    source_ip
                );
                return Ok((checker_data, true, permit));
            }
        }

        // Ensure request passes checkers.
        let mut rejection_reasons = Vec::new();
        for checker in &self.checkers {
            rejection_reasons.extend(checker.check(checker_data.clone(), dry_run).await.map_err(
                |e| AptosTapError::new_with_error_code(e, AptosTapErrorCode::CheckerError),
            )?);
            if !rejection_reasons.is_empty() && self.return_rejections_early {
                break;
            }
        }

        if !rejection_reasons.is_empty() {
            return Err(AptosTapError::new(
                format!("Request rejected by {} checkers", rejection_reasons.len()),
                AptosTapErrorCode::Rejected,
            )
            .rejection_reasons(rejection_reasons));
        }

        Ok((checker_data, false, permit))
    }

    async fn fund_inner(
        &self,
        fund_request: FundRequest,
        // This automagically uses FromRequest to get this data from the request.
        // It takes into things like X-Forwarded-IP and X-Real-IP.
        source_ip: RealIp,
        // Same thing, this uses FromRequest.
        header_map: &HeaderMap,
        dry_run: bool,
    ) -> poem::Result<Vec<SignedTransaction>, AptosTapError> {
        let (checker_data, bypass, _semaphore_permit) = self
            .preprocess_request(&fund_request, source_ip, header_map, dry_run)
            .await?;

        // Fund the account.
        let fund_result = self
            .funder
            .fund(fund_request.amount, checker_data.receiver, false, bypass)
            .await;

        // This might be empty if there is an error and we never got to the
        // point where we could submit a transaction.
        let txn_hashes = match &fund_result {
            Ok(txns) => transaction_hashes(&txns.iter().collect::<Vec<&SignedTransaction>>()),
            Err(e) => e.txn_hashes.to_vec(),
        };

        // Include some additional logging that the logging middleware doesn't do.
        info!(
            source_ip = checker_data.source_ip,
            jwt_sub = jwt_sub(checker_data.headers.clone()).ok(),
            address = checker_data.receiver,
            requested_amount = fund_request.amount,
            txn_hashes = txn_hashes,
            success = fund_result.is_ok(),
        );

        // Give all Checkers the chance to run the completion step. We should
        // monitor for failures in these steps because they could lead to an
        // unintended data state.
        if !bypass {
            let response_is_500 = match &fund_result {
                Ok(_) => false,
                Err(e) => e.error_code.status().is_server_error(),
            };
            let complete_data = CompleteData {
                checker_data,
                txn_hashes: txn_hashes.clone(),
                response_is_500,
            };
            for checker in &self.checkers {
                checker.complete(complete_data.clone()).await.map_err(|e| {
                    AptosTapError::new_with_error_code(e, AptosTapErrorCode::CheckerError)
                })?;
            }
        }

        fund_result
    }
}

/////////////////////////////////////////////////////////////////
/// Legacy /mint endpoint stuff.
/////////////////////////////////////////////////////////////////

#[derive(serde::Deserialize)]
pub struct MintRequest {
    amount: Option<u64>,
    auth_key: Option<String>,
    address: Option<String>,
    pub_key: Option<String>,
    return_txns: Option<bool>,
}

// This is only for the legacy /mint endpoint.
#[derive(Debug, ApiResponse)]
pub enum MintResponse {
    #[oai(status = "200")]
    SubmittedTxnHashes(Json<Vec<String>>),

    // This is a hex string representation of BCS serialization of Vec<SignedTransaction>
    #[oai(status = "200")]
    SubmittedTxns(PlainText<String>),
}

/// This is for backwards compatibility with the old faucet. As such we define
/// it outside of the OpenAPI spec and handle it in its own route.
#[poem::handler]
pub async fn mint(
    fund_api_components: poem::web::Data<&Arc<FundApiComponents>>,
    poem::web::Query(MintRequest {
        amount,
        auth_key,
        address,
        pub_key,
        return_txns,
    }): poem::web::Query<MintRequest>,
    // This automagically uses FromRequest to get this data from the request.
    // It takes into things like X-Forwarded-IP and X-Real-IP.
    source_ip: RealIp,
    // Same thing, this uses FromRequest.
    header_map: &HeaderMap,
) -> poem::Result<MintResponse> {
    // We take the AptosTapError and convert it into an anyhow error with just the
    // message so this endpoint returns a plaintext response like the faucet does.
    // We still return the intended status code though, but not any headers that
    // the /mint endpoint would, e.g. Retry-After.
    let fund_request = FundRequest {
        amount,
        auth_key,
        address,
        pub_key,
    };
    let txns = fund_api_components
        .0
        .fund_inner(fund_request, source_ip, header_map, false)
        .await
        .map_err(|e| {
            poem::Error::from((e.status_and_retry_after().0, anyhow::anyhow!(e.message)))
        })?;
    if return_txns.unwrap_or(false) {
        let txn_bcs =
            aptos_sdk::bcs::to_bytes(&txns).map_err(|e| poem::Error::from(anyhow::anyhow!(e)))?;
        let txn_bcs_hex = hex::encode(txn_bcs);
        Ok(MintResponse::SubmittedTxns(PlainText(txn_bcs_hex)))
    } else {
        Ok(MintResponse::SubmittedTxnHashes(Json(get_hashes(&txns))))
    }
}

/// This returns long hashes with no 0x in front.
fn get_hashes(txns: &[SignedTransaction]) -> Vec<String> {
    txns.iter().map(|t| t.committed_hash().to_hex()).collect()
}
