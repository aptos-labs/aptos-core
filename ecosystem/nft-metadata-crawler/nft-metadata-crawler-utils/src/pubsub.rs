// Copyright © Aptos Foundation

use std::error::Error;

use google_cloud_auth::token_source::TokenSource;
use reqwest::Client;
use serde_json::{json, Value};

pub async fn publish_to_queue(
    client: &Client,
    msg: String,
    ts: &Box<dyn TokenSource>,
    topic_name: &String,
    force: bool,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let url = format!("https://pubsub.googleapis.com/v1/{}:publish", topic_name);

    let res = client
        .post(&url)
        .bearer_auth(ts.token().await?.access_token)
        .json(&json!({
            "messages": [
                {
                    "data": base64::encode(format!("{},{}", msg.clone(), force))
                }
            ]
        }))
        .send()
        .await?;

    match res.status().as_u16() {
        200..=299 => Ok(msg),
        _ => Err(format!("Error publishing to queue: {}", res.text().await?).into()),
    }
}

pub async fn consume_from_queue(
    client: &Client,
    ts: &Box<dyn TokenSource>,
    subsctiption_name: &String,
) -> Result<Vec<(String, String)>, Box<dyn Error + Send + Sync>> {
    let url = format!(
        "https://pubsub.googleapis.com/v1/{}:pull",
        subsctiption_name
    );

    let res = client
        .post(&url)
        .bearer_auth(ts.token().await?.access_token)
        .json(&json!({
            "maxMessages": 10
        }))
        .send()
        .await?;

    let body: Value = res.json().await?;
    if let Some(messages) = body["receivedMessages"].as_array() {
        let mut links = Vec::new();
        for message in messages {
            let msg = message["message"]["data"].as_str();
            if let Some(msg) = msg {
                links.push((
                    String::from_utf8(base64::decode(msg)?)?,
                    String::from(message["ackId"].as_str().unwrap_or("")),
                ));
            }
        }
        Ok(links)
    } else {
        return Err("No message found".into());
    }
}

pub async fn send_ack(
    client: &Client,
    ts: &Box<dyn TokenSource>,
    subscription_name: &String,
    ack: &String,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let url = format!(
        "https://pubsub.googleapis.com/v1/{}:acknowledge",
        subscription_name
    );

    let res = client
        .post(&url)
        .bearer_auth(ts.token().await?.access_token)
        .json(&json!({ "ackIds": [ack] }))
        .send()
        .await?;

    match res.status().as_u16() {
        200..=299 => Ok(()),
        _ => {
            let text = res.text().await?;
            Err(format!("Error acking {}", text).into())
        },
    }
}
