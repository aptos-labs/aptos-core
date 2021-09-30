// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{context::Context, page::Page};

use diem_api_types::{mime_types, Error, LedgerInfo, MoveConverter, Response, Transaction};
use diem_types::{mempool_status::MempoolStatusCode, transaction::SignedTransaction};

use anyhow::{format_err, Result};
use serde_json::json;
use warp::{
    http::{header::CONTENT_TYPE, StatusCode},
    reply, Filter, Rejection, Reply,
};

pub fn routes(context: Context) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    get_transactions(context.clone()).or(post_bcs_transactions(context))
}

// GET /transactions?start={u64}&limit={u16}
pub fn get_transactions(
    context: Context,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("transactions")
        .and(warp::get())
        .and(warp::query::<Page>())
        .and(context.filter())
        .and_then(handle_get_transactions)
}

async fn handle_get_transactions(page: Page, context: Context) -> Result<impl Reply, Rejection> {
    Ok(Transactions::new(context)?.list(page)?)
}

// POST /transactions
pub fn post_bcs_transactions(
    context: Context,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("transactions")
        .and(warp::post())
        .and(warp::header::exact(
            CONTENT_TYPE.as_str(),
            mime_types::BCS_SIGNED_TRANSACTION,
        ))
        .and(warp::body::bytes())
        .and(context.filter())
        .and_then(handle_post_bcs_transactions)
}

async fn handle_post_bcs_transactions(
    body: bytes::Bytes,
    context: Context,
) -> Result<impl Reply, Rejection> {
    let txn = bcs::from_bytes(&body)
        .map_err(|_| {
            format_err!("invalid request body: deserialize SignedTransaction BCS bytes failed")
        })
        .map_err(Error::bad_request)?;
    Ok(Transactions::new(context)?.create(txn).await?)
}

struct Transactions {
    ledger_info: LedgerInfo,
    context: Context,
}

impl Transactions {
    fn new(context: Context) -> Result<Self, Error> {
        let ledger_info = context.get_latest_ledger_info()?;
        Ok(Self {
            ledger_info,
            context,
        })
    }

    pub async fn create(self, txn: SignedTransaction) -> Result<impl Reply, Error> {
        let (mempool_status, vm_status_opt) = self.context.submit_transaction(txn.clone()).await?;
        match mempool_status.code {
            MempoolStatusCode::Accepted => {
                let resp = Response::new(
                    self.context.get_latest_ledger_info()?,
                    &Transaction::from(txn),
                )?;
                Ok(reply::with_status(resp, StatusCode::ACCEPTED))
            }
            MempoolStatusCode::VmError => Err(Error::bad_request(format_err!(
                "invalid transaction: {}",
                vm_status_opt
                    .map(|s| format!("{:?}", s))
                    .unwrap_or_else(|| "UNKNOWN".to_owned())
            ))),
            _ => Err(Error::bad_request(format_err!(
                "transaction is rejected: {}",
                mempool_status,
            ))),
        }
    }

    pub fn list(self, page: Page) -> Result<impl Reply, Error> {
        let ledger_version = self.ledger_info.version();
        let start_version = page.start(ledger_version)?;
        if start_version > ledger_version {
            return Err(transaction_not_found(start_version, ledger_version));
        }
        let limit = page.limit()?;

        let data = self
            .context
            .get_transactions(start_version, limit, ledger_version)?;

        let db = self.context.db();
        let converter = MoveConverter::new(&db);

        let infos = data.proof.transaction_infos;
        let events = data.events.unwrap_or_default();
        let txns: Vec<Transaction> = data
            .transactions
            .iter()
            .enumerate()
            .map(|(i, txn)| {
                converter.try_into_transaction(start_version + i as u64, txn, &infos[i], &events[i])
            })
            .collect::<Result<_>>()?;
        Response::new(self.ledger_info, &txns)
    }
}

fn transaction_not_found(version: u64, ledger_version: u64) -> Error {
    Error::not_found(
        format!("could not find transaction by version: {}", version),
        json!({"ledger_version": ledger_version.to_string()}),
    )
}

#[cfg(any(test))]
mod tests {
    use crate::test_utils::{assert_json, new_test_context};

    use diem_crypto::hash::CryptoHash;
    use diem_types::transaction::{Transaction, TransactionInfoTrait};

    use serde_json::json;

