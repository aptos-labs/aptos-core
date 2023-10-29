// Copyright © Aptos Foundation

use crate::worker::ParserConfig;
use diesel::{
    r2d2::{ConnectionManager, Pool},
    PgConnection,
};
use google_cloud_storage::client::Client;
use serde::Deserialize;

/// Struct to hold context required for parsing
#[derive(Clone)]
pub struct AppContext {
    pub parser_config: ParserConfig,
    pub pool: Pool<ConnectionManager<PgConnection>>,
    pub gcs_client: Client,
}

/// Struct to help deserialize the body of the POST request
#[derive(Deserialize)]
pub struct PubSubBody {
    pub message: Message,
}

/// Struct to help deserialize the raw PubSub message
#[derive(Deserialize)]
pub struct Message {
    pub data: String,
}
