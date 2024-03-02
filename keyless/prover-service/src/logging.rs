// Copyright © Aptos Foundation

use axum::http::StatusCode;
use init_tracing_opentelemetry::tracing_subscriber_ext::{
    build_loglevel_filter_layer, build_otel_layer,
};
use std::env;
use tracing::{error, info, warn};
use tracing_subscriber::{
    fmt::{
        format::{FmtSpan, Format, Json, JsonFields},
        Layer,
    },
    prelude::*,
};

pub fn init_tracing() -> anyhow::Result<()> {
    //setup a temporary subscriber to log output during setup
    let subscriber = tracing_subscriber::registry()
        .with(build_loglevel_filter_layer())
        .with(build_json_log_layer());
    let _guard = tracing::subscriber::set_default(subscriber);
    info!("init logging & tracing");

    if env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_ok() {
        let subscriber = tracing_subscriber::registry()
            .with(build_otel_layer()?)
            .with(build_loglevel_filter_layer())
            .with(build_json_log_layer());

        tracing::subscriber::set_global_default(subscriber)?;
    } else {
        let subscriber = tracing_subscriber::registry()
            .with(build_loglevel_filter_layer())
            .with(build_json_log_layer());

        tracing::subscriber::set_global_default(subscriber)?;
    }

    Ok(())
}

fn build_json_log_layer<S>() -> Layer<S, JsonFields, Format<Json>> {
    tracing_subscriber::fmt::layer()
        .json()
        .flatten_event(true)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_span_list(false)
        .with_current_span(true)
}

pub fn do_tracing(e: anyhow::Error, code: StatusCode, message: &str) {
    match code {
        StatusCode::BAD_REQUEST => {
            let e_box: Box<dyn std::error::Error> = e.into();
            warn!(message, error = e_box)
        },
        StatusCode::INTERNAL_SERVER_ERROR => {
            let e_box: Box<dyn std::error::Error> = e.into();
            error!(message, error = e_box)
        },
        _ => {
            let e_box: Box<dyn std::error::Error> = e.into();
            error!(message, error = e_box)
        },
    };
}
