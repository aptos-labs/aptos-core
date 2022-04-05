// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]
#![recursion_limit = "128"]

pub mod constants;

use aptos_logger::prelude::*;
use aptos_metrics::json_metrics::get_git_rev;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

pub const GA_MEASUREMENT_ID: &str = "GA_MEASUREMENT_ID";
pub const GA_API_SECRET: &str = "GA_API_SECRET";
pub const APTOS_TELEMETRY_OPTOUT: &str = "APTOS_TELEMETRY_OPTOUT";

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

pub fn is_optout() -> bool {
    env::var(APTOS_TELEMETRY_OPTOUT).is_ok()
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

pub async fn send_data(event_name: String, user_id: String, event_params: HashMap<String, String>) {
    if is_optout() {
        debug!("Error sending data: optout of Aptos telemetry");
        return;
    }

    // parse environment variables
    let api_secret = env::var(GA_API_SECRET).unwrap_or(constants::APTOS_GA_API_SECRET.to_string());
    let measurement_id =
        env::var(GA_MEASUREMENT_ID).unwrap_or(constants::APTOS_GA_MEASUREMENT_ID.to_string());

    // dump event params in a new hashmap with some default params to include
    let mut new_event_params: HashMap<String, String> = event_params.clone();
    // attempt to get IP address
    let ip_origin = get_ip_origin().await;
    new_event_params.insert(constants::IP_ADDR_METRIC.to_string(), ip_origin);
    new_event_params.insert(constants::GIT_REV_METRIC.to_string(), get_git_rev());

    let metrics_event = MetricsEvent {
        name: event_name,
        params: new_event_params,
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
        Ok(_) => debug!("Sent telemetry data {:?}", &metrics_dump),
        Err(e) => debug!("{:?}", e),
    }
}
