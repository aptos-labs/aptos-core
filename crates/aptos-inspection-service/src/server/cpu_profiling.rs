use hyper::{Body, StatusCode};
use crate::{
    server::utils::CONTENT_TYPE_SVG, CONFIGURATION_PATH, FORGE_METRICS_PATH, JSON_METRICS_PATH,
    METRICS_PATH, PEER_INFORMATION_PATH, SYSTEM_INFORMATION_PATH,
};
use std::{thread, time};

use std::fs;
use std::fs::File;

use std::io::Read;

pub fn handle_cpu_profiling_request() -> (StatusCode, Body, String) {
    let guard = pprof::ProfilerGuard::new(100).unwrap();
    let five_secs = time::Duration::from_millis(5000);
    thread::sleep(five_secs);

    if let Ok(report) = guard.report().build() {
        let file = File::create("/home/ubuntu/aptos-core/crates/aptos-inspection-service/src/server/profiling_dashboard/flamegraph.svg").unwrap();
        report.flamegraph(file).unwrap();

        println!("report: {:?}", &report);
    };

    (
        StatusCode::OK,
        Body::from("555"),
        CONTENT_TYPE_SVG.into(),
    )
    
}