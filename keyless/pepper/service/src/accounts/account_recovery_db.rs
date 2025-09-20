// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{error::PepperServiceError, utils};
use aptos_infallible::duration_since_epoch;
use aptos_keyless_pepper_common::{account_recovery_db::AccountRecoveryDbEntry, PepperInput};
use aptos_logger::{info, warn};
use firestore::{path, paths, FirestoreDb, FirestoreDbOptions};
use std::io::Write;
use tempfile::NamedTempFile;

// Firestore DB environment variables
const ENV_GOOGLE_PROJECT_ID: &str = "PROJECT_ID";
const ENV_FIRESTORE_DATABASE_ID: &str = "DATABASE_ID";

// Firestore DB constants for unknown values
const UNKNOWN_GOOGLE_PROJECT_ID: &str = "unknown_project";
const UNKNOWN_FIRESTORE_DATABASE_ID: &str = "unknown_database";

// Firestore DB transaction constants
const FIRESTORE_DB_COLLECTION_ID: &str = "accounts";

/// A generic interface for the account recovery database
#[async_trait::async_trait]
pub trait AccountRecoveryDBInterface {
    /// Updates the account recovery database with the given pepper input
    async fn update_db_with_pepper_input(
        &self,
        pepper_input: &PepperInput,
    ) -> Result<(), PepperServiceError>;
}

/// A GCP firestore that holds all account recovery data
pub struct FirestoreAccountRecoveryDB {
    firestore_db: FirestoreDb,
}

impl FirestoreAccountRecoveryDB {
    pub async fn new() -> Self {
        // Get the Google project ID
        let google_project_id = utils::read_environment_variable(ENV_GOOGLE_PROJECT_ID)
            .unwrap_or_else(|error| {
                warn!(
                    "{} is not set! Using the default Google project ID: {}! Error: {}",
                    ENV_GOOGLE_PROJECT_ID, UNKNOWN_GOOGLE_PROJECT_ID, error
                );
                UNKNOWN_GOOGLE_PROJECT_ID.into()
            });

        // Get the Firestore database ID
        let firestore_database_id = utils::read_environment_variable(ENV_FIRESTORE_DATABASE_ID)
            .unwrap_or_else(|error| {
                warn!(
                    "{} is not set! Using the default Firestore database ID: {}! Error: {}",
                    ENV_FIRESTORE_DATABASE_ID, UNKNOWN_FIRESTORE_DATABASE_ID, error
                );
                UNKNOWN_FIRESTORE_DATABASE_ID.into()
            });

        // Create the FirestoreDB instance
        let firestore_db_options = FirestoreDbOptions {
            google_project_id,
            database_id: firestore_database_id,
            max_retries: 1,
            firebase_api_url: None,
        };
        match FirestoreDb::with_options(firestore_db_options).await {
            Ok(firestore_db) => Self { firestore_db },
            Err(error) => {
                panic!(
                    "Failed to create Firestore account recovery database! Error: {}",
                    error
                );
            },
        }
    }
}

