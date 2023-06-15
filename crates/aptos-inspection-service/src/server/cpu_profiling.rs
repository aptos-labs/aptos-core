use hyper::{Body, StatusCode};
use crate::{
    server::utils::CONTENT_TYPE_SVG
};
use std::{thread, time};
use std::fs::File;

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