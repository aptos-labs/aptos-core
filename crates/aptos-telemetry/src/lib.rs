// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]
#![recursion_limit = "128"]

pub mod constants;

use aptos_logger::prelude::*;
use aptos_metrics::{json_metrics::get_git_rev, register_int_counter_vec, IntCounterVec};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

pub const GA_MEASUREMENT_ID: &str = "GA_MEASUREMENT_ID";
pub const GA_API_SECRET: &str = "GA_API_SECRET";
pub const APTOS_TELEMETRY_DISABLE: &str = "APTOS_TELEMETRY_DISABLE";

pub static APTOS_TELEMETRY_SUCCESS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_telemetry_success",
        "Number of telemetry events successfully sent",
        &["event_name"]
    )
    .unwrap()
});

pub static APTOS_TELEMETRY_FAILURE: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_telemetry_failure",
        "Number of telemetry events failed to send",
        &["event_name"]
    )
    .unwrap()
});

#[derive(Debug, Serialize, Deserialize)]
struct MetricsDump {
    client_id: String,
    user_id: String,
    timestamp_micros: String,
    events: Vec<MetricsEvent>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MetricsEvent {
    name: String,
    params: HashMap<String, String>,
}

#[derive(Deserialize)]
struct Ip {
    origin: String,
}

pub fn is_disabled() -> bool {
    env::var(APTOS_TELEMETRY_DISABLE).is_ok()
}

async fn get_ip_origin() -> String {
    let resp = reqwest::get(constants::HTTPBIN_URL).await;
    match resp {
        Ok(json) => match json.json::<Ip>().await {
            Ok(ip) => ip.origin,
            Err(_) => String::new(),
        },
        Err(_) => String::new(),
    }
}

pub async fn send_env_data(
    event_name: String,
    user_id: String,
    event_params: HashMap<String, String>,
) {
    if is_disabled() {
        debug!("Error sending data: disabled Aptos telemetry");
        return;
    }

    // dump event params in a new hashmap with some default params to include
    let mut new_event_params: HashMap<String, String> = event_params.clone();
    // attempt to get IP address
    let ip_origin = get_ip_origin().await;
    new_event_params.insert(constants::IP_ADDR_METRIC.to_string(), ip_origin);
    new_event_params.insert(constants::GIT_REV_METRIC.to_string(), get_git_rev());
    send_data(event_name, user_id, new_event_params).await;
}

pub async fn send_data(event_name: String, user_id: String, event_params: HashMap<String, String>) {
    if is_disabled() {
        debug!("Error sending data: disabled Aptos telemetry");
        return;
    }

    // parse environment variables
    let api_secret =
        env::var(GA_API_SECRET).unwrap_or_else(|_| constants::APTOS_GA_API_SECRET.to_string());
    let measurement_id = env::var(GA_MEASUREMENT_ID)
        .unwrap_or_else(|_| constants::APTOS_GA_MEASUREMENT_ID.to_string());

    let metrics_event = MetricsEvent {
        name: event_name.clone(),
        params: event_params,
    };

    let metrics_dump = MetricsDump {
        client_id: Uuid::new_v4().to_string(),
        user_id,
        timestamp_micros: match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(n) => n.as_micros().to_string(),
            Err(_) => String::new(),
        },
        events: vec![metrics_event],
    };

    let client = reqwest::Client::new();
    // do not block on these requests
    tokio::spawn(async move {
        let res = client
            .post(format!(
                "{}?&measurement_id={}&api_secret={}",
                constants::GA4_URL,
                measurement_id,
                api_secret
            ))
            .json::<MetricsDump>(&metrics_dump)
            .send()
            .await;
        match res {
            Ok(res) => {
                let status_code = res.status().as_u16();
                if status_code > 200 && status_code < 299 {
                    info!("Sent telemetry event {}", event_name.as_str());
                    debug!("Sent telemetry data {:?}", &metrics_dump);
                    APTOS_TELEMETRY_SUCCESS
                        .with_label_values(&[event_name.as_str()])
                        .inc();
                } else {
                    info!(
                        "Failed to send telemetry event {}: {}",
                        res.status(),
                        event_name.as_str()
                    );
                    debug!("{:?}", res.text().await);
                    APTOS_TELEMETRY_FAILURE
                        .with_label_values(&[event_name.as_str()])
                        .inc();
                }
            }
            Err(e) => {
                info!("Failed to send telemetry event {}", event_name.as_str());
                debug!("{:?}", e);
                APTOS_TELEMETRY_FAILURE
                    .with_label_values(&[event_name.as_str()])
                    .inc();
            }
        }
    });
}