#[async_trait::async_trait]
impl AccountRecoveryDBInterface for FirestoreAccountRecoveryDB {
    async fn update_db_with_pepper_input(
        &self,
        pepper_input: &PepperInput,
    ) -> Result<(), PepperServiceError> {
        // Create the database entry and get the document ID
        let entry = AccountRecoveryDbEntry {
            iss: pepper_input.iss.clone(),
            aud: pepper_input.aud.clone(),
            uid_key: pepper_input.uid_key.clone(),
            uid_val: pepper_input.uid_val.clone(),
            first_request_unix_ms_minus_1q: None,
            last_request_unix_ms: None,
            num_requests: None,
        };
        let document_id = entry.document_id();

        // Get the time now
        let now_unix_ms = duration_since_epoch().as_millis() as i64;

        // To update the DB, we use the following strategy:
        // 1. If the document doesn't exist, create the document for the user identifier `(iss, aud, uid_key, uid_val)`,
        //    but leave counter/time fields unspecified.
        // 2. `num_requests += 1`, assuming the default value is 0.
        // 3. `last_request_unix_ms = max(last_request_unix_ms, now)`, assuming the default value is 0.
        // 4. `first_request_unix_ms = min(first_request_unix_ms, now)`, assuming the default value is +inf.
        //
        // This strategy is preferred because all the operations can be done on the server-side,
        // which means the transaction should require only 1 RTT (this is better than using
        // the read-compute-write pattern that requires 2 RTTs).
        //
        // Note: this strategy requires some modifications:
        // - In firestore, the default value of a number field is 0, and we do not have a way to customize it for step 4.
        // - The workaround here is to apply an offset so 0 becomes a legitimate default value.
        // - So we work with `first_request_unix_ms_minus_1q` instead, which is defined as
        //   `first_request_unix_ms - 1_000_000_000_000_000`, where 1_000_000_000_000_000 milliseconds is roughly 31710 years.

        // Create the firestore DB transaction
        let mut firestore_transaction =
            self.firestore_db
                .begin_transaction()
                .await
                .map_err(|error| {
                    PepperServiceError::InternalError(format!(
                        "Firestore DB begin_transaction() error: {}",
                        error
                    ))
                })?;
        self.firestore_db
            .fluent()
            .update()
            .fields(paths!(AccountRecoveryDbEntry::{iss, aud, uid_key, uid_val}))
            .in_col(FIRESTORE_DB_COLLECTION_ID)
            .document_id(&document_id)
            .object(&entry) // Step 1
            .transforms(|builder| {
                builder.fields([
                    builder
                        .field(path!(AccountRecoveryDbEntry::num_requests))
                        .increment(1), // Step 2
                    builder
                        .field(path!(AccountRecoveryDbEntry::last_request_unix_ms))
                        .maximum(now_unix_ms), // Step 3
                    builder
                        .field(path!(
                            AccountRecoveryDbEntry::first_request_unix_ms_minus_1q
                        ))
                        .minimum(now_unix_ms - 1_000_000_000_000_000), // Step 4
                ])
            })
            .add_to_transaction(&mut firestore_transaction)
            .map_err(|error| {
                PepperServiceError::InternalError(format!(
                    "Firestore DB add_to_transaction() error: {}",
                    error
                ))
            })?;

        // Commit the DB transaction
        match firestore_transaction.commit().await {
            Ok(_) => Ok(()),
            Err(error) => {
                let error_message = format!(
                    "Failed to commit Firestore DB transaction for pepper input {:?}! Document ID {}, Error: {}",
                    pepper_input, document_id, error
                );
                Err(PepperServiceError::InternalError(error_message))
            },
        }
    }
}

/// A test implementation of the account recovery database. Internally, the
/// DB is represented as a temporary file, with each line being a single JSON entry.
pub struct TestAccountRecoveryDB {
    temp_file: NamedTempFile,
}

impl TestAccountRecoveryDB {
    pub fn new() -> Self {
        // Open the temporary file
        let temp_file = match NamedTempFile::new() {
            Ok(file) => file,
            Err(error) => panic!("Failed to create temp file! Error: {}", error),
        };

        // Print the temporary file path
        let temp_file_path = temp_file.path().to_str().unwrap().to_string();
        info!(
            "Created temporary account recovery DB at {}",
            temp_file_path
        );

        Self { temp_file }
    }
}

#[async_trait::async_trait]
impl AccountRecoveryDBInterface for TestAccountRecoveryDB {
    async fn update_db_with_pepper_input(
        &self,
        pepper_input: &PepperInput,
    ) -> Result<(), PepperServiceError> {
        // Format the pepper input as a JSON line
        let json_line = match serde_json::to_string(pepper_input) {
            Ok(json) => json,
            Err(error) => {
                return Err(PepperServiceError::InternalError(format!(
                    "Failed to serialize pepper input {:?} to JSON! Error: {}",
                    pepper_input, error
                )));
            },
        };

        // Write the JSON line to the temporary file
        writeln!(&self.temp_file, "{}", json_line).map_err(|error| {
            PepperServiceError::InternalError(format!(
                "Failed to write pepper input {:?} to temp file! Error: {}",
                pepper_input, error
            ))
        })
    }
}

impl Default for TestAccountRecoveryDB {
    fn default() -> Self {
        Self::new()
    }
}