    #[tokio::test]
    async fn test_get_transactions() {
        let context = new_test_context();
        let ledger_info = context.get_latest_ledger_info();
        let txns = context
            .context
            .get_transactions(0, 1, ledger_info.version())
            .unwrap();

        let resp = context.get("/transactions").await;
        assert_eq!(resp[0]["type"], "genesis_transaction");
        assert_eq!(resp[0]["version"], "0");

        let info = txns.proof.transaction_infos[0].clone();
        assert_eq!(resp[0]["hash"], info.transaction_hash().to_hex_literal());
        assert_eq!(
            resp[0]["state_root_hash"],
            info.state_root_hash().to_hex_literal()
        );
        assert_eq!(
            resp[0]["event_root_hash"],
            info.event_root_hash().to_hex_literal()
        );
        assert!(resp[0]["data"].as_str().unwrap().starts_with("0x"));

        let first_event = resp[0]["events"][0].clone();
        assert_json(
            first_event,
            json!({
                "key": "0x00000000000000000000000000000000000000000a550c18",
                "sequence_number": "0",
                "transaction_version": "0",
                "type": {
                    "type": "struct",
                    "address": "0x1",
                    "module": "DiemAccount",
                    "name": "CreateAccountEvent",
                    "generic_type_params": []
                },
                "data": {
                    "created": "0xa550c18",
                    "role_id": "0"
                }
            }),
        );
    }

    #[tokio::test]
    async fn test_get_transactions_with_start_version_is_too_large() {
        let context = new_test_context();
        let ledger_version = context.get_latest_ledger_info().version();
        let resp = context
            .expect_status_code(404)
            .get("/transactions?start=1000000&limit=10")
            .await;
        assert_json(
            resp,
            json!({
              "code": 404,
              "message": "could not find transaction by version: 1000000",
              "data": {
                "ledger_version": ledger_version.to_string()
              }
            }),
        );
    }

    #[tokio::test]
    async fn test_get_transactions_with_invalid_start_version_param() {
        let context = new_test_context();
        let resp = context
            .expect_status_code(400)
            .get("/transactions?start=hello")
            .await;
        assert_json(
            resp,
            json!({
              "code": 400,
              "message": "invalid parameter: start=hello"
            }),
        );
    }

    #[tokio::test]
    async fn test_get_transactions_with_invalid_limit_param() {
        let context = new_test_context();
        let resp = context
            .expect_status_code(400)
            .get("/transactions?limit=hello")
            .await;
        assert_json(
            resp,
            json!({
              "code": 400,
              "message": "invalid parameter: limit=hello"
            }),
        );
    }

    #[tokio::test]
    async fn test_get_transactions_param_limit_exceeds_limit() {
        let context = new_test_context();
        let resp = context
            .expect_status_code(400)
            .get("/transactions?limit=2000")
            .await;
        assert_json(
            resp,
            json!({
              "code": 400,
              "message": "invalid parameter: limit=2000, exceed limit 1000"
            }),
        );
    }

