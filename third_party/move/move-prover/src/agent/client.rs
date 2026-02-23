// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Simple blocking Claude API client for the agentic spec inference loop.

use anyhow::{anyhow, Context};
use log::info;
use serde_json::{json, Value};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

const API_ENDPOINT: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// A blocking client for the Claude Messages API.
pub struct ClaudeClient {
    api_key: String,
    model: String,
    max_tokens: usize,
    http: reqwest::blocking::Client,
}

/// A single message in the conversation.
pub struct Message {
    pub role: &'static str,
    pub content: String,
}

impl ClaudeClient {
    /// Create a new client. Reads `ANTHROPIC_API_KEY` from the environment.
    pub fn new(model: String, max_tokens: usize) -> anyhow::Result<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .context("ANTHROPIC_API_KEY environment variable must be set for --ai mode")?;
        let http = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .context("failed to build HTTP client")?;
        Ok(Self {
            api_key,
            model,
            max_tokens,
            http,
        })
    }

    /// Send a request to the Claude Messages API and return the text response.
    pub fn send(&self, system_prompt: &str, messages: &[Message]) -> anyhow::Result<String> {
        let msgs: Vec<Value> = messages
            .iter()
            .map(|m| {
                json!({
                    "role": m.role,
                    "content": m.content,
                })
            })
            .collect();

        let body = json!({
            "model": self.model,
            "max_tokens": self.max_tokens,
            "system": system_prompt,
            "messages": msgs,
        });

        // Send the request on a background thread so we can log progress while waiting.
        let (tx, rx) = mpsc::channel();
        let http = self.http.clone();
        let api_key = self.api_key.clone();
        std::thread::spawn(move || {
            let result = http
                .post(API_ENDPOINT)
                .header("x-api-key", &api_key)
                .header("anthropic-version", ANTHROPIC_VERSION)
                .header("content-type", "application/json")
                .json(&body)
                .send();
            let _ = tx.send(result);
        });

        let start = Instant::now();
        let response = loop {
            let poll_secs = if start.elapsed().as_secs() < 30 {
                10
            } else {
                30
            };
            match rx.recv_timeout(Duration::from_secs(poll_secs)) {
                Ok(result) => break result,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    info!(
                        "[claude] Waiting for response... ({:.0}s elapsed)",
                        start.elapsed().as_secs_f64()
                    );
                },
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(anyhow!("Claude API request thread terminated unexpectedly"));
                },
            }
        }
        .context("failed to send request to Claude API")?;

        let elapsed = start.elapsed();
        info!(
            "[claude] Response received after {:.1}s",
            elapsed.as_secs_f64()
        );

        let status = response.status();
        let response_text = response
            .text()
            .context("failed to read Claude API response body")?;

        if !status.is_success() {
            return Err(anyhow!(
                "Claude API returned status {}: {}",
                status,
                response_text
            ));
        }

        let parsed: Value =
            serde_json::from_str(&response_text).context("failed to parse Claude API response")?;

        // Extract text from the first content block.
        let text = parsed["content"]
            .as_array()
            .and_then(|blocks| {
                blocks.iter().find_map(|block| {
                    if block["type"].as_str() == Some("text") {
                        block["text"].as_str().map(|s| s.to_string())
                    } else {
                        None
                    }
                })
            })
            .ok_or_else(|| {
                anyhow!(
                    "Claude API response missing text content: {}",
                    response_text
                )
            })?;

        Ok(text)
    }
}
