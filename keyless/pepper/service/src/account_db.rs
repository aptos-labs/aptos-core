// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_logger::warn;
use firestore::{FirestoreDb, FirestoreDbOptions, FirestoreResult};
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
