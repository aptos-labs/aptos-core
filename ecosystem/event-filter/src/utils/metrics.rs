use aptos_metrics_core::{register_gauge, Gauge};
use once_cell::sync::Lazy;

pub static E2E_LATENCY_IN_SECS: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(
        "event_filter_e2e_latency_in_secs",
        "E2E latency observed by event filter",
    )
    .unwrap()
});
