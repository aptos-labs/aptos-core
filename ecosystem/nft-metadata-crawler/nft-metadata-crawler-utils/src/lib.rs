// Copyright © Aptos Foundation

use google_cloud_auth::{
    project::{create_token_source, Config},
    token_source::TokenSource,
};

pub mod gcs;
pub mod pubsub;

/// Retrieves token source from GOOGLE_APPLICATION_CREDENTIALS
pub async fn get_token_source() -> Box<dyn TokenSource> {
    create_token_source(Config {
        audience: None,
        scopes: Some(&["https://www.googleapis.com/auth/cloud-platform"]),
        sub: None,
    })
    .await
    .expect("No token source")
}
