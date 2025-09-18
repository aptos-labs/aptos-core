// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::error::PepperServiceError;
use aptos_infallible::duration_since_epoch;
use aptos_keyless_pepper_common::{account_recovery_db::AccountRecoveryDbEntry, PepperInput};
use aptos_logger::warn;
use firestore::{path, paths, FirestoreDb, FirestoreDbOptions, FirestoreResult};
use once_cell::sync::Lazy;
use tokio::sync::OnceCell;

/// A GCP firestore that holds all account address pre-images
pub static ACCOUNT_RECOVERY_DB: Lazy<OnceCell<FirestoreResult<FirestoreDb>>> =
    Lazy::new(OnceCell::new);

pub async fn init_account_db() -> FirestoreResult<FirestoreDb> {
    let google_project_id = match std::env::var("PROJECT_ID") {
        Ok(id) => id,
        Err(e) => {
            warn!("Could not load envvar `PROJECT_ID`: {e}");
            "unknown_project".to_string()
        },
    };

    let database_id = match std::env::var("DATABASE_ID") {
        Ok(id) => id,
        Err(e) => {
            warn!("Could not load envvar `DATABASE_ID`: {e}");
            "unknown_database".to_string()
        },
    };

    let option = FirestoreDbOptions {
        google_project_id,
        database_id,
        max_retries: 1,
        firebase_api_url: None,
    };
    FirestoreDb::with_options(option).await
}

/// Save a pepper request into the account recovery DB.
///
/// TODO: once the account recovery DB flow is verified working e2e, DB error should not be ignored.
pub async fn update_account_recovery_db(input: &PepperInput) -> Result<(), PepperServiceError> {
    match ACCOUNT_RECOVERY_DB.get_or_init(init_account_db).await {
        Ok(db) => {
            let entry = AccountRecoveryDbEntry {
                iss: input.iss.clone(),
                aud: input.aud.clone(),
                uid_key: input.uid_key.clone(),
                uid_val: input.uid_val.clone(),
                first_request_unix_ms_minus_1q: None,
                last_request_unix_ms: None,
                num_requests: None,
            };
            let doc_id = entry.document_id();
            let now_unix_ms = duration_since_epoch().as_millis() as i64;

            // The update transactions use the following strategy.
            // 1. If not exists, create the document for the user identifier `(iss, aud, uid_key, uid_val)`.
            //    but leave counter/time fields unspecified.
            // 2. `num_requests += 1`, assuming the default value is 0.
            // 3. `last_request_unix_ms = max(last_request_unix_ms, now)`, assuming the default value is 0.
            // 4. `first_request_unix_ms = min(first_request_unix_ms, now)`, assuming the default value is +inf.
            //
            // This strategy is preferred because all the operations can be made server-side,
            // which means the txn should require only 1 RTT,
            // better than using read-compute-write pattern that requires 2 RTTs.
            //
            // This strategy does not work directly:
            // in firestore, the default value of a number field is 0, and we do not know a way to customize it for op 4.
            // The workaround here is apply an offset so 0 becomes a legitimate default value.
            // So we work with `first_request_unix_ms_minus_1q` instead,
            // which is defined as `first_request_unix_ms - 1_000_000_000_000_000`,
            // where 1_000_000_000_000_000 milliseconds is roughly 31710 years.

            let mut txn = db.begin_transaction().await.map_err(|e| {
                PepperServiceError::InternalError(format!("begin_transaction error: {e}"))
            })?;
            db.fluent()
                .update()
                .fields(paths!(AccountRecoveryDbEntry::{iss, aud, uid_key, uid_val}))
                .in_col("accounts")
                .document_id(&doc_id)
                .object(&entry) // op 1
                .transforms(|builder| {
                    builder.fields([
                        builder
                            .field(path!(AccountRecoveryDbEntry::num_requests))
                            .increment(1), // op 2
                        builder
                            .field(path!(AccountRecoveryDbEntry::last_request_unix_ms))
                            .maximum(now_unix_ms), // op 3
                        builder
                            .field(path!(
                                AccountRecoveryDbEntry::first_request_unix_ms_minus_1q
                            ))
                            .minimum(now_unix_ms - 1_000_000_000_000_000), // op 4
                    ])
                })
                .add_to_transaction(&mut txn)
                .map_err(|e| {
                    PepperServiceError::InternalError(format!("add_to_transaction error: {e}"))
                })?;
            let txn_result = txn.commit().await;

            if let Err(e) = txn_result {
                warn!("ACCOUNT_RECOVERY_DB operation failed: {e}");
            }
            Ok(())
        },
        Err(e) => {
            warn!("ACCOUNT_RECOVERY_DB client failed to init: {e}");
            Ok(())
        },
    }
}
