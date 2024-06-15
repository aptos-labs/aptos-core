// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::env::VarError;
use firestore::{FirestoreDb, FirestoreDbOptions};
use google_cloud_auth::project::Config;
use once_cell::sync::Lazy;
use tokio::sync::OnceCell;
use aptos_logger::{info, warn};

pub static AUD_DB: Lazy<OnceCell<FirestoreDb>> = Lazy::new(OnceCell::new);

pub async fn init_account_db() -> FirestoreDb {
    let google_project_id = match std::env::var("KEYLESS_PROJECT_ID") {
        Ok(id) => id,
        Err(e) => {
            warn!("Could not load envvar `KEYLESS_PROJECT_ID`: {e}");
            "unknown_project".to_string()
        }
    };

    let database_id = match std::env::var("DATABASE_ID") {
        Ok(id) => id,
        Err(e) => {
            warn!("Could not load envvar `DATABASE_ID`: {e}");
            "unknown_database".to_string()
        }
    };

    let option = FirestoreDbOptions {
        google_project_id,
        database_id,
        max_retries: 1,
        firebase_api_url: None,
    };
    FirestoreDb::with_options(option).await.unwrap()
}