    #[tokio::test]
    async fn test_get_transactions_output_user_transaction() {
        let mut context = new_test_context();
        let account = context.gen_account();
        let txn = context.create_parent_vasp(&account);
        context.commit_block(&vec![txn.clone()]);

        let txns = context.get("/transactions?start=1").await;
        assert_eq!(2, txns.as_array().unwrap().len());

        let expected_txns = context.get_transactions(1, 2);
        assert_eq!(2, expected_txns.proof.transaction_infos.len());

        let metadata = expected_txns.proof.transaction_infos[0].clone();

        let metadata_txn = match &expected_txns.transactions[0] {
            Transaction::BlockMetadata(txn) => txn.clone(),
            _ => panic!(
                "unexpected transaction: {:?}",
                expected_txns.transactions[0]
            ),
        };
        assert_json(
            txns[0].clone(),
            json!(
            {
                "type": "block_metadata_transaction",
                "version": "1",
                "hash": metadata.transaction_hash().to_hex_literal(),
                "state_root_hash": metadata.state_root_hash().to_hex_literal(),
                "event_root_hash": metadata.event_root_hash().to_hex_literal(),
                "gas_used": metadata.gas_used().to_string(),
                "success": true,
                "id": metadata_txn.id().to_hex_literal(),
                "round": "1",
                "previous_block_votes": [],
                "proposer": context.validator_owner.to_hex_literal(),
            }),
        );

        let user_txn_info = expected_txns.proof.transaction_infos[1].clone();
        assert_json(
            txns[1].clone(),
            json!({
                "type": "user_transaction",
                "version": "2",
                "hash": user_txn_info.transaction_hash().to_hex_literal(),
                "state_root_hash": user_txn_info.state_root_hash().to_hex_literal(),
                "event_root_hash": user_txn_info.event_root_hash().to_hex_literal(),
                "gas_used": user_txn_info.gas_used().to_string(),
                "success": true,
                "sender": "0xb1e55ed",
                "sequence_number": "0",
                "max_gas_amount": "1000000",
                "gas_unit_price": "0",
                "gas_currency_code": "XUS",
                "expiration_timestamp_secs": txn.expiration_timestamp_secs().to_string(),
                "events": [
                    {
                        "key": "0x00000000000000000000000000000000000000000a550c18",
                        "sequence_number": "5",
                        "transaction_version": "2",
                        "type": {
                            "type": "struct",
                            "address": "0x1",
                            "module": "DiemAccount",
                            "name": "CreateAccountEvent",
                            "generic_type_params": []
                        },
                        "data": {
                            "created": account.address().to_hex_literal(),
                            "role_id": "5"
                        }
                    }
                ],
                "payload": {
                    "type": "script_function_payload",
                    "module": {
                        "address": "0x1",
                        "name": "AccountCreationScripts"
                    },
                    "function": "create_parent_vasp_account",
                    "type_arguments": [
                        {
                            "type": "struct",
                            "address": "0x1",
                            "module": "XUS",
                            "name": "XUS",
                            "generic_type_params": []
                        }
                    ],
                    "arguments": [
                        "0",
                        account.address().to_hex_literal(),
                        format!("0x{}", hex::encode(account.authentication_key().prefix())),
                        format!("0x{}", hex::encode("vasp".as_bytes())),
                        true
                    ]
                }
            }),
        )
    }

    #[tokio::test]
    async fn test_post_bcs_format_transaction() {
        let mut context = new_test_context();
        let account = context.gen_account();
        let txn = context.create_parent_vasp(&account);
        let body = bcs::to_bytes(&txn).unwrap();
        let resp = context
            .expect_status_code(202)
            .post_bcs_txn("/transactions", body)
            .await;
        let expiration_timestamp = txn.expiration_timestamp_secs();
        let hash = Transaction::UserTransaction(txn).hash();
        assert_json(
            resp,
            json!({
                "type": "pending_transaction",
                "hash": hash.to_hex_literal(),
                "sender": "0xb1e55ed",
                "sequence_number": "0",
                "max_gas_amount": "1000000",
                "gas_unit_price": "0",
                "gas_currency_code": "XUS",
                "expiration_timestamp_secs": expiration_timestamp.to_string(),
            }),
        );
    }

    #[tokio::test]
    async fn test_post_invalid_bcs_format_transaction() {
        let context = new_test_context();

        let resp = context
            .expect_status_code(400)
            .post_bcs_txn("/transactions", bcs::to_bytes("invalid data").unwrap())
            .await;
        assert_json(
            resp,
            json!({
              "code": 400,
              "message": "invalid request body: deserialize SignedTransaction BCS bytes failed"
            }),
        );
    }

    #[tokio::test]
    async fn test_post_invalid_signature_transaction() {
        let mut context = new_test_context();
        let txn = context.create_invalid_signature_transaction();
        let body = bcs::to_bytes(&txn).unwrap();
        let resp = context
            .expect_status_code(400)
            .post_bcs_txn("/transactions", &body)
            .await;
        assert_json(
            resp,
            json!({
              "code": 400,
              "message": "invalid transaction: INVALID_SIGNATURE"
            }),
        );
    }

    #[tokio::test]
    async fn test_post_transaction_rejected_by_mempool() {
        let mut context = new_test_context();
        let account1 = context.gen_account();
        let account2 = context.gen_account();
        let txn1 = context.create_parent_vasp(&account1);
        let txn2 = context.create_parent_vasp(&account2);

        context
            .expect_status_code(202)
            .post_bcs_txn("/transactions", &bcs::to_bytes(&txn1).unwrap())
            .await;

        let resp = context
            .expect_status_code(400)
            .post_bcs_txn("/transactions", &bcs::to_bytes(&txn2).unwrap())
            .await;
        assert_json(
            resp,
            json!({
              "code": 400,
              "message": "transaction is rejected: InvalidUpdate - Failed to update gas price to 0"
            }),
        );
    }
}
