// Copyright © Aptos Foundation

use serde_json::Value;

pub struct JSONParser;

impl JSONParser {
    /**
     * Parses JSON from input URI.
     * Returns the underlying raw image URI, raw animation URI, and JSON.
     */
    pub async fn parse(_uri: String) -> (Option<String>, Option<String>, Option<Value>) {
        todo!();
    }
}
